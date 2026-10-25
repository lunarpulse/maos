#![forbid(unsafe_code)]

//! Story 18-3 (D-18-3-06/07 A) — the replay composition behind
//! `maos eval distill --class researcher`.
//!
//! Per corpus row: a fresh kernel world; the row's claims journaled at
//! deterministic frame ids (a fabricated cite is surveyed but never
//! journaled, a dropped claim is withheld); the Researcher bound to the
//! kernel `InferencePortAdapter` over the given router, minting its declared
//! provider; its distillate through the kernel `DistillateWriter` (I11); each
//! cite resolved through `LogRecallAdapter` and each statement through the
//! real `CorpusBackedRedactionPolicy`. The cassette's provenance decides its
//! layout (`scorer::live_layout`): a `seed` replay checks every survey
//! against the frozen one and reads the frozen judge verdicts; record
//! (nightly) and a `live-record` replay serve the completion-planted controls
//! from the committed cassette, the other rows from the run's router, then
//! the pinned judge per finding of those rows, through the same port.
//! Scoring is `maos_eval::distill_replay`'s.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::{InferencePort, LogRecallPort};
use maos_eval::distill_replay::{self as scorer, DistillRow, FindingFacts, ScenarioFacts};
use maos_eval::halt_replay::check_consumed;
use maos_eval::onboarding_gate_corpus::sha256_hex;
use maos_kernel_core::capability::cap_tokens::init_monotonic_base;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_kernel_core::iac::transparency_log::FrameKind;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::security::capabilities_required_to_scopes;
use maos_kernel_core::security::manifest::CapabilitiesRequired;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use researcher::{ClaimPayload, RecalledFrame, Researcher, ResearcherInferenceBinding};

use crate::halt_eval::{required, Completions, Counted, World, EVAL_PID};

/// The committed seed cassette `evaluate`'s `committed` driver replays.
pub use maos_eval::distill_replay::CASSETTE as COMMITTED_CASSETTE;

const USAGE: &str = "usage: maos eval distill --class researcher [--cassette <path>]";
const INTENT: &str = "researcher:corpus_claim";

/// A parsed `maos eval distill` invocation.
#[derive(Debug)]
pub struct DistillRequest {
    cassette: Option<PathBuf>,
    /// `MAOS_INFERENCE_MODE=record` (D-18-3-07 A).
    pub record: bool,
}

/// Parse `distill --class researcher [--cassette <p>]`; a replay without
/// `--cassette` reads `env_cassette`. A usage error is the caller's exit 2.
pub fn parse_args(
    argv: &[String],
    mode: Option<&str>,
    env_cassette: Option<PathBuf>,
) -> Result<DistillRequest, String> {
    let mut args = argv.iter().map(String::as_str);
    if args.next() != Some("distill") {
        return Err(USAGE.into());
    }
    let (mut class, mut cassette) = (None, None);
    while let Some(flag) = args.next() {
        match (flag, args.next()) {
            ("--class", Some(value)) => class = Some(value),
            ("--cassette", Some(value)) => cassette = Some(PathBuf::from(value)),
            _ => return Err(USAGE.into()),
        }
    }
    if class != Some("researcher") {
        return Err(format!(
            "the distillation-shipping class is researcher — {USAGE}"
        ));
    }
    let record = match mode {
        None | Some("replay") => false,
        Some("record") => true,
        Some(other) => return Err(format!("MAOS_INFERENCE_MODE={other} is not an eval mode")),
    };
    if record && cassette.is_some() {
        return Err("record writes MAOS_REPLAY_CASSETTE; --cassette is replay-only".into());
    }
    let cassette = cassette.or(env_cassette.filter(|_| !record));
    Ok(DistillRequest { cassette, record })
}

impl DistillRequest {
    /// The cassette a replay reads: `--cassette`, else `MAOS_REPLAY_CASSETTE`,
    /// else the committed one.
    pub fn replay_cassette(&self) -> Option<PathBuf> {
        let committed = || PathBuf::from(scorer::CASSETTE);
        (!self.record).then(|| self.cassette.clone().unwrap_or_else(committed))
    }
}

