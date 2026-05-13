//! Spirit Scheduler port trait per architecture §4.1.
//!
//! At v0.1-α this trait declares the lifecycle-verb surface only;
//! Story 1b.1 lands the audit-spine integration, Story 5.1 lands the
//! full 11-verb lifecycle surface. Method bodies are deferred.

use crate::invariants::i10::{JournalEntry, LifecycleEvent};

/// Spirit Scheduler — supervisor over Security / Memory / IAC / Capability.
///
/// Per §4.0.8 supervisor exception: satisfies P1, P2, P4 but is exempt
/// from P3 (its boundary IS the union of its children's boundaries).
pub trait SpiritSchedulerPort {
    /// Class: supervision
    ///
    /// Append a lifecycle transition to the kernel's per-Host journal.
    /// At v0.1-α the journal lives in `crates/maos-kernel-core/src/journal/`
    /// (an I9-sanctioned holder) but its mechanics ship in Story 1b.1.
    fn journal_lifecycle(&self, entry: JournalEntry);

    /// Class: supervision
    ///
    /// Returns the current lifecycle event most recently journaled for
    /// a Spirit; `None` if no entry has been journaled. Read-only; does
    /// not mutate state. Adapter implementations are expected to query
    /// the journal directly.
    fn last_lifecycle_event(&self, spirit_id: &str) -> Option<LifecycleEvent>;
}
