#![forbid(unsafe_code)]

//! MAOS OpenTelemetry SLO-class adapter (Story 9.5b / NFR-Obs-2).
//!
//! Implements [`TraceSink`] over the OpenTelemetry SDK.  Ships as a
//! **separate non-kernel-core crate** (W1) so `opentelemetry` is
//! structurally absent from the kernel-core dependency tree.
//!
//! # Design invariants
//!
//! - **Off by default** — the composition root must explicitly install a
//!   `TraceSink` for spans to be emitted (AC-2).
//! - **No default endpoint** — absent an explicit collector config, the
//!   adapter does NOT initiate outbound connections (M3 / AC-4).
//! - **Bounded emit pipeline** — a bounded `mpsc` channel with `try_send`;
//!   on full → increment drop counter, return immediately (AC-3).
//! - **Zero principal nexus** in any span attribute (AC-5 / R2-5).
//!
//! # Terminology
//!
//! **"SLO-class" = trace tier** (architecture §4.4).  The metric SLO
//! substrate is the `iac_rt_duration_us` histogram anchored on the
//! 1500µs SLO with PromQL alerts (arch §4.7.1 / §13.1, Epic 1b /
//! Telemetry Stream).  This crate is the complementary **trace path**:
//! structured trace IDs and span linkage per IAC frame, per capability
//! invocation, per halt event.
//!
//! # Deferred / out-of-scope
//!
//! - **SIEM export** — deferred to **v2.0** (NFR-Aud-11 second phase).
//!   Not implemented; the `TraceSink` trait is extensible for future
//!   SIEM-oriented event streams.
//! - **Real-collector backpressure under production load** — NOT
//!   CI-covered.  The bounded-queue test (`gate:otel-degradation`) is
//!   the structural guarantee that protects the kernel regardless.
//!   Document as a known limitation.
//! - **Periodic criterion bench** — absent-sink overhead ceiling is
//!   periodic/non-blocking; per-commit timing gates are
//!   flaky/tautological.

mod otel_sink;
mod schema;

pub use otel_sink::{BoundedExportProbe, OtelTraceSink, OtelTraceSinkConfig};
pub use schema::SPAN_SCHEMA;
