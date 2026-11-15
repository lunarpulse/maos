#![forbid(unsafe_code)]

//! Story 18-2 — the kernel-free half of `maos eval halt` (NFR-Test-4 per class).
//!
//! The Butler + kernel composition lives in `maos-bench` (D-18-2-01 B: the
//! kernel depends on this crate, so no kernel type may appear here). This
//! module scores the verdicts that composition produced, refuses a run that
//! never consumed the model, and defines the signed benchmark sidecar body.
//! The floors come from [`crate::onboarding_gate_corpus`] and are never
//! re-typed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::onboarding_gate_corpus::{
    score_candidate, CandidateInput, CorpusSource, OnboardingCorpus, ResolvedCorpus,
};

/// The floors one class is held to.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassFloors {
    /// The NFR-Onb-1 calendar-conflict recall; `None` for a class without
    /// that subset.
    pub halt_recall_calendar_conflict: Option<f64>,
    /// NFR-Test-4 class recall over every expected-halt row (D-18-2-02 B).
    pub halt_recall_overall: f64,
    pub halt_precision_overall: f64,
}

/// The numbers of one scored run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HaltScore {
    pub rows: usize,
    pub expected_halts: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub halt_recall_calendar_conflict: Option<f64>,
    pub halt_recall_overall: f64,
    pub halt_precision_overall: f64,
    /// Story 28-1 (D-28-1-05 A): rows whose completion was malformed. Each is
    /// scored as no halt (the shipped Spirit writes no scalar); the floor is 0.
    #[serde(default)]
    pub format_errors: usize,
}

/// Score `verdicts` (scenario id → halted) against `corpus`. Calendar-conflict
/// recall and overall precision are [`score_candidate`]'s; overall recall is
/// TP / expected halts. A row without a verdict is an error, never "no halt".
pub fn score_class(
    corpus: &OnboardingCorpus,
    sha256: &str,
    verdicts: &BTreeMap<String, bool>,
    floors: &ClassFloors,
) -> Result<HaltScore, String> {
    if corpus.scenarios.is_empty() {
        return Err("the corpus has no rows".into());
    }
    if let Some(row) = corpus
        .scenarios
        .iter()
        .find(|s| !verdicts.contains_key(&s.scenario_id))
    {
        return Err(format!("row {} has no verdict", row.scenario_id));
    }
    let (rows, distinct) = (corpus.scenarios.len(), verdicts.len());
    if rows != distinct {
        return Err(format!(
            "{rows} rows, {distinct} verdicts — a duplicate scenario_id"
        ));
    }
    let resolved = ResolvedCorpus {
        source: CorpusSource::Butler,
        path: Default::default(),
        sha256: sha256.to_owned(),
    };
    let input = CandidateInput {
        participant_id: "maos-eval-halt".into(),
        compiles_against_abi: true,
        time_to_success_min: 0.0,
        within_window: true,
    };
    let outcome = score_candidate(corpus, &resolved, &input, Some(verdicts));
    let (mut tp, mut fp, mut fn_) = (0, 0, 0);
    for s in &corpus.scenarios {
        match (s.expected_halt, verdicts[&s.scenario_id]) {
            (true, true) => tp += 1,
            (false, true) => fp += 1,
            (true, false) => fn_ += 1,
            (false, false) => {}
        }
    }
    let expected_halts = tp + fn_;
    Ok(HaltScore {
        rows: corpus.scenarios.len(),
        expected_halts,
        true_positives: tp,
        false_positives: fp,
        false_negatives: fn_,
        halt_recall_calendar_conflict: floors
            .halt_recall_calendar_conflict
            .map(|_| outcome.halt_recall_calendar_conflict),
        halt_recall_overall: if expected_halts == 0 {
            1.0
        } else {
            tp as f64 / expected_halts as f64
        },
        halt_precision_overall: outcome.halt_precision_overall,
        format_errors: 0,
    })
}

