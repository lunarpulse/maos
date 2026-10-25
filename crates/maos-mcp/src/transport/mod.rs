//! MCP transport abstraction — per-server-URI selectable.
//!
//! Three concrete transports (stdio / SSE / Streamable HTTP) implement
//! the `McpTransport` trait.  All three return the same vendor-neutral
//! `McpResponse` domain type — consumer code is transport-agnostic.
//!
//! ADR-010 sync trait.  Async callers wrap in `spawn_blocking`.

pub mod sse;
pub mod stdio;
pub mod streamable_http;

use maos_domain::ports::mcp::{McpAttribution, McpRequest, McpResponse, McpTransportId};

/// MCP transport — per-server-URI selectable.
///
/// ADR-010 sync trait.  Async callers wrap in `spawn_blocking`.
pub trait McpTransport: Send + Sync {
    /// Class: data-movement
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError>;

    /// Transport identifier — `stdio` / `sse` / `streamable_http`.
    fn id(&self) -> McpTransportId;
}

/// MCP transport-level error.
#[derive(Debug, thiserror::Error)]
pub enum McpTransportError {
    /// Transport-level failure (connection error, subprocess crash, etc.).
    #[error("transport-level error: {0}")]
    Transport(String),

    /// Server returned a JSON-RPC error.
    #[error("server returned error: {message}")]
    ServerError { code: i32, message: String },

    /// Request timed out.
    #[error("timeout after {0}ms")]
    Timeout(u64),

    /// Story 18-8 CR-3 — a non-2xx, non-401 HTTP answer; `body` is the
    /// server's (truncated) and stays out of `Display`.
    #[error("http status {status}")]
    HttpStatus { status: u16, body: Option<String> },

    /// Story 18-8 CR-3 — HTTP 401: the bearer is missing or rejected. Never
    /// carries the bearer.
    #[error("unauthorized (401): bearer missing or rejected")]
    Unauthorized,

    /// Story 18-8 review (BH-2/EC-4/AA-1) — HTTP 400 carrying an MCP-reserved
    /// JSON-RPC code (−32099..=−32020, e.g. `HeaderMismatch`,
    /// `MissingRequiredClientCapability`, `UnsupportedProtocolVersion`): a
    /// modern server's rejection, never a legacy trigger. `body` (truncated)
    /// stays out of `Display`.
    #[error("modern MCP server rejected the request: code {code}")]
    ModernRejection { code: i64, body: String },

    /// Story 18-8 review (BH-1/EC-1/AA-2) — the bearer has a byte outside
    /// visible ASCII (0x21–0x7E): refused before it reaches a header, whose
    /// validation error would echo it. Never carries the value.
    #[error("MCP bearer refused: a byte outside visible ASCII (value withheld)")]
    InvalidBearer,
}

pub fn extract_jsonrpc_error(value: &serde_json::Value) -> Option<McpTransportError> {
    let err = value.get("error")?;
    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) as i32;
    let message = err
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();
    Some(McpTransportError::ServerError { code, message })
}

/// One JSON-RPC answer as an `McpResponse`, shared by the three transports:
/// an `error` is `ServerError`; `result` is the content, `result.name` the
/// attributed tool and `result.isError` the error flag.
pub(crate) fn response_from(
    value: &serde_json::Value,
    server_name: &str,
    transport: McpTransportId,
) -> Result<McpResponse, McpTransportError> {
    if let Some(err) = extract_jsonrpc_error(value) {
        return Err(err);
    }
    let content = value.get("result").cloned().unwrap_or_default();
    let name = content["name"].as_str().unwrap_or("unknown").to_string();
    let is_error = content["isError"].as_bool().unwrap_or(false);
    Ok(McpResponse::new(
        content,
        is_error,
        McpAttribution::new(server_name.into(), transport, name),
    ))
}
