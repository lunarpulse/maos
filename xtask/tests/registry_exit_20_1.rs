use std::collections::BTreeSet;
use xtask::release_dry_run::ARTIFACTS;
#[test]
fn registry_release_set_and_exit_job_are_enrolled() {
    let binaries: BTreeSet<_> = ARTIFACTS.iter().map(|a| a.binary).collect();
    assert_eq!(
        binaries,
        BTreeSet::from([
            "maos",
            "maosctl",
            "maos-registry-server",
            "maos-spirit",
            "maos-wasm-runner"
        ])
    );
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../.github/workflows/discipline.yml")).unwrap();
    let job = &workflow["jobs"]["epic-20-exit"];
    assert!(job.is_mapping(), "epic-20-exit must actually be a CI job");
    let needs = workflow["jobs"]["aggregate"]["needs"]
        .as_sequence()
        .unwrap();
    assert!(needs.iter().any(|n| n.as_str() == Some("epic-20-exit")));
    let script = job["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|s| s["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "npm ci",
        "npm run componentize",
        "release-dry-run",
        "HOME=",
        "--root",
        "ready=0",
        "ready=1",
        "registry readiness failed",
        "wait",
        "trap",
        "unknown spirit",
        "spirit_session_complete",
        // The default signed daemon and runner are selected by this exact
        // manifest pattern; the shell must emit one real guest reply, not an ack.
        "(maos|maos-wasm-runner)-linux-amd64$",
        "ln -sf \"$b-linux-amd64\" \"dist/$b\"",
        "sha256sum -c -",
        "[ -s shell.log ]",
        "Spirit host (WASM component form, ADR-031) initialized",
        "[ \"$shell_rc\" -eq 0 ]",
        "cat shell.out shell.err > shell.log",
        "awk 'index($0,\"example-spirit-ts:\")==1 { total++; if (!exited && $0==\"example-spirit-ts: echo: hello\") exact++ } $0==\"maos: shell exiting\" { exited=1 } END { exit !(total==1 && exact==1 && exited) }' shell.out",
        "> shell.out 2> shell.err",
        "' shell.out",
        "wasm_engine_off|refused at boot|dispatch refused",
    ] {
        assert!(script.contains(required), "exit job lacks {required}");
    }
    assert!(
        script.contains("for b in maos maosctl maos-registry-server maos-spirit maos-wasm-runner;"),
        "the unsuffixed runner sibling must be created beside dist/maos"
    );
    // The refusal is the engine-free witness; it can never be this job's
    // success condition, and no opt-in feature may mask the default shape.
    assert!(
        !script.contains("grep -Fx \"maos: spirit 'example-spirit-ts' refused at boot"),
        "exit line 9 must not accept the engine-off refusal"
    );
    assert!(
        !script.contains("--features wasm-host"),
        "the exit job must test the default feature shape"
    );
    let steps = job["steps"].as_sequence().unwrap();
    let position = |needle: &str| {
        steps
            .iter()
            .position(|s| s["uses"].as_str().is_some_and(|u| u.contains(needle)))
    };
    let substrate = position("provision-t3-test-substrate").expect("T2 substrate step");
    let dispatch = steps
        .iter()
        .position(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("example-spirit-ts: echo: hello"))
        })
        .expect("shell dispatch step");
    assert!(
        substrate < dispatch,
        "the T2 substrate must precede the installed-Spirit dispatch"
    );
}

