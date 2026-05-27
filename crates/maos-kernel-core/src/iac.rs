#![forbid(unsafe_code)]

//! Story 6.5 Phase-1 backward-compat shim — IAC substrate moved to `maos-iac`.
//!
//! The IAC substrate moved to `maos-iac` per kloc.toml Phase-1 decomposition.
//! This shim preserves `crate::iac::...` import paths inside maos-kernel-core
//! so the rest of the crate compiles unchanged.
//!
//! Per Story 6.5 AC2: all 13 files extracted; this shim provides the
//! backward-compatible re-export surface while callers migrate to
//! `maos_iac::...` paths.

pub use maos_iac::*;

// Story 6.5 — trait bridge: kernel-core implements SpiritActivityTracker via
// a local wrapper struct so the Mailbox (now in maos-iac) can update timestamps
// without coupling to kernel-core types. The wrapper struct is local to
// maos-kernel-core, satisfying the orphan rule.
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use maos_iac::mailbox::SpiritActivityTracker;

/// Local wrapper around the SCB map to implement SpiritActivityTracker.
/// This wrapper is needed because `RwLock<BTreeMap<...>>` is from std
/// and the trait is from maos-iac — neither is local to maos-kernel-core
/// without a wrapper struct.
pub struct ScbTracker {
    inner: Arc<RwLock<BTreeMap<u32, Arc<crate::scheduler::control_block::SpiritControlBlock>>>>,
}

impl ScbTracker {
    pub fn new(
        inner: Arc<RwLock<BTreeMap<u32, Arc<crate::scheduler::control_block::SpiritControlBlock>>>>,
    ) -> Self {
        Self { inner }
    }
}

impl SpiritActivityTracker for ScbTracker {
    fn update_last_inbound_frame(
        &self,
        spirit_id: &str,
        timestamp_ns: u64,
    ) {
        use std::sync::atomic::Ordering;
        if let Ok(scbs) = self.inner.read() {
            for (_, scb) in scbs.iter() {
                if scb.spirit_id == spirit_id {
                    scb.last_inbound_frame_ns.store(timestamp_ns, Ordering::Relaxed);
                }
            }
        }
    }

    fn update_last_progress_iac(
        &self,
        spirit_id: &str,
        timestamp_ns: u64,
    ) {
        use std::sync::atomic::Ordering;
        if let Ok(scbs) = self.inner.read() {
            for (_, scb) in scbs.iter() {
                if scb.spirit_id == spirit_id {
                    scb.last_progress_iac_ns.store(timestamp_ns, Ordering::Relaxed);
                }
            }
        }
    }
}
