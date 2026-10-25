#![forbid(unsafe_code)]

//! Story 18-3 (D-18-3-06 A, D-18-3-07 A) — the kernel-free half of
//! `maos eval distill --class researcher`: NFR-Aud-7's five floors (the one
//! source — `prd/non-functional-requirements.md:62-67`), the frozen corpus row,
//! and the scorer. The replay composition (kernel world, Researcher, the
//! `DistillateWriter`, redaction) is `maos-bench`'s (D-18-2-01 B).
//!
//! Four metrics are deterministic string facts over what the Researcher
//! emits; faithfulness comes from frozen judge verdicts committed with the
//! corpus (`provenance: seed` until the nightly judge records them). Clean
//! scenarios are held to the floors; each negative control must MISS the
//! metric it plants, or the gate is blind.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// NFR-Aud-7 digest-recall floor.
pub const RECALL_FLOOR: f64 = 0.90;
/// NFR-Aud-7 digest-faithfulness floor.
pub const FAITHFULNESS_FLOOR: f64 = 0.98;
/// NFR-Aud-7 digest-hedge-preservation floor.
pub const HEDGE_PRESERVATION_FLOOR: f64 = 0.95;
/// NFR-Aud-7 digest-traceability: 100 %.
pub const TRACEABILITY_FLOOR: f64 = 1.0;
/// NFR-Aud-7 digest-secret-leakage: 0 % (zero tolerance).
pub const SECRET_LEAKAGE_CEILING: f64 = 0.0;

/// The frozen Researcher corpus `maos eval distill` replays.
pub const CORPUS: &str = "spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl";
/// Its committed completions (one per surveyed claim, corpus order).
pub const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/distill-corpus-v0.json";

/// The live judge's prompt template (nightly record only); its SHA-256 is
/// `tests/judge-config.toml`'s `[judge.researcher-faithfulness]` pin.
pub const JUDGE_PROMPT: &str = "You are auditing a research digest. Answer with exactly one \
word: `faithful` if the digest finding states nothing the source claim does not support and \
contradicts nothing in it, otherwise `unfaithful`.";

/// One judge prompt: the template, the source claim, the finding.
pub fn judge_prompt(source: &str, finding: &str) -> String {
    format!("{JUDGE_PROMPT}\n\nSource claim: {source}\nDigest finding: {finding}")
}

/// A live judge completion's verdict; anything but the two words is refused.
pub fn judge_verdict(text: &str) -> Result<bool, String> {
    match text.trim().trim_matches('`').to_ascii_lowercase().as_str() {
        "faithful" => Ok(true),
        "unfaithful" => Ok(false),
        other => Err(format!("judge answered `{other}`, not faithful/unfaithful")),
    }
}

/// The metric a negative control plants a failure into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Control {
    /// The completions drop the source hedges → hedge preservation.
    HedgeDrop,
    /// A claim is surveyed from a frame never journaled → traceability.
    FabricatedCite,
    /// A completion carries a secret → secret leakage.
    PlantedSecret,
    /// A source claim is withheld from the survey → recall.
    DroppedClaim,
}

/// Frozen judge verdicts: finding `claim_id` → faithful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenJudge {
    pub prompt_sha256: String,
    pub provenance: String,
    pub verdicts: BTreeMap<String, bool>,
}

/// One corpus row; `C` is the producer's claim type.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistillRow<C> {
    pub scenario_id: String,
    #[serde(default)]
    pub control: Option<Control>,
    pub claims: Vec<C>,
    /// Claim ids never offered to the survey (the dropped-claim control).
    #[serde(default)]
    pub withheld: Vec<String>,
    /// Claim ids surveyed from a frame the log never journaled.
    #[serde(default)]
    pub unjournaled: Vec<String>,
    /// The frozen survey output a replay must reproduce byte-for-byte.
    pub survey: serde_json::Value,
    pub judge: FrozenJudge,
}

/// The corpus rows and the SHA-256 of the corpus bytes.
pub fn load_rows<C: serde::de::DeserializeOwned>(text: &str) -> Result<Vec<DistillRow<C>>, String> {
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| format!("corpus row: {e}")))
        .collect::<Result<Vec<DistillRow<C>>, _>>()?;
    let mut seen = std::collections::BTreeSet::new();
    match rows
        .iter()
        .find(|row| !seen.insert(row.scenario_id.as_str()))
    {
        Some(dup) => Err(format!("duplicate scenario_id {}", dup.scenario_id)),
        None => Ok(rows),
    }
}

/// The deterministic frame id of claim `claim` in scenario `scenario`.
pub fn frame_id(scenario: usize, claim: usize) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[..4].copy_from_slice(b"rd18");
    id[4..8].copy_from_slice(&(scenario as u32).to_be_bytes());
    id[8..12].copy_from_slice(&(claim as u32).to_be_bytes());
    id
}

/// What one finding of an emitted survey is, measured.
#[derive(Debug, Clone, PartialEq)]
pub struct FindingFacts {
    pub claim_id: String,
    pub statement: String,
    /// The structural hedge list (never the metric's input: the statement is).
    pub hedges: Vec<String>,
    /// Its `source_log_ref` resolves in the Transparency Log.
    pub resolves: bool,
    /// The redaction policy fires on its statement.
    pub leaks: bool,
}

