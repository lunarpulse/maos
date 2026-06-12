#![forbid(unsafe_code)]

//! `onboarding_gate_corpus` — Story 7.5b (NFR-Onb-1 / NFR-Onb-3 / NFR-Onb-4).
//!
//! The mechanically-buildable half of the **30-Minute First Spirit Validation
//! Gate**. This module ships the *gate-execution infrastructure* — it does NOT
//! recruit humans and it does NOT run the live N=12 / 14-day trial (those are an
//! out-of-band human-research activity that *consumes* these artifacts). What
//! lives here:
//!
//!   * AC3 — [`StratificationValidator`]: a cohort manifest is PASS only if it
//!     meets the N=12 strata floor, FAIL names the deficient stratum.
//!   * AC4 — [`resolve_corpus`] + [`score_candidate`]: the Butler-corpus **seam**
//!     (prefer the real `spirits/butler/...` corpus, fall back to the SHA-pinned
//!     7.5b fixture) and the deterministic per-candidate scoring math
//!     (halt-recall on the calendar-conflict subset, halt-precision overall,
//!     corpus pass, time-to-success).
//!   * AC5 — [`evaluate_cohort`]: PASS only when the full NFR-Onb-1 floor is met
//!     (≥10/12 succeed, median ≤45 min, p95 ≤90 min); the verdict is stamped
//!     `provisional` whenever it was scored against the fixture corpus so a
//!     fixture-only PASS can never be mistaken for the live v0.3 gate.
//!   * AC6 — [`CadenceMachine`]: the NFR-Onb-4 iteration cadence — misses are
//!     ledgered and 3 consecutive misses raise [`EscalateReleaseReview`].
//!   * AC1 — [`classify_prerequisites`]: the prerequisite + seam classification
//!     surfaced by `xtask nfr-onb-1-gate`.
//!
//! ## Scoring contract (read before extending — Story 8.1 plugs in here)
//!
//! At the v0.3 prerequisite, [`crate`]'s sibling `LocalRunner` produces a
//! `RunReport` whose `mock_bus_frames` is EMPTY (the mock Ctx gives Spirits no
//! capability handles, so they cannot emit — that is Story 2.4's job). So
//! halt-recall / halt-precision can NOT be read off the bus yet. The corpus
//! scenario therefore carries the ground-truth `expected_halt` AND a baked
//! `observed_halt` (the fixture's stand-in for a competent first-Spirit's
//! response). [`score_candidate`] takes an optional `observations` map
//! (`scenario_id -> observed_halt`): when `Some` (Story 8.1's real path derives
//! it from bus-observed halts) it is used; when `None` (the v0.3 fixture
//! self-trial) it falls back to the scenario's baked `observed_halt`. This is
//! the seam: Story 8.1's real Butler corpus + Story 2.4's bus capture drop in
//! WITHOUT changing this module's public surface.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// NFR-Onb-1 floor constants — the single source of truth for the gate.
// ---------------------------------------------------------------------------

/// Cohort target size (NFR-Onb-1).
pub const COHORT_TARGET_N: usize = 12;
/// ≥10 of 12 participants must succeed.
pub const COHORT_SUCCESS_FLOOR: usize = 10;
/// Median time-to-success must be ≤45 minutes.
pub const MEDIAN_MINUTES_MAX: f64 = 45.0;
/// p95 time-to-success must be ≤90 minutes.
pub const P95_MINUTES_MAX: f64 = 90.0;
/// Halt-recall on the calendar-conflict subset must be ≥0.90.
pub const HALT_RECALL_FLOOR: f64 = 0.90;
/// Halt-precision overall must be ≥0.85.
pub const HALT_PRECISION_FLOOR: f64 = 0.85;
/// NFR-Onb-1 corpus size (the Butler-class regression corpus the candidate
/// Spirit is scored against).
pub const CORPUS_SCENARIO_COUNT: usize = 30;

/// NFR-Onb-4 — consecutive-miss escalation threshold.
pub const ESCALATE_AFTER_CONSECUTIVE_MISSES: u32 = 3;
/// NFR-Onb-4 — the directive surfaced on every miss.
pub const MISS_DIRECTIVE: &str = "run a fresh 6-author cohort within 2 weeks";

/// 7.5b-owned fixture corpus path (relative to workspace root). STAND-IN for
/// Story 8.1's canonical Butler corpus — never written to the `spirits/butler`
/// path (ownership boundary).
pub const FIXTURE_CORPUS_REL: &str =
    "crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl";
/// Story 8.1's canonical Butler corpus path (relative to workspace root). When
/// this exists the resolver PREFERS it and the verdict becomes non-provisional.
pub const BUTLER_CORPUS_REL: &str = "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl";

// ---------------------------------------------------------------------------
// AC3 — stratification: cohort manifest + validator
// ---------------------------------------------------------------------------

/// A single participant record from a cohort manifest. The boolean flags map
/// 1:1 to the stratification strata in `nfr-onb-1-screener.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantRecord {
    /// Opaque/redacted participant id (never a real name — see protocol doc).
    pub participant_id: String,
    /// Stratum 1 — has made no prior MAOS contribution.
    pub no_prior_maos_contribution: bool,
    /// Stratum 2 — has never written a Rust Spirit.
    pub never_wrote_rust_spirit: bool,
    /// Stratum 3 — has never written Rust at all.
    pub never_wrote_rust: bool,
    /// Stratum 4 — is not a native English speaker.
    pub non_english_native: bool,
    /// The participant's native language (informational; for post-hoc analysis
    /// and bias auditing). Not consumed by the gate computation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_language: Option<String>,
    /// Stratum 5 — works offline-only.
    pub offline_only: bool,
}

