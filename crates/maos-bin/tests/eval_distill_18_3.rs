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
            (e["prompt_sha256"].as_str().unwrap().to_owned(), text)
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
