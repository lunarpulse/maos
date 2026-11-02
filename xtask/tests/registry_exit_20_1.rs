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
