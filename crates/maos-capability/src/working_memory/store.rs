#![forbid(unsafe_code)]

//! Per-Spirit-per-tag scalar slot store — Capability Registry sub-module.
//!
//! The `WorkingMemoryStore` holds an `RwLock<HashMap<(spirit_pid, tag),
//! WorkingMemorySlot>>`. Each scalar write replaces the prior value
//! for the same `(spirit_pid, tag)` key (last-write-wins).
//!
//! ## Classification target
//! universal-arithmetic (§4.0.7) — no variance/entropy/EFE/KL computation.

use std::collections::HashMap;
use std::sync::RwLock;

use maos_domain::invariants::i7::ScalarTapEvent;

use super::{SetScalarError, WorkingMemorySlot};

/// Per-Spirit-per-tag scalar slot storage.
///
/// Composite key `(spirit_pid: u32, tag: String)` scopes slots per-Spirit.
/// One slot per tag; new writes replace prior values.
#[maos_attrs::i9_exempt(
    reason = "capability registry tagged-scalar slot — per-Spirit working memory state for ADR-022 universal-arithmetic predicate evaluation; parallel to capability-token ledger, not pattern-learning"
)]
#[derive(Debug, Default)]
pub struct WorkingMemoryStore {
    slots: RwLock<HashMap<(u32, String), WorkingMemorySlot>>,
}

impl WorkingMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Persist a scalar write for `(spirit_pid, tag)`.
    ///
    /// Validation: rejects empty `tag`, empty `derived_from`, and NaN
    /// `value`. Returns a fully-populated `ScalarTapEvent` for the
    /// caller to publish to the telemetry stream.
    ///
    /// Overwrites any prior slot for the same `(spirit_pid, tag)` pair.
    pub fn set_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<ScalarTapEvent, SetScalarError> {
        if tag.is_empty() {
            return Err(SetScalarError::EmptyTag);
        }
        if value.is_nan() {
            return Err(SetScalarError::NanValue);
        }
        if value.is_infinite() {
            return Err(SetScalarError::OverflowingPersistence);
        }
        if derived_from.is_empty() {
            return Err(SetScalarError::EmptyDerivedFrom);
        }

        let timestamp = unix_timestamp_ms();

        let slot =
            WorkingMemorySlot::new(tag.to_string(), value, derived_from.to_string(), timestamp)?;

        {
            let mut map = self.slots.write().unwrap_or_else(|e| e.into_inner());
            map.insert((spirit_pid, tag.to_string()), slot);
        }

        Ok(ScalarTapEvent {
            spirit_id: spirit_id.to_string(),
            tag: tag.to_string(),
            value,
            timestamp,
        })
    }

    /// Read-back a scalar value + timestamp for `(spirit_pid, tag)`.
    /// Returns `None` when no write has been recorded for this tag.
    pub fn get_scalar(&self, spirit_pid: u32, tag: &str) -> Option<(f64, u64)> {
        let map = self.slots.read().unwrap_or_else(|e| e.into_inner());
        map.get(&(spirit_pid, tag.to_string()))
            .map(|slot| (slot.value, slot.timestamp_ms))
    }
}

fn unix_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_scalar_happy_path() {
        let store = WorkingMemoryStore::new();
        let event = store
            .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
            .unwrap();
        assert_eq!(event.spirit_id, "spirit-1");
        assert_eq!(event.tag, "uncertainty");
        assert_eq!(event.value, 0.75);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn set_scalar_rejects_empty_tag() {
        let store = WorkingMemoryStore::new();
        let err = store
            .set_scalar(1, "spirit-1", "", 0.5, "frame-001")
            .unwrap_err();
        assert!(matches!(err, SetScalarError::EmptyTag));
    }

    #[test]
    fn set_scalar_rejects_nan_value() {
        let store = WorkingMemoryStore::new();
        let err = store
            .set_scalar(1, "spirit-1", "tag", f64::NAN, "frame-001")
            .unwrap_err();
        assert!(matches!(err, SetScalarError::NanValue));
    }

    #[test]
    fn set_scalar_rejects_empty_derived_from() {
        let store = WorkingMemoryStore::new();
        let err = store.set_scalar(1, "spirit-1", "tag", 0.5, "").unwrap_err();
        assert!(matches!(err, SetScalarError::EmptyDerivedFrom));
    }

    #[test]
    fn set_scalar_overwrites_same_tag() {
        let store = WorkingMemoryStore::new();
        store
            .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
            .unwrap();
        let event = store
            .set_scalar(1, "spirit-1", "uncertainty", 0.60, "frame-002")
            .unwrap();
        assert_eq!(event.value, 0.60);
    }

    #[test]
    fn get_scalar_reads_back() {
        let store = WorkingMemoryStore::new();
        store
            .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
            .unwrap();
        let (value, _ts) = store.get_scalar(1, "uncertainty").unwrap();
        assert_eq!(value, 0.75);
    }

    #[test]
    fn get_scalar_returns_none_for_unwritten_tag() {
        let store = WorkingMemoryStore::new();
        assert!(store.get_scalar(1, "nonexistent").is_none());
    }

    #[test]
    fn set_scalar_scoped_per_spirit_per_tag() {
        let store = WorkingMemoryStore::new();
        store.set_scalar(1, "spirit-a", "tag-x", 0.1, "f1").unwrap();
        store.set_scalar(2, "spirit-b", "tag-x", 0.9, "f2").unwrap();
        // Different spirit_pid => different slots
        let (v1, _) = store.get_scalar(1, "tag-x").unwrap();
        let (v2, _) = store.get_scalar(2, "tag-x").unwrap();
        assert_eq!(v1, 0.1);
        assert_eq!(v2, 0.9);
    }
}
