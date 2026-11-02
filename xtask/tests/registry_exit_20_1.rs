use std::collections::BTreeSet;
use xtask::release_dry_run::ARTIFACTS;
#[test]
fn registry_release_set_and_exit_job_are_enrolled() {
    let binaries: BTreeSet<_> = ARTIFACTS.iter().map(|a| a.binary).collect();
    assert_eq!(
        binaries,
        BTreeSet::from(["maos", "maosctl", "maos-registry-server", "maos-spirit"])
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
        "wasm_engine_off",
        "unknown spirit",
        "--features wasm-host",
        "spirit_session_complete",
    ] {
        assert!(script.contains(required), "exit job lacks {required}");
    }
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
    for source in [
        include_str!("../../.github/workflows/discipline.yml"),
        include_str!("../../_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/ci/ci-20-1a-verify.yml"),
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
                    .filter(|script| script.contains("--features wasm-host --bin maos"))
            })
            .expect("WASM-enabled daemon build must remain enrolled");
        for command in [
            "cargo build --locked --release -p maos-bin --features wasm-host --bin maos",
            "cargo build --locked --release -p maos-wasm-host --bin maos-wasm-runner",
            "cargo build --locked --release -p worker --bin worker-cli-fixture",
        ] {
            assert!(build.contains(command), "installed proof lacks {command}");
        }
        let execution = steps
            .iter()
            .find_map(|step| {
                step["run"]
                    .as_str()
                    .filter(|script| script.contains("tee component.log"))
            })
            .expect("real installed-component execution must remain enrolled");
        assert!(execution.contains("\"$target/release/maos\" init --plain"));
        assert!(execution.contains("\"$target/release/maos\" run \"$manifest\" --once"));
        assert!(execution.contains("/usr/bin/time -p sudo systemd-run"));
        assert!(execution.contains("spirit_session_complete"));
        assert!(!execution.contains("$target/debug/"));
    }
}
