use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}
fn tree() -> tempfile::TempDir {
    let tree = tempfile::tempdir().unwrap();
    fs::create_dir(tree.path().join("xtask")).unwrap();
    fs::copy(
        root().join("xtask/gate-registry.toml"),
        tree.path().join("xtask/gate-registry.toml"),
    )
    .unwrap();
    tree
}
fn run(tree: &Path, gate: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([gate, "--json"])
        .current_dir(tree)
        .env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .env_remove("MAOS_MIGRATION_CORPUS")
        .env_remove("MAOS_TEST_POSTGRES")
        .output()
        .unwrap()
}
fn report(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}
#[test]
fn absent_inputs_never_pass() {
    for (gate, rc) in [
        ("check-pentest-gate", 0),
        ("check-red-team-gate", 0),
        ("check-skill-conformance", 1),
        ("check-fuzz-floor", 1),
        ("check-migration-merkle", 1),
        ("check-rto-gate", 1),
    ] {
        let tree = tree();
        let out = run(tree.path(), gate);
        let value = report(&out);
        assert_eq!(value["passed"], false, "{gate}: {value}");
        assert_eq!(value["evidence_state"], "ABSENT", "{gate}: {value}");
        assert_eq!(out.status.code(), Some(rc), "{gate}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(if rc == 0 { "::warning::" } else { "::error::" }),
            "{stderr}"
        );
        assert!(value.get("skipped").is_none());
    }
}
#[test]
fn empty_fuzz_ledger_is_absent() {
    let tree = tree();
    fs::copy(
        root().join("fuzz-ledger.json"),
        tree.path().join("fuzz-ledger.json"),
    )
    .unwrap();
    let out = run(tree.path(), "check-fuzz-floor");
    let value = report(&out);
    assert_eq!(value["evidence_state"], "ABSENT");
    assert_eq!(value["passed"], false);
    assert_eq!(out.status.code(), Some(1));
}
#[test]
fn young_fuzz_ledger_is_indeterminate() {
    let tree = tree();
    let ts = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
    fs::write(tree.path().join("fuzz-ledger.json"),serde_json::json!({"records":[{"target":"manifest_parser","cpu_seconds":100,"timestamp":ts,"tier":"T2"}]}).to_string()).unwrap();
    let out = run(tree.path(), "check-fuzz-floor");
    let value = report(&out);
    assert_eq!(value["evidence_state"], "INDETERMINATE");
    assert_eq!(value["passed"], false);
    assert_eq!(value["advisory"], true);
    assert!(out.status.success());
    assert_eq!(value["aggregate_seconds"], 100);
    assert_eq!(value["per_target_seconds"]["manifest_parser"], 100);
}
/// A ledger spanning the full window that meets every floor is PROVEN: the
/// oracle re-derives the floor from the collected ledger. (Classing the gate
/// `advisory-substrate` would project it INDETERMINATE forever — unsigned.)
#[test]
fn mature_fuzz_ledger_meeting_the_floor_passes() {
    let tree = tree();
    let old = (chrono::Utc::now() - chrono::Duration::days(95)).to_rfc3339();
    let new = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
    let mut records =
        vec![serde_json::json!({"target":"manifest_parser","cpu_seconds":1,"timestamp":old})];
    for target in ["manifest_parser", "frame_deser", "fuzz_exec_deps_parse_elf"] {
        records.push(serde_json::json!({"target":target,"cpu_seconds":1_300_000,"timestamp":new}));
    }
    fs::write(
        tree.path().join("fuzz-ledger.json"),
        serde_json::json!({ "records": records }).to_string(),
    )
    .unwrap();
    let out = run(tree.path(), "check-fuzz-floor");
    let value = report(&out);
    assert_eq!(value["evidence_state"], "PROVEN_BLOCKING", "{value}");
    assert_eq!(value["passed"], true, "{value}");
    assert!(out.status.success(), "{out:?}");
}
/// A corrupt ledger is a RED finding that still reports itself as JSON.
#[test]
fn malformed_fuzz_ledger_is_red_with_a_report() {
    let tree = tree();
    let ts = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
    fs::write(
        tree.path().join("fuzz-ledger.json"),
        serde_json::json!({"records":[{"target":"manifest_parser","cpu_seconds":"100","timestamp":ts}]})
            .to_string(),
    )
    .unwrap();
    let out = run(tree.path(), "check-fuzz-floor");
    let value = report(&out);
    assert_eq!(value["evidence_state"], "INDETERMINATE", "{value}");
    assert_eq!(value["passed"], false);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(value["reason"]
        .as_str()
        .unwrap()
        .contains("malformed evidence"));
}
#[test]
fn trial_real_missing_kernel_leg_blocks() {
    // The real kernel-baseline oracle sees an absent source tree, not a fabricated leg.
    let tree = tree();
    let out = run(tree.path(), "check-trial-attestation");
    let value = report(&out);
    assert_eq!(value["passed"], false);
    assert_eq!(out.status.code(), Some(1));
    assert!(value["legs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|leg| leg["label"] == "kernel-abi-diff" && leg["green"] == false));
}
#[cfg(unix)]
#[test]
fn escape_substrate_refusal_never_passes() {
    use std::os::unix::fs::PermissionsExt;
    // Inject subprocess transcripts at the existing cargo process seam. The gate
    // itself parses counts/markers, classifies legs and decides JSON/exit.
    let tree = tree();
    fn copy_dir(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &dst.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
            }
        }
    }
    copy_dir(
        &root().join("crates/maos-kernel-core/src"),
        &tree.path().join("crates/maos-kernel-core/src"),
    );
    fs::copy(
        root().join("xtask/kernel-core-baseline.toml"),
        tree.path().join("xtask/kernel-core-baseline.toml"),
    )
    .unwrap();
    let bin = tree.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    fs::write(&cargo,"#!/bin/sh\ncase \"$*\" in\n *build*) echo escape-fault-inject >&2; exit 1;;\n *tree*) exit 0;;\n *producer_wired_e2e*|*detection_quality_meets_floor*) echo 'SKIP test: sandbox unavailable';;\n *) echo ESCAPE-DETECTOR-QUALITY-MEASURED;;\nesac\necho 'test result: ok. 1 passed; 0 failed;'\n").unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-escape-detector", "--json"])
        .current_dir(tree.path())
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .output()
        .unwrap();
    let value = report(&out);
    assert_eq!(value["passed"], false, "{value}");
    assert_eq!(value["evidence_state"], "INDETERMINATE");
    assert_eq!(value["advisory"], true, "{value}");
    assert!(out.status.success(), "{out:?}");
}

#[cfg(unix)]
#[test]
fn workflow_always_invokes_gates_on_absence() {
    use std::os::unix::fs::PermissionsExt;
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/discipline.yml")).unwrap(),
    )
    .unwrap();
    for gate in [
        "check-pentest-gate",
        "check-red-team-gate",
        "check-migration-merkle",
    ] {
        let tree = tree();
        let cargo = tree.path().join("cargo");
        fs::write(
            &cargo,
            "#!/bin/sh\nprintf '%s\\n' \"$*\" > invoked\nexit 23\n",
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
        let steps = workflow["jobs"][gate]["steps"].as_sequence().unwrap();
        let script = steps
            .iter()
            .filter_map(|step| step["run"].as_str())
            .next_back()
            .unwrap();
        let output = Command::new("bash")
            .args(["-e", "-c", script])
            .current_dir(tree.path())
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    tree.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("GITHUB_STEP_SUMMARY", tree.path().join("summary"))
            .env_remove("MAOS_TEST_POSTGRES")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(23), "{gate}: {output:?}");
        assert!(fs::read_to_string(tree.path().join("invoked"))
            .unwrap()
            .contains(gate));
    }
}