/// The floors `score` misses, each named with its value (empty = pass).
pub fn missed_floors(score: &HaltScore, floors: &ClassFloors) -> Vec<String> {
    let cc = score
        .halt_recall_calendar_conflict
        .zip(floors.halt_recall_calendar_conflict);
    let mut missed: Vec<String> = [
        cc.map(|(v, f)| ("halt_recall_calendar_conflict", v, f)),
        Some((
            "halt_recall_overall",
            score.halt_recall_overall,
            floors.halt_recall_overall,
        )),
        Some((
            "halt_precision_overall",
            score.halt_precision_overall,
            floors.halt_precision_overall,
        )),
    ]
    .into_iter()
    .flatten()
    .filter(|(_, value, floor)| value < floor)
    .map(|(name, value, floor)| format!("{name} {value:.3} below floor {floor:.2}"))
    .collect();
    if score.format_errors > 0 {
        let rate = score.format_errors as f64 / score.rows.max(1) as f64;
        missed.push(format!("format_error_rate {rate:.3} above floor 0.00"));
    }
    missed
}

/// Story 28-1 (D-28-1-04 A) — the schema id of a class's calibrated-floor
/// record, `spirits/<class>/eval/floors.toml`.
pub const FLOORS_SCHEMA: &str = "maos.eval.floors/v1";

/// One calibrated floor and the evidence it rests on (D-28-1-03 A: the
/// larger of the spec minimum and the Wilson lower bound of the tuned
/// measurement, rounded down to 0.01).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorEntry {
    /// A [`ClassFloors`] field name.
    pub metric: String,
    pub floor: f64,
    pub spec_minimum: f64,
    pub spec_source: String,
    /// The operator ruling that lowers a floor below the spec minimum.
    pub operator_ruling: Option<String>,
    pub measured_mean: f64,
    pub wilson95: [f64; 2],
    pub k: usize,
    pub model_id: String,
    pub prompt_sha256: String,
    pub rule: String,
    pub rationale: String,
    pub date: String,
    pub result_file: String,
}

/// A class's floor record: the file `maos eval halt` reads when present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorRecord {
    pub schema: String,
    pub floor: Vec<FloorEntry>,
}

/// `x` rounded strictly down to 0.01 (D-28-1-03 A). `xtask eval-report
/// --calibrate` calls this same function, so the calibrated number and the
/// load-time check cannot disagree.
pub fn round_down_hundredth(x: f64) -> f64 {
    (x * 100.0).floor() / 100.0
}

impl FloorRecord {
    /// `base` (the spec minimums) with this record's floors applied. Refused:
    /// a wrong schema, an unknown or repeated metric, a floor outside [0, 1],
    /// and, without an `operator_ruling`, a floor below the one its own
    /// evidence requires (D-28-1-03 A: the largest of the spec minimum, `base`
    /// and the recorded Wilson lower bound rounded down to 0.01).
    pub fn apply(&self, base: ClassFloors) -> Result<ClassFloors, String> {
        if self.schema != FLOORS_SCHEMA {
            return Err(format!("floor record schema is '{}'", self.schema));
        }
        let mut floors = base;
        let mut seen = Vec::new();
        for e in &self.floor {
            let (slot, compiled) = match e.metric.as_str() {
                "halt_recall_calendar_conflict" => (
                    floors.halt_recall_calendar_conflict.as_mut(),
                    base.halt_recall_calendar_conflict.unwrap_or_default(),
                ),
                "halt_recall_overall" => (
                    Some(&mut floors.halt_recall_overall),
                    base.halt_recall_overall,
                ),
                "halt_precision_overall" => (
                    Some(&mut floors.halt_precision_overall),
                    base.halt_precision_overall,
                ),
                other => return Err(format!("floor record metric '{other}' is not a floor")),
            };
            let slot = slot.ok_or_else(|| format!("class has no floor '{}'", e.metric))?;
            if seen.contains(&e.metric) {
                return Err(format!("floor record repeats '{}'", e.metric));
            }
            seen.push(e.metric.clone());
            let ruled = e
                .operator_ruling
                .as_deref()
                .is_some_and(|r| !r.trim().is_empty());
            let [lo, hi] = e.wilson95;
            let unit = |v: f64| (0.0..=1.0).contains(&v);
            if !(e.spec_minimum.is_finite()
                && unit(e.floor)
                && unit(e.measured_mean)
                && unit(lo)
                && unit(hi)
                && lo <= hi)
            {
                return Err(format!(
                    "floor record '{}': floor, measured_mean and wilson95 must be finite and \
                     within [0, 1] with lo <= hi, and spec_minimum finite",
                    e.metric
                ));
            }
            let lower = round_down_hundredth(lo);
            let required = e.spec_minimum.max(compiled).max(lower);
            if !ruled && e.floor < required {
                return Err(format!(
                    "floor {} of '{}' is below its calibrated {required} (spec minimum {}, \
                     Wilson lower bound {lower}) without an operator ruling",
                    e.floor, e.metric, e.spec_minimum
                ));
            }
            *slot = e.floor;
        }
        Ok(floors)
    }
}

