#![forbid(unsafe_code)]

//! Task 1.1 — Round-trip test: MCP fixtures parse through the real driver parsers.
//! Proves the fixture JSON shapes match what the production code consumes.

use maos_domain::ports::mcp::{McpAttribution, McpResponse, McpTransportId};

fn fixture_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn fixture_to_mcp_response(path: &str) -> McpResponse {
    let raw = std::fs::read_to_string(fixture_dir().join(path))
        .unwrap_or_else(|e| panic!("fixture read {path}: {e}"));
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let content = value
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let is_error = content
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    McpResponse::new(
        content,
        is_error,
        McpAttribution::new("test".into(), McpTransportId::StreamableHttp, "test".into()),
    )
}

#[test]
fn butler_calendar_events_fixture_parses() {
    let resp = fixture_to_mcp_response("j-butler/calendar-events.json");
    let content = maos_mcp::drivers::butler::extract_content(&resp).unwrap();
    let events: Vec<butler::CalendarEvent> = serde_json::from_value(content).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, "evt-a");
    assert_eq!(events[0].status, butler::EventStatus::Confirmed);
}

#[test]
fn butler_comms_messages_fixture_parses() {
    let resp = fixture_to_mcp_response("j-butler/comms-messages.json");
    let content = maos_mcp::drivers::butler::extract_content(&resp).unwrap();
    let messages: Vec<butler::CommsMessage> = serde_json::from_value(content).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].from, "colleague");
}

#[test]
fn researcher_web_search_fixture_parses() {
    let resp = fixture_to_mcp_response("j-researcher/web-search.json");
    let content = maos_mcp::drivers::researcher::extract_content(&resp).unwrap();
    let keys = maos_mcp::drivers::researcher::parse_search_results(&content, "web");
    assert_eq!(keys.len(), 2);
    assert!(keys[0].starts_with("https://"));
}

#[test]
fn researcher_arxiv_search_fixture_parses() {
    let resp = fixture_to_mcp_response("j-researcher/arxiv-search.json");
    let content = maos_mcp::drivers::researcher::extract_content(&resp).unwrap();
    let keys = maos_mcp::drivers::researcher::parse_search_results(&content, "arxiv");
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], "2501.00001");
}

#[test]
fn researcher_citation_graph_fixture_parses() {
    let resp = fixture_to_mcp_response("j-researcher/citation-graph.json");
    let content = maos_mcp::drivers::researcher::extract_content(&resp).unwrap();
    let keys = maos_mcp::drivers::researcher::parse_search_results(&content, "citation-graph");
    assert!(keys.len() >= 2);
    assert!(keys.contains(&"2501.00001".to_string()));
}

#[test]
fn researcher_github_search_fixture_parses() {
    let resp = fixture_to_mcp_response("j-researcher/github-search.json");
    let content = maos_mcp::drivers::researcher::extract_content(&resp).unwrap();
    let keys = maos_mcp::drivers::researcher::parse_search_results(&content, "github");
    assert!(!keys.is_empty());
}
