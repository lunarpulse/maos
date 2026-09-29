//! Story 11.1a AC4 — the T2 (sandbox-kill) column of the fuel<->T2 2x2
//! matrix, missing from `fuel_t2_matrix.rs` (that file only exercises the
//! wasmtime fuel layer; see its own module doc). This file proves the OTHER
//! half: "the forbidden-syscall guest with fuel=`u64::MAX` is killed by T2
//! with the syscall signature (SIGSYS/EACCES/Job-Object) + a sandbox audit
//! row" (AC4), using the kernel's REAL `spawn_sandboxed`/`classify_exit` —
//! not a mock, not a re-implementation.
//!
//! The "guest" here is a native probe binary (`forbidden-syscall-probe`,
//! `test-fixtures/forbidden-syscall-probe`) that issues a raw `ptrace(2)`
//! syscall — present on the kernel's own T2 seccomp `hostile_syscalls`
//! KillProcess list (`maos-kernel-core/src/security/sandbox/linux.rs`).
//! `BridgeSpawnSpec.program` is form-agnostic (11.0 spike finding): the
//! kernel's T2 enforcement does not care whether the supervised binary is a
//! native Spirit or `maos-wasm-runner` — it sandboxes whatever `program` is.
//! Proving T2 kills a forbidden-syscall process under `spawn_sandboxed`
//! proves the backstop this story's WASM runner inherits unconditionally
//! (the runner is launched through the SAME kernel T2 path per AC4's
//! "Given the runner under T2" — wiring `maos-wasm-runner` as `program`
//! changes nothing about T2's enforcement, which is OS-level and
//! program-agnostic by construction).
#![cfg(target_os = "linux")]

use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::LazyLock;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::security::sandbox::{
    classify_exit, spawn_sandboxed, SandboxSpec, SpawnError,
};

/// Off CI a host that refuses the sandbox skips with a named reason; in CI
/// (`CI`/`GITHUB_ACTIONS` set) every runner is declared capable, so the same
/// refusal is a named failure, never a skip (rule 11(b); Story 17-6 AC4).
fn skip_if_perm_denied<T>(result: Result<T, SpawnError>, test_name: &str) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(SpawnError::Io(e)) if e.kind() == io::ErrorKind::PermissionDenied => {
            if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
                panic!(
                    "{test_name}: T2 spawn refused on a runner declared capable (rule 11(b)): \
                     {:?}: {e} — CI/GITHUB_ACTIONS declares this runner capable; a job that \
                     cannot sandbox must be declared incapable in this helper and its YAML \
                     (owner + reason), never skipped",
                    e.kind()
                );
            }
            eprintln!("SKIP {test_name}: sandbox setup requires CAP_SYS_ADMIN / no_new_privs");
            None
        }
        Err(e) => panic!("{test_name}: spawn_sandboxed failed: {e:?}"),
    }
}

fn t2_spec(spirit_id: &str) -> SandboxSpec {
    SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![],
        spirit_id: spirit_id.to_string(),
        output_shape_predicate: None,
    }
}

/// CI prebuilds this fixture and its freshness is checked here; locally Cargo
/// confirms the release probe is current on first use. Concurrent test binaries
/// may race the check, but Cargo's target directory lock serializes a rebuild.
static PROBE_BINARY: LazyLock<PathBuf> = LazyLock::new(build_probe_binary);

fn probe_binary_path() -> PathBuf {
    PROBE_BINARY.clone()
}