/// One scenario, measured: the source claims (id, hedges), the findings and
/// the faithfulness verdicts.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioFacts {
    pub scenario_id: String,
    pub control: Option<Control>,
    pub source: Vec<(String, Vec<String>)>,
    pub findings: Vec<FindingFacts>,
    pub verdicts: BTreeMap<String, bool>,
}

/// The five metrics over a set of scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FiveMetrics {
    pub recall: f64,
    pub faithfulness: f64,
    pub hedge_preservation: f64,
    pub traceability: f64,
    pub secret_leakage: f64,
}

/// A negative control's outcome: `detected` iff its metric missed its floor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlOutcome {
    pub scenario_id: String,
    pub control: Control,
    pub detected: bool,
}

/// A scored run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillScore {
    /// Over the clean scenarios.
    pub metrics: FiveMetrics,
    pub clean_scenarios: usize,
    pub controls: Vec<ControlOutcome>,
}

fn ratio(hit: usize, total: usize) -> f64 {
    if total == 0 {
        1.0
    } else {
        hit as f64 / total as f64
    }
}

/// The five metrics of `scenarios`. A finding without a verdict is an error,
/// never "faithful".
pub fn metrics(scenarios: &[&ScenarioFacts]) -> Result<FiveMetrics, String> {
    let (mut found, mut sources, mut faithful, mut kept, mut hedges) = (0, 0, 0, 0, 0);
    let (mut resolved, mut leaked, mut findings) = (0, 0, 0);
    for s in scenarios {
        for (id, source_hedges) in &s.source {
            sources += 1;
            let Some(finding) = s.findings.iter().find(|f| &f.claim_id == id) else {
                continue;
            };
            found += 1;
            let statement = finding.statement.to_lowercase();
            hedges += source_hedges.len();
            kept += source_hedges
                .iter()
                .filter(|h| statement.contains(&h.to_lowercase()))
                .count();
        }
        for f in &s.findings {
            findings += 1;
            resolved += usize::from(f.resolves);
            leaked += usize::from(f.leaks);
            let verdict = s.verdicts.get(&f.claim_id).ok_or_else(|| {
                format!(
                    "{}: finding {} has no judge verdict",
                    s.scenario_id, f.claim_id
                )
            })?;
            faithful += usize::from(*verdict);
        }
    }
    Ok(FiveMetrics {
        recall: ratio(found, sources),
        faithfulness: ratio(faithful, findings),
        hedge_preservation: ratio(kept, hedges),
        traceability: ratio(resolved, findings),
        secret_leakage: if findings == 0 {
            0.0
        } else {
            leaked as f64 / findings as f64
        },
    })
}

/// The floors `m` misses, each named with its value (empty = pass).
pub fn missed_floors(m: &FiveMetrics) -> Vec<String> {
    let checks = [
        ("recall", m.recall, RECALL_FLOOR, m.recall >= RECALL_FLOOR),
        (
            "faithfulness",
            m.faithfulness,
            FAITHFULNESS_FLOOR,
            m.faithfulness >= FAITHFULNESS_FLOOR,
        ),
        (
            "hedge_preservation",
            m.hedge_preservation,
            HEDGE_PRESERVATION_FLOOR,
            m.hedge_preservation >= HEDGE_PRESERVATION_FLOOR,
        ),
        (
            "traceability",
            m.traceability,
            TRACEABILITY_FLOOR,
            m.traceability >= TRACEABILITY_FLOOR,
        ),
        (
            "secret_leakage",
            m.secret_leakage,
            SECRET_LEAKAGE_CEILING,
            m.secret_leakage <= SECRET_LEAKAGE_CEILING,
        ),
    ];
    checks
        .into_iter()
        .filter(|(.., held)| !held)
        .map(|(name, value, floor, _)| format!("{name} {value:.3} misses its floor {floor}"))
        .collect()
}

/// Score the clean scenarios against the floors and every control against
/// the metric it plants.
pub fn score(scenarios: &[ScenarioFacts]) -> Result<DistillScore, String> {
    let clean: Vec<&ScenarioFacts> = scenarios.iter().filter(|s| s.control.is_none()).collect();
    if clean.is_empty() {
        return Err("the corpus has no clean scenario".into());
    }
    let mut controls = Vec::new();
    for s in scenarios {
        let Some(control) = s.control else { continue };
        let m = metrics(&[s])?;
        let detected = match control {
            Control::HedgeDrop => m.hedge_preservation < HEDGE_PRESERVATION_FLOOR,
            Control::FabricatedCite => m.traceability < TRACEABILITY_FLOOR,
            Control::PlantedSecret => m.secret_leakage > SECRET_LEAKAGE_CEILING,
            Control::DroppedClaim => m.recall < RECALL_FLOOR,
        };
        controls.push(ControlOutcome {
            scenario_id: s.scenario_id.clone(),
            control,
            detected,
        });
    }
    Ok(DistillScore {
        metrics: metrics(&clean)?,
        clean_scenarios: clean.len(),
        controls,
    })
}

/// Everything that fails the gate: a missed floor or an undetected control.
pub fn gate_failures(score: &DistillScore) -> Vec<String> {
    let mut failures = missed_floors(&score.metrics);
    failures.extend(score.controls.iter().filter(|c| !c.detected).map(|c| {
        format!(
            "negative control {} ({:?}) not detected",
            c.scenario_id, c.control
        )
    }));
    failures
}
