#![cfg(unix)]
#![forbid(unsafe_code)]

//! Report-path tests invoke CARGO_BIN_EXE_xtask, serialized on isolated baseline
//! copies. The PATH cargo seam delegates composite legs to a tiny REAL cargo test
//! witness: no canned transcripts, no heavyweight transport/daemon prerequisites.
//! The gate's baseline reader, report/emit code and child spawning remain real.
//! This proves diagnostics, not the composite legs' substantive control claims.

use parking_lot::Mutex;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

static SERIAL: Mutex<()> = Mutex::new(());

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn isolated() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for relative in [
        "crates/maos-kernel-core/src",
        "_bmad-output/planning-artifacts/epics",
    ] {
        copy_tree(&root().join(relative), &dir.path().join(relative));
    }
    fs::create_dir_all(dir.path().join("xtask")).unwrap();
    let sprint = "_bmad-output/implementation-artifacts/sprint-status.yaml";
    fs::create_dir_all(dir.path().join(sprint).parent().unwrap()).unwrap();
    for relative in [
        "xtask/kernel-core-baseline.toml",
        "xtask/gate-registry.toml",
        sprint,
    ] {
        fs::copy(root().join(relative), dir.path().join(relative)).unwrap();
    }
    dir
}

fn executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn command(tree: &Path, bin: &Path, gate: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    let paths = std::iter::once(bin.to_path_buf())
        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>());
    cmd.current_dir(tree)
        .arg(gate)
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("CARGO_BUILD_JOBS", "4")
        .env("CI", "1")
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITHUB_STEP_SUMMARY")
        .env_remove("MAOS_AUDIT_KEY")
        .env("HOME", tree)
        .env("MAOS_LEDGER_ENFORCE", "0");
    cmd
}

fn report(output: &Output, gate: &str) -> Value {
    assert!(!output.status.success(), "{gate} must fail closed");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{gate}: missing/invalid JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["gate"], gate);
    assert!(value["passed"] == false || value["oracle_green"] == false);
    value
}

#[test]
fn abi_tool_failure_preserves_both_streams_and_status() {
    let _guard = SERIAL.lock();
    let dir = tempfile::tempdir().unwrap();
    executable(
        &dir.path().join("cargo"),
        "#!/bin/sh\nprintf 'partial API output\\n'\nprintf 'nightly unavailable\\n' >&2\nexit 42\n",
    );
    for json in [false, true] {
        let mut cmd = command(dir.path(), dir.path(), "abi-diff");
        cmd.args(["--base", "nonexistent-ref"]);
        if json {
            cmd.arg("--json");
        }
        let output = cmd.output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("could not run: nightly unavailable"),
            "{stderr}"
        );
        assert!(!stderr.contains("breaking change detected"), "{stderr}");
        if json {
            let value = report(&output, "abi-diff");
            assert_eq!(value["stdout"], "partial API output\n");
            assert_eq!(value["stderr"], "nightly unavailable\n");
            assert_eq!(value["exit_code"], 42);
        }
    }
}

#[test]
fn abi_denied_diff_is_still_a_breaking_change() {
    let _guard = SERIAL.lock();
    let dir = tempfile::tempdir().unwrap();
    executable(&dir.path().join("cargo"), "#!/bin/sh\nprintf -- '- pub fn removed()\\n'\nprintf 'Error: The API diff is not allowed as per --deny: Removed items not allowed\\n' >&2\nexit 1\n");
    let output = command(dir.path(), dir.path(), "abi-diff")
        .args(["--base", "nonexistent-ref"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("breaking change detected"));
}

#[test]
fn wasm_build_failure_is_not_vacuity() {
    let _guard = SERIAL.lock();
    let tree = isolated();
    executable(
        &tree.path().join("cargo"),
        "#!/bin/sh\nprintf 'error: harness does not compile\\n' >&2\nexit 101\n",
    );
    let output = command(tree.path(), tree.path(), "check-wasm-form-equiv")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("harness failed to build/run"), "{stderr}");
    assert!(!stderr.contains("vacuous"), "{stderr}");
}

#[test]
fn wasm_zero_tests_is_vacuous_but_missing_summary_is_not() {
    let _guard = SERIAL.lock();
    let tree = isolated();
    for (transcript, diagnostic) in [
        (
            "test result: ok. 0 passed; 0 failed; 0 ignored;",
            "is vacuous",
        ),
        (
            "harness exited without a summary",
            "harness failed to build/run",
        ),
    ] {
        executable(
            &tree.path().join("cargo"),
            &format!("#!/bin/sh\nprintf '%s\\n' '{transcript}'\n"),
        );
        let output = command(tree.path(), tree.path(), "check-wasm-form-equiv")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(diagnostic), "{stderr}");
    }
}

