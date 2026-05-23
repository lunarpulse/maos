#![forbid(unsafe_code)]

//! Orchestrator instruction domain types — director-surface seam (Story
//! 3.4) + Orchestrator-class Spirit consumer seam (Story 8.4 founder-loop).
//!
//! `OrchestratorInstruction` is the wire-shape the director enqueues via
//! `maosctl orchestrator queue`; the kernel's `OrchestratorBuffer`
//! routes it to the Orchestrator-class Spirit at safe sequence points.

/// Stable identifier for a queued instruction. Used by `maosctl
/// orchestrator status` to surface the ordered pending set and by the
/// Approval Decision Log row written on enqueue (AC2).
///
/// v0.3-β: monotonic per-Spirit u64 minted by `OrchestratorBuffer::enqueue`.
/// Story 8.4 may promote to ULID for cross-Host ordering; the newtype
/// shields callers from that change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OrchestratorInstructionId(pub u64);

/// The instruction the director hands to the Orchestrator. Free-form
/// natural-language goal at v0.3-β (mirrors `task.assign.goal` shape from
/// Story 3.1's `TaskAssignPayload`); typed structuring lands at Story
/// 8.4 (founder-loop wedge) when the orchestration policy stabilizes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrchestratorInstruction {
    pub id: OrchestratorInstructionId,
    /// Natural-language goal the director wants the Orchestrator to pursue.
    pub goal: String,
    /// Wall-clock nanoseconds at enqueue time (monotonic counter; same
    /// shape as Story 3.1's `IacFrame::timestamp_ns`).
    pub enqueued_at_ns: u64,
}

impl OrchestratorInstruction {
    /// Construct an instruction. Returns `Err` if `goal` is empty or
    /// whitespace-only — mirrors `Resolution::provided_context`
    /// validation from Story 3.3 AC1 (`maos-domain/src/halt.rs:68-74`).
    pub fn new(
        id: OrchestratorInstructionId,
        goal: impl Into<String>,
        enqueued_at_ns: u64,
    ) -> Result<Self, OrchestratorInstructionError> {
        let goal = goal.into();
        if goal.trim().is_empty() {
            return Err(OrchestratorInstructionError::EmptyGoal);
        }
        Ok(Self {
            id,
            goal,
            enqueued_at_ns,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrchestratorInstructionError {
    #[error("orchestrator instruction goal must be non-empty")]
    EmptyGoal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_new_rejects_empty_goal() {
        let result = OrchestratorInstruction::new(OrchestratorInstructionId(1), "", 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrchestratorInstructionError::EmptyGoal
        ));
    }

    #[test]
    fn instruction_new_rejects_whitespace_only_goal() {
        let result = OrchestratorInstruction::new(OrchestratorInstructionId(1), "   ", 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrchestratorInstructionError::EmptyGoal
        ));
    }

    #[test]
    fn instruction_new_accepts_nonempty_goal() {
        let instr =
            OrchestratorInstruction::new(OrchestratorInstructionId(42), "draft the PR", 1000)
                .unwrap();
        assert_eq!(instr.id.0, 42);
        assert_eq!(instr.goal, "draft the PR");
        assert_eq!(instr.enqueued_at_ns, 1000);
    }

    #[test]
    fn instruction_id_serde_round_trip() {
        let id = OrchestratorInstructionId(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let back: OrchestratorInstructionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn instruction_serde_round_trip() {
        let instr =
            OrchestratorInstruction::new(OrchestratorInstructionId(42), "draft the PR", 1000)
                .unwrap();
        let json = serde_json::to_string(&instr).unwrap();
        assert_eq!(
            json,
            r#"{"id":42,"goal":"draft the PR","enqueued_at_ns":1000}"#
        );
        let back: OrchestratorInstruction = serde_json::from_str(&json).unwrap();
        assert_eq!(instr, back);
    }
}
