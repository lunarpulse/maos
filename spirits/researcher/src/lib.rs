#![forbid(unsafe_code)]

//! `researcher` — MAOS Researcher v0.5, the second cognitive reference Spirit
//! and the **primary reference implementation of the §9.5 distillation pattern**
//! (Story 8.2).
//!
//! Researcher is the first Spirit on the **participant-scoped** read path. It:
//!
//! 1. **Walks the Transparency Log** via the Story 4.4 [`LogRecallPort`] —
//!    cursor-paginated [`recall`](LogRecallPort::recall) + on-demand
//!    [`fetch`](LogRecallPort::fetch). The walk is kernel-scoped to the calling
//!    Spirit's emitter frames; a cross-Spirit `fetch` yields
//!    [`LogRecallError::ScopeViolation`]. This is the explicit 8.1→8.2 contract:
//!    Butler used the UNscoped `ranged_recall`; Researcher uses the SCOPED port.
//! 2. **Surveys** the recalled frames Spirit-side (survey-mode cognition,
//!    architecture §6.2) into a [`SurveyOutput`] — `findings` / `open_questions`
//!    / `confidence_map` / `bibliography` (the kernel `[output_shape]`), plus two
//!    scalar proxies (`methodology_conflict`, `load_bearing_confidence`) the
//!    kernel compares against the manifest `[epistemic_policy]`.
//! 3. **Persists the digest** through the Story 4.4 [`DistillationPort`] so the
//!    kernel-enforced **I11 audit chain** (`source_log_ref` flattened to raw,
//!    kernel-computed `intent_lineage`, [`DistillationError::AuditChainMissing`])
//!    is never bypassed Spirit-side.
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI and the PURE `maos-domain`
//! ports/types ([`LogRecallPort`], [`DistillationPort`], the i7 scalar.tap
//! types). It NEVER reaches into `maos-kernel-core`/`maos-iac`. The scoped
//! recall, the I11 chain, and the scalar.tap subscription are PROVEN against the
//! real kernel adapters in `tests/`, which carry kernel crates as
//! dev-dependencies only (dev-deps do not enter the kernel-API surface the
//! boundary gate guards — Butler's resolved pattern).
//!
//! ## Compressor determinism
//! The "LLM compression" is a deterministic seeded survey ([`Researcher::survey`])
//! — no live LLM in CI (NFR-Testability-1). The production compressor model
//! class is declared in the manifest (`provider.complete`, ≥Sonnet-tier).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use unicode_segmentation::UnicodeSegmentation;

use maos_domain::distillation::{
    DigestPayload, DistillationError, DistillationReceipt, DistillationRequest,
};
use maos_domain::invariants::i7::ScalarTapEvent;
use maos_domain::log_recall::{LogRecallEntry, LogRecallError, LogRecallFilter};
use maos_domain::ports::{DistillationPort, LogRecallPort};
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

// ───────────────────────────────────────────────────────────────────────────
// Cognitive posture-set (architecture §6.2). The manifest `[posture]` section
// is the autonomy spectrum; THIS is the Researcher cognitive posture-set.
// ───────────────────────────────────────────────────────────────────────────

/// Researcher's cognitive posture. `Survey` ships at v0.5; `Hypothesize` (the
/// ILP+LLM hybrid for novel-hypothesis generation) is **declared** in the
/// posture-set but ships fully at v1.0 (Decision C) — its generative path is
/// intentionally NOT implemented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResearcherPosture {
    /// Exploratory, reactive, divergent. The only posture that ships at v0.5.
    Survey,
    /// Declared per §6.2; the generative ILP+LLM path lands at v1.0 (Decision C).
    Hypothesize,
}

// ───────────────────────────────────────────────────────────────────────────
// Scenario world-state — the claim payloads the survey reasons over. In
// production these arrive from real web/arXiv/GitHub MCP drivers; at v0.5 they
// are served by the fixture-replay MCP provider (Decision B) and ride in the
// Transparency-Log frame payloads the walker fetches.
// ───────────────────────────────────────────────────────────────────────────

