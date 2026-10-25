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
//! ## Spirit-side boundary
//! This crate depends only on the Spirit SDK/ABI and the PURE `maos-domain`
//! ports/types. Kernel adapters remain at the composition root; the only
//! kernel source delta for Story 13.5d is the bounded caller/token pid guard.
//! Integration proofs use kernel crates as dev-dependencies, which do not enter
//! the Spirit's production dependency surface.
//!
//! ## Compressor determinism
//! The "LLM compression" is a deterministic seeded survey ([`Researcher::survey`])
//! — no live LLM in CI (NFR-Testability-1). The production compressor model
//! class is declared in the manifest (`provider.complete`, ≥Sonnet-tier).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use unicode_segmentation::UnicodeSegmentation;

use maos_domain::distillation::{
    DigestPayload, DistillationError, DistillationReceipt, DistillationRequest,
};
use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::invariants::i7::ScalarTapEvent;
use maos_domain::log_recall::{
    FrameKindLabel, LogRecallCursor, LogRecallEntry, LogRecallError, LogRecallFilter,
};
use maos_domain::memory::{MemoryEntry, MemoryNamespace, MemoryValue};
use maos_domain::ports::inference::{InferenceError, InferenceOptions, InferenceRequest};
use maos_domain::ports::{DistillationPort, EpistemicScalarPort, InferencePort, LogRecallPort};
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
/// Story 8.14c — parallelism bound for the MCP fan-out.
///
/// FORK 1 RESOLVED (Option A): realized Spirit-side as a const, NOT a manifest
/// TOML key (`deny_unknown_fields` would reject it). The manifest documents the
/// intent in a comment under `[capabilities.required]`. The REAL bound is
/// enforced by `Arc<Semaphore>::new(RESEARCHER_PARALLELISM)` in
/// `LiveResearcherMcpPort` (maos-bin). When a cross-Spirit scheduler lands
/// (Epic 10 / HSIS), promote to a parsed `maos-manifest` field + schema v3 bump.
pub const RESEARCHER_PARALLELISM: usize = 8;

// ───────────────────────────────────────────────────────────────────────────
// Story 8.14c — MCP domain port (FORK 1 + FORK 2).
// ───────────────────────────────────────────────────────────────────────────

/// A claim fetched from an MCP tool call, paired with its citable source key.
///
/// `source_key` is the join key that correlates this claim to the
/// `FrameKind::McpInvocation` frame the kernel adapter journaled for the fetch.
/// It is the exact value stored in the call args (`arxiv_id`, `url`, `repo`,
/// or `paper_id`) — see `drivers::researcher` arg builders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FetchedClaim {
    pub claim: ClaimPayload,
    pub source_key: String,
}

/// MCP error variants specific to Researcher's domain port layer.
#[derive(Debug, thiserror::Error)]
pub enum ResearcherMcpError {
    #[error("MCP call failed on {server}/{tool}: {cause}")]
    CallFailed {
        server: String,
        tool: String,
        cause: String,
    },
    #[error("capability token issuance failed: {0}")]
    TokenIssuanceFailed(String),
    #[error("unauthorized MCP call")]
    Unauthorized,
    #[error("no results")]
    NoResults,
    #[error("decode error: {0}")]
    Decode(String),
    /// Story 18-3 (review ruling D-18-3-R2) — a bound `--live` pass surveys
    /// MCP, never the seed claims, and no MCP server is configured.
    #[error("no MCP server is configured (a --live pass never surveys the seed claims)")]
    Unconfigured,
}

/// Story 8.14c — the MCP domain port Researcher's `on_idle` calls to fan out
/// over web/arXiv/GitHub/citation-graph servers.
///
/// The trait is **sync** — `LiveResearcherMcpPort` (maos-bin) blocks internally
/// via `Handle::current().block_on(fanout)` from the `spawn_blocking` pool
/// (FORK 3 resolution, 2026-06-10). The parallelism-8 `JoinSet`/`Semaphore`
/// lives entirely inside the impl; no async leaks into the Spirit crate.
pub trait ResearcherMcpPort: Send + Sync {
    /// Fan out (≤ [`RESEARCHER_PARALLELISM`] concurrent) over the four declared
    /// servers for `query`, and return the parsed claims with their source keys.
    fn survey_literature(&self, query: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError>;
}

/// Story 13.5d — synchronous, mediated collective-memory boundary owned by
/// Researcher. Kernel and runtime types remain at the composition root.
#[derive(Debug, thiserror::Error)]
pub enum ResearcherCollectiveError {
    #[error("collective port is not wired")]
    Unavailable,
    #[error("collective operation denied or unavailable: {0}")]
    Denied(String),
    #[error("collective operation refused because the audit sink is unavailable")]
    AuditUnavailable,
}

pub trait ResearcherCollectivePort: Send + Sync {
    fn collective_write(
        &self,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), ResearcherCollectiveError>;
    fn collective_read(
        &self,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, ResearcherCollectiveError>;
    fn collective_scan(
        &self,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, ResearcherCollectiveError>;
}

/// Story 8.14c — test double for `ResearcherMcpPort`.
/// Ships in the SAME commit as the trait (FORK 1 constraint).
#[cfg(test)]
pub struct FakeResearcherMcpPort {
    pub claims: Vec<FetchedClaim>,
}

#[cfg(test)]
impl ResearcherMcpPort for FakeResearcherMcpPort {
    fn survey_literature(&self, _query: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        Ok(self.claims.clone())
    }
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
        let conflict = self
            .scalars
            .get("methodology_conflict")
            .copied()
            .unwrap_or(0.0);
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
    /// Story 18-3 (D-18-3-05 A) — the bound Inference Port has no binding yet,
    /// or the pass's token mint was refused.
    Unbound(String),
    /// Story 18-3 — `InferencePort::complete` failed (provider error,
    /// capability denial, cassette exhaustion or strict drift).
    Inference(InferenceError),
    /// Story 18-3 (D-18-3-05 A) — a bound pass's MCP survey failed.
    McpSurvey(ResearcherMcpError),
    /// Story 18-3 (review EC-8) — a bound pass surveyed no claims (no
    /// scalar, no distillate).
    NoClaims,
}

impl std::fmt::Display for ResearcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearcherError::Recall(e) => write!(f, "log.recall error: {e}"),
            ResearcherError::Distillation(e) => write!(f, "distillation error: {e}"),
            ResearcherError::BadFrameIdHex(s) => write!(f, "malformed source_log_ref hex: {s}"),
            ResearcherError::Internal(s) => write!(f, "internal consistency error: {s}"),
            ResearcherError::Unbound(s) => write!(f, "inference unavailable: {s}"),
            ResearcherError::Inference(e) => write!(f, "inference failed: {e}"),
            ResearcherError::McpSurvey(e) => write!(f, "MCP survey failed: {e}"),
            ResearcherError::NoClaims => write!(f, "the pass surveyed no claims"),
        }
    }
}
impl std::error::Error for ResearcherError {}

