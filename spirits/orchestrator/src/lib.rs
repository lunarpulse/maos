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
//!    kernel buffer is the `dequeue_at_safe_point` source the gate guards. In
//!    production the `maos run --epic` root's dispatch loop drains ONE
//!    instruction before each NEW section's first frame and appends it to that
//!    section's goal ([`fold_instruction`], Story 19-2 D-19-2-02 B).
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
//! 4. **Splits an epic deterministically** (Story 19-1, [`split_epic`]): one
//!    section per `### Story` heading, bound 1:1 in file order to the
//!    host-bearing Workers; follow-ups go out through
//!    [`Orchestrator::followup_dispatch_remote`]. Its Inference Port
//!    ([`Orchestrator::with_deferred_inference_port`]) serves only
//!    [`Orchestrator::judge_ambiguity`] — the `dispatch_ambiguity` scalar the
//!    kernel halts on; dispatch stays deterministic (Decision E).
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
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, PriorDistillateRef,
    TaskAssignPayload,
};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::orchestrator::OrchestratorInstruction;
use maos_domain::ports::context_budget::{ContextBudgetPort, ContextReportError, ContextStatus};
use maos_domain::ports::inference::{InferenceError, InferenceOptions, InferenceRequest};
use maos_domain::ports::InferencePort;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

pub mod epic;
pub use epic::{split_epic, EpicSection, EpicSplitError, STORY_HEADING};

/// Story 19-1 (D-19-1-12 A) — the epistemic tag the ambiguity judgement
/// reports; the manifest halts `on_value_above` 0.5 on it.
pub const SCALAR_TAG_DISPATCH_AMBIGUITY: &str = "dispatch_ambiguity";

/// The completion budget of one ambiguity judgement (one small JSON object).
const AMBIGUITY_MAX_TOKENS: u32 = 64;

/// Story 19-1 (AC3, mirrors `researcher::InferenceMint`) — mints a fresh
/// `ProviderInfer` token and the admitted pid for ONE judgement.
pub type InferenceMint = Arc<dyn Fn() -> Result<(CapabilityToken, u32), String> + Send + Sync>;

/// Story 19-1 — a host's audited `ProviderInfer` issuance for `(pid,
/// provider)` (the `maos run` root passes its seam's mint).
pub type ProviderMint = Arc<dyn Fn(u32, &str) -> Result<CapabilityToken, String> + Send + Sync>;

/// Story 19-1 — the Orchestrator's admitted pid once the scheduler assigned it.
pub type PidSource = Arc<dyn Fn() -> Option<u32> + Send + Sync>;

/// Story 19-1 (AC3, D19-4 A; mirrors `researcher::ResearcherInferenceBinding`)
/// — the provider the token and every request name, and the per-pass mint.
/// The composition root backfills it after admission.
#[derive(Clone)]
pub struct OrchestratorInferenceBinding {
    pub provider_id: String,
    pub mint: InferenceMint,
}

impl OrchestratorInferenceBinding {
    /// Each judgement mints at the pid `pid_of` resolves then.
    pub fn per_pass(provider_id: String, mint: ProviderMint, pid_of: PidSource) -> Self {
        let provider = provider_id.clone();
        Self {
            provider_id,
            mint: Arc::new(move || {
                let pid = pid_of().ok_or("the Spirit has no admitted pid yet")?;
                mint(pid, &provider).map(|token| (token, pid))
            }),
        }
    }
}

/// The bound Inference Port and its deferred binding.
#[derive(Clone)]
struct OrchestratorInference {
    port: Arc<dyn InferencePort + Send + Sync>,
    binding: Arc<Mutex<Option<OrchestratorInferenceBinding>>>,
}

/// Story 19-1 (D-19-1-12 A) — why an ambiguity judgement produced no scalar.
/// Every variant is typed; none falls back to a default value.
#[derive(Debug)]
pub enum OrchestratorInferenceError {
    /// No Inference Port is wired (unset `MAOS_INFERENCE_MODE`, ADR-064).
    NotWired,
    /// The binding is not backfilled yet, or the per-pass mint refused.
    Unbound(String),
    /// The port refused or failed the completion.
    Inference(InferenceError),
    /// The context report after the completion was refused.
    Context(ContextReportError),
    /// The completion is not a `dispatch_ambiguity` judgement.
    Completion(String),
}