fn build_probe_binary() -> PathBuf {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-fixtures/forbidden-syscall-probe");
    let binary = fixture.join("target/release/forbidden-syscall-probe");
    if !ci_has_prebuilt_probe() || fixture_needs_rebuild(&fixture, &binary) {
        let status = Command::new("cargo")
            .args(["build", "--locked", "--release", "--manifest-path"])
            .arg(fixture.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(fixture.join("target"))
            .env_remove("CARGO_TARGET_DIR")
            .status()
            .expect("start current forbidden-syscall-probe release build");
        assert!(
            status.success(),
            "forbidden-syscall-probe release build failed in {}",
            fixture.display()
        );
    }
    assert!(
        binary.is_file(),
        "AC4's T2 proof requires the forbidden-syscall-probe binary at {}",
        binary.display()
    );
    binary
}

fn fixture_needs_rebuild(fixture: &Path, binary: &Path) -> bool {
    let Ok(binary_modified) = binary.metadata().and_then(|metadata| metadata.modified()) else {
        return true;
    };
    ["Cargo.toml", "Cargo.lock", "src/main.rs"]
        .iter()
        .any(|input| {
            fixture
                .join(input)
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_or(true, |modified| modified > binary_modified)
        })
}

fn ci_has_prebuilt_probe() -> bool {
    std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some()
}

/// One observable line per test, printed with `--nocapture`, that the CI step
/// turns into a public annotation (Story 17-6 AC1): the test and the exact
/// `ExitStatus` it saw.
fn outcome(test: &str, status: &ExitStatus) {
    println!("T2-OUTCOME {test}: {status}");
}

fn outcome_with_detail(test: &str, status: &ExitStatus, detail: impl std::fmt::Display) {
    println!("T2-OUTCOME {test}: {status}; {detail}");
}

/// Read one piped stream of a sandboxed child to the end.
fn read_all(stream: Option<impl Read>) -> String {
    let mut text = String::new();
    stream
        .expect("stream was piped")
        .read_to_string(&mut text)
        .expect("read child stream");
    text
}

/// AC4 (the missing T2 cell): "the forbidden-syscall guest with
/// fuel=`u64::MAX` is killed by T2 with the syscall signature
/// (SIGSYS/EACCES/Job-Object) + a sandbox audit row." Fuel is irrelevant
/// here by construction (this is a native probe, not a wasmtime guest) —
/// the point is proving the T2 backstop the runner inherits is real and
/// load-bearing, independent of fuel metering entirely.
#[test]
fn forbidden_syscall_killed_by_t2_with_sigsys() {
    let spec = t2_spec("test-spirit-t2-forbidden-syscall");
    let mut cmd = Command::new(probe_binary_path());
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "forbidden_syscall")
    {
        Some(c) => c,
        None => return,
    };
    let status = child.wait().unwrap();

    assert!(
        !status.success(),
        "a forbidden ptrace(2) syscall must NOT be allowed to complete under T2 (observed {status})"
    );

    // DERIVED cause attribution via classify_exit — the same function the
    // kernel uses to produce the sandbox audit row (AC4: "+ a sandbox audit
    // row"). NOT a bare `exit_code != 0` check.
    let violation = classify_exit(status).unwrap_or_else(|| {
        panic!(
            "T2 must classify this exit as a SandboxViolation, not an ordinary failure \
             (observed {status})"
        )
    });
    outcome_with_detail(
        "forbidden_syscall_killed_by_t2_with_sigsys",
        &status,
        format_args!("classified_violation_tier={}", violation.sandbox_tier),
    );
    assert_eq!(
        violation.sandbox_tier,
        SandboxTier::T2,
        "violation must be attributed to T2 (observed {status})"
    );
    assert_eq!(
        status.signal(),
        Some(libc::SIGSYS),
        "seccomp KillProcess on a hostile syscall reports as SIGSYS per Linux semantics \
         (observed {status})"
    );
}

/// The load-bearing negative control (AC4): a benign process under the SAME
/// T2 spec completes cleanly — proves the kill above is caused by the
/// forbidden syscall, not by T2 itself being globally hostile.
#[test]
fn benign_process_survives_t2_under_same_spec() {
    let spec = t2_spec("test-spirit-t2-benign-control");
    let mut cmd = Command::new("/bin/true");
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t2_benign_control")
    {
        Some(c) => c,
        None => return,
    };
    let status = child.wait().unwrap();
    outcome("benign_process_survives_t2_under_same_spec", &status);
    assert!(
        status.success(),
        "a benign process under the identical T2 spec must survive — proves the \
         forbidden-syscall kill is caused by the syscall, not the sandbox itself (observed {status})"
    );
    assert!(
        classify_exit(status).is_none(),
        "a clean exit must not be misclassified as a sandbox violation (observed {status})"
    );
}

/// Story 17-6 D-17-6-H — the Rust negative control. The SAME probe binary in
/// `--benign` mode (spawn + join one thread, print, exit 0) under the identical
/// T2 spec: a Rust program reaches `main` (`poll`, `rt_sigaction` in std init)
/// and starts a thread (`clone3`). Dropping any of those from the allow-list
/// reds this test — signal 11 or rc 101 — while the ptrace mode still dies of
/// SIGSYS.
#[test]
fn benign_rust_probe_survives_t2_under_same_spec() {
    let spec = t2_spec("test-spirit-t2-benign-rust");
    let mut cmd = Command::new(probe_binary_path());
    cmd.arg("--benign").stdout(Stdio::piped());
    let mut child =
        match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t2_benign_rust_control") {
            Some(c) => c,
            None => return,
        };
    let stdout = read_all(child.child_mut().stdout.take());
    let status = child.wait().unwrap();
    outcome("benign_rust_probe_survives_t2_under_same_spec", &status);
    assert!(
        status.success(),
        "a Rust binary under the identical T2 spec must reach main, start a thread and exit 0 \
         (observed {status}; stdout {stdout:?})"
    );
    assert!(
        stdout.contains("forbidden-syscall-probe: benign ok"),
        "the benign mode must have run to its print (observed {status}; stdout {stdout:?})"
    );
    assert!(
        classify_exit(status).is_none(),
        "a clean exit must not be misclassified as a sandbox violation (observed {status})"
    );
}

