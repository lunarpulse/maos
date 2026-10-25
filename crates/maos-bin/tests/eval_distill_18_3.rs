#![forbid(unsafe_code)]

//! Story 18-3 AC2 — `maos eval distill --class researcher` (D-18-3-06/07 A).
//!
//! - The committed cassette replays the frozen corpus through the kernel
//!   adapter: every NFR-Aud-7 floor holds over the clean rows, all four
//!   negative controls are detected, provenance `seed` is published.
//! - A drifted prompt hash, and a completion whose text no longer reproduces
//!   the frozen survey (re-pin needed), exit 1; a non-researcher class is a
//!   usage error (exit 2).
//! - Record through a router whose `anthropic` driver is a local stub writes
//!   real-hash `live-record` entries (surveys first, then the live judge) that
//!   then replay green.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_bench::distill_eval::evaluate;
use maos_bench::halt_eval::Completions;
use maos_bin::cassette_replay::{CassetteRecordProvider, CassetteRecorder};
use maos_domain::ports::inference::{
    InferenceRequest, InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
};
use maos_eval::distill_replay::{CASSETTE, JUDGE_PROMPT};
use maos_kernel_core::inference::router::MultiProviderRouter;
use serde_json::Value;
use sha2::{Digest, Sha256};

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

fn committed() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(CASSETTE)).unwrap()).unwrap()
}

fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    let mut cassette = committed();
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
    path.display().to_string()
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
    let controls = line["score"]["controls"].as_array().unwrap();
    assert_eq!(controls.len(), 4, "{controls:?}");
    assert!(
        controls.iter().all(|c| c["detected"] == true),
        "{controls:?}"
    );
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

/// Answers a survey prompt with the committed completion for its hash and
/// every judge prompt with `faithful`.
struct StubProvider(Vec<(String, String)>);

impl maos_providers::Provider for StubProvider {
    fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, maos_providers::provider::ProviderError> {
        let hash = format!("{:x}", Sha256::digest(req.prompt.as_bytes()));
        let text = match self.0.iter().find(|(h, _)| *h == hash) {
            Some((_, text)) => text.clone(),
            None if req.prompt.starts_with(JUDGE_PROMPT) => "faithful".into(),
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
                model_id: Some("stub-model".into()),
            },
        })
    }

    fn credential_fingerprint(&self) -> u64 {
        0
    }
}

#[test]
fn record_through_the_router_writes_a_live_cassette_that_replays_green() {
    let seed = committed();
    let seeded = seed["entries"].as_array().unwrap();
    let answers = seeded
        .iter()
        .map(|e| {
            let text = e["response"]["text"].as_str().unwrap().to_owned();
            (e["prompt_sha256"].as_str().unwrap().to_owned(), text)
        })
        .collect();
    let home = tempfile::tempdir().unwrap();
    let recorded = home.path().join("distill-corpus-v0.live.json");
    let recorder = Arc::new(CassetteRecorder::new(
        recorded.clone(),
        "maos".into(),
        "eval-distill-record".into(),
    ));
    let driver = CassetteRecordProvider::new(Arc::new(StubProvider(answers)), recorder.clone());
    let drivers = [("anthropic".to_owned(), Arc::new(driver) as Arc<_>)].into();
    let router = Arc::new(MultiProviderRouter::new(drivers, Some("anthropic".into())));

    // The composition reads workspace-relative paths, like the verb.
    std::env::set_current_dir(root()).unwrap();
    let run = evaluate(Completions::Record {
        router,
        cassette: recorded.clone(),
        flush: Box::new(move || recorder.flush().map(drop)),
    })
    .unwrap();
    assert!(run.failures.is_empty(), "{:?}", run.failures);
    let line: Value = serde_json::from_str(&run.line).unwrap();
    assert_eq!(line["judge_provenance"], serde_json::json!(["live-record"]));

    let cassette: Value =
        serde_json::from_str(&std::fs::read_to_string(&recorded).unwrap()).unwrap();
    assert_eq!(cassette["provenance"], "live-record");
    let entries = cassette["entries"].as_array().unwrap();
    assert!(entries.len() > seeded.len(), "surveys then judge verdicts");
    for (got, want) in entries.iter().zip(seeded) {
        assert_eq!(got["prompt_sha256"], want["prompt_sha256"]);
        assert_eq!(got["prompt_len"], want["prompt_len"]);
    }

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
}
