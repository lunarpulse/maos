//! Story 17-7 — pin the T3 terminal-facts workflow route and aggregate gate.
use serde_yaml::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn workflow() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/discipline.yml");
    serde_yaml::from_str(&fs::read_to_string(path).expect("discipline workflow"))
        .expect("workflow YAML")
}
fn get<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .as_mapping()
        .expect("mapping")
        .get(&Value::String(name.into()))
        .unwrap_or_else(|| panic!("missing {name}"))
}
fn steps(job: &Value) -> &[Value] {
    get(job, "steps").as_sequence().expect("steps")
}
fn named<'a>(job: &'a Value, name: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}
// Replant the old skip-ignoring predicate into a copy of the real workflow.
fn workflow_ignoring_skips() -> Value {
    let mut value = workflow();
    let step = value["jobs"]["aggregate"]["steps"]
        .as_sequence_mut()
        .expect("aggregate steps")
        .iter_mut()
        .find(|step| step["name"].as_str() == Some("Check for failures"))
        .expect("status step");
    let script = step["run"].as_str().expect("status script");
    let clause = " or (.key != \"macos-check\" and .value.result == \"skipped\")";
    assert!(
        script.contains(clause),
        "skip clause must exist before mutation"
    );
    step["run"] = Value::String(script.replace(clause, ""));
    value
}

fn dependencies(aggregate: &Value) -> BTreeMap<String, String> {
    get(aggregate, "needs")
        .as_sequence()
        .expect("aggregate needs")
        .iter()
        .map(|name| {
            (
                name.as_str().expect("job name").to_owned(),
                "success".to_owned(),
            )
        })
        .collect()
}

fn failure_selected(predicate: &str, status_output: &str) -> bool {
    assert_eq!(
        predicate, "${{ steps.status.outputs.overall == 'failure' }}",
        "failure step must consume the executed status step's output"
    );
    status_output.lines().any(|line| line == "overall=failure")
}

fn fresh_dir(prefix: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).expect("fresh temp dir");
    path
}

fn output(script: &str, needs: &BTreeMap<String, String>) -> String {
    let dir = fresh_dir("maos-terminal-workflow");
    let file = dir.join("output");
    let needs: BTreeMap<_, _> = needs
        .iter()
        .map(|(name, result)| (name, serde_json::json!({ "result": result })))
        .collect();
    let status = std::process::Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("GITHUB_OUTPUT", &file)
        .env(
            "NEEDS_JSON",
            serde_json::to_string(&needs).expect("needs JSON"),
        )
        .status()
        .expect("bash status script");
    let value = fs::read_to_string(&file).expect("GITHUB_OUTPUT");
    let _ = fs::remove_dir_all(dir);
    assert!(status.success(), "aggregate status shell failed");
    value
}

fn run_successful(script: &str) -> bool {
    let dir = fresh_dir("maos-terminal-failure");
    let status = std::process::Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("GITHUB_OUTPUT", dir.join("output"))
        .status()
        .expect("failure step shell");
    let _ = fs::remove_dir_all(dir);
    status.success()
}

#[test]
fn aggregate_workflow_executes_non_success_handling_and_failure_step() {
    let value = workflow();
    let aggregate = &value["jobs"]["aggregate"];
    assert!(
        get(aggregate, "needs")
            .as_sequence()
            .expect("aggregate needs")
            .iter()
            .any(|dependency| dependency.as_str() == Some("epic-17-exit")),
        "epic-17-exit must be a required aggregate dependency"
    );
    let dependencies = dependencies(aggregate);
    let script = named(aggregate, "Check for failures")["run"]
        .as_str()
        .expect("status script");
    let failure = named(aggregate, "Fail if any gate failed");
    let failure_if = failure["if"].as_str().expect("failure predicate");
    let failure_run = failure["run"].as_str().expect("failure shell");

    for state in ["skipped", "failure", "cancelled"] {
        let job = "epic-17-exit";
        let mut needs = dependencies.clone();
        needs.insert(job.to_owned(), state.to_owned());
        let observed = output(script, &needs);
        println!("{job}={state}: {}", observed.trim());
        assert_eq!(observed.trim(), "overall=failure", "{job}={state}");
        assert!(
            failure_selected(failure_if, &observed),
            "failure step must run for {job}={state}"
        );
        assert!(
            !run_successful(failure_run),
            "failure step must exit nonzero for {job}={state}"
        );
    }

    let observed = output(script, &dependencies);
    println!("all-success control: {}", observed.trim());
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(failure_if, &observed),
        "failure step selected all-success control"
    );

    // The same executed vector exposes the old skip-ignoring predicate as a false green.
    let without = workflow_ignoring_skips();
    let omitted = &without["jobs"]["aggregate"];
    let omitted_script = named(omitted, "Check for failures")["run"]
        .as_str()
        .expect("status script without target predicate");
    let mut needs = dependencies.clone();
    needs.insert("epic-17-exit".into(), "skipped".into());
    let observed = output(omitted_script, &needs);
    println!(
        "planted skip-ignoring predicate, epic-17-exit=skipped: {}",
        observed.trim()
    );
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(
            named(omitted, "Fail if any gate failed")["if"]
                .as_str()
                .expect("failure predicate"),
            &observed
        ),
        "predicate removal must expose skipped job as missed failure"
    );
}

