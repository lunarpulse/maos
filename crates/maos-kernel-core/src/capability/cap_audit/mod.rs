#![forbid(unsafe_code)]

//! Capability audit — bounded-MPSC slow-path writer.
//!
//! Per ADR-030: the audit path goes via bounded MPSC to a single writer
//! task. The hot path NEVER blocks on the audit channel (uses `try_send`
//! and falls back to an `AuditDrop` counter).

pub mod writer_task;

pub use writer_task::CapAuditWriter;

use std::sync::atomic::{AtomicU64, Ordering};

use maos_domain::invariants::i1::{Scope, TokenId};

/// Audit channel depth — load-bearing per ADR-030.
pub const AUDIT_CHANNEL_DEPTH: usize = 8192;

/// Global counter of dropped audit events due to channel saturation.
static AUDIT_DROP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Record one dropped audit event.
pub fn record_drop() {
    AUDIT_DROP_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Return the total number of dropped audit events since boot.
pub fn audit_drop_count() -> u64 {
    AUDIT_DROP_COUNTER.load(Ordering::Relaxed)
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
        reason: crate::capability::cap_tokens::RevokeReason,
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
    fn drop_counter_increments() {
        let before = audit_drop_count();
        record_drop();
        assert_eq!(audit_drop_count(), before + 1);
    }
}
