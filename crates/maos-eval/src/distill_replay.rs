#![forbid(unsafe_code)]

//! Story 18-3 (D-18-3-06 A, D-18-3-07 A) — the kernel-free half of
//! `maos eval distill --class researcher`: NFR-Aud-7's five floors (the one
//! source — `prd/non-functional-requirements.md:62-67`), the frozen corpus row,
//! and the scorer. The replay composition (kernel world, Researcher, the
//! `DistillateWriter`, redaction) is `maos-bench`'s (D-18-2-01 B).
//!
//! Four metrics are deterministic string facts over what the Researcher
//! emits; faithfulness comes from frozen judge verdicts committed with the
//! corpus (`provenance: seed` until the nightly judge records them) or, for a
//! `live-record` cassette, from its recorded judge completions. Clean
//! scenarios are held to the floors; each negative control must MISS the
//! metric it plants, or the gate is blind — including the faithfulness
//! control the control rows' frozen `false` verdicts plant.

use std::collections::{BTreeMap, BTreeSet};

use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::ports::inference::{InferenceOptions, InferenceRequest, InferenceResponse};

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
/// The pinned-judge contract; `[judge.researcher-faithfulness]` is the
/// nightly record's judge.
pub const JUDGE_CONFIG: &str = "tests/judge-config.toml";
/// The hosted bound on judge repeats (`journey-nightly.yml` `repeats`, the
/// `--judge-repeat` CLI bound and a judge-evidence record's `k`).
pub const MAX_JUDGE_REPEAT: usize = 10;
/// The fewest repeats that can say anything about judge AGREEMENT. One repeat
/// is a legal judge-only pass (one verdict per finding, the first-verdict
/// gate), but every finding trivially agrees with itself, so it establishes no
/// agreement and cannot back a floor record's `[judge]` evidence.
pub const MIN_AGREEMENT_REPEAT: usize = 2;

/// The live judge's prompt template (nightly record only); its SHA-256 is
/// `tests/judge-config.toml`'s `[judge.researcher-faithfulness]` pin.
pub const JUDGE_PROMPT: &str = "You are auditing a research digest. Answer with exactly one \
word: `faithful` if the digest finding states nothing the source claim does not support and \
contradicts nothing in it, otherwise `unfaithful`.";

/// One judge prompt: the template, the source claim, the finding.
pub fn judge_prompt(source: &str, finding: &str) -> String {
    format!("{JUDGE_PROMPT}\n\nSource claim: {source}\nDigest finding: {finding}")
}

/// The pinned judge's request for one finding: temperature 0, the wire
/// `model` pinned in `options.model_id` (R3). `top_p`/`seed` are not Inference
/// Port options.
pub fn judge_request(
    pid: u32,
    token: CapabilityToken,
    (source, finding): (&str, &str),
    provider: &str,
    model: &str,
) -> InferenceRequest {
    let options = InferenceOptions {
        max_tokens: 8,
        temperature: Some(0.0),
        model_id: Some(model.to_owned()),
    };
    let prompt = judge_prompt(source, finding);
    InferenceRequest::new(
        pid,
        token,
        prompt,
        options,
        Some(provider.into()),
        Vec::new(),
    )
}

/// A live judge completion's verdict: the one word, any non-letters around it
/// (`Faithful.`, `**faithful**`, `` `unfaithful` ``) stripped; anything else
/// is refused.
pub fn judge_verdict(text: &str) -> Result<bool, String> {
    let word = text.trim_matches(|c: char| !c.is_ascii_alphabetic());
    match word.to_ascii_lowercase().as_str() {
        "faithful" => Ok(true),
        "unfaithful" => Ok(false),
        _ => Err(format!(
            "judge answered `{}`, not faithful/unfaithful",
            text.trim()
        )),
    }
}

/// The `[judge.<name>]` rows of [`JUDGE_CONFIG`] this scorer reads.
#[derive(Debug, Deserialize)]
pub struct JudgeConfig {
    pub judge: BTreeMap<String, JudgeRow>,
}

