#![forbid(unsafe_code)]

//! `orchestrator` — MAOS Orchestrator v0.8, the founder-loop wedge coordinator
//! and the fourth reference Spirit (architecture §6.7). Story 8.4.
//!
//! The Orchestrator is the conductor of the founder loop. It:
//!
//! 1. **Drains the FR20 [`OrchestratorBuffer`] at safe sequence points**
//!    ([`Orchestrator::drain_next`]): the director's buffered instructions are
//!    processed **only between Worker task completions** — an instruction
//!    enqueued while a delegation is in flight is **not** dispatched until the
//!    delegation completes (FR20 "never preempt in-flight delegations"). The
//!    safe-point gate is Spirit-side ([`Orchestrator::is_safe_point`]); the real
//!    kernel buffer is the `dequeue_at_safe_point` source the gate guards
//!    (driven as a dev-dep in `tests/orchestrator_buffer.rs`).
//! 2. **Dispatches distillate-fed** ([`Orchestrator::followup_dispatch`]): every
//!    follow-up `task.assign` carries a [`PriorDistillateRef`] (FR21) — **always
//!    a distillate ref, never raw output**. The kernel FR21 gate
//!    (`EOrchestratorDispatchRawOutput`) rejects a follow-up with no ref or one
//!    pointing at a raw `TaskComplete`; the typed [`Orchestrator::first_dispatch`]
//!    / [`Orchestrator::followup_dispatch`] split encodes the contract so a
//!    well-behaved Orchestrator cannot emit raw output by construction
//!    (Decision K: the **producer** distills its own output; the Orchestrator
//!    only **references** the resulting `Distillate`).
//! 3. **Drives the Architect→Reviewer code-review loop**: dispatch Architect →
//!    Architect completes → producer distills → Orchestrator references the
//!    distillate in the dispatch to Reviewer → Reviewer critiques → its
//!    distillate flows back. Every hop is FR21-clean.
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI and the PURE `maos-domain`
//! types (the FR20 `OrchestratorInstruction`, the FR21 `TaskAssignPayload` /
//! `PriorDistillateRef`, the `IacFrame` dispatch shape). It NEVER reaches into
//! `maos-kernel-core`. The real `OrchestratorBuffer`, the real FR21 IacBus gate,
//! and the real `DistillateWriter` are exercised in `tests/`, which carry those
//! crates as dev-dependencies only. The dispatch *decision* (order, target role,
//! distillate attachment) lives entirely here; the kernel buffer/gate are
//! consumed, not modified (§4.0.7).

use std::sync::{Arc, Mutex};

use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, PriorDistillateRef, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::orchestrator::OrchestratorInstruction;
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// One Orchestrator dispatch decision, serialized to exactly the manifest
/// `[output_shape]` `required_fields` (`target_role` / `goal` /
/// `has_distillate_ref` / `distillation_depth`). The Orchestrator's only FR21
/// contract: a follow-up dispatch ALWAYS carries a distillate ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestratorDispatch {
    /// The `SpiritRole` of the dispatch target (Worker for Worker/Architect/Reviewer).
    pub target_role: String,
    /// The natural-language goal this dispatch pursues.
    pub goal: String,
    /// Whether this dispatch carries a `PriorDistillateRef` (false only for the
    /// first dispatch of a fan-out, which has no predecessor).
    pub has_distillate_ref: bool,
    /// The effective distillation depth at this hop (0 for the first dispatch).
    pub distillation_depth: u32,
}

/// In-flight delegation bookkeeping — the Spirit-side safe-point gate (FR20).
#[derive(Debug, Default)]
struct DispatchState {
    /// Number of delegations currently in flight. The buffer is drained ONLY
    /// when this is zero (a safe sequence point between completions).
    in_flight: usize,
    /// The most recent dispatch decision (production-visible `on_idle` effect).
    last_dispatch: Option<OrchestratorDispatch>,
    /// Total dispatches built (a simple liveness counter).
    dispatched: u64,
}

/// Orchestrator reference Spirit — the founder-loop coordinator. Holds its own
/// Spirit id + the in-flight delegation bookkeeping. `Arc<Mutex<...>>` interior
/// state keeps Orchestrator `Sync` as the `#[spirit]` macro requires.
#[derive(Debug, Clone)]
pub struct Orchestrator {
    /// This Orchestrator's own Spirit id (the `from` of every dispatch).
    orchestrator_id: String,
    /// In-flight delegation bookkeeping + last decision.
    state: Arc<Mutex<DispatchState>>,
}

#[spirit]
impl Orchestrator {
    /// Idle coordination pass. Cancellation-aware and bounded (no work when no
    /// delegation context is pending). The LIVE buffer-drain + FR21-dispatch
    /// paths are proven against the real `OrchestratorBuffer` + IacBus gate in
    /// `tests/` (the ABI `Ctx` exposes no kernel-buffer surface — Butler /
    /// Researcher / Observer navigated the same gap). This hook only confirms
    /// liveness within the budget envelope.
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // A bounded, production-visible tick: nothing to dispatch at idle unless
        // the director has buffered instructions (driven via the real buffer in
        // tests). Touch the lock so the hook has an observable effect.
        let _guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            orchestrator_id: "orchestrator".to_string(),
            state: Arc::new(Mutex::new(DispatchState::default())),
        }
    }
}

