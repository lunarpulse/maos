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