/// The live executed-count guard (its first `grep -Eq` ERE) fires on SKIP, on
/// nextest's indented zero-count summary and on any nonzero nextest `skipped`
/// count (an ignored test), never on a full 10- or 20-test run or on nextest's
/// `Starting ... (N binaries skipped)` header.
fn guard_refuses_skip_ignored_and_zero(run: &str) -> bool {
    let Some(guard) = run
        .split("grep -Eq '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
    else {
        return false;
    };
    let guard = regex::Regex::new(&format!("(?m){guard}")).expect("ERE guard");
    [
        ("        SKIP [   0.000s] maos-bin::scene skipped", true),
        (
            "     Summary [   0.012s] 0 tests run: 0 passed, 0 skipped",
            true,
        ),
        (
            "     Summary [  12.345s] 30 tests run: 30 passed, 1 skipped",
            true,
        ),
        (
            "     Summary [  12.345s] 3 tests run: 3 passed (1 slow), 12 skipped",
            true,
        ),
        (
            "     Summary [  12.345s] 10 tests run: 10 passed, 0 skipped",
            false,
        ),
        (
            "     Summary [  12.345s] 20 tests run: 20 passed, 0 skipped",
            false,
        ),
        (
            "    Starting 23 tests across 1 binary (5 binaries skipped)",
            false,
        ),
    ]
    .into_iter()
    .all(|(log, fires)| guard.is_match(log) == fires)
}

const SKIPPED_ALTERNATIVE: &str = "|tests? run: .*, [1-9][0-9]* skipped";

#[test]
fn epic17_archive_keeps_terminal_and_quota_proofs_on_both_t3_images() {
    let workflow = workflow();
    let job = &workflow["jobs"]["epic-17-exit"];
    assert_eq!(job["strategy"]["fail-fast"], Value::Bool(false));
    let images: Vec<_> = job["strategy"]["matrix"]["image"]
        .as_sequence()
        .expect("T3 image matrix")
        .iter()
        .map(|image| image.as_str().expect("image"))
        .collect();
    assert_eq!(images, ["ubuntu-24.04", "ubuntu-26.04"]);
    assert_eq!(job["runs-on"].as_str(), Some("${{ matrix.image }}"));
    assert_eq!(job["env"]["MAOS_FIXTURE_TRANSPORT"].as_str(), Some("1"));
    assert_eq!(job["env"]["EPIC17_TERMINAL_SUITE_TESTS"].as_u64(), Some(32));
    assert!(job["env"]["MAOS_CGROUP_CANARY_ARCHIVE"].is_null());
    for value in job["env"].as_mapping().expect("job environment").values() {
        assert!(
            !value
                .as_str()
                .is_some_and(|value| value.contains("runner.")),
            "runner context is unavailable in job-level env"
        );
    }
    let required: Vec<_> = job["env"]["EPIC17_TERMINAL_REQUIRED_TESTS"]
        .as_str()
        .expect("terminal proof names")
        .lines()
        .collect();
    assert_eq!(required, [
        "root_crash::root_crash_retains_stopped_container_and_purge_reports_it",
        "runtime_scenes::client_first_stop_snapshots_only_a_terminal_state",
        "runtime_scenes::fallback_oom_marker_persists_classified_fault_through_worker_binding",
        "runtime_scenes::ownership_is_by_cid_never_by_requested_name",
        "s1_immediate_exit_terminal_facts",
        "s2_intentional_exit_137_is_not_a_signal",
        "s3_external_sigkill_requires_pidfd_fact",
        "s4_oom_marker_is_not_payload_evidence_alone",
        "startup_scenes::root_signal_during_init_joins_owned_cid_before_forced_exit",
        "stub_scenes::bearer_create_diagnostic_and_runtime_arguments_do_not_echo_secret",
        "stub_scenes::concurrent_facts_and_teardown_share_snapshot_and_verdict",
        "stub_scenes::deadlines_hung_exists_is_typed_and_bounded",
        "stub_scenes::deadlines_hung_rm_is_bounded",
        "stub_scenes::deadlines_kill_is_bounded_and_verdict_is_shared",
        "stub_scenes::deadlines_remove_failure_is_typed",
        "stub_scenes::init_pidfile_invalid_forms_refuse_and_remove_only_receipt_cid",
        "stub_scenes::launch_refuses_nonempty_and_wrong_mode_state_before_runtime",
        "stub_scenes::ownership_create_timeout_cleans_the_receipt_cid",
        "stub_scenes::ownership_receipt_authority_refuses_foreign_and_disagreement",
        "supervisor_scenes::binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease",
        "supervisor_scenes::bridge_finalized_exit_rows_round_trip_through_fr4_audit_query",
        "supervisor_scenes::clean_exit_marker_completes_from_persisted_bridge_output",
        "supervisor_scenes::deadlines_root_stop_tears_down_n_cids_under_one_deadline",
        "supervisor_scenes::failed_create_and_finalized_exit_rows_never_echo_bearer",
        "supervisor_scenes::fallback_client_exit_persists_nonzero_fault_through_binding_classifier",
        "supervisor_scenes::late_attachment_after_stop_tears_down_once",
        "supervisor_scenes::sandbox_applied_report_survives_journal_reopen",
        "supervisor_scenes::unexpected_client_echild_finalizer_first_persists_one_truncated_fault",
        "supervisor_scenes::unexpected_client_observer_first_persists_one_truncated_fault",
        "unload_scenes::s5_on_unload_over_budget_keeps_planned_container_ownership",
        "unload_scenes::s5_on_unload_panic_before_stop_keeps_planned_container_ownership",
    ]);
    for (name, suite, unit) in [
        (
            "Epic-17 exit line 4 — retained T3 terminal facts (Story 17-7)",
            "worker_terminal_facts_17_7",
            "maos-17-7-terminal",
        ),
        (
            "CLI-wrapper spawn refuses on issuance-quota exhaustion (Story 17-9)",
            "cli_wrapper_exhaustion_17_9",
            "maos-17-7-cli-quota-17-9",
        ),
    ] {
        let step = named(job, name);
        assert!(step["if"].is_null(), "required T3 proof cannot be omitted");
        assert_eq!(
            step["env"]["MAOS_CGROUP_CANARY_ARCHIVE"].as_str(),
            Some("${{ runner.temp }}/ci-test-archives/maos-bin-network-wasm.tar.zst")
        );
        let run = step["run"].as_str().expect("proof command");
        for witness in [
            format!("run --mode delegated --unit {unit}"),
            "--rootless-canary".into(),
            "cargo-nextest nextest run --archive-file".into(),
            "/ci-test-archives/maos-bin-network.tar.zst".into(),
            format!("binary_id(maos-bin::{suite})"),
            "--color never --profile archive-consumer --no-capture --test-threads 1".into(),
            "test ${PIPESTATUS[0]} -eq 0".into(),
            "--required-test".into(),
            format!("SKIP|(^|[[:space:]])0 tests run:{SKIPPED_ALTERNATIVE}' \"$RUNNER_TEMP/"),
            "exit \"$rc\"".into(),
        ] {
            assert!(run.contains(&witness), "{name} lost {witness}");
        }
        assert!(guard_refuses_skip_ignored_and_zero(run), "{name}");
        let planted = run.replacen(SKIPPED_ALTERNATIVE, "", 1);
        assert_ne!(planted, run, "{name}: guard plant must change the step");
        assert!(!guard_refuses_skip_ignored_and_zero(&planted), "{name}");
        if suite == "worker_terminal_facts_17_7" {
            assert!(run.contains("done <<< \"${EPIC17_TERMINAL_REQUIRED_TESTS}\""));
            assert!(run.contains("--expected-count \"${EPIC17_TERMINAL_SUITE_TESTS}\""));
        } else {
            assert!(run.contains("--expected-count 4"));
            for witness in [
                "remote_exhaustion_refuses_before_workload",
                "local_exhaustion_refuses_before_workload",
                "absent_grant_keeps_d1_tokenless_proceed_posture",
                "absent_grant_is_policy_denied_even_with_spent_quota",
                "CLI-QUOTA-17-9 remote exhaustion refused before workload",
                "CLI-QUOTA-17-9 local exhaustion refused before workload",
                "CLI-QUOTA-17-9 absent grant D1 completed without token",
                "CLI-QUOTA-17-9 absent grant policy denial precedes quota",
                "grep -qF -- \"$marker\"",
            ] {
                assert!(run.contains(witness), "quota proof lost {witness}");
            }
        }
    }
}

#[test]
fn epic17_archive_keeps_observer_counts_oracle_and_delegated_launch() {
    let workflow = workflow();
    let job = &workflow["jobs"]["epic-17-exit"];
    let step = named(
        job,
        "Epic-17 exit line 5 — Observer live scene + activity corpus (Story 17-4)",
    );
    assert!(step["if"].is_null());
    let run = step["run"].as_str().expect("observer command");
    for witness in [
        "binary_id(maos-bin::observer_activity_corpus_17_4)",
        "binary_id(maos-bin::observer_live_17_4)",
        "--archive-file \"$RUNNER_TEMP/ci-test-archives/maos-bin-network.tar.zst\"",
        "--expected-count 16 --expected-count 3",
        "done <<< \"${EPIC17_LINE5_REQUIRED_TESTS}\"",
        "\"${required_tests[@]}\"",
        "if [ \"$corpus_rc\" -ne 0 ]; then rc=$corpus_rc; fi",
        "if [ ! -s \"$RUNNER_TEMP/observer-activity-17-4/summary.json\" ]; then",
        "exit \"$rc\"",
        "--color never --profile archive-consumer --no-capture --test-threads 1",
        "' \"$RUNNER_TEMP/observer-17-4.log\"; then",
        "::SKIP, ignored, or zero tests\"",
    ] {
        assert!(run.contains(witness), "observer proof lost {witness}");
    }
    // Six required names cover 6 of the 19 tests: only the guard refuses an
    // ignored (nextest-skipped) one.
    assert!(guard_refuses_skip_ignored_and_zero(run));
    let planted = run.replacen(SKIPPED_ALTERNATIVE, "", 1);
    assert_ne!(planted, run, "observer guard plant must change the step");
    assert!(!guard_refuses_skip_ignored_and_zero(&planted));
    let required: Vec<_> = step["env"]["EPIC17_LINE5_REQUIRED_TESTS"]
        .as_str()
        .expect("observer proof names")
        .lines()
        .collect();
    assert_eq!(
        required,
        [
            "observer_activity_corpus_production_path",
            "overflow_counts_exact_loss_and_preserves_first_plus_suffix",
            "real_corpus_meets_all_scenario_floors",
            "observer_serves_and_reports_a_door_loaded_butler_live",
            "a_door_loaded_observer_reports_like_the_root_one",
            "once_roots_with_an_observer_exit_cleanly_without_activity_lines",
        ]
    );
    let launch = named(
        job,
        "Epic-17 exit line 2 — the TypeScript Spirit launch (Story 17-3c)",
    );
    let run = launch["run"].as_str().expect("delegated launch");
    for witness in [
        "sudo systemd-run",
        "--property=Delegate=yes",
        "scripts/run_17_3c_delegated.sh\" target/release/maos run",
        "if grep -q '^SKIP[: ]' \"$RUNNER_TEMP/line2.log\"; then rc=1",
        "exit \"$rc\"",
    ] {
        assert!(run.contains(witness), "launch proof lost {witness}");
    }
}
