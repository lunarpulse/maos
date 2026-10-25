#![forbid(unsafe_code)]

//! `butler` — MAOS Butler v0.3, the first cognitive reference Spirit (Story 8.1).
//!
//! Butler is an **anticipatory single-Spirit** (architecture §6.1). It fires
//! `on_idle`, reasons over a calendar/comms world-state (the "scenario
//! input"), and surfaces two things:
//!
//! 1. **Anticipatory reasoning** — calendar-conflict detection + comms triage,
//!    producing an *uncertainty proxy* (`belief_variance`) and a
//!    *preference-drift proxy* (`user_preference_drift`). The Spirit computes
//!    the proxies; the **kernel** does the universal-arithmetic comparison
//!    against the manifest `[epistemic_policy]` and fires the halt (Story 4.2 /
//!    4.1). Butler never decides a halt itself — see [`Assessment::primary_scalar`].
//! 2. **Morning digest** (FR17, Spirit-side) — composed from the Story 3.4
//!    `ranged_recall` log-composition primitive over the last-24h window, with
//!    every claimed completion citing a `source_log_ref` (the 16-byte frame id
//!    obtained via [`maos_audit::query`], which `ranged_recall` does not
//!    expose). Persisted through the Story 4.4 distillation port (`DistillationPort`)
//!    so the kernel-enforced I11 audit chain (`EDigestAuditChainMissing`) is
//!    never bypassed Spirit-side.
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI, the `maos-audit` read free-fns,
//! and the pure `maos-domain` distillation/notification types. It NEVER reaches
//! into `maos-kernel-core`. The halt + I11 chain are PROVEN against the real
//! kernel adapters in `tests/`, which carry kernel crates as dev-dependencies
//! only (dev-deps do not enter the kernel-API surface the boundary gate guards).

use std::path::Path;
use std::sync::Arc;

use maos_audit::log_composition::{ranged_recall, ComposedPayload, LogRange, LogSource};
use maos_audit::{query, AuditError, AuditFilter};
use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::halt::HaltReceipt;
use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::notification::NotificationEvent;
use maos_domain::ports::inference::{InferenceError, InferenceOptions, InferenceRequest};
use maos_domain::ports::mcp::McpError;
use maos_domain::ports::{EpistemicScalarPort, InferencePort};
use maos_spirit_sdk::clock::{local_day, Clock, SystemClock};
use maos_spirit_sdk::principal::PrincipalMemoryPort;
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

pub mod acceptance;
pub use acceptance::{
    parse_option, AcceptanceFixture, AcceptanceLedger, AcceptanceRecord, AcceptanceWindow,
    FixtureDay, OptionAction, Response, Scene, ACCEPTANCE_INTENT,
};

// ───────────────────────────────────────────────────────────────────────────
// Story 8.14b — ButlerMcpPort (FORK 1) + option-pick dispatch (FORK 2).
// ───────────────────────────────────────────────────────────────────────────

/// MCP error variants specific to Butler's domain port layer (FORK 1).
/// Distinct from `ButlerError` (the digest path); this covers the MCP call
/// + option-pick flow.
#[derive(Debug, thiserror::Error)]
pub enum ButlerMcpError {
    /// An MCP tool call failed (transport, decode, or server error).
    #[error("MCP call failed: {server}/{tool}: {cause}")]
    CallFailed {
        server: String,
        tool: String,
        cause: McpError,
    },
    /// Token issuance for the MCP call was denied by the capability registry.
    #[error("token issuance failed for MCP call")]
    TokenIssuanceFailed,
    /// The MCP call was denied because the Spirit's manifest does not declare
    /// the requested server/tool scope.
    #[error("unauthorized MCP call: capability scope mismatch")]
    Unauthorized,
    /// `handle_option_pick` was called but no notification is pending.
    #[error("no pending notification to pick from")]
    NoPendingNotification,
    /// Story 18-4 — the pick names no option of the pending notification;
    /// nothing is consumed.
    #[error("pick {pick} is out of range: the notification offers {options} option(s)")]
    PickOutOfRange { pick: usize, options: usize },
    /// Story 18-4 (D-18-4-03 A) — the picked option declares no known action.
    #[error("the picked option declares no known action: {0}")]
    UndeclaredAction(String),
    /// Story 18-4 (D-18-4-02 A) — the decision could not be recorded in the
    /// principal namespace or attested in the TL; its effect did not run.
    #[error("acceptance record: {0}")]
    Ledger(String),
}

/// Story 8.14b FORK 1 — the MCP domain port Butler's `on_idle` calls to
/// fetch real calendar/comms data and write a Linear note on option pick.
///
/// `async-trait` in butler, no `spawn_blocking`, no tokio in `[dependencies]`.
/// MCP calls are low-frequency (once per `on_idle` cycle), not inner loops.
#[async_trait::async_trait]
pub trait ButlerMcpPort: Send + Sync {
    /// Fetch calendar events from the real Calendar MCP server.
    async fn calendar_events(&self) -> Result<Vec<CalendarEvent>, ButlerMcpError>;
    /// Fetch comms messages from the real Slack MCP server.
    async fn comms_messages(&self) -> Result<Vec<CommsMessage>, ButlerMcpError>;
    /// Write a Linear note (issue) with the given title and content.
    async fn write_linear_note(&self, title: &str, content: &str) -> Result<(), ButlerMcpError>;
    /// Fetch a Figma file summary (design context for future reasoning).
    async fn fetch_figma_summary(&self) -> Result<serde_json::Value, ButlerMcpError>;
}

/// The notification payload Butler stores from its last `on_idle` pass, so
/// `handle_option_pick` can dispatch the selected option. FORK 2 state.
///
/// Story 18-1 (D-18-1-03 A): carries `options`, the manifest's fourth
/// `[output_shape]` field — the model's options when the Inference Port is
/// bound, the three `handle_option_pick` picks on the deterministic path.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationPayload {
    /// The pattern Butler detected (e.g. "calendar-conflict").
    pub pattern: String,
    /// Confidence in the detection [0.0, 1.0].
    pub confidence: f32,
    /// Human-readable evidence string.
    pub evidence: String,
    /// The director's options for this notification.
    pub options: Vec<String>,
    /// The conflict summary used as content for a Linear note (option a).
    pub conflict_summary: String,
}

/// Story 18-1 (D-18-1-03 A) — the deterministic path's options. Story 18-4
/// (D-18-4-03 A): each names its action, `"<action>: <text>"`, and
/// [`Butler::handle_option_pick`] runs the action the picked option names.
pub const DETERMINISTIC_OPTIONS: [&str; 3] = [
    "linear_note: Create a Linear issue to track the conflict resolution",
    "reminder: Send a Slack reminder to reschedule",
    "snooze: Snooze and revisit later",
];

/// Story 18-1 (D-18-1-03 A) — the completion contract: ONE JSON object.
/// Every field is required; a missing or ill-typed field is a typed
/// [`ButlerError::Proposal`], never a default.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ButlerProposal {
    pub pattern: String,
    pub confidence: f32,
    pub evidence: String,
    pub options: Vec<String>,
    pub user_preference_drift: f32,
}

/// Parse a completion into a [`ButlerProposal`]. Typed refusal on a missing
/// or ill-typed field, a unit-interval value outside `[0, 1]`, no option, or
/// (review EC-6) a blank `pattern`, `evidence` or option; Story 18-4
/// (D-18-4-03 A): an option with an unknown, missing or duplicate action.
pub fn parse_proposal(text: &str) -> Result<ButlerProposal, ButlerError> {
    let proposal: ButlerProposal = serde_json::from_str(text.trim())
        .map_err(|e| ButlerError::Proposal(format!("completion is not a Butler proposal: {e}")))?;
    for (field, value) in [
        ("confidence", proposal.confidence),
        ("user_preference_drift", proposal.user_preference_drift),
    ] {
        if !(0.0..=1.0).contains(&value) {
            return Err(ButlerError::Proposal(format!(
                "field `{field}` must be in [0, 1], got {value}"
            )));
        }
    }
    if proposal.options.is_empty() {
        return Err(ButlerError::Proposal(
            "field `options` must name at least one option".into(),
        ));
    }
    for (field, value) in [
        ("pattern", proposal.pattern.as_str()),
        ("evidence", proposal.evidence.as_str()),
    ]
    .into_iter()
    .chain(proposal.options.iter().map(|o| ("options", o.as_str())))
    {
        if value.trim().is_empty() {
            return Err(ButlerError::Proposal(format!(
                "field `{field}` must not be blank"
            )));
        }
    }
    acceptance::check_options(&proposal.options).map_err(ButlerError::Proposal)?;
    Ok(proposal)
}

/// Story 18-1 (AC1) — the prompt, built from the scenario (calendar, comms)
/// and the arithmetic conflict set. Deterministic for a given scenario, so a
/// recorded cassette's prompt hash is stable.
pub fn proposal_prompt(
    scenario: &ScenarioInput,
    assessment: &Assessment,
) -> Result<String, ButlerError> {
    let scenario_json = serde_json::to_string(scenario)
        .map_err(|e| ButlerError::Internal(format!("scenario encode: {e}")))?;
    let conflicts = assessment
        .conflicts
        .iter()
        .map(|c| format!("{}~{}", c.a, c.b))
        .collect::<Vec<_>>()
        .join(",");
    // Review BH-1/EC-5: the scale's direction is stated — `primary_scalar`
    // reads the value as alignment and the manifest halts `on_value_below`.
    // Story 18-4 (D-18-4-03 A): the option vocabulary is closed and stated.
    Ok(format!(
        "You are Butler, the director's anticipatory assistant. Read the calendar and \
         comms below and propose ONE notification. Reply with exactly one JSON object \
         and nothing else: {{\"pattern\": string, \"confidence\": number in [0,1], \
         \"evidence\": string, \"options\": [string, ...], \"user_preference_drift\": \
         number in [0,1]}}. Each option is \"<action>: <text>\" where <action> is one of \
         linear_note, reminder, snooze, each at most once; picking the option runs that \
         action. user_preference_drift: 1.0 = fully aligned with the \
         director's preferences; lower = more drift.\n\nUnresolved calendar conflicts: \
         [{conflicts}]\n\nScenario: {scenario_json}"
    ))
}

/// Story 18-1 (D-18-1-05 A) — mints a fresh `ProviderInfer` token and the
/// admitted pid for ONE `on_idle` pass (the composition root's audited,
/// policy-checked issuance; 60 s TTL per mint).
pub type InferenceMint = Arc<dyn Fn() -> Result<(CapabilityToken, u32), String> + Send + Sync>;

