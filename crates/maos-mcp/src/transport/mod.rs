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

use maos_domain::ports::mcp::{McpRequest, McpResponse, McpTransportId};

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
