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

pub use client::{McpClient, McpServerEntry};
pub use transport::{McpTransport, McpTransportError};