/// Story 18-1 (D-18-1-02 A) — the binding the composition root backfills
/// after admission: the provider Butler's token names (its first declared
/// `ProviderInfer`) and the per-pass mint. The request names the same id.
#[derive(Clone)]
pub struct ButlerInferenceBinding {
    pub provider_id: String,
    pub mint: InferenceMint,
}

/// The bound Inference Port and its deferred binding.
#[derive(Clone)]
struct ButlerInference {
    port: Arc<dyn InferencePort + Send + Sync>,
    binding: Arc<std::sync::Mutex<Option<ButlerInferenceBinding>>>,
}

/// Story 18-1 — a host's audited `ProviderInfer` issuance for `(pid,
/// provider)`. The `maos run` root's mint captures its capability registry
/// weakly: the seam outlives the root's audit-drain points.
pub type ProviderMint = Arc<dyn Fn(u32, &str) -> Result<CapabilityToken, String> + Send + Sync>;

/// Story 18-1 — a Spirit's admitted pid once the scheduler assigned it.
pub type PidSource = Arc<dyn Fn() -> Option<u32> + Send + Sync>;

/// Story 18-1 — builds a bound Inference Port over the host's router; `None`
/// once the host's kernel objects are gone.
pub type PortBuilder = Arc<dyn Fn() -> Option<Arc<dyn InferencePort + Send + Sync>> + Send + Sync>;

/// Story 18-4 (D-18-4-02 A) — builds the principal port the host fuses to
/// the pid it resolved; the Spirit never names a pid.
pub type PrincipalPortFor = Arc<dyn Fn(u32) -> Arc<dyn PrincipalMemoryPort> + Send + Sync>;

/// Story 18-4 (D-18-4-02 A) — the host's TL write of one decision's
/// non-personal attestation `(pid, payload)`; an error refuses the effect.
pub type AttestationSink = Arc<dyn Fn(u32, &[u8]) -> Result<(), String> + Send + Sync>;

/// Story 18-4 (CR-4) — what binds a Butler's acceptance record: the
/// director's principal id (from the scene fixture, D-18-4-05 A), the
/// pid-fused port factory and the attestation sink.
#[derive(Clone)]
pub struct AcceptanceSeam {
    pub principal: String,
    pub port_for: PrincipalPortFor,
    pub attest: AttestationSink,
}

/// A Butler's bound acceptance record: restored once its pid resolves.
struct AcceptanceBinding {
    seam: AcceptanceSeam,
    pid_of: PidSource,
    restored: std::sync::Mutex<bool>,
}

/// Story 18-1 (D-16-1-W; review ruling D-18-1-R1) — what a host attaches to
/// EVERY Butler it builds (the `maos run` boot arm, the operator door's
/// `load`, the hot-swap successor), so a successor is as bound as its
/// predecessor — never the arithmetic fallback D-18-1-04 A forbids.
#[derive(Clone)]
pub struct ButlerSeam {
    /// `--live` / explicit mode: the port builder and the host router's
    /// registered provider ids. `None` ⇒ deterministic (ADR-064).
    pub inference: Option<(PortBuilder, Vec<String>)>,
    /// The host's mint (the `maos run` root also mints the Researcher's
    /// token with it).
    pub mint: ProviderMint,
    /// The FR17 digest renderer.
    pub digest: Arc<MorningDigestRenderer>,
    /// A serving host also checks the day after every pass; `--once` renders
    /// after its pass instead.
    pub rollover: bool,
    /// Story 18-4 (CR-1) — the host's clock (the wall clock at
    /// `MAOS_UTC_OFFSET`, or the scene's scenario clock); the renderer reads
    /// the same one.
    pub clock: Arc<dyn Clock>,
    /// Story 18-4 (D-18-4-02/05 A) — `Some` only when a principal is known
    /// (the scene fixture's header); a serving daemon has no pick input.
    pub acceptance: Option<AcceptanceSeam>,
}

impl ButlerSeam {
    /// Bind `butler` (D-18-1-02/05 A): its token names its first declared
    /// `ProviderInfer` (`declared`) and the request names the same id; the
    /// mint runs per pass at the pid `pid_of` resolves. Review ruling
    /// D-18-1-R2: a declared provider the router does not register is
    /// refused here — at boot or load — never pass by pass.
    ///
    /// Story 18-4 (CR-4): the clock and the acceptance record bind here too,
    /// so the boot arm, the door load and the hot-swap successor read one
    /// clock and restore the record from the principal namespace at the pid
    /// `pid_of` resolves (on the first pass or decision, once it resolves).
    pub fn attach(
        &self,
        mut butler: Butler,
        declared: impl FnOnce() -> Option<String>,
        pid_of: PidSource,
    ) -> Result<Butler, String> {
        butler = butler.with_clock(Arc::clone(&self.clock));
        if let Some(seam) = &self.acceptance {
            butler.acceptance = Some(Arc::new(AcceptanceBinding {
                seam: seam.clone(),
                pid_of: Arc::clone(&pid_of),
                restored: std::sync::Mutex::new(false),
            }));
        }
        if let Some((build_port, registered)) = self.inference.clone() {
            let provider = declared().ok_or("butler inference: the Spirit declares no provider")?;
            if !registered.contains(&provider) {
                return Err(format!(
                    "butler inference: no inference provider is configured for its declared \
                     provider `{provider}`"
                ));
            }
            let port = build_port().ok_or("butler inference: the host is shutting down")?;
            eprintln!("butler inference seam wired (provider `{provider}`)");
            let (mint, pid) = (Arc::clone(&self.mint), Arc::clone(&pid_of));
            let binding = ButlerInferenceBinding {
                provider_id: provider.clone(),
                mint: Arc::new(move || {
                    let pid = pid().ok_or("the Spirit has no admitted pid yet")?;
                    mint(pid, &provider).map(|token| (token, pid))
                }),
            };
            butler = butler
                .with_deferred_inference_port(port, Arc::new(std::sync::Mutex::new(Some(binding))));
        }
        if self.rollover {
            let digest = Arc::clone(&self.digest);
            butler = butler.with_post_pass_hook(Arc::new(move || {
                digest.render_logged(pid_of().unwrap_or(0));
            }));
        }
        Ok(butler)
    }
}

/// Story 18-1 — the TL intent of the rendered morning digest's frame.
pub const MORNING_DIGEST_INTENT: &str = "butler:morning_digest";

/// Story 18-1 (D-18-1-08 A, AC3) — the FR17 morning digest on the production
/// path, over the host's own Transparency Log and lifecycle journal, once per
/// local day (Story 18-4, D-18-4-06 A: the day of the injected [`Clock`] at
/// its fixed offset): `{"event":"morning_digest"}` on stdout plus a
/// [`MORNING_DIGEST_INTENT`] TL frame the host's `frame_sink` writes
/// (review AA-2).
pub struct MorningDigestRenderer {
    audit_db: std::path::PathBuf,
    journal_path: std::path::PathBuf,
    frame_sink: Arc<dyn Fn(u32, &[u8]) + Send + Sync>,
    clock: Arc<dyn Clock>,
    rendered_day: std::sync::Mutex<Option<u64>>,
}

impl MorningDigestRenderer {
    pub fn new(
        audit_db: std::path::PathBuf,
        journal_path: std::path::PathBuf,
        frame_sink: Arc<dyn Fn(u32, &[u8]) + Send + Sync>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            audit_db,
            journal_path,
            frame_sink,
            clock,
            rendered_day: std::sync::Mutex::new(None),
        }
    }

    /// Render now (the clock's instant) unless today's digest was already
    /// rendered.
    pub fn render_if_new_day(&self, pid: u32) -> Result<(), String> {
        self.render_at(self.clock.now_ns(), pid).map(drop)
    }

    /// [`Self::render_if_new_day`] for a serving host: a failure is logged.
    pub fn render_logged(&self, pid: u32) {
        if let Err(error) = self.render_if_new_day(pid) {
            eprintln!("maos run: {error}");
        }
    }

    /// Render at `now_ns` unless a digest was already rendered for that local
    /// day or a later one (a clock stepped back across midnight never
    /// renders a duplicate; review EC-7). Returns the event when it rendered.
    pub fn render_at(&self, now_ns: u64, pid: u32) -> Result<Option<serde_json::Value>, String> {
        let offset = self.clock.utc_offset_secs();
        let day = local_day(now_ns, offset);
        let mut rendered_day = self
            .rendered_day
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if rendered_day.is_some_and(|d| day <= d) {
            return Ok(None);
        }
        let digest = Butler::new()
            .production_morning_digest(&self.audit_db, &self.journal_path, now_ns, offset)
            .map_err(|error| format!("butler morning digest: {error}"))?;
        let event = serde_json::json!({
            "event": "morning_digest",
            "now_ns": now_ns,
            "digest": digest,
        });
        let line = event.to_string();
        (self.frame_sink)(pid, line.as_bytes());
        println!("{line}");
        *rendered_day = Some(day);
        Ok(Some(event))
    }
}

/// Outcome of a director option pick (FORK 2; Story 18-4 D-18-4-03/04 A:
/// the picked option's declared action, and what its effect was).
#[derive(Debug, Clone, Serialize)]
pub struct OptionPickOutcome {
    /// The action the picked option declared (`linear_note` / `reminder` /
    /// `snooze`).
    pub action: OptionAction,
    /// A pick whose action is not `snooze` (D-18-4-01 A).
    pub accepted: bool,
    /// `written`, `refused (mcp unconfigured)`, `stub (live send v0.4)` or
    /// `snoozed` — never "written" for an effect that did not run.
    pub effect: String,
    /// true if a Linear note was written.
    pub linear_note_written: bool,
    /// true if a Slack reminder was queued (stub until v0.4).
    pub reminder_set: bool,
    /// true if the notification was snoozed.
    pub snoozed: bool,
    /// The clock instant a snooze re-checks at (clock + 15 min).
    pub snooze_until_ns: Option<u64>,
    /// Human-readable message for the shell to render.
    pub message: String,
}

/// Butler's own Spirit id — the `spirit_id` it writes scalars under and the id
/// every digest cite / halt frame is attributed to.
pub const BUTLER_SPIRIT_ID: &str = "butler";

/// Butler's spirit_pid at v0.3-β. The daemon is single-Spirit (consistent with
/// the rest of the composition root, which uses pid 0); Story 8.11 threads the
/// real per-Spirit pid through the production port.
pub const BUTLER_SPIRIT_PID: u32 = 0;

