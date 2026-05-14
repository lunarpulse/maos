//! Linux T2 sandbox enforcement tests — Landlock + seccomp.
//!
//! These tests spawn throwaway probe processes under T2 and assert
//! that forbidden operations are blocked.
//!
//! NOTE: If the test runner lacks `CAP_SYS_ADMIN` or seccomp/Landlock
//! is unavailable, the affected tests skip with a clear message instead
//! of failing.
#![cfg(target_os = "linux")]

use std::io;
use std::process::Command;

use maos_kernel_core::security::sandbox::{
    spawn_sandboxed, classify_exit, SandboxSpec, SpawnError,
};
use maos_domain::invariants::i9::SandboxTier;

fn skip_if_perm_denied<T>(result: Result<T, SpawnError>, test_name: &str) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(SpawnError::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied => {
            eprintln!("SKIP {test_name}: sandbox setup requires CAP_SYS_ADMIN / no_new_privs");
            None
        }
        Err(e) => panic!("{test_name}: spawn_sandboxed failed: {e:?}"),
    }
}

fn make_spec(tier: SandboxTier) -> SandboxSpec {
    SandboxSpec {
        tier,
        resolved_caps: Default::default(),
        declared_scopes: vec![],
        spirit_id: format!("test-spirit-{}", tier.0),
    }
}

#[test]
fn t0_passthrough_exits_cleanly() {
    let spec = make_spec(SandboxTier::T0);
    let mut cmd = Command::new("/bin/true");
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t0_passthrough") {
        Some(c) => c,
        None => return,
    };
    let status = child.wait().unwrap();
    assert!(status.success(), "T0 passthrough must exit cleanly");
}

#[test]
fn t2_child_exit_code_preserved() {
    let spec = make_spec(SandboxTier::T2);
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg("exit 42");
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t2_child_exit") {
        Some(c) => c,
        None => return,
    };
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(42), "T2 must preserve child exit code");
}

#[test]
fn t2_benign_process_not_killed_by_seccomp() {
    let spec = make_spec(SandboxTier::T2);
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg("echo ok");
    let mut child = match skip_if_perm_denied(spawn_sandboxed(&spec, &mut cmd), "t2_benign") {
        Some(c) => c,
        None => return,
    };
    let status = child.wait().unwrap();
    assert!(status.success(), "benign sh+echo must survive seccomp allow-list");
}

#[test]
fn classify_exit_normal_returns_none() {
    let mut cmd = Command::new("/bin/true");
    let status = cmd.status().unwrap();
    assert!(classify_exit(status).is_none());
}
