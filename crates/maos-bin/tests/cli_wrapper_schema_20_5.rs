#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 20-5 AC3 — a Worker manifest's `[cli_wrapper]` table carries a schema version
//! (`manifest_schema_version`, absent means 1, window `1..=1`) that is read and checked
//! BEFORE the strict parse, through the real `maos run` composition root.
//!
//! "Admitted" is observed as the run reaching the stage after the `[cli_wrapper]` parse
//! (the host-grant tier gate refuses the nonexistent CLI image there); "refused" is the
//! typed schema refusal, with no `cli_wrapper_loaded` event and no stage after it.

use std::path::PathBuf;
use std::process::Command;

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-20-5-{tag}-{nanos}"));
    std::fs::create_dir_all(&path).unwrap();
    Scratch(path)
}

fn worker_manifest(cli_wrapper_keys: &str) -> String {
    format!(
        r#"[cli_wrapper]
{cli_wrapper_keys}
command = "no-such-cli-20-5"
argv_prefix = ["--maos-worker"]
output_shape_version = "1.0.0"
recovery_policy = "respawn_fresh"

[cli_wrapper.posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
shutdown_signal = "SIGTERM"

[sandbox]
tier = "T3"

[author]
name = "MAOS Project"
homepage = "https://github.com/lunarpulse/maos"
"#
    )
}

struct Outcome {
    stdout: String,
    stderr: String,
}

fn run(manifest: &str, tag: &str) -> Outcome {
    let dir = scratch(tag);
    let path = dir.0.join("worker.toml");
    std::fs::write(&path, manifest).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", path.to_str().unwrap(), "--once"])
        .env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", dir.0.join("xdg"))
        .env_remove("MAOS_HOST_GRANTS")
        .env_remove("MAOS_INFERENCE_MODE")
        .current_dir(workspace_root())
        .output()
        .expect("run maos");
    Outcome {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Past the `[cli_wrapper]` parse and the schema check: the tier gate speaks next.
fn assert_admitted(outcome: &Outcome) {
    assert!(
        outcome.stderr.contains("tier not granted"),
        "the manifest must reach the host-grant tier gate; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("manifest_schema_version"),
        "an admitted schema version is not mentioned; stderr:\n{}",
        outcome.stderr
    );
}

/// The typed schema refusal, before any later stage.
fn assert_refused_as_unsupported(outcome: &Outcome, declared: &str) {
    let needle = format!("manifest_schema_version {declared} unsupported");
    // `maos run` prints its error through `{:?}`, which escapes the quotes of a string value.
    let shown = outcome.stderr.replace("\\\"", "\"");
    assert!(
        shown.contains(&needle) && shown.contains("1..=1"),
        "expected the typed refusal naming 1..=1 ({needle}); stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("unknown field"),
        "a too-new manifest is refused as too new, never as an unknown field; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("tier not granted"),
        "the refusal comes before the tier gate; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stdout.contains("cli_wrapper_loaded"),
        "no load event past the refusal; stdout:\n{}",
        outcome.stdout
    );
}

#[test]
fn an_absent_version_is_version_one_and_admitted() {
    assert_admitted(&run(&worker_manifest(""), "absent"));
}

#[test]
fn version_one_is_admitted() {
    assert_admitted(&run(&worker_manifest("manifest_schema_version = 1"), "one"));
}

#[test]
fn a_newer_manifest_with_an_unknown_key_is_refused_as_too_new_not_as_an_unknown_field() {
    let outcome = run(
        &worker_manifest("manifest_schema_version = 2\nfrobnicate = true"),
        "two",
    );
    assert_refused_as_unsupported(&outcome, "2");
}

#[test]
fn a_version_outside_the_window_or_of_the_wrong_type_is_refused_typed() {
    for (value, shown) in [
        ("0", "0"),
        ("-1", "-1"),
        ("\"x\"", "\"x\""),
        ("1.5", "1.5"),
        ("true", "true"),
        ("4294967297", "4294967297"),
        ("[1]", "[1]"),
    ] {
        let outcome = run(
            &worker_manifest(&format!("manifest_schema_version = {value}")),
            "bad",
        );
        assert_refused_as_unsupported(&outcome, shown);
    }
}

#[test]
fn every_other_key_stays_strict_when_the_version_is_valid() {
    let outcome = run(
        &worker_manifest("manifest_schema_version = 1\nfrobnicate = true"),
        "strict",
    );
    assert!(
        outcome.stderr.contains("unknown field"),
        "an unknown key under a valid version is still refused; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("tier not granted"),
        "{}",
        outcome.stderr
    );
}

#[test]
fn every_shipped_worker_manifest_declares_the_version_explicitly() {
    let dir = workspace_root().join("spirits/worker");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !(name.starts_with("manifest") && name.ends_with(".toml")) {
            continue;
        }
        let root: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let declared = root["cli_wrapper"]
            .get("manifest_schema_version")
            .and_then(toml::Value::as_integer);
        assert_eq!(
            declared,
            Some(1),
            "{name} must declare manifest_schema_version = 1"
        );
        seen += 1;
    }
    assert_eq!(seen, 5, "spirits/worker ships five Worker manifests");
}
