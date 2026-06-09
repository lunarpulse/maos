//! MCP driver helpers — tool-call arg builders + response parsers.
//!
//! These are PURE functions: no async, no capability tokens. Token issuance
//! is the caller's responsibility (`LiveButlerMcpPort` in `maos-bin`).
pub mod butler;