#[test]
fn engine_free_refusal_and_default_proof_are_separate_build_shapes() {
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../.github/workflows/discipline.yml")).unwrap();
    let script = |job: &str| {
        workflow["jobs"][job]["steps"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|s| s["run"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let admission = script("example-spirit-ts-tests");
    for required in [
        "--no-default-features --features network --test wasm_form_admission_17_3b",
        "--no-default-features --features network --test installed_spirit_dispatch_20_1",
        "maos_run_refuses_wasm_component_manifests_by_named_engine_state",
        "installed_component_dispatch_reports_typed_boot_refusal",
        "--test wasm_form_admission_17_3b --test wasm_skill_authoring_20_8 --test installed_spirit_dispatch_20_1",
        "boot_rechecks_manifest_component_publisher_and_claim",
        "an engine-free-only witness ran in the default shape",
    ] {
        assert!(admission.contains(required), "admission job lacks {required}");
    }
    assert!(
        !admission.contains("--features wasm-host"),
        "the default admission run must not use the redundant opt-in"
    );
    let default_run = admission
        .lines()
        .find(|line| line.contains("--test wasm_skill_authoring_20_8"))
        .expect("default admission command");
    assert!(!default_run.contains("--no-default-features"));
    for line in admission
        .lines()
        .filter(|line| line.contains("--no-default-features"))
    {
        assert!(
            line.contains("--no-default-features --features network --test "),
            "engine-free shape must be exactly --no-default-features --features network: {line}"
        );
    }
}

#[test]
fn portability_legs_compile_the_default_daemon_and_runner_without_runtime_claims() {
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../.github/workflows/discipline.yml")).unwrap();
    let run_lines = |job: &str| {
        workflow["jobs"][job]["steps"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|s| s["run"].as_str())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let windows = run_lines("windows-check");
    assert!(windows.iter().any(|run| run
        == "cargo build --locked -p maos-bin -p maos-wasm-host --bin maos --bin maos-wasm-runner"));
    let macos = run_lines("macos-check");
    assert!(macos.iter().any(|run| run
        == "cargo check --locked -p maos-bin -p maos-wasm-host --all-targets --target aarch64-apple-darwin"));
    for run in windows.iter().chain(macos.iter()) {
        assert!(
            !run.contains("--features network,wasm-host"),
            "no opt-in feature may mask the tested default shape: {run}"
        );
    }
    // Compile evidence is not a Windows release: no Windows release target exists.
    let release = include_str!("../../.github/workflows/release.yml");
    assert!(
        !release.contains("pc-windows"),
        "a Windows release artifact is not authorized by the compile leg"
    );
}

#[test]
fn package_jobs_provision_registered_registry_siblings_before_tests() {
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../.github/workflows/discipline.yml")).unwrap();
    for (job, consumer) in [
        ("check-multi-tenant-loom", "check-multi-tenant-loom --json"),
        ("air-gap-import-corpus", "cargo test -p maos-spirit-cli"),
    ] {
        let steps = workflow["jobs"][job]["steps"].as_sequence().unwrap();
        let setup = steps
            .iter()
            .position(|step| {
                step["run"].as_str().is_some_and(|script| {
                    script.contains("build-test-fixtures")
                        && ["maosctl", "maos-registry-server", "maos-spirit"]
                            .iter()
                            .all(|binary| script.contains(binary))
                })
            })
            .expect("registered sibling provenance must be built");
        let test = steps
            .iter()
            .position(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|script| script.contains(consumer))
            })
            .expect("package consumer must remain enrolled");
        assert!(
            setup < test,
            "{job} must provision siblings before {consumer}"
        );
    }
}

#[test]
fn installed_component_proof_uses_the_accepted_release_profile() {
    // The archived 20-1a CI snapshot is history and keeps its freshly built opt-in
    // binary; the live workflow (Story 21-9) runs the signed default artifacts and
    // builds no substitute `maos` or runner.
    for (live, source, maos_build) in [
        (
            true,
            include_str!("../../.github/workflows/discipline.yml"),
            "",
        ),
        (
            false,
            include_str!("../../_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/ci/ci-20-1a-verify.yml"),
            "cargo build --locked --release -p maos-bin --features wasm-host --bin maos",
        ),
    ] {
        let workflow: serde_yaml::Value = serde_yaml::from_str(source).unwrap();
        let steps = workflow["jobs"]["epic-20-exit"]["steps"]
            .as_sequence()
            .unwrap();
        let build = steps
            .iter()
            .find_map(|step| {
                step["run"]
                    .as_str()
                    .filter(|script| script.contains("-p worker --bin worker-cli-fixture"))
            })
            .expect("worker fixture build must remain enrolled");
        assert!(build.contains("cargo build --locked --release -p worker --bin worker-cli-fixture"));
        let execution = steps
            .iter()
            .find_map(|step| {
                step["run"]
                    .as_str()
                    .filter(|script| script.contains("tee component.log"))
            })
            .expect("real installed-component execution must remain enrolled");
        assert!(execution.contains("/usr/bin/time -p sudo systemd-run"));
        assert!(execution.contains("spirit_session_complete"));
        assert!(!execution.contains("$target/debug/"));
        if live {
            // No fresh binary is substituted for the guest exercise.
            let all = steps
                .iter()
                .filter_map(|s| s["run"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            for forbidden in [
                "cargo build --locked --release -p maos-bin",
                "cargo build --locked --release -p maos-wasm-host",
                "$target/release/maos",
                "target/release/maos-wasm-runner",
            ] {
                assert!(!all.contains(forbidden), "live exit job builds/uses {forbidden}");
            }
            assert!(execution.contains("\"$PWD/dist/maos\" init --plain"));
            assert!(execution.contains("\"$PWD/dist/maos\" run \"$manifest\" --once"));
            assert!(execution.contains("sha256sum -c -"), "AC4 must re-verify the signed sums");
            assert!(execution.contains("MAOS_INFERENCE_MODE=replay"));
            assert!(execution.contains("Replay-fixture witness"));
            // Component exchange from the printed attributable session report.
            assert!(execution.contains("r[\"turns\"]>=1"));
            assert!(execution.contains("delivered_frames") && execution.contains("denied_frames"));
            // The shell dispatch also runs the signed pair under the exact delegated unit.
            let shell = steps
                .iter()
                .find_map(|step| {
                    step["run"]
                        .as_str()
                        .filter(|script| script.contains("example-spirit-ts: echo: hello"))
                })
                .expect("shell dispatch step");
            assert!(shell.contains("systemd-run --unit=maos-17-3c-proof --wait --pipe --collect"));
            assert!(shell.contains("--property=Delegate=yes"));
            assert!(shell.contains("\"$PWD/scripts/run_17_3c_delegated.sh\""));
            assert!(shell.contains("\"$PWD/dist/maos\" shell --plain > shell.out 2> shell.err || shell_rc=$?"));
            assert!(shell.contains("a delivery acknowledgment is NOT that reply"));
            assert!(!shell.contains("| MAOS_INFERENCE_MODE=replay"), "shell must not run outside the unit");
        } else {
            assert!(build.contains(maos_build), "installed proof lacks {maos_build}");
            assert!(build.contains("cargo build --locked --release -p maos-wasm-host --bin maos-wasm-runner"));
            assert!(execution.contains("\"$target/release/maos\" init --plain"));
            assert!(execution.contains("\"$target/release/maos\" run \"$manifest\" --once"));
        }
    }
}