/// A single research claim, carried in a recalled frame's payload (JSON).
///
/// Frames whose payload does not parse as a `ClaimPayload` still contribute a
/// `bibliography` entry and a low-confidence finding (so the walker is never a
/// silent no-op on opaque payloads).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimPayload {
    /// Stable claim id (also the `confidence_map` key).
    pub claim_id: String,
    /// The claim statement (hedges included verbatim — preserved into findings).
    pub statement: String,
    /// Topic the claim is about — two claims on the SAME topic with opposite
    /// `polarity` and both strong methodology are a methodology-strength conflict.
    pub topic: String,
    /// Methodology strength in `[0.0, 1.0]`.
    pub methodology_strength: f32,
    /// Researcher's confidence in the claim in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Whether the claim is load-bearing for a downstream decision.
    #[serde(default)]
    pub load_bearing: bool,
    /// Direction of the claim on its topic (supports / refutes).
    #[serde(default)]
    pub polarity: bool,
    /// Hedge words/phrases that MUST survive compression (hedge-preservation).
    #[serde(default)]
    pub hedges: Vec<String>,
}

// ───────────────────────────────────────────────────────────────────────────
// Survey output — the §6.2 `[output_shape]`: findings / open_questions /
// confidence_map / bibliography. Serializes with EXACTLY those top-level keys
// so it satisfies the kernel `required_fields` predicate.
// ───────────────────────────────────────────────────────────────────────────

/// One survey finding, citing the raw frame it was distilled from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub claim_id: String,
    pub statement: String,
    pub confidence: f32,
    /// Hedges preserved from the source claim (hedge-preservation metric).
    pub hedges: Vec<String>,
    /// 32-char hex of the 16-byte source frame id (the cite / traceability).
    pub source_log_ref: String,
}

/// A bibliography entry — a recalled source frame and its intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BibEntry {
    pub source_log_ref: String,
    pub intent: String,
}

/// The composed survey output (architecture §6.2 Researcher output shape).
///
/// Serializes to the four `required_fields` top-level keys. `scalars` is an
/// auxiliary block (the epistemic-policy proxies + any observed `scalar.tap`
/// pattern) and is NOT one of the required fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveyOutput {
    /// (a) findings — claims surveyed, each citing its raw source frame.
    pub findings: Vec<Finding>,
    /// (b) open questions — contradictions + low-confidence claims.
    pub open_questions: Vec<String>,
    /// (c) confidence map — claim_id → confidence (plus any observed scalars).
    pub confidence_map: BTreeMap<String, f32>,
    /// (d) bibliography — the recalled source frames.
    pub bibliography: Vec<BibEntry>,
    /// Auxiliary epistemic scalars (NOT a required output field).
    #[serde(default)]
    pub scalars: BTreeMap<String, f64>,
}

impl SurveyOutput {
    /// The single most-salient scalar the kernel compares against the manifest
    /// `[epistemic_policy]`. Mirrors Butler's `primary_scalar` discipline: the
    /// Spirit reports `(tag, value, derived_from)`; the kernel owns the halt.
    ///
    /// A methodology-strength conflict dominates; otherwise the weakest
    /// load-bearing confidence is reported.
    pub fn primary_scalar(&self) -> (&'static str, f64, String) {
        let conflict = self.scalars.get("methodology_conflict").copied().unwrap_or(0.0);
        if conflict > 0.0 {
            return (
                "methodology_conflict",
                conflict,
                "methodology-strength-conflict".to_string(),
            );
        }
        let load_bearing = self
            .scalars
            .get("load_bearing_confidence")
            .copied()
            .unwrap_or(1.0);
        (
            "load_bearing_confidence",
            load_bearing,
            "weakest-load-bearing-claim".to_string(),
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Recalled frame — a frame the walker pulled (id + intent + fetched payload).
// ───────────────────────────────────────────────────────────────────────────

/// A frame recalled + fetched by the participant-scoped walker.
#[derive(Debug, Clone, PartialEq)]
pub struct RecalledFrame {
    pub frame_id: [u8; 16],
    pub intent: String,
    pub payload: Vec<u8>,
}

// ───────────────────────────────────────────────────────────────────────────
// Errors.
// ───────────────────────────────────────────────────────────────────────────

/// Errors Researcher's recall/survey/distillation path can surface.
#[derive(Debug)]
pub enum ResearcherError {
    /// A participant-scoped log.recall / fetch failed.
    Recall(LogRecallError),
    /// Building or writing the distillation request violated I11.
    Distillation(DistillationError),
    /// A finding cited a malformed `source_log_ref` hex.
    BadFrameIdHex(String),
    /// Internal consistency check failed.
    Internal(String),
}

impl std::fmt::Display for ResearcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearcherError::Recall(e) => write!(f, "log.recall error: {e}"),
            ResearcherError::Distillation(e) => write!(f, "distillation error: {e}"),
            ResearcherError::BadFrameIdHex(s) => write!(f, "malformed source_log_ref hex: {s}"),
            ResearcherError::Internal(s) => write!(f, "internal consistency error: {s}"),
        }
    }
}
impl std::error::Error for ResearcherError {}

