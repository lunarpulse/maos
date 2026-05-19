#![forbid(unsafe_code)]

//! Tagged-scalar slot per architecture §4.6 + ADR-022.
//!
//! The kernel holds a per-Spirit-per-tag scalar slot in the Capability
//! Registry (NOT the Memory Manager — Story 4.3 owns the three-tier
//! mechanics). Spirits write tagged scalars via
//! `working_memory.set_scalar(tag, value, derived_from)` and the kernel
//! routes writes to `[epistemic_policy]` predicate evaluation.
//!
//! ## Classification target
//!
//! All public items in this module classify as **universal-arithmetic**
//! per §4.0.7. The kernel does NOT compute variance, entropy, EFE, KL,
//! or derivatives — only the four ADR-022 predicates.

pub mod store;
pub mod policy_runtime;
pub mod orchestrator;

use maos_domain::invariants::i7::ScalarTapEvent;

/// Per-Spirit-per-tag scalar slot persisted on `set_scalar`.
///
/// One slot per unique `(spirit_pid, tag)` pair; a new write replaces
/// the prior value for the same tag (last-write-wins).
#[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]
#[derive(Debug, Clone, PartialEq)]
pub struct WorkingMemorySlot {
    #[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]
    pub tag: String,
    #[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]
    pub value: f64,
    #[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]
    pub derived_from: String,
    #[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]
    pub timestamp_ms: u64,
}

impl WorkingMemorySlot {
    pub fn new(
        tag: String,
        value: f64,
        derived_from: String,
        timestamp_ms: u64,
    ) -> Result<Self, SetScalarError> {
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
        Ok(Self {
            tag,
            value,
            derived_from,
            timestamp_ms,
        })
    }
}

/// Typed-error enum for `set_scalar` rejection paths.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SetScalarError {
    #[error("scalar value is NaN; predicate scalars must be comparable")]
    NanValue,
    #[error("tag must be non-empty")]
    EmptyTag,
    #[error("derived_from must be non-empty")]
    EmptyDerivedFrom,
    #[error("slot-persistence overflowed; per-Spirit-per-tag slot map invariant violated")]
    OverflowingPersistence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_memory_slot_new_well_formed() {
        let slot = WorkingMemorySlot::new(
            "uncertainty".into(),
            0.75,
            "frame-001".into(),
            1_700_000_000_000_000_000,
        )
        .unwrap();
        assert_eq!(slot.tag, "uncertainty");
        assert_eq!(slot.value, 0.75);
        assert_eq!(slot.derived_from, "frame-001");
    }

    #[test]
    fn working_memory_slot_rejects_nan() {
        let err = WorkingMemorySlot::new(
            "t".into(),
            f64::NAN,
            "d".into(),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, SetScalarError::NanValue));
    }

    #[test]
    fn working_memory_slot_rejects_empty_tag() {
        let err = WorkingMemorySlot::new(
            "".into(),
            0.5,
            "d".into(),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, SetScalarError::EmptyTag));
    }

    #[test]
    fn working_memory_slot_rejects_empty_derived_from() {
        let err = WorkingMemorySlot::new(
            "t".into(),
            0.5,
            "".into(),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, SetScalarError::EmptyDerivedFrom));
    }

    #[test]
    fn working_memory_slot_rejects_infinite_value() {
        let err = WorkingMemorySlot::new(
            "t".into(),
            f64::INFINITY,
            "d".into(),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, SetScalarError::OverflowingPersistence));
        let err = WorkingMemorySlot::new(
            "t".into(),
            f64::NEG_INFINITY,
            "d".into(),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, SetScalarError::OverflowingPersistence));
    }
}
