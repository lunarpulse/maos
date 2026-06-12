//! Streamable HTTP Transport — single-shot HTTP POST + chunked response.
//!
//! Per the MCP 2025-03 Streamable HTTP binding.  Layered atop
//! `IoSubsystemPort::http_post` with appropriate headers.
//! This is the **default transport** at v0.5-α per ADR-008.

use maos_domain::ports::io_subsystem::IoSubsystemPort;
use maos_domain::ports::mcp::{McpAttribution, McpRequest, McpResponse, McpTransportId};

use super::{McpTransport, McpTransportError};

/// Streamable HTTP transport — DEFAULT at v0.5-α.
pub struct StreamableHttpTransport {
    transport_inner: std::sync::Arc<dyn IoSubsystemPort>,
    endpoint_url: String,
}

impl StreamableHttpTransport {
    pub fn new(transport_inner: std::sync::Arc<dyn IoSubsystemPort>, endpoint_url: String) -> Self {
        Self {
            transport_inner,
            endpoint_url,
        }
    }
}

impl McpTransport for StreamableHttpTransport {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        let body = build_streamable_http_request_body(&request)
            .map_err(|e| McpTransportError::Transport(format!("encode: {e}")))?;

        let headers: &[(&str, &str)] = &[
            ("Content-Type", "application/json"),
            ("Accept", "application/json, text/event-stream"),
        ];

        let resp_body = self
            .transport_inner
            .http_post(&self.endpoint_url, &body, headers)
            .map_err(|e| McpTransportError::Transport(format!("http_post: {e}")))?;

        let resp_str = String::from_utf8(resp_body)
            .map_err(|e| McpTransportError::Transport(format!("utf8: {e}")))?;

        parse_streamable_http_response(&resp_str, &request.server)
    }

    fn id(&self) -> McpTransportId {
        McpTransportId::StreamableHttp
    }
}

/// Build the JSON-RPC 2.0 request body for Streamable HTTP transport.
pub fn build_streamable_http_request_body(
    request: &McpRequest,
) -> Result<Vec<u8>, serde_json::Error> {
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

/// Parse a Streamable HTTP response body into an `McpResponse`.
///
/// Supports two response shapes:
/// - Single-shot JSON (Content-Type: application/json)
/// - SSE-format chunks (Content-Type: text/event-stream)
///
/// For now, we treat the body as a single-shot JSON response.
pub fn parse_streamable_http_response(
    body: &str,
    server_name: &str,
) -> Result<McpResponse, McpTransportError> {
    // Try streamed SSE first
    if !body.trim_start().starts_with('{') && (body.contains("event:") || body.starts_with("data:"))
    {
        return crate::transport::sse::parse_sse_response(body, server_name);
    }

    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| McpTransportError::Transport(format!("http parse: {e}")))?;

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
        McpAttribution::new(server_name.into(), McpTransportId::StreamableHttp, name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_request_body_contains_tool_name() {
        let req = McpRequest::new("srv".into(), "tool1".into(), json!({"a":1}));
        let body = build_streamable_http_request_body(&req).unwrap();
        let s = String::from_utf8(body).unwrap();
        assert!(s.contains("tools/call"));
        assert!(s.contains("tool1"));
    }

    #[test]
    fn parse_single_shot_json_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"ok"}]}}"#;
        let resp = parse_streamable_http_response(body, "test-server").unwrap();
        assert!(!resp.is_error);
        assert_eq!(
            resp.attribution.transport_id,
            McpTransportId::StreamableHttp
        );
    }

    #[test]
    fn parse_error_response() {
        let body =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let err = parse_streamable_http_response(body, "srv").unwrap_err();
        match err {
            McpTransportError::ServerError { code, .. } => assert_eq!(code, -32600),
            other => panic!("expected ServerError, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_format_response_delegates_to_sse_parser() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"name\":\"echo\",\"content\":[]}}\n\n";
        let resp = parse_streamable_http_response(body, "test-server").unwrap();
        assert_eq!(resp.attribution.transport_id, McpTransportId::Sse);
    }
}