/// One pinned judge: `model = "<provider>:<model>@<version>"` and the SHA-256
/// of its prompt.
#[derive(Debug, Deserialize)]
pub struct JudgeRow {
    pub model: String,
    pub prompt_version_hash: String,
}

impl JudgeConfig {
    /// `[judge.researcher-faithfulness]`'s wire model id — the one documented
    /// derivation: the part of `<provider>:<model>@<version>` between `:` and
    /// `@` (`anthropic:claude-sonnet-5-5@2026-10-25` → `claude-sonnet-5-5`).
    pub fn wire_model(&self) -> Result<String, String> {
        let pin = &self
            .judge
            .get("researcher-faithfulness")
            .ok_or("no [judge.researcher-faithfulness] row")?
            .model;
        pin.split_once(':')
            .and_then(|(_, rest)| rest.split_once('@'))
            .map(|(model, _)| model.to_owned())
            .filter(|model| !model.is_empty())
            .ok_or_else(|| format!("judge model {pin} is not <provider>:<model>@<version>"))
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
    /// A finding the judge rules unfaithful → faithfulness. Scored over every
    /// control row carrying a `false` verdict (the fifth control).
    UnfaithfulFinding,
}

impl Control {
    /// Planted in the committed completions themselves: a live model plants
    /// no hedge drop or secret, so record and a `live-record` replay serve
    /// these rows from the committed cassette.
    pub fn completion_planted(self) -> bool {
        matches!(self, Control::HedgeDrop | Control::PlantedSecret)
    }
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

/// The corpus rows. A blank hedge is refused (it would match every
/// statement).
pub fn load_rows<C: serde::de::DeserializeOwned>(text: &str) -> Result<Vec<DistillRow<C>>, String> {
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row: serde_json::Value =
                serde_json::from_str(line).map_err(|e| format!("corpus row: {e}"))?;
            let hedges = row["claims"].as_array().into_iter().flatten();
            if hedges
                .flat_map(|claim| claim["hedges"].as_array().into_iter().flatten())
                .any(|hedge| hedge.as_str().is_none_or(|h| h.trim().is_empty()))
            {
                return Err(format!("corpus row {}: an empty hedge", row["scenario_id"]));
            }
            serde_json::from_value(row).map_err(|e| format!("corpus row: {e}"))
        })
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

/// Every row whose frozen verdicts were judged under another prompt than
/// [`JUDGE_PROMPT`] (AA-2: a replay fails, never scores them).
pub fn stale_judge_prompts<C>(rows: &[DistillRow<C>]) -> Vec<String> {
    let prompt = crate::onboarding_gate_corpus::sha256_hex(JUDGE_PROMPT.as_bytes());
    rows.iter()
        .filter(|row| row.judge.prompt_sha256 != prompt)
        .map(|row| {
            format!(
                "{}: frozen verdicts pin a different judge prompt",
                row.scenario_id
            )
        })
        .collect()
}

/// Served by the run's own completions: every row but a
/// [`Control::completion_planted`] one.
pub fn served_live<C>(row: &DistillRow<C>) -> bool {
    !row.control.is_some_and(Control::completion_planted)
}

/// Whether a cassette is `live-record` rather than `seed`. `seed`: every
/// row's survey completions in corpus order (faithfulness from the frozen
/// verdicts). `live-record`: the [`served_live`] rows' survey completions,
/// then one judge completion per finding of those rows, in corpus order.
pub fn live_layout(cassette: &serde_json::Value) -> Result<bool, String> {
    match cassette["provenance"].as_str() {
        Some("seed") => Ok(false),
        Some("live-record") => Ok(true),
        other => Err(format!(
            "cassette provenance {other:?} is neither seed nor live-record"
        )),
    }
}

/// Read a cassette: its bytes and its JSON.
pub fn read_cassette(path: &std::path::Path) -> Result<(Vec<u8>, serde_json::Value), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cassette {path:?}: {e}"))?;
    let json = serde_json::from_slice(&bytes).map_err(|e| format!("cassette {path:?}: {e}"))?;
    Ok((bytes, json))
}

