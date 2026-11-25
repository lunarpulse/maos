#![forbid(unsafe_code)]

//! Story 18-3 AC2 — `maos eval distill --class researcher` (D-18-3-06/07 A,
//! review rulings R3/R4).
//!
//! - The committed (`seed`) cassette replays the frozen corpus through the
//!   kernel adapter: every NFR-Aud-7 floor holds over the clean rows, all five
//!   negative controls (four planted rows + the frozen `false` verdicts'
//!   faithfulness control) are detected, provenance `seed` is published.
//! - A drifted prompt hash, and a completion whose text no longer reproduces
//!   the frozen survey (re-pin needed), exit 1; a non-researcher class is a
//!   usage error (exit 2).
//! - Record through a router whose `anthropic` driver is a local stub writes a
//!   `live-record` cassette (the live rows' surveys, then one pinned-judge
//!   reply per finding of those rows; the completion-planted controls come
//!   from the committed cassette) plus the per-finding verdict sidecar. Its
//!   replay scores the RECORDED judge replies, refuses an unconsumed entry;
//!   record refuses a judge reply attributed to another model.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_bench::distill_eval::{evaluate, DistillRun};
use maos_bench::halt_eval::Completions;
use maos_bin::cassette_replay::{CassetteRecordProvider, CassetteRecorder, CassetteReplayProvider};
use maos_domain::ports::inference::{
    InferenceRequest, InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
};
use maos_eval::distill_replay::{CASSETTE, JUDGE_PROMPT};
use maos_kernel_core::inference::router::MultiProviderRouter;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// `tests/judge-config.toml`'s `anthropic:claude-sonnet-5-5@2026-10-25`.
const PINNED: &str = "claude-sonnet-5-5";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn eval(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maos"))
        .current_dir(root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"))
        .args(["eval", "distill"])
        .args(args)
        .output()
        .expect("spawn maos")
}

fn line(out: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one stdout line: {stdout}");
    serde_json::from_str(lines[0]).expect("JSON line")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn committed() -> Value {
    read_json(&root().join(CASSETTE))
}

fn write_variant(dir: &Path, mut cassette: Value, edit: impl FnOnce(&mut Value)) -> String {
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
    path.display().to_string()
}

fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    write_variant(dir, committed(), edit)
}

fn assert_all_five_controls_detected(line: &Value) {
    let controls = line["score"]["controls"].as_array().unwrap();
    assert_eq!(controls.len(), 5, "{controls:?}");
    assert!(
        controls.iter().all(|c| c["detected"] == true),
        "{controls:?}"
    );
    assert!(
        controls
            .iter()
            .any(|c| c["control"] == "unfaithful-finding"),
        "{controls:?}"
    );
}

#[test]
fn the_committed_cassette_holds_all_five_floors_and_detects_every_control() {
    let home = tempfile::tempdir().unwrap();
    let out = eval(home.path(), &["--class", "researcher"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["event"], "eval_distill");
    assert_eq!(line["pass"], true);
    assert_eq!(line["provenance"], "seed");
    assert_eq!(line["judge_provenance"], serde_json::json!(["seed"]));
    let entries = committed()["entries"].as_array().unwrap().len();
    assert_eq!(line["completions_consumed"], entries);
    let m = &line["score"]["metrics"];
    assert!(m["recall"].as_f64().unwrap() >= 0.90);
    assert!(m["faithfulness"].as_f64().unwrap() >= 0.98);
    assert!(m["hedge_preservation"].as_f64().unwrap() >= 0.95);
    assert_eq!(m["traceability"], 1.0);
    assert_eq!(m["secret_leakage"], 0.0);
    assert_all_five_controls_detected(&line);
}

#[test]
fn a_drifted_prompt_or_a_survey_that_no_longer_reproduces_exits_1() {
    let home = tempfile::tempdir().unwrap();
    let drifted = variant(home.path(), |c| {
        c["entries"][0]["prompt_sha256"] = "ab".repeat(32).into();
    });
    let out = eval(
        home.path(),
        &["--class", "researcher", "--cassette", &drifted],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("drift"), "{}", stderr(&out));

    let reworded = variant(home.path(), |c| {
        c["entries"][0]["response"]["text"] = "A different digest sentence.".into();
    });
    let out = eval(
        home.path(),
        &["--class", "researcher", "--cassette", &reworded],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("re-pin needed"), "{}", stderr(&out));
    assert_eq!(line(&out)["pass"], false);
}

#[test]
fn a_class_other_than_the_researcher_is_a_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let out = eval(home.path(), &["--class", "butler"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
}

/// Answers a survey prompt with the committed completion for its hash and a
/// judge prompt with `judge(prompt)`; attributes every reply to `model`, else
/// to the request's `options.model_id` (the R3 provider contract).
struct StubProvider {
    answers: Vec<(String, String)>,
    judge: fn(&str) -> &'static str,
    model: Option<&'static str>,
}

impl maos_providers::Provider for StubProvider {
    fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, maos_providers::provider::ProviderError> {
        let hash = format!("{:x}", Sha256::digest(req.prompt.as_bytes()));
        let text = match self.answers.iter().find(|(h, _)| *h == hash) {
            Some((_, text)) => text.clone(),
            None if req.prompt.starts_with(JUDGE_PROMPT) => (self.judge)(&req.prompt).into(),
            None => panic!("an unseeded prompt: {}", req.prompt),
        };
        let model_id = match self.model {
            Some(model) => Some(model.to_owned()),
            None => req.options.model_id.clone(),
        };
        Ok(InferenceResponse {
            text,
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "anthropic".into(),
                endpoint_url: "stub://local".into(),
                model_id,
            },
        })
    }

    fn credential_fingerprint(&self) -> u64 {
        0
    }
}

/// `MAOS_INFERENCE_MODE=record` through the stub: the run and the recorded
/// cassette's path.
fn record(
    dir: &Path,
    judge: fn(&str) -> &'static str,
    model: Option<&'static str>,
) -> (Result<DistillRun, String>, PathBuf) {
    let answers = committed()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let text = e["response"]["text"].as_str().unwrap().to_owned();
            // Story 28-3: the v1 prompt's schema is one JSON object
            // `{"finding": string}`; the stub models a compliant model, so
            // the recorded cassette replays with zero format errors.
            // Idempotent: the committed cassette is already v1-shaped.
            let finding = if text.starts_with("{\"finding\"") {
                text
            } else {
                serde_json::json!({ "finding": text }).to_string()
            };
            (e["prompt_sha256"].as_str().unwrap().to_owned(), finding)
        })
        .collect();
    let recorded = dir.join("distill-corpus-v0.live.json");
    let recorder = Arc::new(CassetteRecorder::new(
        recorded.clone(),
        "maos".into(),
        "eval-distill-record".into(),
    ));
    let stub = StubProvider {
        answers,
        judge,
        model,
    };
    let driver = CassetteRecordProvider::new(Arc::new(stub), recorder.clone());
    let drivers = [("anthropic".to_owned(), Arc::new(driver) as Arc<_>)].into();
    let router = Arc::new(MultiProviderRouter::new(drivers, Some("anthropic".into())));
    let committed = CassetteReplayProvider::from_file(&root().join(CASSETTE), true).unwrap();

    // The composition reads workspace-relative paths, like the verb.
    std::env::set_current_dir(root()).unwrap();
    let run = evaluate(
        Completions::Record {
            router,
            cassette: recorded.clone(),
            flush: Box::new(move || recorder.flush().map(drop)),
        },
        Arc::new(committed),
    );
    (run, recorded)
}

