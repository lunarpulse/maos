#![forbid(unsafe_code)]

//! Task assignment port — in-flight task ledger for FR12 / FR50.
//!
//! Story 5.3 — per-SCB `task_assignments_in_flight` holds records of
//! tasks the Spirit has accepted but not yet completed.

use crate::invariants::i1::TokenId;

/// A single in-flight task assignment on a Spirit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskAssignmentRecord {
    /// Opaque task identifier (Spirit-assigned).
    pub task_id: String,
    /// Capability token that authorised this task assignment.
    pub capability_token: TokenId,
    /// Monotonic deadline (ns) after which the task is considered stale.
    pub ttl_deadline_ns: u64,
    /// Intent classification (Standard, Epistemic, etc.).
    pub intent_class: crate::invariants::i1::IntentClass,
    /// Spirit that originated the task.assign frame.
    pub originator_spirit_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_assignment_record_construction() {
        let rec = TaskAssignmentRecord {
            task_id: "task-001".into(),
            capability_token: TokenId([0u8; 16]),
            ttl_deadline_ns: 1_000_000_000,
            intent_class: crate::invariants::i1::IntentClass::Standard,
            originator_spirit_id: "butler".into(),
        };
        assert_eq!(rec.task_id, "task-001");
    }
}
