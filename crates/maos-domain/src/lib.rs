#![forbid(unsafe_code)]

//! `maos-domain` — pure types, invariants, and pure functions (ADR-010).
//!
//! The domain core compiles without any async runtime. It codifies invariants
//! I1–I14 as zero-async-dependency Rust types with doctested invariant
//! statements. See `invariants` module for the per-invariant type codifications.
//!
//! # Zero-async-dependency guarantee
//!
//! - No `tokio`, `reqwest`, `sqlx`, `async-std`, `smol`, or `futures`.
//! - Only `serde` (derive) and `thiserror` are declared as dependencies.

pub mod audit_key;
pub mod invariants; // NEW — Story 9.1 FR44 audit signing key loader (Decision B)
pub use invariants::*;

pub mod cli_wrapper; // NEW — Story 6.2 AC5 CliWrapperSpirit admission errors
pub mod cost; // NEW — Story 9.3b FR64 cost attribution (ADR-046)
pub mod distillation; // NEW — Story 4.4 distillation domain types
pub mod frame;
pub mod governance; // NEW — Story 9.3b FR62 governance audit artifacts (ADR-045)
pub mod halt;
pub mod host_grant; // NEW — Story 8.12 AC5 host-side capability/tier grant allowlist (FORK A)
pub mod hot_swap;
pub mod iac_bus_types;
pub mod lifecycle; // NEW — Story 5.1 lifecycle types + LifecycleResolver trait
pub mod log_recall; // NEW — Story 4.4 log-recall domain types
pub mod memory; // NEW — Story 4.3 memory tier types
pub mod notification;
pub mod orchestrator;
pub mod ports;
pub mod provenance; // NEW — Story 9.4b AC-6 model-provenance admission error taxonomy (D5/D6)
pub mod region; // NEW — Story 9.4b AC-5/AC-12 region-pinning jurisdiction label
pub mod reserved_namespaces; // NEW — Story 9.4b AC-7 multi-operator tenancy reservation (D8)
pub mod revocation; // NEW — Story 5.4 CRL types + RegistryClient trait
pub mod sandbox; // NEW — Story 5.5a T3 container isolation domain types
pub mod self_telemetry; // NEW — Story 4.3 self-telemetry types
pub mod supervision; // NEW — Story 5.3 crash / hang / silent-failure detection
pub mod team; // Story 13.1 — shared canonical tenant identity (ADR-055)
