#![forbid(unsafe_code)]

//! maos-bench — §13.1 rust-inproc measurement gate
//!
//! This crate provides the measurement harness for the §13.1 Spirit-form
//! measurement gate. It measures J1 (founder-loop CliWrapper IPC overhead)
//! and J4 (Mira-Nash Observer colocation latency) using subprocess-form
//! Spirits and produces a data-driven DecisionRecord that gates whether the
//! rust-inproc Spirit form is unlocked at v0.5.
//!
//! ## Crate architecture
//!
//! - `report.rs` — JSON-serializable schemas: `BenchReport`, `JourneyResult`,
//!   `DecisionRecord`, `LatencyHistogram`.
//! - `decision.rs` — pure function `decide()` applying the §13.1 decision rule.
//! - `harness/` — measurement primitives: timer, quantile computation, J1/J4 loops.
//! - `fixture_replay.rs` — canned-latency runner for fast-mode / smoke-arm use.
//! - `benches/section_13_1.rs` — criterion bench entry point.
//! - `src/bin/section_13_1_run.rs` — operator-facing orchestrator binary.
//!
//! ## Safety
//!
//! Every module in this crate follows ADR-039: `#![forbid(unsafe_code)]`.

pub mod decision;
#[cfg(any(test, feature = "fixture_replay"))]
pub mod fixture_replay;
pub mod harness;
pub mod report;