/// Story 8.11 AC5(f) — the scalar **tag** Butler writes for a calendar-conflict
/// epistemic halt. This is the canary constant shared between the production
/// daemon (`maos-bin`) and the JB-3 journey harness (`maos-journey-test`): JB-3
/// references THIS constant, not a string literal, so any drift (the sketch's
/// erroneous `self.belief_variance` → production `belief_variance`) is a COMPILE
/// error, not a silent body edit (Story 8.11 corrected the sketch mismatch).
pub const SCALAR_TAG_BELIEF_VARIANCE: &str = "belief_variance";

/// Story 8.11 AC5(f) — the rendered halt screen-string the `maos run` daemon
/// prints to stdout when an epistemic halt fires (`"halted on <tag>"`). The JB-3
/// PTY screen assertion binds to THIS function output, so the production halt
/// render-string and the harness assertion can never drift apart.
pub fn halt_screen_line(tag: &str) -> String {
    format!("halted on {tag}")
}

// ───────────────────────────────────────────────────────────────────────────
// Scenario world-state (the non-scored `input` object of each corpus row).
// ───────────────────────────────────────────────────────────────────────────

/// Lifecycle status of a calendar event. Only `Confirmed` events participate
/// in an *unresolved* conflict — a `Tentative`/`Cancelled` overlap is
/// auto-resolvable and does NOT warrant a halt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// A calendar event in the scenario world-state. Times are minutes-of-day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_min: u32,
    pub end_min: u32,
    pub status: EventStatus,
}

/// A comms message (e.g. a Slack thread) in the scenario world-state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommsMessage {
    pub id: String,
    pub from: String,
    /// 0..=3 — higher is more urgent.
    pub urgency: u8,
    pub awaiting_reply: bool,
}

/// The world-state Butler's `on_idle` reasons over. In production these come
/// from real Calendar/Slack MCP drivers; at v0.3 they are served by the
/// fixture-replay MCP provider seeded per corpus scenario (Decision B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ScenarioInput {
    #[serde(default)]
    pub calendar: Vec<CalendarEvent>,
    #[serde(default)]
    pub comms: Vec<CommsMessage>,
    /// Director-preference alignment in [0.0, 1.0]; `None` ⇒ fully aligned
    /// (1.0). A low value is the comms-triage *drift* signal.
    #[serde(default)]
    pub preference_alignment: Option<f32>,
}

// ───────────────────────────────────────────────────────────────────────────
// Anticipatory reasoning output.
// ───────────────────────────────────────────────────────────────────────────

/// A detected overlap between two confirmed calendar events.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConflictPair {
    pub a: String,
    pub b: String,
}

/// A single comms-triage outcome (highest-urgency awaiting-reply first).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TriagedComms {
    pub id: String,
    pub urgency: u8,
    pub needs_attention: bool,
}

/// The result of Butler's anticipatory pass over a scenario.
///
/// `belief_variance` / `user_preference_drift` are *proxies* — the kernel
/// compares them against the manifest `[epistemic_policy]` thresholds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Assessment {
    pub conflicts: Vec<ConflictPair>,
    pub triage: Vec<TriagedComms>,
    pub belief_variance: f32,
    pub user_preference_drift: f32,
    /// Story 18-4 (D-18-4-01 A) — the rolling acceptance window, `Some` only
    /// when it informed `belief_variance` (≥ 7 decided offers); its
    /// per-work-mode rates are the halt's diagnostic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<AcceptanceWindow>,
}

impl Assessment {
    /// The single scalar Butler writes for this scenario — the most salient
    /// concern. The kernel decides whether it crosses the `[epistemic_policy]`
    /// threshold; Butler does NOT hardcode the thresholds.
    ///
    /// Returns `(tag, value, derived_from)`. Story 18-4: a ledger-informed
    /// `belief_variance` is the scalar whenever the window informs it.
    pub fn primary_scalar(&self) -> (&'static str, f64, String) {
        if let Some(window) = &self.acceptance {
            (
                SCALAR_TAG_BELIEF_VARIANCE,
                self.belief_variance as f64,
                format!("acceptance-window:{}/{}", window.accepted, window.offers),
            )
        } else if !self.conflicts.is_empty() {
            let refs = self
                .conflicts
                .iter()
                .map(|c| format!("{}~{}", c.a, c.b))
                .collect::<Vec<_>>()
                .join(",");
            (
                SCALAR_TAG_BELIEF_VARIANCE,
                self.belief_variance as f64,
                format!("calendar-conflict:{refs}"),
            )
        } else if self.user_preference_drift < 1.0 {
            (
                "user_preference_drift",
                self.user_preference_drift as f64,
                "comms-preference-drift".to_string(),
            )
        } else {
            (
                SCALAR_TAG_BELIEF_VARIANCE,
                self.belief_variance as f64,
                "no-conflict".to_string(),
            )
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Morning digest (FR17, Spirit-side).
// ───────────────────────────────────────────────────────────────────────────

/// A task completed in the last-24h window, with its citeable audit ref.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompletedTask {
    pub intent: String,
    pub outcome: String,
    /// 32-char hex of the 16-byte Transparency-Log frame id (the cite).
    pub source_log_ref: String,
}

/// An open halt requiring director resolution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenHalt {
    pub intent: String,
    pub source_log_ref: String,
}

/// A flagged anomaly that cleared the confidence floor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnomalyItem {
    pub observer: String,
    pub subject: String,
    pub summary: String,
    pub confidence: f32,
    /// Story 18-4 (D-18-4-07 A) — the `anomaly.flagged:` TL frame it was read
    /// from, on the production path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_log_ref: Option<String>,
}

/// The composed FR17 morning digest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MorningDigest {
    /// (a) tasks completed in the last 24h with outcome tags.
    pub completed: Vec<CompletedTask>,
    /// (b) open halts requiring resolution.
    pub open_halts: Vec<OpenHalt>,
    /// (c) flagged anomalies with confidence ≥ the floor.
    pub anomalies: Vec<AnomalyItem>,
    /// (d) trust bar reflecting yesterday's predicate-fire rate (1.0 = full trust).
    pub trust_bar: f32,
}

/// Anomaly confidence floor for digest item (c) — architecture §6.1 / AC5.
pub const ANOMALY_CONFIDENCE_FLOOR: f32 = 0.6;

// ── Frame kind constants (avoid stringly-typed matching) ────────────────────
const FRAME_KIND_TASK_COMPLETE: &str = "task.complete";
const FRAME_KIND_EPISTEMIC_HALT: &str = "epistemic.halt";
const FRAME_KIND_TELEMETRY_EVENT: &str = "telemetry.event";

/// Nanoseconds in a day (the digest's day is the [`Clock`]'s local day).
pub const NANOS_PER_DAY: u64 = maos_spirit_sdk::clock::NANOS_PER_DAY;

/// Story 18-1 (review ruling D-18-1-R3) — the Transparency Log intent prefix
/// of ONE predicate evaluation. The composition root writes a
/// `telemetry.event` frame `epistemic.evaluated:<tag>` for every scalar write
/// it evaluates against `[epistemic_policy]`: the TL records no pass or hook
/// fire (measured: an `on_idle` pass writes no frame of its own, and the
/// deterministic path writes no inference frame), so this frame is the
/// fire rate's denominator.
pub const INTENT_PREDICATE_EVALUATED: &str = "epistemic.evaluated:";

/// Story 18-4 (D-18-4-07 A) — the TL intent prefix of a surfaced anomaly:
/// `anomaly.flagged:<source>`, a `telemetry.event` frame whose payload is the
/// `NotificationEvent::AnomalyFlagged` fields.
pub const INTENT_ANOMALY_FLAGGED: &str = "anomaly.flagged:";

/// Story 18-4 (D-18-4-07 A) — which anomaly producers journal
/// [`INTENT_ANOMALY_FLAGGED`] frames, and which do not yet (the Observer's
/// structural anomalies): an empty `(c)` reads against this basis.
pub const ANOMALY_SOURCES_WIRED: [&str; 1] = ["escape-detector"];
pub const ANOMALY_SOURCES_UNWIRED: [&str; 1] = ["observer"];

/// FR17 (d)'s input: yesterday's predicate fires over predicate evaluations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PredicateFireRate {
    /// The local day (days since the epoch at the clock's offset) the rate
    /// covers.
    pub local_day: u64,
    /// `epistemic.halt` frames whose intent is an evaluated tag.
    pub fires: usize,
    /// `epistemic.evaluated:<tag>` frames.
    pub evaluations: usize,
}

impl PredicateFireRate {
    /// Fires over evaluations, in `[0, 1]`; 0.0 when nothing was evaluated
    /// (no predicate could fire).
    pub fn rate(&self) -> f32 {
        if self.evaluations == 0 {
            0.0
        } else {
            (self.fires as f32 / self.evaluations as f32).min(1.0)
        }
    }
}

/// Story 18-1 (review ruling D-18-1-R3) — FR17 (d)'s source: over the local
/// day before `now_ns` (Story 18-4, D-18-4-06 A: at `utc_offset_secs`), the
/// `epistemic.halt` frames whose intent is a tag with an
/// `epistemic.evaluated:<tag>` frame (a predicate fire; planned-unload
/// receipts and halts of tags never evaluated are not fires), over those
/// evaluation frames.
pub fn yesterday_predicate_fire_rate(
    audit_db: &Path,
    now_ns: u64,
    utc_offset_secs: i32,
) -> Result<PredicateFireRate, ButlerError> {
    use maos_spirit_sdk::clock::local_day_start_ns;
    let today = local_day(now_ns, utc_offset_secs);
    let yesterday = today.saturating_sub(1);
    let entries = query(
        audit_db,
        AuditFilter {
            since_ns: Some(local_day_start_ns(yesterday, utc_offset_secs)),
            until_ns: Some(local_day_start_ns(today, utc_offset_secs).saturating_sub(1)),
            ..Default::default()
        },
    )
    .map_err(ButlerError::Audit)?;
    let evaluated: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == FRAME_KIND_TELEMETRY_EVENT)
        .filter_map(|e| e.intent.strip_prefix(INTENT_PREDICATE_EVALUATED))
        .collect();
    let fires = entries
        .iter()
        .filter(|e| e.kind == FRAME_KIND_EPISTEMIC_HALT && evaluated.contains(&e.intent.as_str()))
        .count();
    Ok(PredicateFireRate {
        local_day: yesterday,
        fires,
        evaluations: evaluated.len(),
    })
}

