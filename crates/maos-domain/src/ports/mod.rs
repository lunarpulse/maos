//! Hexagonal port traits per ADR-010.
//!
//! One trait per supervisor / supervised service / internal module.
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

pub mod scheduler;
pub mod security;
pub mod memory;
pub mod iac_bus;
pub mod capability;
pub mod io_subsystem;
pub mod telemetry;

pub use scheduler::SpiritSchedulerPort;
pub use security::SecurityManagerPort;
pub use memory::MemoryManagerPort;
pub use iac_bus::IacBusPort;
pub use capability::CapabilityRegistryPort;
pub use io_subsystem::IoSubsystemPort;
pub use telemetry::TelemetryStreamPort;
