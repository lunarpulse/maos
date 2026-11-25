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
use maos_eval::distill_replay::{DistillFloorRecord, DistillFloors};
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
use maos_kernel_core::security::manifest::{pinned_model, CapabilitiesRequired};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use researcher::{ClaimPayload, RecalledFrame, Researcher, ResearcherInferenceBinding};

use crate::halt_eval::{required, Completions, Counted, World, EVAL_PID};

/// The committed seed cassette `evaluate`'s `committed` driver replays.
pub use maos_eval::distill_replay::CASSETTE as COMMITTED_CASSETTE;

const USAGE: &str = "usage: maos eval distill --class researcher [--cassette <path>] \
                     [--judge-repeat <k>]";
const INTENT: &str = "researcher:corpus_claim";
/// The Researcher's calibrated-floor record, read when present.
const FLOORS_FILE: &str = scorer::FLOORS_FILE;
/// The hosted dispatch bound on repeats, shared with the floor record's judge
/// evidence: a judge-only repeat is `1..=MAX_JUDGE_REPEAT` live judge calls per
/// finding (see [`scorer::MIN_AGREEMENT_REPEAT`] for what one repeat cannot say).
pub const MAX_JUDGE_REPEAT: usize = scorer::MAX_JUDGE_REPEAT;

/// A parsed `maos eval distill` invocation.
#[derive(Debug)]
pub struct DistillRequest {
    pub cassette: Option<PathBuf>,
    /// `MAOS_INFERENCE_MODE=record` (D-18-3-07 A).
    pub record: bool,
    /// Story 28-3 (D-28-3-03 A): judge-only repeat count. `Some(k)` replays
    /// the recorded survey of `cassette` and asks the live judge k times per
    /// finding, reporting per-finding agreement and faithfulness mean/min.
    pub judge_repeat: Option<usize>,
}

/// Parse `distill --class researcher [--cassette <p>]`; a replay without
/// `--cassette` reads `env_cassette`. A usage error is the caller's exit 2.
pub fn parse_args(
    argv: &[String],
    mode: Option<&str>,
    env_cassette: Option<PathBuf>,
) -> Result<DistillRequest, String> {
    parse_args_with_repeat(argv, mode, env_cassette, None)
}

/// Parse with the Story 28-3 judge-only repeat flag (D-28-3-03 A):
/// `--judge-repeat <k>` replays ONE recorded survey (a `--cassette`
/// `live-record`) and asks the live judge k times per finding. It needs a
/// live router, so it is record-mode-only; it needs the recorded cassette,
/// so the record/`--cassette` refusal is lifted for it alone.
pub fn parse_args_with_repeat(
    argv: &[String],
    mode: Option<&str>,
    env_cassette: Option<PathBuf>,
    default_repeat: Option<usize>,
) -> Result<DistillRequest, String> {
    let mut args = argv.iter().map(String::as_str);
    if args.next() != Some("distill") {
        return Err(USAGE.into());
    }
    let (mut class, mut cassette, mut judge_repeat) = (None, None, default_repeat);
    while let Some(flag) = args.next() {
        match (flag, args.next()) {
            ("--class", Some(value)) => class = Some(value),
            ("--cassette", Some(value)) => cassette = Some(PathBuf::from(value)),
            ("--judge-repeat", Some(value)) => {
                judge_repeat = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("--judge-repeat wants a count 1-10: {value}"))?,
                );
            }
            _ => return Err(USAGE.into()),
        }
    }
    if class != Some("researcher") {
        return Err(format!(
            "the distillation-shipping class is researcher — {USAGE}"
        ));
    }
    if let Some(k) = judge_repeat {
        check_repeat_count(k)?;
    }
    if judge_repeat.is_some() && cassette.is_none() {
        return Err(format!(
            "--judge-repeat replays a recorded survey — pass the live-record --cassette ({USAGE})"
        ));
    }
    let record = match mode {
        None | Some("replay") => false,
        Some("record") => true,
        Some(other) => return Err(format!("MAOS_INFERENCE_MODE={other} is not an eval mode")),
    };
    if judge_repeat.is_some() && !record {
        return Err(format!(
            "--judge-repeat asks the LIVE judge — set MAOS_INFERENCE_MODE=record ({USAGE})"
        ));
    }
    if record && cassette.is_some() && judge_repeat.is_none() {
        return Err("record writes MAOS_REPLAY_CASSETTE; --cassette is replay-only".into());
    }
    if let (true, Some(output)) = (record, &env_cassette) {
        let survey = cassette.as_deref().filter(|_| judge_repeat.is_some());
        check_destinations(output, survey)?;
    }
    let cassette = cassette.or_else(|| env_cassette.filter(|_| !record || judge_repeat.is_some()));
    Ok(DistillRequest {
        cassette,
        record,
        judge_repeat,
    })
}