fn faithful(_: &str) -> &'static str {
    "Faithful."
}

#[test]
fn record_through_the_router_writes_a_live_cassette_and_sidecar_that_replay_green() {
    let home = tempfile::tempdir().unwrap();
    let (run, recorded) = record(home.path(), faithful, None);
    let run = run.unwrap();
    assert!(run.failures.is_empty(), "{:?}", run.failures);
    let line: Value = serde_json::from_str(&run.line).unwrap();
    // The completion-planted controls keep their frozen (seed) verdicts.
    assert_eq!(
        line["judge_provenance"],
        serde_json::json!(["live-record", "seed"])
    );
    assert_all_five_controls_detected(&line);
    // Story 18-4 (BH-3, `deferred-work.md:1253`): record mode mints ONE token
    // per judge call — one loop-wide token would show 1 distinct token here.
    // (The 60 s expiry itself is not reproduced: the TTL is checked against
    // the capability crate's process-global monotonic clock.)
    let judge_calls = line["judge_calls"].as_u64().expect("judge_calls");
    assert!(
        judge_calls > 1,
        "the record run asks the judge more than once"
    );
    assert_eq!(
        line["judge_tokens_minted"], judge_calls,
        "one distinct token per judge call"
    );

    let cassette = read_json(&recorded);
    assert_eq!(cassette["provenance"], "live-record");
    let entries = cassette["entries"].as_array().unwrap();
    let seed = committed();
    let seeded: Vec<&str> = seed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["prompt_sha256"].as_str().unwrap())
        .collect();
    let surveys: Vec<&str> = entries
        .iter()
        .map(|e| e["prompt_sha256"].as_str().unwrap())
        .take_while(|hash| seeded.contains(hash))
        .collect();
    // rd13 (hedge-drop) and rd15 (planted-secret) survey two claims each and
    // are served from the committed cassette, never recorded.
    assert_eq!(surveys.len(), seeded.len() - 4, "{surveys:?}");
    let mut rest = seeded.iter();
    assert!(
        surveys.iter().all(|hash| rest.any(|seed| seed == hash)),
        "the live surveys keep corpus order"
    );
    assert!(entries.len() > surveys.len(), "surveys, then judge replies");
    for judge in &entries[surveys.len()..] {
        assert_eq!(
            judge["response"]["provider_attribution"]["model_id"],
            PINNED
        );
    }

    let sidecar = read_json(&home.path().join("distill-corpus-v0.live.verdicts.json"));
    assert_eq!(sidecar["judge_model"], PINNED);
    // `claude-sonnet-5-5` rejects `temperature`: the pinned 0.0 was requested
    // but never applied, and both artifacts say so.
    assert_eq!(sidecar["judge_temperature_applied"], false);
    assert_eq!(line["judge_temperature_applied"], false);
    let prompt = format!("{:x}", Sha256::digest(JUDGE_PROMPT.as_bytes()));
    assert_eq!(sidecar["judge_prompt_sha256"], prompt.as_str());
    assert_eq!(sidecar["verdicts"]["rd01"]["rd01-c1"], true);
    assert_eq!(
        sidecar["verdicts"]["rd14-fabricated-cite"]["rd14-fabricated-cite-c1"],
        true
    );
    assert!(sidecar["verdicts"].get("rd13-hedge-drop").is_none());
    assert!(sidecar["verdicts"].get("rd15-planted-secret").is_none());
    let judged: usize = sidecar["verdicts"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_object().unwrap().len())
        .sum();
    assert_eq!(
        judged,
        entries.len() - surveys.len(),
        "one reply per finding"
    );

    let out = eval(
        home.path(),
        &[
            "--class",
            "researcher",
            "--cassette",
            recorded.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let replayed = self::line(&out);
    assert_eq!(replayed["provenance"], "live-record");
    assert_eq!(replayed["pass"], true);
    assert_eq!(
        replayed["completions_consumed"],
        seeded.len() + entries.len(),
        "the whole committed cassette, then the whole live one"
    );
    assert_all_five_controls_detected(&replayed);

    // A trailing entry the run never consumed fails the replay.
    let mut extra = entries.last().unwrap().clone();
    extra["sequence"] = entries.len().into();
    let trailing = write_variant(home.path(), cassette.clone(), |c| {
        c["entries"].as_array_mut().unwrap().push(extra);
    });
    let out = eval(
        home.path(),
        &["--class", "researcher", "--cassette", &trailing],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("unconsumed"), "{}", stderr(&out));
}

#[test]
fn a_live_record_replay_scores_the_recorded_judge_replies_not_the_frozen_verdicts() {
    // The frozen verdict for rd01-c1 is `true`; the recorded judge says no.
    fn rd01_unfaithful(prompt: &str) -> &'static str {
        if prompt.contains("Digest finding: The source reports that retrieval grounding") {
            "**unfaithful**"
        } else {
            "faithful"
        }
    }
    let home = tempfile::tempdir().unwrap();
    let (run, recorded) = record(home.path(), rd01_unfaithful, None);
    let run = run.unwrap();
    assert!(
        run.failures.iter().any(|f| f.contains("faithfulness")),
        "{:?}",
        run.failures
    );
    let sidecar = read_json(&home.path().join("distill-corpus-v0.live.verdicts.json"));
    assert_eq!(sidecar["verdicts"]["rd01"]["rd01-c1"], false);

    let out = eval(
        home.path(),
        &[
            "--class",
            "researcher",
            "--cassette",
            recorded.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let replayed = line(&out);
    assert_eq!(replayed["pass"], false);
    assert!(
        replayed["score"]["metrics"]["faithfulness"]
            .as_f64()
            .unwrap()
            < 1.0
    );
    assert!(stderr(&out).contains("faithfulness"), "{}", stderr(&out));
}

#[test]
fn record_refuses_a_judge_reply_attributed_to_another_model_after_flushing() {
    let home = tempfile::tempdir().unwrap();
    let (run, recorded) = record(home.path(), faithful, Some("claude-haiku-9"));
    let error = run.unwrap_err();
    assert!(
        error.contains("not the pinned claude-sonnet-5-5"),
        "{error}"
    );
    assert!(error.contains("claude-haiku-9"), "{error}");
    assert_eq!(
        read_json(&recorded)["provenance"],
        "live-record",
        "the recording is flushed before the refusal"
    );
    assert!(!home
        .path()
        .join("distill-corpus-v0.live.verdicts.json")
        .exists());
}

/// A judge whose verdict flips every call: faithful, unfaithful, faithful, …
/// Keyed on a process-wide call counter, so the same prompt is answered
/// differently across the k repeats of D-28-3-03 A.
struct FlippingJudge {
    surveys: Vec<(String, String)>,
    calls: std::sync::atomic::AtomicUsize,
}

impl maos_providers::Provider for FlippingJudge {
    fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, maos_providers::provider::ProviderError> {
        let hash = format!("{:x}", Sha256::digest(req.prompt.as_bytes()));
        let text = match self.surveys.iter().find(|(h, _)| *h == hash) {
            Some((_, text)) => text.clone(),
            None if req.prompt.starts_with(JUDGE_PROMPT) => {
                let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n % 2 == 0 {
                    "faithful"
                } else {
                    "**unfaithful**"
                }
                .to_owned()
            }
            None => panic!("an unseeded prompt: {}", req.prompt),
        };
        Ok(InferenceResponse {
            text,
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "anthropic".into(),
                endpoint_url: "stub://local".into(),
                model_id: Some(PINNED.to_owned()),
            },
        })
    }

    fn credential_fingerprint(&self) -> u64 {
        0
    }
}

/// Story 28-3 (D-28-3-03 A) — the judge-only repeat: the recorded survey
/// replays (zero survey calls through the live router), the live judge runs
/// k times per finding, and the line reports per-finding agreement with the
/// first verdict plus the faithfulness mean/min. A flipping judge pins both
/// arms of the agreement count.
#[test]
fn a_judge_only_repeat_replays_the_survey_and_reports_per_finding_agreement() {
    use maos_bench::distill_eval::{evaluate_with_repeat, JudgeRepeat};

    // The parser refuses the repeat without the record mode or a cassette.
    for (argv, mode) in [
        (
            vec![
                "distill".to_owned(),
                "--class".to_owned(),
                "researcher".to_owned(),
                "--cassette".to_owned(),
                "x.json".to_owned(),
                "--judge-repeat".to_owned(),
                "3".to_owned(),
            ],
            Some("replay"),
        ),
        (
            vec![
                "distill".to_owned(),
                "--class".to_owned(),
                "researcher".to_owned(),
                "--judge-repeat".to_owned(),
                "3".to_owned(),
            ],
            Some("record"),
        ),
    ] {
        assert!(
            maos_bench::distill_eval::parse_args_with_repeat(&argv, mode, None, None).is_err(),
            "refused: {argv:?} under {mode:?}"
        );
    }
    let ok = maos_bench::distill_eval::parse_args_with_repeat(
        &[
            "distill".to_owned(),
            "--class".to_owned(),
            "researcher".to_owned(),
            "--cassette".to_owned(),
            "recorded.json".to_owned(),
            "--judge-repeat".to_owned(),
            "3".to_owned(),
        ],
        Some("record"),
        None,
        None,
    )
    .expect("record + cassette + judge-repeat parses");
    assert_eq!((ok.judge_repeat, ok.record), (Some(3), true));

    // One recorded live run supplies the survey cassette.
    let home = tempfile::tempdir().unwrap();
    let (_, recorded) = record(home.path(), faithful, None);
    let answers = committed()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let text = e["response"]["text"].as_str().unwrap().to_owned();
            let finding = if text.starts_with("{\"finding\"") {
                text
            } else {
                serde_json::json!({ "finding": text }).to_string()
            };
            (e["prompt_sha256"].as_str().unwrap().to_owned(), finding)
        })
        .collect();
    let repeat_target = home.path().join("judge-repeat.json");
    let recorder = Arc::new(CassetteRecorder::new(
        repeat_target.clone(),
        "maos".into(),
        "eval-distill-judge-repeat".into(),
    ));
    let stub = Arc::new(FlippingJudge {
        surveys: answers,
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let driver = CassetteRecordProvider::new(stub, recorder.clone());
    let drivers = [("anthropic".to_owned(), Arc::new(driver) as Arc<_>)].into();
    let router = Arc::new(MultiProviderRouter::new(drivers, Some("anthropic".into())));
    let survey = Arc::new(CassetteReplayProvider::from_file(&recorded, true).unwrap());
    std::env::set_current_dir(root()).unwrap();
    let run = evaluate_with_repeat(
        Completions::Record {
            router,
            cassette: repeat_target.clone(),
            flush: Box::new(move || recorder.flush().map(drop)),
        },
        Arc::new(CassetteReplayProvider::from_file(&root().join(CASSETTE), true).unwrap()),
        Some(JudgeRepeat {
            k: 3,
            survey,
            survey_cassette: recorded.clone(),
        }),
    )
    .expect("the judge-repeat run scores");
    let line: Value = serde_json::from_str(&run.line).unwrap();
    let repeat = &line["judge_repeat"];
    assert_eq!(repeat["k"], 3);
    assert_eq!(repeat["survey_cassette"], recorded.display().to_string());
    let findings = repeat["findings"].as_object().unwrap();
    assert!(!findings.is_empty(), "one row per live finding");
    // The flipping judge: verdicts alternate per call, so over k = 3 the
    // first verdict repeats 2 of 3 times and faithfulness is 2 of 3 or 1 of 3
    // depending on the finding's first call parity.
    for (claim, row) in findings {
        let (agree, faithful) = (
            row["agree_with_first"].as_u64().unwrap(),
            row["faithful"].as_u64().unwrap(),
        );
        assert_eq!(row["k"], 3, "{claim}");
        assert!(
            agree == 2 && (faithful == 2 || faithful == 1),
            "{claim}: agree {agree}, faithful {faithful}"
        );
    }
    let mean = repeat["faithfulness_mean"].as_f64().unwrap();
    assert!(
        (0.49..=0.52).contains(&mean),
        "the flipping judge is faithful about half the calls: {mean}"
    );
    let judge_calls = repeat["judge_calls"].as_u64().unwrap() as usize;
    assert_eq!(judge_calls, findings.len() * 3, "k per finding");
    // The applied verdicts are the FIRST repeat's (one reply per finding);
    // the surveys replayed from the recorded cassette, so the run consumed
    // the surveys plus k judge calls per finding.
    assert_eq!(
        line["judge_calls"].as_u64().unwrap() as usize,
        findings.len()
    );
    assert!(
        line["completions_consumed"].as_u64().unwrap() as usize > judge_calls,
        "surveys plus k-per-finding judge calls were consumed"
    );
}

// ---------------------------------------------------------------------------
// Story 28-3 review fixes (findings 2, 3, 4, 5, 7) — the judge-only repeat's
// artifacts, per-reply attribution, pass statistics and CLI/evaluator bounds.
// ---------------------------------------------------------------------------

/// What a scripted judge answers to its n-th call (0-based, process order):
/// the verdict text and, when set, the model its reply is attributed to.
type Script = Box<dyn Fn(usize) -> (&'static str, Option<&'static str>) + Send + Sync>;

struct ScriptedJudge {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    script: Script,
}

impl maos_providers::Provider for ScriptedJudge {
    fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, maos_providers::provider::ProviderError> {
        assert!(
            req.prompt.starts_with(JUDGE_PROMPT),
            "a judge-only repeat sends only judge prompts through the live router"
        );
        let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (text, model) = (self.script)(n);
        Ok(InferenceResponse {
            text: text.into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "anthropic".into(),
                endpoint_url: "stub://local".into(),
                model_id: Some(model.unwrap_or(PINNED).to_owned()),
            },
        })
    }

    fn credential_fingerprint(&self) -> u64 {
        0
    }
}

