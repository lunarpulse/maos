#![forbid(unsafe_code)]

//! Story 18-2 — `maos eval halt` at process level (exit line 2).
//!
//! Every case runs `env!("CARGO_BIN_EXE_maos")` from the workspace root with
//! a fresh `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, no provider key and
//! `MAOS_OLLAMA_URL=skip`; no test spawns cargo.
//!
//! - AC1: the seed cassette exits 0 with the numbers and a verifiable
//!   sidecar; the drift-blind and precision-planted cassettes exit 1 naming
//!   the missed floor; a 29-entry cassette and a drifted prompt hash exit 1
//!   naming the row; usage errors exit 2.
//! - AC3: `record` without a provider refuses and writes nothing; a record
//!   through a router whose `anthropic` driver is a local stub writes 30
//!   real-hash `live-record` entries that then replay green.
//! - AC4: the Researcher class is published `model_driven: false`.
//! - Review (2026-10-25): a replay honours `MAOS_REPLAY_CASSETTE` (BH-5);
//!   only the committed cassette or a record run publishes the sidecar
//!   (D-18-2-R2); a duplicated `scenario_id` is refused (EC-4).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_bench::halt_eval::{evaluate, parse_args, Completions};
use maos_bin::cassette_replay::{CassetteRecordProvider, CassetteRecorder};
use maos_domain::ports::inference::{
    InferenceRequest, InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
};
use maos_eval::halt_replay::sidecar;
use maos_kernel_core::inference::router::MultiProviderRouter;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CORPUS: &str = "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl";
const SEED: &str = "crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json";
const DRIFT_BLIND: &str = "spirits/butler/tests/fixtures/halt-corpus-v0.3.drift-blind.json";
const PRECISION_PLANTED: &str =
    "spirits/butler/tests/fixtures/halt-corpus-v0.3.precision-planted.json";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn eval(home: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(root())
        .arg("eval")
        .args(args)
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"))
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_SECRETS_BACKEND", "env");
    for var in [
        "MAOS_ANTHROPIC_API_KEY",
        "MAOS_OPENAI_API_KEY",
        "MAOS_INFERENCE_MODE",
        "MAOS_REPLAY_CASSETTE",
        "MAOS_REPLAY_STRICT",
        "MAOS_EVAL_PRODUCER_SEED",
        "MAOS_EVAL_PRODUCER_PUBKEY",
    ] {
        cmd.env_remove(var);
    }
    cmd.envs(env.iter().copied());
    cmd.output().expect("spawn maos")
}

fn butler(home: &Path, cassette: Option<&str>) -> Output {
    match cassette {
        Some(cassette) => eval(
            home,
            &["halt", "--class", "butler", "--cassette", cassette],
            &[],
        ),
        None => eval(home, &["halt", "--class", "butler"], &[]),
    }
}

/// The single stdout line, parsed.
fn line(out: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one stdout line: {stdout}");
    let line: Value = serde_json::from_str(lines[0]).expect("a JSON line");
    assert_eq!(line["event"], "eval_halt");
    line
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn sidecar_path(home: &Path, class: &str, version: &str) -> PathBuf {
    home.join(".local/share/maos/registry/spirits")
        .join(class)
        .join(version)
        .join("benchmark.signed.json")
}

fn sha256_file(rel: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(std::fs::read(root().join(rel)).unwrap())
    )
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn exit_line_2_replays_the_seed_cassette_and_holds_every_floor() {
    let home = tempfile::tempdir().unwrap();
    let out = butler(home.path(), None);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["class"], "butler");
    assert_eq!(line["score"]["halt_recall_calendar_conflict"], 1.0);
    assert_eq!(line["score"]["halt_recall_overall"], 1.0);
    assert_eq!(line["score"]["halt_precision_overall"], 1.0);
    assert_eq!(line["score"]["expected_halts"], 15);
    assert_eq!(line["completions_consumed"], 30);
    assert_eq!(line["provenance"], "seed");
    assert_eq!(line["model_driven"], true);
    assert_eq!(line["signer"], "dev-seed");
    assert_eq!(
        line["halt_recall_calendar_conflict_basis"],
        "arithmetic: belief_variance (18-1 AC1.1)"
    );
    assert_eq!(line["pass"], true);
    assert_eq!(line["corpus_sha256"], sha256_file(CORPUS));
    assert_eq!(line["cassette_sha256"], sha256_file(SEED));

    // AC5: the sidecar the run wrote verifies, and its body is the line's.
    let bytes = std::fs::read(sidecar_path(home.path(), "butler", "0.3.0")).unwrap();
    // TIA-3: the child signed with the dev seed; verify against its key, not the ambient env.
    let dev_pubkey = maos_audit::sealed_export::derive_pubkey(&sidecar::DEV_SEED);
    let (body, _) = sidecar::verify(&bytes, &dev_pubkey, "butler", "0.3.0").unwrap();
    let body = serde_json::to_value(body).unwrap();
    let mut expected = line.clone();
    expected.as_object_mut().unwrap().remove("event");
    assert_eq!(body, expected);
}