// ───────────────────────────────────────────────────────────────────────────
// The Researcher Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Story 8.11 / 18-3 — the bound Inference Port, its deferred binding and the
/// count of completions it returned (the root's `--once` verdict compares it
/// with the surveyed claims).
#[derive(Clone)]
struct LiveInference {
    port: Arc<dyn InferencePort + Send + Sync>,
    binding: Arc<Mutex<Option<ResearcherInferenceBinding>>>,
    consumed: Arc<AtomicUsize>,
}

impl std::fmt::Debug for LiveInference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveInference")
            .field(
                "bound",
                &self.binding.lock().is_ok_and(|binding| binding.is_some()),
            )
            .finish_non_exhaustive()
    }
}

/// Story 18-3 (CR-1) — mints a fresh `ProviderInfer` token and the admitted
/// pid for ONE pass (D-18-3-03 A: the root's audited issuance, 60 s TTL).
pub type InferenceMint = Arc<dyn Fn() -> Result<(CapabilityToken, u32), String> + Send + Sync>;

/// Story 18-3 — a host's audited `ProviderInfer` issuance for `(pid,
/// provider)` (the `maos run` root passes its Butler seam's mint).
pub type ProviderMint = Arc<dyn Fn(u32, &str) -> Result<CapabilityToken, String> + Send + Sync>;

/// Story 18-3 — the Spirit's admitted pid once the scheduler assigned it.
pub type PidSource = Arc<dyn Fn() -> Option<u32> + Send + Sync>;

/// Story 18-3 (D-18-3-01 A) — the root journals one seed claim payload at
/// `pid` as a Transparency Log frame and returns that frame's id.
pub type SeedSink = Arc<dyn Fn(u32, &[u8]) -> Result<[u8; 16], String> + Send + Sync>;

/// Story 18-3 (CR-1, mirrors `butler::ButlerInferenceBinding`) — the provider
/// the token and every request name, and the per-pass mint.
#[derive(Clone)]
pub struct ResearcherInferenceBinding {
    pub provider_id: String,
    pub mint: InferenceMint,
}

