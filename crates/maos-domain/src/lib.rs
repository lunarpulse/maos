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

pub mod invariants;
pub use invariants::*;

pub mod distillation; // NEW — Story 4.4 distillation domain types
pub mod frame;
pub mod halt;
pub mod hot_swap;
pub mod iac_bus_types;
pub mod lifecycle; // NEW — Story 5.1 lifecycle types + LifecycleResolver trait
pub mod log_recall; // NEW — Story 4.4 log-recall domain types
pub mod memory; // NEW — Story 4.3 memory tier types
pub mod notification;
pub mod orchestrator;
pub mod ports;
pub mod revocation; // NEW — Story 5.4 CRL types + RegistryClient trait
pub mod sandbox; // NEW — Story 5.5a T3 container isolation domain types
pub mod self_telemetry; // NEW — Story 4.3 self-telemetry types
pub mod supervision; // NEW — Story 5.3 crash / hang / silent-failure detection