impl std::fmt::Display for OrchestratorInferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWired => f.write_str(
                "orchestrator inference: no inference port is wired (the ambiguity judgement \
                 needs an explicit MAOS_INFERENCE_MODE)",
            ),
            Self::Unbound(reason) => write!(f, "orchestrator inference: unbound: {reason}"),
            Self::Inference(error) => write!(f, "orchestrator inference: {error}"),
            Self::Context(error) => write!(f, "orchestrator inference: {error}"),
            Self::Completion(reason) => write!(
                f,
                "orchestrator inference: completion is not a {SCALAR_TAG_DISPATCH_AMBIGUITY} \
                 judgement: {reason}"
            ),
        }
    }
}

impl std::error::Error for OrchestratorInferenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inference(error) => Some(error),
            Self::Context(error) => Some(error),
            _ => None,
        }
    }
}

/// Story 19-1 (D-19-1-12 A) — the ambiguity-judgement prompt for one epic
/// section's text ([`EpicSection::text`]). Deterministic for a given text, so
/// a recorded cassette's prompt hash is stable (the class corpus pins it).
pub fn ambiguity_prompt(section_text: &str) -> String {
    format!(
        "You are the MAOS Orchestrator. Before this story section is dispatched to a \
         Worker, judge how ambiguous its acceptance criteria are: could a competent Worker \
         build the wrong thing from this text alone? Reply with exactly one JSON object and \
         nothing else: {{\"{SCALAR_TAG_DISPATCH_AMBIGUITY}\": number in [0,1]}}. \
         {SCALAR_TAG_DISPATCH_AMBIGUITY}: 0.0 = unambiguous, every criterion is testable as \
         written; 1.0 = the section cannot be dispatched without the director's \
         clarification.\n\nSection:\n{section_text}"
    )
}

/// Story 19-1 (D-19-1-12 A) — parse a completion into its
/// `dispatch_ambiguity` value. The completion must be ONE JSON object whose
/// `dispatch_ambiguity` field is a number in `[0, 1]` (other fields are
/// tolerated). Anything else — an array, a bare scalar, a missing or
/// non-numeric field, a value outside `[0, 1]` — is a typed
/// [`OrchestratorInferenceError::Completion`], never a default.
pub fn parse_ambiguity(text: &str) -> Result<f64, OrchestratorInferenceError> {
    let completion: serde_json::Value = serde_json::from_str(text.trim())
        .map_err(|e| OrchestratorInferenceError::Completion(e.to_string()))?;
    let object = completion.as_object().ok_or_else(|| {
        OrchestratorInferenceError::Completion("the completion is not a JSON object".into())
    })?;
    let value = object
        .get(SCALAR_TAG_DISPATCH_AMBIGUITY)
        .ok_or_else(|| {
            OrchestratorInferenceError::Completion(format!(
                "field `{SCALAR_TAG_DISPATCH_AMBIGUITY}` is missing"
            ))
        })?
        .as_f64()
        .ok_or_else(|| {
            OrchestratorInferenceError::Completion(format!(
                "field `{SCALAR_TAG_DISPATCH_AMBIGUITY}` is not a number"
            ))
        })?;
    if !(0.0..=1.0).contains(&value) {
        return Err(OrchestratorInferenceError::Completion(format!(
            "field `{SCALAR_TAG_DISPATCH_AMBIGUITY}` must be in [0, 1], got {value}"
        )));
    }
    Ok(value)
}

/// j1-crosshost-1a AC1.4 — the consent intent a `developer-remote` delegation
/// carries.
///
/// ADR-012 names **effect authority**, not verbs and not job categories: its own
/// worked example contrasts `diagnosis-handoff:read-only-evidence` with
/// `code-mutation-directive`, and neither names a verb. What this frame actually
/// grants is a T3 worker running `codex exec --sandbox workspace-write` —
/// arbitrary code execution and filesystem mutation **on the receiver**. The
/// most powerful grant in the system must say so out loud, because a rung-2
/// operator reads this string when deciding whether to admit the frame from a
/// remote peer. `task:assign` describes the envelope; `development-task`
/// describes a job category; neither states the authority.
///
/// Canonical under `A2AIntent::is_canonical` (`maos-domain/src/invariants/i8.rs`):
/// `namespace:verb`, one colon, kebab segments. `task.assign` is NOT — the `.`
/// is rejected, and such a frame fails closed at `prepare_outbound`
/// (`maos-a2a-core/src/router.rs:696-701`) before it ever routes. Both facts are
/// pinned in `unit_tests::delegation_intent_is_canonical_and_task_assign_is_not`.
pub const DELEGATION_CONSENT_INTENT: &str = "development-task:write-workspace";