/// The provenances behind a run's verdicts: the frozen rows' and, for a live
/// run, `live-record`.
pub fn judge_provenance<C>(rows: &[DistillRow<C>], live: bool) -> BTreeSet<&str> {
    let frozen = rows.iter().filter(|row| !live || !served_live(*row));
    let mut provenance: BTreeSet<&str> = frozen.map(|row| row.judge.provenance.as_str()).collect();
    if live {
        provenance.insert("live-record");
    }
    provenance
}

/// A replay consumes every entry of its cassette: a trailing entry (an
/// unscored judge reply) fails the run.
pub fn fully_consumed(cassette: &serde_json::Value, consumed: usize) -> Result<(), String> {
    match cassette["entries"].as_array().map(Vec::len) {
        Some(entries) if entries == consumed => Ok(()),
        entries => Err(format!(
            "cassette has {entries:?} entries, the run consumed {consumed} (unconsumed entries)"
        )),
    }
}

/// One judge completion: (scenario_id, finding claim_id, the response).
#[derive(Debug, Clone)]
pub struct JudgeReply(pub String, pub String, pub InferenceResponse);

/// One judge reply's verdict: refused unless its provider attributes it to the
/// pinned `model` (R3) and it is one of the two words. Applied to EVERY reply
/// of a judge-only repeat, not only the first. The attribution must be what the
/// provider reports the SERVING model to be (the Anthropic adapter reads the
/// response's `model`); a reply with no attribution is unverified and is never
/// counted as pinned-judge evidence.
pub fn judge_reply_verdict(response: &InferenceResponse, model: &str) -> Result<bool, String> {
    match response.provider_attribution.model_id.as_deref() {
        Some(attributed) if attributed == model => judge_verdict(&response.text),
        Some(attributed) => Err(format!(
            "judge reply attributed to {:?}, not the pinned {model}",
            Some(attributed)
        )),
        None => Err(format!(
            "judge reply carries no model attribution (unverified), not the pinned {model}"
        )),
    }
}

/// Parse every reply into its scenario's verdicts. A reply its provider
/// attributes to any model but the pinned `model` is refused (R3), as is a
/// reply that is not one of the two words.
pub fn apply_judge(
    facts: &mut [ScenarioFacts],
    replies: Vec<JudgeReply>,
    model: &str,
) -> Result<(), String> {
    for JudgeReply(scenario_id, claim_id, response) in replies {
        let at = format!("{scenario_id}/{claim_id}");
        let verdict = judge_reply_verdict(&response, model).map_err(|e| format!("{at}: {e}"))?;
        facts
            .iter_mut()
            .find(|f| f.scenario_id == scenario_id)
            .ok_or_else(|| format!("{at}: no such scenario"))?
            .verdicts
            .insert(claim_id, verdict);
    }
    Ok(())
}

/// Where record writes the live verdicts: beside the cassette,
/// `<stem>.verdicts.json` (`distill-corpus-v0.live.verdicts.json`).
pub fn verdicts_sidecar(cassette: &std::path::Path) -> std::path::PathBuf {
    let stem = cassette.file_stem().unwrap_or_default().to_string_lossy();
    cassette.with_file_name(format!("{stem}.verdicts.json"))
}

/// The sidecar's bytes: the judge model, whether its pinned temperature was
/// applied on the wire, the prompt's SHA-256 and every [`served_live`]
/// scenario's verdicts (scenario → claim_id → faithful).
pub fn live_verdicts(
    facts: &[ScenarioFacts],
    model: &str,
    temperature_applied: bool,
) -> Result<String, String> {
    let verdicts: BTreeMap<&str, &BTreeMap<String, bool>> = facts
        .iter()
        .filter(|f| !f.control.is_some_and(Control::completion_planted))
        .map(|f| (f.scenario_id.as_str(), &f.verdicts))
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "judge_model": model,
        "judge_temperature_applied": temperature_applied,
        "judge_prompt_sha256": crate::onboarding_gate_corpus::sha256_hex(JUDGE_PROMPT.as_bytes()),
        "verdicts": verdicts,
    }))
    .map_err(|e| format!("verdicts sidecar: {e}"))
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
    /// Story 28-3 (D-28-3-04 C): survey completions that were not the one
    /// JSON object `{"finding": string}`. Counted from the cassette; the
    /// floor is 0 (see `gate_failures_with`).
    #[serde(default)]
    pub format_errors: usize,
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
            hedges += source_hedges.len();
            kept += source_hedges
                .iter()
                .filter(|h| keeps_hedge(&finding.statement, h))
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
        format_errors: 0,
    })
}

