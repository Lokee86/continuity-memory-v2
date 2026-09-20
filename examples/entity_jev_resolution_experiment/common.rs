use serde_json::Value;

pub fn norm(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn filtered(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|mention| !generic(mention["text"].as_str().unwrap_or("")))
        .collect()
}

pub fn row_mentions(row: &Value) -> Vec<Value> {
    if row["actual_entity_mentions"].is_array() {
        filtered(&row["actual_entity_mentions"])
    } else {
        filtered(&row["baseline_entity_mentions"])
    }
}

fn generic(text: &str) -> bool {
    let raw = text.trim();
    let lower = raw.to_ascii_lowercase();
    let heads = [
        "flow",
        "lane",
        "mechanism",
        "request",
        "response",
        "seam",
        "state",
        "status",
    ];
    if raw.split_whitespace().count() > 1
        && heads.contains(&lower.split_whitespace().last().unwrap_or(""))
    {
        return true;
    }
    const GENERIC: &[&str] = &[
        "agent",
        "the agent",
        "application",
        "the application",
        "assistant",
        "the assistant",
        "client",
        "the client",
        "code",
        "the code",
        "data",
        "the data",
        "documentation",
        "the documentation",
        "entry",
        "the entry",
        "file",
        "the file",
        "flow",
        "the flow",
        "implementation",
        "the implementation",
        "lane",
        "the lane",
        "pipeline",
        "the pipeline",
        "project",
        "the project",
        "repository",
        "the repository",
        "request",
        "the request",
        "response",
        "the response",
        "script",
        "the script",
        "scripts",
        "the scripts",
        "server",
        "the server",
        "session",
        "the session",
        "shader",
        "the shader",
        "state",
        "the state",
        "status",
        "the status",
        "system",
        "the system",
        "tooling",
        "the tooling",
        "login status",
        "the login status",
        "requested state",
        "the requested state",
        "relevant repository",
        "the relevant repository",
        "existing server",
        "the existing server",
    ];
    (lower.starts_with("the ") || raw == lower) && GENERIC.contains(&lower.as_str())
}
