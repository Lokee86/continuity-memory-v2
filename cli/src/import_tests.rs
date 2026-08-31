use crate::args::ImportCommand;
use crate::import_cmd;
use reliquary_memory::{Cva, EchoEventKind};
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn graph_jsonl_import_persists_turn_attached_echo_in_source_order() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("reliquary-echo-import-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("graph.jsonl");
    let rel = dir.join("project.prj.rel");
    let records = [
        json!({"kind":"node","id":"u1","conversation_id":"c1","parent_id":null,"role":"user","timestamp_ns":1,"content":"question"}),
        json!({"kind":"node","id":"a1","conversation_id":"c1","parent_id":"u1","role":"assistant","timestamp_ns":2,"content":"answer"}),
        json!({"kind":"echo","conversation_id":"c1","message_id":"a1","sequence":0,"timestamp_ns":2,"model_round":1,"event_kind":"reasoning_trace","content":"raw reasoning"}),
        json!({"kind":"echo","conversation_id":"c1","message_id":"a1","sequence":1,"timestamp_ns":3,"model_round":1,"event_kind":"tool_call","correlation_id":"call-1","name":"memory_search","content":"{\"query\":\"routing\"}"}),
        json!({"kind":"echo","conversation_id":"c1","message_id":"a1","sequence":2,"timestamp_ns":4,"model_round":1,"event_kind":"tool_result","correlation_id":"call-1","name":"memory_search","content":"{\"owners\":[]}"}),
        json!({"kind":"branch","id":"b1","conversation_id":"c1","leaf_node_id":"a1","canonical":true}),
    ];
    let input_text = records
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&input, input_text).unwrap();

    import_cmd::run(ImportCommand::GraphJsonl {
        input,
        cva: rel.clone(),
        turns: 4,
        overlap: 1,
        leave_tail_open: false,
    })
    .unwrap();

    let mut reopened = Cva::open(rel).unwrap();
    let echo = reopened.echo_events("c1", "a1");
    assert_eq!(echo.len(), 3);
    assert_eq!(echo[0].kind, EchoEventKind::ReasoningTrace);
    assert_eq!(echo[0].content, "raw reasoning");
    assert_eq!(echo[1].kind, EchoEventKind::ToolCall);
    assert_eq!(echo[1].correlation_id.as_deref(), Some("call-1"));
    assert_eq!(echo[2].kind, EchoEventKind::ToolResult);
    assert_eq!(reopened.conversation_turns("c1", "a1").unwrap().len(), 2);
}