/// Errors Butler's digest and (Story 18-1) inference paths can surface.
#[derive(Debug)]
pub enum ButlerError {
    /// The audit read (`ranged_recall` / `query`) failed.
    Audit(AuditError),
    /// A digest item cited a malformed `source_log_ref` hex.
    BadFrameIdHex(String),
    /// Building the distillation request violated I11.
    Distillation(DistillationError),
    /// Internal consistency check failed (e.g. composed vs query views disagree).
    Internal(String),
    /// Story 18-1 — the bound Inference Port has no binding yet, or the
    /// per-pass token mint was refused.
    Unbound(String),
    /// Story 18-1 — `InferencePort::complete` failed (provider error,
    /// capability denial, cassette exhaustion or strict drift).
    Inference(InferenceError),
    /// Story 18-1 (D-18-1-03 A) — the completion is not a valid proposal.
    Proposal(String),
    /// Story 18-8 AC5 (review ruling D-18-8-R4, D-18-1-04 A) — bound, and
    /// BOTH MCP fetches failed with something other than `Unconfigured`:
    /// the pass is refused before `complete` (no model call).
    McpUnavailable {
        calendar: ButlerMcpError,
        comms: ButlerMcpError,
    },
    /// Story 18-4 (D-18-4-02 A) — the acceptance record could not be
    /// restored from (or written to) the principal namespace.
    Ledger(String),
}

impl std::fmt::Display for ButlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButlerError::Audit(e) => write!(f, "audit read error: {e}"),
            ButlerError::BadFrameIdHex(s) => write!(f, "malformed source_log_ref hex: {s}"),
            ButlerError::Distillation(e) => write!(f, "distillation error: {e}"),
            ButlerError::Internal(s) => write!(f, "internal consistency error: {s}"),
            ButlerError::Unbound(s) => write!(f, "inference unavailable: {s}"),
            ButlerError::Inference(e) => write!(f, "inference failed: {e}"),
            ButlerError::Proposal(s) => write!(f, "malformed proposal: {s}"),
            ButlerError::McpUnavailable { calendar, comms } => {
                write!(
                    f,
                    "MCP data unavailable: calendar: {calendar}; comms: {comms}"
                )
            }
            ButlerError::Ledger(s) => write!(f, "acceptance record: {s}"),
        }
    }
}
impl std::error::Error for ButlerError {}

// ───────────────────────────────────────────────────────────────────────────
// The Butler Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Butler reference Spirit. Optionally holds the scenario its `on_idle` pass
/// reasons over (the kernel/fixture-replay seeds this in production).
#[derive(Clone)]
pub struct Butler {
    pending: Option<ScenarioInput>,
    /// The assessment from the most recent `on_idle` firing. Persists so the
    /// computation is observable (not silently discarded) and testable.
    /// Thread-safe (`Arc<Mutex<..>>`) so Butler remains `Sync` as required
    /// by the `#[spirit]` macro.
    last_assessment: Arc<std::sync::Mutex<Option<Assessment>>>,
    /// Story 8.10 AC1 — the epistemic-scalar **write** port. When `Some`,
    /// `on_idle` drives the assessed scalar through the kernel policy path so
    /// the `[epistemic_policy]` halt can fire. `None` (the test/daemon default
    /// pre-8.11) is store-only — the `None`-footgun closed by construction in
    /// Story 8.11 (production port non-`Option` or boot-loud).
    scalar_port: Option<Arc<dyn EpistemicScalarPort>>,
    /// The `Option<HaltReceipt>` from the most recent `on_idle` scalar write, so
    /// the firing is OBSERVABLE (not silently discarded). `None` when no port is
    /// wired or no halt fired.
    last_halt_receipt: Arc<std::sync::Mutex<Option<HaltReceipt>>>,
    /// Story 8.14b FORK 1 — the MCP domain port for fetching real calendar/comms
    /// data and writing Linear notes. When `Some`, `on_idle` calls it to build
    /// the `ScenarioInput` from real data instead of using `self.pending`.
    mcp_port: Option<Arc<dyn ButlerMcpPort>>,
    /// Story 8.14b FORK 2 — the pending notification from the last `on_idle`,
    /// so `handle_option_pick` can dispatch the selected option.
    /// `parking_lot::Mutex` — Butler is `Send + Sync` behind `Arc`; `parking_lot`
    /// preferred over `std::sync::Mutex` for non-poisoning behavior.
    last_notification: Arc<parking_lot::Mutex<Option<NotificationPayload>>>,
    /// JB-5 — shared output channel for daemon-level output_shape validation.
    /// The daemon sets this before loading the Spirit; Butler serializes its
    /// notification to JSON here during `on_idle`. After `fire_on_idle` returns,
    /// the daemon reads from this channel and validates against the manifest's
    /// `OutputShapePredicate`.
    output_channel: Option<Arc<std::sync::Mutex<Option<serde_json::Value>>>>,
    /// Story 18-1 (AC1) — the bound Inference Port. `None` ⇒ the deterministic
    /// path (ADR-064: unset mode never touches the router).
    inference: Option<ButlerInference>,
    /// Story 18-1 (D-18-1-04 A) — the typed refusal of the most recent pass,
    /// taken by the composition root (`--once` exits non-zero on it).
    pass_error: Arc<std::sync::Mutex<Option<ButlerError>>>,
    /// Story 18-1 (D-18-1-08 A) — run after every `on_idle` pass; the
    /// composition root renders the morning digest from it on a day rollover.
    post_pass_hook: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Story 18-4 (CR-1) — the clock the renderer, the snooze and the
    /// acceptance window read (the wall clock in UTC unless a host binds one).
    clock: Arc<dyn Clock>,
    /// Story 18-4 (D-18-4-01 A) — the director's decisions, one per local day.
    ledger: Arc<std::sync::Mutex<AcceptanceLedger>>,
    /// Story 18-4 (D-18-4-02 A) — the principal-namespace binding of the
    /// record (`None` ⇒ in-memory only: no principal is known).
    acceptance: Option<Arc<AcceptanceBinding>>,
    /// Story 18-4 — notifications stored so far (the attestation's ref).
    notifications: Arc<std::sync::atomic::AtomicU64>,
}

impl std::fmt::Debug for Butler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Butler")
            .field("pending", &self.pending)
            .field("has_scalar_port", &self.scalar_port.is_some())
            .field("has_mcp_port", &self.mcp_port.is_some())
            .field("has_inference", &self.inference.is_some())
            .finish()
    }
}

#[spirit]
impl Butler {
    /// Anticipatory idle pass. Cancellation-aware; bounded (it does a single
    /// linear pass over the pending scenario, well within `time_cap_seconds`).
    ///
    /// Story 8.14b: when `mcp_port` is `Some`, fetches real calendar/comms
    /// data via the MCP domain port and builds a live `ScenarioInput`. Falls
    /// back to `self.pending` when `mcp_port` is `None` (backwards-compat
    /// preserved — existing smoke tests unchanged).
    ///
    /// Story 18-1: the post-pass hook (the composition root's morning-digest
    /// rollover check) runs after every pass, refused or not.
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        self.idle_pass();
        if let Some(hook) = &self.post_pass_hook {
            hook();
        }
    }
}

impl Default for Butler {
    fn default() -> Self {
        Self {
            pending: None,
            last_assessment: Arc::new(std::sync::Mutex::new(None)),
            scalar_port: None,
            last_halt_receipt: Arc::new(std::sync::Mutex::new(None)),
            mcp_port: None,
            last_notification: Arc::new(parking_lot::Mutex::new(None)),
            output_channel: None,
            inference: None,
            pass_error: Arc::new(std::sync::Mutex::new(None)),
            post_pass_hook: None,
            clock: Arc::new(SystemClock::UTC),
            ledger: Arc::default(),
            acceptance: None,
            notifications: Arc::default(),
        }
    }
}