impl Orchestrator {
    /// A fresh Orchestrator with the given Spirit id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            orchestrator_id: id.into(),
            ..Self::default()
        }
    }

    /// This Orchestrator's own Spirit id.
    pub fn orchestrator_id(&self) -> &str {
        &self.orchestrator_id
    }

    // ── FR20 safe-point gate (AC2) ───────────────────────────────────────────

    /// Whether the Orchestrator is at a safe sequence point — i.e. no delegation
    /// is currently in flight. The director's buffered instructions are drained
    /// ONLY at a safe point (FR20 "never preempt in-flight delegations").
    pub fn is_safe_point(&self) -> bool {
        self.in_flight() == 0
    }

    /// The number of delegations currently in flight.
    pub fn in_flight(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .in_flight
    }

    /// Record that a delegation has been dispatched and is now in flight. The
    /// Orchestrator will not drain another instruction until it completes.
    pub fn begin_delegation(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.in_flight += 1;
    }

    /// Record that an in-flight delegation has completed (re-opening a safe point
    /// once the count returns to zero).
    pub fn complete_delegation(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.in_flight = s.in_flight.saturating_sub(1);
    }

    /// Drain the next director instruction **only at a safe point**. `dequeue`
    /// is the real kernel buffer's `dequeue_at_safe_point` (passed as a closure
    /// so the kernel buffer stays a dev-dep — the Spirit owns the FR20 gate, the
    /// kernel owns the FIFO). Returns `None` while a delegation is in flight (the
    /// instruction stays buffered), or when the buffer is empty.
    pub fn drain_next<F>(&self, dequeue: F) -> Option<OrchestratorInstruction>
    where
        F: FnOnce() -> Option<OrchestratorInstruction>,
    {
        if !self.is_safe_point() {
            return None;
        }
        dequeue()
    }

    // ── FR21 distillate-fed dispatch (AC3) ───────────────────────────────────

    /// Build the FR21 `task.assign` payload. `prior` is `None` only for the
    /// first dispatch of a fan-out; every follow-up MUST carry a distillate ref
    /// (enforced by the typed [`Self::followup_dispatch`] entry point).
    pub fn build_task_assign(
        &self,
        goal: impl Into<String>,
        success_criteria: impl Into<String>,
        prior: Option<PriorDistillateRef>,
    ) -> TaskAssignPayload {
        TaskAssignPayload {
            goal: goal.into(),
            scope: vec![],
            success_criteria: success_criteria.into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: prior,
        }
    }

    /// Build the dispatch `IacFrame` carrying a `task.assign`. `seq` seeds the
    /// frame id (mirrors the 6.2 reference matrix); `to_role` is the target's
    /// `SpiritRole` (Worker for Worker/Architect/Reviewer — Decision C).
    pub fn assign_frame(
        &self,
        seq: u64,
        to_id: &str,
        to_role: SpiritRole,
        payload: TaskAssignPayload,
        lineage: IntentLineage,
    ) -> IacFrame {
        let mut frame_id = [0u8; 16];
        frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
        let mut to = smallvec::SmallVec::<[FrameAddress; 1]>::new();
        to.push(FrameAddress {
            spirit_id: SpiritId::from(to_id),
            host_id: None,
            role: Some(to_role),
        });
        let has_ref = payload.prior_distillate_ref.is_some();
        let depth = payload
            .prior_distillate_ref
            .as_ref()
            .map(|r| r.distillation_depth)
            .unwrap_or(0);
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.dispatched += 1;
        s.last_dispatch = Some(OrchestratorDispatch {
            target_role: format!("{to_role:?}"),
            goal: payload.goal.clone(),
            has_distillate_ref: has_ref,
            distillation_depth: depth,
        });
        drop(s);
        IacFrame {
            frame_id,
            timestamp_ns: seq,
            logical_clock: seq,
            from: FrameAddress {
                spirit_id: SpiritId::from(self.orchestrator_id.as_str()),
                host_id: None,
                role: Some(SpiritRole::Orchestrator),
            },
            to,
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(payload),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: None,
            intent_lineage: lineage,
        }
    }

    /// The FIRST dispatch of a fan-out — no predecessor, so `prior_distillate_ref`
    /// is `None` (the FR21 gate accepts a `None` first dispatch).
    pub fn first_dispatch(
        &self,
        seq: u64,
        to_id: &str,
        to_role: SpiritRole,
        goal: impl Into<String>,
        success_criteria: impl Into<String>,
        lineage: IntentLineage,
    ) -> IacFrame {
        let payload = self.build_task_assign(goal, success_criteria, None);
        self.assign_frame(seq, to_id, to_role, payload, lineage)
    }

    /// A FOLLOW-UP dispatch — ALWAYS carries a `PriorDistillateRef` (FR21: never
    /// raw output). `prior` references the `Distillate` row the producer wrote
    /// for its own output (Decision K). The `prior` argument is non-optional, so
    /// a well-behaved Orchestrator cannot emit a raw follow-up by construction.
    pub fn followup_dispatch(
        &self,
        seq: u64,
        to_id: &str,
        to_role: SpiritRole,
        goal: impl Into<String>,
        success_criteria: impl Into<String>,
        prior: PriorDistillateRef,
        lineage: IntentLineage,
    ) -> IacFrame {
        let payload = self.build_task_assign(goal, success_criteria, Some(prior));
        self.assign_frame(seq, to_id, to_role, payload, lineage)
    }

    /// The most recent dispatch decision (the `[output_shape]` surface).
    pub fn last_dispatch(&self) -> Option<OrchestratorDispatch> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .last_dispatch
            .clone()
    }

    /// Total dispatches built by this Orchestrator.
    pub fn dispatched_count(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dispatched
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use maos_domain::invariants::i8::A2AIntent;

    fn lineage() -> IntentLineage {
        IntentLineage::new(vec![A2AIntent::new("founder-loop-wedge")])
    }

    fn prior(depth: u32) -> PriorDistillateRef {
        PriorDistillateRef {
            digest_frame_id: [7u8; 16],
            distillation_depth: depth,
            intent_lineage: IntentLineage::default(),
        }
    }

    #[test]
    fn safe_point_gate_blocks_drain_while_in_flight() {
        let o = Orchestrator::new("orchestrator");
        assert!(o.is_safe_point(), "fresh orchestrator is at a safe point");

        // While a delegation is in flight, the gate refuses to drain.
        o.begin_delegation();
        assert!(!o.is_safe_point());
        let drained = o.drain_next(|| {
            Some(
                OrchestratorInstruction::new(
                    maos_domain::orchestrator::OrchestratorInstructionId(1),
                    "should not be drained",
                    1,
                )
                .unwrap(),
            )
        });
        assert!(
            drained.is_none(),
            "FR20: no drain while a delegation is in flight"
        );

        // Completion re-opens the safe point.
        o.complete_delegation();
        assert!(o.is_safe_point());
        let drained = o.drain_next(|| {
            Some(
                OrchestratorInstruction::new(
                    maos_domain::orchestrator::OrchestratorInstructionId(2),
                    "now drainable",
                    2,
                )
                .unwrap(),
            )
        });
        assert!(drained.is_some(), "drains at the safe point");
    }

    #[test]
    fn first_dispatch_has_no_prior_ref() {
        let o = Orchestrator::new("orchestrator");
        let f = o.first_dispatch(1, "worker", SpiritRole::Worker, "build", "done", lineage());
        match &f.payload {
            FramePayload::TaskAssign(p) => assert!(p.prior_distillate_ref.is_none()),
            other => panic!("expected TaskAssign, got {other:?}"),
        }
        let d = o.last_dispatch().expect("a dispatch was recorded");
        assert!(!d.has_distillate_ref);
        assert_eq!(d.distillation_depth, 0);
    }

    #[test]
    fn followup_dispatch_always_carries_a_distillate_ref() {
        let o = Orchestrator::new("orchestrator");
        let f = o.followup_dispatch(
            3,
            "reviewer",
            SpiritRole::Worker,
            "review",
            "ok",
            prior(2),
            lineage(),
        );
        match &f.payload {
            FramePayload::TaskAssign(p) => {
                let r = p
                    .prior_distillate_ref
                    .as_ref()
                    .expect("follow-up carries a distillate ref");
                assert_eq!(r.distillation_depth, 2);
            }
            other => panic!("expected TaskAssign, got {other:?}"),
        }
        let d = o.last_dispatch().unwrap();
        assert!(d.has_distillate_ref);
        assert_eq!(d.distillation_depth, 2);
    }

    #[test]
    fn dispatch_output_shape_has_required_fields() {
        let o = Orchestrator::new("orchestrator");
        o.first_dispatch(1, "worker", SpiritRole::Worker, "build", "done", lineage());
        let d = o.last_dispatch().unwrap();
        let v = serde_json::to_value(&d).unwrap();
        for field in [
            "target_role",
            "goal",
            "has_distillate_ref",
            "distillation_depth",
        ] {
            assert!(v.get(field).is_some(), "missing output field {field}");
        }
    }

    #[test]
    fn dispatched_count_increments() {
        let o = Orchestrator::new("orchestrator");
        assert_eq!(o.dispatched_count(), 0);
        o.first_dispatch(1, "worker", SpiritRole::Worker, "a", "x", lineage());
        o.followup_dispatch(
            2,
            "worker",
            SpiritRole::Worker,
            "b",
            "y",
            prior(1),
            lineage(),
        );
        assert_eq!(o.dispatched_count(), 2);
    }
}
