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

#[test]
fn unreadable_security_evidence_is_red_with_a_report() {
    for (gate, path) in [
        ("check-pentest-gate", "docs/pen-test/findings/summary.toml"),
        (
            "check-red-team-gate",
            "docs/red-team/results/red-team-results.toml",
        ),
    ] {
        let tree = tree();
        let path = tree.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, [0xff, 0xfe]).unwrap();
        let out = run(tree.path(), gate);
        let value = report(&out);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        assert_eq!(value["gate"], gate);
        assert_eq!(value["passed"], false);
        assert_ne!(value["evidence_state"], "ABSENT");
        assert!(value["error"].as_str().unwrap().contains("cannot read"));
    }
}

#[test]
fn malformed_pentest_toml_is_red_with_a_report() {
    let tree = tree();
    let path = tree.path().join("docs/pen-test/findings/summary.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "[gate\n").unwrap();
    let out = run(tree.path(), "check-pentest-gate");
    let value = report(&out);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(value["gate"], "check-pentest-gate");
    assert_eq!(value["passed"], false);
    assert_ne!(value["evidence_state"], "ABSENT");
    assert!(value["error"].as_str().unwrap().contains("malformed"));
}

#[test]
fn empty_rto_ledger_is_absent_with_a_report() {
    let tree = tree();
    fs::write(
        tree.path().join("xtask/rto-evidence.toml"),
        "evidence = []\n",
    )
    .unwrap();
    let out = run(tree.path(), "check-rto-gate");
    let value = report(&out);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(value["gate"], "check-rto-gate");
    assert_eq!(value["evidence_state"], "ABSENT");
    assert_eq!(value["passed"], false);
}

#[test]
fn malformed_evidence_gates_emit_named_red_reports() {
    for (gate, path) in [
        (
            "check-red-team-gate",
            "docs/red-team/results/red-team-results.toml",
        ),
        ("check-rto-gate", "xtask/rto-evidence.toml"),
        (
            "check-skill-conformance",
            "docs/skill-conformance/results/skill-conformance-results.toml",
        ),
    ] {
        let tree = tree();
        let path = tree.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "[invalid\n").unwrap();
        let out = run(tree.path(), gate);
        let value = report(&out);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        assert_eq!(value["gate"], gate);
        assert_eq!(value["passed"], false);
        assert_ne!(value["evidence_state"], "ABSENT");
        assert!(value["error"].as_str().is_some(), "{value}");
    }
}

#[test]
fn migration_manifest_and_corpus_errors_emit_named_red_reports() {
    for valid_manifest in [false, true] {
        let tree = tree();
        let corpus = tree
            .path()
            .join("tests/corpora/migration-corpus-1e6.sqlite");
        fs::create_dir_all(corpus.parent().unwrap()).unwrap();
        fs::write(corpus, b"not a SQLite database").unwrap();
        let manifest = tree.path().join("tests/corpora/MANIFEST.toml");
        if valid_manifest {
            fs::copy(root().join("tests/corpora/MANIFEST.toml"), manifest).unwrap();
        } else {
            fs::write(manifest, "[invalid\n").unwrap();
        }
        let out = run(tree.path(), "check-migration-merkle");
        let value = report(&out);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        assert_eq!(value["gate"], "check-migration-merkle");
        assert_eq!(value["passed"], false);
        assert_ne!(value["evidence_state"], "ABSENT");
        assert!(value["error"].as_str().is_some(), "{value}");
    }
}

fn check_gate_workflow(workflow: &serde_yaml::Value) -> Result<Vec<(&'static str, &str)>, String> {
    let mut scripts = Vec::new();
    for (gate, name) in [
        ("check-pentest-gate", "Check pen-test results"),
        ("check-third-party-trial", "Check third-party trial results"),
        ("check-red-team-gate", "Check red-team gate results"),
        (
            "check-migration-merkle",
            "Check migration Merkle gate results",
        ),
    ] {
        let job = workflow["jobs"][gate]
            .as_mapping()
            .ok_or_else(|| format!("missing job {gate}"))?;
        for key in ["if", "continue-on-error"] {
            if job.contains_key(&serde_yaml::Value::from(key)) {
                return Err(format!("{gate}: job has {key}"));
            }
        }
        let steps = workflow["jobs"][gate]["steps"]
            .as_sequence()
            .ok_or_else(|| format!("{gate}: missing steps"))?;
        let step = steps
            .iter()
            .find(|step| step["name"].as_str() == Some(name))
            .ok_or_else(|| format!("{gate}: missing named gate step {name}"))?;
        let mapping = step
            .as_mapping()
            .ok_or_else(|| format!("{gate}: invalid step"))?;
        for key in ["if", "continue-on-error"] {
            if mapping.contains_key(&serde_yaml::Value::from(key)) {
                return Err(format!("{gate}: step has {key}"));
            }
        }
        let script = step["run"]
            .as_str()
            .ok_or_else(|| format!("{gate}: missing run"))?;
        if !script.split_whitespace().any(|word| word == gate) {
            return Err(format!("{gate}: named step does not invoke gate"));
        }
        scripts.push((gate, script));
    }
    Ok(scripts)
}

#[test]
fn workflow_checker_rejects_evidence_condition_plant() {
    let mut workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/discipline.yml")).unwrap(),
    )
    .unwrap();
    let steps = workflow["jobs"]["check-pentest-gate"]["steps"]
        .as_sequence_mut()
        .unwrap();
    let step = steps
        .iter_mut()
        .find(|step| step["name"] == "Check pen-test results")
        .unwrap();
    step["if"] = serde_yaml::Value::from("hashFiles('docs/pen-test/findings/summary.toml') != ''");
    let error = check_gate_workflow(&workflow).unwrap_err();
    assert!(error.contains("check-pentest-gate: step has if"), "{error}");
}

#[cfg(unix)]
#[test]
fn workflow_always_invokes_gates_on_absence() {
    use std::os::unix::fs::PermissionsExt;
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/discipline.yml")).unwrap(),
    )
    .unwrap();
    for (gate, script) in check_gate_workflow(&workflow).unwrap() {
        let tree = tree();
        let cargo = tree.path().join("cargo");
        fs::write(
            &cargo,
            "#!/bin/sh\nprintf '%s\\n' \"$*\" > invoked\nexit 23\n",
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
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