// ───────────────────────────────────────────────────────────────────────────
// The Researcher Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Researcher reference Spirit. Optionally holds the frames its `on_idle`
/// survey reasons over (in production the walker seeds these from `log.recall`).
#[derive(Debug, Clone)]
pub struct Researcher {
    /// The cognitive posture-set (§6.2). `Survey` is active at v0.5; `Hypothesize`
    /// is declared but its generative path is not implemented (Decision C).
    posture_set: Vec<ResearcherPosture>,
    /// Frames to survey on the next `on_idle` (seeded by the walker / fixture).
    pending: Option<Vec<RecalledFrame>>,
    /// The most recent survey output. `Arc<Mutex<...>>` so Researcher stays
    /// `Sync` as required by the `#[spirit]` macro.
    last_output: Arc<Mutex<Option<SurveyOutput>>>,
}

#[spirit]
impl Researcher {
    /// Survey idle pass. Cancellation-aware; bounded (a single linear pass over
    /// the pending frames, well within `time_cap_seconds`).
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        if let Some(frames) = &self.pending {
            // Survey the recalled frames and store the output so the hook has a
            // production-visible effect. The scalar.tap write / epistemic-policy
            // halt path is exercised against the real kernel adapters in tests
            // (the ABI `Ctx` exposes no scalar-write surface yet — Butler 8.1
            // navigated the same gap).
            let output = self.survey(frames);
            let mut guard = self.last_output.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(output);
        }
    }
}

impl Default for Researcher {
    fn default() -> Self {
        Self {
            posture_set: vec![ResearcherPosture::Survey, ResearcherPosture::Hypothesize],
            pending: None,
            last_output: Arc::new(Mutex::new(None)),
        }
    }
}

impl Researcher {
    /// A Researcher with no pending frames (production default).
    pub fn new() -> Self {
        Self::default()
    }

    /// A Researcher whose `on_idle` will survey `frames` (e.g. fixture-seeded).
    pub fn with_frames(frames: Vec<RecalledFrame>) -> Self {
        Self {
            pending: Some(frames),
            ..Self::default()
        }
    }

    /// The declared cognitive posture-set (§6.2).
    pub fn posture_set(&self) -> &[ResearcherPosture] {
        &self.posture_set
    }

    /// The active posture at v0.5 — always `Survey` (Decision C).
    pub fn active_posture(&self) -> ResearcherPosture {
        ResearcherPosture::Survey
    }

    /// The output from the most recent `on_idle` survey, if any.
    pub fn last_output(&self) -> Option<SurveyOutput> {
        self.last_output.lock().unwrap().clone()
    }

    // ── the participant-scoped log.recall walker (Story 4.4) ─────────────────

