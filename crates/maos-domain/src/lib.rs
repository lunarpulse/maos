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

pub mod ports;
pub mod frame;
pub mod iac_bus_types;
pub mod lifecycle;       // NEW — Story 5.1 lifecycle types + LifecycleResolver trait
pub mod notification;
pub mod halt;
pub mod hot_swap;
pub mod orchestrator;
pub mod memory;          // NEW — Story 4.3 memory tier types
pub mod self_telemetry;  // NEW — Story 4.3 self-telemetry types
pub mod log_recall;      // NEW — Story 4.4 log-recall domain types
pub mod distillation;    // NEW — Story 4.4 distillation domain types
