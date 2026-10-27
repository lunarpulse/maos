//! Story 17-2 — execute aggregate status handling against required-job results.
use serde_yaml::Value;
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

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
        .find(|s| s["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
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
    let dir = fresh_dir("maos-cgroup-workflow");
    let file = dir.join("output");
    let needs: BTreeMap<_, _> = needs
        .iter()
        .map(|(name, result)| (name, serde_json::json!({ "result": result })))
        .collect();
    let status = Command::new("bash")
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
    let dir = fresh_dir("maos-cgroup-failure");
    let status = Command::new("bash")
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
fn aggregate_reports_every_required_job_non_success_as_failure() {
    let w = workflow();
    let aggregate = get(get(&w, "jobs"), "aggregate");
    let required = ["cgroup-ceiling-delegated", "cgroup-ceiling-undelegated"];
    let successful_dependencies: BTreeMap<String, String> = get(aggregate, "needs")
        .as_sequence()
        .expect("aggregate needs")
        .iter()
        .map(|value| {
            (
                value.as_str().expect("job name").to_owned(),
                "success".to_owned(),
            )
        })
        .collect();
    let status = named(aggregate, "Check for failures");
    let script = get(status, "run").as_str().expect("status shell script");
    let failure = named(aggregate, "Fail if any gate failed");
    let failure_if = get(failure, "if").as_str().expect("failure step predicate");
    let failure_run = get(failure, "run").as_str().expect("failure step shell");
    for failed in required {
        for state in ["skipped", "failure", "cancelled"] {
            let mut needs = successful_dependencies.clone();
            if let Some(result) = needs.get_mut(failed) {
                *result = state.to_owned();
            }
            assert!(
                needs.contains_key(failed),
                "{failed} must remain an aggregate need"
            );
            let observed = output(script, &needs);
            println!("{failed}={state}: {}", observed.trim());
            assert_eq!(observed.trim(), "overall=failure", "{failed}={state}");
            let selected = failure_selected(failure_if, &observed);
            assert!(selected, "failure step did not select {failed}={state}");
            assert!(
                !run_successful(failure_run),
                "failure step exited successfully with {failed}={state}"
            );
        }
    }
    let observed = output(script, &successful_dependencies);
    println!("all-success control: {}", observed.trim());
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(failure_if, &observed),
        "failure step selected all-success control"
    );
}

#[test]
fn archive_cgroup_jobs_keep_real_delegation_counts_and_rootless_canary() {
    let workflow = workflow();
    let jobs = get(&workflow, "jobs");
    for name in ["cgroup-ceiling-delegated", "cgroup-ceiling-undelegated"] {
        let job = get(jobs, name);
        assert_eq!(job["strategy"]["fail-fast"], Value::Bool(false));
        let images: Vec<_> = job["strategy"]["matrix"]["image"]
            .as_sequence()
            .expect("cgroup image matrix")
            .iter()
            .map(|image| image.as_str().expect("image"))
            .collect();
        assert_eq!(
            images,
            [
                "ubuntu-24.04",
                "ubuntu-26.04",
                "ubuntu-24.04-arm",
                "ubuntu-26.04-arm",
            ]
        );
        assert_eq!(job["runs-on"].as_str(), Some("${{ matrix.image }}"));
        assert_eq!(job["env"]["MAOS_FIXTURE_TRANSPORT"].as_str(), Some("1"));
        assert_eq!(
            job["env"]["MAOS_CGROUP_CEILING_FIXTURE"].as_str(),
            Some("${{ github.workspace }}/target/tmp/cgroup-ceiling-fixture")
        );
        assert!(get(job, "needs")
            .as_sequence()
            .expect("producer dependency")
            .iter()
            .any(|need| need.as_str() == Some("ci-test-producer")));
    }
    let delegated = get(jobs, "cgroup-ceiling-delegated");
    for (step, mode, suite, archive, count) in [
        ("Native cgroup-v2 smoke — four real cases", "delegated", "cgroup_ceiling_smoke", "kernel", 4),
        ("Real admitted session suite — 23 cases, serial", "delegated", "spawned_scope_17_3c", "maos-bin-network-wasm", 23),
        ("Rootless T3 Worker resource enforcement (x64)", "delegated", "worker_resource_limits_17_2", "maos-bin-network-wasm", 2),
        ("Full undelegated session suite — live fallback, strict refusal, persistence and ordering", "undelegated", "spawned_scope_17_3c", "maos-bin-network-wasm", 23),
    ] {
        let job = if mode == "delegated" { delegated } else { get(jobs, "cgroup-ceiling-undelegated") };
        let step = named(job, step);
        let run = get(step, "run").as_str().expect("cgroup suite command");
        assert!(run.contains(&format!("scripts/run_cgroup_ceiling_17_2.py run --mode {mode} --unit maos-17-2-")));
        assert!(run.contains("cargo-nextest nextest run --archive-file"));
        assert!(run.contains(&format!("/ci-test-archives/{archive}.tar.zst")));
        assert!(run.contains(&format!("::{suite})")));
        assert!(run.contains("--no-capture --test-threads 1"));
        // Every test runs (no fail-fast) and none is killed short of the unit
        // `--timeout`: the repo config's `archive-consumer` profile.
        assert!(run.contains("--extract-overwrite --color never --profile archive-consumer --"));
        assert!(run.contains(" --timeout 1800 ") || run.contains(" --timeout 2400 "));
        assert!(run.contains("test ${PIPESTATUS[0]} -eq 0"));
        assert!(run.contains(&format!("check-log --exit-code 0 --expected-count {count} ")));
        if suite == "cgroup_ceiling_smoke" {
            // Execute every original resource case, not the nonignored fixture
            // validation tests (those run in workspace-test-suite).
            assert!(run.contains("--profile archive-consumer --run-ignored ignored-only"));
            // Ignored-only runs legitimately report the unignored cases as
            // nextest-skipped; no skipped-count guard applies here.
            assert!(!run.contains("[1-9][0-9]* skipped"));
            for name in [
                "native_child_is_placed_before_guest_and_resource_limits_bite",
                "same_spirit_simultaneous_native_spawns_have_distinct_owned_leaves",
                "creation_limit_write_and_exec_failures_release_owned_leaves",
                "killed_leader_cleanup_kills_its_surviving_descendant",
            ] {
                assert!(run.contains(&format!("--required-test {name}")));
            }
            assert!(step["if"].is_null(), "native resource proof runs on every image");
        } else if suite == "worker_resource_limits_17_2" {
            for name in [
                "real_maos_worker_applies_independent_t3_limits_and_observes_exhaustion",
                "fd_request_above_inherited_hard_limit_refuses_before_worker_payload",
            ] {
                assert!(run.contains(&format!("--required-test {name}")));
            }
            assert!(run.contains("--rootless-canary"));
            assert_eq!(
                step["env"]["MAOS_CGROUP_CANARY_ARCHIVE"].as_str(),
                Some("${{ runner.temp }}/ci-test-archives/maos-bin-network-wasm.tar.zst")
            );
            assert_eq!(step["if"].as_str(), Some("${{ !endsWith(matrix.image, '-arm') }}"));
        } else {
            assert!(step["if"].is_null(), "session proof runs on every image");
            let required: Vec<_> = step["env"]["EPIC17_SPAWNED_REQUIRED_TESTS"]
                .as_str()
                .expect("session proof names")
                .lines()
                .collect();
            assert_eq!(
                required,
                [
                    "eight_real_admitted_forms_use_trusted_sender_policy_not_echoed_claims",
                    "forged_guest_provenance_is_restamped_by_the_kernel",
                    "a_delivery_needs_its_iac_send_grant",
                    "wasm_delivery_is_refused_when_its_issuance_window_is_exhausted",
                    "kernel_only_cross_host_oversized_and_foreign_retract_frames_are_refused",
                    "guest_floods_end_in_typed_failures_not_a_wedged_session",
                    "guest_diagnostics_are_bounded_per_session",
                    "runner_refusal_exits_before_ready_are_typed_and_not_crashes",
                    "a_once_session_that_never_completes_its_turn_is_not_success",
                    "hostile_metadata_fails_before_serialization_under_real_t2",
                    "hostile_aliased_strings_fail_before_serialization_under_real_t2",
                    "one_aliased_string_under_the_hostcall_budget_clears_the_canonical_call",
                    "actual_runner_sigsys_reaches_escape_consumer_but_planned_stop_does_not",
                    "repeated_mailbox_exports_survive_idle_without_an_active_task_or_stall",
                    "a_new_turn_after_idle_restarts_the_progress_window",
                    "configured_enterprise_and_mixed_scope_refusals_are_atomic_for_both_forms",
                    "resource_downgrade_memory_only_is_persisted_for_each_fast_t2_restart",
                    "memory_only_t2_live_child_has_exact_rlimits_and_mode_truthful_journal",
                    "resource_downgrade_no_resource_or_fd_only_t2_spawn_emits_no_row",
                    "resource_downgrade_strict_cpu_pid_and_failed_t2_spawns_emit_no_row",
                    "resource_downgrade_append_panic_follows_live_child_observer_installation",
                    "admitted_t2_echo_enforces_exact_live_caps_in_delegated_mode",
                    "fast_t2_crash_then_restart_orders_one_downgrade_after_crash",
                ]
            );
            assert!(run.contains("required_tests=()"));
            assert!(run.contains("required_tests+=(--required-test \"$test_name\")"));
            assert!(run.contains("done <<< \"${EPIC17_SPAWNED_REQUIRED_TESTS}\""));
            assert!(run.contains(
                "check-log --exit-code 0 --expected-count 23 \"${required_tests[@]}\""
            ));
        }
    }
    let fixture = named(
        delegated,
        "Build the actual Worker CLI fixture before the in-unit canary (x64)",
    );
    assert_eq!(
        fixture["if"].as_str(),
        Some("${{ !endsWith(matrix.image, '-arm') }}")
    );
    assert_eq!(
        fixture["run"].as_str(),
        Some("test -x target/debug/worker-cli-fixture")
    );
}

#[test]
fn observer_artifact_stages_fixed_cargo_scratch_beside_the_log() {
    let workflow = workflow();
    let job = get(get(&workflow, "jobs"), "epic-17-exit");
    let step = named(
        job,
        "Epic-17 exit line 5 — Observer live scene + activity corpus (Story 17-4)",
    );
    let run = get(step, "run").as_str().expect("observer command");
    for witness in [
        "rm -rf target/tmp/observer-activity-17-4",
        "rm -rf \"$RUNNER_TEMP/observer-activity-17-4\"",
        "if [ -d target/tmp/observer-activity-17-4 ]; then",
        "cp -a target/tmp/observer-activity-17-4 \"$RUNNER_TEMP/\" || rc=1",
        "if [ ! -s \"$RUNNER_TEMP/observer-activity-17-4/summary.json\" ]; then",
    ] {
        assert!(run.contains(witness), "observer staging lost {witness}");
    }
    let upload = steps(job)
        .iter()
        .find(|step| step["with"]["name"].as_str() == Some("epic-17-observer-${{ matrix.image }}"))
        .expect("observer artifact upload");
    assert_eq!(upload["if"].as_str(), Some("always()"));
    assert_eq!(
        upload["with"]["path"].as_str(),
        Some(
            "${{ runner.temp }}/observer-17-4.log\n\
             ${{ runner.temp }}/observer-activity-17-4/\n"
        )
    );
}
