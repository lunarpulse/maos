#![forbid(unsafe_code)]

//! Cross-Spirit isolation corpus runner — NFR-Sec-14 enforcement.
//!
//! Story 4.5 — hosts the 200-scenario adversarial CI gate.
//! Architecture §8.1 + ADR-040; depends on Story 2.4 framework hooks +
//! Story 4.3 MemoryManagerAdapter isolation wiring + Story 4.4
//! LogRecallAdapter isolation wiring.
//!
//! The `isolation/` directory is parallel-shaped to `halt/` and
//! `orchestrator/`, reflecting the architectural fact that cross-Spirit
//! isolation enforcement is a cross-cutting kernel surface.

pub mod runner;

pub use runner::{
    IsolationCorpusRunner, IsolationCorpusReport, IsolationCorpusError,
    ScenarioOutcome,
};