/// One judge-only repeat over a freshly recorded survey: `output` is where the
/// recorder writes (default: a sibling distinct from the survey).
struct Repeat {
    run: Result<DistillRun, String>,
    survey: PathBuf,
    output: PathBuf,
    calls: usize,
}

fn judge_repeat(dir: &Path, k: usize, output_is_survey: bool, script: Script) -> Repeat {
    judge_repeat_over(dir, None, k, output_is_survey, script)
}

/// `survey`: replay this cassette instead of recording a fresh `live-record` one.
fn judge_repeat_over(
    dir: &Path,
    survey: Option<PathBuf>,
    k: usize,
    output_is_survey: bool,
    script: Script,
) -> Repeat {
    let survey = survey.unwrap_or_else(|| record(dir, faithful, None).1);
    let output = if output_is_survey {
        survey.clone()
    } else {
        dir.join("judge-only.live.json")
    };
    judge_repeat_at(survey, output, k, script)
}

/// The judge-only repeat of `survey`, recording to `output`.
fn judge_repeat_at(survey: PathBuf, output: PathBuf, k: usize, script: Script) -> Repeat {
    use maos_bench::distill_eval::{evaluate_with_repeat, JudgeRepeat};

    let recorder = Arc::new(CassetteRecorder::new(
        output.clone(),
        "maos".into(),
        "eval-distill-judge-repeat".into(),
    ));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stub = ScriptedJudge {
        calls: Arc::clone(&calls),
        script,
    };
    let driver = CassetteRecordProvider::new(Arc::new(stub), recorder.clone());
    let drivers = [("anthropic".to_owned(), Arc::new(driver) as Arc<_>)].into();
    let router = Arc::new(MultiProviderRouter::new(drivers, Some("anthropic".into())));
    let replay = Arc::new(CassetteReplayProvider::from_file(&survey, true).unwrap());
    std::env::set_current_dir(root()).unwrap();
    let run = evaluate_with_repeat(
        Completions::Record {
            router,
            cassette: output.clone(),
            flush: Box::new(move || recorder.flush().map(drop)),
        },
        Arc::new(CassetteReplayProvider::from_file(&root().join(CASSETTE), true).unwrap()),
        Some(JudgeRepeat {
            k,
            survey: replay,
            survey_cassette: survey.clone(),
        }),
    );
    Repeat {
        run,
        survey,
        output,
        calls: calls.load(std::sync::atomic::Ordering::SeqCst),
    }
}

