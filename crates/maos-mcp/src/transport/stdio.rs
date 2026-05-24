//! Stdio Transport — subprocess MCP server over stdin/stdout NDJSON.
//!
//! Spawns the configured binary as a child process and exchanges
//! NDJSON frames bidirectionally.  JoinHandle self-prunes per
//! Story 5.4 §1368.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use maos_domain::ports::mcp::{McpAttribution, McpRequest, McpResponse, McpTransportId};

use super::{McpTransport, McpTransportError};

/// Stdio transport — launches the MCP server as a child process and
/// communicates via NDJSON over stdin/stdout.
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
}

impl StdioTransport {
    /// Construct a new stdio transport.
    pub fn new(command: String, args: Vec<String>) -> Result<Self, McpTransportError> {
        // Validate the binary exists by probing spawnability (lazy — actual
        // spawn happens per-invocation; a new design may pre-spawn at
        // construction if needed).
        Ok(Self { command, args })
    }

    fn spawn_and_invoke(
        &self,
        request: &McpRequest,
    ) -> Result<McpResponse, McpTransportError> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| McpTransportError::Transport(format!("spawn failed: {e}")))?;

        let json_request = build_stdio_request_body(request)
            .map_err(|e| McpTransportError::Transport(format!("encode: {e}")))?;

        // Write NDJSON frame to child stdin
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| McpTransportError::Transport("stdin pipe unavailable".into()))?;
            stdin
                .write_all(&json_request)
                .map_err(|e| McpTransportError::Transport(format!("write: {e}")))?;
            // stdin is dropped here → child sees EOF on its stdin for the frame.
        }

        // Read NDJSON line from child stdout
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpTransportError::Transport("stdout pipe unavailable".into()))?;
        let reader = BufReader::new(stdout);
        let response_line = reader
            .lines()
            .next()
            .ok_or_else(|| McpTransportError::Transport("child produced no output".into()))?
            .map_err(|e| McpTransportError::Transport(format!("read: {e}")))?;

        // Wait for child to exit (JoinHandle self-prune on wait)
        let _status = child
            .wait()
            .map_err(|e| McpTransportError::Transport(format!("wait: {e}")))?;

        parse_stdio_response(&response_line, &request.server)
    }
}

impl McpTransport for StdioTransport {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        self.spawn_and_invoke(&request)
    }

    fn id(&self) -> McpTransportId {
        McpTransportId::Stdio
    }
}

/// Build a JSON-RPC 2.0 request body for stdio transport (NDJSON frame).
pub fn build_stdio_request_body(request: &McpRequest) -> Result<Vec<u8>, serde_json::Error> {
    let rpc = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": request.tool,
            "arguments": request.args,
        }
    });
    let mut bytes = serde_json::to_vec(&rpc)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Parse a single NDJSON line from stdio into an `McpResponse`.
pub fn parse_stdio_response(
    line: &str,
    server_name: &str,
) -> Result<McpResponse, McpTransportError> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|e| McpTransportError::Transport(format!("parse: {e}")))?;

    let server = server_name.to_string();
    let tool = value
        .get("result")
        .and_then(|r| r.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    if let Some(err) = super::extract_jsonrpc_error(&value) {
        return Err(err);
    }

    let content = value
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let is_error = content
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(McpResponse::new(
        content,
        is_error,
        McpAttribution::new(server, McpTransportId::Stdio, tool),
    ))
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // StdioTransport is stateless at v0.5-α — the child process is
        // spawned and waited per-invocation.  No persistent child to kill.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_stdio_request_body_produces_ndjson_line() {
        let req = McpRequest::new("test-server".into(), "echo".into(), json!({"msg": "hi"}));
        let body = build_stdio_request_body(&req).unwrap();
        let s = String::from_utf8(body).unwrap();
        assert!(s.contains("tools/call"));
        assert!(s.contains(r#""msg":"hi""#));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn parse_stdio_response_with_result() {
        let resp_line = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"hello"}]}}"#;
        let resp = parse_stdio_response(resp_line, "test-server").unwrap();
        assert!(!resp.is_error);
        assert_eq!(resp.attribution.server_name, "test-server");
        assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    }

    #[test]
    fn parse_stdio_response_with_error() {
        let resp_line =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let err = parse_stdio_response(resp_line, "test-server").unwrap_err();
        assert!(matches!(
            err,
            McpTransportError::ServerError {
                code: -32601,
                ..
            }
        ));
    }

    #[test]
    fn parse_stdio_response_malformed_returns_transport_error() {
        let resp_line = "not json at all";
        let err = parse_stdio_response(resp_line, "srv").unwrap_err();
        assert!(matches!(err, McpTransportError::Transport(_)));
    }
}
