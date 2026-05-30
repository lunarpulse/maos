//! `maos-registry` — Spirit Registry v0.5-α.
//!
//! The MAOS Spirit Registry substrate ships both the **server side**
//! (`SpiritRegistryServer` — an MCP-Streamable-HTTP server exposing
//! `registry.search` / `registry.manifest` / `registry.artifact` /
//! `registry.publish` / `registry.deprecate` tools) and the **client
//! side** (`McpSpiritRegistryClient` — kernel-side client implementing
//! `SpiritRegistryClient` and routing through Story 5.5c's `McpClient::call`).
//!
//! # v0.5-α scope
//!
//! - 5 MCP-Streamable-HTTP operations per ADR-008 binding-v0.5
//! - Three-trust-tier strictest-of-floor admission per ADR-009 binding-v0.5
//! - Structural-only ComplianceClaim verification (Ed25519 signature + fingerprint hash match)
//! - Single-threaded TCP listener on 127.0.0.1 (HTTP only per `SECURITY.md`)
//! - Content-addressed filesystem storage at `~/.local/share/maos/registry/`
//! - Yank propagation via the 5-min polling loop (distinct from FR13 CRL)
//!
//! # Surface Stability Contract
//!
//! - `SpiritRegistryClient` trait — STABLE at v0.5-α. Story 7.2 ADDS methods.
//! - `SignedPackage` wire shape — STABLE at v0.5-α. Story 7.2 may add fields.
//! - `registry.<op>` JSON-RPC method names — STABLE at v0.5-α.
//! - `FrameKind::SpiritAdmitted` / `FrameKind::RegistryYank` — STABLE once assigned.

pub mod admission;
pub mod client;
pub mod compliance_verify;
pub mod fixture_replay;
pub mod handlers;
pub mod import;
pub mod operations;
pub mod origin;
pub mod server;
pub mod storage;
pub mod yank;

pub use client::McpSpiritRegistryClient;
pub use operations::RegistryOperation;
pub use server::SpiritRegistryServer;
pub use maos_domain::ports::registry::TrustTier;