/// Story 17-6 AC3 — T2 must allow exactly `prctl(PR_GET_AUXV)`, required by
/// current coreutils, while denying unrelated `prctl` options by the seccomp
/// default action. The native probe executes both paths under the real Linux
/// T2 setup; it refuses to report success unless the denied calls return
/// precisely `-1/EPERM`.
#[test]
fn pr_get_auxv_is_allowed_other_prctl_options_are_denied_by_t2() {
    let spec = t2_spec("test-spirit-t2-prctl-auxv");
    let mut cmd = Command::new(probe_binary_path());
    cmd.arg("--auxv")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t2_prctl_auxv") {
        Some(c) => c,
        None => return,
    };
    let stdout = read_all(child.child_mut().stdout.take());
    let stderr = read_all(child.child_mut().stderr.take());
    let status = child.wait().unwrap();
    outcome(
        "pr_get_auxv_is_allowed_other_prctl_options_are_denied_by_t2",
        &status,
    );

    assert!(
        status.success(),
        "PR_GET_AUXV must succeed while other prctl options are denied under T2 \
         (observed {status}; stdout {stdout:?}; stderr {stderr:?})"
    );
    assert!(
        stderr.is_empty(),
        "the successful probe must not report an unexpected syscall result: {stderr:?}"
    );
    let bytes = stdout
        .trim()
        .strip_prefix("forbidden-syscall-probe: PR_GET_AUXV bytes=")
        .and_then(|text| text.strip_suffix("; PR_GET_DUMPABLE,PR_GET_NAME denied=EPERM"))
        .and_then(|text| text.parse::<i32>().ok());
    assert!(
        matches!(bytes, Some(bytes) if bytes > 0),
        "probe must report a positive PR_GET_AUXV byte count and exact -1/EPERM \
         denials for PR_GET_DUMPABLE and PR_GET_NAME; stdout {stdout:?}"
    );
    assert!(
        classify_exit(status).is_none(),
        "a successful, narrowly allowed prctl call must not be classified as a sandbox violation"
    );
}

/// AC4's load-bearing negative control: "a granted capability works while
/// an un-granted fs/net capability is refused." Two T2 specs differ ONLY in
/// `declared_scopes` (one grants `Scope::FsRead` for a tempdir, the other
/// grants nothing); both spawn `cat` on a file inside that tempdir. The granted
/// spec must print the content; the ungranted spec must be denied by Landlock
/// ON THE READ — `cat` itself started (its exec set is granted, Story 17-6 AC2)
/// and its stderr names the file with `Permission denied`, never a failed exec.
#[test]
fn granted_fs_capability_works_ungranted_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("secret.txt");
    std::fs::write(&file_path, b"capability-gated content").unwrap();
    let subtree = dir.path().to_str().unwrap().to_string();
    let cat_cmd = || {
        let mut cmd = Command::new("/bin/cat");
        cmd.arg(&file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    };

    // Granted: T2 spec declares Scope::FsRead for the tempdir.
    let granted_spec = SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![maos_domain::invariants::i1::Scope::FsRead {
            subtree: subtree.clone(),
        }],
        spirit_id: "test-spirit-t2-fs-granted".to_string(),
        output_shape_predicate: None,
    };
    let mut granted_child =
        match skip_if_perm_denied(spawn_sandboxed(&granted_spec, &mut cat_cmd()), "fs_granted") {
            Some(c) => c,
            None => return,
        };
    let granted_out = read_all(granted_child.child_mut().stdout.take());
    let granted_err = read_all(granted_child.child_mut().stderr.take());
    let granted_status = granted_child.wait().unwrap();
    outcome(
        "granted_fs_capability_works_ungranted_is_refused/granted",
        &granted_status,
    );
    assert!(
        granted_status.success() && granted_out == "capability-gated content",
        "a granted FsRead capability must allow reading the declared subtree \
         (observed {granted_status}; stdout {granted_out:?}; stderr {granted_err:?})"
    );

    // Ungranted: identical T2 spec, but declared_scopes is empty — no
    // filesystem capability at all.
    let ungranted_spec = SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![],
        spirit_id: "test-spirit-t2-fs-ungranted".to_string(),
        output_shape_predicate: None,
    };
    let mut ungranted_child = match skip_if_perm_denied(
        spawn_sandboxed(&ungranted_spec, &mut cat_cmd()),
        "fs_ungranted",
    ) {
        Some(c) => c,
        None => return,
    };
    let ungranted_out = read_all(ungranted_child.child_mut().stdout.take());
    let ungranted_err = read_all(ungranted_child.child_mut().stderr.take());
    let ungranted_status = ungranted_child.wait().unwrap();
    outcome_with_detail(
        "granted_fs_capability_works_ungranted_is_refused/ungranted",
        &ungranted_status,
        format_args!(
            "denied_file={}; cat_stderr={ungranted_err:?}",
            file_path.display()
        ),
    );
    assert!(
        !ungranted_status.success() && ungranted_out.is_empty(),
        "an UNgranted FsRead capability must refuse reading the same subtree \
         (observed {ungranted_status}; stdout {ungranted_out:?})"
    );
    assert!(
        ungranted_err.contains(file_path.to_str().unwrap())
            && ungranted_err.contains("Permission denied"),
        "the refusal must be Landlock denying cat's READ of the named file — not a failed exec \
         (observed {ungranted_status}; stderr {ungranted_err:?})"
    );
}
