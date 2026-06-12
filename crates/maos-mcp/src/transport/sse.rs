//! SSE Transport — HTTP + SSE long-lived connection.
//!
//! Layered atop `IoSubsystemPort::http_get` with `Accept: text/event-stream`
//! per the MCP 2024-11-05 SSE binding.

use maos_domain::ports::io_subsystem::IoSubsystemPort;
use maos_domain::ports::mcp::{McpAttribution, McpRequest, McpResponse, McpTransportId};

use super::{McpTransport, McpTransportError};

/// SSE transport — long-lived HTTP connection with SSE event parsing.
pub struct SseTransport {
    transport_inner: std::sync::Arc<dyn IoSubsystemPort>,
    endpoint_url: String,
}

impl SseTransport {
    pub fn new(transport_inner: std::sync::Arc<dyn IoSubsystemPort>, endpoint_url: String) -> Self {
        Self {
            transport_inner,
            endpoint_url,
        }
    }
}

impl McpTransport for SseTransport {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        let resp_body = self
            .transport_inner
            .http_get(&self.endpoint_url)
            .map_err(|e| McpTransportError::Transport(format!("http_get: {e}")))?;

        let resp_str = String::from_utf8(resp_body)
            .map_err(|e| McpTransportError::Transport(format!("utf8: {e}")))?;

        parse_sse_response(&resp_str, &request.server)
    }

    fn id(&self) -> McpTransportId {
        McpTransportId::Sse
    }
}

/// Build the JSON-RPC request body for SSE transport.
pub fn build_sse_request_body(request: &McpRequest) -> Result<Vec<u8>, serde_json::Error> {
    let rpc = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": request.tool,
            "arguments": request.args,
        }
    });
    serde_json::to_vec(&rpc)
}

/// Parse SSE event stream into a single aggregated `McpResponse`.
///
/// Per the MCP SSE binding, each `data:` line is a JSON payload.
/// For v0.5-α single-shot responses, we return the first non-empty data line.
pub fn parse_sse_response(body: &str, server_name: &str) -> Result<McpResponse, McpTransportError> {
    let mut last_data: Option<String> = None;

    for line in body.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() {
                last_data = Some(data.to_string());
            }
        }
    }

    let data = last_data
        .ok_or_else(|| McpTransportError::Transport("no data events in SSE stream".into()))?;

    let value: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| McpTransportError::Transport(format!("sse parse: {e}")))?;

    if let Some(err) = super::extract_jsonrpc_error(&value) {
        return Err(err);
    }

    let content = value
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let name = content
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();
    let is_error = content
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(McpResponse::new(
        content,
        is_error,
        McpAttribution::new(server_name.into(), McpTransportId::Sse, name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_single_event() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"name\":\"echo\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}\n\n";
        let resp = parse_sse_response(body, "test-server").unwrap();
        assert!(!resp.is_error);
        assert_eq!(resp.attribution.server_name, "test-server");
        assert_eq!(resp.attribution.transport_id, McpTransportId::Sse);
    }

    #[test]
    fn parse_sse_error_event() {
        let body =
            "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32601,\"message\":\"unknown tool\"}}\n\n";
        let err = parse_sse_response(body, "srv").unwrap_err();
        match err {
            McpTransportError::ServerError { code, .. } => assert_eq!(code, -32601),
            other => panic!("expected ServerError, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_no_data_events() {
        let body = "event: ping\ndata:\n\n";
        let err = parse_sse_response(body, "srv").unwrap_err();
        assert!(matches!(err, McpTransportError::Transport(_)));
    }
}
