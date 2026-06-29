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
//! Two kinds of test live here:
//!   - POSITIVE controls — the sandbox spawns/exits/preserves-exit-code.
//!   - ENFORCEMENT negative controls (Story 10.5 §A2 re-review R4) — prove the
//!     restrictions actually BITE, by exit code alone (the sandbox spawn does
//!     not wire child stdio, so stdout is unavailable; we make each child's
//!     PASS/FAIL observable purely through its exit status):
//!       * memory: a child that commits > `memory_max_mb` is stopped by the
//!         `JOB_OBJECT_LIMIT_PROCESS_MEMORY` cap (non-zero exit), while the same
//!         child under a generous cap succeeds — so a cap that did not cap would
//!         flip the first assertion red. This is the falsifier R4 named ("a Job
//!         cap that doesn't cap … would pass green").
//!       * integrity: a child reports its own token's mandatory-integrity SID
//!         (`whoami /groups`); we assert the Low SID `S-1-16-4096` is present —
//!         so an integrity label that did not apply (token stays Medium,
//!         `S-1-16-8192`) would flip it red.
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

fn caps_mem(mb: u32) -> ResolvedCaps {
    ResolvedCaps {
        cpu_max_pct: None,
        memory_max_mb: Some(mb),
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

/// A PowerShell child that commits a ~1 GiB byte array (touching one byte per
/// MiB to force the pages in). Under a Job Object whose per-process commit cap
/// is below that, the allocation cannot commit and PowerShell exits non-zero
/// (terminating OutOfMemoryException). Under a generous cap it commits and
/// `exit 0`. Exit code is the only observable signal (stdio is not wired).
fn memory_hog_command() -> Command {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        // 1073741824 = 1 GiB; touch every 1 MiB (1048576) to commit pages.
        "$ErrorActionPreference='Stop'; $b=[byte[]]::new(1073741824); \
         for($i=0;$i -lt $b.Length;$i+=1048576){$b[$i]=1}; exit 0",
    ]);
    cmd
}

// ── Positive controls ────────────────────────────────────────────────

/// The restricted-token + Job-Object spawn path runs a benign child to
/// completion. Proves the unsafe FFI WORKS end-to-end (not merely compiles).
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

/// H1 falsifier: a memory-capped spawn of a benign child must not error and
/// must run under budget. If the Windows memory cap were wired as
/// `JOB_OBJECT_LIMIT_WORKINGSET` with a zero minimum (the old H1 bug),
/// `SetInformationJobObject` returns `ERROR_INVALID_PARAMETER` and
/// `spawn_sandboxed` returns Err here — turning this test RED.
#[test]
fn windows_memory_capped_child_runs() {
    let s = spec(SandboxTier::T2, caps_mem(64), "win-mem-cap");
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

// ── R4 enforcement negative controls ─────────────────────────────────

/// R4 (a) — the memory cap must actually BITE. A child that tries to commit
/// ~1 GiB under a 256 MiB per-process commit cap cannot allocate and exits
/// non-zero. If the Job Object's `JOB_OBJECT_LIMIT_PROCESS_MEMORY` did not
/// enforce (the exact R4 blind spot), the allocation would succeed and the
/// child would `exit 0` — flipping this assertion red.
#[test]
fn windows_memory_cap_kills_overbudget_child() {
    let s = spec(SandboxTier::T2, caps_mem(256), "win-mem-overbudget");
    let mut cmd = memory_hog_command();
    let mut child = spawn_sandboxed(&s, &mut cmd)
        .expect("over-budget spawn must start (the cap bites at commit, not at spawn)");
    let status = child.wait().expect("wait");
    assert!(
        !status.success(),
        "a child committing > memory_max_mb must be stopped by the Job Object commit cap \
         (non-zero exit); exit 0 here means the cap did not enforce (R4)"
    );
}

/// R4 (a) control — the SAME ~1 GiB hog under a generous 4 GiB cap commits and
/// `exit 0`. This proves the non-zero exit above is attributable to the cap,
/// not to PowerShell or the allocation itself failing for an unrelated reason.
#[test]
fn windows_memory_cap_allows_underbudget_child() {
    let s = spec(SandboxTier::T2, caps_mem(4096), "win-mem-underbudget");
    let mut cmd = memory_hog_command();
    let mut child = spawn_sandboxed(&s, &mut cmd).expect("under-budget spawn must start");
    let status = child.wait().expect("wait");
    assert!(
        status.success(),
        "the ~1 GiB hog must commit and exit 0 under a 4 GiB cap (negative-control baseline)"
    );
}

/// R4 (b) — the low-integrity mandatory label must actually APPLY. The child
/// asks its own token for its integrity SID via `whoami /groups`; we match the
/// Low Mandatory Level SID `S-1-16-4096`. `findstr` (last in the pipe) sets the
/// exit code: 0 iff the Low SID is present. If the integrity label did not
/// apply, the token would stay Medium (`S-1-16-8192`), `findstr` would not
/// match, and the child would exit non-zero — flipping this red.
#[test]
fn windows_child_runs_at_low_integrity() {
    let s = spec(SandboxTier::T2, no_caps(), "win-low-integrity");
    let mut cmd = Command::new("cmd");
    // No inner quotes / metachars beyond the pipe — the whole pipeline is one
    // `/C` argument (quoted by the sandbox's CRT-style arg quoting). `findstr`
    // matching a SID literal needs no quoting.
    cmd.args(["/C", "whoami /groups | findstr S-1-16-4096"]);
    let mut child = spawn_sandboxed(&s, &mut cmd).expect("spawn low-integrity probe");
    let status = child.wait().expect("wait");
    assert!(
        status.success(),
        "sandboxed child must run at Low integrity (S-1-16-4096 present in its token groups); \
         non-zero exit means the low-integrity label did not apply (R4)"
    );
}