/// `statement` carries `hedge` as whole words, case-insensitively (EC-6):
/// `likely` never matches inside `unlikely`, `most` inside `almost`/`mostly`,
/// `could` inside `couldn't`.
pub fn keeps_hedge(statement: &str, hedge: &str) -> bool {
    let (statement, hedge) = (statement.to_lowercase(), hedge.to_lowercase());
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    statement.match_indices(&hedge).any(|(at, _)| {
        !word(statement[..at].chars().next_back())
            && !word(statement[at + hedge.len()..].chars().next())
    })
}

/// The floors `m` misses, each named with its value (empty = pass).
pub fn missed_floors(m: &FiveMetrics) -> Vec<String> {
    missed_floors_with(m, &DistillFloors::spec())
}

/// Story 28-3 (D-28-3-04 C) — a survey completion that is not the one JSON
/// object `{"finding": string}` (no code fence, no missing, extra or duplicate
/// field, `finding` a string): the SAME strict decoder the Researcher uses
/// (`maos_domain::distillation::parse_finding_completion`). It checks shape
/// only; a schema-valid string may still hold several sentences or meta-text,
/// which only the judge rubric (support/contradiction) can catch. Counted from
/// the replayed/recorded cassette; the floor is 0.
pub fn is_finding_completion(text: &str) -> bool {
    maos_domain::distillation::parse_finding_completion(text).is_some()
}

/// The distill floors of one run: the four NFR-Aud-7 lower-bound metrics.
/// `secret_leakage` is a zero-tolerance CEILING, not a floor: it stays the
/// compiled `SECRET_LEAKAGE_CEILING` and only an operator spec change can
/// move it (a Wilson bound has no meaning against a 0.0 ceiling).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistillFloors {
    pub recall: f64,
    pub faithfulness: f64,
    pub hedge_preservation: f64,
    pub traceability: f64,
}

impl Default for DistillFloors {
    fn default() -> Self {
        Self::spec()
    }
}

impl DistillFloors {
    /// The NFR-Aud-7 spec minimums (`prd/non-functional-requirements.md`).
    pub fn spec() -> Self {
        Self {
            recall: RECALL_FLOOR,
            faithfulness: FAITHFULNESS_FLOOR,
            hedge_preservation: HEDGE_PRESERVATION_FLOOR,
            traceability: TRACEABILITY_FLOOR,
        }
    }
}

/// Story 28-3 (AC1) — the pinned judge a Researcher floor record rests on and
/// its raw repeat agreement counts, copied from the archived judge-only run.
/// The counts are disclosure only: they never set a floor minimum and never
/// gate a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgeEvidence {
    /// The pinned judge's wire model id (`[judge.researcher-faithfulness]`).
    pub model: String,
    /// SHA-256 of [`JUDGE_PROMPT`] the repeats were judged under.
    pub prompt_sha256: String,
    /// Judge repeats per finding.
    pub k: usize,
    /// Findings judged.
    pub findings: usize,
    /// Judge calls made: `k * findings`.
    pub calls: usize,
    /// Repeats agreeing with their finding's first verdict, summed (every
    /// finding's first verdict agrees with itself).
    pub agree: usize,
    /// The archived judge-only result this evidence was read from.
    pub result_file: String,
    /// SHA-256 of that archived result file's bytes (the loader recomputes it
    /// when the file is present; `--require-archive` makes it mandatory).
    /// `xtask eval-report --calibrate --judge-result` emits it, nobody types it.
    pub result_sha256: String,
    pub rule: String,
    pub rationale: String,
}

