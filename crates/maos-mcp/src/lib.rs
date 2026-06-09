#![forbid(unsafe_code)]

//! `maos-mcp` — Model Context Protocol client (ADR-008).
//!
//! Three transports (stdio / SSE / Streamable HTTP) per MCP 2024-11-05 +
//! 2025-03 bindings; Streamable HTTP is the v0.5-α default.
//!
//! Consumer-facing surface: `McpClient::call(server, tool, args)`.
//! Per-server transport selection is operator-configurable via the
//! `[mcp].servers[i].transport` manifest field.
//!
//! The kernel-side capability-mediation adapter lives in
//! `maos-kernel-core::mcp::McpClientAdapter` — this crate provides ONLY
//! the wire-protocol implementation.  No capability tokens are checked here.

pub mod client;
#[cfg(any(test, feature = "fixture_replay"))]
pub mod fixture_replay;
pub mod transport;
pub mod drivers;

// Re-export the trait (primary port abstraction) and the concrete impl.
pub use client::{McpClientImpl, McpServerEntry};
pub use transport::{McpTransport, McpTransportError};

/// Story 7.2 (closes 5.5d Medium #23) — `McpClient` trait abstraction.
///
/// The concrete `client::McpClientImpl` struct gets a blanket impl of
/// `McpClient` so all existing call sites keep working; consumers should
/// migrate to `Arc<dyn McpClient + Send + Sync>` for new code. The
/// minimal trait surface lifts ONLY the `call` method — wider MCP port
/// refactor (streaming, batching, server-side abstractions) is v0.7+ scope
/// per the Story 5.5d remediation #21 deferral.
pub trait McpClient: Send + Sync {
    fn call(
        &self,
        server_name: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<maos_domain::ports::mcp::McpCallResponse, maos_domain::ports::mcp::McpError>;
}

impl McpClient for client::McpClientImpl {
    fn call(
        &self,
        server_name: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<maos_domain::ports::mcp::McpCallResponse, maos_domain::ports::mcp::McpError> {
        client::McpClientImpl::call(self, server_name, tool, args)
    }
}