#[test]
fn the_drift_blind_negative_control_misses_overall_recall() {
    let home = tempfile::tempdir().unwrap();
    let out = butler(home.path(), Some(DRIFT_BLIND));
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["score"]["halt_recall_calendar_conflict"], 1.0);
    assert_eq!(line["score"]["halt_recall_overall"], 0.8);
    assert_eq!(line["pass"], false);
    assert!(
        stderr(&out).contains("halt_recall_overall 0.800 below floor 0.90"),
        "{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("override cassette — numbers only, no sidecar written"),
        "{}",
        stderr(&out)
    );
    assert!(
        !sidecar_path(home.path(), "butler", "0.3.0").exists(),
        "an override cassette publishes no class sidecar (D-18-2-R2)"
    );
}

#[test]
fn an_env_cassette_is_replayed_and_publishes_no_sidecar() {
    let home = tempfile::tempdir().unwrap();
    let env = [("MAOS_REPLAY_CASSETTE", DRIFT_BLIND)];
    let out = eval(home.path(), &["halt", "--class", "butler"], &env);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(line(&out)["score"]["halt_recall_overall"], 0.8);
    assert!(
        stderr(&out).contains("override cassette — numbers only, no sidecar written"),
        "{}",
        stderr(&out)
    );
    assert!(!sidecar_path(home.path(), "butler", "0.3.0").exists());
}

#[test]
fn the_committed_cassette_named_explicitly_publishes_the_sidecar() {
    let home = tempfile::tempdir().unwrap();
    let seed = format!("./{SEED}");
    let out = eval(
        home.path(),
        &["halt", "--class", "butler"],
        &[("MAOS_REPLAY_CASSETTE", &seed)],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(sidecar_path(home.path(), "butler", "0.3.0").is_file());
}

#[test]
fn a_duplicate_scenario_id_is_refused() {
    use maos_eval::halt_replay::{score_class, ClassFloors};
    use maos_eval::onboarding_gate_corpus::{OnbScenario, OnboardingCorpus};
    let row = |id: &str| OnbScenario {
        scenario_id: id.into(),
        calendar_conflict: true,
        expected_halt: true,
        observed_halt: false,
    };
    let corpus = OnboardingCorpus {
        meta: None,
        scenarios: vec![row("cc01"), row("cc01"), row("cc02")],
    };
    let verdicts = [("cc01".to_owned(), true), ("cc02".to_owned(), true)].into();
    let floors = ClassFloors {
        halt_recall_calendar_conflict: Some(0.9),
        halt_recall_overall: 0.9,
        halt_precision_overall: 0.85,
    };
    let err = score_class(&corpus, "", &verdicts, &floors).unwrap_err();
    assert!(
        err.contains("3 rows, 2 verdicts — a duplicate scenario_id"),
        "{err}"
    );
}

#[test]
fn the_precision_planted_cassette_misses_precision() {
    let home = tempfile::tempdir().unwrap();
    let out = butler(home.path(), Some(PRECISION_PLANTED));
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(line(&out)["score"]["false_positives"], 3);
    assert!(
        stderr(&out).contains("halt_precision_overall 0.833 below floor 0.85"),
        "{}",
        stderr(&out)
    );
}

/// A variant of the seed cassette in `dir`.
fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    let mut cassette = read_json(&root().join(SEED));
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, cassette.to_string()).unwrap();
    path.display().to_string()
}

#[test]
fn a_29_entry_cassette_exits_1_naming_the_row() {
    let home = tempfile::tempdir().unwrap();
    let cassette = variant(home.path(), |c| {
        c["entries"].as_array_mut().unwrap().pop();
    });
    let out = butler(home.path(), Some(&cassette));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("row tn09: pass refused") && err.contains("exhausted"),
        "{err}"
    );
    assert!(out.stdout.is_empty(), "a run without numbers prints none");
    assert!(!sidecar_path(home.path(), "butler", "0.3.0").exists());
}

#[test]
fn a_drifted_prompt_hash_exits_1() {
    let home = tempfile::tempdir().unwrap();
    let cassette = variant(home.path(), |c| {
        c["entries"][0]["prompt_sha256"] = "ab".repeat(32).into();
    });
    let out = butler(home.path(), Some(&cassette));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("row cc01: pass refused") && err.contains("drift"),
        "{err}"
    );
}