    /// Walk the Transparency Log via the participant-scoped [`LogRecallPort`],
    /// returning every frame visible to `spirit_pid` (cursor-paginated) with its
    /// payload lazily [`fetch`](LogRecallPort::fetch)ed.
    ///
    /// Results are kernel-scoped to the calling Spirit's emitter frames; a
    /// cross-Spirit `fetch` returns [`LogRecallError::ScopeViolation`] (proven
    /// against the real `LogRecallAdapter` in `tests/`). The walker NEVER uses
    /// the unscoped `ranged_recall` — the explicit 8.1→8.2 contract.
    pub fn walk(
        &self,
        port: &dyn LogRecallPort,
        spirit_pid: u32,
        base: LogRecallFilter,
    ) -> Result<Vec<RecalledFrame>, ResearcherError> {
        let entries = recall_all(port, spirit_pid, base).map_err(ResearcherError::Recall)?;
        fetch_payloads(port, spirit_pid, &entries).map_err(ResearcherError::Recall)
    }

    // ── survey cognition (the deterministic seeded compressor) ───────────────

    /// Survey the recalled frames into a [`SurveyOutput`]. Deterministic and
    /// pure (no live LLM): the production compressor model class is declared in
    /// the manifest; here the compression is a seeded survey.
    ///
    /// - **findings**: one per claim frame; hedges preserved verbatim.
    /// - **methodology_conflict** scalar: two claims on the same `topic`, both
    ///   `methodology_strength ≥ 0.7`, opposite `polarity` → `min(ms_a, ms_b)`.
    /// - **load_bearing_confidence** scalar: the weakest load-bearing confidence.
    /// - **open_questions**: contradictions + claims below the 0.7 floor.
    pub fn survey(&self, frames: &[RecalledFrame]) -> SurveyOutput {
        let mut findings = Vec::new();
        let mut bibliography = Vec::new();
        let mut confidence_map: BTreeMap<String, f32> = BTreeMap::new();
        let mut open_questions = Vec::new();
        // (topic, polarity, methodology_strength) per claim for conflict scan.
        let mut topic_claims: Vec<(String, bool, f32)> = Vec::new();
        // Confidences of LOAD-BEARING claims only (the second epistemic rule).
        let mut load_bearing_confidences: Vec<f32> = Vec::new();

        // Cooperative cancellation: yield every N frames so a long survey
        // cannot monopolize the thread (the runtime's cancellation signal is
        // checked at hook boundaries; intra-hook we yield cooperatively).
        const YIELD_EVERY_N_FRAMES: usize = 256;
        for (idx, frame) in frames.iter().enumerate() {
            if idx > 0 && idx % YIELD_EVERY_N_FRAMES == 0 {
                std::thread::yield_now();
            }
            let hex = encode_frame_id_hex(&frame.frame_id);
            bibliography.push(BibEntry {
                source_log_ref: hex.clone(),
                intent: frame.intent.clone(),
            });
            match serde_json::from_slice::<ClaimPayload>(&frame.payload) {
                Ok(claim) => {
                    let confidence = sanitize_unit(claim.confidence);
                    if confidence_map.insert(claim.claim_id.clone(), confidence).is_some() {
                        open_questions.push(format!(
                            "duplicate claim_id '{}' — later frame overwrites earlier",
                            claim.claim_id
                        ));
                    }
                    if claim.load_bearing {
                        load_bearing_confidences.push(confidence);
                        if confidence < LOAD_BEARING_CONFIDENCE_FLOOR {
                            open_questions.push(format!(
                                "load-bearing claim '{}' is below the {LOAD_BEARING_CONFIDENCE_FLOOR} confidence floor",
                                claim.claim_id
                            ));
                        }
                    }
                    topic_claims.push((
                        claim.topic.clone(),
                        claim.polarity,
                        sanitize_unit(claim.methodology_strength),
                    ));
                    findings.push(Finding {
                        claim_id: claim.claim_id,
                        // Compression: the digest carries a BOUNDED summary of the
                        // source statement, not the verbatim source — a digest is
                        // smaller than its inputs (§9.5). Hedges are preserved
                        // separately below, so summarizing never drops a hedge.
                        statement: summarize(&claim.statement),
                        confidence,
                        hedges: claim.hedges,
                        source_log_ref: hex,
                    });
                }
                Err(_) => {
                    // Opaque payload — record a low-confidence finding so the
                    // frame is never silently dropped from the survey.
                    let claim_id = format!("opaque::{hex}");
                    confidence_map.insert(claim_id.clone(), 0.0);
                    open_questions.push(format!("opaque source frame {hex} could not be surveyed"));
                    findings.push(Finding {
                        claim_id,
                        statement: "<opaque payload — not a structured claim>".to_string(),
                        confidence: 0.0,
                        hedges: Vec::new(),
                        source_log_ref: hex,
                    });
                }
            }
        }

        // methodology_conflict: strongest conflicting pair sharing a topic.
        // Cap input cardinality to prevent O(n²) stalls on unexpectedly large
        // recall results (defense-in-depth against misconfigured filters).
        const MAX_CONFLICT_CLAIMS: usize = 10_000;
        let capped_claims = if topic_claims.len() > MAX_CONFLICT_CLAIMS {
            open_questions.push(format!(
                "methodology_conflict scan capped at {MAX_CONFLICT_CLAIMS} claims ({} received)",
                topic_claims.len()
            ));
            &topic_claims[..MAX_CONFLICT_CLAIMS]
        } else {
            &topic_claims[..]
        };
        let mut conflict: f32 = 0.0;
        for i in 0..capped_claims.len() {
            if i > 0 && i % 1024 == 0 {
                std::thread::yield_now();
            }
            for j in (i + 1)..capped_claims.len() {
                let (ref ta, pa, ma) = capped_claims[i];
                let (ref tb, pb, mb) = capped_claims[j];
                if ta == tb
                    && pa != pb
                    && ma >= METHODOLOGY_STRONG_FLOOR
                    && mb >= METHODOLOGY_STRONG_FLOOR
                {
                    let pair = ma.min(mb);
                    if pair > conflict {
                        conflict = pair;
                        open_questions.push(format!(
                            "methodology-strength conflict on topic '{ta}' (both ≥ {METHODOLOGY_STRONG_FLOOR})"
                        ));
                    }
                }
            }
        }

        // load_bearing_confidence: weakest confidence among LOAD-BEARING claims;
        // 1.0 if there are none (a survey with no load-bearing claim never fires
        // the below-0.7 rule on an unrelated low-confidence exploratory claim).
        let load_bearing_confidence = load_bearing_confidences
            .into_iter()
            .fold(None, |acc: Option<f32>, c| Some(acc.map_or(c, |a| a.min(c))))
            .unwrap_or(1.0);

        let mut scalars: BTreeMap<String, f64> = BTreeMap::new();
        scalars.insert("methodology_conflict".to_string(), conflict as f64);
        scalars.insert(
            "load_bearing_confidence".to_string(),
            load_bearing_confidence as f64,
        );

        // Deduplicate bibliography by source_log_ref (frame_id) — the same frame
        // may appear twice if pagination or filtering overlaps.
        let mut seen_refs: std::collections::HashSet<String> = std::collections::HashSet::new();
        bibliography.retain(|b| seen_refs.insert(b.source_log_ref.clone()));

        SurveyOutput {
            findings,
            open_questions,
            confidence_map,
            bibliography,
            scalars,
        }
    }

