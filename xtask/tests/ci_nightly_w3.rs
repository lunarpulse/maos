//! CI release-ready program W3 (2026-11-15) — the shape of `ci-nightly.yml`, of the jobs that moved
//! into it from `discipline.yml`, and of the nextest serial group the timing-bound binaries joined.
//! Parses the real files as configuration (the `ci_shape_18_0.rs` precedent); reads YAML, TOML and
//! markdown only.

use std::fs;
use std::path::PathBuf;

use serde_yaml::Value;

/// Slow or sensitive jobs that protect a release claim. They ran nightly (W3) and RETURNED to `discipline.yml`,
/// `aggregate.needs`, `release-ready.needs` and `xtask/release-ready-required.toml` for GA, as ordinary jobs
/// with NO `continue-on-error`. They are no longer nightly jobs.
const RETURNED: [&str; 2] = ["nfr-rel-11-halt-receipt-999pct", "expensive-tests"];

/// Advisory / measurement jobs: they carried `continue-on-error` in `discipline.yml` and keep it.
const ADVISORY: [&str; 6] = [
    "reproducible-build",
    "determinism-tests",
    "nfr-perf-1-iac-routing-budget",
    "nfr-perf-j4-latency",
    "nfr-perf-8-orchestrator-fanout",
    "clippy-advisory",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

fn workflow(name: &str) -> Value {
    serde_yaml::from_str(&read(&format!(".github/workflows/{name}")))
        .unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// Jobs moved off the merge and release path by lean-gates WP-06 (advisory, `continue-on-error`). They carry
/// no `deferred-work.md` row: the tracker rows that recorded earlier moves are not part of this move.
const LEAN_GATES_ADVISORY: [&str; 5] = [
    "windows-check",
    "check-perf-counts",
    "check-perf-runner-policy",
    "check-cert-rotation-trigger",
    "check-model-currency",
];

fn moved() -> impl Iterator<Item = &'static str> {
    ADVISORY.into_iter().chain(LEAN_GATES_ADVISORY)
}

fn needs(job: &Value) -> Vec<String> {
    match &job["needs"] {
        Value::Null => Vec::new(),
        Value::String(one) => vec![one.clone()],
        list => list
            .as_sequence()
            .expect("needs list or string")
            .iter()
            .map(|need| need.as_str().expect("need string").to_owned())
            .collect(),
    }
}

/// An `if:` condition with any `${{ … }}` wrapper and all whitespace removed.
fn condition(step: &Value) -> String {
    let raw: String = step["if"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    raw.strip_prefix("${{")
        .and_then(|inner| inner.strip_suffix("}}"))
        .unwrap_or(&raw)
        .to_owned()
}

#[test]
fn ci_nightly_is_scheduled_pinned_read_only_and_takes_no_expression_in_a_script() {
    let nightly = workflow("ci-nightly.yml");
    assert!(
        nightly["on"]["schedule"][0]["cron"].as_str().is_some(),
        "ci-nightly.yml needs a schedule"
    );
    assert!(
        nightly["on"]
            .as_mapping()
            .is_some_and(|on| on.contains_key(Value::String("workflow_dispatch".into()))),
        "ci-nightly.yml needs workflow_dispatch"
    );
    assert!(
        nightly["on"].get("push").is_none() && nightly["on"].get("pull_request").is_none(),
        "ci-nightly.yml runs on schedule and dispatch only; a push trigger would put it back on the merge path"
    );
    let permissions = nightly["permissions"].as_mapping().expect("permissions");
    assert_eq!(permissions.len(), 1, "permissions: contents: read only");
    assert_eq!(nightly["permissions"]["contents"].as_str(), Some("read"));
    assert_eq!(nightly["concurrency"]["group"].as_str(), Some("ci-nightly"));
    assert_eq!(
        nightly["concurrency"]["cancel-in-progress"].as_bool(),
        Some(false)
    );
    let jobs = nightly["jobs"].as_mapping().expect("jobs");
    assert_eq!(jobs.len(), moved().count(), "exactly the moved jobs");
    for (name, job) in jobs {
        let name = name.as_str().expect("job name");
        assert!(moved().any(|m| m == name), "unexpected job {name}");
        // `windows-check` is the one job that needs Windows; it keeps the runner it had in `discipline.yml`.
        let image = if name == "windows-check" {
            "windows-latest"
        } else {
            "ubuntu-24.04"
        };
        assert_eq!(
            job["runs-on"].as_str(),
            Some(image),
            "{name}: pinned runner image"
        );
        assert!(
            job["timeout-minutes"].as_u64().is_some(),
            "{name}: timeout-minutes"
        );
        assert!(
            job["needs"].is_null(),
            "{name}: nightly jobs are independent"
        );
        for step in job["steps"].as_sequence().expect("steps") {
            // Inputs and contexts reach scripts through `env:`, never by expression interpolation.
            if let Some(run) = step["run"].as_str() {
                assert!(
                    !run.contains("${{"),
                    "{name}: `${{{{ }}}}` inside a run script; pass it through env:\n{run}"
                );
            }
        }
    }
    assert!(
        !read(".github/workflows/ci-nightly.yml").contains("ubuntu-latest"),
        "no unpinned runner image"
    );
}

#[test]
fn moved_jobs_run_nightly_and_are_on_no_merge_or_ship_gate_path() {
    let discipline = workflow("discipline.yml");
    let nightly = workflow("ci-nightly.yml");
    let registry: toml::Value =
        toml::from_str(&read("xtask/gate-registry.toml")).expect("registry");
    let registry_rows: Vec<&str> = registry["ship_gate"]
        .as_array()
        .expect("ship_gate rows")
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    for name in RETURNED {
        assert!(
            nightly["jobs"][name].is_null(),
            "{name} returned to discipline.yml; it must not also be a nightly job"
        );
    }
    let discipline_jobs = discipline["jobs"].as_mapping().expect("jobs");
    for name in moved() {
        assert!(nightly["jobs"][name].is_mapping(), "{name} runs nightly");
        assert!(
            discipline["jobs"][name].is_null(),
            "{name} runs nightly; it must not also be a discipline job"
        );
        for gate in ["aggregate", "v1-0-ship-gate", "v1-5-ship-gate"] {
            assert!(
                !needs(&discipline["jobs"][gate]).iter().any(|n| n == name),
                "{name} must not be in {gate}.needs"
            );
        }
        for (other, job) in discipline_jobs {
            assert!(
                !needs(job).iter().any(|n| n == name),
                "{name} must not be needed by {other:?}"
            );
        }
        // The standing rule for leaving the merge path: no `[[ship_gate]]` row (hence no blocking
        // binding and no EXPECTED_GATES name, which the completeness gate requires a row for).
        assert!(
            !registry_rows.contains(&name),
            "{name} has a [[ship_gate]] row; it cannot leave the merge path"
        );
    }
    // `example-spirit-ts-tests` was considered and REJECTED: `wasm-host-tests` (a v1-0-ship-gate
    // member) `needs` it and downloads its component artifact, so it stays where it is.
    assert!(discipline["jobs"]["example-spirit-ts-tests"].is_mapping());
    assert!(needs(&discipline["jobs"]["wasm-host-tests"])
        .iter()
        .any(|n| n == "example-spirit-ts-tests"));
}

#[test]
fn every_nightly_job_records_its_result_as_summary_line_and_artifact() {
    let nightly = workflow("ci-nightly.yml");
    for (name, job) in nightly["jobs"].as_mapping().expect("jobs") {
        let name = name.as_str().expect("job name");
        let steps = job["steps"].as_sequence().expect("steps");
        let record = steps.last().expect("a last step");
        assert_eq!(
            record["uses"].as_str(),
            Some("./.github/actions/record-nightly-result"),
            "{name}: the last step records the result"
        );
        assert_eq!(
            condition(record),
            "always()",
            "{name}: a red job is recorded too"
        );
        assert_eq!(record["with"]["job"].as_str(), Some(name));
        // Every job records `job.status` verbatim, except `determinism-tests`: its advisory step is
        // `continue-on-error`, so the job is green even when the tests failed. It records the surfaced
        // step result when there is one (a test failure beats job success), else `job.status`.
        let expected = if name == "determinism-tests" {
            "${{ steps.surface.outputs.result || job.status }}"
        } else {
            "${{ job.status }}"
        };
        assert_eq!(
            record["with"]["status"].as_str(),
            Some(expected),
            "{name}: recorded status expression"
        );
    }
    let action: Value =
        serde_yaml::from_str(&read(".github/actions/record-nightly-result/action.yml"))
            .expect("parse record-nightly-result");
    let mut descriptive = vec![
        ("name", action["name"].as_str().expect("action name")),
        (
            "description",
            action["description"].as_str().expect("action description"),
        ),
    ];
    for input in ["job", "status"] {
        descriptive.push((
            input,
            action["inputs"][input]["description"]
                .as_str()
                .unwrap_or_else(|| panic!("input {input} description")),
        ));
    }
    for (field, text) in descriptive {
        assert!(
            !text.contains("${{"),
            "action metadata {field} must stay literal: expression examples are evaluated by the runner"
        );
    }
    let steps = action["runs"]["steps"].as_sequence().expect("steps");
    let write = steps[0]["run"].as_str().expect("write step");
    assert!(write.contains("$GITHUB_STEP_SUMMARY"), "job summary line");
    assert!(
        !write.contains("${{"),
        "the action's script takes its inputs through env:"
    );
    let upload = &steps[1];
    assert!(upload["uses"]
        .as_str()
        .is_some_and(|uses| uses.starts_with("actions/upload-artifact@")));
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("nightly-result-${{ inputs.job }}")
    );
    // The Unix smoke below supplies the values resolved by these bindings.
    let env = steps[0]["env"].as_mapping().expect("write step env");
    let wiring = [
        ("NIGHTLY_JOB", "${{ inputs.job }}"),
        ("NIGHTLY_STATUS", "${{ inputs.status }}"),
        ("NIGHTLY_SHA", "${{ github.sha }}"),
        (
            "NIGHTLY_RUN_URL",
            "${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}",
        ),
    ];
    for (key, expected) in wiring {
        assert_eq!(
            env.get(key).and_then(|value| value.as_str()),
            Some(expected),
            "the write step's env {key} wiring"
        );
    }
    assert_eq!(
        upload["with"]["path"].as_str(),
        Some("${{ runner.temp }}/nightly-result/${{ inputs.job }}.txt"),
        "the upload path is the recorder's actual output file"
    );
    assert_eq!(
        upload["with"]["retention-days"].as_u64(),
        Some(90),
        "result artifacts are retained 90 days"
    );
    assert_eq!(
        upload["with"]["if-no-files-found"].as_str(),
        Some("error"),
        "a missing result file must fail the upload, not pass silently"
    );
    // Where the old job uploaded an artifact, the nightly job still does.
    let expect_upload = [("clippy-advisory", "clippy-advisory-diagnostics")];
    for (job, artifact) in expect_upload {
        assert!(
            nightly["jobs"][job]["steps"]
                .as_sequence()
                .expect("steps")
                .iter()
                .any(|step| step["with"]["name"].as_str() == Some(artifact)),
            "{job} keeps uploading {artifact}"
        );
    }
}

#[test]
fn returned_jobs_are_ordinary_release_path_jobs_and_advisory_jobs_keep_their_labelled_continue_on_error(
) {
    let nightly = workflow("ci-nightly.yml");
    let discipline = workflow("discipline.yml");
    let required: toml::Value =
        toml::from_str(&read("xtask/release-ready-required.toml")).expect("release-ready roster");
    let required_jobs: Vec<&str> = required["jobs"]
        .as_array()
        .expect("roster jobs")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect();
    for name in RETURNED {
        let job = &discipline["jobs"][name];
        assert!(job.is_mapping(), "{name} is a discipline job again");
        assert!(
            job["continue-on-error"].is_null(),
            "{name}: a red job is the signal; it carries no continue-on-error"
        );
        for step in job["steps"].as_sequence().expect("steps") {
            assert!(
                step["continue-on-error"].is_null(),
                "{name}: no step-level continue-on-error"
            );
        }
        for gate in ["aggregate", "release-ready"] {
            assert!(
                needs(&discipline["jobs"][gate]).iter().any(|n| n == name),
                "{name} must be in {gate}.needs"
            );
        }
        assert!(
            required_jobs.contains(&name),
            "{name} must be pinned in release-ready-required.toml"
        );
        assert!(
            nightly["jobs"][name].is_null(),
            "{name} is no longer a nightly job"
        );
    }
    for name in ADVISORY.into_iter().chain(LEAN_GATES_ADVISORY) {
        let job = &nightly["jobs"][name];
        let masked = job["continue-on-error"].as_bool() == Some(true)
            || job["steps"]
                .as_sequence()
                .expect("steps")
                .iter()
                .any(|step| step["continue-on-error"].as_bool() == Some(true));
        assert!(masked, "{name}: advisory jobs keep their continue-on-error");
    }
}

#[test]
fn timing_bound_binaries_are_in_the_serial_test_group_with_no_budget_change() {
    let config: toml::Value = toml::from_str(&read(".config/nextest.toml")).expect("nextest.toml");
    let overrides = config["profile"]["default"]["overrides"]
        .as_array()
        .expect("default overrides");
    let serial: Vec<&toml::Value> = overrides
        .iter()
        .filter(|row| row.get("test-group").and_then(toml::Value::as_str) == Some("serial"))
        .collect();
    assert!(!serial.is_empty());
    let filter = serial
        .iter()
        .filter_map(|row| row.get("filter").and_then(toml::Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    for binary in [
        "maos-bin::erasure_million_20_11",
        "maos-bin::root_shutdown_unload_16_3",
        "maos-kernel-core::t3_terminal_helpers_17_7",
    ] {
        assert!(
            filter.contains(&format!("binary_id({binary})")),
            "{binary} must be in the serial test group"
        );
    }
    // Overrides must never set a slow-timeout (the archive-consumer profile relies on it).
    for row in overrides {
        assert!(
            row.get("slow-timeout").is_none(),
            "an override must not set a slow-timeout"
        );
    }
}

#[test]
fn deferred_work_records_every_move_with_owner_and_return_rule() {
    let deferred = read("_bmad-output/implementation-artifacts/deferred-work.md");
    for name in ADVISORY {
        let row = deferred
            .lines()
            .find(|line| {
                line.starts_with("- **")
                    && line.contains(&format!("`{name}`"))
                    && line.contains("ci-nightly.yml")
            })
            .unwrap_or_else(|| panic!("deferred-work.md has no ci-nightly row naming `{name}`"));
        assert!(
            row.contains("Owner: `ops-ci-release-ready`") && row.contains("rule10(a)"),
            "{name}: the row needs the owner and rule10(a)"
        );
        assert!(row.contains("Status: open"), "{name}: the row is open");
    }
    for name in RETURNED {
        let row = deferred
            .lines()
            .find(|line| {
                line.starts_with("- **")
                    && line.contains(&format!("`{name}`"))
                    && line.contains("returned from `ci-nightly.yml`")
            })
            .unwrap_or_else(|| {
                panic!("deferred-work.md has no row recording the return of `{name}`")
            });
        assert!(row.contains("Status: closed"), "{name}: the row is closed");
        assert!(
            row.contains("repromotion.yml"),
            "{name}: the row names the repromotion dispatch as its evidence"
        );
    }
}

/// Runs the real write step under bash (unix only) for the three statuses the recorder is given: the
/// line carries exactly the allowed fields and the artifact file holds the same line.
#[cfg(unix)]
#[test]
fn record_nightly_result_write_step_emits_one_line_to_summary_and_artifact_file() {
    let action: Value =
        serde_yaml::from_str(&read(".github/actions/record-nightly-result/action.yml"))
            .expect("parse record-nightly-result");
    let write = action["runs"]["steps"][0]["run"]
        .as_str()
        .expect("write step");
    for status in ["success", "failure", "cancelled"] {
        let dir = tempfile::tempdir().expect("tempdir");
        let summary = dir.path().join("summary");
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(write)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .env("NIGHTLY_JOB", "determinism-tests")
            .env("NIGHTLY_STATUS", status)
            .env("NIGHTLY_SHA", "abc123")
            .env("NIGHTLY_RUN_URL", "https://example.test/runs/1")
            .env("GITHUB_STEP_SUMMARY", &summary)
            .env("RUNNER_TEMP", dir.path())
            .output()
            .expect("run write step");
        assert!(out.status.success(), "write step failed for {status}");
        let line = fs::read_to_string(dir.path().join("nightly-result/determinism-tests.txt"))
            .expect("result file");
        let fields: Vec<&str> = line.trim_end().split(' ').collect();
        assert_eq!(fields.len(), 6, "line fields for {status}: {line}");
        assert_eq!(fields[0], "nightly-result");
        assert_eq!(fields[1], "job=determinism-tests");
        assert_eq!(fields[2], format!("status={status}"));
        assert_eq!(fields[3], "sha=abc123");
        let at = fields[4]
            .strip_prefix("at=")
            .unwrap_or_else(|| panic!("timestamp field: {line}"));
        assert!(at.ends_with('Z'), "UTC suffix Z: {at}");
        let parsed = chrono::DateTime::parse_from_rfc3339(at)
            .unwrap_or_else(|e| panic!("RFC3339 timestamp {at}: {e}"));
        assert_eq!(parsed.offset().local_minus_utc(), 0, "UTC offset: {at}");
        assert_eq!(fields[5], "run=https://example.test/runs/1");
        assert_eq!(
            fs::read_to_string(&summary).expect("summary"),
            line,
            "summary line equals the artifact line"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            line,
            "stdout echoes the same line"
        );
    }
}
