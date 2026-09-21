#![forbid(unsafe_code)]

//! Capability audit — bounded-MPSC slow-path writer types and channel.
//!
//! Per ADR-030: the audit path goes via bounded MPSC to a single writer
//! task. The hot path NEVER blocks on the audit channel (uses `try_send`
//! and falls back to an `AuditDrop` counter).
//!
//! NOTE: `CapAuditWriter` (the task that drains to TransparencyLog) lives
//! in `maos-kernel-core` because it depends on `crate::iac::transparency_log`.
//! This module provides only the types and channel factory.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use maos_domain::invariants::i1::{Scope, TokenId};
/// Audit channel depth — load-bearing per ADR-030.
pub const AUDIT_CHANNEL_DEPTH: usize = 8192;

/// Audit-event send sites. The fixed list is the class-wide instrument:
/// callers must name the site whose event could not reach the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum AuditDropSite {
    Issue,
    Revoke,
    RevokeAll,
    Verification,
    Invocation,
    SandboxBlock,
    T3EscapeBlock,
    Quarantine,
    AcpNotification,
}

impl AuditDropSite {
    const COUNT: usize = 9;
}

/// Why an audit event did not reach the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditDropReason {
    /// The bounded queue was full. A later event may still succeed.
    Full,
    /// The receiver was dropped. The process cannot recover its audit writer.
    Closed,
}

/// Process-wide audit health, readable without the failed audit channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditHealthSnapshot {
    pub total_drops: u64,
    pub degraded: bool,
    by_site: [u64; AuditDropSite::COUNT],
}

impl AuditHealthSnapshot {
    pub fn count(self, site: AuditDropSite) -> u64 {
        self.by_site[site as usize]
    }
}

static AUDIT_DROP_COUNTERS: [AtomicU64; AuditDropSite::COUNT] =
    [const { AtomicU64::new(0) }; AuditDropSite::COUNT];
static AUDIT_DEGRADED: AtomicBool = AtomicBool::new(false);

/// Record one audit event that did not reach the writer.
pub fn record_drop(site: AuditDropSite, reason: AuditDropReason) {
    AUDIT_DROP_COUNTERS[site as usize].fetch_add(1, Ordering::Relaxed);
    if reason == AuditDropReason::Closed {
        AUDIT_DEGRADED.store(true, Ordering::Release);
    }
}

/// Return the out-of-band audit health snapshot.
pub fn audit_health_snapshot() -> AuditHealthSnapshot {
    let mut by_site = [0; AuditDropSite::COUNT];
    for (count, value) in AUDIT_DROP_COUNTERS.iter().zip(&mut by_site) {
        *value = count.load(Ordering::Relaxed);
    }
    AuditHealthSnapshot {
        total_drops: by_site.iter().sum(),
        degraded: AUDIT_DEGRADED.load(Ordering::Acquire),
        by_site,
    }
}

/// Record a failed bounded-channel send with its permanent/transient cause.
pub fn record_send_error(
    site: AuditDropSite,
    error: &tokio::sync::mpsc::error::TrySendError<CapAuditEvent>,
) {
    let reason = match error {
        tokio::sync::mpsc::error::TrySendError::Full(_) => AuditDropReason::Full,
        tokio::sync::mpsc::error::TrySendError::Closed(_) => AuditDropReason::Closed,
    };
    record_drop(site, reason);
}

/// Outcome of a token verification — structured per AC3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    Ok,
    Expired,
    Revoked,
    SignatureMismatch,
    PostureMismatch,
    SpiritIdMismatch,
    UnknownToken,
}

impl VerifyOutcome {
    pub fn from_result(res: &Result<(), maos_domain::ports::capability::CapError>) -> Self {
        match res {
            Ok(()) => VerifyOutcome::Ok,
            Err(e) => match e {
                maos_domain::ports::capability::CapError::Expired => VerifyOutcome::Expired,
                maos_domain::ports::capability::CapError::Revoked => VerifyOutcome::Revoked,
                maos_domain::ports::capability::CapError::SignatureMismatch => {
                    VerifyOutcome::SignatureMismatch
                }
                maos_domain::ports::capability::CapError::PostureMismatch => {
                    VerifyOutcome::PostureMismatch
                }
                maos_domain::ports::capability::CapError::SpiritIdMismatch => {
                    VerifyOutcome::SpiritIdMismatch
                }
                maos_domain::ports::capability::CapError::UnknownToken => {
                    VerifyOutcome::UnknownToken
                }
                _ => VerifyOutcome::UnknownToken,
            },
        }
    }
}

/// Events emitted by the capability sub-modules to the audit channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapAuditEvent {
    /// Token was issued.
    Issue {
        token_id: TokenId,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
    },
    /// Token was verified with structured outcome.
    Verify {
        token_id: TokenId,
        spirit_pid: u32,
        outcome: VerifyOutcome,
    },
    /// Token was revoked.
    Revoke {
        token_id: TokenId,
        reason: crate::cap_tokens::RevokeReason,
    },
    /// Token was invoked (external call mediated).
    Invocation {
        token_id: TokenId,
        spirit_pid: u32,
        capability_token_bytes: Vec<u8>,
        intent: String,
        payload: Vec<u8>,
    },
    /// Sandbox blocked a syscall (Story 1b.3 socket).
    SandboxBlock {
        spirit_pid: u32,
        attempted_syscall: String,
        sandbox_tier: maos_domain::invariants::i9::SandboxTier,
    },
}

/// Sender side of the audit channel.
pub type Sender = tokio::sync::mpsc::Sender<CapAuditEvent>;

/// Create a bounded audit channel.
pub fn channel() -> (Sender, tokio::sync::mpsc::Receiver<CapAuditEvent>) {
    tokio::sync::mpsc::channel(AUDIT_CHANNEL_DEPTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_creation() {
        let (tx, _rx) = channel();
        assert_eq!(tx.max_capacity(), AUDIT_CHANNEL_DEPTH);
    }

    #[test]
    fn drop_instrument_names_site_and_latches_closed_writer() {
        let before = audit_health_snapshot();
        record_drop(AuditDropSite::Invocation, AuditDropReason::Closed);
        let after = audit_health_snapshot();
        assert_eq!(after.total_drops, before.total_drops + 1);
        assert_eq!(
            after.count(AuditDropSite::Invocation),
            before.count(AuditDropSite::Invocation) + 1
        );
        assert!(after.degraded);
    }
}
