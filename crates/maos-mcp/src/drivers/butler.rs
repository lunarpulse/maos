//! Pure arg builders and response extractors for Butler-facing MCP tool calls.
//!
//! Every function in this module is synchronous and side-effect free.
//! The caller (typically `LiveButlerMcpPort`) is responsible for issuing
//! capability tokens and performing the final `serde_json::from_value`
//! into butler domain types.

use maos_domain::ports::mcp::{McpError, McpResponse};

// ---------------------------------------------------------------------------
// Arg builders — each returns the JSON args object for a specific MCP tool.
// ---------------------------------------------------------------------------

/// Args for `calendar.list_events` — no parameters required.
pub fn calendar_list_events_args() -> serde_json::Value {
    serde_json::json!({})
}

/// Args for `slack.list_messages` — no parameters required.
pub fn slack_list_messages_args() -> serde_json::Value {
    serde_json::json!({})
}

/// Args for `linear.create_issue`.
pub fn linear_create_issue_args(title: &str, content: &str) -> serde_json::Value {
    serde_json::json!({
        "title": title,
        "description": content,
    })
}

/// Args for `figma.get_file` — no parameters required.
pub fn figma_get_file_args() -> serde_json::Value {
    serde_json::json!({})
}

// ---------------------------------------------------------------------------
// Response extractor
// ---------------------------------------------------------------------------

/// Extract the `content` field from an MCP response.
///
/// Returns `Err(McpError::Decode)` when the server signalled an error
/// (i.e. `response.is_error` is true).
pub fn extract_content(response: &McpResponse) -> Result<serde_json::Value, McpError> {
    if response.is_error {
        return Err(McpError::Decode(format!(
            "MCP server returned error: {}",
            response.content
        )));
    }
    Ok(response.content.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::mcp::{McpAttribution, McpError, McpResponse};

    fn fake_response(content: &str, is_error: bool) -> McpResponse {
        McpResponse::new(
            serde_json::from_str(content).unwrap(),
            is_error,
            McpAttribution::new(
                "test".into(),
                maos_domain::ports::mcp::McpTransportId::Stdio,
                "test".into(),
            ),
        )
    }

    #[test]
    fn arg_builders_return_valid_json() {
        assert!(calendar_list_events_args().as_object().unwrap().is_empty());
        assert!(slack_list_messages_args().as_object().unwrap().is_empty());
        assert!(figma_get_file_args().as_object().unwrap().is_empty());

        let args = linear_create_issue_args("t", "c");
        let obj = args.as_object().unwrap();
        assert_eq!(obj["title"].as_str(), Some("t"));
        assert_eq!(obj["description"].as_str(), Some("c"));
    }

    #[test]
    fn extract_content_ok() {
        let resp = fake_response(r#"{"events": []}"#, false);
        let content = extract_content(&resp).unwrap();
        assert!(content["events"].as_array().unwrap().is_empty());
    }

    #[test]
    fn extract_content_error_yields_decode() {
        let resp = fake_response(r#"{"message":"boom"}"#, true);
        let err = extract_content(&resp).unwrap_err();
        match err {
            McpError::Decode(msg) => assert!(msg.contains("boom")),
            other => panic!("expected Decode, got {other:?}"),
        }
    }
}
