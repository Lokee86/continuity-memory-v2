use reliquary_memory::{
    Cva, EpisodeConfig, FragmentConfig, GeneralEndpoint, GeneralEndpointError, InsomniaExtractor,
    InsomniaWorkerConfig, SimulatedEmbeddingEndpoint, VectorNormalization,
};
use serde_json::{Value, json};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct SyntheticMemoryEndpoint {
    calls: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
    delay: Duration,
    evidence_every: usize,
}

impl SyntheticMemoryEndpoint {
    fn new(delay: Duration, evidence_every: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            delay,
            evidence_every,
        }
    }
}

impl GeneralEndpoint for SyntheticMemoryEndpoint {
    fn model(&self) -> &str {
        "synthetic-memory"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        _schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }

        let payload: Value = serde_json::from_str(user_payload)
            .map_err(|_| GeneralEndpointError::InvalidResponse("invalid synthetic payload"))?;
        let final_round = payload.get("authoritative_episode").is_some();
        let episode = payload.get("authoritative_episode").unwrap_or(&payload);
        let turn = episode["turns"]
            .as_array()
            .and_then(|turns| turns.iter().find(|turn| turn["role"] == "user"))
            .ok_or(GeneralEndpointError::InvalidResponse(
                "missing synthetic user turn",
            ))?;
        let node_id = turn["id"]
            .as_str()
            .ok_or(GeneralEndpointError::InvalidResponse(
                "missing synthetic node id",
            ))?;
        let source = turn["content"]
            .as_str()
            .ok_or(GeneralEndpointError::InvalidResponse(
                "missing synthetic source",
            ))?;
        let index = node_id
            .strip_prefix("u-")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);

        self.active.fetch_sub(1, Ordering::SeqCst);
        if !final_round && self.evidence_every > 0 && index % self.evidence_every == 0 {
            return Ok(json!({
                "candidates": [],
                "evidence_requests": [{
                    "kind": "archive_search",
                    "conversation_id": "",
                    "node_id": "",
                    "start_node_id": "",
                    "end_node_id": "",
                    "query": format!("durable value {index}"),
                    "limit": 1
                }]
            }));
        }
        Ok(json!({
            "candidates": [{
                "authority_kind": "direct",
                "category": "decision",
                "type": "project",
                "title": format!("Archive fact {node_id}"),
                "content": source,
                "source_node_id": node_id,
                "source_quote": source,
                "authority_source_conversation_id": "",
                "authority_source_node_id": "",
                "authority_source_quote": "",
                "grounding_source_conversation_id": "",
                "grounding_source_node_id": "",
                "grounding_source_quote": ""
            }],
            "evidence_requests": []
        }))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let episodes: usize = args.next().unwrap_or_else(|| "5000".into()).parse()?;
    let workers: usize = args.next().unwrap_or_else(|| "48".into()).parse()?;
    let dimensions: usize = args.next().unwrap_or_else(|| "1024".into()).parse()?;
    let delay_ms: u64 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let evidence_every: usize = args.next().unwrap_or_else(|| "0".into()).parse()?;

    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("continuity-insomnia-stress-{unique}.cva"));

    let build_started = Instant::now();
    let mut cva = Cva::create(&path)?;
    for index in 0..episodes {
        let conversation = format!("stress-{index}");
        let node = format!("u-{index}");
        cva.append_node(
            node.clone(),
            conversation.clone(),
            None,
            "user".into(),
            i64::try_from(index + 1)?,
            &format!("Project Atlas archive fact {index} has durable value {index}."),
        )?;
        cva.materialize_path_fragments(&conversation, &node, FragmentConfig::default(), true)?;
        cva.finalize_import_path_and_queue(
            &conversation,
            &node,
            EpisodeConfig::default(),
            i64::try_from(episodes + index + 1)?,
        )?;
    }
    let build_elapsed = build_started.elapsed();

    let endpoint = SyntheticMemoryEndpoint::new(Duration::from_millis(delay_ms), evidence_every);
    let extractor = InsomniaExtractor::new(endpoint);
    let embedding =
        SimulatedEmbeddingEndpoint::new(u32::try_from(dimensions)?, VectorNormalization::L2, 7);
    let config = InsomniaWorkerConfig {
        workers,
        poll_interval_ns: 100_000,
        ..Default::default()
    };

    let drain_started = Instant::now();
    let result = cva.drain_insomnia_backlog(&extractor, &embedding, config)?;
    let drain_elapsed = drain_started.elapsed();
    cva.sync()?;
    let bytes = fs::metadata(&path)?.len();
    drop(cva);

    let reopen_started = Instant::now();
    let reopened = Cva::open(&path)?;
    let reopen_elapsed = reopen_started.elapsed();
    let memory_count = reopened.memory_stats().memories;
    let stats = reopened.insomnia_stats();

    assert_eq!(result.completed_episodes, episodes);
    assert_eq!(result.memories_created, episodes);
    assert_eq!(memory_count, episodes);
    assert_eq!(stats.complete, episodes);
    assert_eq!(stats.pending + stats.processing + stats.failed, 0);
    assert_eq!(result.memory_vectors_embedded, episodes);

    println!(
        "episodes={episodes} workers={workers} dimensions={dimensions} delay_ms={delay_ms} evidence_every={evidence_every}"
    );
    println!("build_ms={:.3}", build_elapsed.as_secs_f64() * 1000.0);
    println!("drain_ms={:.3}", drain_elapsed.as_secs_f64() * 1000.0);
    println!(
        "episodes_per_s={:.3}",
        episodes as f64 / drain_elapsed.as_secs_f64()
    );
    println!("peak_active_workers={}", result.peak_active_workers);
    println!("memories_created={}", result.memories_created);
    println!("vectors_embedded={}", result.memory_vectors_embedded);
    println!("evidence_turns={}", result.evidence_turns);
    println!("file_bytes={bytes}");
    println!("reopen_ms={:.3}", reopen_elapsed.as_secs_f64() * 1000.0);

    drop(reopened);
    fs::remove_file(path)?;
    Ok(())
}