/// A cohort manifest conforming to `nfr-onb-1-cohort.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortManifest {
    pub cohort_id: String,
    pub participants: Vec<ParticipantRecord>,
}

impl CohortManifest {
    /// Load a cohort manifest from a JSON file.
    pub fn load_from(path: &Path) -> Result<Self, crate::CorpusError> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| crate::CorpusError::Parse {
            path: path.display().to_string(),
            source: e,
        })
    }
}

/// The five stratification strata + their floors (NFR-Onb-1). Named so a FAIL
/// can point at the exact deficient stratum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stratum {
    NoPriorMaosContribution,
    NeverWroteRustSpirit,
    NeverWroteRust,
    NonEnglishNative,
    OfflineOnly,
}

impl Stratum {
    /// The minimum count required for this stratum in an N=12 cohort.
    pub const fn floor(self) -> usize {
        match self {
            Stratum::NoPriorMaosContribution => 4,
            Stratum::NeverWroteRustSpirit => 3,
            Stratum::NeverWroteRust => 2,
            Stratum::NonEnglishNative => 2,
            Stratum::OfflineOnly => 1,
        }
    }

    pub const ALL: [Stratum; 5] = [
        Stratum::NoPriorMaosContribution,
        Stratum::NeverWroteRustSpirit,
        Stratum::NeverWroteRust,
        Stratum::NonEnglishNative,
        Stratum::OfflineOnly,
    ];
}

/// A single failing stratum (or the N gate) in a stratification result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratumFailure {
    pub stratum: String,
    pub required: usize,
    pub actual: usize,
}

/// Result of running the stratification validator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratificationResult {
    pub passed: bool,
    pub cohort_size: usize,
    pub failures: Vec<StratumFailure>,
}

/// AC3 — assert N=12 AND every stratum floor; FAIL names the deficient stratum.
pub fn validate_stratification(cohort: &CohortManifest) -> StratificationResult {
    let n = cohort.participants.len();
    let mut failures = Vec::new();

    if n != COHORT_TARGET_N {
        failures.push(StratumFailure {
            stratum: "cohort_size".into(),
            required: COHORT_TARGET_N,
            actual: n,
        });
    }

    for stratum in Stratum::ALL {
        let actual = cohort
            .participants
            .iter()
            .filter(|p| match stratum {
                Stratum::NoPriorMaosContribution => p.no_prior_maos_contribution,
                Stratum::NeverWroteRustSpirit => p.never_wrote_rust_spirit,
                Stratum::NeverWroteRust => p.never_wrote_rust,
                Stratum::NonEnglishNative => p.non_english_native,
                Stratum::OfflineOnly => p.offline_only,
            })
            .count();
        if actual < stratum.floor() {
            failures.push(StratumFailure {
                stratum: serde_json::to_value(stratum)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| format!("{stratum:?}")),
                required: stratum.floor(),
                actual,
            });
        }
    }

    StratificationResult {
        passed: failures.is_empty(),
        cohort_size: n,
        failures,
    }
}

// ---------------------------------------------------------------------------
// AC4 — Butler-class corpus seam + per-candidate scoring
// ---------------------------------------------------------------------------

/// Which corpus the resolver landed on. Drives the `provisional` stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusSource {
    /// Story 8.1's canonical Butler corpus (real → verdict NOT provisional).
    Butler,
    /// The 7.5b SHA-pinned fixture stand-in (→ verdict provisional).
    Fixture,
}

impl CorpusSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CorpusSource::Butler => "butler",
            CorpusSource::Fixture => "fixture",
        }
    }
    /// A fixture-sourced verdict is always provisional.
    pub fn is_provisional(self) -> bool {
        matches!(self, CorpusSource::Fixture)
    }
}

/// The resolved corpus: which source, the absolute path, and its SHA-256.
#[derive(Debug, Clone)]
pub struct ResolvedCorpus {
    pub source: CorpusSource,
    pub path: PathBuf,
    pub sha256: String,
}

/// The expected SHA-256 of the 7.5b fixture corpus (pinned in the README).
/// When the resolver lands on the fixture, it asserts the computed digest
/// matches this pin — a tamper-evidence check that fails loudly on drift.
pub const FIXTURE_CORPUS_SHA256: &str =
    "1a5b0738e959b537f1dd07993ed0d9978c889e1348d20a2007a03235d03d9110";

/// Validate that a loaded corpus has the expected scenario count. Called by
/// the gate and self-trial after loading from a real file (unit tests that
/// build synthetic corpora bypass this).
pub fn validate_corpus_size(corpus: &OnboardingCorpus) -> Result<(), String> {
    if corpus.scenarios.is_empty() {
        return Err("corpus must contain at least one scenario — an empty corpus would vacuously pass every candidate".into());
    }
    if corpus.scenarios.len() != CORPUS_SCENARIO_COUNT {
        return Err(format!(
            "corpus must contain exactly {CORPUS_SCENARIO_COUNT} scenarios, \
             found {} — a truncated or mis-sized corpus would produce unreliable scores",
            corpus.scenarios.len()
        ));
    }
    Ok(())
}

