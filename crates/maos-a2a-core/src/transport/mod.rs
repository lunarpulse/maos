//! JSON-RPC over mTLS/TCP transport for A2A cross-Host (FR23b v1.0).
//!
//! Per architecture §7.5: A2A is the substrate's fourth protocol. Story 6.3
//! ships JSON-RPC framing as the wire shape — hand-rolled via `serde_json`
//! per FR47 (no `jsonrpc-core` / `jsonrpsee` / similar). The framing carries
//! the same `IacFrame` shape used on the same-Host bus; same consent
//! envelope; same logical clock; restricted to two endpoints per ADR-003.

pub mod json_rpc;
pub mod logical_clock;