/// The stdout line and every gate failure (empty = exit 0).
#[derive(Debug)]
pub struct DistillRun {
    pub line: String,
    pub failures: Vec<String>,
}

/// Replay (or record) the frozen corpus and score it. `committed` is the
/// strict replay driver over the committed `scorer::CASSETTE`. `Err` is a run
/// with no numbers: a refused pass, drift, exhaustion, consumed ≠ calls, an
/// unconsumed entry, a judge reply not the pinned model's.
pub fn evaluate(
    completions: Completions,
    committed: Arc<dyn maos_providers::Provider>,
) -> Result<DistillRun, String> {
    init_monotonic_base();
    let text = std::fs::read_to_string(scorer::CORPUS)
        .map_err(|e| format!("corpus {}: {e}", scorer::CORPUS))?;
    let rows: Vec<DistillRow<ClaimPayload>> = scorer::load_rows(&text)?;
    let model = std::fs::read_to_string(scorer::JUDGE_CONFIG)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str::<scorer::JudgeConfig>(&t).map_err(|e| e.to_string()))
        .and_then(|config| config.wire_model())
        .map_err(|e| format!("{}: {e}", scorer::JUDGE_CONFIG))?;
    let manifest: toml::Value =
        toml::from_str(include_str!("../../../spirits/researcher/manifest.toml"))
            .map_err(|e| format!("researcher manifest: {e}"))?;
    let declared = capabilities_required_to_scopes(
        &CapabilitiesRequired::from_toml_str(&required(&manifest)?)
            .map_err(|e| format!("researcher [capabilities.required]: {e}"))?,
    );
    let provider = declared
        .iter()
        .find_map(|scope| match scope {
            Scope::ProviderInfer { provider } => Some(provider.clone()),
            _ => None,
        })
        .ok_or("researcher declares no inference provider")?;
    let route = |driver: Arc<dyn maos_providers::Provider>| {
        let drivers = BTreeMap::from([(provider.clone(), driver)]);
        Arc::new(MultiProviderRouter::new(drivers, Some(provider.clone())))
    };
    let (router, cassette, flush) = match completions {
        Completions::Replay { driver, cassette } => (route(driver), cassette, None),
        Completions::Record {
            router,
            cassette,
            flush,
        } => (router, cassette, Some(flush)),
    };
    let record = flush.is_some();
    let replayed = (!record)
        .then(|| scorer::read_cassette(&cassette))
        .transpose()?;
    let live = replayed
        .as_ref()
        .map_or(Ok(true), |(_, json)| scorer::live_layout(json))?;
    let consumed = Arc::new(AtomicUsize::new(0));
    let port_over = |world: &World, router: &Arc<MultiProviderRouter>| Counted {
        inner: InferencePortAdapter::new(
            Arc::clone(router),
            Arc::clone(&world.capability),
            Arc::clone(&world.tl),
            Arc::new(IacRtMetrics::new()),
        ),
        consumed: Arc::clone(&consumed),
    };
    let redaction = CorpusBackedRedactionPolicy::new();
    // One row surveyed through `router`: its facts, its failures and the
    // completions it took. A `frozen` row must reproduce its frozen survey
    // and scores the frozen verdicts.
    let survey = |s: usize,
                  router: &Arc<MultiProviderRouter>,
                  frozen: bool|
     -> Result<(ScenarioFacts, Vec<String>, usize), String> {
        let row = &rows[s];
        let id = &row.scenario_id;
        let world = World::new(declared.clone())?;
        let mut frames = Vec::new();
        for (c, claim) in row.claims.iter().enumerate() {
            if row.withheld.contains(&claim.claim_id) {
                continue;
            }
            let payload = serde_json::to_vec(claim).map_err(|e| format!("{id}: {e}"))?;
            let frame_id = scorer::frame_id(s, c);
            if !row.unjournaled.contains(&claim.claim_id) {
                let _ = world.tl.insert_frame_event_with_id(
                    Some(frame_id),
                    FrameKind::TelemetryEvent,
                    EVAL_PID,
                    "",
                    "",
                    None,
                    INTENT,
                    &payload,
                    FrameOrigin::SpiritAuto,
                );
            }
            frames.push(RecalledFrame {
                frame_id,
                intent: INTENT.into(),
                payload,
            });
        }
        let mint = minter(&world.capability);
        let binding = ResearcherInferenceBinding::per_pass(
            provider.clone(),
            Arc::new(move |pid, provider| mint(pid, provider)),
            Arc::new(|| Some(EVAL_PID)),
        );
        let researcher = Researcher::new().with_deferred_inference_port(
            Arc::new(port_over(&world, router)),
            Arc::new(Mutex::new(Some(binding))),
        );
        let output = researcher
            .infer_survey(&frames)
            .map_err(|e| format!("{id}: pass refused: {e}"))?;
        let writer = DistillateWriter::new(Arc::clone(&world.tl), Arc::new(0u8));
        let written = researcher
            .distill_through(&writer, EVAL_PID, &output, 1)
            .is_ok();
        let recall = LogRecallAdapter::new(Arc::clone(&world.tl));
        let findings: Vec<FindingFacts> = output
            .findings
            .iter()
            .map(|f| FindingFacts {
                claim_id: f.claim_id.clone(),
                statement: f.statement.clone(),
                hedges: f.hedges.clone(),
                resolves: researcher::decode_frame_id_hex(&f.source_log_ref)
                    .is_ok_and(|frame| recall.fetch(EVAL_PID, frame).is_ok()),
                leaks: matches!(redaction.redact(f.statement.as_bytes()),
                    Cow::Owned(r) if r != f.statement.as_bytes()),
            })
            .collect();
        let mut failures = Vec::new();
        if written != findings.iter().all(|f| f.resolves) {
            failures.push(format!("{id}: the I11 writer and cite resolution disagree"));
        }
        let recomputed = serde_json::to_value(&output).map_err(|e| format!("{id}: {e}"))?;
        if frozen && recomputed != row.survey {
            failures.push(format!(
                "{id}: survey differs from the frozen one (re-pin needed): {recomputed}"
            ));
        }
        let fact = ScenarioFacts {
            scenario_id: id.clone(),
            control: row.control,
            source: row
                .claims
                .iter()
                .map(|c| (c.claim_id.clone(), c.hedges.clone()))
                .collect(),
            findings,
            verdicts: frozen
                .then(|| row.judge.verdicts.clone())
                .unwrap_or_default(),
        };
        Ok((fact, failures, frames.len()))
    };
    let committed_router = route(committed);
    let mut failures = scorer::stale_judge_prompts(&rows);
    let (mut facts, mut calls) = (BTreeMap::new(), 0);
    // Pass 1 — every row through the committed completions (a `seed`
    // cassette's own); a live run keeps only the completion-planted controls.
    for (s, row) in rows.iter().enumerate() {
        let (fact, failed, n) = survey(s, if live { &committed_router } else { &router }, true)?;
        calls += n;
        failures.extend(failed);
        if !live || !scorer::served_live(row) {
            facts.insert(s, fact);
        }
    }
    let committed_calls = calls;
    // Pass 2 (live) — the other rows through the run's router, then the
    // pinned judge per finding of those rows, one token minted per call.
    let mut replies = Vec::new();
    // Story 18-4 (BH-3, `deferred-work.md:1253`): the distinct tokens the
    // judge was asked with, against the judge calls made.
    let mut judge_tokens = std::collections::HashSet::new();
    let mut ask = || -> Result<(), String> {
        let live_rows = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| scorer::served_live(*row));
        for (s, _) in live_rows.clone() {
            let (fact, failed, n) = survey(s, &router, false)?;
            calls += n;
            failures.extend(failed);
            facts.insert(s, fact);
        }
        let world = World::new(declared.clone())?;
        let (judge, mint) = (port_over(&world, &router), minter(&world.capability));
        for (s, row) in live_rows {
            for f in &facts[&s].findings {
                let source = row.claims.iter().find(|c| c.claim_id == f.claim_id);
                let pair = (source.map_or("", |c| c.statement.as_str()), &*f.statement);
                let token = mint(EVAL_PID, &provider)?;
                // Story 18-4 (BH-3 witness): one token per judge call, counted.
                judge_tokens.insert(token.token_id.0);
                let req = scorer::judge_request(EVAL_PID, token, pair, &provider, &model);
                let response = judge.complete(req).map_err(|e| format!("judge: {e}"))?;
                let (scenario, claim) = (row.scenario_id.clone(), f.claim_id.clone());
                replies.push(scorer::JudgeReply(scenario, claim, response));
                calls += 1;
            }
        }
        Ok(())
    };
    let asked = if live { ask() } else { Ok(()) };
    // BH-4: what was recorded is flushed before any reply is parsed.
    asked.and(flush.map_or(Ok(()), |flush| flush()))?;
    let judge_calls = replies.len();
    let mut facts: Vec<ScenarioFacts> = facts.into_values().collect();
    scorer::apply_judge(&mut facts, replies, &model)?;
    check_consumed(consumed.load(Ordering::SeqCst), calls)?;
    if live {
        let (_, json) = scorer::read_cassette(Path::new(scorer::CASSETTE))?;
        scorer::fully_consumed(&json, committed_calls).map_err(|e| format!("committed {e}"))?;
    }
    if let Some((_, json)) = &replayed {
        let served = if live { calls - committed_calls } else { calls };
        scorer::fully_consumed(json, served).map_err(|e| format!("{cassette:?}: {e}"))?;
    }
    if record {
        let sidecar = scorer::verdicts_sidecar(&cassette);
        std::fs::write(&sidecar, scorer::live_verdicts(&facts, &model)?)
            .map_err(|e| format!("verdicts sidecar {sidecar:?}: {e}"))?;
    }
    let judge_provenance = scorer::judge_provenance(&rows, live);
    let score = scorer::score(&facts)?;
    failures.extend(scorer::gate_failures(&score));
    let (bytes, json) = match replayed {
        Some(read) => read,
        None => scorer::read_cassette(&cassette)?,
    };
    let line = serde_json::json!({
        "event": "eval_distill",
        "class": "researcher",
        "corpus": scorer::CORPUS,
        "corpus_sha256": sha256_hex(text.as_bytes()),
        "cassette": cassette.display().to_string(),
        "cassette_sha256": sha256_hex(&bytes),
        "provenance": json.get("provenance"),
        "judge_provenance": judge_provenance,
        "judge_prompt_sha256": sha256_hex(scorer::JUDGE_PROMPT.as_bytes()),
        "completions_consumed": calls,
        "judge_calls": judge_calls,
        "judge_tokens_minted": judge_tokens.len(),
        "score": score,
        "pass": failures.is_empty(),
        "missed_floors": &failures,
    });
    Ok(DistillRun {
        line: line.to_string(),
        failures,
    })
}

type Mint = Box<
    dyn Fn(u32, &str) -> Result<maos_domain::invariants::i1::CapabilityToken, String> + Send + Sync,
>;

/// The world's audited `ProviderInfer` issuance.
fn minter(capability: &Arc<CapabilityRegistryAdapter>) -> Mint {
    let capability = Arc::clone(capability);
    Box::new(move |pid, provider| {
        let scope = Scope::ProviderInfer {
            provider: provider.to_owned(),
        };
        capability
            .issue_with_mediation(pid, scope, 60, [0; 32], IntentClass::Standard)
            .map_err(|e| format!("token issue failed: {e}"))
    })
}
