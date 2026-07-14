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

use std::io;
use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::security::sandbox::{
    classify_exit, spawn_sandboxed, SandboxSpec, SpawnError,
};

fn skip_if_perm_denied<T>(result: Result<T, SpawnError>, test_name: &str) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(SpawnError::Io(e)) if e.kind() == io::ErrorKind::PermissionDenied => {
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

fn probe_binary_path() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::PathBuf::from(format!(
        "{manifest_dir}/test-fixtures/forbidden-syscall-probe/target/release/forbidden-syscall-probe"
    ));
    assert!(
        path.exists(),
        "AC4's T2 proof requires the forbidden-syscall-probe binary at {} — \
         build it first: `cargo build --release` in \
         crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/",
        path.display()
    );
    path
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
        "a forbidden ptrace(2) syscall must NOT be allowed to complete under T2"
    );

    // DERIVED cause attribution via classify_exit — the same function the
    // kernel uses to produce the sandbox audit row (AC4: "+ a sandbox audit
    // row"). NOT a bare `exit_code != 0` check.
    let violation = classify_exit(status)
        .expect("T2 must classify this exit as a SandboxViolation, not an ordinary failure");
    assert_eq!(
        violation.sandbox_tier,
        SandboxTier::T2,
        "violation must be attributed to T2"
    );

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(libc::SIGSYS),
            "seccomp KillProcess on a hostile syscall reports as SIGSYS per Linux semantics"
        );
    }
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
    assert!(
        status.success(),
        "a benign process under the identical T2 spec must survive — \
         proves the forbidden-syscall kill is caused by the syscall, not the sandbox itself"
    );
    assert!(
        classify_exit(status).is_none(),
        "a clean exit must not be misclassified as a sandbox violation"
    );
}

/// AC4's load-bearing negative control: "a granted capability works while
/// an un-granted fs/net capability is refused." Two T2 specs differ ONLY in
/// `declared_scopes` (one grants `Scope::FsRead` for a tempdir, the other
/// grants nothing); both spawn a shell that tries to `cat` a file inside
/// that tempdir. The granted spec must succeed; the ungranted spec must be
/// denied by Landlock — proving the capability gate is load-bearing, not
/// merely declared.
#[test]
fn granted_fs_capability_works_ungranted_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("secret.txt");
    std::fs::write(&file_path, b"capability-gated content").unwrap();
    let subtree = dir.path().to_str().unwrap().to_string();
    let cat_cmd = || {
        let mut cmd = Command::new("/bin/cat");
        cmd.arg(&file_path);
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
    let granted_status = granted_child.wait().unwrap();
    assert!(
        granted_status.success(),
        "a granted FsRead capability must allow reading the declared subtree"
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
    let ungranted_status = ungranted_child.wait().unwrap();
    assert!(
        !ungranted_status.success(),
        "an UNgranted FsRead capability must refuse reading the same subtree \
         (Landlock must deny — this is the load-bearing negative half of AC4)"
    );
}