/// A model-driven class's number is the model's only if every row consumed
/// exactly one completion (AC1: the arithmetic path is refused, not scored).
pub fn check_consumed(consumed: usize, rows: usize) -> Result<(), String> {
    if consumed == rows {
        Ok(())
    } else {
        Err(format!(
            "{consumed} of {rows} completions consumed — not a model-dependent number"
        ))
    }
}

/// The signed benchmark sidecar body (D-18-2-04 A). Its canonical bytes are
/// its compact JSON in field order; [`sidecar`] signs exactly those.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkBody {
    pub schema: String,
    pub class: String,
    pub spirit_version: String,
    pub maos_version: String,
    pub corpus: String,
    pub corpus_sha256: String,
    pub cassette: Option<String>,
    pub cassette_sha256: Option<String>,
    /// What the cassette says it is (`seed` / `live-record`), never more.
    pub provenance: Option<String>,
    pub completions_consumed: Option<usize>,
    pub model_driven: bool,
    pub model_driven_reason: Option<String>,
    /// What `halt_recall_calendar_conflict` measures when it is not the
    /// model's number (AC1.2: labelled arithmetic under the 18-1 contract).
    pub halt_recall_calendar_conflict_basis: Option<String>,
    pub score: HaltScore,
    pub floors: ClassFloors,
    pub missed_floors: Vec<String>,
    pub pass: bool,
    /// `producer` (`MAOS_EVAL_PRODUCER_SEED`) or `dev-seed`.
    pub signer: String,
}

/// The sidecar body's schema id.
pub const BENCHMARK_SCHEMA: &str = "maos.benchmark.halt/v1";

/// Ed25519 signing and verification of the sidecar, keyed like Story 11.7's
/// producer (`trial_attestation.rs`): `MAOS_EVAL_PRODUCER_SEED` /
/// `MAOS_EVAL_PRODUCER_PUBKEY` (64 hex chars each), else the documented
/// development seed below, whose signatures say `signer: dev-seed`.
#[cfg(feature = "benchmark-sidecar")]
pub mod sidecar {
    use super::{BenchmarkBody, BENCHMARK_SCHEMA};
    use maos_audit::release_verify::{sign_sha256sums, verify_release_signature};
    use maos_audit::sealed_export::derive_pubkey;
    use serde::{Deserialize, Serialize};
    use serde_json::value::RawValue;

    /// Documented development seed (local and CI-without-secret runs only):
    /// 32 bytes of `0x18 0x02`. A signature under it proves integrity, not
    /// provenance, and is labelled `dev-seed`.
    pub const DEV_SEED: [u8; 32] = [
        0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18,
        0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02, 0x18, 0x02,
        0x18, 0x02,
    ];

    /// The file: the body's exact signed bytes and its hex signature.
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SignedBenchmark {
        pub body: Box<RawValue>,
        pub signature: String,
    }