/// A judge-only repeat count is `1..=`[`MAX_JUDGE_REPEAT`]; 0 is refused, never
/// dropped into an ordinary paid survey-and-judge pass.
fn check_repeat_count(k: usize) -> Result<(), String> {
    if (1..=MAX_JUDGE_REPEAT).contains(&k) {
        Ok(())
    } else {
        Err(format!(
            "--judge-repeat wants a count 1-{MAX_JUDGE_REPEAT}, got {k}"
        ))
    }
}

/// `p` resolved as far as the file system lets it: made absolute, a final
/// symlink followed even when it dangles (its target need not exist yet),
/// `.`/`..` removed lexically, then the nearest existing ancestor canonicalised
/// (symlinks followed) and the missing tail appended.
fn resolved(p: &Path) -> PathBuf {
    let mut current = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    for _ in 0..40 {
        let is_link = std::fs::symlink_metadata(&current).is_ok_and(|m| m.file_type().is_symlink());
        let Some(target) = is_link.then(|| std::fs::read_link(&current).ok()).flatten() else {
            break;
        };
        current = match current.parent() {
            Some(dir) => dir.join(target),
            None => target,
        };
    }
    let mut clean = PathBuf::new();
    for part in current.components() {
        match part {
            std::path::Component::ParentDir => {
                clean.pop();
            }
            std::path::Component::CurDir => {}
            other => clean.push(other),
        }
    }
    let (mut tail, mut at) = (Vec::new(), clean.as_path());
    loop {
        if let Ok(mut found) = at.canonicalize() {
            found.extend(tail.iter().rev());
            return found;
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_owned());
                at = parent;
            }
            _ => return clean,
        }
    }
}

/// Destinations claimed BEFORE any inference, created with `create_new`
/// (`O_CREAT|O_EXCL`: an existing file, directory or symlink — even a dangling
/// one — fails, and a final symlink is never followed), so a name that was
/// free when checked cannot be a link by the time it is written. An
/// unwritten (still empty) reservation is removed when this guard drops, so a
/// refused run leaves nothing behind. The recorder and the sidecar replace the
/// reserved file by rename and never write through a link.
struct Reserved(Vec<PathBuf>);

impl Reserved {
    fn new(paths: &[&Path]) -> Result<Self, String> {
        let mut held = Self(Vec::new());
        for path in paths {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| {
                    format!(
                        "{} cannot be reserved ({e}): it exists (a file, a directory or a \
                         symlink, dangling or not) or is not creatable; record to a fresh path",
                        path.display()
                    )
                })?;
            held.0.push(path.to_path_buf());
        }
        Ok(held)
    }
}

impl Drop for Reserved {
    fn drop(&mut self) {
        for path in &self.0 {
            if std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() == 0) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

/// Write `bytes` over the reserved `path` by a fresh temp file and a rename:
/// the rename replaces whatever sits at `path` (a link included) instead of
/// following it.
fn write_replacing(path: &Path, bytes: &str) -> Result<(), String> {
    use std::io::Write;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("{}: {e}", temp.display()))?;
    file.write_all(bytes.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("{}: {e}", temp.display()))?;
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("{}: {e}", path.display())
    })
}

/// Whether two paths name one file: the same resolved path or, on unix, the
/// same device and inode (hard links).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(x), Ok(y)) = (std::fs::metadata(a), std::fs::metadata(b)) {
            if (x.dev(), x.ino()) == (y.dev(), y.ino()) {
                return true;
            }
        }
    }
    resolved(a) == resolved(b)
}