/// Story 19-2 (D-19-2-02 B) — the fixed delimiter a drained director
/// instruction follows in the next section's goal.
pub const DIRECTOR_INSTRUCTION_DELIMITER: &str =
    "\n\n---\nDirector instruction (queued, delivered at this safe point):\n";

/// Story 19-2 (D-19-2-02 B) — `goal` with ONE drained instruction appended
/// under [`DIRECTOR_INSTRUCTION_DELIMITER`]. One per safe point keeps the
/// Worker's single argv goal bounded; later instructions wait for later
/// safe points.
pub fn fold_instruction(goal: String, instruction: &OrchestratorInstruction) -> String {
    format!("{goal}{DIRECTOR_INSTRUCTION_DELIMITER}{}", instruction.goal)
}

/// A remote dispatch cannot cross a consent boundary with missing or malformed
/// provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteFrameError {
    EmptyIntentLineage,
    NonCanonicalIntent(String),
}

impl std::fmt::Display for RemoteFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIntentLineage => {
                f.write_str("remote frame intent lineage must not be empty")
            }
            Self::NonCanonicalIntent(intent) => {
                write!(
                    f,
                    "remote frame intent lineage contains non-canonical intent '{intent}'"
                )
            }
        }
    }
}

impl std::error::Error for RemoteFrameError {}

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
/// state keeps Orchestrator `Sync` as the `#[spirit]` macro requires; a clone
/// shares the bookkeeping, the deferred inference binding and the context
/// report (D-19-1-04 A: the root's loop drives a clone of the loaded instance).
#[derive(Clone)]
pub struct Orchestrator {
    /// This Orchestrator's own Spirit id (the `from` of every dispatch).
    orchestrator_id: String,
    /// In-flight delegation bookkeeping + last decision.
    state: Arc<Mutex<DispatchState>>,
    /// Story 19-1 (AC3) — the Inference Port; it serves only the ambiguity
    /// judgement, never a dispatch decision. `None` ⇒ deterministic (ADR-064).
    inference: Option<OrchestratorInference>,
    /// Story 19-1 (AC3) — the root's pid- and window-bound context reporter.
    context: Option<Arc<dyn ContextBudgetPort>>,
    /// The status of the most recent context report.
    last_context: Arc<Mutex<Option<ContextStatus>>>,
}

