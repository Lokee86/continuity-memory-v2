use reliquary_memory::{
    ConfiguredGeneralEndpoint, GeneralEndpoint, GeneralEndpointError, InsomniaCandidate,
    InsomniaExtractionError, InsomniaOwnership, MemoryTextField, enrich_entity_calibration,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub fn run(
    endpoint: ConfiguredGeneralEndpoint,
    gold: Vec<Value>,
    candidates: HashMap<String, Value>,
    workers: usize,
) -> Result<Vec<Value>, String> {
    let count = gold.len();
    let endpoint = Arc::new(endpoint);
    let gold = Arc::new(gold);
    let candidates = Arc::new(candidates);
    let next = Arc::new(Mutex::new(0usize));
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..workers.min(count.max(1)) {
            let endpoint = Arc::clone(&endpoint);
            let gold = Arc::clone(&gold);
            let candidates = Arc::clone(&candidates);
            let next = Arc::clone(&next);
            let sender = sender.clone();
            scope.spawn(move || {
                loop {
                    let index = {
                        let mut next = next.lock().expect("calibration index lock");
                        if *next >= gold.len() {
                            break;
                        }
                        let index = *next;
                        *next += 1;
                        index
                    };
                    let result = run_case(&endpoint, &gold[index], &candidates, index);
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
    });

    let mut results = vec![None; count];
    let mut completed = 0usize;
    for (index, result) in receiver {
        results[index] = Some(result?);
        completed += 1;
        if completed % 8 == 0 || completed == count {
            println!("completed {completed}/{count} cases");
        }
    }
    results
        .into_iter()
        .map(|row| row.ok_or_else(|| "calibration worker omitted a case".into()))
        .collect()
}

fn run_case(
    endpoint: &ConfiguredGeneralEndpoint,
    gold: &Value,
    candidates: &HashMap<String, Value>,
    index: usize,
) -> Result<Value, String> {
    let memory_id = gold["memory_id"]
        .as_str()
        .ok_or_else(|| "gold memory_id missing".to_owned())?;
    let source = candidates
        .get(memory_id)
        .ok_or_else(|| format!("gold Memory missing from candidates corpus: {memory_id}"))?;
    let mut candidate = prepare(gold, source, index)?;
    let raw = Mutex::new(None);
    let capture = CapturingEndpoint {
        inner: endpoint,
        raw: &raw,
    };
    let error = match enrich_entity_calibration(&capture, std::slice::from_mut(&mut candidate)) {
        Ok(()) => None,
        Err(InsomniaExtractionError::InvalidOutput(error)) => Some(error),
        Err(error @ InsomniaExtractionError::Endpoint(_)) => return Err(error.to_string()),
    };
    let raw = raw.lock().expect("calibration capture lock").clone();
    Ok(result_row(gold, &candidate, error.as_deref(), raw))
}

struct CapturingEndpoint<'a> {
    inner: &'a ConfiguredGeneralEndpoint,
    raw: &'a Mutex<Option<Value>>,
}

impl GeneralEndpoint for CapturingEndpoint<'_> {
    fn model(&self) -> &str {
        self.inner.model()
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        let result = self
            .inner
            .complete_json(system_prompt, user_payload, schema_name, schema)?;
        *self.raw.lock().expect("calibration capture lock") = Some(result.clone());
        Ok(result)
    }
}

fn prepare(gold: &Value, source: &Value, index: usize) -> Result<InsomniaCandidate, String> {
    Ok(InsomniaCandidate {
        key: format!("case-{index:03}"),
        authority_kind: "direct".into(),
        category: "fact".into(),
        memory_type: "project".into(),
        temporal_status: "current".into(),
        ownership: if gold["owner_kind"] == "phy" {
            InsomniaOwnership::User
        } else {
            InsomniaOwnership::Project
        },
        title: source["title"]
            .as_str()
            .ok_or("candidate title missing")?
            .into(),
        content: source["content"]
            .as_str()
            .ok_or("candidate content missing")?
            .into(),
        source_node_id: "calibration".into(),
        source_quote: "calibration".into(),
        authority_source_conversation_id: None,
        authority_source_node_id: None,
        authority_source_quote: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        grounding_source_quote: None,
        routing_metadata: None,
    })
}

fn result_row(
    gold: &Value,
    candidate: &InsomniaCandidate,
    error: Option<&str>,
    raw_output: Option<Value>,
) -> Value {
    let mentions = candidate
        .routing_metadata
        .as_ref()
        .map(|routing| {
            routing
                .entity_mentions
                .iter()
                .map(|mention| {
                    json!({
                        "field": match mention.field {
                            MemoryTextField::Title => "title",
                            MemoryTextField::Content => "content",
                        },
                        "start_byte": mention.start_byte,
                        "end_byte": mention.end_byte,
                        "text": mention.text,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "owner_kind": gold["owner_kind"],
        "memory_id": gold["memory_id"],
        "body_id": gold["body_id"],
        "tags": gold["tags"],
        "expected_entity_mentions": gold["expected_entity_mentions"],
        "actual_entity_mentions": mentions,
        "raw_output": raw_output,
        "error": error,
    })
}
