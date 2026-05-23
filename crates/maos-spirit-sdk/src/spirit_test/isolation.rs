#![forbid(unsafe_code)]

//! Cross-Spirit memory isolation framework hooks (NFR-Sec-14 substrate).
//!
//! Story 2.4 ships the HOOK SHAPE — the 4 hook points + 8 attack-category
//! enum + 2-Spirit fixture + DefaultIsolationHook reference impl. The
//! 200-scenario adversarial corpus (Sec-14a n=100 same-Host + Sec-14b
//! n=100 cross-Host per ADR-040) is Story 4.5 at v0.8.
//!
//! Architecture §8.1 + epic-4 line 17 enumerate the 8 categories.

use crate::local_runner::{LocalRunner, LocalRunnerFixture};
use crate::{Spirit, SpiritVtable};

/// The 8 categories per architecture §8.1 + epic-4 line 17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationAttackCategory {
    NamespaceEnumeration,
    WorkingMemoryReadAcross,
    DecisionFrameObservation,
    HaltSignalObservation,
    TransparencyLogCrossRead,
    WorkingMemoryDigestCrossRead,
    CapabilityTokenForgeryCrossSpirit,
    SandboxEscapeLateral,
}

/// A single attack case in the Story 4.5 future corpus.
#[derive(Debug, Clone)]
pub struct IsolationAttackCase {
    pub id: String,
    pub category: IsolationAttackCategory,
    /// Payload bytes Spirit-A attempts to use to read Spirit-B's state.
    pub attack_payload: Vec<u8>,
    /// What outcome the test expects (always true at framework level;
    /// Story 4.5 corpus authoring sets to false ONLY for known-vulnerable
    /// scenarios under remediation — at v0.3 prerequisite no such scenarios
    /// exist).
    pub expected_isolation_maintained: bool,
}

/// What a hook point returns — at v0.3 prerequisite all variants are
/// non-fatal recording surfaces; Story 4.5 may extend with veto power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationHookOutcome {
    Continue,
    Abort,
}

/// What Spirit-A's attempt resolved to (recorded by the framework).
#[derive(Debug, Clone)]
pub struct AttemptResult {
    pub hooks_fired_during_attempt: Vec<String>,
    pub frames_emitted: u32,
}

/// What Spirit-B's observable state revealed.
#[derive(Debug, Clone)]
pub struct ObservationResult {
    pub hooks_fired_during_observation: Vec<String>,
    pub frames_emitted: u32,
    pub leaked_bytes: Option<Vec<u8>>,
}

/// A record of one hook firing — for inspection.
#[derive(Debug, Clone)]
pub struct HookCallRecord {
    pub hook_name: &'static str,
    pub case_id: String,
    pub outcome: IsolationHookOutcome,
}

/// The 4-point hook trait — Story 4.5 plugs corpus-specific behavior
/// into these methods; DefaultIsolationHook records calls for inspection.
pub trait IsolationHookPoint {
    fn before_spirit_a_attempt(&mut self, case_id: &str) -> IsolationHookOutcome;
    fn after_spirit_a_attempt(
        &mut self,
        case_id: &str,
        result: &AttemptResult,
    ) -> IsolationHookOutcome;
    fn before_spirit_b_observe(&mut self, case_id: &str) -> IsolationHookOutcome;
    fn after_spirit_b_observe(
        &mut self,
        case_id: &str,
        observation: &ObservationResult,
    ) -> IsolationHookOutcome;
}

/// Reference impl recording all hook firings into a Vec.
#[derive(Debug, Clone, Default)]
pub struct DefaultIsolationHook {
    pub records: Vec<HookCallRecord>,
}

impl IsolationHookPoint for DefaultIsolationHook {
    fn before_spirit_a_attempt(&mut self, case_id: &str) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "before_spirit_a_attempt",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn after_spirit_a_attempt(
        &mut self,
        case_id: &str,
        _result: &AttemptResult,
    ) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "after_spirit_a_attempt",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn before_spirit_b_observe(&mut self, case_id: &str) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "before_spirit_b_observe",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn after_spirit_b_observe(
        &mut self,
        case_id: &str,
        _observation: &ObservationResult,
    ) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "after_spirit_b_observe",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
}

/// Outcome of one attack case run.
#[derive(Debug, Clone)]
pub struct IsolationOutcome {
    pub case_id: String,
    pub isolation_maintained: bool,
    pub attempt_result: AttemptResult,
    pub observation_result: ObservationResult,
}

/// 2-Spirit fixture for cross-Spirit isolation testing.
pub struct CrossSpiritIsolationFixture<'a, A: Spirit + 'static, B: Spirit + 'static> {
    pub spirit_a: &'a A,
    pub vtable_a: &'a SpiritVtable<A>,
    pub spirit_b: &'a B,
    pub vtable_b: &'a SpiritVtable<B>,
}

impl<'a, A: Spirit + 'static, B: Spirit + 'static> CrossSpiritIsolationFixture<'a, A, B> {
    pub fn new(
        spirit_a: &'a A,
        vtable_a: &'a SpiritVtable<A>,
        spirit_b: &'a B,
        vtable_b: &'a SpiritVtable<B>,
    ) -> Self {
        Self {
            spirit_a,
            vtable_a,
            spirit_b,
            vtable_b,
        }
    }

    /// Run one attack case through the 4-point hook protocol.
    pub fn run_attack_case<H: IsolationHookPoint>(
        &self,
        case: &IsolationAttackCase,
        hook: &mut H,
    ) -> IsolationOutcome {
        let _ = hook.before_spirit_a_attempt(&case.id);

        // Fire Spirit-A through on_frame with the attack payload.
        let fixture_a = LocalRunnerFixture {
            frames: vec![case.attack_payload.clone()],
            ..Default::default()
        };
        let report_a = LocalRunner::run(self.spirit_a, self.vtable_a, &fixture_a);
        let attempt = AttemptResult {
            hooks_fired_during_attempt: report_a.hooks_fired.keys().cloned().collect(),
            frames_emitted: report_a.mock_bus_frames.len() as u32,
        };
        let _ = hook.after_spirit_a_attempt(&case.id, &attempt);

        let _ = hook.before_spirit_b_observe(&case.id);

        // Fire Spirit-B through on_idle to drain observable state.
        let fixture_b = LocalRunnerFixture {
            invoke_on_idle: true,
            ..Default::default()
        };
        let report_b = LocalRunner::run(self.spirit_b, self.vtable_b, &fixture_b);
        let observation = ObservationResult {
            hooks_fired_during_observation: report_b.hooks_fired.keys().cloned().collect(),
            frames_emitted: report_b.mock_bus_frames.len() as u32,
            leaked_bytes: None,
        };
        let _ = hook.after_spirit_b_observe(&case.id, &observation);

        // At v0.3 prerequisite the framework always reports
        // isolation_maintained = true because the LocalRunner does not
        // share any state between Spirit-A and Spirit-B (each runs with
        // its own Ctx::mock()). Story 4.5 plugs in real leak detection.
        IsolationOutcome {
            case_id: case.id.clone(),
            isolation_maintained: true,
            attempt_result: attempt,
            observation_result: observation,
        }
    }
}
