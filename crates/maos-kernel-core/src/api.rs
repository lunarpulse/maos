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
pub use crate::iac::TransparencyLogAdapter;   // NEW — Story 1b.1 Transparency Log audit-spine
pub use crate::journal::JournalAdapter;       // NEW — Story 1b.1 Lifecycle Journal per I10
pub use crate::capability::CapabilityRegistryAdapter;
pub use crate::io::IoSubsystemAdapter;
pub use crate::telemetry::TelemetryStreamAdapter;
// Story 4.2 — Working Memory scalar slot + predicate exports
pub use crate::capability::WorkingMemorySlot;
pub use crate::capability::SetScalarError;
pub use crate::capability::WorkingMemoryStore;
pub use crate::security::manifest::ScalarPredicate;
pub use crate::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;

// Story 4.3 — Memory Manager three tiers + Principal Namespace + Self-Telemetry
pub use crate::memory::private::PrivateMemoryStore;
pub use crate::memory::shared::SharedMemoryStore;
pub use crate::memory::principal::PrincipalNamespaceIndex;
pub use crate::memory::self_telemetry::SelfTelemetryAggregator;
pub use crate::memory::for_spirit::SpiritMemoryView;

// Story 4.4 — Log-recall + Distillate audit chain
pub use crate::iac::log_recall::LogRecallAdapter;
pub use crate::iac::distillate::DistillateWriter;

// Story 5.2 — Hot-Swap Coordinator
pub use crate::hot_swap::HotSwapCoordinator;