fn witness(tree: &Path) {
    let real_cargo = std::env::var_os("CARGO").expect("cargo supplies CARGO");
    let witness = tree.join("witness");
    fs::create_dir_all(witness.join("src")).unwrap();
    fs::write(witness.join("Cargo.toml"), "[package]\nname = \"diagnostic-witness\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[workspace]\n").unwrap();
    fs::write(
        witness.join("src/lib.rs"),
        "#[test]\nfn report_dependency_witness() { assert_eq!(2 + 2, 4); }\n",
    )
    .unwrap();
    // Keep the original rustup home: HOME in the gate is deliberately isolated.
    let rustup = std::env::var_os("RUSTUP_HOME").unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap())
            .join(".rustup")
            .into()
    });
    executable(&tree.join("cargo"), &format!(
        "#!/bin/sh\nexport RUSTUP_HOME='{}'\nexec '{}' test --offline --quiet --manifest-path '{}' --target-dir '{}' --lib\n",
        Path::new(&rustup).display(), Path::new(&real_cargo).display(), witness.join("Cargo.toml").display(), witness.join("target").display()
    ));
}

fn composite_case(gate: &str, infrastructure_error: bool) {
    let _guard = SERIAL.lock();
    let tree = isolated();
    witness(tree.path());
    let pin = tree.path().join("xtask/kernel-core-baseline.toml");
    if infrastructure_error {
        fs::remove_file(&pin).unwrap();
    } else if gate == "check-epic-close-coherence" {
        let sprint = tree
            .path()
            .join("_bmad-output/implementation-artifacts/sprint-status.yaml");
        let statuses = fs::read_to_string(&sprint).unwrap().replacen(
            "development_status:",
            "development_status:\n  epic-999: done\n  999-1-planted-open-story: in-progress",
            1,
        );
        fs::write(&sprint, statuses).unwrap();
    } else {
        let original = fs::read_to_string(&pin).unwrap();
        let old = original
            .lines()
            .find(|line| line.starts_with("src_lines ="))
            .unwrap();
        let number: u64 = old.split_once('=').unwrap().1.trim().parse().unwrap();
        fs::write(
            &pin,
            original.replace(old, &format!("src_lines = {}", number + 1)),
        )
        .unwrap();
    }
    let output = command(tree.path(), tree.path(), gate)
        .arg("--json")
        .output()
        .unwrap();
    let value = report(&output, gate);
    if gate == "check-multi-tenant-loom" {
        let expected = if infrastructure_error {
            "kernel baseline check ERRORED"
        } else {
            "kernel baseline mismatch"
        };
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(expected),
            "{value}"
        );
    } else if !infrastructure_error && gate == "check-cohort-mesh" {
        assert!(String::from_utf8_lossy(&output.stderr).contains("kernel-abi-diff RED"));
    } else if !infrastructure_error {
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("epic-over-stories"),
            "{value}"
        );
    }
}

#[test]
fn cohort_mismatch_emits_json() {
    composite_case("check-cohort-mesh", false);
}
#[test]
fn cohort_pin_read_error_emits_json() {
    composite_case("check-cohort-mesh", true);
}
#[test]
fn loom_mismatch_emits_json() {
    composite_case("check-multi-tenant-loom", false);
}
#[test]
fn loom_pin_read_error_emits_json() {
    composite_case("check-multi-tenant-loom", true);
}
#[test]
fn epic_mismatch_emits_json() {
    composite_case("check-epic-close-coherence", false);
}
#[test]
fn epic_pin_read_error_emits_json() {
    composite_case("check-epic-close-coherence", true);
}

#[test]
fn epic_status_read_error_emits_json() {
    let _guard = SERIAL.lock();
    let tree = isolated();
    fs::remove_file(
        tree.path()
            .join("_bmad-output/implementation-artifacts/sprint-status.yaml"),
    )
    .unwrap();
    let output = command(tree.path(), tree.path(), "check-epic-close-coherence")
        .arg("--json")
        .output()
        .unwrap();
    let value = report(&output, "check-epic-close-coherence");
    assert!(value["error"]
        .as_str()
        .unwrap()
        .contains("no development_status"));
}

#[test]
fn composite_child_spawn_error_emits_json() {
    let _guard = SERIAL.lock();
    let tree = isolated();
    // A non-executable cargo in a PATH containing no other cargo plants the real
    // OS command-spawn error, rather than setting an application test-only flag.
    fs::write(tree.path().join("cargo"), "not executable").unwrap();
    for gate in ["check-cohort-mesh", "check-multi-tenant-loom"] {
        let output = command(tree.path(), tree.path(), gate)
            .env("PATH", tree.path())
            // Binding uses the normal Actions environment so no git executable
            // is needed when this vector deliberately removes PATH tools.
            .env("GITHUB_ACTIONS", "true")
            .env("GITHUB_SHA", "diagnostic-vector")
            .env("GITHUB_RUN_ID", "1")
            .env("GITHUB_RUN_ATTEMPT", "1")
            .arg("--json")
            .output()
            .unwrap();
        report(&output, gate);
    }
}
