//! Hexagonal port traits per ADR-010.
//!
//! One trait per supervisor / supervised service / internal module,
//! plus the `crypto` port per FR48 / NFR-Sec-15 / §8.6.
//! Adapter implementations live in `maos-kernel-core::<service>::<Service>Adapter`.
//!
//! # Sync-only trait method signatures
//!
//! Per ADR-010's binding-v0.1 gate "domain core compiles without async
//! runtime", port traits declared here MUST NOT use `async fn` or return
//! `impl Future`. Async behavior — when adapters need it — wraps the
//! sync trait method behind a `Pin<Box<dyn Future>>` or returns a typed
//! handle the adapter caller can `.await`. Story 1b.x lands the actual
//! async behavior; this story declares the sync trait shapes only.
//!
//! # Computational class per method
//!
//! Every public trait method MUST carry a `/// Class: <class>` doc-line
//! immediately above its declaration, where `<class>` is one of:
//!   - `universal-arithmetic` — numeric comparison via the four ADR-022
//!     predicates (`on_value_above` / `_below` / `_within` / `_outside`).
//!   - `data-movement` — moves frames/tokens/payloads between holders;
//!     does no semantic interpretation.
//!   - `supervision` — lifecycle/control over child task or actor;
//!     read/write of a kernel-managed audit log.
//!
//! The `xtask/kernel-api-classes.toml` classifications consume these
//! `/// Class:` doc tags as the source of truth (AC4). A method whose
//! doc lacks a `/// Class:` line, OR carries a class not in the three-element
//! set, defaults to `other` and fails the surface gate.

pub mod a2a; // NEW — Story 6.3 A2ARouter port per ADR-003 + ADR-012
pub mod capability;
pub mod distillation; // NEW — Story 4.4 DistillationPort per AC2
pub mod epistemic_scalar; // NEW — Story 8.10 AC1 cognitive-Spirit scalar-write port
pub mod hot_swap; // NEW — ADR-041 (Epic 6 retro §A3 2026-05-28) Phase-4 extraction prep
pub mod iac_bus;
pub mod inference;
pub mod io_subsystem;
pub mod log_recall; // NEW — Story 4.4 LogRecallPort per AC1
pub mod mcp; // NEW — Story 5.5c McpClientPort + domain types
pub mod memory;
pub mod registry; // NEW — Story 5.5d SpiritRegistryClient port + domain types
pub mod scheduler;
pub mod security;
pub mod self_telemetry; // NEW — Story 4.3 SelfTelemetryPort per FR56
pub mod task;
pub mod telemetry; // NEW — Story 5.3 in-flight task assignment record
pub mod trace_sink; // NEW — Story 9.5b TraceSink port per NFR-Obs-2 / R2-2

pub mod crypto; // NEW — Story 1a.3 CryptoProvider port per FR48 / NFR-Sec-15 / §8.6

pub use a2a::A2ARouter; // NEW — Story 6.3
pub use capability::CapabilityRegistryPort;
pub use crypto::{CryptoError, CryptoProvider};
pub use distillation::DistillationPort; // NEW — Story 4.4
pub use epistemic_scalar::{EpistemicScalarPort, ScalarPortError}; // NEW — Story 8.10 AC1
pub use hot_swap::HotSwapPort; // NEW — ADR-041 Phase-4 extraction prep
pub use iac_bus::IacBusPort;
pub use inference::InferencePort;
pub use io_subsystem::IoSubsystemPort;
pub use log_recall::LogRecallPort; // NEW — Story 4.4
pub use mcp::{McpAttribution, McpClientPort, McpError, McpRequest, McpResponse, McpTransportId}; // NEW — Story 5.5c
pub use memory::MemoryManagerPort;
pub use registry::{
    PublishReceipt, RegistryError, SearchQuery, SearchResultItem, SearchResults, SignedArtifact,
    SignedManifest, SignedPackage, SpiritRegistryClient, YankEntry, YankList, YankReason,
    YankReceipt,
}; // NEW — Story 5.5d
pub use scheduler::SpiritSchedulerPort;
pub use security::SecurityManagerPort;
pub use self_telemetry::SelfTelemetryPort; // NEW — Story 4.3
pub use telemetry::TelemetryStreamPort; // NEW
pub use trace_sink::{
    CapabilitySpanAttrs, HaltSpanAttrs, IacFrameSpanAttrs, SpanContext, SpanGuard, TraceSink,
}; // NEW — Story 9.5b
