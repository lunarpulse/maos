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

use std::{collections::HashSet, hash::Hash, sync::Arc};

use parking_lot::Mutex;

pub use maos_domain::ports::SpiritSchedulerPort;

pub mod control_block;
pub mod hook_dispatch;
pub mod idle_watchdog;
pub mod kernel_ctx;
pub mod schedule_watchdog;
pub mod scheduler_loop;
pub mod verb_resolver;

pub use control_block::{make_spirit_obj, AnySpiritObj, SpiritControlBlock, SpiritManifestBundle};
pub use hook_dispatch::{HookDispatcher, HookOutcome};
pub use idle_watchdog::IdleWatchdog;
pub use kernel_ctx::KernelCtx;
pub use schedule_watchdog::ScheduleWatchdog;
pub use scheduler_loop::{pick_next_spirit_from_slice, SpiritSchedulerAdapter, SCHEDULER_QUANTUM};
pub use verb_resolver::KernelLifecycleResolver;

/// One in-flight watchdog key, held until the last task using its slot ends.
#[maos_attrs::i9_exempt(reason = "bounded firing slot held only while its work is live")]
struct FiringReservation<K: Eq + Hash> {
    set: Arc<Mutex<HashSet<K>>>,
    key: K,
}

impl<K: Clone + Eq + Hash> FiringReservation<K> {
    fn try_reserve(set: &Arc<Mutex<HashSet<K>>>, key: K, max: usize) -> Option<Self> {
        let mut guard = set.lock();
        if guard.len() >= max || !guard.insert(key.clone()) {
            return None;
        }
        drop(guard);
        Some(Self {
            set: Arc::clone(set),
            key,
        })
    }
}

impl<K: Eq + Hash> Drop for FiringReservation<K> {
    fn drop(&mut self) {
        self.set.lock().remove(&self.key);
    }
}