impl std::fmt::Debug for Orchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Orchestrator")
            .field("orchestrator_id", &self.orchestrator_id)
            .field("state", &self.state)
            .field("inference_bound", &self.inference.is_some())
            .field("context_bound", &self.context.is_some())
            .finish_non_exhaustive()
    }
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
            inference: None,
            context: None,
            last_context: Arc::new(Mutex::new(None)),
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

    // ── Story 19-1 (AC3, D19-4 A) — the inference seam ──────────────────────

    /// Wire the Inference Port before scheduler load (the Researcher shape:
    /// the binding names the provider and mints per judgement; `None` until
    /// the composition root backfills it after admission).
    pub fn with_deferred_inference_port(
        mut self,
        port: Arc<dyn InferencePort + Send + Sync>,
        binding: Arc<Mutex<Option<OrchestratorInferenceBinding>>>,
    ) -> Self {
        self.inference = Some(OrchestratorInference { port, binding });
        self
    }

    /// Story 18-5 shape — the root's pid- and window-bound context reporter.
    pub fn with_context_port(mut self, port: Arc<dyn ContextBudgetPort>) -> Self {
        self.context = Some(port);
        self
    }

    /// Whether the Inference Port is wired (any explicit inference mode).
    pub fn is_inference_bound(&self) -> bool {
        self.inference.is_some()
    }

    /// The status of the most recent context report.
    pub fn last_context(&self) -> Option<ContextStatus> {
        *self.last_context.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Story 19-1 (D-19-1-12 A) — judge one section's acceptance-criterion
    /// ambiguity through the bound port: one mint, one completion of
    /// [`ambiguity_prompt`], one context report, then [`parse_ambiguity`].
    /// Returns `(tag, value, derived_from)` for the kernel's scalar path; the
    /// kernel owns the halt. The judgement never changes the split, the
    /// section ↔ entry binding or any frame (dispatch stays deterministic).
    pub fn judge_ambiguity(
        &self,
        section: &EpicSection,
    ) -> Result<(&'static str, f64, String), OrchestratorInferenceError> {
        let live = self
            .inference
            .as_ref()
            .ok_or(OrchestratorInferenceError::NotWired)?;
        let binding = live
            .binding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| {
                OrchestratorInferenceError::Unbound("the inference binding is not ready".into())
            })?;
        let (token, pid) = (binding.mint)().map_err(OrchestratorInferenceError::Unbound)?;
        let request = InferenceRequest::new(
            pid,
            token,
            ambiguity_prompt(&section.text),
            InferenceOptions {
                max_tokens: AMBIGUITY_MAX_TOKENS,
                temperature: Some(0.0),
                model_id: None,
            },
            Some(binding.provider_id.clone()),
            Vec::new(),
        );
        let response = live
            .port
            .complete(request)
            .map_err(OrchestratorInferenceError::Inference)?;
        if let Some(port) = &self.context {
            let usage = &response.usage;
            let status = port
                .report_context_used(usage.input_tokens.saturating_add(usage.output_tokens))
                .map_err(OrchestratorInferenceError::Context)?;
            *self.last_context.lock().unwrap_or_else(|e| e.into_inner()) = Some(status);
        }
        let value = parse_ambiguity(&response.text)?;
        Ok((
            SCALAR_TAG_DISPATCH_AMBIGUITY,
            value,
            format!(
                "completion:{}:section-{}",
                binding.provider_id, section.ordinal
            ),
        ))
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

    /// Story 19-6 (D-19-6-03 A) — release a delegation whose Worker crashed,
    /// once its kernel-sent orphan frame was drained. Not a completion: the
    /// Worker produced nothing, so no distillate follows. `false` when nothing
    /// was in flight — a second release of one orphan, which the caller
    /// refuses rather than masking.
    pub fn release_orphaned_delegation(&self) -> bool {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let released = s.in_flight > 0;
        s.in_flight = s.in_flight.saturating_sub(1);
        released
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

    /// j1-crosshost-1a AC1.3 — the **remote** dispatch frame: the same
    /// `task.assign` [`Self::assign_frame`] builds, promoted onto the A2A wire.
    ///
    /// A NEW entry point rather than a widened [`Self::assign_frame`] signature:
    /// `assign_frame` has 12 call sites across 4 files and every one of them
    /// wants the same-Host shape (`host_id: None`, `consent_envelope: None`).
    ///
    /// Three fields the local shape leaves unset, and why each is load-bearing:
    ///
    /// * `to[..].host_id = to_host` — selects the Phase-3 cross-host branch in
    ///   `Mailbox::deliver` and picks the peer's `send_allowlist`.
    /// * `from.host_id = from_host` — the receiver identifies the peer by
    ///   `frame.from.host_id` and applies THAT peer's `accept_allowlist`
    ///   (`maos-a2a-core/src/router.rs:1087-1090`). On loopback the allowlist is
    ///   keyed by the SOURCE host, not the destination.
    /// * `consent_envelope` — `granter == frame.from` on BOTH `spirit_id` and
    ///   `host_id`, or intake NACKs `CODE_CONSENT_GRANTER_MISMATCH`
    ///   (`router.rs:1169-1206`). `None` fails closed at the sender's
    ///   `prepare_outbound` as `ConsentUnclassified`.
    ///
    /// `intent` names the **effect authority** the receiving host grants, per
    /// ADR-012 — see [`DELEGATION_CONSENT_INTENT`]. `lineage` must be non-empty
    /// with every entry canonical (AC1.5): an empty lineage on a Spirit-emitted
    /// cross-Spirit frame is rejected `EIntentLineageBroken` (architecture
    /// §7.3.2).
    ///
    /// Note (trap 19): [`ConsentEnvelope::with_fine_grained_intent`] mints a
    /// non-expiring envelope (`timestamp_ns = 0`, `valid_until_ns = None`). That
    /// is stated, not fixed — a consent TTL is `j1-crosshost-2` surface.
    #[allow(clippy::too_many_arguments)]
    pub fn assign_frame_remote(
        &self,
        seq: u64,
        run_nonce: u64,
        to_id: &str,
        to_role: SpiritRole,
        payload: TaskAssignPayload,
        lineage: IntentLineage,
        to_host: &str,
        from_host: &str,
        intent: A2AIntent,
    ) -> Result<IacFrame, RemoteFrameError> {
        if lineage.is_empty() {
            return Err(RemoteFrameError::EmptyIntentLineage);
        }
        if let Some(non_canonical) = lineage
            .as_slice()
            .iter()
            .find(|entry| !entry.is_canonical())
        {
            return Err(RemoteFrameError::NonCanonicalIntent(
                non_canonical.as_str().to_string(),
            ));
        }

        let mut frame = self.assign_frame(seq, to_id, to_role, payload, lineage);
        frame.frame_id[8..16].copy_from_slice(&run_nonce.to_le_bytes());
        for addr in frame.to.iter_mut() {
            addr.host_id = Some(HostId(to_host.to_string()));
        }
        frame.from.host_id = Some(HostId(from_host.to_string()));
        frame.consent_envelope = Some(ConsentEnvelope::with_fine_grained_intent(
            frame.from.clone(),
            intent,
        ));
        Ok(frame)
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

    /// Story 19-1 (AC4, D-19-1-06 A) — the REMOTE follow-up: the
    /// [`Self::followup_dispatch`] contract on the wire path production uses.
    /// `prior` is non-optional (FR21: a follow-up ALWAYS references the
    /// producer's `Distillate`, never raw output), and the frame is built by
    /// [`Self::assign_frame_remote`], so its lineage, host and consent rules
    /// (granter == `frame.from`, ADR-012) apply unchanged. The local
    /// [`Self::followup_dispatch`] stays the same-Host shape.
    #[allow(clippy::too_many_arguments)]
    pub fn followup_dispatch_remote(
        &self,
        seq: u64,
        run_nonce: u64,
        to_id: &str,
        to_role: SpiritRole,
        goal: impl Into<String>,
        success_criteria: impl Into<String>,
        prior: PriorDistillateRef,
        lineage: IntentLineage,
        to_host: &str,
        from_host: &str,
        intent: A2AIntent,
    ) -> Result<IacFrame, RemoteFrameError> {
        let payload = self.build_task_assign(goal, success_criteria, Some(prior));
        self.assign_frame_remote(
            seq, run_nonce, to_id, to_role, payload, lineage, to_host, from_host, intent,
        )
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
    fn orphaned_delegation_release_reopens_the_safe_point_once() {
        let o = Orchestrator::new("orchestrator");
        o.begin_delegation();
        assert!(
            o.release_orphaned_delegation(),
            "the in-flight delegation is released"
        );
        assert!(
            o.is_safe_point(),
            "FR20: the orphan re-opens the safe point"
        );
        assert!(
            !o.release_orphaned_delegation(),
            "a second release of the same orphan is refused, never a negative count"
        );
        assert_eq!(o.in_flight(), 0);
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

    // ── j1-crosshost-1a AC1.3-1.5 — the remote delegation emit ──────────────

    /// AC1.4 pin — the delegation intent is canonical under `i8.rs:85-115`, and
    /// `task.assign` is NOT. Without this pair the trap is unpinned: a `.` in a
    /// consent intent fails closed at `prepare_outbound` (`router.rs:696-701`)
    /// BEFORE routing, so the frame would never reach a peer and the failure
    /// would look like a transport bug.
    #[test]
    fn delegation_intent_is_canonical_and_task_assign_is_not() {
        assert!(
            A2AIntent::new(DELEGATION_CONSENT_INTENT).is_canonical(),
            "the delegation consent intent must be canonical (namespace:verb, one colon, kebab)"
        );
        assert!(
            !A2AIntent::new("task.assign").is_canonical(),
            "`task.assign` contains a `.` and is NOT a legal consent intent — it would fail \
             closed at prepare_outbound, never route"
        );
    }

    #[test]
    fn assign_frame_remote_sets_both_host_ids_and_a_granter_bound_envelope() {
        let o = Orchestrator::new("orchestrator");
        let payload = o.build_task_assign("delegated goal", "exit 0", None);
        let f = o
            .assign_frame_remote(
                9,
                0xA11CE,
                "developer-remote",
                SpiritRole::Worker,
                payload,
                IntentLineage::new(vec![A2AIntent::new(DELEGATION_CONSENT_INTENT)]),
                "developer-remote-host",
                "founder-loop-host",
                A2AIntent::new(DELEGATION_CONSENT_INTENT),
            )
            .expect("canonical, non-empty lineage builds a remote frame");

        assert_eq!(&f.frame_id[..8], &9u64.to_le_bytes());
        assert_eq!(&f.frame_id[8..], &0xA11CEu64.to_le_bytes());

        // AC1.3 — every recipient carries the destination host, and `from`
        // carries the sender host (the intake peer key, `router.rs:1087-1090`).
        assert!(!f.to.is_empty());
        for addr in f.to.iter() {
            assert_eq!(
                addr.host_id.as_ref().map(|h| h.as_str()),
                Some("developer-remote-host")
            );
        }
        assert_eq!(
            f.from.host_id.as_ref().map(|h| h.as_str()),
            Some("founder-loop-host")
        );

        // AC1.3 — granter binding (`router.rs:1169-1206`): granter == frame.from
        // on BOTH spirit_id and host_id, else intake NACKs
        // CODE_CONSENT_GRANTER_MISMATCH.
        let env = f.consent_envelope.as_ref().expect(
            "a remote frame must carry a consent envelope — None fails closed at the sender",
        );
        assert_eq!(env.granter.spirit_id, f.from.spirit_id);
        assert_eq!(env.granter.host_id, f.from.host_id);
        assert_eq!(
            env.intent_class.as_ref().map(|i| i.as_str()),
            Some(DELEGATION_CONSENT_INTENT)
        );
        // Trap 19 — `with_fine_grained_intent` never expires. Stated, not fixed:
        // a TTL is rung-2 surface (`j1-crosshost-2`).
        assert!(env.valid_until_ns.is_none());

        // AC1.5 — non-empty lineage whose every entry is canonical.
        assert!(!f.intent_lineage.is_empty());
        assert!(f
            .intent_lineage
            .as_slice()
            .iter()
            .all(A2AIntent::is_canonical));
    }

    #[test]
    fn assign_frame_remote_rejects_empty_lineage_before_recording_a_dispatch() {
        let o = Orchestrator::new("orchestrator");
        let payload = o.build_task_assign("delegated goal", "exit 0", None);
        let err = o
            .assign_frame_remote(
                1,
                2,
                "developer-remote",
                SpiritRole::Worker,
                payload,
                IntentLineage::default(),
                "developer-remote-host",
                "founder-loop-host",
                A2AIntent::new(DELEGATION_CONSENT_INTENT),
            )
            .expect_err("empty remote lineage must fail closed");
        assert_eq!(err, RemoteFrameError::EmptyIntentLineage);
        assert_eq!(o.dispatched_count(), 0);
    }

    #[test]
    fn assign_frame_remote_rejects_non_canonical_lineage_before_recording_a_dispatch() {
        let o = Orchestrator::new("orchestrator");
        let payload = o.build_task_assign("delegated goal", "exit 0", None);
        let err = o
            .assign_frame_remote(
                1,
                2,
                "developer-remote",
                SpiritRole::Worker,
                payload,
                IntentLineage::new(vec![A2AIntent::new("task.assign")]),
                "developer-remote-host",
                "founder-loop-host",
                A2AIntent::new(DELEGATION_CONSENT_INTENT),
            )
            .expect_err("non-canonical remote lineage must fail closed");
        assert_eq!(
            err,
            RemoteFrameError::NonCanonicalIntent("task.assign".to_string())
        );
        assert_eq!(o.dispatched_count(), 0);
    }

    #[test]
    fn assign_frame_remote_leaves_local_assign_frame_untouched() {
        let o = Orchestrator::new("orchestrator");
        let local = o.first_dispatch(1, "worker", SpiritRole::Worker, "a", "x", lineage());
        assert!(local.to.iter().all(|a| a.host_id.is_none()));
        assert!(local.from.host_id.is_none());
        assert!(local.consent_envelope.is_none());
    }
}
