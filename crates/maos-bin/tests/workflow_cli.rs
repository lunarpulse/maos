#![cfg(feature = "network")]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};
use tempfile::tempdir;

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/workflows")
        .join(name)
}

fn workflow(
    home: &Path,
    verb: &str,
    first: impl AsRef<OsStr>,
    second: impl AsRef<OsStr>,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maos"))
        .current_dir(home)
        .env_clear()
        .env("HOME", home)
        .env("MAOS_HOME", home.join("maos"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .args(["workflow", verb])
        .arg(first)
        .arg(second)
        .output()
        .unwrap()
}

#[track_caller]
fn check_status(output: &Output, success: bool) {
    assert!(
        output.status.code().is_some(),
        "terminated by signal: {output:?}"
    );
    assert_eq!(output.status.success(), success, "{output:?}");
}

// Ignore unrelated log records. A result may span lines or be followed by another record.
#[track_caller]
fn result(output: Output, success: bool, keys: &[&str]) -> Value {
    check_status(&output, success);
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    std::iter::once(0)
        .chain(stdout.match_indices('\n').map(|(i, _)| i + 1))
        .map(|start| stdout[start..].trim_start_matches([' ', '\t']))
        .filter(|record| record.starts_with('{'))
        .filter_map(|record| {
            serde_json::Deserializer::from_str(record)
                .into_iter::<Value>()
                .next()?
                .ok()
        })
        .find(|record| keys.iter().all(|key| record.get(*key).is_some()))
        .unwrap_or_else(|| panic!("no workflow result with {keys:?} in {stdout}"))
}

fn plan(home: &Path, topology: &str, file: &str) -> Value {
    result(
        workflow(home, "plan", example(topology), file),
        true,
        &["digest", "previous", "changes"],
    )
}

#[track_caller]
fn assert_run(home: &Path, id: &str, input: &str, outcome: &str, steps: &[&str]) -> Value {
    let input = example(input);
    let run = result(
        workflow(home, "run", id, &input),
        true,
        &["artifact", "outcome", "path", "run_id"],
    );
    let artifact: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    assert_eq!(run["artifact"], artifact);
    assert_eq!(run["outcome"], outcome);
    let path = run["path"].as_array().unwrap();
    assert_eq!(path.len(), steps.len(), "{path:?}");
    for (hop, step) in path.iter().zip(steps) {
        assert_eq!(
            hop.get("step").unwrap_or(hop).as_str(),
            Some(*step),
            "{path:?}"
        );
    }
    run
}

#[track_caller]
fn assert_eval(home: &Path, id: &str, suite: &Path, passed: usize, total: usize) {
    let report = result(
        workflow(home, "eval", id, suite),
        passed == total,
        &["passed", "total", "cases"],
    );
    assert_eq!(report["passed"], json!(passed), "{report}");
    assert_eq!(report["total"], json!(total), "{report}");
    assert_eq!(report["cases"].as_array().unwrap().len(), total, "{report}");
}

fn assert_logged_handoffs(home: &Path, run: &Value) {
    let db = rusqlite::Connection::open_with_flags(
        home.join("maos/audit/transparency.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut query = db
        .prepare(
            "SELECT intent, payload_redacted FROM transparency_log
         WHERE intent IN ('workflow:dispatch', 'workflow:handoff', 'workflow:completed')
         ORDER BY rowid",
        )
        .unwrap();
    let events: Vec<_> = query
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .unwrap()
        .map(|row| {
            let (intent, payload) = row.unwrap();
            (intent, serde_json::from_slice::<Value>(&payload).unwrap())
        })
        .filter(|(_, payload)| payload["run_id"] == run["run_id"])
        .collect();
    let path = run["path"].as_array().unwrap();
    assert_eq!(events.len(), 2 * path.len() + 1);
    for (i, hop) in path.iter().enumerate() {
        let (dispatch, input) = &events[2 * i];
        let (handoff, receipt) = &events[2 * i + 1];
        assert_eq!(dispatch, "workflow:dispatch");
        assert_eq!(input["step"], hop["step"]);
        assert_eq!(audit_digest(&input["input_digest"]), hop["input_digest"]);
        assert_eq!(handoff, "workflow:handoff");
        assert_eq!(
            audit_digest(&receipt["receipt"]["output_digest"]),
            hop["output_digest"]
        );
        if i + 1 < path.len() {
            assert_eq!(hop["output_digest"], path[i + 1]["input_digest"]);
        }
    }
    assert_eq!(events.last().unwrap().0, "workflow:completed");
    assert_eq!(events.last().unwrap().1["outcome"], run["outcome"]);
}

fn audit_digest(value: &Value) -> String {
    value
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap()
        .replace('.', "")
}

#[test]
fn approved_revisions_control_dispatch_and_schema() {
    let temp = tempdir().unwrap();
    let home = temp.path();
    fs::create_dir(home.join("maos")).unwrap();
    let catalog = home.join("maos/workflow-catalog.toml");
    let original = fs::read_to_string(example("catalog.toml")).unwrap();
    fs::write(&catalog, &original).unwrap();

    let invoice = "invoice-review";
    let baseline = plan(home, "invoice-baseline.toml", "baseline.json");
    assert!(!home.join("maos/workflow-deployments.sqlite").exists());
    assert!(!home.join("maos/audit/transparency.sqlite").exists());
    let digest_a = baseline["digest"].as_str().unwrap();
    check_status(&workflow(home, "apply", "baseline.json", digest_a), true);
    let assert_baseline = || {
        assert_run(
            home,
            invoice,
            "invoice-input.json",
            "approval_recommended",
            &["intake", "review"],
        );
    };
    assert_baseline();
    assert_eval(home, invoice, &example("invoice-baseline-cases.json"), 2, 2);

    let revised = plan(home, "invoice-revised.toml", "revised.json");
    let digest_b = revised["digest"].as_str().unwrap();
    assert_ne!(digest_a, digest_b);
    assert!(!revised["previous"].is_null(), "{revised}");
    assert_baseline();

    // Only B depends on this registration. A must survive the refused apply.
    let mut edited: toml::Table = toml::from_str(&original).unwrap();
    edited
        .get_mut("agents")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .remove("duplicate-check")
        .unwrap();
    fs::write(&catalog, toml::to_string(&edited).unwrap()).unwrap();
    let refusal = workflow(home, "apply", "revised.json", digest_b);
    check_status(&refusal, false);
    assert!(String::from_utf8_lossy(&refusal.stderr).contains("unauthorized agent duplicate-check"));
    assert_baseline();
    fs::write(&catalog, original).unwrap();
    check_status(&workflow(home, "apply", "revised.json", digest_b), true);
    let held = assert_run(
        home,
        invoice,
        "invoice-input.json",
        "held",
        &["intake", "duplicate-check"],
    );
    assert_logged_handoffs(home, &held);
    let stale = workflow(home, "apply", "baseline.json", digest_a);
    check_status(&stale, false);
    assert!(String::from_utf8_lossy(&stale.stderr).contains("active revision changed"));
    assert_eval(home, invoice, &example("invoice-revised-cases.json"), 3, 3);

    // Correct terminal outcome, with one wrong route and one wrong specialist label.
    let wrong_path = home.join("wrong-path.json");
    let duplicate: Value =
        serde_json::from_slice(&fs::read(example("invoice-input.json")).unwrap()).unwrap();
    let cases = json!([
        {"id": "unexpected-review", "input": duplicate, "expect": {
            "outcome": "held", "path": ["intake", "duplicate-check", "review"]
        }},
        {"id": "unexpected-specialist-label", "input": duplicate, "expect": {
            "outcome": "held", "path": ["intake", "duplicate-check"],
            "step_outcomes": ["ready", "clear"]
        }}
    ]);
    fs::write(&wrong_path, serde_json::to_vec(&cases).unwrap()).unwrap();
    assert_eval(home, invoice, &wrong_path, 0, 2);

    let library = plan(home, "book-return.toml", "library.json");
    check_status(
        &workflow(
            home,
            "apply",
            "library.json",
            library["digest"].as_str().unwrap(),
        ),
        true,
    );
    assert_run(
        home,
        "book-return",
        "book-return-input.json",
        "send_to_repair",
        &["inspect"],
    );
    assert_eval(
        home,
        "book-return",
        &example("book-return-cases.json"),
        2,
        2,
    );
    check_status(
        &workflow(home, "run", "book-return", example("invoice-input.json")),
        false,
    );
}
