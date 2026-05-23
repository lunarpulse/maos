//! T3 quarantine structural seam.
//!
//! Story 5.4's `RevocationAction::Quarantine` calls this function when an
//! in-process Spirit needs to be re-spawned into a T3 container. At v0.5-α
//! this is deferred because in-process Spirits cannot be re-spawned into a
//! container without the subprocess wire protocol from Epic 6.
//!
//! The function is **wired with a deferred-activation flag**:
//! - v0.5-α: returns `Err(T3Error::QuarantineRequiresSubprocessForm)`
//!   with a documented "wired for Epic 6" rationale.
//! - Epic 6+: activates the real quarantine→T3 path with zero kernel-core
//!   changes.
//!
//! The revocation applier's `Quarantine` arm (at
//! `crates/maos-kernel-core/src/revocation/applier.rs`) calls this function;
//! on `Err(QuarantineRequiresSubprocessForm)`, it falls back to
//! drain-then-terminate with a documented audit marker.

use maos_domain::invariants::i9::SandboxTier;
use maos_domain::sandbox::T3Error;
use maos_domain::ports::scheduler::SpiritSchedulerPort;

use crate::capability::cap_audit::{self, CapAuditEvent};

pub fn quarantine_spirit(
    scheduler: &dyn SpiritSchedulerPort,
    pid: u32,
    target_tier: SandboxTier,
    audit_sender: Option<&cap_audit::Sender>,
) -> Result<QuarantineReport, T3Error> {
    let spirit_id = format!("spirit-pid-{pid}");

    let _ = scheduler.journal_lifecycle(maos_domain::invariants::i10::JournalEntry::Lifecycle(
        maos_domain::invariants::i10::LifecycleEntry {
            timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
            lifecycle_event: maos_domain::invariants::i10::LifecycleEvent::SandboxApplied,
            spirit_id: spirit_id.clone(),
            effective_sandbox_tier: Some(target_tier),
        },
    ));

    if let Some(sender) = audit_sender {
        let event = CapAuditEvent::SandboxBlock {
            spirit_pid: pid,
            attempted_syscall: "quarantine.requested.deferred".into(),
            sandbox_tier: SandboxTier::T3,
        };
        if sender.try_send(event).is_err() {
            cap_audit::record_drop();
        }
    }

    Err(T3Error::QuarantineRequiresSubprocessForm)
}

/// Result of a quarantine operation.
#[derive(Debug, Clone)]
pub struct QuarantineReport {
    pub spirit_id: String,
    pub spirit_pid: u32,
    pub target_tier: SandboxTier,
    pub outcome: QuarantineOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineOutcome {
    Deferred,
    Completed,
}
