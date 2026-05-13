#![forbid(unsafe_code)]

//! Surface-classification anchor for NFR-Test-2.
//!
//! Per Epic 0 retro: "Story 1a.2's `pub mod api` lands → MUST add
//! classifications same-PR or surface-diff rejects". This module
//! re-exports the seven adapter types so the xtask
//! `check-service-boundary` walk produces a stable, classifiable surface
//! at `maos_kernel_core::api::*`. Adding a new public re-export here
//! requires matching it with a classification entry in
//! `xtask/kernel-api-classes.toml` per AC4.

pub use crate::scheduler::SpiritSchedulerAdapter;
pub use crate::security::SecurityManagerAdapter;
pub use crate::security::RingCryptoProvider;  // NEW — Story 1a.3 default crypto provider
pub use crate::memory::MemoryManagerAdapter;
pub use crate::iac::IacBusAdapter;
pub use crate::capability::CapabilityRegistryAdapter;
pub use crate::io::IoSubsystemAdapter;
pub use crate::telemetry::TelemetryStreamAdapter;
