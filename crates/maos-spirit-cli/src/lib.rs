#![forbid(unsafe_code)]

//! `maos-spirit-cli` — Spirit-author publish CLI per FR35 (Story 7.2 v1.0).
//!
//! Produces a Story 5.5d-compatible `SignedPackage` by signing
//! `sha256(manifest_toml || artifact_bytes)` with the publisher's Ed25519
//! key, then dispatches `registry.publish` through `SpiritRegistryClient`.
//! Story 7.3 will extend this CLI to consume the CCAC semantic evaluator.

pub mod compliance_claim;
pub mod errors;
pub mod publish;
pub mod signing;

pub use errors::CliError;
pub use publish::{run_publish, PublishArgs};