impl JudgeEvidence {
    /// Refuse evidence that is not the pinned judge's, whose counts do not
    /// cohere, or that points at no archived result.
    fn check(&self, pinned_model: &str) -> Result<(), String> {
        if self.model.trim().is_empty() || self.model != pinned_model {
            return Err(format!(
                "judge model '{}' is not the pinned {pinned_model}",
                self.model
            ));
        }
        let prompt = crate::onboarding_gate_corpus::sha256_hex(JUDGE_PROMPT.as_bytes());
        if self.prompt_sha256 != prompt {
            return Err("judge prompt_sha256 is not the pinned judge prompt's".into());
        }
        if self.findings == 0 {
            return Err("judge evidence needs at least one finding".into());
        }
        if self.k < MIN_AGREEMENT_REPEAT {
            return Err(format!(
                "judge evidence k {} is below {MIN_AGREEMENT_REPEAT}: one repeat establishes no agreement",
                self.k
            ));
        }
        if self.k > MAX_JUDGE_REPEAT {
            return Err(format!(
                "judge evidence k {} is above the hosted bound {MAX_JUDGE_REPEAT}",
                self.k
            ));
        }
        if self.k.checked_mul(self.findings) != Some(self.calls) {
            return Err("judge calls must equal k * findings".into());
        }
        if self.agree > self.calls || self.agree < self.findings {
            return Err("judge agree must be in [findings, calls]".into());
        }
        let path = std::path::Path::new(&self.result_file);
        let traceable = !self.result_file.trim().is_empty()
            && path.is_relative()
            && path
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)));
        if !traceable {
            return Err("judge result_file must be a relative archived path".into());
        }
        if !is_sha256_hex(&self.result_sha256) {
            return Err("judge result_sha256 must be 64 lowercase hex characters".into());
        }
        if self.rule.trim().is_empty() || self.rationale.trim().is_empty() {
            return Err("judge evidence needs a rule and a rationale".into());
        }
        Ok(())
    }
}

/// Story 28-3 (AC1, AC6) — the Researcher's calibrated-floor record,
/// `spirits/researcher/eval/floors.toml`, same floor rule as Butler's
/// (28-1 D-28-1-04 A): floor = the larger of the spec minimum, the compiled
/// floor and the recorded Wilson lower bound rounded down to 0.01; a floor
/// below that requires a non-blank `operator_ruling`. Unlike Butler's, the
/// record also carries its required [`JudgeEvidence`] (judge model, prompt and
/// repeat agreement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistillFloorRecord {
    pub schema: String,
    pub judge: JudgeEvidence,
    pub floor: Vec<crate::halt_replay::FloorEntry>,
}