fn sha_of(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

fn always_faithful() -> Script {
    Box::new(|_| ("faithful", None))
}

fn repeat_args(extra: &[&str]) -> Vec<String> {
    ["distill", "--class", "researcher"]
        .iter()
        .chain(extra)
        .map(|s| (*s).to_owned())
        .collect()
}

/// Finding 2 — `cassette` names the judge OUTPUT and `cassette_sha256` is its
/// digest; the survey input is its own `survey_cassette` with its own digest.
#[test]
fn a_judge_repeat_reports_both_artifacts_each_with_its_own_digest() {
    let home = tempfile::tempdir().unwrap();
    let r = judge_repeat(home.path(), 2, false, always_faithful());
    let line: Value = serde_json::from_str(&r.run.unwrap().line).unwrap();
    assert_ne!(sha_of(&r.survey), sha_of(&r.output));
    assert_eq!(line["cassette"], r.output.display().to_string());
    assert_eq!(line["cassette_sha256"], sha_of(&r.output).as_str());
    let repeat = &line["judge_repeat"];
    assert_eq!(repeat["survey_cassette"], r.survey.display().to_string());
    assert_eq!(repeat["survey_cassette_sha256"], sha_of(&r.survey).as_str());
}

/// Finding 2 — the recorder would overwrite the source survey with judge-only
/// replies: refused before any inference, the survey untouched.
#[test]
fn the_survey_and_the_judge_output_may_not_be_the_same_file() {
    let home = tempfile::tempdir().unwrap();
    // Evaluator boundary.
    let r = judge_repeat(home.path(), 2, true, always_faithful());
    let error = r.run.unwrap_err();
    assert!(error.contains("same file"), "{error}");
    assert_eq!(r.calls, 0, "refused before the live judge is asked");
    let (_, survey) = (0, &r.survey);
    let cassette = read_json(survey);
    assert!(
        cassette["entries"].as_array().unwrap().len() > 2,
        "the survey cassette still holds its survey entries"
    );
    // CLI boundary: `--cassette` and `MAOS_REPLAY_CASSETTE` are one path,
    // spelled differently or not.
    let spelled = home.path().join("./x.json");
    let alias = home.path().join("x.json");
    for (flag, env) in [
        ("x.json", "x.json"),
        (spelled.to_str().unwrap(), alias.to_str().unwrap()),
    ] {
        let parsed = maos_bench::distill_eval::parse_args_with_repeat(
            &repeat_args(&["--cassette", flag, "--judge-repeat", "3"]),
            Some("record"),
            Some(PathBuf::from(env)),
            None,
        );
        let error = parsed.unwrap_err();
        assert!(error.contains("same file"), "{flag} vs {env}: {error}");
    }
    let fine = maos_bench::distill_eval::parse_args_with_repeat(
        &repeat_args(&["--cassette", "x.json", "--judge-repeat", "3"]),
        Some("record"),
        Some(PathBuf::from("y.json")),
        None,
    );
    assert_eq!(fine.unwrap().judge_repeat, Some(3));
}

/// Finding 7 — the hosted 1-10 bound holds at the CLI and the evaluator; 0 is
/// refused, never silently dropped into an ordinary paid pass.
#[test]
fn the_judge_repeat_count_is_one_to_ten_at_the_cli_and_the_evaluator() {
    for count in ["0", "11", "100", "-1", "x"] {
        for extra in [
            &["--judge-repeat", count][..],
            &["--cassette", "a.json", "--judge-repeat", count],
        ] {
            for mode in [Some("record"), Some("replay"), None] {
                let parsed = maos_bench::distill_eval::parse_args_with_repeat(
                    &repeat_args(extra),
                    mode,
                    Some(PathBuf::from("b.json")),
                    None,
                );
                assert!(parsed.is_err(), "{extra:?} under {mode:?}: {parsed:?}");
            }
        }
    }
    for count in ["1", "10"] {
        let parsed = maos_bench::distill_eval::parse_args_with_repeat(
            &repeat_args(&["--cassette", "a.json", "--judge-repeat", count]),
            Some("record"),
            Some(PathBuf::from("b.json")),
            None,
        );
        assert_eq!(parsed.unwrap().judge_repeat, count.parse().ok());
    }
    // A hostile default is bounded the same way.
    let parsed = maos_bench::distill_eval::parse_args_with_repeat(
        &repeat_args(&["--cassette", "a.json"]),
        Some("record"),
        Some(PathBuf::from("b.json")),
        Some(0),
    );
    assert!(parsed.is_err());
    // Evaluator boundary: nothing is asked of the live judge.
    for k in [0, 11] {
        let home = tempfile::tempdir().unwrap();
        let r = judge_repeat(home.path(), k, false, always_faithful());
        let error = r.run.unwrap_err();
        assert!(error.contains("1-10"), "k={k}: {error}");
        assert_eq!(r.calls, 0, "k={k}");
    }
}

/// Finding 3 — attribution is validated on EVERY reply, not the first per
/// finding; the recording is flushed before the refusal.
#[test]
fn a_later_repeat_reply_from_another_model_is_refused_after_flushing() {
    let home = tempfile::tempdir().unwrap();
    // k = 3: call 4 is the SECOND reply of the second finding.
    let script: Script = Box::new(|n| ("faithful", (n == 4).then_some("claude-haiku-9")));
    let r = judge_repeat(home.path(), 3, false, script);
    let error = r.run.unwrap_err();
    assert!(
        error.contains("not the pinned claude-sonnet-5-5"),
        "{error}"
    );
    assert!(error.contains("claude-haiku-9"), "{error}");
    assert_eq!(
        read_json(&r.output)["provenance"],
        "live-record",
        "the recording is flushed before the refusal"
    );
}

/// Finding 5 — four findings that each flip once: in ONE pass (that pass is
/// 4 findings worse) or in FOUR different passes (each pass 1 finding worse).
/// Per-finding agreement and the mean are equal; the worst pass is not.
#[test]
fn faithfulness_statistics_tell_a_one_pass_flip_from_four_passes_flipping() {
    let k = 5;
    let stats = |script: Script| {
        let home = tempfile::tempdir().unwrap();
        let r = judge_repeat(home.path(), k, false, script);
        let line: Value = serde_json::from_str(&r.run.unwrap().line).unwrap();
        line
    };
    // Call n judges finding n / k in pass n % k.
    let same_pass: Script = Box::new(move |n| {
        (
            if n % k == 1 && n / k < 4 {
                "unfaithful"
            } else {
                "faithful"
            },
            None,
        )
    });
    let different_passes: Script = Box::new(move |n| {
        (
            if n / k < 4 && n % k == n / k + 1 {
                "unfaithful"
            } else {
                "faithful"
            },
            None,
        )
    });
    let (same, different) = (stats(same_pass), stats(different_passes));
    let (a, b) = (&same["judge_repeat"], &different["judge_repeat"]);
    let findings = a["findings"].as_object().unwrap().len() as f64;
    assert!(findings > 4.0, "{findings} findings judged");
    assert!(
        a["findings"]
            .as_object()
            .unwrap()
            .keys()
            .eq(b["findings"].as_object().unwrap().keys()),
        "both runs judged the same findings"
    );
    for stat in [a, b] {
        let agree: u64 = stat["findings"]
            .as_object()
            .unwrap()
            .values()
            .map(|f| f["agree_with_first"].as_u64().unwrap())
            .sum();
        assert_eq!(
            agree as f64,
            findings * k as f64 - 4.0,
            "four flips in total"
        );
        assert_eq!(stat["faithfulness_per_pass"].as_array().unwrap().len(), k);
    }
    let mean = |s: &Value| s["faithfulness_mean"].as_f64().unwrap();
    let worst = |s: &Value| s["faithfulness_worst_pass"].as_f64().unwrap();
    assert!(
        (mean(a) - mean(b)).abs() < 1e-12,
        "the mean cannot tell them apart"
    );
    assert!(
        (worst(a) - (findings - 4.0) / findings).abs() < 1e-12,
        "{}",
        worst(a)
    );
    assert!(
        (worst(b) - (findings - 1.0) / findings).abs() < 1e-12,
        "{}",
        worst(b)
    );
    // The gate is the FIRST pass (every first verdict is faithful here) and
    // says so; the repeat statistics never feed it.
    assert_eq!(same["pass"], different["pass"]);
    assert_eq!(a["gate_basis"], "first-verdict");
}

/// Finding 4 — every distill line carries the claim-level decisions the
/// report's interval is taken over.
#[test]
fn the_line_carries_the_decisions_of_every_clean_claim() {
    let home = tempfile::tempdir().unwrap();
    let (run, _) = record(home.path(), faithful, None);
    let line: Value = serde_json::from_str(&run.unwrap().line).unwrap();
    let decisions = line["decisions"].as_array().expect("decisions");
    assert_eq!(decisions.len(), 25, "the clean rows' claims");
    let claim = &decisions[0];
    assert_eq!(claim["scenario_id"], "rd01");
    assert_eq!(claim["finding"]["faithful"], true);
    assert_eq!(claim["finding"]["resolves"], true);
    assert_eq!(claim["finding"]["leaks"], false);
    assert!(claim["finding"]["hedges"].is_array());
}

/// A judge-only repeat over a `seed` survey would make zero judge calls and
/// still print numbers (the frozen verdicts): refused before anything is asked.
#[test]
fn a_judge_repeat_over_a_seed_survey_is_refused_before_the_judge_is_asked() {
    let home = tempfile::tempdir().unwrap();
    let r = judge_repeat_over(
        home.path(),
        Some(root().join(CASSETTE)),
        3,
        false,
        always_faithful(),
    );
    let error = r.run.unwrap_err();
    assert!(error.contains("live-record"), "{error}");
    assert_eq!(r.calls, 0);
}

/// The archived judge output holds exactly the measured judge calls — nothing
/// a survey replay or a second recorder left behind.
#[test]
fn the_judge_output_cassette_holds_exactly_the_measured_judge_calls() {
    let home = tempfile::tempdir().unwrap();
    let r = judge_repeat(home.path(), 2, false, always_faithful());
    let line: Value = serde_json::from_str(&r.run.unwrap().line).unwrap();
    let measured = line["judge_repeat"]["judge_calls"].as_u64().unwrap() as usize;
    assert!(measured > 0);
    assert_eq!(r.calls, measured);
    let output = read_json(&r.output);
    let entries = output["entries"].as_array().unwrap();
    assert_eq!(entries.len(), measured);
    for entry in entries {
        assert_eq!(
            entry["response"]["provider_attribution"]["model_id"],
            PINNED
        );
    }
}

/// Round 2 (B) — EVERY write destination (the recorded cassette and its
/// `.verdicts.json` sidecar) against EVERY read input (survey cassette and its
/// sidecar, the committed seed, the corpus, the judge config, the floor record)
/// and against each other: symlinks, hard links, relative and not-yet-existing
/// spellings. Refused before the judge is asked; nothing is overwritten.
#[test]
fn no_write_destination_may_alias_a_read_input_or_another_write() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let dir = home.path();
    std::env::set_current_dir(root()).unwrap();
    let (_, survey) = record(dir, faithful, None);
    let survey_sidecar = dir.join("distill-corpus-v0.live.verdicts.json");
    assert!(survey_sidecar.exists());
    let seed = root().join(CASSETTE);
    let fixed = |rel: &str| root().join(rel);
    // A survey whose NAME is the sidecar of the output below.
    let trapped = dir.join("trap.verdicts.json");
    std::fs::copy(&survey, &trapped).unwrap();
    let link = dir.join("link.json");
    symlink(&survey, &link).unwrap();
    let hard = dir.join("hard.json");
    std::fs::hard_link(&survey, &hard).unwrap();
    let relative =
        PathBuf::from("spirits/researcher/tests/fixtures/../fixtures/distill-corpus-v0.jsonl");

    let before = |p: &Path| std::fs::read(p).unwrap();
    let (survey_bytes, seed_bytes, sidecar_bytes) =
        (before(&survey), before(&seed), before(&survey_sidecar));
    let cases: Vec<(&str, PathBuf, PathBuf)> = vec![
        (
            "output is the survey's sidecar name",
            survey.clone(),
            survey_sidecar.clone(),
        ),
        ("output is the committed seed", survey.clone(), seed.clone()),
        (
            "output's sidecar is the survey",
            trapped.clone(),
            dir.join("trap.json"),
        ),
        (
            "output is the corpus",
            survey.clone(),
            fixed("spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl"),
        ),
        (
            "output is the corpus, spelled relative",
            survey.clone(),
            relative,
        ),
        (
            "output is the judge config",
            survey.clone(),
            fixed("tests/judge-config.toml"),
        ),
        (
            "output is the (absent) floor record",
            survey.clone(),
            fixed("spirits/researcher/eval/floors.toml"),
        ),
        ("output is a symlink to the survey", survey.clone(), link),
        ("output is a hard link to the survey", survey.clone(), hard),
        ("output is the survey", survey.clone(), survey.clone()),
    ];
    for (name, survey, output) in cases {
        let r = judge_repeat_at(survey, output, 2, always_faithful());
        let error = r.run.unwrap_err();
        assert!(error.contains("same file"), "{name}: {error}");
        assert_eq!(r.calls, 0, "{name}: refused before the judge is asked");
    }
    assert_eq!(before(&survey), survey_bytes, "the survey is untouched");
    assert_eq!(before(&seed), seed_bytes, "the seed is untouched");
    assert_eq!(before(&survey_sidecar), sidecar_bytes);

    // The plain record (no repeat) refuses the same destinations.
    let parsed = maos_bench::distill_eval::parse_args_with_repeat(
        &repeat_args(&[
            "--cassette",
            survey.to_str().unwrap(),
            "--judge-repeat",
            "2",
        ]),
        Some("record"),
        Some(survey_sidecar.clone()),
        None,
    );
    assert!(parsed.unwrap_err().contains("same file"));
    let parsed = maos_bench::distill_eval::parse_args_with_repeat(
        &repeat_args(&[]),
        Some("record"),
        Some(seed),
        None,
    );
    assert!(parsed.unwrap_err().contains("same file"));
}