#[test]
fn usage_errors_exit_2() {
    let home = tempfile::tempdir().unwrap();
    for (args, env) in [
        (vec!["halt", "--class", "nosuch"], vec![]),
        (vec!["halt"], vec![]),
        (vec!["status"], vec![]),
        (
            vec!["halt", "--class", "researcher", "--cassette", SEED],
            vec![],
        ),
        (
            vec!["halt", "--class", "butler"],
            vec![("MAOS_INFERENCE_MODE", "live")],
        ),
    ] {
        let out = eval(home.path(), &args, &env);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).starts_with("maos eval: "), "{}", stderr(&out));
    }
}

#[test]
fn the_researcher_class_is_published_as_not_model_driven() {
    let home = tempfile::tempdir().unwrap();
    let out = eval(home.path(), &["halt", "--class", "researcher"], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["model_driven"], false);
    assert!(line["model_driven_reason"]
        .as_str()
        .unwrap()
        .contains("claim-metadata arithmetic"));
    assert_eq!(line["completions_consumed"], Value::Null);
    assert_eq!(line["provenance"], Value::Null);
    assert_eq!(line["floors"]["halt_recall_overall"], 0.7);
    assert_eq!(line["score"]["true_positives"], 14);
    assert_eq!(line["score"]["false_negatives"], 1);
    assert_eq!(line["score"]["halt_precision_overall"], 1.0);
    assert!(sidecar_path(home.path(), "researcher", "0.5.0").is_file());
}

#[test]
fn record_without_a_provider_refuses_and_writes_no_cassette() {
    let home = tempfile::tempdir().unwrap();
    let recorded = home.path().join("recorded.json");
    let out = eval(
        home.path(),
        &["halt", "--class", "butler"],
        &[
            ("MAOS_INFERENCE_MODE", "record"),
            ("MAOS_REPLAY_CASSETTE", recorded.to_str().unwrap()),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("row cc01: pass refused"),
        "{}",
        stderr(&out)
    );
    assert!(
        !recorded.exists(),
        "a refused record run writes no cassette"
    );
}

/// A local provider answering each prompt with the seed's completion for it.
struct StubProvider(Vec<(String, String)>);

impl maos_providers::Provider for StubProvider {
    fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, maos_providers::provider::ProviderError> {
        let hash = format!("{:x}", Sha256::digest(req.prompt.as_bytes()));
        let (_, text) = self
            .0
            .iter()
            .find(|(h, _)| *h == hash)
            .expect("a seeded prompt");
        Ok(InferenceResponse {
            text: text.clone(),
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
fn record_through_the_router_writes_a_cassette_that_replays_green() {
    let seed = read_json(&root().join(SEED));
    let answers = seed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let text = e["response"]["text"].as_str().unwrap().to_owned();
            (e["prompt_sha256"].as_str().unwrap().to_owned(), text)
        })
        .collect();
    let home = tempfile::tempdir().unwrap();
    let recorded = home.path().join("recorded.json");
    let recorder = Arc::new(CassetteRecorder::new(
        recorded.clone(),
        "maos".into(),
        "eval-record".into(),
    ));
    let driver = CassetteRecordProvider::new(Arc::new(StubProvider(answers)), recorder.clone());
    let drivers = [("anthropic".to_owned(), Arc::new(driver) as Arc<_>)].into();
    let router = Arc::new(MultiProviderRouter::new(drivers, Some("anthropic".into())));

    // The composition reads workspace-relative corpus paths, like the verb.
    std::env::set_current_dir(root()).unwrap();
    // TIA-3: the in-process run signs with the dev seed whatever the host exports.
    std::env::remove_var("MAOS_EVAL_PRODUCER_SEED");
    std::env::remove_var("MAOS_EVAL_PRODUCER_PUBKEY");
    let request = parse_args(
        &["halt".into(), "--class".into(), "butler".into()],
        Some("record"),
        None,
    )
    .unwrap();
    let run = evaluate(
        &request,
        Some(Completions::Record {
            router,
            cassette: recorded.clone(),
            flush: Box::new(move || recorder.flush().map(drop)),
        }),
    )
    .unwrap();
    assert!(run.pass, "{:?}", run.missed_floors);

    let cassette = read_json(&recorded);
    assert_eq!(cassette["provenance"], "live-record");
    let entries = cassette["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 30);
    for (got, want) in entries.iter().zip(seed["entries"].as_array().unwrap()) {
        assert_eq!(got["prompt_sha256"], want["prompt_sha256"]);
        assert_eq!(got["prompt_len"], want["prompt_len"]);
    }

    let out = butler(home.path(), Some(recorded.to_str().unwrap()));
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(line(&out)["provenance"], "live-record");
}