impl DistillFloorRecord {
    /// `base` (the spec minimums) with this record's floors applied. Refused:
    /// a wrong schema, judge evidence that is not `pinned_judge_model`'s or is
    /// incoherent, an unknown or repeated metric, evidence that is not a
    /// finite ordered interval in [0, 1], and, without a non-blank
    /// `operator_ruling`, a floor below the one its own evidence requires.
    pub fn apply(
        &self,
        base: DistillFloors,
        pinned_judge_model: &str,
    ) -> Result<DistillFloors, String> {
        if self.schema != crate::halt_replay::FLOORS_SCHEMA {
            return Err(format!("floor record schema is '{}'", self.schema));
        }
        self.judge.check(pinned_judge_model)?;
        let mut floors = base;
        let mut seen = Vec::new();
        for e in &self.floor {
            let (slot, compiled) = match e.metric.as_str() {
                "recall" => (&mut floors.recall, base.recall),
                "faithfulness" => (&mut floors.faithfulness, base.faithfulness),
                "hedge_preservation" => (&mut floors.hedge_preservation, base.hedge_preservation),
                "traceability" => (&mut floors.traceability, base.traceability),
                other => return Err(format!("floor record metric '{other}' is not a floor")),
            };
            if seen.contains(&e.metric) {
                return Err(format!("floor record metric '{}' repeats", e.metric));
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
            let required = compiled
                .max(e.spec_minimum)
                .max(crate::halt_replay::round_down_hundredth(lo));
            // NFR-Aud-7 digest-traceability is 100 %: no free-text ruling
            // lowers it (and leakage is not a calibrated floor at all).
            if e.metric == "traceability" && e.floor < compiled {
                return Err(format!(
                    "floor record 'traceability': floor {} is below the NFR-Aud-7 {compiled}; \
                     no operator ruling lowers traceability",
                    e.floor
                ));
            }
            if e.floor < required {
                if !ruled {
                    return Err(format!(
                        "floor record '{}': floor {} is below the one its evidence requires \
                         ({required}); an operator ruling is needed to go lower",
                        e.metric, e.floor
                    ));
                }
                let ruling = e.operator_ruling.as_deref().unwrap_or_default().trim();
                if !ruling_shape_ok(ruling) {
                    return Err(format!(
                        "floor record '{}': operator_ruling `{ruling}` is not a decision id \
                         (`D-28-3-NN[ X]` or `RULING-YYYY-MM-DD-<slug>`)",
                        e.metric
                    ));
                }
                if !e.ruling_sha256.as_deref().is_some_and(is_sha256_hex) {
                    return Err(format!(
                        "floor record '{}': a ruling needs `ruling_sha256`, the SHA-256 of the \
                         decision text it cites",
                        e.metric
                    ));
                }
            }
            *slot = e.floor;
        }
        Ok(floors)
    }
}

/// 64 lowercase hex characters.
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The SHAPE of a decision id a below-spec floor may cite: `D-28-3-NN` with an
/// optional ` <A-Z>` option letter, or `RULING-YYYY-MM-DD-<3..64 [A-Za-z0-9-]>`.
/// The loader cannot verify that the cited row EXISTS (the decision files live
/// outside this repository): the record's `ruling_sha256` binds the cited text,
/// and an operator still has to trust that the cited row says what it is used
/// for.
pub fn ruling_shape_ok(ruling: &str) -> bool {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    if let Some(rest) = ruling.strip_prefix("D-28-3-") {
        let (num, option) = rest.split_at(rest.len().min(2));
        return digits(num, 2)
            && (option.is_empty()
                || option
                    .strip_prefix(' ')
                    .is_some_and(|o| o.len() == 1 && o.bytes().all(|b| b.is_ascii_uppercase())));
    }
    if let Some(rest) = ruling.strip_prefix("RULING-") {
        let parts: Vec<&str> = rest.splitn(4, '-').collect();
        return parts.len() == 4
            && digits(parts[0], 4)
            && digits(parts[1], 2)
            && digits(parts[2], 2)
            && (3..=64).contains(&parts[3].len())
            && parts[3]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    }
    false
}

/// The largest archived artifact the loaders read.
pub const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

/// The counts a judge-only result line proves: read from its per-finding
/// verdicts, never from a typed number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeRepeatCounts {
    pub k: usize,
    pub findings: usize,
    pub calls: usize,
    /// Repeats equal to their finding's first verdict, summed.
    pub agree: usize,
    /// `judge_prompt_sha256` of the line.
    pub prompt_sha256: String,
}

/// Read and cross-check a judge-only (`--judge-repeat`) result line: the
/// repeat count in the hosted bounds, every finding judged exactly `k` times,
/// `judge_calls == k * findings`, agreement recomputed from the verdicts.
pub fn judge_repeat_counts(line: &serde_json::Value) -> Result<JudgeRepeatCounts, String> {
    if line["event"] != "eval_distill" {
        return Err("not an eval_distill result line".into());
    }
    let repeat = line
        .get("judge_repeat")
        .filter(|v| v.is_object())
        .ok_or("the line is not a judge-only repeat (no `judge_repeat`)")?;
    let k = repeat["k"]
        .as_u64()
        .ok_or("judge_repeat.k is not a count")? as usize;
    if !(MIN_AGREEMENT_REPEAT..=MAX_JUDGE_REPEAT).contains(&k) {
        return Err(format!(
            "judge_repeat.k {k} is outside {MIN_AGREEMENT_REPEAT}..={MAX_JUDGE_REPEAT}"
        ));
    }
    let findings = repeat["findings"]
        .as_object()
        .filter(|f| !f.is_empty())
        .ok_or("judge_repeat.findings is empty")?;
    let mut agree = 0usize;
    for (claim, f) in findings {
        let verdicts = f["verdicts"]
            .as_array()
            .filter(|v| v.len() == k)
            .ok_or_else(|| format!("finding {claim} was not judged exactly {k} times"))?;
        let first = verdicts[0]
            .as_bool()
            .ok_or_else(|| format!("finding {claim}: a verdict is not a boolean"))?;
        for v in verdicts {
            agree += usize::from(v.as_bool() == Some(first));
        }
    }
    let calls = k * findings.len();
    if repeat["judge_calls"].as_u64() != Some(calls as u64) {
        return Err("judge_repeat.judge_calls is not k * findings".into());
    }
    Ok(JudgeRepeatCounts {
        k,
        findings: findings.len(),
        calls,
        agree,
        prompt_sha256: line["judge_prompt_sha256"]
            .as_str()
            .ok_or("no judge_prompt_sha256")?
            .to_owned(),
    })
}