impl ResearcherInferenceBinding {
    /// D-18-3-03 A — each pass mints at the pid `pid_of` resolves then.
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

/// Story 18-3 (D-18-3-02 B) — the provider a Researcher's token and requests
/// name: its first declared `ProviderInfer` the host router registers; none
/// registered is a typed boot refusal naming every declared provider.
pub fn declared_provider(declared: &[String], registered: &[String]) -> Result<String, String> {
    if declared.is_empty() {
        return Err("researcher inference: the Spirit declares no provider".into());
    }
    declared
        .iter()
        .find(|provider| registered.contains(provider))
        .cloned()
        .ok_or_else(|| {
            let names: Vec<String> = declared.iter().map(|p| format!("`{p}`")).collect();
            format!(
                "researcher inference: no inference provider is configured for its declared \
                 providers {}",
                names.join(", ")
            )
        })
}

/// Story 18-3 — the scheduler id the Researcher's scalar writes claim (the
/// root's adapter refuses any other).
pub const RESEARCHER_SPIRIT_ID: &str = "researcher";

/// Story 18-3 (D-18-3-01 A) — the TL intent of a journaled seed claim.
pub const SEED_CLAIM_INTENT: &str = "researcher:seed_claim";

/// Story 18-3 (D-18-3-01 A) — what a bound pass surveys when no MCP port is
/// wired: two strong-methodology claims that contradict each other on one
/// topic (`methodology_conflict` 0.8 against the manifest's 0.7 halt — the
/// J-Researcher contradiction halt) and one hedged load-bearing claim.
pub fn seed_claims() -> Vec<ClaimPayload> {
    let claim =
        |id: &str, statement: &str, topic: &str, strength: f32, polarity: bool| ClaimPayload {
            claim_id: id.into(),
            statement: statement.into(),
            topic: topic.into(),
            methodology_strength: strength,
            confidence: strength,
            load_bearing: false,
            polarity,
            hedges: Vec::new(),
        };
    vec![
        claim(
            "seed-rag-helps",
            "Retrieval augmentation reduces hallucinated citations in literature surveys.",
            "rag-citation-accuracy",
            0.85,
            true,
        ),
        claim(
            "seed-rag-noisy",
            "Retrieval augmentation does not reduce hallucinated citations over a noisy corpus.",
            "rag-citation-accuracy",
            0.8,
            false,
        ),
        ClaimPayload {
            load_bearing: true,
            hedges: vec!["may improve".into(), "appear limited".into()],
            ..claim(
                "seed-self-consistency",
                "Self-consistency voting may improve factual recall, though gains appear limited \
                 to larger models.",
                "self-consistency",
                0.75,
                true,
            )
        },
    ]
}

/// Story 8.11 / 18-3 — the finding-synthesis prompt for one source claim
/// (cassette entries pin its SHA-256 and length).
pub fn synthesis_prompt(statement: &str) -> String {
    format!(
        "Summarize this research claim in one sentence for a digest. \
         Preserve any hedges; add no facts.\n\nClaim: {statement}"
    )
}

/// Idle surveys retain fetched MCP frames and resume strictly after the last
/// successfully fetched entry, including when the previous page was terminal.
#[derive(Debug, Default)]
struct McpRecallState {
    cursor: Option<LogRecallCursor>,
    frames: Vec<RecalledFrame>,
}

/// Researcher reference Spirit. Optionally holds the frames its `on_idle`
/// survey reasons over (in production the walker seeds these from `log.recall`).
#[derive(Clone)]
pub struct Researcher {
    /// The cognitive posture-set (§6.2). `Survey` is active at v0.5; `Hypothesize`
    /// is declared but its generative path is not implemented (Decision C).
    posture_set: Vec<ResearcherPosture>,
    /// Frames to survey on the next `on_idle` (seeded by the walker / fixture).
    /// When `mcp_port` is `Some`, `on_idle` uses the MCP fan-out instead.
    pending: Option<Vec<RecalledFrame>>,
    /// The most recent survey output. `Arc<Mutex<...>>` so Researcher stays
    /// `Sync` as required by the `#[spirit]` macro.
    last_output: Arc<Mutex<Option<SurveyOutput>>>,
    /// Story 8.11 / AC2 — `None` (the hermetic-CI default) ⇒ the deterministic
    /// survey, byte-identical to v0.5. `Some` ⇒ live finding-synthesis through
    /// the Inference Port. The walker, every cite, and the I11 chain are
    /// unchanged in BOTH modes.
    inference: Option<LiveInference>,
    /// Story 8.14c — `None` (default) ⇒ byte-identical v0.5 path. `Some` ⇒
    /// `on_idle` fans out over real MCP servers, walks the participant-scoped
    /// log for McpInvocation frames, joins by `source_key`, then surveys.
    mcp_port: Option<Arc<dyn ResearcherMcpPort>>,
    /// Story 8.14c — the participant-scoped `LogRecallPort` used to recall
    /// McpInvocation frames after the fan-out. Required when `mcp_port` is `Some`.
    log_recall_port: Option<Arc<dyn LogRecallPort>>,
    /// Incremental MCP log walk shared by cloned hook instances.
    mcp_recall: Arc<Mutex<McpRecallState>>,
    /// Story 8.14c — the REAL spirit pid, backfilled after scheduler load.
    spirit_pid: Arc<AtomicU32>,
    /// Story 13.5d — mediated collective tier, wired by the composition root.
    collective_port: Option<Arc<dyn ResearcherCollectivePort>>,
    /// Set only after the production `on_idle` hook writes and reads back the
    /// mediated collective readiness row.
    collective_probe_completed: Arc<AtomicBool>,
    /// Set when the latest collective readiness round-trip fails so a one-shot
    /// driver can return a non-zero status without changing the hook ABI.
    collective_probe_failed: Arc<AtomicBool>,
    /// Story 18-3 (D-18-3-05 A) — the typed refusal of the most recent pass,
    /// taken by the composition root (`--once` exits non-zero on it).
    pass_error: Arc<Mutex<Option<ResearcherError>>>,
    /// Story 18-3 (D-18-3-01 A) — journals the seed claims a bound pass with
    /// no MCP port surveys; the journaled frames are shared by clones.
    seed_sink: Option<SeedSink>,
    seed_frames: Arc<Mutex<Option<Vec<RecalledFrame>>>>,
    /// Story 18-3 (AC4.1) — a surveyed pass writes its distillate here (I11).
    distillation_port: Option<Arc<dyn DistillationPort + Send + Sync>>,
    /// Story 18-3 (D-18-3-09 A) — a surveyed pass writes its primary scalar
    /// here; the kernel owns the `[epistemic_policy]` halt.
    scalar_port: Option<Arc<dyn EpistemicScalarPort>>,
}

impl std::fmt::Debug for Researcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Researcher")
            .field("posture_set", &self.posture_set)
            .field("pending", &self.pending)
            .field("last_output", &self.last_output)
            .field("inference", &self.inference)
            .field("mcp_port", &self.mcp_port.is_some())
            .field("log_recall_port", &self.log_recall_port.is_some())
            .field("collective_port", &self.collective_port.is_some())
            .field("scalar_port", &self.scalar_port.is_some())
            .field("distillation_port", &self.distillation_port.is_some())
            .field("spirit_pid", &self.spirit_pid)
            .finish()
    }
}

#[spirit]
impl Researcher {
    /// Survey idle pass. Cancellation-aware; bounded (a single linear pass over
    /// the pending frames, well within `time_cap_seconds`).
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        match self.ensure_collective_route() {
            Ok(()) => self.collective_probe_failed.store(false, Ordering::Release),
            Err(error) => {
                self.collective_probe_failed.store(true, Ordering::Release);
                eprintln!("researcher: collective readiness round-trip failed: {error}");
            }
        }
        // Story 18-3 (D-18-3-05 A) — a bound pass infers or is refused typed;
        // the deterministic pass (ADR-064 unset mode) keeps its v0.5 shape.
        let Some(live) = &self.inference else {
            let frames = match self.input_frames() {
                Ok(Some(frames)) => frames,
                Ok(None) => return,
                Err(error) => return eprintln!("researcher: {error}"),
            };
            let output = self.survey(&frames);
            if let Err(error) = self.persist(&output, self.spirit_pid.load(Ordering::Acquire)) {
                eprintln!("researcher: {error}");
            }
            *self.last_output.lock().unwrap_or_else(|e| e.into_inner()) = Some(output);
            return;
        };
        match self.bound_pass(live) {
            Ok(output) => {
                *self.last_output.lock().unwrap_or_else(|e| e.into_inner()) = Some(output);
            }
            Err(error) => self.refuse(error),
        }
    }
}