/// AC4 — corpus resolver. PREFERS the canonical Butler corpus
/// (`spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`) and FALLS BACK to
/// the 7.5b fixture, logging which corpus + SHA-256 it used. Fail-loud: the
/// caller is expected to surface `source`/`sha256` (the gate + self-trial do).
pub fn resolve_corpus(workspace_root: &Path) -> Result<ResolvedCorpus, crate::CorpusError> {
    let butler = workspace_root.join(BUTLER_CORPUS_REL);
    let (source, path) = if butler.is_file() {
        (CorpusSource::Butler, butler)
    } else {
        (
            CorpusSource::Fixture,
            workspace_root.join(FIXTURE_CORPUS_REL),
        )
    };
    let bytes = std::fs::read(&path)?;
    let sha256 = sha256_hex(&bytes);
    if source == CorpusSource::Fixture {
        assert_eq!(
            sha256, FIXTURE_CORPUS_SHA256,
            "fixture corpus SHA-256 drift detected — the fixture was modified without updating \
             FIXTURE_CORPUS_SHA256"
        );
    }
    Ok(ResolvedCorpus {
        source,
        path,
        sha256,
    })
}

/// SHA-256 of the given bytes, lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Fixture corpus header/manifest line (first JSONL record) marking it a
/// STAND-IN for Story 8.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusMeta {
    /// Always present on the meta line; distinguishes it from scenario lines.
    pub stand_in_for: String,
    pub corpus: String,
    pub note: String,
}

/// One Butler-class regression scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnbScenario {
    pub scenario_id: String,
    /// True iff this scenario is part of the calendar-conflict subset that
    /// halt-recall is measured over.
    pub calendar_conflict: bool,
    /// Ground truth — should a correct Spirit halt/flag on this scenario?
    pub expected_halt: bool,
    /// v0.3 fixture STAND-IN for the candidate's observed response. Story 8.1's
    /// real path derives this from the bus instead (see module scoring contract).
    pub observed_halt: bool,
}

/// A loaded Butler-class corpus.
#[derive(Debug, Clone)]
pub struct OnboardingCorpus {
    pub meta: Option<CorpusMeta>,
    pub scenarios: Vec<OnbScenario>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CorpusLine {
    Meta(CorpusMeta),
    Scenario(OnbScenario),
}

impl CorpusLine {
    fn from_str(line: &str, source: &str) -> Result<Self, crate::CorpusError> {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| crate::CorpusError::Parse {
                path: source.into(),
                source: e,
            })?;
        if v.get("stand_in_for").is_some() {
            serde_json::from_value(v)
                .map(Self::Meta)
                .map_err(|e| crate::CorpusError::Parse {
                    path: source.into(),
                    source: e,
                })
        } else {
            serde_json::from_value(v)
                .map(Self::Scenario)
                .map_err(|e| crate::CorpusError::Parse {
                    path: source.into(),
                    source: e,
                })
        }
    }
}

impl OnboardingCorpus {
    /// Load a JSONL corpus. The first line MAY be a [`CorpusMeta`] header
    /// (present in the fixture); every other line is an [`OnbScenario`].
    pub fn load_jsonl(path: &Path) -> Result<Self, crate::CorpusError> {
        let content = std::fs::read_to_string(path)?;
        let mut meta = None;
        let mut scenarios = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parsed = CorpusLine::from_str(line, &path.display().to_string())?;
            match parsed {
                CorpusLine::Meta(m) => meta = Some(m),
                CorpusLine::Scenario(s) => scenarios.push(s),
            }
        }
        Ok(Self { meta, scenarios })
    }

    /// Scenarios in the calendar-conflict subset.
    pub fn calendar_conflict_subset(&self) -> impl Iterator<Item = &OnbScenario> {
        self.scenarios.iter().filter(|s| s.calendar_conflict)
    }
}

/// Inputs the scorer cannot derive from the corpus alone.
#[derive(Debug, Clone)]
pub struct CandidateInput {
    /// Opaque/redacted participant id.
    pub participant_id: String,
    /// Did the candidate Spirit compile against the published ABI?
    pub compiles_against_abi: bool,
    /// Measured time-to-success in minutes.
    pub time_to_success_min: f64,
    /// Did the candidate complete within the 14-day trial window?
    pub within_window: bool,
}

/// One scored candidate — emitted as a row in `outcomes.jsonl`
/// (conforms to `nfr-onb-1-outcomes.schema.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateOutcome {
    pub participant_id: String,
    pub corpus_source: String,
    pub corpus_sha256: String,
    pub compiles_against_abi: bool,
    /// Candidate produced a decision for every one of the 30 scenarios.
    pub corpus_pass: bool,
    pub halt_recall_calendar_conflict: f64,
    pub halt_precision_overall: f64,
    pub time_to_success_min: f64,
    pub within_window: bool,
    /// `succeed` = (a) ∧ (b) ∧ (c) ∧ (d) within the window — exactly NFR-Onb-1.
    pub succeed: bool,
    /// True iff scored against the fixture corpus (never the live gate).
    pub provisional: bool,
}