    /// Incorporate an observed `scalar.tap` event into a survey output (AC6):
    /// the received scalar pattern is folded into the `confidence_map` and an
    /// open question, so a SUBSEQUENT distillate carries the observation.
    pub fn incorporate_scalar(&self, output: &mut SurveyOutput, event: &ScalarTapEvent) {
        let key = format!("observed::{}::{}", event.spirit_id, event.tag);
        let value = if event.value.is_nan() {
            output.open_questions.push(format!(
                "observed scalar.tap '{}' from '{}' is NaN — treated as 0.0",
                event.tag, event.spirit_id
            ));
            0.0
        } else {
            event.value
        };
        output
            .confidence_map
            .insert(key.clone(), sanitize_unit(value as f32));
        output.scalars.insert(key, value);
        output.open_questions.push(format!(
            "observed scalar.tap '{}' from '{}' = {:.3}",
            event.tag, event.spirit_id, value
        ));
    }

    // ── I11 distillation ─────────────────────────────────────────────────────

    /// Build the I11 distillation request for a survey output. `source_log_ref`
    /// is the deduped set of cited finding frame ids; `distillation_depth` is
    /// `depth.max(1)`. The kernel re-validates the chain at write time.
    pub fn to_distillation_request(
        &self,
        output: &SurveyOutput,
        depth: u32,
    ) -> Result<DistillationRequest, ResearcherError> {
        let mut refs: Vec<[u8; 16]> = Vec::new();
        let mut seen: std::collections::HashSet<[u8; 16]> = std::collections::HashSet::new();
        for finding in &output.findings {
            let id = decode_frame_id_hex(&finding.source_log_ref)?;
            if seen.insert(id) {
                refs.push(id);
            }
        }
        let payload = DigestPayload::Json(
            serde_json::to_value(output)
                .map_err(|e| ResearcherError::Internal(format!("survey serialize: {e}")))?,
        );
        DistillationRequest::new(refs, depth.max(1), payload, None)
            .map_err(ResearcherError::Distillation)
    }