impl Default for Researcher {
    fn default() -> Self {
        Self {
            posture_set: vec![ResearcherPosture::Survey, ResearcherPosture::Hypothesize],
            pending: None,
            last_output: Arc::new(Mutex::new(None)),
            inference: None,
            mcp_port: None,
            log_recall_port: None,
            mcp_recall: Arc::new(Mutex::new(McpRecallState::default())),
            spirit_pid: Arc::new(AtomicU32::new(0)),
            collective_port: None,
            collective_probe_completed: Arc::new(AtomicBool::new(false)),
            collective_probe_failed: Arc::new(AtomicBool::new(false)),
            pass_error: Arc::new(Mutex::new(None)),
            seed_sink: None,
            seed_frames: Arc::new(Mutex::new(None)),
            distillation_port: None,
            scalar_port: None,
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

    /// Wire the Inference Port before scheduler load (Story 18-3, CR-1: the
    /// binding names the provider and mints per pass; `None` until bound).
    pub fn with_deferred_inference_port(
        mut self,
        port: Arc<dyn InferencePort + Send + Sync>,
        binding: Arc<Mutex<Option<ResearcherInferenceBinding>>>,
    ) -> Self {
        self.inference = Some(LiveInference {
            port,
            binding,
            consumed: Arc::new(AtomicUsize::new(0)),
        });
        self
    }

    /// Convenience builder for callers that already own an admitted pid and
    /// a token for `provider_id`: every pass reuses that token.
    pub fn with_inference_port(
        self,
        port: Arc<dyn InferencePort + Send + Sync>,
        provider_id: &str,
        token: CapabilityToken,
        spirit_pid: u32,
    ) -> Self {
        let binding = ResearcherInferenceBinding {
            provider_id: provider_id.into(),
            mint: Arc::new(move || Ok((token.clone(), spirit_pid))),
        };
        self.with_deferred_inference_port(port, Arc::new(Mutex::new(Some(binding))))
    }

    /// Story 18-3 (D-18-3-01 A) — the root's seed journal.
    pub fn with_seed_sink(mut self, sink: SeedSink) -> Self {
        self.seed_sink = Some(sink);
        self
    }

    /// Story 18-3 (AC4.1) — the root's `DistillateWriter`.
    pub fn with_distillation_port(mut self, port: Arc<dyn DistillationPort + Send + Sync>) -> Self {
        self.distillation_port = Some(port);
        self
    }

    /// Story 18-3 (D-18-3-09 A) — the root's epistemic-scalar write port.
    pub fn with_scalar_port(mut self, port: Arc<dyn EpistemicScalarPort>) -> Self {
        self.scalar_port = Some(port);
        self
    }

    /// Story 18-3 (D-18-3-05 A) — take the typed refusal of the most recent
    /// pass, if it was refused.
    pub fn take_pass_error(&self) -> Option<ResearcherError> {
        self.pass_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Story 18-3 — completions the bound Inference Port returned so far.
    pub fn completions_consumed(&self) -> usize {
        self.inference
            .as_ref()
            .map_or(0, |live| live.consumed.load(Ordering::SeqCst))
    }

    /// Story 8.14c — wire the MCP domain port. `maos run` supplies the
    /// `LiveResearcherMcpPort` (parallelism-8 fan-out over web/arXiv/GitHub/
    /// citation-graph). When set, `on_idle` fans out instead of surveying
    /// `self.pending`.
    pub fn with_mcp_port(mut self, port: Arc<dyn ResearcherMcpPort>) -> Self {
        self.mcp_port = Some(port);
        self
    }

    /// Story 13.5d — wire the mediated collective port. The port issues its
    /// capability token at each operation, after scheduler PID backfill.
    pub fn with_collective_port(mut self, port: Arc<dyn ResearcherCollectivePort>) -> Self {
        self.collective_port = Some(port);
        self
    }

    /// A shared status flag for one-shot drivers to observe readiness failures.
    pub fn collective_route_failure_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.collective_probe_failed)
    }

    fn ensure_collective_route(&self) -> Result<(), ResearcherCollectiveError> {
        if self.collective_port.is_none() || self.collective_probe_completed.load(Ordering::Acquire)
        {
            return Ok(());
        }
        if self
            .collective_probe_completed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }

        let expected = MemoryValue::Text("researcher collective route ready".into());
        let result = (|| {
            self.collective_write(
                &MemoryNamespace::Default,
                "researcher/collective-route-ready",
                expected.clone(),
            )?;
            match self.collective_read(
                &MemoryNamespace::Default,
                "researcher/collective-route-ready",
            )? {
                Some(actual) if actual == expected => Ok(()),
                other => Err(ResearcherCollectiveError::Denied(format!(
                    "collective readiness read-back mismatch: {other:?}"
                ))),
            }
        })();
        if result.is_err() {
            self.collective_probe_completed
                .store(false, Ordering::Release);
        }
        result
    }

    /// Write through the mediated collective boundary.
    ///
    /// A constructed but unwired Researcher fails closed; the composition root
    /// must install the port after the tenant store exists.
    pub fn collective_write(
        &self,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), ResearcherCollectiveError> {
        self.collective_port
            .as_ref()
            .ok_or(ResearcherCollectiveError::Unavailable)?
            .collective_write(namespace, key, value)
    }