/// Every file a record run writes — the recorded cassette (judge replies, or
/// the recorder's survey) and its `.verdicts.json` sidecar — against every file
/// it reads (the survey cassette and its sidecar, the committed seed, the
/// corpus, the judge config, the floor record) and against each other. Equality
/// is by canonical path (symlinks, relative/absolute spellings) or device and
/// inode (hard links); a destination that does not exist yet is compared by its
/// canonical parent and name. Refused before any inference.
fn check_destinations(output: &Path, survey: Option<&Path>) -> Result<(), String> {
    let sidecar = scorer::verdicts_sidecar(output);
    let mut reads: Vec<(&str, PathBuf)> = vec![
        (
            "the committed seed cassette",
            PathBuf::from(scorer::CASSETTE),
        ),
        ("the corpus", PathBuf::from(scorer::CORPUS)),
        ("the judge config", PathBuf::from(scorer::JUDGE_CONFIG)),
        ("the floor record", PathBuf::from(FLOORS_FILE)),
    ];
    if let Some(survey) = survey {
        reads.push(("the survey cassette", survey.to_path_buf()));
        reads.push((
            "the survey's verdict sidecar",
            scorer::verdicts_sidecar(survey),
        ));
    }
    let writes = [
        ("the recorded cassette (MAOS_REPLAY_CASSETTE)", output),
        ("its verdict sidecar", sidecar.as_path()),
    ];
    for (write, path) in writes {
        for (read, other) in &reads {
            if same_file(path, other) {
                return Err(format!(
                    "{write} {} is the same file as {read} {}; the recorder would overwrite it",
                    path.display(),
                    other.display()
                ));
            }
        }
    }
    if same_file(output, &sidecar) {
        return Err(format!(
            "the recorded cassette and its verdict sidecar are the same file {}",
            output.display()
        ));
    }
    Ok(())
}

impl DistillRequest {
    /// The cassette a replay reads: `--cassette`, else `MAOS_REPLAY_CASSETTE`,
    /// else the committed one.
    pub fn replay_cassette(&self) -> Option<PathBuf> {
        let committed = || PathBuf::from(scorer::CASSETTE);
        (!self.record)
            .then(|| self.cassette.clone().unwrap_or_else(committed))
            .or_else(|| {
                self.judge_repeat
                    .map(|_| self.cassette.clone().unwrap_or_else(committed))
            })
    }
}

/// Story 28-3 (D-28-3-03 A) — one judge-only repeat run: `k` live judge
/// calls per finding over the survey of a recorded `live-record` cassette.
#[derive(Clone)]
pub struct JudgeRepeat {
    pub k: usize,
    /// The replay driver over the recorded `live-record` cassette: the
    /// survey replays through it (zero survey cost; only the judge is live).
    pub survey: Arc<dyn maos_providers::Provider>,
    /// The cassette's path, for provenance in the line.
    pub survey_cassette: std::path::PathBuf,
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
    evaluate_with_repeat(completions, committed, None)
}

