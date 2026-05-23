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
//! use maos_domain::invariants::i10::{InvariantI10, LifecycleEvent, JournalEntry, LifecycleEntry};
//!
//! let _marker: InvariantI10 = InvariantI10;
//! let entry = JournalEntry::Lifecycle(LifecycleEntry {
//!     timestamp: 1_700_000_000,
//!     lifecycle_event: LifecycleEvent::Load,
//!     spirit_id: "spirit-nash".into(),
//!     effective_sandbox_tier: None,
//! });
//! assert!(matches!(entry, JournalEntry::Lifecycle(_)));
//! ```

use crate::invariants::i1::TokenId;
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
    /// Story 5.3 — Spirit crashed (panic, signal, OOM, timeout).
    Crash = 12,
    /// Story 5.3 — Spirit holding in-flight task emitted no progress IAC for > threshold.
    Stalled = 13,
    /// Story 5.3 — Spirit emitted heartbeat but no progress IAC for > threshold.
    SilentFailureSuspect = 14,
    /// Story 5.4 — Spirit upgraded via hot-swap, cold-swap, or migrator.
    Upgrade = 15,
    /// Story 5.4 — Spirit revoked via CRL propagation.
    Revoked = 16,
}

/// A single lifecycle journal entry — the v0.3-β shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LifecycleEntry {
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

/// A single in-flight task journal entry — cold-restart recovery.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InFlightEntry {
    /// Unix timestamp (nanoseconds) of the record.
    pub timestamp_ns: u64,
    /// Spirit identifier holding the task.
    pub spirit_id: String,
    /// Task identifier.
    pub task_id: String,
    /// Capability token bytes.
    pub capability_token: TokenId,
    /// TTL deadline (nanoseconds).
    pub ttl_deadline_ns: u64,
    /// Intent class as string.
    pub intent_class: String,
    /// Originator Spirit identifier.
    pub originator_spirit_id: String,
}

/// A single journal entry — immutable record of a lifecycle transition
/// or in-flight task state.
///
/// Story 5.3 refactor: struct → `#[serde(tag = "kind")]` enum.
/// Backward compatibility: old records without "kind" field
/// deserialize as `Lifecycle` via a custom `Deserialize` impl.
///
/// Serde derive is used for serialization only; deserialization is
/// manually implemented to support the v0.3-β struct-to-enum migration.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum JournalEntry {
    Lifecycle(LifecycleEntry),
    InFlight(InFlightEntry),
}

impl<'de> serde::Deserialize<'de> for JournalEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let value = serde_json::Value::deserialize(deserializer)?;

        match value.get("kind").and_then(|v| v.as_str()) {
            Some("lifecycle") => {
                let entry: LifecycleEntry =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(JournalEntry::Lifecycle(entry))
            }
            Some("in_flight") => {
                let entry: InFlightEntry =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(JournalEntry::InFlight(entry))
            }
            Some(other) => Err(D::Error::custom(format!(
                "unknown JournalEntry kind: {other:?}"
            ))),
            None => {
                // v0.3-β back-compat: no "kind" field → LifecycleEntry
                let entry: LifecycleEntry =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(JournalEntry::Lifecycle(entry))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_entry_construction() {
        let e = JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Halt,
            spirit_id: "s".into(),
            effective_sandbox_tier: None,
        });
        assert!(matches!(e, JournalEntry::Lifecycle(_)));
    }

    #[test]
    fn journal_entry_backward_compat_deser() {
        // Old NDJSON line without effective_sandbox_tier must still parse
        let old = r#"{"timestamp":42,"lifecycle_event":"Load","spirit_id":"legacy"}"#;
        let entry: JournalEntry = serde_json::from_str(old).unwrap();
        match entry {
            JournalEntry::Lifecycle(le) => {
                assert_eq!(le.timestamp, 42);
                assert_eq!(le.lifecycle_event, LifecycleEvent::Load);
                assert_eq!(le.spirit_id, "legacy");
                assert_eq!(le.effective_sandbox_tier, None);
            }
            other => panic!("expected Lifecycle variant, got {other:?}"),
        }
    }

    #[test]
    fn journal_entry_with_tier_roundtrips() {
        let entry = JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 99,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "test".into(),
            effective_sandbox_tier: Some(SandboxTier::T2),
        });
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("T2"));
        let back: JournalEntry = serde_json::from_str(&json).unwrap();
        match back {
            JournalEntry::Lifecycle(le) => {
                assert_eq!(le.effective_sandbox_tier, Some(SandboxTier::T2));
            }
            other => panic!("expected Lifecycle variant, got {other:?}"),
        }
    }

    #[test]
    fn in_flight_entry_roundtrip() {
        let entry = JournalEntry::InFlight(InFlightEntry {
            timestamp_ns: 1_000_000,
            spirit_id: "spirit-1".into(),
            task_id: "task-1".into(),
            capability_token: TokenId::ZERO,
            ttl_deadline_ns: 2_000_000,
            intent_class: "standard".into(),
            originator_spirit_id: "origin-1".into(),
        });
        let json = serde_json::to_string(&entry).unwrap();
        let back: JournalEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back, entry);
    }

    // ---- Story 5.3 — LifecycleEvent variant tests ----

    #[test]
    fn lifecycle_event_crash_serde_roundtrip() {
        let original = LifecycleEvent::Crash;
        let json = serde_json::to_string(&original).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleEvent::Crash);
    }

    #[test]
    fn lifecycle_event_stalled_serde_roundtrip() {
        let original = LifecycleEvent::Stalled;
        let json = serde_json::to_string(&original).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleEvent::Stalled);
    }

    #[test]
    fn lifecycle_event_silent_failure_suspect_serde_roundtrip() {
        let original = LifecycleEvent::SilentFailureSuspect;
        let json = serde_json::to_string(&original).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleEvent::SilentFailureSuspect);
    }

    #[test]
    fn lifecycle_event_non_exhaustive_match() {
        let ev = LifecycleEvent::Crash;
        let _name = match ev {
            LifecycleEvent::Crash => "crash",
            LifecycleEvent::Stalled => "stalled",
            LifecycleEvent::SilentFailureSuspect => "silent_failure",
            _ => "other",
        };
    }

    // ---- Story 5.4 — LifecycleEvent variant tests ----

    #[test]
    fn lifecycle_event_upgrade_discriminant() {
        assert_eq!(LifecycleEvent::Upgrade as u8, 15);
    }

    #[test]
    fn lifecycle_event_revoked_discriminant() {
        assert_eq!(LifecycleEvent::Revoked as u8, 16);
    }

    #[test]
    fn lifecycle_event_upgrade_serde_roundtrip() {
        let original = LifecycleEvent::Upgrade;
        let json = serde_json::to_string(&original).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleEvent::Upgrade);
    }

    #[test]
    fn lifecycle_event_revoked_serde_roundtrip() {
        let original = LifecycleEvent::Revoked;
        let json = serde_json::to_string(&original).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleEvent::Revoked);
    }
}
