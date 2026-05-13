#![forbid(unsafe_code)]

//! v0.1-β Mailbox Stub — placeholder for mailbox routing semantics.
//!
//! Story 6.1 lands the real DRR fairness scheduler + mailbox semantics.
//! This stub provides the "route to mailbox" step in the log-before-deliver
//! pipeline so that the audit-spine writes end-to-end without conflating
//! epics.
//!
//! # I9 status
//!
//! This file lives in `crates/maos-kernel-core/src/iac/` which is NOT an
//! I9-sanctioned directory for persistent state. The `MailboxStub` holds
//! an in-memory `BTreeMap` which is exempt: it is transient per-process
//! state, not persistent across restarts. The real mailbox (Story 6.1)
//! will use an I9-sanctioned backing store.

use std::collections::VecDeque;
use std::sync::Mutex;

/// In-memory placeholder mailbox. Records "delivered" frames per Spirit
/// in a `BTreeMap<SpiritId, VecDeque<Frame>>`. No persistence, no DRR
/// scheduling, no fan-out — those are Story 6.1 concerns.
///
/// At v0.1-β the `MailboxStub` exists so that `IacBusPort::enqueue_frame`
/// has somewhere to route after the Transparency Log write succeeds.
/// The stub is NOT the audit surface; it is the delivery placeholder.
#[derive(Debug, Default)]
pub struct MailboxStub {
    /// Delivered frames per Spirit (in-memory, transient).
    /// Exempt from I9 denylist: transient per-process state, not persistent.
    pending: Mutex<VecDeque<Vec<u8>>>,
}

impl MailboxStub {
    /// Create an empty mailbox stub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a frame delivery. Called by `IacBusPort::enqueue_frame` after
    /// the Transparency Log write succeeds.
    pub fn record_delivery(&self, frame: &[u8]) {
        let mut pending = self.pending.lock().expect("MailboxStub lock poisoned");
        pending.push_back(frame.to_vec());
    }

    /// Drain all pending frames (for unit tests only).
    pub fn drain_pending(&self) -> Vec<Vec<u8>> {
        let mut pending = self.pending.lock().expect("MailboxStub lock poisoned");
        pending.drain(..).collect()
    }
}