impl Butler {
    /// The body of [`Butler::on_idle`].
    fn idle_pass(&self) {
        // Story 18-4 (D-18-4-02 A) — the record is restored before the first
        // pass reads the window; a failed restore refuses the pass typed.
        if let Err(error) = self.ensure_restored() {
            return self.refuse(error);
        }
        // Story 8.14b — build the scenario from real MCP data when the port
        // is wired (--live), otherwise fall back to the injected pending
        // scenario (fixture-replay / smoke tests).
        let scenario = if let Some(mcp) = &self.mcp_port {
            // MCP calls are low-frequency (once per on_idle cycle), not inner
            // loops. block_on is safe here because on_idle runs inside
            // spawn_blocking and the underlying MCP client is sync.
            let calendar = block_on_sync(mcp.calendar_events());
            let comms = block_on_sync(mcp.comms_messages());
            match (calendar, comms) {
                // Story 18-8 AC5 (review ruling D-18-8-R4): bound, and BOTH
                // fetches failed with something other than `Unconfigured` —
                // the model would reason over nothing, so the pass is refused
                // before `complete`. One unconfigured server plus one failed
                // fetch (or any partial data) is not "both": it proceeds.
                (Err(calendar), Err(comms))
                    if self.inference.is_some()
                        && !is_unconfigured(&calendar)
                        && !is_unconfigured(&comms) =>
                {
                    return self.refuse(ButlerError::McpUnavailable { calendar, comms });
                }
                (calendar, comms) => Some(ScenarioInput {
                    calendar: fetched("calendar", calendar),
                    comms: fetched("comms", comms),
                    preference_alignment: None,
                }),
            }
        } else {
            self.pending.clone()
        };

        if let Some(scenario) = &scenario {
            // Run the anticipatory reasoning and store the result so it is
            // observable (not silently discarded).
            let mut assessment = self.assess(scenario);
            let conflict_summary = assessment
                .conflicts
                .iter()
                .map(|c| format!("{} ↔ {}", c.a, c.b))
                .collect::<Vec<_>>()
                .join("; ");

            let notification = match &self.inference {
                // Story 18-1 (D-18-1-03/04 A) — bound: the proposal and
                // `user_preference_drift` are the model's. A failed or
                // malformed completion refuses the pass typed: no
                // notification, no scalar write, never the arithmetic one.
                // Review ruling D-18-1-R6: the model is called every pass
                // (the drift scalar is its), but a notification is stored and
                // emitted only when conflicts exist, as on the deterministic
                // path.
                Some(live) => match self.infer_proposal(live, scenario, &assessment) {
                    Ok(proposal) => {
                        assessment.user_preference_drift = proposal.user_preference_drift;
                        (!assessment.conflicts.is_empty()).then(|| NotificationPayload {
                            pattern: proposal.pattern,
                            confidence: proposal.confidence,
                            evidence: proposal.evidence,
                            options: proposal.options,
                            conflict_summary,
                        })
                    }
                    Err(error) => return self.refuse(error),
                },
                // Story 8.14b FORK 2 — if there are conflicts, store a
                // notification payload so handle_option_pick can dispatch.
                None if !assessment.conflicts.is_empty() => Some(NotificationPayload {
                    pattern: "calendar-conflict".into(),
                    confidence: assessment.belief_variance,
                    evidence: format!(
                        "{} unresolved calendar conflict(s)",
                        assessment.conflicts.len()
                    ),
                    options: DETERMINISTIC_OPTIONS
                        .iter()
                        .map(|o| (*o).to_owned())
                        .collect(),
                    conflict_summary,
                }),
                None => None,
            };
            if let Some(notification) = notification {
                *self.last_notification.lock() = Some(notification);
                self.notifications
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }

            // JB-5 — serialize the notification to the output channel so the
            // daemon can validate it against the manifest's OutputShapePredicate.
            if let Some(ch) = &self.output_channel {
                let guard = self.last_notification.lock();
                if let Some(ref notification) = *guard {
                    let json = serde_json::to_value(notification).unwrap_or_default();
                    *ch.lock().unwrap() = Some(json);
                }
            }

            // Story 8.10 AC1 — when the epistemic-scalar port is wired, drive
            // the assessed scalar through the kernel policy path so the
            // `[epistemic_policy]` halt fires.
            if let Some(port) = &self.scalar_port {
                let (tag, value, derived_from) = assessment.primary_scalar();
                let result = port.write_scalar(
                    BUTLER_SPIRIT_PID,
                    BUTLER_SPIRIT_ID,
                    tag,
                    value,
                    &derived_from,
                );
                let mut guard = self
                    .last_halt_receipt
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(receipt) => *guard = receipt,
                    Err(_) => *guard = None,
                }
            }

            let mut guard = self.last_assessment.lock().unwrap();
            *guard = Some(assessment);
        }
    }

    /// Story 18-1 (AC1) — one `InferencePort::complete` per pass: mint the
    /// pass's token, name the bound provider, parse the completion.
    fn infer_proposal(
        &self,
        live: &ButlerInference,
        scenario: &ScenarioInput,
        assessment: &Assessment,
    ) -> Result<ButlerProposal, ButlerError> {
        let binding = live
            .binding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| ButlerError::Unbound("the inference binding is not ready".into()))?;
        let (token, spirit_pid) = (binding.mint)().map_err(ButlerError::Unbound)?;
        let req = InferenceRequest::new(
            spirit_pid,
            token,
            proposal_prompt(scenario, assessment)?,
            InferenceOptions {
                max_tokens: 512,
                temperature: Some(0.0),
                model_id: None,
            },
            Some(binding.provider_id),
            Vec::new(),
        );
        let response = live.port.complete(req).map_err(ButlerError::Inference)?;
        parse_proposal(&response.text)
    }

    /// A Butler with no pending scenario (digest-only / production default).
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_scenario(scenario: ScenarioInput) -> Self {
        Self {
            pending: Some(scenario),
            ..Self::default()
        }
    }

    /// Story 18-4 (CR-1) — read `clock` instead of the wall clock: the
    /// renderer's day, the snooze time and the acceptance window.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Story 18-4 — today's local day on Butler's clock.
    fn today(&self) -> u64 {
        local_day(self.clock.now_ns(), self.clock.utc_offset_secs())
    }

    /// Story 18-4 (D-18-4-01 A) — the rolling window `[today - 7, today - 1]`
    /// on Butler's clock.
    pub fn acceptance_window(&self) -> AcceptanceWindow {
        self.ledger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .window(self.today())
    }

    /// The bound record's pid-fused port, once the pid resolved.
    fn principal_port(binding: &AcceptanceBinding) -> Result<Arc<dyn PrincipalMemoryPort>, String> {
        let pid = (binding.pid_of)().ok_or("the Spirit has no admitted pid yet")?;
        Ok((binding.seam.port_for)(pid))
    }

    /// Story 18-4 (D-18-4-02 A) — restore the record from the principal
    /// namespace once per binding; prints
    /// `{"event":"acceptance_ledger_restored","records":N,"pid":p}`. A
    /// restored day never overwrites one already decided in this process.
    fn ensure_restored(&self) -> Result<(), ButlerError> {
        let Some(binding) = &self.acceptance else {
            return Ok(());
        };
        let mut restored = binding.restored.lock().unwrap_or_else(|e| e.into_inner());
        if *restored {
            return Ok(());
        }
        let port = Self::principal_port(binding).map_err(ButlerError::Ledger)?;
        let markdown = port
            .read(
                &binding.seam.principal,
                acceptance::ACCEPTANCE_SCHEMA,
                acceptance::ACCEPTANCE_KEY,
            )
            .map_err(ButlerError::Ledger)?;
        let record = match markdown {
            Some(markdown) => {
                AcceptanceLedger::from_markdown(&markdown).map_err(ButlerError::Ledger)?
            }
            None => AcceptanceLedger::default(),
        };
        println!(
            "{}",
            serde_json::json!({
                "event": "acceptance_ledger_restored",
                "records": record.len(),
                "pid": port.pid(),
            })
        );
        self.ledger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .merge_restored(record);
        *restored = true;
        Ok(())
    }

    /// Write the whole record to the principal namespace (Markdown, the
    /// durable Private-tier kind).
    fn persist(&self) -> Result<(), String> {
        let Some(binding) = &self.acceptance else {
            return Ok(());
        };
        let markdown = self
            .ledger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .to_markdown()?;
        Self::principal_port(binding)?.write(
            &binding.seam.principal,
            acceptance::ACCEPTANCE_SCHEMA,
            acceptance::ACCEPTANCE_KEY,
            &markdown,
        )
    }

    /// Story 18-4 (AC2.4, I2) — record one decision in the principal
    /// namespace and attest it in the TL (decision id, notification ref,
    /// action kind, scenario time: nothing personal) BEFORE its effect.
    fn record_decision(&self, record: AcceptanceRecord) -> Result<(), String> {
        self.ensure_restored().map_err(|e| e.to_string())?;
        let attestation = serde_json::json!({
            "decision_id": record.local_day,
            "notification_ref": self.notifications.load(std::sync::atomic::Ordering::SeqCst),
            "action": record.action,
            "sim_ns": record.sim_ns,
        })
        .to_string();
        self.ledger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record);
        self.persist()?;
        if let Some(binding) = &self.acceptance {
            let pid = (binding.pid_of)().ok_or("the Spirit has no admitted pid yet")?;
            (binding.seam.attest)(pid, attestation.as_bytes())?;
        }
        Ok(())
    }

    /// Story 18-4 (CR-3) — seed decisions taken before the scene (history
    /// days ≤ 0, picks indexing [`DETERMINISTIC_OPTIONS`]); written to the
    /// principal namespace, not attested (no notification was offered).
    pub fn seed_acceptance_history(&self, fixture: &AcceptanceFixture) -> Result<(), ButlerError> {
        self.ensure_restored()?;
        {
            let mut ledger = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
            for day in fixture.history() {
                let action = match day.response {
                    Response::Ignored => None,
                    Response::Pick(n) => Some(
                        DETERMINISTIC_OPTIONS
                            .get(n - 1)
                            .and_then(|option| parse_option(option).ok())
                            .map(|(action, _)| action)
                            .ok_or_else(|| {
                                ButlerError::Ledger(format!(
                                    "history day {}: pick {n} names no seed option",
                                    day.day
                                ))
                            })?,
                    ),
                };
                ledger.insert(AcceptanceRecord {
                    local_day: fixture.local_day(day.day),
                    sim_ns: fixture.day_ns(day.day),
                    work_mode: day.work_mode.clone(),
                    response: day.response_raw.clone(),
                    action: action.map_or("ignored", OptionAction::name).to_owned(),
                    accepted: action.is_some_and(|a| a != OptionAction::Snooze),
                });
            }
        }
        self.persist().map_err(ButlerError::Ledger)
    }

    /// Story 18-4 (CR-2, AC1) — the Spirit-side half of one simulated day,
    /// after the host's pass verdict: the halt line (the scene ends) or the
    /// director's fixture response through [`Self::handle_option_pick`] /
    /// [`Self::ignore_pending`] and the `acceptance_day` line. Returns whether
    /// the halt fired. Stdout carries scenario values only (ADR-028 row d).
    pub fn scene_step(&self, day: &FixtureDay) -> Result<bool, String> {
        let assessment = self
            .last_assessment()
            .ok_or_else(|| format!("butler scene day {}: the pass assessed nothing", day.day))?;
        let window = self.acceptance_window();
        if self.last_halt_receipt().is_some() {
            let (tag, _, _) = assessment.primary_scalar();
            println!(
                "{}",
                serde_json::json!({
                    "event": "halt",
                    "render": halt_screen_line(tag),
                    "day": day.day,
                    "belief_variance": assessment.belief_variance,
                    "diagnostic": {"rate": window.rate, "by_mode": window.by_mode},
                })
            );
            return Ok(true);
        }
        let (action, accepted, effect) = match day.response {
            Response::Pick(n) => {
                let outcome = self
                    .handle_option_pick(n, &day.work_mode, None)
                    .map_err(|e| format!("butler scene day {}: pick {n}: {e}", day.day))?;
                (outcome.action.name(), outcome.accepted, outcome.effect)
            }
            Response::Ignored => {
                self.ignore_pending(&day.work_mode)
                    .map_err(|e| format!("butler scene day {}: ignored: {e}", day.day))?;
                ("ignored", false, "none".to_owned())
            }
        };
        println!(
            "{}",
            serde_json::json!({
                "event": "acceptance_day",
                "day": day.day,
                "work_mode": day.work_mode,
                "response": day.response_raw,
                "action": action,
                "accepted": accepted,
                "effect": effect,
                "window": window,
                "belief_variance": assessment.belief_variance,
            })
        );
        Ok(false)
    }

    /// Story 18-4 — the director let the pending notification go: a declined
    /// offer, recorded and attested; no effect runs.
    pub fn ignore_pending(&self, work_mode: &str) -> Result<(), ButlerMcpError> {
        let mut pending = self.last_notification.lock();
        if pending.is_none() {
            return Err(ButlerMcpError::NoPendingNotification);
        }
        self.record_decision(AcceptanceRecord {
            local_day: self.today(),
            sim_ns: self.clock.now_ns(),
            work_mode: work_mode.to_owned(),
            response: "ignored".into(),
            action: "ignored".into(),
            accepted: false,
        })
        .map_err(ButlerMcpError::Ledger)?;
        *pending = None;
        Ok(())
    }

    /// Story 18-1 (AC1) — wire the Inference Port before scheduler load,
    /// mirroring Researcher's builder. The composition root backfills the
    /// provider and the per-pass mint into `binding` after admission.
    pub fn with_deferred_inference_port(
        mut self,
        port: Arc<dyn InferencePort + Send + Sync>,
        binding: Arc<std::sync::Mutex<Option<ButlerInferenceBinding>>>,
    ) -> Self {
        self.inference = Some(ButlerInference { port, binding });
        self
    }

    /// Whether the Inference Port is bound (any explicit mode or `--live`).
    pub fn is_inference_bound(&self) -> bool {
        self.inference.is_some()
    }

    /// Story 18-1 (D-18-1-04 A) — take the typed refusal of the most recent
    /// pass, if it was refused.
    pub fn take_pass_error(&self) -> Option<ButlerError> {
        self.pass_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Refuse this pass typed (D-18-1-04 A): no notification, no output, no
    /// scalar write. Review BH-3 (18-1): a refused pass leaves no stale
    /// notification pending from an earlier pass.
    fn refuse(&self, error: ButlerError) {
        eprintln!("butler: on_idle pass refused: {error}");
        *self.last_notification.lock() = None;
        if let Some(ch) = &self.output_channel {
            *ch.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        *self.pass_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
    }

    /// Story 18-1 (D-18-1-08 A) — install the hook `on_idle` runs after
    /// every pass.
    pub fn with_post_pass_hook(mut self, hook: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.post_pass_hook = Some(hook);
        self
    }

    /// Story 8.10 AC1 — inject the epistemic-scalar **write** port (test/daemon
    /// builder). When set, `on_idle` drives the assessed scalar through the
    /// kernel policy path so the `[epistemic_policy]` halt can fire.
    pub fn with_scalar_port(mut self, port: Arc<dyn EpistemicScalarPort>) -> Self {
        self.scalar_port = Some(port);
        self
    }
    /// JB-5 — inject a shared output channel for daemon-level output_shape
    /// validation. Butler serializes its notification here during `on_idle`.
    pub fn with_output_channel(
        mut self,
        ch: Arc<std::sync::Mutex<Option<serde_json::Value>>>,
    ) -> Self {
        self.output_channel = Some(ch);
        self
    }

    /// Story 8.14b FORK 1 — inject the MCP domain port. When set, `on_idle`
    /// fetches real calendar/comms data via MCP instead of using `self.pending`.
    pub fn with_mcp_port(mut self, port: Arc<dyn ButlerMcpPort>) -> Self {
        self.mcp_port = Some(port);
        self
    }

    /// Story 8.14b FORK 2; Story 18-4 (D-18-4-03/04 A) — the director picks
    /// option `pick` (1-based) of the pending notification, and Butler runs
    /// the action THAT option declares (`"<action>: <text>"`), whatever its
    /// position: `linear_note` writes the option's text through `mcp_port`
    /// (none ⇒ the effect is refused, typed — never "written"), `reminder`
    /// is the v0.4 stub (labelled), `snooze` re-checks at the clock + 15 min.
    /// The decision is recorded and attested before the effect (I2).
    ///
    /// `NoPendingNotification` when nothing is pending; an out-of-range pick
    /// or an option with no declared action consumes nothing.
    pub fn handle_option_pick(
        &self,
        pick: usize,
        work_mode: &str,
        mcp_port: Option<&dyn ButlerMcpPort>,
    ) -> Result<OptionPickOutcome, ButlerMcpError> {
        let mut pending = self.last_notification.lock();
        let notification = pending
            .as_ref()
            .ok_or(ButlerMcpError::NoPendingNotification)?;
        let option = pick
            .checked_sub(1)
            .and_then(|i| notification.options.get(i))
            .ok_or(ButlerMcpError::PickOutOfRange {
                pick,
                options: notification.options.len(),
            })?;
        let (action, text) = parse_option(option).map_err(ButlerMcpError::UndeclaredAction)?;
        let text = text.to_owned();
        let now_ns = self.clock.now_ns();
        self.record_decision(AcceptanceRecord {
            local_day: self.today(),
            sim_ns: now_ns,
            work_mode: work_mode.to_owned(),
            response: format!("pick:{pick}"),
            action: action.name().to_owned(),
            accepted: action != OptionAction::Snooze,
        })
        .map_err(ButlerMcpError::Ledger)?;
        let Some(notification) = pending.take() else {
            return Err(ButlerMcpError::NoPendingNotification);
        };
        drop(pending);
        let mut outcome = OptionPickOutcome {
            action,
            accepted: action != OptionAction::Snooze,
            effect: String::new(),
            linear_note_written: false,
            reminder_set: false,
            snoozed: false,
            snooze_until_ns: None,
            message: String::new(),
        };
        match action {
            OptionAction::LinearNote => {
                let title = format!("Calendar conflict — {}", notification.conflict_summary);
                match mcp_port {
                    Some(port) => {
                        block_on_sync(port.write_linear_note(&title, &text))?;
                        outcome.linear_note_written = true;
                        outcome.effect = "written".into();
                        outcome.message = format!("Linear note written: {title}");
                    }
                    None => {
                        outcome.effect = "refused (mcp unconfigured)".into();
                        outcome.message =
                            "Butler: Linear note not written — no MCP port is configured".into();
                    }
                }
            }
            OptionAction::Reminder => {
                outcome.reminder_set = true;
                outcome.effect = "stub (live send v0.4)".into();
                outcome.message = format!("Butler: Slack message queued: {text} (live send v0.4)");
            }
            OptionAction::Snooze => {
                let until = now_ns + SNOOZE_NS;
                outcome.snoozed = true;
                outcome.snooze_until_ns = Some(until);
                outcome.effect = "snoozed".into();
                outcome.message = format!(
                    "Butler: snoozed — will re-check at {}",
                    clock_hhmm(until, self.clock.utc_offset_secs())
                );
            }
        }
        Ok(outcome)
    }

    /// The pending notification from the last `on_idle`, if any.
    pub fn last_notification(&self) -> Option<NotificationPayload> {
        self.last_notification.lock().clone()
    }

    /// The assessment from the most recent `on_idle` firing, if any.
    pub fn last_assessment(&self) -> Option<Assessment> {
        self.last_assessment.lock().unwrap().clone()
    }

    /// Story 8.10 AC1 — the `HaltReceipt` from the most recent `on_idle` scalar
    /// write, if a halt fired. `None` when no port is wired or no halt fired.
    /// Makes the firing observable for the seam test.
    pub fn last_halt_receipt(&self) -> Option<HaltReceipt> {
        self.last_halt_receipt
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Anticipatory reasoning over a scenario: calendar-conflict detection +
    /// comms triage → uncertainty/drift proxies. Deterministic and pure.
    pub fn assess(&self, scenario: &ScenarioInput) -> Assessment {
        let conflicts = detect_unresolved_conflicts(&scenario.calendar);

        // belief_variance: low when nothing warrants attention; climbs with
        // each unresolved confirmed-event overlap. One conflict already crosses
        // the §6.1 0.7 halt threshold the kernel checks.
        let conflict_proxy = if conflicts.is_empty() {
            0.2
        } else {
            (0.75 + 0.05 * (conflicts.len().min(5) as f32)).min(0.99)
        };
        // Story 18-4 (D-18-4-01 A) — once the rolling window holds ≥ 7
        // decided offers, the uncertainty is about the director: H₂ of the
        // pooled acceptance rate. Before that the conflict proxy stands.
        let window = self.acceptance_window();
        let (belief_variance, acceptance) = match window.belief_variance() {
            Some(entropy) => (entropy, Some(window)),
            None => (conflict_proxy, None),
        };

        // user_preference_drift: the comms-triage alignment signal. Fully
        // aligned (1.0) absent any signal. Reject NaN / out-of-range values
        // so they don't silently bypass the epistemic-policy halt rule.
        let user_preference_drift = scenario
            .preference_alignment
            .and_then(|v| {
                if v.is_nan() {
                    None
                } else {
                    Some(v.clamp(0.0, 1.0))
                }
            })
            .unwrap_or(1.0);

        let mut triage: Vec<TriagedComms> = scenario
            .comms
            .iter()
            .map(|m| TriagedComms {
                id: m.id.clone(),
                urgency: m.urgency,
                needs_attention: m.awaiting_reply && m.urgency >= 2,
            })
            .collect();
        triage.sort_by(|a, b| b.urgency.cmp(&a.urgency).then_with(|| a.id.cmp(&b.id)));

        Assessment {
            conflicts,
            triage,
            belief_variance,
            user_preference_drift,
            acceptance,
        }
    }

    /// Compose the FR17 morning digest from the last-24h audit window.
    ///
    /// Reads the composed narrative via Story 3.4 `ranged_recall` (the contract
    /// AC5 names) and obtains citeable 16-byte frame ids via [`maos_audit::query`]
    /// (which `ranged_recall` redacts away). `anomalies` are the director-surface
    /// notification events; only `AnomalyFlagged` with confidence ≥
    /// [`ANOMALY_CONFIDENCE_FLOOR`] are included.
    pub fn morning_digest(
        &self,
        audit_db: &Path,
        journal_path: &Path,
        now_ns: u64,
        anomalies: &[NotificationEvent],
        predicate_fire_rate_24h: f32,
    ) -> Result<MorningDigest, ButlerError> {
        let range = LogRange::last_24h(now_ns);

        // Story 3.4 composed view — establishes the narrative window + ordering
        // contract Butler relies on. (Open-halt presence is cross-checked below
        // against the citeable query rows.)
        let composed =
            ranged_recall(audit_db, journal_path, range, None).map_err(ButlerError::Audit)?;
        let composed_has_halt = composed.iter().any(|e| {
            e.source == LogSource::TransparencyLog
                && matches!(&e.payload, ComposedPayload::Frame { frame_kind, .. } if frame_kind == "epistemic.halt")
        });

        // Citeable rows (carry frame_id_hex). Same file, same window.
        let entries = query(
            audit_db,
            AuditFilter {
                since_ns: Some(range.since_ns),
                until_ns: Some(range.until_ns),
                ..Default::default()
            },
        )
        .map_err(ButlerError::Audit)?;

        let mut completed = Vec::new();
        let mut open_halts = Vec::new();
        for e in &entries {
            // Validate the frame id early — a malformed hex here indicates
            // corruption in the audit trail and must not propagate to the
            // distillation request where it would fail later with a less
            // useful error message.
            if e.frame_id_hex.len() != 32 || !e.frame_id_hex.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Err(ButlerError::BadFrameIdHex(format!(
                    "audit query returned invalid frame_id_hex: {}",
                    e.frame_id_hex
                )));
            }
            match e.kind.as_str() {
                FRAME_KIND_TASK_COMPLETE => completed.push(CompletedTask {
                    intent: e.intent.clone(),
                    outcome: outcome_tag(&e.intent),
                    source_log_ref: e.frame_id_hex.clone(),
                }),
                FRAME_KIND_EPISTEMIC_HALT => open_halts.push(OpenHalt {
                    intent: e.intent.clone(),
                    source_log_ref: e.frame_id_hex.clone(),
                }),
                _ => {}
            }
        }
        // The composed view and the citeable view must agree on halt presence —
        // a divergence would mean Butler is narrating from a different window.
        // This is a regular assertion (not debug-only) because a mismatch
        // indicates a serious consistency bug that must not go to production.
        if composed_has_halt != !open_halts.is_empty() {
            return Err(ButlerError::Internal(
                "composed and query views disagree on halt presence — journal/schema drift?".into(),
            ));
        }

        let anomalies = anomalies.iter().filter_map(anomaly_item).collect();

        // Guard NaN so the trust bar doesn't serialize to `null` downstream.
        let trust_bar = if predicate_fire_rate_24h.is_finite() {
            (1.0 - predicate_fire_rate_24h.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        Ok(MorningDigest {
            completed,
            open_halts,
            anomalies,
            trust_bar,
        })
    }

    /// Story 18-1 (D-18-1-08 A; review ruling D-18-1-R3) — the FR17 digest as
    /// the production path renders it: (a)/(b) over the last 24 h; (d) from
    /// [`yesterday_predicate_fire_rate`] on the local day, its basis carried
    /// beside it as `trust_bar_basis` with `utc_offset_secs`; Story 18-4
    /// (D-18-4-07 A) (c) from the window's [`INTENT_ANOMALY_FLAGGED`] frames
    /// through the same ≥ [`ANOMALY_CONFIDENCE_FLOOR`] filter, each citing its
    /// frame, with `anomalies_basis` naming the wired and unwired producers.
    pub fn production_morning_digest(
        &self,
        audit_db: &Path,
        journal_path: &Path,
        now_ns: u64,
        utc_offset_secs: i32,
    ) -> Result<serde_json::Value, ButlerError> {
        let basis = yesterday_predicate_fire_rate(audit_db, now_ns, utc_offset_secs)?;
        let mut digest = self.morning_digest(audit_db, journal_path, now_ns, &[], basis.rate())?;
        digest.anomalies = anomaly_feed(audit_db, now_ns)?;
        let mut value = serde_json::to_value(&digest)
            .map_err(|e| ButlerError::Internal(format!("digest encode: {e}")))?;
        value["anomalies_basis"] = serde_json::json!({
            "wired": ANOMALY_SOURCES_WIRED,
            "unwired": ANOMALY_SOURCES_UNWIRED,
        });
        value["trust_bar_basis"] = serde_json::to_value(basis)
            .map_err(|e| ButlerError::Internal(format!("trust-bar basis encode: {e}")))?;
        value["utc_offset_secs"] = serde_json::Value::from(utc_offset_secs);
        Ok(value)
    }

    /// Build the I11 distillation request for a digest. `source_log_ref` is the
    /// union of every cited completion + open-halt frame id (deduped, order
    /// preserved). The kernel re-validates this chain at write time.
    pub fn digest_to_distillation_request(
        &self,
        digest: &MorningDigest,
    ) -> Result<DistillationRequest, ButlerError> {
        let mut refs: Vec<[u8; 16]> = Vec::new();
        let mut seen: std::collections::HashSet<[u8; 16]> = std::collections::HashSet::new();
        for hex in digest
            .completed
            .iter()
            .map(|c| &c.source_log_ref)
            .chain(digest.open_halts.iter().map(|h| &h.source_log_ref))
        {
            let id = decode_frame_id_hex(hex)?;
            if seen.insert(id) {
                refs.push(id);
            }
        }
        let payload = DigestPayload::Json(
            serde_json::to_value(digest)
                .map_err(|e| ButlerError::BadFrameIdHex(format!("digest serialize: {e}")))?,
        );
        DistillationRequest::new(refs, 1, payload, None).map_err(ButlerError::Distillation)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Story 8.14b — sync/async bridge + stub helpers.
// ───────────────────────────────────────────────────────────────────────────

/// Story 18-8 AC5 — an unconfigured server is absent data, not a failure.
fn is_unconfigured(error: &ButlerMcpError) -> bool {
    matches!(
        error,
        ButlerMcpError::CallFailed {
            cause: McpError::Unconfigured,
            ..
        }
    )
}

/// A fetch's data, or none (logged) when it failed.
fn fetched<T>(what: &str, result: Result<Vec<T>, ButlerMcpError>) -> Vec<T> {
    result.unwrap_or_else(|e| {
        eprintln!("butler: {what} MCP fetch failed: {e}");
        vec![]
    })
}

/// Minimal single-threaded `block_on` for bridging async-trait futures inside
/// `on_idle` (which runs in `spawn_blocking`). The underlying MCP calls are
/// synchronous (`McpClientPort::call` is sync), so the future resolves
/// immediately — no waker parking needed.
fn block_on_sync<F: std::future::Future>(fut: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(val) => val,
        Poll::Pending => panic!(
            "ButlerMcpPort future returned Pending — MCP calls must resolve immediately \
             (sync McpClientPort under the hood)"
        ),
    }
}

/// Story 18-4 (D-18-4-04 A) — a snooze re-checks this long after the pick,
/// on Butler's clock. Real scheduling is v0.4.
const SNOOZE_NS: u64 = 15 * 60 * 1_000_000_000;

/// `HH:MM` of `ns` at the clock's offset (UTC when the offset is 0).
fn clock_hhmm(ns: u64, utc_offset_secs: i32) -> String {
    let local = (i128::from(ns / 1_000_000_000) + i128::from(utc_offset_secs)).max(0) as u64;
    let (hours, minutes) = ((local / 3600) % 24, (local % 3600) / 60);
    let zone = if utc_offset_secs == 0 { " UTC" } else { "" };
    format!("{hours:02}:{minutes:02}{zone}")
}

/// FR17 (c)'s filter: an `AnomalyFlagged` event at or above the floor.
fn anomaly_item(event: &NotificationEvent) -> Option<AnomalyItem> {
    match event {
        NotificationEvent::AnomalyFlagged {
            observer,
            subject,
            summary,
            confidence,
        } if confidence.is_finite() && *confidence >= ANOMALY_CONFIDENCE_FLOOR => {
            Some(AnomalyItem {
                observer: observer.clone(),
                subject: subject.clone(),
                summary: summary.clone(),
                confidence: *confidence,
                source_log_ref: None,
            })
        }
        _ => None,
    }
}

/// The payload of an [`INTENT_ANOMALY_FLAGGED`] frame.
#[derive(Deserialize)]
struct FlaggedPayload {
    observer: String,
    subject: String,
    summary: String,
    confidence: f32,
}

/// Story 18-4 (D-18-4-07 A) — FR17 (c) on the production path: the last
/// 24 h's [`INTENT_ANOMALY_FLAGGED`] `telemetry.event` frames, each citing
/// its frame; a frame whose payload is not the `AnomalyFlagged` fields is a
/// typed error, never a silent drop.
fn anomaly_feed(audit_db: &Path, now_ns: u64) -> Result<Vec<AnomalyItem>, ButlerError> {
    let range = LogRange::last_24h(now_ns);
    let entries = query(
        audit_db,
        AuditFilter {
            since_ns: Some(range.since_ns),
            until_ns: Some(range.until_ns),
            ..Default::default()
        },
    )
    .map_err(ButlerError::Audit)?;
    let mut items = Vec::new();
    for entry in entries.iter().filter(|e| {
        e.kind == FRAME_KIND_TELEMETRY_EVENT && e.intent.starts_with(INTENT_ANOMALY_FLAGGED)
    }) {
        let payload: FlaggedPayload = serde_json::from_str(&entry.payload).map_err(|e| {
            ButlerError::Internal(format!(
                "anomaly frame {} is not an AnomalyFlagged payload: {e}",
                entry.frame_id_hex
            ))
        })?;
        let event = NotificationEvent::AnomalyFlagged {
            observer: payload.observer,
            subject: payload.subject,
            summary: payload.summary,
            confidence: payload.confidence,
        };
        if let Some(mut item) = anomaly_item(&event) {
            item.source_log_ref = Some(entry.frame_id_hex.clone());
            items.push(item);
        }
    }
    Ok(items)
}

// ───────────────────────────────────────────────────────────────────────────
// Pure helpers.
// ───────────────────────────────────────────────────────────────────────────

/// Maximum confirmed events to check for conflicts before capping to stay
/// within the manifest `time_cap_seconds` budget. At 100 events the O(n²)
/// loop performs ~5k comparisons — well under the 30s cap on any modern CPU.
const MAX_EVENTS_FOR_CONFLICT_CHECK: usize = 100;

/// Detect overlapping pairs of CONFIRMED calendar events (an unresolved
/// conflict). Tentative/cancelled overlaps are auto-resolvable ⇒ ignored.
/// Capped at [`MAX_EVENTS_FOR_CONFLICT_CHECK`] to prevent a malformed
/// oversized input from blowing the `time_cap_seconds` budget.
fn detect_unresolved_conflicts(events: &[CalendarEvent]) -> Vec<ConflictPair> {
    let mut out = Vec::new();
    let confirmed: Vec<&CalendarEvent> = events
        .iter()
        .filter(|e| e.status == EventStatus::Confirmed)
        .take(MAX_EVENTS_FOR_CONFLICT_CHECK)
        .collect();
    for i in 0..confirmed.len() {
        for j in (i + 1)..confirmed.len() {
            let a = confirmed[i];
            let b = confirmed[j];
            // Half-open overlap [start, end).
            if a.start_min < b.end_min && b.start_min < a.end_min {
                out.push(ConflictPair {
                    a: a.id.clone(),
                    b: b.id.clone(),
                });
            }
        }
    }
    out
}

/// Derive a coarse outcome tag from a completion intent for digest item (a).
/// Uses whole-word matching to avoid false positives (e.g. "review failure
/// report" must NOT tag as failed).
fn outcome_tag(intent: &str) -> String {
    let lower = intent.to_ascii_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words
        .iter()
        .any(|w| matches!(*w, "fail" | "error" | "failed" | "errored"))
    {
        "failed".to_string()
    } else if words.iter().any(|w| matches!(*w, "cancel" | "cancelled")) {
        "cancelled".to_string()
    } else {
        "succeeded".to_string()
    }
}

/// Decode a 32-char hex string into a 16-byte frame id.
fn decode_frame_id_hex(hex: &str) -> Result<[u8; 16], ButlerError> {
    if hex.len() != 32 {
        return Err(ButlerError::BadFrameIdHex(hex.to_string()));
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        let s = &hex[i * 2..i * 2 + 2];
        *byte =
            u8::from_str_radix(s, 16).map_err(|_| ButlerError::BadFrameIdHex(hex.to_string()))?;
    }
    Ok(out)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn ev(id: &str, start: u32, end: u32, status: EventStatus) -> CalendarEvent {
        CalendarEvent {
            id: id.into(),
            title: id.into(),
            start_min: start,
            end_min: end,
            status,
        }
    }

    #[test]
    fn confirmed_overlap_is_an_unresolved_conflict() {
        let s = ScenarioInput {
            calendar: vec![
                ev("a", 540, 600, EventStatus::Confirmed),
                ev("b", 570, 630, EventStatus::Confirmed),
            ],
            ..Default::default()
        };
        let a = Butler::new().assess(&s);
        assert_eq!(a.conflicts.len(), 1);
        assert!(
            a.belief_variance > 0.7,
            "one conflict crosses the halt floor"
        );
        let (tag, value, _) = a.primary_scalar();
        assert_eq!(tag, "belief_variance");
        assert!(value > 0.7);
    }

    #[test]
    fn tentative_overlap_is_resolvable_no_conflict() {
        let s = ScenarioInput {
            calendar: vec![
                ev("a", 540, 600, EventStatus::Confirmed),
                ev("b", 570, 630, EventStatus::Tentative),
            ],
            ..Default::default()
        };
        let a = Butler::new().assess(&s);
        assert!(a.conflicts.is_empty());
        assert!(a.belief_variance < 0.7);
    }

    #[test]
    fn preference_drift_drives_the_drift_scalar() {
        let s = ScenarioInput {
            comms: vec![CommsMessage {
                id: "m1".into(),
                from: "ceo".into(),
                urgency: 3,
                awaiting_reply: true,
            }],
            preference_alignment: Some(0.4),
            ..Default::default()
        };
        let a = Butler::new().assess(&s);
        assert!(a.conflicts.is_empty());
        let (tag, value, _) = a.primary_scalar();
        assert_eq!(tag, "user_preference_drift");
        assert!(value < 0.6, "low alignment crosses the drift halt floor");
    }

    #[test]
    fn no_signal_picks_low_belief_variance() {
        let s = ScenarioInput::default();
        let a = Butler::new().assess(&s);
        let (tag, value, _) = a.primary_scalar();
        assert_eq!(tag, "belief_variance");
        assert!(value < 0.7);
    }

    #[test]
    fn decode_hex_roundtrip() {
        let id = [0xABu8; 16];
        let hex = id.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(decode_frame_id_hex(&hex).unwrap(), id);
        assert!(decode_frame_id_hex("zz").is_err());
    }
}

// Story 8.14b — FakeButlerMcpPort ships in the SAME commit as the trait (FORK 1 constraint).
#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A test double for `ButlerMcpPort` that returns canned data and records calls.
    pub struct FakeButlerMcpPort {
        pub calendar: Vec<CalendarEvent>,
        pub comms: Vec<CommsMessage>,
        pub write_linear_call_count: AtomicUsize,
    }

    impl FakeButlerMcpPort {
        pub fn with_conflict() -> Self {
            Self {
                calendar: vec![
                    CalendarEvent {
                        id: "evt-a".into(),
                        title: "Board review".into(),
                        start_min: 540,
                        end_min: 600,
                        status: EventStatus::Confirmed,
                    },
                    CalendarEvent {
                        id: "evt-b".into(),
                        title: "Investor call".into(),
                        start_min: 570,
                        end_min: 630,
                        status: EventStatus::Confirmed,
                    },
                ],
                comms: vec![],
                write_linear_call_count: AtomicUsize::new(0),
            }
        }

        pub fn empty() -> Self {
            Self {
                calendar: vec![],
                comms: vec![],
                write_linear_call_count: AtomicUsize::new(0),
            }
        }

        pub fn linear_writes(&self) -> usize {
            self.write_linear_call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl ButlerMcpPort for FakeButlerMcpPort {
        async fn calendar_events(&self) -> Result<Vec<CalendarEvent>, ButlerMcpError> {
            Ok(self.calendar.clone())
        }
        async fn comms_messages(&self) -> Result<Vec<CommsMessage>, ButlerMcpError> {
            Ok(self.comms.clone())
        }
        async fn write_linear_note(
            &self,
            _title: &str,
            _content: &str,
        ) -> Result<(), ButlerMcpError> {
            self.write_linear_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn fetch_figma_summary(&self) -> Result<serde_json::Value, ButlerMcpError> {
            Ok(serde_json::json!({"status": "mock"}))
        }
    }
}

#[cfg(test)]
mod mcp_port_tests {
    use super::test_support::FakeButlerMcpPort;
    use super::*;

    #[test]
    fn on_idle_fetches_real_calendar_via_mcp_port() {
        // AC3 test 1: TestButlerMcpPort feeds a conflict scenario;
        // assert last_assessment().conflicts.len() == 1
        let fake = Arc::new(FakeButlerMcpPort::with_conflict());
        let butler = Butler::new().with_mcp_port(fake);

        // Drive on_idle through the spirit hook (simulated).
        let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
            maos_spirit_abi::ctx::CapabilityHandle(0),
            maos_spirit_abi::ctx::MailboxHandle(0),
        );
        butler.on_idle(&mut ctx);

        let assessment = butler
            .last_assessment()
            .expect("assessment should be stored");
        assert_eq!(assessment.conflicts.len(), 1, "one conflict from MCP data");
        assert!(
            assessment.belief_variance > 0.7,
            "conflict crosses halt floor"
        );
    }

    #[test]
    fn handle_option_a_calls_linear_write() {
        // AC3 test 2: picking option 1 (`linear_note: …`) writes the note.
        let fake = Arc::new(FakeButlerMcpPort::with_conflict());
        let butler = Butler::new().with_mcp_port(fake.clone());

        let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
            maos_spirit_abi::ctx::CapabilityHandle(0),
            maos_spirit_abi::ctx::MailboxHandle(0),
        );
        butler.on_idle(&mut ctx);

        // A notification should be pending after on_idle detected a conflict.
        assert!(butler.last_notification().is_some(), "notification pending");

        let outcome = butler
            .handle_option_pick(1, "deep", Some(fake.as_ref()))
            .expect("option a should succeed");
        assert!(outcome.linear_note_written, "linear note written flag set");
        assert_eq!(
            fake.linear_writes(),
            1,
            "mcp_port.write_linear_note must be called"
        );
        assert!(
            outcome.message.contains("Linear note written"),
            "message confirms write"
        );
    }

    #[test]
    fn mcp_port_none_falls_back_to_pending_scenario() {
        // AC3 test 3: mcp_port = None, pending = Some(scenario); assert on_idle
        // uses the pending scenario (no panic, backwards-compatible).
        let scenario = ScenarioInput {
            calendar: vec![
                CalendarEvent {
                    id: "a".into(),
                    title: "A".into(),
                    start_min: 540,
                    end_min: 600,
                    status: EventStatus::Confirmed,
                },
                CalendarEvent {
                    id: "b".into(),
                    title: "B".into(),
                    start_min: 570,
                    end_min: 630,
                    status: EventStatus::Confirmed,
                },
            ],
            comms: vec![],
            preference_alignment: None,
        };
        let butler = Butler::with_scenario(scenario);

        let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
            maos_spirit_abi::ctx::CapabilityHandle(0),
            maos_spirit_abi::ctx::MailboxHandle(0),
        );
        butler.on_idle(&mut ctx);

        let assessment = butler
            .last_assessment()
            .expect("assessment from pending scenario");
        assert_eq!(assessment.conflicts.len(), 1);
    }

    #[test]
    fn handle_option_pick_no_pending_returns_error() {
        let butler = Butler::new();
        let fake = FakeButlerMcpPort::empty();
        let result = butler.handle_option_pick(1, "deep", Some(&fake));
        assert!(result.is_err(), "no pending notification → error");
    }

    #[test]
    fn halt_screen_line_contract() {
        // FORK 3 companion unit test (Murat constraint).
        assert_eq!(
            halt_screen_line(SCALAR_TAG_BELIEF_VARIANCE),
            "halted on belief_variance"
        );
    }

    #[test]
    fn option_b_stub_renders_message() {
        let fake = Arc::new(FakeButlerMcpPort::with_conflict());
        let butler = Butler::new().with_mcp_port(fake.clone());

        let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
            maos_spirit_abi::ctx::CapabilityHandle(0),
            maos_spirit_abi::ctx::MailboxHandle(0),
        );
        butler.on_idle(&mut ctx);

        let outcome = butler
            .handle_option_pick(2, "deep", Some(fake.as_ref()))
            .expect("option b should succeed");
        assert!(outcome.reminder_set);
        assert!(outcome.message.contains("Slack message queued"));
    }

    #[test]
    fn option_c_stub_renders_snooze() {
        let fake = Arc::new(FakeButlerMcpPort::with_conflict());
        let butler = Butler::new().with_mcp_port(fake.clone());

        let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
            maos_spirit_abi::ctx::CapabilityHandle(0),
            maos_spirit_abi::ctx::MailboxHandle(0),
        );
        butler.on_idle(&mut ctx);

        let outcome = butler
            .handle_option_pick(3, "deep", Some(fake.as_ref()))
            .expect("option c should succeed");
        assert!(outcome.snoozed);
        assert!(outcome.message.contains("snoozed"));
    }
}