/// AC4 — score ONE candidate against the resolved corpus.
///
/// `observations` is the seam: `Some(map)` uses the candidate's
/// bus-observed halts (Story 8.1's real path); `None` falls back to each
/// scenario's baked `observed_halt` (the v0.3 fixture self-trial).
pub fn score_candidate(
    corpus: &OnboardingCorpus,
    resolved: &ResolvedCorpus,
    input: &CandidateInput,
    observations: Option<&BTreeMap<String, bool>>,
) -> CandidateOutcome {
    // Resolve observed-halt per scenario via the seam.
    let observed = |s: &OnbScenario| -> Option<bool> {
        match observations {
            Some(map) => map.get(&s.scenario_id).copied(),
            None => Some(s.observed_halt),
        }
    };

    assert!(
        !corpus.scenarios.is_empty(),
        "corpus must contain at least one scenario — an empty corpus would \
         vacuously pass every candidate"
    );

    // corpus_pass = the candidate produced a decision for EVERY scenario.
    let corpus_pass = corpus.scenarios.iter().all(|s| observed(s).is_some());

    // halt-recall over the calendar-conflict subset: TP / (TP + FN).
    // The denominator is restricted to `calendar_conflict && expected_halt`
    // because a calendar-conflict scenario with `expected_halt=false` is NOT
    // a missed halt — it is a correctly-ignored conflict. Including those in
    // the denominator would punish the system for correct behavior. Story 8.1's
    // corpus contract should document this semantic; if a plain "all cc" scope
    // is desired, that is a spec amendment, not a code divergence.
    let mut cc_tp = 0usize;
    let mut cc_expected = 0usize;
    // halt-precision overall: TP / (TP + FP)
    // (denominator = scenarios where the candidate halted).
    let mut tp = 0usize;
    let mut predicted_positive = 0usize;
    for s in &corpus.scenarios {
        let obs = observed(s).unwrap_or(false);
        if s.calendar_conflict && s.expected_halt {
            cc_expected += 1;
            if obs {
                cc_tp += 1;
            }
        }
        if obs {
            predicted_positive += 1;
            if s.expected_halt {
                tp += 1;
            }
        }
    }

    // A vacuous denominator scores 1.0 (no opportunity to miss).
    let halt_recall_calendar_conflict = ratio_or_one(cc_tp, cc_expected);
    let halt_precision_overall = ratio_or_one(tp, predicted_positive);

    let provisional = resolved.source.is_provisional();
    let succeed = input.compiles_against_abi
        && corpus_pass
        && halt_recall_calendar_conflict >= HALT_RECALL_FLOOR
        && halt_precision_overall >= HALT_PRECISION_FLOOR
        && input.within_window;

    CandidateOutcome {
        participant_id: input.participant_id.clone(),
        corpus_source: resolved.source.as_str().to_string(),
        corpus_sha256: resolved.sha256.clone(),
        compiles_against_abi: input.compiles_against_abi,
        corpus_pass,
        halt_recall_calendar_conflict,
        halt_precision_overall,
        time_to_success_min: input.time_to_success_min,
        within_window: input.within_window,
        succeed,
        provisional,
    }
}

fn ratio_or_one(num: usize, den: usize) -> f64 {
    if den == 0 {
        1.0
    } else {
        num as f64 / den as f64
    }
}

// ---------------------------------------------------------------------------
// AC5 — cohort gate evaluator
// ---------------------------------------------------------------------------

/// The cohort gate verdict. PASS only when the FULL NFR-Onb-1 floor is met;
/// every failing sub-criterion is named.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohortVerdict {
    pub passed: bool,
    pub cohort_size: usize,
    pub success_count: usize,
    pub median_time_min: Option<f64>,
    pub p95_time_min: Option<f64>,
    /// True iff ANY scored outcome was provisional (fixture corpus). A
    /// fixture-only PASS is stamped provisional so it can never be mistaken for
    /// the live v0.3 gate.
    pub provisional: bool,
    pub failing_criteria: Vec<String>,
}

/// AC5 — evaluate an `outcomes.jsonl` cohort against the NFR-Onb-1 floor.
/// `median`/`p95` are computed over the *successful* outcomes' time-to-success.
pub fn evaluate_cohort(outcomes: &[CandidateOutcome]) -> CohortVerdict {
    let cohort_size = outcomes.len();
    let success_count = outcomes.iter().filter(|o| o.succeed).count();
    let provisional = outcomes.iter().any(|o| o.provisional);

    let mut success_times: Vec<f64> = outcomes
        .iter()
        .filter(|o| o.succeed)
        .map(|o| o.time_to_success_min)
        .collect();
    success_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = median(&success_times);
    let p95 = percentile_nearest_rank(&success_times, 0.95);

    let mut failing_criteria = Vec::new();
    if success_count < COHORT_SUCCESS_FLOOR {
        failing_criteria.push(format!(
            "success-count {success_count} < required {COHORT_SUCCESS_FLOOR}"
        ));
    }
    match median {
        Some(m) if m <= MEDIAN_MINUTES_MAX => {}
        Some(m) => failing_criteria.push(format!(
            "median-time {m:.1}min > budget {MEDIAN_MINUTES_MAX:.0}min"
        )),
        None => failing_criteria.push("median-time undefined (no successful outcomes)".into()),
    }
    match p95 {
        Some(p) if p <= P95_MINUTES_MAX => {}
        Some(p) => failing_criteria.push(format!(
            "p95-time {p:.1}min > budget {P95_MINUTES_MAX:.0}min"
        )),
        None => failing_criteria.push("p95-time undefined (no successful outcomes)".into()),
    }

    CohortVerdict {
        passed: failing_criteria.is_empty(),
        cohort_size,
        success_count,
        median_time_min: median,
        p95_time_min: p95,
        provisional,
        failing_criteria,
    }
}

