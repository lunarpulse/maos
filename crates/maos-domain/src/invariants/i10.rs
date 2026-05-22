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
//!     effective_sandbox_tier: None,
//! };
//! assert!(matches!(entry.lifecycle_event, LifecycleEvent::Load));
//! ```

use crate::invariants::i9::SandboxTier;

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
    /// Story 3.2 — director-initiated runtime posture shift.
    PostureShift = 7,
    /// Story 3.4 — director-initiated resume from paused state.
    /// Preserves existing discriminator values for wire stability.
    Resume = 8,
    /// Story 5.2 — hot-swap aborted (swap-out failed, swap-in failed, or halt-continuity violation).
    HotSwapAborted = 9,
    /// Story 5.2 — hot-swap auto-reverted (post-swap invariant violation within 30s window).
    HotSwapAutoReverted = 10,
    /// Story 5.2 — hot-swap completed successfully.
    HotSwap = 11,
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
    /// Effective sandbox tier at admission (Story 1b.3).
    /// `serde(default)` keeps old NDJSON journal lines parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_sandbox_tier: Option<SandboxTier>,
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
            effective_sandbox_tier: None,
        };
        assert!(matches!(e.lifecycle_event, LifecycleEvent::Halt));
    }

    #[test]
    fn journal_entry_backward_compat_deser() {
        // Old NDJSON line without effective_sandbox_tier must still parse
        let old = r#"{"timestamp":42,"lifecycle_event":"Load","spirit_id":"legacy"}"#;
        let entry: JournalEntry = serde_json::from_str(old).unwrap();
        assert_eq!(entry.timestamp, 42);
        assert_eq!(entry.lifecycle_event, LifecycleEvent::Load);
        assert_eq!(entry.spirit_id, "legacy");
        assert_eq!(entry.effective_sandbox_tier, None);
    }

    #[test]
    fn journal_entry_with_tier_roundtrips() {
        let entry = JournalEntry {
            timestamp: 99,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "test".into(),
            effective_sandbox_tier: Some(SandboxTier::T2),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("T2"));
        let back: JournalEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.effective_sandbox_tier, Some(SandboxTier::T2));
    }
}