impl DistillFloorRecord {
    /// Recompute the judge evidence from the archived result it names, under
    /// `root`: the file's SHA-256 must be `result_sha256` and the counts and
    /// judge prompt must be the ones its line proves. An ABSENT file passes
    /// unless `require` (the promoting command's `--require-archive`): the loader
    /// of an ordinary run has no archive to check and says so by passing.
    pub fn verify_archive(&self, root: &std::path::Path, require: bool) -> Result<(), String> {
        let judge = &self.judge;
        let path = root.join(&judge.result_file);
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !require => return Ok(()),
            Err(e) => {
                return Err(format!(
                    "judge result_file {}: {e}{}",
                    path.display(),
                    if require { " (--require-archive)" } else { "" }
                ))
            }
        };
        if !meta.is_file() || meta.len() > MAX_ARTIFACT_BYTES {
            return Err(format!(
                "judge result_file {} is not a regular file within {MAX_ARTIFACT_BYTES} bytes",
                path.display()
            ));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if crate::onboarding_gate_corpus::sha256_hex(&bytes) != judge.result_sha256 {
            return Err(format!(
                "judge result_file {} is not the archive its record names (result_sha256)",
                path.display()
            ));
        }
        let line: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("judge result_file {}: {e}", path.display()))?;
        let counts = judge_repeat_counts(&line)
            .map_err(|e| format!("judge result_file {}: {e}", path.display()))?;
        let recorded = (judge.k, judge.findings, judge.calls, judge.agree);
        if recorded != (counts.k, counts.findings, counts.calls, counts.agree)
            || counts.prompt_sha256 != judge.prompt_sha256
        {
            return Err(format!(
                "judge evidence (k, findings, calls, agree) = {recorded:?} is not what its \
                 archive proves ({}, {}, {}, {})",
                counts.k, counts.findings, counts.calls, counts.agree
            ));
        }
        Ok(())
    }
}

/// The Researcher's calibrated-floor record, relative to the workspace root.
pub const FLOORS_FILE: &str = "spirits/researcher/eval/floors.toml";

/// The floors a Researcher run is held to: the compiled NFR-Aud-7 minimums when
/// there is no record, else the record applied to them (schema, evidence,
/// ruling shape and hash, pinned judge) and its judge archive verified under
/// `root` — required to be present when `require_archive` (the promoting
/// command's `--require-archive`; an ordinary run passes without it). The
/// caller reads and parses the TOML (this crate carries no TOML parser).
/// `eval-report` resolves the floors through this, so a result line cannot
/// name a floor of its own.
pub fn resolve_floors(
    record: Option<DistillFloorRecord>,
    root: &std::path::Path,
    pinned_judge_model: &str,
    require_archive: bool,
) -> Result<DistillFloors, String> {
    match record {
        None if require_archive => {
            Err("--require-archive: there is no floor record to promote".into())
        }
        None => Ok(DistillFloors::spec()),
        Some(record) => {
            let floors = record.apply(DistillFloors::spec(), pinned_judge_model)?;
            record.verify_archive(root, require_archive)?;
            Ok(floors)
        }
    }
}

