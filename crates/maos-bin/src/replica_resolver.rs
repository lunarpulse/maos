//! Story 19-6 (D-19-6-05 A) — the production FR50 replica resolver.
//!
//! The root installs ONE [`RootReplicaResolver`] and hands the same `Arc` to
//! the `CrashDetector`, the scheduler and the Worker supervisor's executor, so
//! every disposition path asks the same question of the same state.
//!
//! **What the kernel row means (rule 11).** `enforce_disposition` resolves by
//! the task's `IntentClass` string, which selects nothing for a Worker record
//! (every one is `"standard"`), and on `Some(_)` it writes `task.reassigned`
//! with `replica_spirit_id` = the task's ORIGINATOR. That is the truth under
//! D-19-6-04 A: there is no standing replica pool — the J1 root, through the
//! Orchestrator that originated the task, re-dispatches the section once to a
//! fresh Worker from the same entry manifest. The fresh Worker's identity is
//! in the new `task.assign` row and its bind rows, not in `task.reassigned`.
//!
//! So the resolver is armed by ROOT STATE, not by its key: unarmed it answers
//! `None` (the kernel falls through to `task.escalated`); armed around exactly
//! one delegated Worker run it answers the originator's pid once, then `None`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_domain::supervision::ReplicaResolver;

/// The root's resolver. Pid 0 (the kernel's own) is never a Spirit pid, so it
/// encodes "unarmed".
#[derive(Debug, Default)]
pub struct RootReplicaResolver {
    armed_pid: AtomicU32,
}

impl RootReplicaResolver {
    /// An unarmed resolver.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Arm for ONE Worker run whose manifest declares `reassign-to-replica`,
    /// with the pid of the originator that will re-dispatch. The guard
    /// disarms on every exit path (drop); `originator_pid == 0` arms nothing.
    pub fn arm(&self, originator_pid: u32) -> ReplicaArm<'_> {
        self.armed_pid.store(originator_pid, Ordering::SeqCst);
        ReplicaArm {
            resolver: self,
            armed: originator_pid != 0,
        }
    }
}

impl ReplicaResolver for RootReplicaResolver {
    /// Budget one: the first call while armed takes the pid; every later call,
    /// and every call while unarmed, is `None`. The key is ignored (module doc).
    fn find_replica(&self, _intent_class: &str) -> Option<u32> {
        Some(self.armed_pid.swap(0, Ordering::SeqCst)).filter(|pid| *pid != 0)
    }
}

/// One armed run. Dropping it disarms the resolver.
#[derive(Debug)]
#[must_use = "dropping the guard disarms the resolver at once"]
pub struct ReplicaArm<'a> {
    resolver: &'a RootReplicaResolver,
    armed: bool,
}

impl ReplicaArm<'_> {
    /// `true` once the kernel has taken the budget — i.e. a `task.reassigned`
    /// row was written for this run and the originator owes the re-dispatch.
    pub fn spent(&self) -> bool {
        self.armed && self.resolver.armed_pid.load(Ordering::SeqCst) == 0
    }
}

impl Drop for ReplicaArm<'_> {
    fn drop(&mut self) {
        self.resolver.armed_pid.store(0, Ordering::SeqCst);
    }
}
