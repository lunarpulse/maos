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
use maos_domain::notification::NotificationEvent;
use maos_domain::ports::EpistemicScalarPort;
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// Butler's own Spirit id — the `spirit_id` it writes scalars under and the id
/// every digest cite / halt frame is attributed to.
pub const BUTLER_SPIRIT_ID: &str = "butler";

/// Butler's spirit_pid at v0.3-β. The daemon is single-Spirit (consistent with
/// the rest of the composition root, which uses pid 0); Story 8.11 threads the
/// real per-Spirit pid through the production port.
pub const BUTLER_SPIRIT_PID: u32 = 0;

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
}

impl Assessment {
    /// The single scalar Butler writes for this scenario — the most salient
    /// concern. The kernel decides whether it crosses the `[epistemic_policy]`
    /// threshold; Butler does NOT hardcode the thresholds.
    ///
    /// Returns `(tag, value, derived_from)`.
    pub fn primary_scalar(&self) -> (&'static str, f64, String) {
        if !self.conflicts.is_empty() {
            let refs = self
                .conflicts
                .iter()
                .map(|c| format!("{}~{}", c.a, c.b))
                .collect::<Vec<_>>()
                .join(",");
            (
                "belief_variance",
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
                "belief_variance",
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

/// Errors Butler's digest path can surface.
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
}

impl std::fmt::Display for ButlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButlerError::Audit(e) => write!(f, "audit read error: {e}"),
            ButlerError::BadFrameIdHex(s) => write!(f, "malformed source_log_ref hex: {s}"),
            ButlerError::Distillation(e) => write!(f, "distillation error: {e}"),
            ButlerError::Internal(s) => write!(f, "internal consistency error: {s}"),
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
    /// Thread-safe (`Arc<Mutex<...>>`) so Butler remains `Sync` as required
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
}

impl std::fmt::Debug for Butler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Butler")
            .field("pending", &self.pending)
            .field("has_scalar_port", &self.scalar_port.is_some())
            .finish()
    }
}

#[spirit]
impl Butler {
    /// Anticipatory idle pass. Cancellation-aware; bounded (it does a single
    /// linear pass over the pending scenario, well within `time_cap_seconds`).
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        if let Some(scenario) = &self.pending {
            // Run the anticipatory reasoning and store the result so it is
            // observable (not silently discarded).
            let assessment = self.assess(scenario);

            // Story 8.10 AC1 — when the epistemic-scalar port is wired, drive
            // the assessed scalar through the kernel policy path so the
            // `[epistemic_policy]` halt fires. The halt DECISION is the kernel's
            // (universal-arithmetic over Butler's manifest policy); Butler only
            // supplies its own assessed scalar. The `Option<HaltReceipt>` is
            // stored so the firing is observable (proving `assessment → on_idle
            // → port → halt`, the link Story 8.1 never exercised). With `port =
            // None` this is store-only (the original v0.3-β behavior).
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
                    Err(_) => *guard = None, // clear stale receipt on backend failure
                }
            }

            let mut guard = self.last_assessment.lock().unwrap();
            *guard = Some(assessment);
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
        }
    }
}

impl Butler {
    /// A Butler with no pending scenario (digest-only / production default).
    pub fn new() -> Self {
        Self::default()
    }

    /// A Butler whose `on_idle` will reason over `scenario`.
    pub fn with_scenario(scenario: ScenarioInput) -> Self {
        Self {
            pending: Some(scenario),
            last_assessment: Arc::new(std::sync::Mutex::new(None)),
            scalar_port: None,
            last_halt_receipt: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Story 8.10 AC1 — inject the epistemic-scalar **write** port (test/daemon
    /// builder). When set, `on_idle` drives the assessed scalar through the
    /// kernel policy path so the `[epistemic_policy]` halt can fire.
    pub fn with_scalar_port(mut self, port: Arc<dyn EpistemicScalarPort>) -> Self {
        self.scalar_port = Some(port);
        self
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
        let belief_variance = if conflicts.is_empty() {
            0.2
        } else {
            (0.75 + 0.05 * (conflicts.len().min(5) as f32)).min(0.99)
        };

        // user_preference_drift: the comms-triage alignment signal. Fully
        // aligned (1.0) absent any signal. Reject NaN / out-of-range values
        // so they don't silently bypass the epistemic-policy halt rule.
        let user_preference_drift = scenario
            .preference_alignment
            .and_then(|v| if v.is_nan() { None } else { Some(v.clamp(0.0, 1.0)) })
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
            if e.frame_id_hex.len() != 32
                || !e.frame_id_hex.chars().all(|c| c.is_ascii_hexdigit())
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

        let anomalies = anomalies
            .iter()
            .filter_map(|ev| match ev {
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
                    })
                }
                _ => None,
            })
            .collect();

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
    if words.iter().any(|w| matches!(*w, "fail" | "error" | "failed" | "errored")) {
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
        *byte = u8::from_str_radix(s, 16).map_err(|_| ButlerError::BadFrameIdHex(hex.to_string()))?;
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
        assert!(a.belief_variance > 0.7, "one conflict crosses the halt floor");
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
