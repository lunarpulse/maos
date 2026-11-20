#![cfg(feature = "network")]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::{tempdir, TempDir};

const ID: &str = "invoice-evidence";
const MISSING: &str = "invoice-evidence-missing.json";
const PRESENT: &str = "invoice-evidence-present.json";
const MISSING_TAPE: &str = "invoice-evidence-missing.cassette.json";
const PRESENT_TAPE: &str = "invoice-evidence-present.cassette.json";
const INFERENCE: &str = "infer:anthropic->anthropic:claude-haiku-4-5-20251001";

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/workflows")
        .join(name)
}

fn setup() -> TempDir {
    let dir = tempdir().unwrap();
    let home = dir.path().join("maos");
    fs::create_dir(&home).unwrap();
    fs::copy(example("catalog.toml"), home.join("workflow-catalog.toml")).unwrap();
    dir
}

fn workflow(
    dir: &Path,
    verb: &str,
    first: impl AsRef<OsStr>,
    second: impl AsRef<OsStr>,
    cassette: Option<&Path>,
) -> Output {
    workflow_command(dir, verb, first, second, cassette)
        .output()
        .unwrap()
}

fn workflow_command(
    dir: &Path,
    verb: &str,
    first: impl AsRef<OsStr>,
    second: impl AsRef<OsStr>,
    cassette: Option<&Path>,
) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(dir)
        .env_clear()
        .env("HOME", dir)
        .env("MAOS_HOME", dir.join("maos"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("MAOS_SECRETS_BACKEND", "env");
    if let Some(path) = cassette {
        cmd.env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_STRICT", "1")
            .env("MAOS_REPLAY_CASSETTE", path);
    }
    cmd.args(["workflow", verb]).arg(first).arg(second);
    cmd
}

#[track_caller]
fn result(out: Output, field: &str) -> Value {
    assert!(out.status.success(), "{out:?}");
    record(&out, field)
}

#[track_caller]
fn record(out: &Output, field: &str) -> Value {
    std::str::from_utf8(&out.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|record| record.get(field).is_some())
        .unwrap_or_else(|| panic!("missing {field}: {out:?}"))
}

fn deploy(dir: &Path, topology: &str, plan_file: &str) -> Value {
    let plan = result(
        workflow(dir, "plan", example(topology), plan_file, None),
        "digest",
    );
    let apply = workflow(
        dir,
        "apply",
        plan_file,
        plan["digest"].as_str().unwrap(),
        Some(&example(MISSING_TAPE)),
    );
    assert!(apply.status.success(), "{apply:?}");
    plan
}

fn audit(dir: &Path) -> Connection {
    Connection::open_with_flags(
        dir.join("maos/audit/transparency.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
}

// Scope evidence to this child, including refusals that never produce a CLI run report.
fn watermark(db: &Connection) -> i64 {
    db.query_row(
        "SELECT coalesce(max(rowid), 0) FROM transparency_log",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn trace(db: &Connection, after: i64) -> Vec<(String, Value)> {
    let mut stmt = db
        .prepare(
            "SELECT intent, payload_redacted FROM transparency_log
             WHERE rowid > ?1 AND intent LIKE 'workflow:%' ORDER BY rowid",
        )
        .unwrap();
    stmt.query_map([after], |r| Ok((r.get(0)?, r.get::<_, Vec<u8>>(1)?)))
        .unwrap()
        .map(|row| {
            let (intent, bytes) = row.unwrap();
            (intent, serde_json::from_slice(&bytes).unwrap())
        })
        .collect()
}

fn digest(value: &Value) -> String {
    value
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap()
        .replace('.', "")
}

fn assert_finding(dir: &Path, plan: &Value, present: bool, terminal: &str) {
    let (input, tape, label, quote) = if present {
        (
            PRESENT,
            PRESENT_TAPE,
            "documented",
            "Purchase order PO-921 is attached.",
        )
    } else {
        (
            MISSING,
            MISSING_TAPE,
            "missing_order",
            "No purchase order was supplied.",
        )
    };
    let db = audit(dir);
    let before = watermark(&db);
    let report = result(
        workflow(dir, "run", ID, example(input), Some(&example(tape))),
        "run_id",
    );
    assert_eq!(report["revision"], plan["digest"]);
    assert_eq!(report["outcome"], terminal);
    assert_eq!(
        report["artifact"],
        json!({"purchase_order_present": present, "quote": quote})
    );

    let events = trace(&db, before);
    let intents: Vec<_> = events.iter().map(|(intent, _)| intent.as_str()).collect();
    assert_eq!(
        intents,
        [
            "workflow:dispatch",
            "workflow:handoff",
            "workflow:dispatch",
            "workflow:handoff",
            "workflow:completed",
        ]
    );
    let path = report["path"].as_array().unwrap();
    assert_eq!(path.len(), 2);
    for (_, payload) in &events {
        assert_eq!(payload["run_id"], report["run_id"]);
        assert_eq!(digest(&payload["revision"]), report["revision"]);
    }
    for (i, step) in ["extract", "review"].iter().enumerate() {
        let dispatch = &events[2 * i].1;
        let handoff = &events[2 * i + 1].1;
        assert_eq!(dispatch["step"], *step);
        assert_eq!(handoff["receipt"]["step"], *step);
        assert_eq!(path[i]["step"], *step);
        assert!(dispatch["invocation_id"].is_string());
        assert_eq!(dispatch["invocation_id"], handoff["invocation_id"]);
        assert_eq!(digest(&dispatch["input_digest"]), path[i]["input_digest"]);
        for field in ["input_digest", "output_digest"] {
            assert_eq!(digest(&handoff["receipt"][field]), path[i][field]);
        }
    }
    let produced = &events[1].1["receipt"];
    let reviewed = &events[3].1["receipt"];
    assert_ne!(events[0].1["input_digest"], produced["output_digest"]);
    assert_eq!(produced["output_digest"], events[2].1["input_digest"]);
    assert_eq!(events[1].1["recipient"], "review");
    assert_eq!(reviewed["from_step"], "extract");
    assert_eq!(produced["outcome"], "finding_ready");
    assert_eq!(reviewed["outcome"], label);
    assert_eq!(reviewed["route"], json!({"finish":terminal}));
    assert_eq!(events[4].1["outcome"], terminal);

    // Infer rows are emitted by the capability-verifying adapter, not the workflow reporter.
    let calls: Vec<String> = db
        .prepare(
            "SELECT intent FROM transparency_log WHERE rowid > ?1 AND intent LIKE 'infer:%'
             AND capability_token IS NOT NULL AND spirit_pid = (
                 SELECT spirit_pid FROM transparency_log WHERE rowid > ?1 AND intent = 'workflow:dispatch'
                 ORDER BY rowid LIMIT 1)",
        )
        .unwrap()
        .query_map([before], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(calls, [INFERENCE]);
}

#[test]
fn approved_reroute_changes_only_the_missing_order_terminal() {
    let temp = setup();
    let dir = temp.path();
    let base = deploy(dir, "invoice-evidence.toml", "base.json");
    assert_finding(dir, &base, false, "request_order");
    let revised = deploy(dir, "invoice-evidence-reroute.toml", "revised.json");
    assert_eq!(revised["previous"], base["digest"]);
    assert_ne!(revised["digest"], base["digest"]);
    assert_eq!(revised["changes"]["changed"], json!(["review"]));
    assert_finding(dir, &revised, false, "hold_for_procurement");
    assert_finding(dir, &revised, true, "ready_for_approval");
}

#[test]
fn comparison_grades_intermediate_evidence_without_activating_the_candidate() {
    let temp = setup();
    let dir = temp.path();
    let base = deploy(dir, "invoice-evidence.toml", "base.json");
    let candidate = result(
        workflow(
            dir,
            "plan",
            example("invoice-evidence-reroute.toml"),
            "candidate.json",
            None,
        ),
        "digest",
    );
    let candidate_digest = candidate["digest"].as_str().unwrap();
    let cases = example("invoice-evidence-compare-cases.json");
    let cassette = dir.join("comparison.cassette.json");
    let mut tape: Value =
        serde_json::from_slice(&fs::read(example(MISSING_TAPE)).unwrap()).unwrap();
    tape["entries"][0]["sequence"] = json!(0);
    let mut second = tape["entries"][0].clone();
    second["sequence"] = json!(1);
    tape["entries"].as_array_mut().unwrap().push(second);
    let write_tape = |tape: &Value| {
        let bytes = serde_json::to_vec(tape).unwrap();
        fs::write(&cassette, &bytes).unwrap();
        hex::encode(Sha256::digest(bytes))
    };
    let compare = |digest: &str, cases: &Path| {
        workflow_command(dir, "compare", "candidate.json", digest, Some(&cassette))
            .arg(cases)
            .output()
            .unwrap()
    };

    let digest = write_tape(&tape);
    let db = audit(dir);
    let before = watermark(&db);
    let deployment_file = dir.join("maos/workflow-deployments.sqlite");
    let deployment_before = fs::read(&deployment_file).unwrap();
    let good = result(compare(candidate_digest, &cases), "candidate_revision");
    assert_eq!(good["baseline_revision"], base["digest"]);
    assert_eq!(good["candidate_revision"], candidate_digest);
    assert_eq!(
        good["cases_sha256"],
        hex::encode(Sha256::digest(fs::read(&cases).unwrap()))
    );
    assert_eq!(good["cassette_sha256"], digest);
    assert_eq!(good["mode"], "strict_replay");
    assert_eq!(good["passed"], true);
    assert_eq!(good["baseline_passed"], 0);
    assert_eq!(good["candidate_passed"], 1);
    assert_eq!(
        good["cases"][0]["baseline"]["observed_outcome"],
        "request_order"
    );
    assert_eq!(
        good["cases"][0]["candidate"]["observed_outcome"],
        "hold_for_procurement"
    );

    // The shorter quote is verbatim and passes the runtime contract, but is not the gold label.
    // The reviewer still returns the right final answer.
    let mut finding: Value =
        serde_json::from_str(tape["entries"][1]["response"]["text"].as_str().unwrap()).unwrap();
    finding["artifact"]["quote"] = json!("No purchase order");
    tape["entries"][1]["response"]["text"] = json!(finding.to_string());
    write_tape(&tape);
    let out = compare(candidate_digest, &cases);
    assert!(!out.status.success(), "{out:?}");
    let bad = record(&out, "candidate_revision");
    let row = &bad["cases"][0]["candidate"];
    assert_eq!(row["observed_outcome"], "hold_for_procurement");
    assert_eq!(row["passed"], false);
    assert_eq!(
        row["artifact_checks"],
        json!([{
            "step": "extract", "visited": true, "mismatched_fields": ["quote"],
        }])
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("No purchase order"));
    assert_eq!(fs::read(deployment_file).unwrap(), deployment_before);
    assert!(trace(&db, before)
        .iter()
        .all(|(intent, _)| !intent.starts_with("workflow:activat")));
    assert_finding(dir, &base, false, "request_order");

    let unlabelled = dir.join("unlabelled.json");
    let mut labels: Value = serde_json::from_slice(&fs::read(&cases).unwrap()).unwrap();
    labels[0].as_object_mut().unwrap().remove("step_artifacts");
    fs::write(&unlabelled, labels.to_string()).unwrap();
    let before = watermark(&db);
    for (digest, cases, reason) in [
        ("wrong-digest", &cases, "does not match the candidate plan"),
        (candidate_digest, &unlabelled, "needs step_artifacts"),
    ] {
        let out = compare(digest, cases);
        assert!(
            !out.status.success() && String::from_utf8_lossy(&out.stderr).contains(reason),
            "{out:?}"
        );
    }
    assert!(trace(&db, before).is_empty());

    let applied = workflow(
        dir,
        "apply",
        "candidate.json",
        candidate_digest,
        Some(&example(MISSING_TAPE)),
    );
    assert!(applied.status.success(), "{applied:?}");
    let before = watermark(&db);
    let out = compare(candidate_digest, &cases);
    assert!(
        !out.status.success()
            && String::from_utf8_lossy(&out.stderr).contains("replan before comparing"),
        "{out:?}"
    );
    assert!(trace(&db, before).is_empty());
}

#[test]
fn invalid_replay_never_dispatches_the_reviewer() {
    let temp = setup();
    let dir = temp.path();
    let plan = deploy(dir, "invoice-evidence.toml", "base.json");
    assert_finding(dir, &plan, false, "request_order");
    let db = audit(dir);
    let input = example(MISSING);
    let tape = example(MISSING_TAPE);
    let variant = dir.join("variant.json");
    let mut cassette: Value = serde_json::from_slice(&fs::read(&tape).unwrap()).unwrap();
    let text = cassette["entries"][0]["response"]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    let finding: Value = serde_json::from_str(&text).unwrap();

    let refused = |input: &Path, tape: &Path, reason: &str| {
        let before = watermark(&db);
        let out = workflow(dir, "run", ID, input, Some(tape));
        assert!(
            out.status.code().is_some() && !out.status.success(),
            "{out:?}"
        );
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(reason),
            "{out:?}"
        );
        trace(&db, before)
    };
    let refused_at_extract = |input: &Path, tape: &Path, reason: &str| {
        let events = refused(input, tape, reason);
        let intents: Vec<_> = events.iter().map(|(intent, _)| intent.as_str()).collect();
        assert_eq!(
            intents,
            ["workflow:dispatch", "workflow:run-refused"],
            "{reason}"
        );
        let last = &events[1].1;
        assert!(last["run_id"].is_string());
        assert_eq!(events[0].1["run_id"], last["run_id"]);
        assert_eq!(events[0].1["step"], "extract");
        assert_eq!(last["step"], "extract");
    };

    cassette["entries"][0]["response"]["text"] = json!("{");
    fs::write(&variant, serde_json::to_vec(&cassette).unwrap()).unwrap();
    refused_at_extract(&input, &variant, "malformed evidence JSON");

    for (field, invalid, reason) in [
        (
            "/artifact/purchase_order_present",
            "false",
            "purchase_order_present",
        ),
        ("/outcome", "approved", "outcome"),
        (
            "/artifact/quote",
            "A purchase order is attached.",
            "citation",
        ),
    ] {
        let mut answer = finding.clone();
        *answer.pointer_mut(field).unwrap() = json!(invalid);
        cassette["entries"][0]["response"]["text"] = json!(answer.to_string());
        fs::write(&variant, serde_json::to_vec(&cassette).unwrap()).unwrap();
        refused_at_extract(&input, &variant, reason);
    }

    // Same byte length; even the expected quote remains valid. Only the prompt hash differs.
    let drift = dir.join("drift.json");
    fs::write(
        &drift,
        fs::read_to_string(&input).unwrap().replace("41", "43"),
    )
    .unwrap();
    refused_at_extract(&drift, &tape, "cassette drift");

    // A wildcard is refused when loading the cassette, before a run ID can be issued.
    cassette["entries"][0]["response"]["text"] = json!(text);
    cassette["entries"][0]["prompt_sha256"] = json!("0".repeat(64));
    fs::write(&variant, serde_json::to_vec(&cassette).unwrap()).unwrap();
    assert!(refused(&input, &variant, "exact prompt hash").is_empty());
}

#[test]
fn operator_policy_denies_inference_before_dispatching_the_consumer() {
    let temp = setup();
    let dir = temp.path();
    deploy(dir, "invoice-evidence.toml", "base.json");
    let db = audit(dir);
    let before = watermark(&db);
    let result = workflow_command(
        dir,
        "run",
        ID,
        example(MISSING),
        Some(&example(MISSING_TAPE)),
    )
    .env(
        "MAOS_PDP_POLICY_INLINE",
        r#"forbid(principal, action == Action::"provider.infer", resource);"#,
    )
    .output()
    .unwrap();
    assert!(!result.status.success(), "{result:?}");
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("PDP denied capability issuance"),
        "{result:?}"
    );
    let events = trace(&db, before);
    assert_eq!(
        events
            .iter()
            .map(|(intent, _)| intent.as_str())
            .collect::<Vec<_>>(),
        ["workflow:dispatch", "workflow:run-refused"]
    );
    assert_eq!(events[0].1["step"], "extract");
    let calls: i64 = db
        .query_row(
            "SELECT count(*) FROM transparency_log WHERE rowid > ?1 AND intent LIKE 'infer:%'",
            [before],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(calls, 0);
}
