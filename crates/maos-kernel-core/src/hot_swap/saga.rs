#![forbid(unsafe_code)]

//! Saga-style compensating transactions for hot-swap failure boundaries.
//!
//! ADR-017 §Decision sentence 4: three compensating arms for three
//! failure boundaries. The saga records the state needed to compensate
//! for each phase's failure.
//!
//! ## State machine
//!
//! ```text
//! NotStarted → HaltContinuityChecked → SwapOutFired → SnapshotTaken
//!   → SwapInFired → Committed → MonitorComplete
//! ```

use std::sync::Arc;
use std::time::Instant;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::{ScbRuntimeSnapshot, SpiritControlBlock};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i3::FrameOrigin;

use super::post_swap_monitor::PostSwapInvariantViolation;

/// Phases of the hot-swap saga.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaPhase {
    NotStarted,
    HaltContinuityChecked,
    SwapOutFired,
    SnapshotTaken,
    SwapInFired,
    Committed,
}

/// Compensation action for a failed swap phase.
#[derive(Debug, Clone)]
pub enum SagaCompensation {
    RestorePredecessor {
        reason: String,
    },
    DiscardSuccessor {
        reason: String,
    },
    AutoRevert {
        invariant: PostSwapInvariantViolation,
    },
}

/// A saga records the state needed to compensate for each phase's failure.
pub struct HotSwapSaga {
    /// Runtime-only predecessor snapshot. The SCB allocation remains in the
    /// scheduler map and is restored through `replace_runtime`.
    pub pre_swap_runtime: Option<ScbRuntimeSnapshot>,
    pub predecessor_spirit_id: Option<String>,
    pub predecessor_pid: u32,
    pub started_at: Instant,
    pub current_phase: SagaPhase,
}

impl HotSwapSaga {
    pub fn new() -> Self {
        Self {
            pre_swap_runtime: None,
            predecessor_spirit_id: None,
            predecessor_pid: 0,
            started_at: Instant::now(),
            current_phase: SagaPhase::NotStarted,
        }
    }

    /// Require that with_pre_swap_snapshot has been called before compensation.
    fn ensure_snapshot(&self) {
        assert!(
            self.pre_swap_runtime.is_some(),
            "HotSwapSaga::compensate called before with_pre_swap_snapshot"
        );
    }

    /// Capture only the predecessor runtime for rollback. The caller retains
    /// the stable SCB Arc and restores this snapshot on it.
    pub fn with_pre_swap_snapshot(mut self, scb: Arc<SpiritControlBlock>) -> Self {
        self.predecessor_pid = scb.pid;
        self.predecessor_spirit_id = Some(scb.spirit_id.clone());
        self.pre_swap_runtime = Some(scb.runtime_snapshot());
        self
    }

    pub fn advance_to(&mut self, phase: SagaPhase) {
        self.current_phase = phase;
    }

    /// Execute the compensation action for a failed phase.
    pub async fn compensate(
        &self,
        compensation: SagaCompensation,
        tl: &TransparencyLogAdapter,
        journal: &JournalAdapter,
    ) {
        self.ensure_snapshot();
        let spirit_id = self
            .predecessor_spirit_id
            .clone()
            .unwrap_or_else(|| format!("pid-{}", self.predecessor_pid));

        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        // Story 5.2 review backfill: per AC3, the phase label MUST reflect the
        // saga's current state at compensation time. Exhaustive match — no silent
        // wildcards (carry-forward Epic 4 retro pattern #5).
        let (reason, phase_str) = match &compensation {
            SagaCompensation::RestorePredecessor { reason } => {
                let phase = match self.current_phase {
                    SagaPhase::NotStarted => "NotStarted",
                    SagaPhase::HaltContinuityChecked => "HaltContinuityChecked",
                    SagaPhase::SwapOutFired => "SwapOutFired",
                    SagaPhase::SnapshotTaken => "SnapshotTaken",
                    SagaPhase::SwapInFired => "SwapInFired",
                    SagaPhase::Committed => "Committed",
                };
                (reason.clone(), phase)
            }
            // DiscardSuccessor fires from step 9 (on_swap_in failure), which is
            // SagaPhase::SwapInFired — NOT SnapshotTaken (the prior label).
            SagaCompensation::DiscardSuccessor { reason } => (reason.clone(), "SwapInFired"),
            SagaCompensation::AutoRevert { invariant } => {
                (format!("{invariant:?}"), "PostSwapWindow")
            }
        };

        // Journal the abort.
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: timestamp_ns,
            lifecycle_event: LifecycleEvent::HotSwapAborted,
            spirit_id: spirit_id.clone(),
            payload: None,
            effective_sandbox_tier: None,
        }));

        // Emit IAC transparency log frame.
        let payload = serde_json::json!({
            "spirit_pid": self.predecessor_pid,
            "reason": reason,
            "phase": phase_str,
            "compensation": format!("{compensation:?}"),
        });
        tl.insert_frame_event(
            FrameKind::HotSwapAborted,
            self.predecessor_pid,
            None,
            "hot_swap.aborted",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
    }
}

impl Default for HotSwapSaga {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::control_block::{
        make_spirit_obj, SpiritControlBlock, SpiritManifestBundle,
    };
    use maos_spirit_abi::lifecycle::Spirit;

    struct TestSpirit;
    impl Spirit for TestSpirit {}

    fn make_test_scb() -> Arc<SpiritControlBlock> {
        Arc::new(SpiritControlBlock::new(
            42,
            "test-spirit".into(),
            SpiritManifestBundle::default(),
            make_spirit_obj(TestSpirit),
            0xCAFE,
        ))
    }

    #[test]
    fn saga_advances_through_phases() {
        let mut saga = HotSwapSaga::new();
        assert_eq!(saga.current_phase, SagaPhase::NotStarted);
        saga.advance_to(SagaPhase::HaltContinuityChecked);
        assert_eq!(saga.current_phase, SagaPhase::HaltContinuityChecked);
        saga.advance_to(SagaPhase::Committed);
        assert_eq!(saga.current_phase, SagaPhase::Committed);
    }

    #[test]
    fn saga_captures_pre_swap_runtime() {
        let scb = make_test_scb();
        let saga = HotSwapSaga::new().with_pre_swap_snapshot(Arc::clone(&scb));
        assert_eq!(saga.predecessor_pid, 42);
        assert!(saga.pre_swap_runtime.is_some());
    }
}