    fn key_from_env(name: &str) -> Result<Option<[u8; 32]>, String> {
        let hex = match std::env::var(name) {
            Err(std::env::VarError::NotPresent) => return Ok(None),
            // A non-UTF-8 value refuses like any malformed one; it never falls back.
            other => other.unwrap_or_default(),
        };
        // The value is never echoed: the seed is a secret.
        parse_hex(&hex)
            .and_then(|bytes| bytes.try_into().ok())
            .map(Some)
            .ok_or_else(|| format!("{name} is not 64 hex characters"))
    }

    fn parse_hex(hex: &str) -> Option<Vec<u8>> {
        if hex.len() % 2 != 0 || !hex.is_ascii() {
            return None;
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect()
    }

    /// The only `signer` label `pubkey` vouches for: the development seed's
    /// key ⇒ `dev-seed` (integrity, not provenance); any other ⇒ `producer`.
    pub fn signer_for(pubkey: &[u8; 32]) -> &'static str {
        if *pubkey == derive_pubkey(&DEV_SEED) {
            "dev-seed"
        } else {
            "producer"
        }
    }

    /// The producer seed and its label: `MAOS_EVAL_PRODUCER_SEED`, else the
    /// development seed. A malformed seed refuses; it never falls back.
    pub fn producer_seed() -> Result<([u8; 32], &'static str), String> {
        let seed = key_from_env("MAOS_EVAL_PRODUCER_SEED")?.unwrap_or(DEV_SEED);
        Ok((seed, signer_for(&derive_pubkey(&seed))))
    }

    /// The trusted pubkey: `MAOS_EVAL_PRODUCER_PUBKEY`, else the development
    /// seed's.
    pub fn producer_pubkey() -> Result<[u8; 32], String> {
        Ok(key_from_env("MAOS_EVAL_PRODUCER_PUBKEY")?.unwrap_or_else(|| derive_pubkey(&DEV_SEED)))
    }

    /// Sign `body` (whose `signer` the caller set from [`producer_seed`]) over
    /// its compact JSON, stored verbatim as the file's `body`.
    pub fn sign(body: BenchmarkBody, seed: &[u8; 32]) -> Result<Vec<u8>, String> {
        let canonical = serde_json::to_string(&body).map_err(|e| format!("sidecar encode: {e}"))?;
        let signature = sign_sha256sums(canonical.as_bytes(), seed)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let body = RawValue::from_string(canonical).map_err(|e| format!("sidecar encode: {e}"))?;
        serde_json::to_vec_pretty(&SignedBenchmark { body, signature })
            .map_err(|e| format!("sidecar encode: {e}"))
    }

    /// Verify the signature over the body's STORED bytes against `pubkey`
    /// (never a re-encoding), then parse it and refuse a body that is not
    /// `class` `version`'s, not [`BENCHMARK_SCHEMA`], or claims a signer
    /// `pubkey` does not vouch for. Returns the body and its exact bytes.
    pub fn verify(
        bytes: &[u8],
        pubkey: &[u8; 32],
        class: &str,
        version: &str,
    ) -> Result<(BenchmarkBody, Box<RawValue>), String> {
        let signed: SignedBenchmark =
            serde_json::from_slice(bytes).map_err(|e| format!("sidecar malformed: {e}"))?;
        let signature: [u8; 64] = parse_hex(&signed.signature)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or("sidecar signature is not 128 hex characters")?;
        verify_release_signature(signed.body.get().as_bytes(), &signature, pubkey)
            .map_err(|e| format!("sidecar signature verification failed: {e}"))?;
        let body: BenchmarkBody = serde_json::from_str(signed.body.get())
            .map_err(|e| format!("sidecar body malformed: {e}"))?;
        let claims = [
            ("schema", body.schema.as_str(), BENCHMARK_SCHEMA),
            ("class", body.class.as_str(), class),
            ("spirit_version", body.spirit_version.as_str(), version),
            ("signer", body.signer.as_str(), signer_for(pubkey)),
        ];
        if let Some((field, got, want)) = claims.into_iter().find(|(_, got, want)| got != want) {
            return Err(format!("sidecar {field} is '{got}', expected '{want}'"));
        }
        Ok((body, signed.body))
    }
}
