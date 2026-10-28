#![forbid(unsafe_code)]

//! Story 19-1 AC5 (D-19-1-14 A) — `maos eval halt --class orchestrator`, the
//! Orchestrator class gate, at process level in `workspace-test-suite`.
//!
//! Every case runs `env!("CARGO_BIN_EXE_maos")` with a fresh
//! `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, no provider key and
//! `MAOS_OLLAMA_URL=skip`; no test spawns cargo.
//!
//! - AC5.1: the committed seed replays 30 completions through
//!   `Orchestrator::judge_ambiguity` and holds overall recall >= 0.70 and
//!   precision >= 0.85, `model_driven: true`, with a verifiable sidecar.
//! - AC5.5: the miss twin (five expected-halt rows judged clear) exits 1
//!   naming the missed floor and publishes no sidecar — the standing negative
//!   control.
//! - AC5.4 (D-19-1-15 A, `deferred-work.md:1178`): the corpus and the
//!   committed cassette resolve against the compile-time workspace root, so
//!   the run is the same from a cwd that is not the workspace root.
//! - Strict replay: a drifted prompt hash and a 29-entry cassette exit 1
//!   naming the row.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use maos_eval::halt_replay::sidecar;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CORPUS: &str = "spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.jsonl";
const SEED: &str = "crates/maos-journey-test/cassettes/j1/orchestrator-halt-corpus-v0.1.json";
const MISS_TWIN: &str = "spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.miss-twin.json";
const VERSION: &str = "0.8.0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn eval(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(cwd)
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
    cmd.output().expect("spawn maos")
}

/// `maos eval halt --class orchestrator [--cassette <c>]` from the root.
fn orchestrator(home: &Path, cassette: Option<&str>) -> Output {
    let mut args = vec!["halt", "--class", "orchestrator"];
    args.extend(cassette.into_iter().flat_map(|c| ["--cassette", c]));
    eval(home, &root(), &args)
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

fn sidecar_path(home: &Path) -> PathBuf {
    home.join(".local/share/maos/registry/spirits/orchestrator")
        .join(VERSION)
        .join("benchmark.signed.json")
}

fn sha256_file(rel: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(std::fs::read(root().join(rel)).unwrap())
    )
}

/// A variant of the seed cassette in `dir`.
fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    let mut cassette: Value =
        serde_json::from_str(&std::fs::read_to_string(root().join(SEED)).unwrap()).unwrap();
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, cassette.to_string()).unwrap();
    path.display().to_string()
}

/// The seed run's line and the class floors (AC5.1).
fn assert_the_seed_holds_every_floor(line: &Value) {
    assert_eq!(line["class"], "orchestrator");
    assert_eq!(line["spirit_version"], VERSION);
    assert_eq!(line["corpus"], CORPUS);
    assert_eq!(line["corpus_sha256"], sha256_file(CORPUS));
    assert_eq!(line["cassette"], SEED);
    assert_eq!(line["cassette_sha256"], sha256_file(SEED));
    assert_eq!(line["provenance"], "seed");
    assert_eq!(line["completions_consumed"], 30);
    assert_eq!(line["model_driven"], true);
    assert_eq!(line["model_driven_reason"], Value::Null);
    assert_eq!(line["floors"]["halt_recall_overall"], 0.7);
    assert_eq!(line["floors"]["halt_precision_overall"], 0.85);
    assert_eq!(line["floors"]["halt_recall_calendar_conflict"], Value::Null);
    let score = &line["score"];
    assert_eq!(score["rows"], 30);
    assert_eq!(score["expected_halts"], 15);
    assert_eq!(score["true_positives"], 15);
    assert_eq!(score["false_positives"], 0);
    assert_eq!(score["false_negatives"], 0);
    assert_eq!(score["halt_recall_overall"], 1.0);
    assert_eq!(score["halt_precision_overall"], 1.0);
    assert_eq!(score["halt_recall_calendar_conflict"], Value::Null);
    assert_eq!(line["missed_floors"], serde_json::json!([]));
    assert_eq!(line["pass"], true);
}