    /// Survey + persist in one step against a real [`DistillationPort`]: walk's
    /// output → request → `write_distillate`. Returns the kernel
    /// [`DistillationReceipt`] (transitively-flattened `effective_source_log_ref`
    /// + kernel-computed `intent_lineage`).
    pub fn distill_through(
        &self,
        port: &dyn DistillationPort,
        spirit_pid: u32,
        output: &SurveyOutput,
        depth: u32,
    ) -> Result<DistillationReceipt, ResearcherError> {
        let request = self.to_distillation_request(output, depth)?;
        port.write_distillate(spirit_pid, request)
            .map_err(ResearcherError::Distillation)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Free-function walker primitives (pure over the domain port).
// ───────────────────────────────────────────────────────────────────────────

/// Safety cap on recall pages so a misbehaving cursor cannot loop forever.
/// At MAX_LIMIT=1024 entries/page this is ~1M frames — far beyond any survey.
pub const MAX_RECALL_PAGES: usize = 1024;

/// Confidence floor below which a load-bearing claim becomes an open question
/// and the `load_bearing_confidence` epistemic rule fires (§6.2).
pub const LOAD_BEARING_CONFIDENCE_FLOOR: f32 = 0.7;

/// Methodology-strength floor above which two opposing same-topic claims are a
/// methodology-strength conflict (§6.2).
pub const METHODOLOGY_STRONG_FLOOR: f32 = 0.7;

/// Recall EVERY entry visible to `spirit_pid`, following `next_cursor` across
/// pages. Reuses the base filter's kind/time/limit/intent on each page so the
/// participant scope + clamp are preserved (`LogRecallFilter::new`).
pub fn recall_all(
    port: &dyn LogRecallPort,
    spirit_pid: u32,
    base: LogRecallFilter,
) -> Result<Vec<LogRecallEntry>, LogRecallError> {
    let mut out = Vec::new();
    let mut cursor = base.cursor.clone();
    for _ in 0..MAX_RECALL_PAGES {
        let filter = LogRecallFilter::new(
            base.kind.clone(),
            base.since_ns,
            base.until_ns,
            base.limit,
            cursor.clone(),
            base.intent_filter.clone(),
        );
        let page = port.recall(spirit_pid, filter)?;
        out.extend(page.entries);
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return Ok(out),
        }
    }
    Err(LogRecallError::InvalidCursor(
        "exceeded MAX_RECALL_PAGES — pagination did not terminate".to_string(),
    ))
}

/// Fetch the payload for each entry that advertises one, building
/// [`RecalledFrame`]s. Entries without a payload still contribute (empty bytes)
/// so the bibliography stays complete.
pub fn fetch_payloads(
    port: &dyn LogRecallPort,
    spirit_pid: u32,
    entries: &[LogRecallEntry],
) -> Result<Vec<RecalledFrame>, LogRecallError> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let payload = if entry.payload_available {
            port.fetch(spirit_pid, entry.frame_id)?.payload_redacted
        } else {
            Vec::new()
        };
        out.push(RecalledFrame {
            frame_id: entry.frame_id,
            intent: entry.intent.clone(),
            payload,
        });
    }
    Ok(out)
}

