//! I10: Every Spirit lifecycle transition is journaled.
//!
//! Crash recovery rehydrates from the journal. Crash detection ≤2s;
//! `task.orphaned` IAC frame ≤5s with exit-cause recorded.
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — Journal at Spirit Scheduler + supervisor
//!   monitors child-process exit.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i10::{InvariantI10, LifecycleEvent, JournalEntry};
//!
//! let _marker: InvariantI10 = InvariantI10;
//! let entry = JournalEntry {
//!     timestamp: 1_700_000_000,
//!     lifecycle_event: LifecycleEvent::Load,
//!     spirit_id: "spirit-nash".into(),
//! };
//! assert!(matches!(entry.lifecycle_event, LifecycleEvent::Load));
//! ```

/// I10 marker type — Every Spirit lifecycle transition is journaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI10;

/// Lifecycle events that MUST be journaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum LifecycleEvent {
    /// Spirit binary loaded into memory.
    Load = 0,
    /// Spirit initialized and ready for events.
    Start = 1,
    /// Spirit paused (tokens retained, mailbox drained).
    Pause = 2,
    /// Spirit hot-swapped to successor.
    Swap = 3,
    /// Spirit migrated to a different Host.
    Migrate = 4,
    /// Spirit unloaded (graceful shutdown).
    Unload = 5,
    /// Spirit halted (epistemic or operator trigger).
    Halt = 6,
}

/// A single journal entry — immutable record of a lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JournalEntry {
    /// Unix timestamp (seconds) of the transition.
    pub timestamp: u64,
    /// Which lifecycle event occurred.
    pub lifecycle_event: LifecycleEvent,
    /// Spirit identifier affected.
    pub spirit_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_entry_construction() {
        let e = JournalEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Halt,
            spirit_id: "s".into(),
        };
        assert!(matches!(e.lifecycle_event, LifecycleEvent::Halt));
    }
}
