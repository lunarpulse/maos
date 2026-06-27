//! Windows sandbox enforcement tests — restricted token + Job Object.
//!
//! Exercises the REAL unsafe FFI spawn path in
//! `security/sandbox/windows.rs` (CreateRestrictedToken + low-integrity label
//! + CreateProcessAsUserW(CREATE_SUSPENDED) → assign-to-Job → ResumeThread).
//!
//! This is the runtime exercise that review finding R4 said was missing: the
//! existing `windows-check` CI job only `cargo check`ed this code, so a handle
//! leak, a mis-applied token, or a Job Object that doesn't actually cap would
//! all pass green. These tests force the unsafe path to actually run.
//!
//! Runs ONLY on a Windows runner (`windows-latest`); the kernel sandbox path is
//! `#[cfg(target_os = "windows")]`, so this file compiles to nothing elsewhere.
#![cfg(target_os = "windows")]

use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::security::manifest::ResolvedCaps;
use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec};

fn no_caps() -> ResolvedCaps {
    ResolvedCaps {
        cpu_max_pct: None,
        memory_max_mb: None,
        fd_max: None,
    }
}

fn spec(tier: SandboxTier, resolved_caps: ResolvedCaps, id: &str) -> SandboxSpec {
    SandboxSpec {
        tier,
        resolved_caps,
        declared_scopes: vec![],
        spirit_id: id.into(),
        output_shape_predicate: None,
    }
}

/// Positive control: the restricted-token + Job-Object spawn path runs a benign
/// child to completion. Proves the unsafe FFI WORKS end-to-end (not merely
/// compiles) — the exact gap R4 flagged.
#[test]
fn windows_benign_child_exits_zero() {
    let s = spec(SandboxTier::T2, no_caps(), "win-benign");
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "exit 0"]);
    let mut child = spawn_sandboxed(&s, &mut cmd)
        .expect("spawn_sandboxed under restricted token + Job Object must succeed");
    let status = child.wait().expect("wait on sandboxed child");
    assert!(
        status.success(),
        "benign child must exit 0 under T2 sandbox"
    );
}

/// The sandbox must preserve the child's real exit code through the
/// suspended-start → assign-to-job → resume sequence.
#[test]
fn windows_exit_code_preserved() {
    let s = spec(SandboxTier::T2, no_caps(), "win-exit-code");
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "exit 42"]);
    let mut child = spawn_sandboxed(&s, &mut cmd).expect("spawn");
    let status = child.wait().expect("wait");
    assert_eq!(
        status.code(),
        Some(42),
        "exit code must survive the restricted-token + job sandbox"
    );
}

/// Resource-cap smoke + H1 falsifier: a memory-capped spawn must not error and
/// must run a child that stays under budget. If the Windows memory cap is wired
/// as JOB_OBJECT_LIMIT_WORKINGSET with a zero minimum (review finding H1),
/// SetInformationJobObject returns ERROR_INVALID_PARAMETER and `spawn_sandboxed`
/// returns Err here — turning this test RED and surfacing H1.
///
/// NOTE: this proves the capped-spawn PATH does not error. A true
/// memory-enforcement negative control (spawn a hog, assert it is killed on
/// exceed) is deferred to the 10.5 AC3 rework alongside the H1 fix.
#[test]
fn windows_memory_capped_child_runs() {
    let caps = ResolvedCaps {
        cpu_max_pct: None,
        memory_max_mb: Some(64),
        fd_max: None,
    };
    let s = spec(SandboxTier::T2, caps, "win-mem-cap");
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "exit 0"]);
    let mut child = spawn_sandboxed(&s, &mut cmd)
        .expect("memory-capped spawn must succeed (H1 falsifier: working-set min=0 would error)");
    let status = child.wait().expect("wait");
    assert!(
        status.success(),
        "memory-capped child under budget must exit cleanly"
    );
}