#[test]
fn the_orchestrator_class_replays_its_seed_and_holds_every_floor() {
    let home = tempfile::tempdir().unwrap();
    let out = orchestrator(home.path(), None);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let line = line(&out);
    assert_the_seed_holds_every_floor(&line);
    assert_eq!(line["signer"], "dev-seed");

    // The sidecar the run wrote verifies, and its body is the line's.
    let bytes = std::fs::read(sidecar_path(home.path())).unwrap();
    let dev_pubkey = maos_audit::sealed_export::derive_pubkey(&sidecar::DEV_SEED);
    let (body, _) = sidecar::verify(&bytes, &dev_pubkey, "orchestrator", VERSION).unwrap();
    let body = serde_json::to_value(body).unwrap();
    let mut expected = line.clone();
    expected.as_object_mut().unwrap().remove("event");
    assert_eq!(body, expected);
}

/// AC5.5 — the standing negative control: am01–am05 judged 0.2 leave 10 of
/// 15 expected halts, recall 0.667 < 0.70.
#[test]
fn the_miss_twin_exits_1_naming_the_missed_recall_floor() {
    let home = tempfile::tempdir().unwrap();
    let out = orchestrator(home.path(), Some(MISS_TWIN));
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["score"]["true_positives"], 10);
    assert_eq!(line["score"]["false_negatives"], 5);
    assert_eq!(line["score"]["halt_precision_overall"], 1.0);
    assert_eq!(line["completions_consumed"], 30);
    assert_eq!(line["pass"], false);
    assert_eq!(
        line["missed_floors"],
        serde_json::json!(["halt_recall_overall 0.667 below floor 0.70"])
    );
    let err = stderr(&out);
    assert!(
        err.contains("maos eval: halt_recall_overall 0.667 below floor 0.70"),
        "{err}"
    );
    assert!(
        err.contains("override cassette — numbers only, no sidecar written"),
        "{err}"
    );
    assert!(!sidecar_path(home.path()).exists());
}

/// D-19-1-15 A — the same committed run from a cwd that is not the
/// workspace root: a subdirectory of the checkout and a directory outside it.
#[test]
fn the_class_gate_runs_from_a_cwd_other_than_the_workspace_root() {
    let elsewhere = tempfile::tempdir().unwrap();
    let inside = root().join("crates/maos-bench/src");
    for cwd in [inside.as_path(), elsewhere.path()] {
        let home = tempfile::tempdir().unwrap();
        let out = eval(home.path(), cwd, &["halt", "--class", "orchestrator"]);
        assert_eq!(out.status.code(), Some(0), "{cwd:?}: {}", stderr(&out));
        assert_the_seed_holds_every_floor(&line(&out));
        assert!(sidecar_path(home.path()).is_file(), "{cwd:?}");
    }
}

#[test]
fn a_drifted_prompt_hash_exits_1_naming_the_row() {
    let home = tempfile::tempdir().unwrap();
    let cassette = variant(home.path(), |c| {
        c["entries"][0]["prompt_sha256"] = "ab".repeat(32).into();
    });
    let out = orchestrator(home.path(), Some(&cassette));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("row am01: pass refused") && err.contains("drift"),
        "{err}"
    );
    assert!(out.stdout.is_empty(), "a run without numbers prints none");
}

#[test]
fn a_29_entry_cassette_exits_1_naming_the_row() {
    let home = tempfile::tempdir().unwrap();
    let cassette = variant(home.path(), |c| {
        c["entries"].as_array_mut().unwrap().pop();
    });
    let out = orchestrator(home.path(), Some(&cassette));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("row cl15: pass refused") && err.contains("exhausted"),
        "{err}"
    );
    assert!(out.stdout.is_empty(), "a run without numbers prints none");
    assert!(!sidecar_path(home.path()).exists());
}

#[test]
fn the_usage_names_the_orchestrator_class() {
    let home = tempfile::tempdir().unwrap();
    let out = eval(home.path(), &root(), &["halt", "--class", "nosuch"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("--class <butler|researcher|orchestrator>"),
        "{}",
        stderr(&out)
    );
}