fn median(sorted: &[f64]) -> Option<f64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

/// Nearest-rank percentile on an ascending-sorted slice. `q` in [0,1].
fn percentile_nearest_rank(sorted: &[f64], q: f64) -> Option<f64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    // rank = ceil(q * n), clamped to [1, n]; index = rank - 1.
    let rank = (q * n as f64).ceil().max(1.0) as usize;
    let idx = rank.min(n) - 1;
    Some(sorted[idx])
}

// ---------------------------------------------------------------------------
// AC6 — NFR-Onb-4 iteration-cadence machinery
// ---------------------------------------------------------------------------

/// One run-ledger entry (`run-ledger.jsonl`, private; schema committed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunLedgerEntry {
    pub run_id: String,
    pub passed: bool,
    /// The NFR-Onb-4 directive surfaced on a miss (None on a PASS).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub directive: Option<String>,
}

/// The escalation signal raised after 3 consecutive misses (NFR-Onb-4):
/// PRD-author + architecture lead + research lead review the release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalateReleaseReview {
    pub consecutive_misses: u32,
    pub recipients: Vec<String>,
}

impl EscalateReleaseReview {
    fn new(consecutive_misses: u32) -> Self {
        Self {
            consecutive_misses,
            recipients: vec![
                "prd-author".into(),
                "architecture-lead".into(),
                "research-lead".into(),
            ],
        }
    }
}

/// The NFR-Onb-4 cadence machine. Records gate runs; a PASS resets the
/// consecutive-miss counter; 3 consecutive misses raise
/// [`EscalateReleaseReview`].
#[derive(Debug, Clone, Default)]
pub struct CadenceMachine {
    entries: Vec<RunLedgerEntry>,
    consecutive_misses: u32,
}

impl CadenceMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replay an existing ledger to reconstruct the consecutive-miss counter.
    pub fn from_ledger(entries: Vec<RunLedgerEntry>) -> Self {
        let mut consecutive = 0u32;
        for e in &entries {
            if e.passed {
                consecutive = 0;
            } else {
                consecutive += 1;
            }
        }
        Self {
            entries,
            consecutive_misses: consecutive,
        }
    }

    /// Record a gate run. Returns `Some(EscalateReleaseReview)` when the
    /// consecutive-miss count reaches (or remains at/beyond) the escalation
    /// threshold. The signal re-fires on every subsequent miss until a PASS
    /// resets the counter — the alarm condition persists until resolved.
    pub fn record(&mut self, run_id: &str, passed: bool) -> Option<EscalateReleaseReview> {
        let directive = if passed {
            self.consecutive_misses = 0;
            None
        } else {
            self.consecutive_misses += 1;
            Some(MISS_DIRECTIVE.to_string())
        };
        self.entries.push(RunLedgerEntry {
            run_id: run_id.to_string(),
            passed,
            directive,
        });
        if !passed && self.consecutive_misses >= ESCALATE_AFTER_CONSECUTIVE_MISSES {
            Some(EscalateReleaseReview::new(self.consecutive_misses))
        } else {
            None
        }
    }

    pub fn consecutive_misses(&self) -> u32 {
        self.consecutive_misses
    }

    pub fn entries(&self) -> &[RunLedgerEntry] {
        &self.entries
    }

    /// True once the escalation threshold has been reached.
    pub fn is_escalated(&self) -> bool {
        self.consecutive_misses >= ESCALATE_AFTER_CONSECUTIVE_MISSES
    }
}

// ---------------------------------------------------------------------------
// AC1 — prerequisite + seam classification
// ---------------------------------------------------------------------------

/// The AC1 prerequisite + seam classification (surfaced by `nfr-onb-1-gate`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereqReport {
    pub template_present: bool,
    pub local_runner_present: bool,
    pub example_spirit_present: bool,
    pub corpus_harness_present: bool,
    /// Butler corpus is EXPECTED absent at v0.3 → the seam is active.
    pub butler_corpus_absent: bool,
    pub seam_active: bool,
    pub corpus_source: String,
    pub all_prereqs_present: bool,
}

