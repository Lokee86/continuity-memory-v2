use crate::{
    Branch, Cva, EpisodeConfig, GeneralEndpoint, GeneralEndpointError, InsomniaExtractor,
    InsomniaWorkerConfig, SimulatedEmbeddingEndpoint, VectorNormalization,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-workers-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn queue_import(cva: &mut Cva, conversation: &str, index: usize) {
    let user = format!("{conversation}-u");
    cva.append_node(
        user.clone(),
        conversation.into(),
        None,
        "user".into(),
        i64::try_from(index + 1).unwrap(),
        "No durable memory here.",
    )
    .unwrap();
    cva.finalize_import_path_and_queue(
        conversation,
        &user,
        EpisodeConfig::default(),
        i64::try_from(index + 100).unwrap(),
    )
    .unwrap();
}

struct TrackingEndpoint {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    failures_remaining: Arc<AtomicUsize>,
    invalid_configuration: bool,
    delay: Duration,
}

impl GeneralEndpoint for TrackingEndpoint {
    fn model(&self) -> &str {
        "tracking-model"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        _schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.invalid_configuration {
            return Err(GeneralEndpointError::InvalidConfiguration("test failure"));
        }
        if self
            .failures_remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_sub(1)
            })
            .is_ok()
        {
            return Err(GeneralEndpointError::Failure(
                "transient test failure".into(),
            ));
        }
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        thread::sleep(self.delay);
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(json!({"candidates": [], "evidence_requests": []}))
    }
}

fn tracking_endpoint(
    delay: Duration,
    failures: usize,
    invalid_configuration: bool,
) -> (TrackingEndpoint, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let peak = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    (
        TrackingEndpoint {
            active: Arc::new(AtomicUsize::new(0)),
            peak: Arc::clone(&peak),
            calls: Arc::clone(&calls),
            failures_remaining: Arc::new(AtomicUsize::new(failures)),
            invalid_configuration,
            delay,
        },
        peak,
        calls,
    )
}

#[test]
fn canonical_import_registration_materializes_and_queues_the_whole_file() {
    let path = test_path("registration.cva");
    let mut cva = Cva::create(path).unwrap();
    cva.append_node(
        "u0".into(),
        "c1".into(),
        None,
        "user".into(),
        1,
        "Remember this import.",
    )
    .unwrap();
    cva.append_branch(Branch {
        id: "main".into(),
        conversation_id: "c1".into(),
        leaf_node_id: "u0".into(),
        canonical: true,
    })
    .unwrap();

    cva.finalize_canonical_imports_and_queue(EpisodeConfig::default(), 10)
        .unwrap();
    assert_eq!(cva.episodes().len(), 1);
    assert_eq!(cva.insomnia_stats().total, 1);
    cva.finalize_canonical_imports_and_queue(EpisodeConfig::default(), 20)
        .unwrap();
    assert_eq!(cva.episodes().len(), 1);
    assert_eq!(cva.insomnia_stats().total, 1);
}

#[test]
fn worker_pool_overlaps_model_calls_and_drains_the_backlog() {
    let path = test_path("parallel.cva");
    let mut cva = Cva::create(path).unwrap();
    for index in 0..8 {
        queue_import(&mut cva, &format!("c{index}"), index);
    }
    let (endpoint, endpoint_peak, calls) = tracking_endpoint(Duration::from_millis(30), 0, false);
    let extractor = InsomniaExtractor::new(endpoint);
    let config = InsomniaWorkerConfig {
        workers: 4,
        poll_interval_ns: 1_000_000,
        ..Default::default()
    };
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let result = cva
        .drain_insomnia_backlog(&extractor, &embedding, config)
        .unwrap();

    assert_eq!(result.completed_episodes, 8);
    assert_eq!(result.claimed_attempts, 8);
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    assert!(endpoint_peak.load(Ordering::SeqCst) > 1);
    assert!(result.peak_active_workers > 1);
    let stats = cva.insomnia_stats();
    assert_eq!(stats.complete, 8);
    assert_eq!(stats.pending + stats.processing + stats.failed, 0);
}

#[test]
fn retryable_failure_is_reclaimed_and_completed_in_the_same_drain() {
    let path = test_path("retry.cva");
    let mut cva = Cva::create(path).unwrap();
    queue_import(&mut cva, "c1", 0);
    let (endpoint, _, calls) = tracking_endpoint(Duration::ZERO, 1, false);
    let extractor = InsomniaExtractor::new(endpoint);
    let config = InsomniaWorkerConfig {
        workers: 1,
        retry_delay_ns: 1_000_000,
        poll_interval_ns: 100_000,
        max_attempts: 3,
        ..Default::default()
    };
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let result = cva
        .drain_insomnia_backlog(&extractor, &embedding, config)
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.claimed_attempts, 2);
    assert_eq!(result.failed_attempts, 1);
    assert_eq!(result.completed_episodes, 1);
    assert_eq!(result.terminal_episodes, 0);
    assert_eq!(cva.insomnia_stats().attempts, 1);
}

#[test]
fn invalid_endpoint_configuration_becomes_terminal_without_retry() {
    let path = test_path("terminal.cva");
    let mut cva = Cva::create(path).unwrap();
    queue_import(&mut cva, "c1", 0);
    let (endpoint, _, calls) = tracking_endpoint(Duration::ZERO, 0, true);
    let extractor = InsomniaExtractor::new(endpoint);
    let config = InsomniaWorkerConfig {
        workers: 2,
        poll_interval_ns: 100_000,
        ..Default::default()
    };
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let result = cva
        .drain_insomnia_backlog(&extractor, &embedding, config)
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.claimed_attempts, 1);
    assert_eq!(result.completed_episodes, 0);
    assert_eq!(result.terminal_episodes, 1);
    assert_eq!(cva.insomnia_stats().terminal, 1);
}