/// `missed_floors` against `floors` instead of the compiled spec minimums. The
/// zero-tolerance `secret_leakage` ceiling is never calibrated: it is the
/// compiled [`SECRET_LEAKAGE_CEILING`] whatever record is applied.
pub fn missed_floors_with(m: &FiveMetrics, floors: &DistillFloors) -> Vec<String> {
    let checks = [
        ("recall", m.recall, floors.recall, m.recall >= floors.recall),
        (
            "faithfulness",
            m.faithfulness,
            floors.faithfulness,
            m.faithfulness >= floors.faithfulness,
        ),
        (
            "hedge_preservation",
            m.hedge_preservation,
            floors.hedge_preservation,
            m.hedge_preservation >= floors.hedge_preservation,
        ),
        (
            "traceability",
            m.traceability,
            floors.traceability,
            m.traceability >= floors.traceability,
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
/// Score the clean scenarios against the floors, every control row against
/// the metric it plants, and the control rows' `false` verdicts against the
/// faithfulness floor (the fifth control). Control rows never enter a floor.
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
            Control::UnfaithfulFinding => m.faithfulness < FAITHFULNESS_FLOOR,
        };
        controls.push(ControlOutcome {
            scenario_id: s.scenario_id.clone(),
            control,
            detected,
        });
    }
    let judged: Vec<&ScenarioFacts> = scenarios
        .iter()
        .filter(|s| s.control.is_some() && s.verdicts.values().any(|v| !v))
        .collect();
    controls.push(ControlOutcome {
        scenario_id: judged
            .iter()
            .map(|s| s.scenario_id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        control: Control::UnfaithfulFinding,
        detected: !judged.is_empty() && metrics(&judged)?.faithfulness < FAITHFULNESS_FLOOR,
    });
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

/// `gate_failures` against `floors`; the format-error floor is absolute 0
/// (D-28-3-04 C: a malformed completion is a format error, never "faithful").
pub fn gate_failures_with(score: &DistillScore, floors: &DistillFloors) -> Vec<String> {
    let mut failures = missed_floors_with(&score.metrics, floors);
    if score.metrics.format_errors > 0 {
        failures.push(format!(
            "format_errors {} above floor 0",
            score.metrics.format_errors
        ));
    }
    failures.extend(score.controls.iter().filter(|c| !c.detected).map(|c| {
        format!(
            "negative control {} ({:?}) not detected",
            c.scenario_id, c.control
        )
    }));
    failures
}

/// Story 28-3 review (finding 4) — what a run decided about one finding of a
/// clean scenario: the unit `xtask eval-report` takes the per-claim majority
/// over k repeats of, so the Wilson interval of the agreed unit (claim for
/// recall/faithfulness/traceability/leakage, hedge for preservation) exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingDecision {
    pub faithful: bool,
    pub resolves: bool,
    pub leaks: bool,
    /// One flag per source hedge, in source order: the finding keeps it.
    pub hedges: Vec<bool>,
}

/// One clean-scenario source claim's decisions; `finding` is `None` when the
/// survey carried no finding for it (a recall miss).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimDecision {
    pub scenario_id: String,
    pub claim_id: String,
    pub finding: Option<FindingDecision>,
}

/// The decisions behind [`metrics`] over the clean scenarios, corpus order.
/// A finding without a judge verdict is an error, as in [`metrics`].
pub fn claim_decisions(scenarios: &[ScenarioFacts]) -> Result<Vec<ClaimDecision>, String> {
    let mut decisions = Vec::new();
    for s in scenarios.iter().filter(|s| s.control.is_none()) {
        for (id, source_hedges) in &s.source {
            let finding = match s.findings.iter().find(|f| &f.claim_id == id) {
                None => None,
                Some(f) => Some(FindingDecision {
                    faithful: *s.verdicts.get(&f.claim_id).ok_or_else(|| {
                        format!(
                            "{}: finding {} has no judge verdict",
                            s.scenario_id, f.claim_id
                        )
                    })?,
                    resolves: f.resolves,
                    leaks: f.leaks,
                    hedges: source_hedges
                        .iter()
                        .map(|h| keeps_hedge(&f.statement, h))
                        .collect(),
                }),
            };
            decisions.push(ClaimDecision {
                scenario_id: s.scenario_id.clone(),
                claim_id: id.clone(),
                finding,
            });
        }
    }
    Ok(decisions)
}