/// AC1 — classify the prerequisites + Butler-corpus seam mechanically.
pub fn classify_prerequisites(workspace_root: &Path) -> PrereqReport {
    let exists = |rel: &str| workspace_root.join(rel).exists();
    let file_contains = |rel: &str, needle: &str| -> bool {
        std::fs::read_to_string(workspace_root.join(rel))
            .map(|c| c.contains(needle))
            .unwrap_or(false)
    };

    let template_present = exists("templates/spirit-rust/Cargo.toml")
        && exists("templates/spirit-rust/manifest.toml")
        && exists("templates/spirit-rust/src/lib.rs");
    let local_runner_present = file_contains(
        "crates/maos-spirit-sdk/src/local_runner.rs",
        "impl LocalRunner",
    );
    let example_spirit_present = exists("examples/example-spirit/src/lib.rs");
    let corpus_harness_present =
        exists("crates/maos-eval/src/lib.rs") && exists(FIXTURE_CORPUS_REL);

    let butler = workspace_root.join(BUTLER_CORPUS_REL);
    let butler_corpus_absent = !butler.is_file();
    let corpus_source = if butler_corpus_absent {
        CorpusSource::Fixture
    } else {
        CorpusSource::Butler
    };

    PrereqReport {
        template_present,
        local_runner_present,
        example_spirit_present,
        corpus_harness_present,
        butler_corpus_absent,
        // The seam is "active" while the fixture stands in for the real corpus.
        seam_active: butler_corpus_absent,
        corpus_source: corpus_source.as_str().to_string(),
        all_prereqs_present: template_present
            && local_runner_present
            && example_spirit_present
            && corpus_harness_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> PathBuf {
        // tests run with CWD = crate dir (crates/maos-eval); workspace is ../..
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn participant(id: &str, flags: [bool; 5]) -> ParticipantRecord {
        ParticipantRecord {
            participant_id: id.into(),
            no_prior_maos_contribution: flags[0],
            never_wrote_rust_spirit: flags[1],
            never_wrote_rust: flags[2],
            non_english_native: flags[3],
            native_language: None,
            offline_only: flags[4],
        }
    }

    /// A cohort that meets every floor exactly (the redacted example shape).
    fn passing_cohort() -> CohortManifest {
        // 12 participants. Distribute flags to clear every floor:
        // no_prior ≥4, never_spirit ≥3, never_rust ≥2, non_english ≥2, offline ≥1.
        let mut participants = Vec::new();
        for i in 0..12 {
            participants.push(participant(
                &format!("P{i:02}"),
                [
                    i < 6,   // no_prior_maos_contribution: 6 (≥4)
                    i < 5,   // never_wrote_rust_spirit: 5 (≥3)
                    i < 3,   // never_wrote_rust: 3 (≥2)
                    i >= 9,  // non_english_native: 3 (≥2)
                    i == 11, // offline_only: 1 (≥1)
                ],
            ));
        }
        CohortManifest {
            cohort_id: "example".into(),
            participants,
        }
    }

    // ---- AC3 stratification ----

    #[test]
    fn ac3_passing_cohort_passes() {
        let r = validate_stratification(&passing_cohort());
        assert!(r.passed, "expected PASS, failures: {:?}", r.failures);
        assert_eq!(r.cohort_size, 12);
    }

    #[test]
    fn ac3_deficient_non_english_fails_with_correct_stratum() {
        let mut cohort = passing_cohort();
        // Knock non_english_native down to 1 (floor is 2).
        for (i, p) in cohort.participants.iter_mut().enumerate() {
            p.non_english_native = i == 11;
        }
        let r = validate_stratification(&cohort);
        assert!(!r.passed);
        assert_eq!(r.failures.len(), 1, "exactly one failing stratum");
        assert_eq!(r.failures[0].stratum, "non_english_native");
        assert_eq!(r.failures[0].required, 2);
        assert_eq!(r.failures[0].actual, 1);
    }

    #[test]
    fn ac3_wrong_n_fails() {
        let mut cohort = passing_cohort();
        cohort.participants.pop();
        let r = validate_stratification(&cohort);
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.stratum == "cohort_size"));
    }

    // ---- AC4 scoring math at the boundaries ----

    fn corpus_with(scenarios: Vec<OnbScenario>) -> OnboardingCorpus {
        OnboardingCorpus {
            meta: None,
            scenarios,
        }
    }

    fn scen(id: &str, cc: bool, expected: bool, observed: bool) -> OnbScenario {
        OnbScenario {
            scenario_id: id.into(),
            calendar_conflict: cc,
            expected_halt: expected,
            observed_halt: observed,
        }
    }

    fn fixture_resolved() -> ResolvedCorpus {
        ResolvedCorpus {
            source: CorpusSource::Fixture,
            path: PathBuf::from("fixture"),
            sha256: "deadbeef".into(),
        }
    }

    #[test]
    fn ac4_recall_exactly_at_floor_passes() {
        // 10 calendar-conflict scenarios all expected_halt; observe 9 → recall 0.9.
        let mut scenarios = Vec::new();
        for i in 0..10 {
            scenarios.push(scen(&format!("cc{i}"), true, true, i != 0));
        }
        let corpus = corpus_with(scenarios);
        let out = score_candidate(
            &corpus,
            &fixture_resolved(),
            &CandidateInput {
                participant_id: "p".into(),
                compiles_against_abi: true,
                time_to_success_min: 20.0,
                within_window: true,
            },
            None,
        );
        assert!((out.halt_recall_calendar_conflict - 0.9).abs() < 1e-9);
        // precision: 9 predicted-positive, all expected → 1.0
        assert!((out.halt_precision_overall - 1.0).abs() < 1e-9);
        assert!(out.succeed, "0.90 recall meets the ≥0.90 floor");
        assert!(out.provisional);
    }

    #[test]
    fn ac4_recall_below_floor_fails() {
        // 10 cc scenarios expected_halt; observe 8 → recall 0.8 < 0.90.
        let mut scenarios = Vec::new();
        for i in 0..10 {
            scenarios.push(scen(&format!("cc{i}"), true, true, i >= 2));
        }
        let out = score_candidate(
            &corpus_with(scenarios),
            &fixture_resolved(),
            &CandidateInput {
                participant_id: "p".into(),
                compiles_against_abi: true,
                time_to_success_min: 20.0,
                within_window: true,
            },
            None,
        );
        assert!((out.halt_recall_calendar_conflict - 0.8).abs() < 1e-9);
        assert!(!out.succeed, "0.80 recall is below the ≥0.90 floor");
    }

    #[test]
    fn ac4_precision_exactly_at_floor_passes() {
        // Overall: 20 predicted-positive, 17 true → precision 0.85.
        // Keep recall over the cc subset at 1.0 so only precision is at the edge.
        let mut scenarios = Vec::new();
        // calendar-conflict subset: 5 expected & observed (recall 1.0, all TP).
        for i in 0..5 {
            scenarios.push(scen(&format!("cc{i}"), true, true, true));
        }
        // non-cc: 12 more true-positive halts (expected & observed).
        for i in 0..12 {
            scenarios.push(scen(&format!("tp{i}"), false, true, true));
        }
        // non-cc: 3 false-positive halts (observed but not expected).
        for i in 0..3 {
            scenarios.push(scen(&format!("fp{i}"), false, false, true));
        }
        // Total predicted-positive = 5 + 12 + 3 = 20; true = 17 → 0.85.
        let out = score_candidate(
            &corpus_with(scenarios),
            &fixture_resolved(),
            &CandidateInput {
                participant_id: "p".into(),
                compiles_against_abi: true,
                time_to_success_min: 20.0,
                within_window: true,
            },
            None,
        );
        assert!(
            (out.halt_precision_overall - 0.85).abs() < 1e-9,
            "precision was {}",
            out.halt_precision_overall
        );
        assert!((out.halt_recall_calendar_conflict - 1.0).abs() < 1e-9);
        assert!(out.succeed, "0.85 precision meets the ≥0.85 floor");
    }

    #[test]
    fn ac4_observations_seam_overrides_baked_observed() {
        // Baked observed says halt; the candidate's real observations say no-halt.
        let corpus = corpus_with(vec![scen("cc0", true, true, true)]);
        let mut obs = BTreeMap::new();
        obs.insert("cc0".to_string(), false);
        let out = score_candidate(
            &corpus,
            &fixture_resolved(),
            &CandidateInput {
                participant_id: "p".into(),
                compiles_against_abi: true,
                time_to_success_min: 20.0,
                within_window: true,
            },
            Some(&obs),
        );
        // recall now 0 (missed the one expected halt).
        assert_eq!(out.halt_recall_calendar_conflict, 0.0);
    }

    #[test]
    fn ac4_non_compiling_candidate_never_succeeds() {
        let corpus = corpus_with(vec![scen("cc0", true, true, true)]);
        let out = score_candidate(
            &corpus,
            &fixture_resolved(),
            &CandidateInput {
                participant_id: "p".into(),
                compiles_against_abi: false,
                time_to_success_min: 5.0,
                within_window: true,
            },
            None,
        );
        assert!(!out.succeed);
    }

    // ---- AC5 cohort evaluator ----

    fn outcome(succeed: bool, time: f64, provisional: bool) -> CandidateOutcome {
        CandidateOutcome {
            participant_id: "p".into(),
            corpus_source: if provisional { "fixture" } else { "butler" }.into(),
            corpus_sha256: "x".into(),
            compiles_against_abi: succeed,
            corpus_pass: succeed,
            halt_recall_calendar_conflict: if succeed { 1.0 } else { 0.0 },
            halt_precision_overall: if succeed { 1.0 } else { 0.0 },
            time_to_success_min: time,
            within_window: true,
            succeed,
            provisional,
        }
    }

    fn cohort(successes: usize, fails: usize, success_time: f64) -> Vec<CandidateOutcome> {
        let mut v = Vec::new();
        for _ in 0..successes {
            v.push(outcome(true, success_time, true));
        }
        for _ in 0..fails {
            v.push(outcome(false, success_time, true));
        }
        v
    }

    #[test]
    fn ac5_ten_of_twelve_within_budget_passes() {
        let v = cohort(10, 2, 40.0);
        let verdict = evaluate_cohort(&v);
        assert!(verdict.passed, "failures: {:?}", verdict.failing_criteria);
        assert_eq!(verdict.success_count, 10);
        assert!(verdict.provisional, "fixture-sourced → provisional");
    }

    #[test]
    fn ac5_nine_of_twelve_fails_on_success_count() {
        let v = cohort(9, 3, 40.0);
        let verdict = evaluate_cohort(&v);
        assert!(!verdict.passed);
        assert_eq!(verdict.failing_criteria.len(), 1);
        assert!(verdict.failing_criteria[0].contains("success-count"));
    }

    #[test]
    fn ac5_median_over_budget_fails_on_median() {
        // 12 successes all at 50 min → median 50 > 45, p95 50 ≤ 90, count 12 ≥ 10.
        let v = cohort(12, 0, 50.0);
        let verdict = evaluate_cohort(&v);
        assert!(!verdict.passed);
        assert_eq!(verdict.failing_criteria.len(), 1);
        assert!(verdict.failing_criteria[0].contains("median-time"));
    }

    #[test]
    fn ac5_p95_over_budget_fails_on_p95() {
        // 11 successes at 40, 1 at 95 → median 40 ≤45, p95 95 >90, count 12 ≥10.
        let mut v = Vec::new();
        for _ in 0..11 {
            v.push(outcome(true, 40.0, true));
        }
        v.push(outcome(true, 95.0, true));
        let verdict = evaluate_cohort(&v);
        assert!(!verdict.passed);
        assert_eq!(
            verdict.failing_criteria.len(),
            1,
            "only p95 should fail: {:?}",
            verdict.failing_criteria
        );
        assert!(verdict.failing_criteria[0].contains("p95-time"));
    }

    #[test]
    fn ac5_real_corpus_is_not_provisional() {
        let v = cohort(10, 2, 40.0)
            .into_iter()
            .map(|mut o| {
                o.provisional = false;
                o.corpus_source = "butler".into();
                o
            })
            .collect::<Vec<_>>();
        let verdict = evaluate_cohort(&v);
        assert!(verdict.passed);
        assert!(!verdict.provisional);
    }

    // ---- AC6 cadence machinery ----

    #[test]
    fn ac6_three_consecutive_misses_escalate() {
        let mut m = CadenceMachine::new();
        assert!(m.record("r1", false).is_none());
        assert!(m.record("r2", false).is_none());
        let esc = m.record("r3", false).expect("3rd miss escalates");
        assert_eq!(esc.consecutive_misses, 3);
        assert_eq!(esc.recipients.len(), 3);
        assert!(m.is_escalated());
        assert!(m
            .entries()
            .iter()
            .all(|e| e.directive.as_deref() == Some(MISS_DIRECTIVE)));
    }

    #[test]
    fn ac6_fourth_miss_re_escalates() {
        let mut m = CadenceMachine::new();
        m.record("r1", false);
        m.record("r2", false);
        m.record("r3", false);
        let esc = m.record("r4", false).expect("4th miss re-escalates");
        assert_eq!(esc.consecutive_misses, 4);
        assert_eq!(m.consecutive_misses(), 4);
        assert!(m.is_escalated());
    }

    #[test]
    fn ac6_pass_resets_consecutive_counter() {
        let mut m = CadenceMachine::new();
        m.record("r1", false);
        m.record("r2", false);
        assert_eq!(m.consecutive_misses(), 2);
        assert!(m.record("r3", true).is_none(), "a PASS does not escalate");
        assert_eq!(m.consecutive_misses(), 0);
        // Two more misses after the reset must NOT escalate (only 2 in a row).
        assert!(m.record("r4", false).is_none());
        assert!(m.record("r5", false).is_none());
        assert_eq!(m.consecutive_misses(), 2);
    }

    #[test]
    fn ac6_from_ledger_reconstructs_counter() {
        let ledger = vec![
            RunLedgerEntry {
                run_id: "r1".into(),
                passed: true,
                directive: None,
            },
            RunLedgerEntry {
                run_id: "r2".into(),
                passed: false,
                directive: Some(MISS_DIRECTIVE.into()),
            },
            RunLedgerEntry {
                run_id: "r3".into(),
                passed: false,
                directive: Some(MISS_DIRECTIVE.into()),
            },
        ];
        let m = CadenceMachine::from_ledger(ledger);
        assert_eq!(m.consecutive_misses(), 2);
    }

    // ---- AC1 prerequisites / AC4 fixture load ----

    #[test]
    fn ac1_prerequisites_classified() {
        let report = classify_prerequisites(&workspace_root());
        assert!(
            report.template_present,
            "Story 2.3 template must be present"
        );
        assert!(report.local_runner_present);
        assert!(report.example_spirit_present);
        assert!(report.corpus_harness_present);
        // Story 8.1 LANDED the canonical Butler corpus → seam CLOSED, butler source.
        // (Updated from the 7.5b seam-active expectation; see Story 8.1 AC4.)
        assert!(!report.butler_corpus_absent);
        assert!(!report.seam_active);
        assert_eq!(report.corpus_source, "butler");
        assert!(report.all_prereqs_present);
    }

    #[test]
    fn ac4_butler_corpus_resolves_and_fixture_remains_a_valid_fallback() {
        // Story 8.1: the resolver now prefers the canonical Butler corpus the
        // instant it exists. (Renamed/updated from the 7.5b
        // `ac4_fixture_corpus_loads_and_resolves` which asserted the seam-active
        // Fixture state; see Story 8.1 AC4 — this is the documented maos-eval
        // test edit, justified by the seam closing.)
        let root = workspace_root();
        let resolved = resolve_corpus(&root).expect("resolve");
        assert_eq!(resolved.source, CorpusSource::Butler);
        assert_eq!(resolved.sha256.len(), 64, "sha-256 hex is 64 chars");
        let corpus = OnboardingCorpus::load_jsonl(&resolved.path).expect("load butler corpus");
        assert_eq!(
            corpus.scenarios.len(),
            CORPUS_SCENARIO_COUNT,
            "Butler corpus must carry exactly 30 scenarios"
        );
        assert!(
            corpus.meta.is_none(),
            "real Butler corpus carries NO stand_in_for meta line (Decision D)"
        );
        assert!(
            corpus.calendar_conflict_subset().count() > 0,
            "Butler corpus must carry a calendar-conflict subset"
        );

        // The 7.5b fixture remains a valid SHA-pinned fallback (its drift
        // assertion still guards it): loading it directly still yields 30
        // scenarios with the STAND-IN meta header.
        let fixture = OnboardingCorpus::load_jsonl(&root.join(FIXTURE_CORPUS_REL))
            .expect("load fixture fallback");
        assert_eq!(fixture.scenarios.len(), CORPUS_SCENARIO_COUNT);
        assert!(
            fixture.meta.is_some(),
            "fixture keeps its STAND-IN meta header"
        );
    }
}
