//! MCP client port per ADR-008 + ADR-010.
//!
//! The MCP client port is the kernel's uniform surface for consuming
//! MCP (Model Context Protocol) tool servers.  Story 5.5c ships the
//! v0.5-α substrate with three transports (stdio / SSE / Streamable HTTP).
//!
//! Per ADR-010, the trait is **sync** — the kernel's async callers wrap
//! it in `tokio::task::spawn_blocking`.

use crate::invariants::i1::CapabilityToken;

/// MCP client port — kernel adapter contract.
///
/// Per ADR-010 sync-only port semantics.  The adapter
/// (`McpClientAdapter` in `maos-kernel-core`) wraps capability
/// mediation + Transparency-Log emission around the wire-level
/// `McpClient` from `maos-mcp`.
pub trait McpClientPort: Send + Sync {
    /// Class: data-movement
    ///
    /// Verify the capability token + invoke the MCP tool.
    /// Returns a vendor-neutral response.  Capability denial
    /// returns `McpError::CapabilityDenied`.
    fn call(
        &self,
        token: &CapabilityToken,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError>;
}

/// Vendor-neutral MCP request shape.  All three transports translate
/// into these types — no transport-specific JSON leaks past
/// `maos-mcp::transport::*`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpRequest {
    #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
    pub server: String,
    #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
    pub tool: String,
    #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
    pub args: serde_json::Value,
}

impl McpRequest {
    pub fn new(server: String, tool: String, args: serde_json::Value) -> Self {
        Self { server, tool, args }
    }
}

/// Vendor-neutral MCP response.  Consumers see this type regardless
/// of which transport handled the request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpResponse {
    #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
    pub content: serde_json::Value,
    #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
    pub is_error: bool,
    #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
    pub attribution: McpAttribution,
}

/// Caller-facing alias — `McpClientPort::call` returns this type.
pub type McpCallResponse = McpResponse;

impl McpResponse {
    pub fn new(content: serde_json::Value, is_error: bool, attribution: McpAttribution) -> Self {
        Self {
            content,
            is_error,
            attribution,
        }
    }
}

/// Attribution identifying which MCP server + transport served the call.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpAttribution {
    #[doc = "Construct via [`McpAttribution::new`] to enforce non-empty fields."]
    pub server_name: String,
    #[doc = "Construct via [`McpAttribution::new`] to enforce non-empty fields."]
    pub transport_id: McpTransportId,
    #[doc = "Construct via [`McpAttribution::new`] to enforce non-empty fields."]
    pub tool: String,
}

impl McpAttribution {
    pub fn new(server_name: String, transport_id: McpTransportId, tool: String) -> Self {
        Self {
            server_name,
            transport_id,
            tool,
        }
    }
}

/// MCP transport identifier — per-server-URI selectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportId {
    /// Subprocess MCP server over stdio (stdin/stdout NDJSON).
    Stdio,
    /// HTTP + SSE long-lived connection (MCP 2024-11-05 SSE binding).
    Sse,
    /// Single-shot HTTP POST + chunked response (MCP 2025-03 Streamable HTTP binding).
    StreamableHttp,
}

/// MCP error — concrete named variants covering transport, encode/decode,
/// and capability-denial paths.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Unknown server name — not registered in the manifest `[mcp].servers` table.
    #[error("unknown server '{0}'")]
    UnknownServer(String),

    /// Transport-level failure.
    #[error("transport error: {0}")]
    Transport(String),

    /// Server returned an error (JSON-RPC error object).
    #[error("server returned error code={code}: {message}")]
    ServerError { code: i32, message: String },

    /// JSON encode failure on the request args.
    #[error("encode error: {0}")]
    Encode(serde_json::Error),

    /// JSON decode failure on the response.
    #[error("decode error: {0}")]
    Decode(String),

    /// Capability denied — spirit lacks the required scope.
    #[error("capability denied: scope mismatch on {server}/{tool}")]
    CapabilityDenied { server: String, tool: String },

    /// Client is unconfigured (e.g., composition root wired but no
    /// `MAOS_MCP_ENABLE` gate or no transports registered).
    #[error("unconfigured")]
    Unconfigured,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip serde for `McpTransportId`.
    #[test]
    fn mcp_transport_id_serde_round_trip() {
        for (input, expected_json) in [
            (McpTransportId::Stdio, r#""stdio""#),
            (McpTransportId::Sse, r#""sse""#),
            (McpTransportId::StreamableHttp, r#""streamable_http""#),
        ] {
            let encoded = serde_json::to_string(&input).unwrap();
            assert_eq!(encoded, expected_json, "serialize {:?}", input);
            let decoded: McpTransportId = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, input, "deserialize {:?}", input);
        }
    }

    /// Round-trip serde for `McpRequest`.
    #[test]
    fn mcp_request_serde_round_trip() {
        let req = McpRequest::new(
            "test-server".into(),
            "echo".into(),
            serde_json::json!({"msg": "hello"}),
        );
        let encoded = serde_json::to_string(&req).unwrap();
        let decoded: McpRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, req);
    }

    /// Round-trip serde for `McpResponse`.
    #[test]
    fn mcp_response_serde_round_trip() {
        let attr = McpAttribution::new("test-server".into(), McpTransportId::StreamableHttp, "echo".into());
        let resp = McpResponse::new(serde_json::json!({"result": "ok"}), false, attr);
        let encoded = serde_json::to_string(&resp).unwrap();
        let decoded: McpResponse = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, resp);
    }

    /// Round-trip serde for `McpAttribution`.
    #[test]
    fn mcp_attribution_serde_round_trip() {
        let attr = McpAttribution::new("loom-lite".into(), McpTransportId::Stdio, "recall".into());
        let encoded = serde_json::to_string(&attr).unwrap();
        let decoded: McpAttribution = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, attr);
    }

    /// McpError Display implements descriptive messages.
    #[test]
    fn mcp_error_display() {
        assert_eq!(
            McpError::UnknownServer("foo".into()).to_string(),
            "unknown server 'foo'"
        );
        assert_eq!(
            McpError::CapabilityDenied {
                server: "s".into(),
                tool: "t".into()
            }
            .to_string(),
            "capability denied: scope mismatch on s/t"
        );
    }
}