// ───────────────────────────────────────────────────────────────────────────
// Pure helpers.
// ───────────────────────────────────────────────────────────────────────────

/// Max chars of a source statement carried into a digest finding. The
/// distillation pattern compresses — the digest is a bounded summary, not the
/// verbatim source (§9.5 / App F.4 adaptive-chunk-ratio: compress the middle,
/// keep the digest small).
pub const MAX_FINDING_SUMMARY_CHARS: usize = 280;

/// Compress a source statement to a bounded summary (first
/// [`MAX_FINDING_SUMMARY_CHARS`] chars, on a char boundary, with an ellipsis if
/// truncated). Deterministic; no live LLM.
fn summarize(statement: &str) -> String {
    let graphemes: Vec<&str> = statement.graphemes(true).collect();
    if graphemes.len() <= MAX_FINDING_SUMMARY_CHARS {
        return statement.to_string();
    }
    let head: String = graphemes.into_iter().take(MAX_FINDING_SUMMARY_CHARS).collect();
    format!("{head}…")
}

/// Clamp a value to `[0.0, 1.0]`, mapping NaN to 0.0 so a malformed scalar can
/// never silently bypass an epistemic-policy threshold.
fn sanitize_unit(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// Encode a 16-byte frame id as colon-separated hex pairs-of-bytes, e.g.
/// `00ff:1122:…` (8 groups of 4 hex chars).
///
/// **Why colon-separated, not 32 continuous hex chars:** the kernel
/// pre-write redaction filter treats any run of ≥32 consecutive hex chars as an
/// Ed25519 capability-token secret and redacts it. A digest payload that cites
/// raw 32-char hex frame ids would therefore "leak" (redaction fires) and fail
/// the secret-leakage metric. Inserting a `:` every 4 hex chars caps the longest
/// hex run at 4 so the cite survives the filter cleanly — exactly the convention
/// `DistillateWriter::format_frame_id_hex` uses for the same reason.
pub fn encode_frame_id_hex(id: &[u8; 16]) -> String {
    id.chunks(2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(":")
}

/// Decode a frame id from either the colon-separated form produced by
/// [`encode_frame_id_hex`] or a bare 32-char hex string.
pub fn decode_frame_id_hex(hex: &str) -> Result<[u8; 16], ResearcherError> {
    let clean: String = hex.chars().filter(|c| *c != ':').collect();
    if clean.len() != 32 || !clean.is_ascii() {
        return Err(ResearcherError::BadFrameIdHex(hex.to_string()));
    }
    // encode_frame_id_hex emits lowercase; reject uppercase to avoid subtle mismatches.
    if clean.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(ResearcherError::BadFrameIdHex(hex.to_string()));
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        let s = &clean[i * 2..i * 2 + 2];
        *byte =
            u8::from_str_radix(s, 16).map_err(|_| ResearcherError::BadFrameIdHex(hex.to_string()))?;
    }
    Ok(out)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn claim_frame(id_byte: u8, claim: &ClaimPayload) -> RecalledFrame {
        RecalledFrame {
            frame_id: [id_byte; 16],
            intent: "inform".into(),
            payload: serde_json::to_vec(claim).unwrap(),
        }
    }

    fn claim(id: &str, topic: &str, ms: f32, conf: f32, polarity: bool, lb: bool) -> ClaimPayload {
        ClaimPayload {
            claim_id: id.into(),
            statement: format!("{id}: the effect is likely present"),
            topic: topic.into(),
            methodology_strength: ms,
            confidence: conf,
            load_bearing: lb,
            polarity,
            hedges: vec!["likely".into(), "uncertain".into()],
        }
    }

    #[test]
    fn survey_preserves_hedges_and_cites_frames() {
        let f = claim_frame(0x11, &claim("c1", "t", 0.9, 0.95, true, false));
        let out = Researcher::new().survey(&[f]);
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.findings[0].source_log_ref, encode_frame_id_hex(&[0x11; 16]));
        assert!(out.findings[0].hedges.contains(&"likely".to_string()));
        assert_eq!(out.bibliography.len(), 1);
        assert!(out.confidence_map.contains_key("c1"));
    }

    #[test]
    fn opposing_strong_methodology_is_a_conflict() {
        let a = claim_frame(0x01, &claim("a", "fusion", 0.85, 0.9, true, false));
        let b = claim_frame(0x02, &claim("b", "fusion", 0.8, 0.9, false, false));
        let out = Researcher::new().survey(&[a, b]);
        let (tag, value, _) = out.primary_scalar();
        assert_eq!(tag, "methodology_conflict");
        assert!(value >= 0.7, "conflict {value} must cross the 0.7 halt floor");
    }

    #[test]
    fn weak_load_bearing_confidence_drives_the_second_rule() {
        let a = claim_frame(0x03, &claim("lb", "x", 0.4, 0.55, true, true));
        let out = Researcher::new().survey(&[a]);
        let (tag, value, _) = out.primary_scalar();
        assert_eq!(tag, "load_bearing_confidence");
        assert!(value < 0.7, "weak load-bearing confidence crosses the below-0.7 floor");
        assert!(out.open_questions.iter().any(|q| q.contains("confidence floor")));
    }

    #[test]
    fn output_serializes_with_required_fields() {
        let f = claim_frame(0x21, &claim("c", "t", 0.9, 0.9, true, false));
        let out = Researcher::new().survey(&[f]);
        let v = serde_json::to_value(&out).unwrap();
        for field in ["findings", "open_questions", "confidence_map", "bibliography"] {
            assert!(v.get(field).is_some(), "missing required output field {field}");
        }
    }

    #[test]
    fn distillation_request_dedups_and_requires_nonempty() {
        let f = claim_frame(0x31, &claim("c", "t", 0.9, 0.9, true, false));
        let out = Researcher::new().survey(&[f]);
        let req = Researcher::new().to_distillation_request(&out, 1).unwrap();
        assert_eq!(req.source_log_ref.len(), 1);
        assert!(req.distillation_depth >= 1);

        // An empty survey has no cited frames → AuditChainMissing at the
        // request layer (I11 author-guard).
        let empty = SurveyOutput {
            findings: vec![],
            open_questions: vec![],
            confidence_map: BTreeMap::new(),
            bibliography: vec![],
            scalars: BTreeMap::new(),
        };
        let err = Researcher::new().to_distillation_request(&empty, 1).unwrap_err();
        assert!(matches!(
            err,
            ResearcherError::Distillation(DistillationError::AuditChainMissing { .. })
        ));
    }

    #[test]
    fn incorporate_scalar_folds_observation_in() {
        let f = claim_frame(0x41, &claim("c", "t", 0.9, 0.9, true, false));
        let mut out = Researcher::new().survey(&[f]);
        let before = out.open_questions.len();
        Researcher::new().incorporate_scalar(
            &mut out,
            &ScalarTapEvent {
                spirit_id: "observer".into(),
                tag: "drift".into(),
                value: 0.42,
                timestamp: 1,
            },
        );
        assert_eq!(out.open_questions.len(), before + 1);
        assert!(out.confidence_map.keys().any(|k| k.starts_with("observed::")));
    }

    #[test]
    fn hex_roundtrip() {
        let id = [0xCD; 16];
        assert_eq!(decode_frame_id_hex(&encode_frame_id_hex(&id)).unwrap(), id);
        assert!(decode_frame_id_hex("zz").is_err());
    }

    #[test]
    fn posture_set_declares_hypothesize_but_active_is_survey() {
        let r = Researcher::new();
        assert!(r.posture_set().contains(&ResearcherPosture::Hypothesize));
        assert_eq!(r.active_posture(), ResearcherPosture::Survey);
    }
}