/// `evaluate` with the judge-only repeat of D-28-3-03 A.
pub fn evaluate_with_repeat(
    completions: Completions,
    committed: Arc<dyn maos_providers::Provider>,
    judge_repeat: Option<JudgeRepeat>,
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
    let caps = CapabilitiesRequired::from_toml_str(&required(&manifest)?)
        .map_err(|e| format!("researcher [capabilities.required]: {e}"))?;
    let declared = capabilities_required_to_scopes(&caps);
    let provider = declared
        .iter()
        .find_map(|scope| match scope {
            Scope::ProviderInfer { provider } => Some(provider.clone()),
            _ => None,
        })
        .ok_or("researcher declares no inference provider")?;
    // S1 (Q6): the survey completions are served by the model the Researcher's
    // manifest pins for its provider, as in production.
    let pinned = pinned_model(&caps.provider.complete, &provider);
    // The pinned `temperature = 0.0` reaches the wire only if the judge's model
    // accepts one (the Anthropic driver omits it for the 5.x families); the
    // verdict sidecar and the report line say which, so the provenance is true.
    let temperature_applied =
        provider != "anthropic" || maos_providers::anthropic::accepts_sampling(&model);
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
    if let Some(repeat) = &judge_repeat {
        if !record {
            return Err("--judge-repeat needs the record mode's live router".into());
        }
        // Before any inference: the hosted bound.
        check_repeat_count(repeat.k)?;
    }
    // ... and every write destination claimed before inference (kept for the
    // whole run; an unwritten claim is released on an early return).
    let _reserved = if record {
        // No write destination over any read input or another write.
        let survey = judge_repeat.as_ref().map(|r| r.survey_cassette.as_path());
        check_destinations(&cassette, survey)?;
        let sidecar = scorer::verdicts_sidecar(&cassette);
        Some(Reserved::new(&[cassette.as_path(), sidecar.as_path()])?)
    } else {
        None
    };
    let survey_replay_router = judge_repeat
        .as_ref()
        .map(|repeat| route(Arc::clone(&repeat.survey)));
    let replayed = match &judge_repeat {
        Some(repeat) => Some(scorer::read_cassette(&repeat.survey_cassette)?),
        None if !record => Some(scorer::read_cassette(&cassette)?),
        None => None,
    };
    let live = replayed
        .as_ref()
        .map_or(Ok(true), |(_, json)| scorer::live_layout(json))?;
    if let Some(repeat) = judge_repeat.as_ref().filter(|_| !live) {
        return Err(format!(
            "--judge-repeat needs a live-record survey cassette; {} is not (a seed survey \
             would ask the judge nothing and print the frozen verdicts)",
            repeat.survey_cassette.display()
        ));
    }
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
        )
        .with_model(pinned.clone());
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
    let mut judge_repeat_total = 0usize;
    // Story 28-3 (D-28-3-03 A): per-finding judge verdicts over the repeats and
    // per-pass faithful totals (pass p = the p-th reply of every finding).
    let repeats = judge_repeat.as_ref().map_or(1, |repeat| repeat.k);
    let mut judge_repeat_stats = serde_json::Map::new();
    let mut pass_faithful = vec![0usize; repeats];
    let mut judged_findings = 0usize;
    let mut ask = || -> Result<(), String> {
        let live_rows = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| scorer::served_live(*row));
        for (s, _) in live_rows.clone() {
            let router = survey_replay_router.as_ref().unwrap_or(&router);
            let (fact, failed, n) = survey(s, router, false)?;
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
                let mut verdicts = Vec::with_capacity(repeats);
                for (pass, pass_total) in pass_faithful.iter_mut().enumerate() {
                    let token = mint(EVAL_PID, &provider)?;
                    // Story 18-4 (BH-3 witness): one token per judge call.
                    judge_tokens.insert(token.token_id.0);
                    let req = scorer::judge_request(EVAL_PID, token, pair, &provider, &model);
                    let response = judge.complete(req).map_err(|e| format!("judge: {e}"))?;
                    calls += 1;
                    judge_repeat_total += 1;
                    // Every reply — not only the first of a finding — must be
                    // the pinned model's and one of the two words (R3). An
                    // error here still flushes the recording (BH-4).
                    let verdict = scorer::judge_reply_verdict(&response, &model)
                        .map_err(|e| format!("{}/{}: {e}", row.scenario_id, f.claim_id))?;
                    *pass_total += usize::from(verdict);
                    if pass == 0 {
                        let (scenario, claim) = (row.scenario_id.clone(), f.claim_id.clone());
                        replies.push(scorer::JudgeReply(scenario, claim, response));
                    }
                    verdicts.push(verdict);
                }
                judged_findings += 1;
                if let Some(repeat) = &judge_repeat {
                    let first = verdicts[0];
                    judge_repeat_stats.insert(
                        f.claim_id.clone(),
                        serde_json::json!({
                            "k": repeat.k,
                            "agree_with_first": verdicts.iter().filter(|v| **v == first).count(),
                            "faithful": verdicts.iter().filter(|v| **v).count(),
                            "verdicts": verdicts,
                        }),
                    );
                }
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
        if let Some(repeat) = &judge_repeat {
            // Judge-repeat consumes the survey cassette's SURVEY entries only
            // (its recorded judge replies are re-judged live, never replayed).
            let surveys = calls - committed_calls - judge_repeat_total;
            let entries = json["entries"].as_array().map(Vec::len).unwrap_or(0);
            if entries < surveys {
                return Err(format!(
                    "{}: {entries} entries cannot serve {surveys} survey completions",
                    repeat.survey_cassette.display()
                ));
            }
        } else {
            let served = if live { calls - committed_calls } else { calls };
            scorer::fully_consumed(json, served).map_err(|e| format!("{cassette:?}: {e}"))?;
        }
    }
    if record {
        let sidecar = scorer::verdicts_sidecar(&cassette);
        write_replacing(
            &sidecar,
            &scorer::live_verdicts(&facts, &model, temperature_applied)?,
        )
        .map_err(|e| format!("verdicts sidecar {sidecar:?}: {e}"))?;
    }
    let judge_provenance = scorer::judge_provenance(&rows, live);
    let floors = distill_floors(&model)?;
    let mut score = scorer::score(&facts)?;
    // Story 28-3 (D-28-3-04 C): the run's own SURVEY completions decide the
    // format-error count (the Spirit falls back to the deterministic summary,
    // so the count is only visible here, from the cassette). Judge replies
    // are one word by contract and never count.
    let (bytes, json) = match replayed {
        Some(read) => read,
        None => scorer::read_cassette(&cassette)?,
    };
    let survey_hashes: std::collections::HashSet<String> = rows
        .iter()
        .flat_map(|row| {
            row.claims
                .iter()
                .filter(|c| !row.withheld.contains(&c.claim_id))
                .map(|c| {
                    sha256_hex(researcher::synthesis_prompt(&c.statement, &c.hedges).as_bytes())
                })
        })
        .collect();
    score.metrics.format_errors = json["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|e| {
                    survey_hashes.contains(e["prompt_sha256"].as_str().unwrap_or(""))
                        && !scorer::is_finding_completion(
                            e["response"]["text"].as_str().unwrap_or(""),
                        )
                })
                .count()
        })
        .unwrap_or_default();
    failures.extend(scorer::gate_failures_with(&score, &floors));
    // Story 28-3 review (finding 2): a judge-only repeat names two artifacts,
    // each with its own digest — the survey it replayed (`survey_cassette`)
    // and the judge replies it recorded (`cassette`, the one `xtask
    // eval-report` re-hashes).
    let judge_output = judge_repeat
        .as_ref()
        .map(|_| scorer::read_cassette(&cassette))
        .transpose()?;
    // The archived judge output holds exactly the measured judge calls (no
    // survey entry, no zero-call measurement).
    if let Some((_, json)) = &judge_output {
        let entries = json["entries"].as_array().map_or(0, Vec::len);
        if judge_repeat_total == 0 || entries != judge_repeat_total {
            return Err(format!(
                "{}: {entries} recorded entries for {judge_repeat_total} judge calls",
                cassette.display()
            ));
        }
    }
    let (artifact_bytes, artifact_json) = judge_output
        .as_ref()
        .map_or((&bytes, &json), |o| (&o.0, &o.1));
    let repeat_report = judge_repeat.as_ref().map(|repeat| {
        let calls_per_pass = judged_findings.max(1) as f64;
        let per_pass: Vec<f64> = pass_faithful
            .iter()
            .map(|faithful| *faithful as f64 / calls_per_pass)
            .collect();
        let sum = |key: &str| -> usize {
            judge_repeat_stats
                .values()
                .map(|v| v[key].as_u64().unwrap_or(0) as usize)
                .sum()
        };
        let total = (repeat.k * judged_findings).max(1) as f64;
        serde_json::json!({
            "k": repeat.k,
            "survey_cassette": repeat.survey_cassette.display().to_string(),
            "survey_cassette_sha256": sha256_hex(&bytes),
            "judge_calls": judge_repeat_total,
            // The gate (`pass`, `missed_floors`) rests on the FIRST verdict of
            // each finding; everything below is repeat statistics, reported,
            // never gated.
            "gate_basis": "first-verdict",
            "findings": serde_json::Value::Object(judge_repeat_stats.clone()),
            "agreement_rate": sum("agree_with_first") as f64 / total,
            "faithfulness_per_pass": per_pass,
            "faithfulness_mean": sum("faithful") as f64 / total,
            "faithfulness_worst_pass": per_pass.iter().copied().fold(f64::INFINITY, f64::min),
        })
    });
    let line = serde_json::json!({
        "event": "eval_distill",
        "class": "researcher",
        "corpus": scorer::CORPUS,
        "corpus_sha256": sha256_hex(text.as_bytes()),
        "cassette": cassette.display().to_string(),
        "cassette_sha256": sha256_hex(artifact_bytes),
        "provenance": artifact_json.get("provenance"),
        "judge_provenance": judge_provenance,
        "judge_prompt_sha256": sha256_hex(scorer::JUDGE_PROMPT.as_bytes()),
        "judge_temperature_applied": temperature_applied,
        "completions_consumed": calls,
        "judge_calls": judge_calls,
        "judge_tokens_minted": judge_tokens.len(),
        "floors": floors,
        "judge_repeat": repeat_report,
        "decisions": scorer::claim_decisions(&facts)?,
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

/// Story 28-3 (AC6) — the Researcher's floors: its calibrated record
/// `spirits/researcher/eval/floors.toml` when one exists, else the NFR-Aud-7
/// spec minimums, resolved by the logic `xtask eval-report` shares
/// ([`scorer::resolve_floors`]: schema, evidence, ruling shape and hash, and
/// the judge archive when present). A record that does not parse or does not
/// apply is an error, never a silent fallback (same rule as Butler's, 28-1). An
/// ordinary run does not demand the archive (`require_archive = false`).
fn distill_floors(judge_model: &str) -> Result<DistillFloors, String> {
    let path = Path::new(FLOORS_FILE);
    let shown = |e: &dyn std::fmt::Display| format!("floor record {}: {e}", path.display());
    let record = match std::fs::read_to_string(path) {
        Ok(text) => Some(toml::from_str::<DistillFloorRecord>(&text).map_err(|e| shown(&e))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(shown(&e)),
    };
    scorer::resolve_floors(record, Path::new("."), judge_model, false).map_err(|e| shown(&e))
}