    /// Read through the mediated collective boundary.
    pub fn collective_read(
        &self,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, ResearcherCollectiveError> {
        self.collective_port
            .as_ref()
            .ok_or(ResearcherCollectiveError::Unavailable)?
            .collective_read(namespace, key)
    }

    /// Scan through the mediated collective boundary.
    pub fn collective_scan(
        &self,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, ResearcherCollectiveError> {
        self.collective_port
            .as_ref()
            .ok_or(ResearcherCollectiveError::Unavailable)?
            .collective_scan(namespace, prefix, limit)
    }
    /// Wire participant-scoped recall. The scheduler pid is atomically
    /// backfilled after load, before the first hook fires.
    pub fn with_log_recall_port(
        mut self,
        port: Arc<dyn LogRecallPort>,
        spirit_pid: Arc<AtomicU32>,
    ) -> Self {
        self.log_recall_port = Some(port);
        self.spirit_pid = spirit_pid;
        self.mcp_recall = Arc::new(Mutex::new(McpRecallState::default()));
        self
    }

    /// Whether the live-inference seam is wired (the `--live` path).
    pub fn is_live(&self) -> bool {
        self.inference.is_some()
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

    // ── Story 8.14c — source-key join (FORK 2) ───────────────────────────────

    /// Join `FetchedClaim`s to McpInvocation `RecalledFrame`s by exact
    /// `source_key` match. Each joined frame carries the `ClaimPayload` as its
    /// payload and the McpInvocation frame id as its `frame_id`, so `survey()`
    /// cites the genuine kernel-journaled fetch frame.
    ///
    /// The join filters to Phase-2 intents (`get_paper`/`fetch`/`get_repo`/
    /// `get_citations`) — Phase-1 search/traverse frames are excluded because
    /// their args carry a query, not a citable source-key.
    pub fn join_claims_to_frames(
        &self,
        claims: &[FetchedClaim],
        mcp_frames: &[RecalledFrame],
    ) -> Vec<RecalledFrame> {
        let mut result = Vec::new();
        for claim in claims {
            for frame in mcp_frames {
                // Only Phase-2 fetch intents are citable.
                if !frame.intent.starts_with("mcp:")
                    || frame.intent.ends_with("/search")
                    || frame.intent.ends_with("/traverse")
                    || frame.intent.ends_with("/search_code")
                {
                    continue;
                }
                if let Ok(args) = serde_json::from_slice::<serde_json::Value>(&frame.payload) {
                    let frame_key = args
                        .get("arxiv_id")
                        .or_else(|| args.get("url"))
                        .or_else(|| args.get("repo"))
                        .or_else(|| args.get("paper_id"))
                        .and_then(|v| v.as_str());
                    if frame_key == Some(&claim.source_key) {
                        result.push(RecalledFrame {
                            frame_id: frame.frame_id,
                            intent: frame.intent.clone(),
                            payload: serde_json::to_vec(&claim.claim).unwrap_or_default(),
                        });
                        break;
                    }
                }
            }
        }
        result
    }

    // ── survey cognition (the deterministic seeded compressor) ───────────────

    /// Survey the recalled frames into a [`SurveyOutput`] with the
    /// deterministic compressor: never the model, on a bound Researcher too
    /// (its model pass is [`Researcher::infer_survey`]).
    ///
    /// - **findings**: one per claim frame; hedges preserved verbatim.
    /// - **methodology_conflict** scalar: two claims on the same `topic`, both
    ///   `methodology_strength ≥ 0.7`, opposite `polarity` → `min(ms_a, ms_b)`.
    /// - **load_bearing_confidence** scalar: the weakest load-bearing confidence.
    /// - **open_questions**: contradictions + claims below the 0.7 floor.
    pub fn survey(&self, frames: &[RecalledFrame]) -> SurveyOutput {
        match self.survey_with(frames, |statement| {
            Ok::<_, std::convert::Infallible>(summarize(statement))
        }) {
            Ok(output) => output,
            Err(never) => match never {},
        }
    }

    /// Story 18-3 (AC1.2) — the survey a pass emits: through the bound
    /// Inference Port (one mint, one completion per claim, the request naming
    /// the bound provider) or, unbound, [`Researcher::survey`]. A missing
    /// binding, a refused mint or any `InferenceError` is a typed error —
    /// never a fallback to the deterministic summary.
    pub fn infer_survey(&self, frames: &[RecalledFrame]) -> Result<SurveyOutput, ResearcherError> {
        let Some(live) = &self.inference else {
            return Ok(self.survey(frames));
        };
        let (binding, token, pid) = Self::mint(live)?;
        Self::survey_bound(self, live, &binding, &token, pid, frames)
    }

    fn mint(
        live: &LiveInference,
    ) -> Result<(ResearcherInferenceBinding, CapabilityToken, u32), ResearcherError> {
        let binding = live
            .binding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| ResearcherError::Unbound("the inference binding is not ready".into()))?;
        let (token, pid) = (binding.mint)().map_err(ResearcherError::Unbound)?;
        Ok((binding, token, pid))
    }

    fn survey_bound(
        &self,
        live: &LiveInference,
        binding: &ResearcherInferenceBinding,
        token: &CapabilityToken,
        pid: u32,
        frames: &[RecalledFrame],
    ) -> Result<SurveyOutput, ResearcherError> {
        self.survey_with(frames, |statement| {
            let req = InferenceRequest::new(
                pid,
                token.clone(),
                synthesis_prompt(statement),
                InferenceOptions {
                    max_tokens: 256,
                    temperature: Some(0.0),
                    model_id: None,
                },
                Some(binding.provider_id.clone()),
                Vec::new(),
            );
            let response = live
                .port
                .complete(req)
                .map_err(ResearcherError::Inference)?;
            live.consumed.fetch_add(1, Ordering::SeqCst);
            // §9.5: the model's text is bounded, so the digest stays smaller
            // than its inputs regardless of model verbosity.
            Ok(summarize(&response.text))
        })
    }

    /// Story 18-3 — one bound pass: mint, input (MCP fan-out, pending frames
    /// or the journaled seed), model survey, scalar, distillate.
    fn bound_pass(&self, live: &LiveInference) -> Result<SurveyOutput, ResearcherError> {
        let (binding, token, pid) = Self::mint(live)?;
        // The minted pid is the admitted one on every construction path (a
        // door-loaded Researcher's recall pid is never backfilled otherwise).
        self.spirit_pid.store(pid, Ordering::Release);
        let frames = match self.input_frames()? {
            Some(frames) => frames,
            None => self.seed_frames(pid)?,
        };
        let output = self.survey_bound(live, &binding, &token, pid, &frames)?;
        if output.findings.is_empty() {
            return Err(ResearcherError::NoClaims);
        }
        self.persist(&output, pid)?;
        Ok(output)
    }

    /// Story 8.14c — MCP fan-out → scoped walk → join; else the pending
    /// frames; `None` when neither is wired.
    fn input_frames(&self) -> Result<Option<Vec<RecalledFrame>>, ResearcherError> {
        let Some(mcp_port) = &self.mcp_port else {
            return Ok(self.pending.clone());
        };
        // v0.5 fixture placeholder — production derives query from Spirit's
        // pending research context or operator configuration.
        let fetched = mcp_port
            .survey_literature("positional-bias")
            .map_err(ResearcherError::McpSurvey)?;
        let log_port = self.log_recall_port.as_ref().ok_or_else(|| {
            ResearcherError::Internal(
                "MCP port wired but no LogRecallPort — cannot join claims".into(),
            )
        })?;
        let mut state = self.mcp_recall.lock().unwrap_or_else(|e| e.into_inner());
        let filter = LogRecallFilter {
            kind: Some(FrameKindLabel::McpInvocation),
            cursor: state.cursor.clone(),
            order_by_insertion: true,
            ..Default::default()
        };
        let spirit_pid = self.spirit_pid.load(Ordering::Acquire);
        let entries =
            recall_all(log_port.as_ref(), spirit_pid, filter).map_err(ResearcherError::Recall)?;
        for entry in entries {
            let frame = fetch_payload(log_port.as_ref(), spirit_pid, &entry)
                .map_err(ResearcherError::Recall)?;
            state.frames.push(frame);
            state.cursor = Some(LogRecallCursor::new(entry.timestamp_ns, entry.frame_id));
        }
        Ok(Some(self.join_claims_to_frames(&fetched, &state.frames)))
    }

    /// Story 18-3 (D-18-3-01 A) — the seed claims, journaled once through the
    /// root's sink at the pass's pid so every finding cites a resolvable frame.
    fn seed_frames(&self, pid: u32) -> Result<Vec<RecalledFrame>, ResearcherError> {
        let mut seeded = self.seed_frames.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(frames) = seeded.as_ref() {
            return Ok(frames.clone());
        }
        let sink = self
            .seed_sink
            .as_ref()
            .ok_or(ResearcherError::McpSurvey(ResearcherMcpError::Unconfigured))?;
        let mut frames = Vec::new();
        for claim in seed_claims() {
            let payload = serde_json::to_vec(&claim)
                .map_err(|e| ResearcherError::Internal(format!("seed encode: {e}")))?;
            let frame_id = sink(pid, &payload)
                .map_err(|e| ResearcherError::Internal(format!("seed journal: {e}")))?;
            frames.push(RecalledFrame {
                frame_id,
                intent: SEED_CLAIM_INTENT.into(),
                payload,
            });
        }
        *seeded = Some(frames.clone());
        Ok(frames)
    }

    /// Story 18-3 (D-18-3-09 A, AC4.1) — the surveyed pass's primary scalar
    /// (the kernel decides the halt) and its I11 distillate. The request is
    /// built first (review EC-8): a pass it refuses writes no scalar.
    fn persist(&self, output: &SurveyOutput, pid: u32) -> Result<(), ResearcherError> {
        let request = match &self.distillation_port {
            Some(port) => Some((port, self.to_distillation_request(output, 1)?)),
            None => None,
        };
        if let Some(port) = &self.scalar_port {
            let (tag, value, derived_from) = output.primary_scalar();
            port.write_scalar(pid, RESEARCHER_SPIRIT_ID, tag, value, &derived_from)
                .map_err(|e| ResearcherError::Internal(format!("scalar write refused: {e}")))?;
        }
        if let Some((port, request)) = request {
            port.write_distillate(pid, request)
                .map_err(ResearcherError::Distillation)?;
        }
        Ok(())
    }

    /// Refuse this pass typed (D-18-3-05 A): no `last_output`, and the error
    /// for the root to take.
    fn refuse(&self, error: ResearcherError) {
        eprintln!("researcher: on_idle pass refused: {error}");
        *self.last_output.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.pass_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
    }

    fn survey_with<E>(
        &self,
        frames: &[RecalledFrame],
        mut synthesize: impl FnMut(&str) -> Result<String, E>,
    ) -> Result<SurveyOutput, E> {
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
                    if confidence_map
                        .insert(claim.claim_id.clone(), confidence)
                        .is_some()
                    {
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
                        // Story 8.11 / 18-3 — the bound model's text, else the
                        // deterministic bounded summary.
                        statement: synthesize(&claim.statement)?,
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
            .fold(None, |acc: Option<f32>, c| {
                Some(acc.map_or(c, |a| a.min(c)))
            })
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

        Ok(SurveyOutput {
            findings,
            open_questions,
            confidence_map,
            bibliography,
            scalars,
        })
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
        let filter = LogRecallFilter {
            kind: base.kind.clone(),
            since_ns: base.since_ns,
            until_ns: base.until_ns,
            limit: base.limit.min(LogRecallFilter::MAX_LIMIT),
            cursor: cursor.clone(),
            intent_filter: base.intent_filter.clone(),
            order_by_insertion: base.order_by_insertion,
        };
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
        out.push(fetch_payload(port, spirit_pid, entry)?);
    }
    Ok(out)
}

fn fetch_payload(
    port: &dyn LogRecallPort,
    spirit_pid: u32,
    entry: &LogRecallEntry,
) -> Result<RecalledFrame, LogRecallError> {
    let payload = if entry.payload_available {
        port.fetch(spirit_pid, entry.frame_id)?.payload_redacted
    } else {
        Vec::new()
    };
    Ok(RecalledFrame {
        frame_id: entry.frame_id,
        intent: entry.intent.clone(),
        payload,
    })
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
    let head: String = graphemes
        .into_iter()
        .take(MAX_FINDING_SUMMARY_CHARS)
        .collect();
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
    use std::fmt::Write as _;
    let mut label = String::with_capacity(39);
    for (index, pair) in id.chunks_exact(2).enumerate() {
        if index != 0 {
            label.push(':');
        }
        write!(label, "{:02x}{:02x}", pair[0], pair[1])
            .expect("writing a frame id to a String cannot fail");
    }
    label
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
        *byte = u8::from_str_radix(s, 16)
            .map_err(|_| ResearcherError::BadFrameIdHex(hex.to_string()))?;
    }
    Ok(out)
}
#[cfg(test)]
mod unit_tests {
    use super::*;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::log_recall::{FrameKindLabel, LogFetchResponse, LogRecallPage};

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
        assert_eq!(
            out.findings[0].source_log_ref,
            encode_frame_id_hex(&[0x11; 16])
        );
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
        assert!(
            value >= 0.7,
            "conflict {value} must cross the 0.7 halt floor"
        );
    }

    #[test]
    fn weak_load_bearing_confidence_drives_the_second_rule() {
        let a = claim_frame(0x03, &claim("lb", "x", 0.4, 0.55, true, true));
        let out = Researcher::new().survey(&[a]);
        let (tag, value, _) = out.primary_scalar();
        assert_eq!(tag, "load_bearing_confidence");
        assert!(
            value < 0.7,
            "weak load-bearing confidence crosses the below-0.7 floor"
        );
        assert!(out
            .open_questions
            .iter()
            .any(|q| q.contains("confidence floor")));
    }

    #[test]
    fn output_serializes_with_required_fields() {
        let f = claim_frame(0x21, &claim("c", "t", 0.9, 0.9, true, false));
        let out = Researcher::new().survey(&[f]);
        let v = serde_json::to_value(&out).unwrap();
        for field in [
            "findings",
            "open_questions",
            "confidence_map",
            "bibliography",
        ] {
            assert!(
                v.get(field).is_some(),
                "missing required output field {field}"
            );
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
        let err = Researcher::new()
            .to_distillation_request(&empty, 1)
            .unwrap_err();
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
                seq: 0,
                spirit_id: "observer".into(),
                tag: "drift".into(),
                value: 0.42,
                timestamp: 1,
            },
        );
        assert_eq!(out.open_questions.len(), before + 1);
        assert!(out
            .confidence_map
            .keys()
            .any(|k| k.starts_with("observed::")));
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

    // ── Story 8.14c — MCP fan-out + join + survey (FORK 1 + FORK 2 + FORK 3) ──

    /// A fake `LogRecallPort` that returns pre-baked McpInvocation frames.
    struct FakeLogRecallPort {
        frames: Vec<RecalledFrame>,
    }
    impl LogRecallPort for FakeLogRecallPort {
        fn recall(
            &self,
            _spirit_pid: u32,
            _filter: LogRecallFilter,
        ) -> Result<LogRecallPage, LogRecallError> {
            Ok(LogRecallPage {
                entries: self
                    .frames
                    .iter()
                    .map(|f| LogRecallEntry {
                        frame_id: f.frame_id,
                        timestamp_ns: 0,
                        kind: FrameKindLabel::McpInvocation,
                        intent: f.intent.clone(),
                        peer_spirit_pid: 0,
                        payload_available: true,
                    })
                    .collect(),
                next_cursor: None,
                scope: maos_domain::log_recall::RecallScope::Participant,
            })
        }
        fn fetch(
            &self,
            _spirit_pid: u32,
            frame_id: [u8; 16],
        ) -> Result<LogFetchResponse, LogRecallError> {
            self.frames
                .iter()
                .find(|f| f.frame_id == frame_id)
                .map(|f| LogFetchResponse {
                    frame_id,
                    timestamp_ns: 0,
                    kind: FrameKindLabel::McpInvocation,
                    intent: f.intent.clone(),
                    payload_redacted: f.payload.clone(),
                    capability_token: None,
                    origin: FrameOrigin::SpiritAuto,
                })
                .ok_or(LogRecallError::ScopeViolation {
                    frame_id,
                    requested_pid: 0,
                    owner_pid: 0,
                })
        }
    }
    #[test]
    fn mcp_fan_out_joins_claims_to_invocation_frames() {
        let claim = ClaimPayload {
            claim_id: "arxiv-2501.12345".into(),
            statement: "LLM-as-judge exhibits positional bias.".into(),
            topic: "positional-bias".into(),
            methodology_strength: 0.85,
            confidence: 0.92,
            load_bearing: true,
            polarity: true,
            hedges: vec!["likely".into()],
        };
        let fetched = vec![FetchedClaim {
            claim: claim.clone(),
            source_key: "2501.12345".into(),
        }];
        let mcp_frames = vec![RecalledFrame {
            frame_id: [0xAB; 16],
            intent: "mcp:arxiv/get_paper".into(),
            payload: serde_json::to_vec(&serde_json::json!({"arxiv_id": "2501.12345"})).unwrap(),
        }];
        let joined = Researcher::new().join_claims_to_frames(&fetched, &mcp_frames);
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].frame_id, [0xAB; 16]);
        assert_eq!(
            serde_json::from_slice::<ClaimPayload>(&joined[0].payload).unwrap(),
            claim
        );
    }
    #[test]
    fn on_idle_surveys_via_mcp_port_when_wired() {
        let claim = ClaimPayload {
            claim_id: "arxiv-2501.12345".into(),
            statement: "Positional bias exists.".into(),
            topic: "positional-bias".into(),
            methodology_strength: 0.9,
            confidence: 0.95,
            load_bearing: true,
            polarity: true,
            hedges: vec![],
        };
        let mcp_port = FakeResearcherMcpPort {
            claims: vec![FetchedClaim {
                claim: claim.clone(),
                source_key: "2501.12345".into(),
            }],
        };
        let log_port = FakeLogRecallPort {
            frames: vec![RecalledFrame {
                frame_id: [0xCD; 16],
                intent: "mcp:arxiv/get_paper".into(),
                payload: serde_json::to_vec(&serde_json::json!({"arxiv_id": "2501.12345"}))
                    .unwrap(),
            }],
        };
        let researcher = Researcher::new()
            .with_mcp_port(Arc::new(mcp_port))
            .with_log_recall_port(Arc::new(log_port), Arc::new(AtomicU32::new(42)));
        let mut ctx = Ctx::mock();
        researcher.on_idle(&mut ctx);
        let output = researcher.last_output().unwrap();
        assert_eq!(output.findings.len(), 1);
        assert_eq!(
            output.findings[0].source_log_ref,
            encode_frame_id_hex(&[0xCD; 16])
        );
    }

    #[test]
    fn on_idle_falls_back_to_pending_when_mcp_port_is_none() {
        let f = claim_frame(0x11, &claim("c1", "t", 0.9, 0.95, true, false));
        let researcher = Researcher::with_frames(vec![f.clone()]);
        let mut ctx = Ctx::mock();
        researcher.on_idle(&mut ctx);
        let output = researcher.last_output().unwrap();
        assert_eq!(output.findings.len(), 1);
    }

    /// W5 — Negative falsifiability: a fabricated source_key that never appeared
    /// in any McpInvocation frame joins to nothing → survey produces empty findings.
    #[test]
    fn fabricated_cite_replays_empty() {
        let fabricated = FetchedClaim {
            claim: ClaimPayload {
                claim_id: "fake-1".into(),
                statement: "Never fetched.".into(),
                topic: "positional-bias".into(),
                methodology_strength: 0.9,
                confidence: 0.95,
                load_bearing: true,
                polarity: true,
                hedges: vec![],
            },
            source_key: "never-fetched-paper-id".into(),
        };
        let mcp_frames = vec![RecalledFrame {
            frame_id: [0xAB; 16],
            intent: "mcp:arxiv/get_paper".into(),
            payload: serde_json::to_vec(&serde_json::json!({"arxiv_id": "real-paper-id"})).unwrap(),
        }];
        let joined = Researcher::new().join_claims_to_frames(&vec![fabricated], &mcp_frames);
        assert!(joined.is_empty(), "fabricated key must not match any frame");
        let output = Researcher::new().survey(&joined);
        assert!(output.findings.is_empty(), "empty join → empty findings");
        assert_eq!(
            output
                .scalars
                .get("methodology_conflict")
                .copied()
                .unwrap_or(0.0),
            0.0
        );
    }

    /// W6 partial — Determinism floor: survey over fixed frames produces the
    /// same output shape every run (catches Option<McpPort> plumbing perturbing
    /// ordering or scalar values even when None).
    #[test]
    fn survey_over_fixed_frames_is_deterministic() {
        let f1 = claim_frame(
            0x11,
            &ClaimPayload {
                claim_id: "c1".into(),
                statement: "s1".into(),
                topic: "t".into(),
                methodology_strength: 0.9,
                confidence: 0.95,
                load_bearing: true,
                polarity: true,
                hedges: vec!["likely".into()],
            },
        );
        let f2 = claim_frame(
            0x22,
            &ClaimPayload {
                claim_id: "c2".into(),
                statement: "s2".into(),
                topic: "t".into(),
                methodology_strength: 0.85,
                confidence: 0.88,
                load_bearing: true,
                polarity: false,
                hedges: vec!["likely".into()],
            },
        );
        let out1 = Researcher::new().survey(&[f1.clone(), f2.clone()]);
        let out2 = Researcher::new().survey(&[f1.clone(), f2.clone()]);
        // Byte-identical serialization is the determinism guard.
        let bytes1 = serde_json::to_vec(&out1).unwrap();
        let bytes2 = serde_json::to_vec(&out2).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "survey must be deterministic for fixed input"
        );
        // The contradictory pair (polarity true vs false, both ms ≥ 0.7) must
        assert!(
            out1.scalars
                .get("methodology_conflict")
                .copied()
                .unwrap_or(0.0)
                >= 0.7,
            "Chen-vs-Tanaka shape: opposite polarity + ms≥0.7 → conflict≥0.7, got {}",
            out1.scalars
                .get("methodology_conflict")
                .copied()
                .unwrap_or(0.0)
        );
    }
    #[derive(Default)]
    struct RecordingCollectivePort {
        writes: std::sync::atomic::AtomicUsize,
        fail_writes: std::sync::atomic::AtomicBool,
        reads: std::sync::atomic::AtomicUsize,
        value: Mutex<Option<MemoryValue>>,
    }

    impl ResearcherCollectivePort for RecordingCollectivePort {
        fn collective_write(
            &self,
            _namespace: &MemoryNamespace,
            _key: &str,
            value: MemoryValue,
        ) -> Result<(), ResearcherCollectiveError> {
            if self.fail_writes.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(ResearcherCollectiveError::Denied("write failed".into()));
            }
            self.writes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.value.lock().unwrap() = Some(value);
            Ok(())
        }

        fn collective_read(
            &self,
            _namespace: &MemoryNamespace,
            _key: &str,
        ) -> Result<Option<MemoryValue>, ResearcherCollectiveError> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.value.lock().unwrap().clone())
        }

        fn collective_scan(
            &self,
            _namespace: &MemoryNamespace,
            _prefix: &str,
            _limit: usize,
        ) -> Result<Vec<MemoryEntry>, ResearcherCollectiveError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn collective_route_is_fail_closed_until_wired_then_reaches_port() {
        let unwired = Researcher::new();
        assert!(matches!(
            unwired.collective_write(
                &MemoryNamespace::Default,
                "dead-wire",
                MemoryValue::Text("must not land".into()),
            ),
            Err(ResearcherCollectiveError::Unavailable)
        ));

        let port = Arc::new(RecordingCollectivePort::default());
        let wired = Researcher::new().with_collective_port(port.clone());
        let mut ctx = Ctx::mock();
        wired.on_idle(&mut ctx);
        wired.on_idle(&mut ctx);
        assert_eq!(
            port.writes.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the production hook must make one falsifiable backing-port write"
        );
        assert_eq!(
            port.reads.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the production hook must read its backing row once and remain idempotent"
        );
    }

    #[test]
    fn collective_readiness_failure_is_observable() {
        let port = Arc::new(RecordingCollectivePort::default());
        port.fail_writes
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let researcher = Researcher::new().with_collective_port(port);
        let failure = researcher.collective_route_failure_flag();
        let mut ctx = Ctx::mock();
        researcher.on_idle(&mut ctx);
        assert!(failure.load(std::sync::atomic::Ordering::Acquire));
    }
}
