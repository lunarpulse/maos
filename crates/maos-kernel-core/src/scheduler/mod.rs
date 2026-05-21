#![forbid(unsafe_code)]

//! Spirit Scheduler — supervisor / composition root for the four
//! supervised services (Security / Memory / IAC / Capability).
//!
//! Per architecture §4.0.8 supervisor exception, this module satisfies
//! P1 (own crate at v0.5+), P2 (own bin target at v0.5+), and P4
//! (independently restartable) but is exempt from P3 (boundary manifest
//! in the standard shape — its boundary is the union of its children's).
//!
//! Story 5.1 lands the real `SpiritSchedulerAdapter` body replacing the
//! v0.1-β zero-size placeholder. The `LifecycleResolver` trait per
//! architecture §4.0.9 lives in `maos-domain::lifecycle`, not here.

pub use maos_domain::ports::SpiritSchedulerPort;

pub mod control_block;
pub mod scheduler_loop;
pub mod hook_dispatch;
pub mod kernel_ctx;
pub mod idle_watchdog;
pub mod verb_resolver;
pub mod resource_ceiling;

pub use control_block::{SpiritControlBlock, SpiritManifestBundle, AnySpiritObj, make_spirit_obj};
pub use scheduler_loop::{SpiritSchedulerAdapter, pick_next_spirit_from_slice, SCHEDULER_QUANTUM};
pub use hook_dispatch::{HookDispatcher, HookOutcome};
pub use kernel_ctx::KernelCtx;
pub use idle_watchdog::IdleWatchdog;
pub use verb_resolver::KernelLifecycleResolver;
