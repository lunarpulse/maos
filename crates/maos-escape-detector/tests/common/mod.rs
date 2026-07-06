//! Shared helpers for the Story 11.4b live-syscall integration tests.
//!
//! These tests drive the REAL kernel producer (`spawn_sandboxed` →
//! `SandboxedChild::wait` → `classify_exit` → `emit_sandbox_block` → the live
//! `CapAuditWriter` → the on-disk Transparency Log) over a REAL seccomp-enforced
//! child (the `forbidden-syscall-probe` binary), then read the produced TL row
//! back through the detector. `maos-kernel-core` is a DEV-dependency only — the
//! library graph stays kernel-core-free (the maos-audit template; L3).
//!
//! `reap_and_emit_violation` below is the **edge-wired producer seam** (Task 1):
//! it lives at the composition-root edge and calls only PUBLIC kernel-core API,
//! so it draws ZERO kernel-core lines. It is replicated here (the detector crate
//! cannot depend on `maos-bin`, a binary) — the canonical production seam lives
//! in `maos-bin`; this is its test-twin over the identical public calls.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_audit::{CapAuditEvent, CapAuditWriter};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::security::sandbox::{
    classify_exit, spawn_sandboxed, SandboxSpec, SandboxViolation, SpawnError,
};
use maos_kernel_core::security::SecurityManagerAdapter;

/// A no-op `SecurityManagerAdapter` the seam emits through. `emit_sandbox_block`
/// only needs the cap-audit `Sender` (it builds the `CapAuditEvent` and
/// `try_send`s it); the policy table is unused on this path.
pub fn emit_only_security() -> SecurityManagerAdapter {
    use std::sync::Arc;
    SecurityManagerAdapter::new(Arc::new(
        maos_kernel_core::capability::cap_policy::PolicyTable::new(),
    ))
}

/// The edge-wired producer seam (Task 1 / AC3). Spawn a real sandboxed child,
/// reap it on the launcher reap (`SandboxedChild::wait`), classify the exit
/// (`classify_exit`), and on a real violation emit a `SandboxBlock` audit event
/// (`emit_sandbox_block`) — the existing kernel-core mechanism called from the
/// edge. Returns the classified violation (if any) for the caller to assert on.
///
/// Under `escape-fault-inject` the emit is SEVERED (the `emit_sandbox_block` call
/// is skipped) so the producer-wired proven-red goes RED — proving the TL row is
/// produced by the real wiring, not a canned fixture (AC3 falsifier, §A7.3).
pub fn reap_and_emit_violation(
    spec: &SandboxSpec,
    command: &mut Command,
    sender: &maos_kernel_core::capability::cap_audit::Sender,
    security: &SecurityManagerAdapter,
    spirit_pid: u32,
) -> Result<Option<SandboxViolation>, SpawnError> {
    let mut child = spawn_sandboxed(spec, command)?;
    let status = child.wait().map_err(SpawnError::Io)?;
    let violation = classify_exit(status);
    // AC3 falsifier — `escape-fault-inject` severs the wiring: the emit is skipped,
    // so no `SandboxBlock` TL row is produced → the producer-wired proven-red REDs
    // (the row came from the real wiring, not a canned fixture, §A7.3). Gating the
    // whole `if let` keeps the binding used under both cfgs (no unused-var warning).
    #[cfg(not(feature = "escape-fault-inject"))]
    if let Some(v) = &violation {
        security.emit_sandbox_block(sender, spirit_pid, &v.attempted_syscall, v.sandbox_tier);
    }
    Ok(violation)
}

/// Drain a cap-audit channel to completion against a Transparency Log on a
/// dedicated tokio runtime (the writer task is async). The caller `try_send`s
/// its events and `drop`s the sender BEFORE calling this; the writer drains the
/// buffered events and exits when the channel closes.
pub fn flush_audit_channel(
    rx: tokio::sync::mpsc::Receiver<CapAuditEvent>,
    tl: std::sync::Arc<TransparencyLogAdapter>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime for cap-audit writer");
    rt.block_on(async move {
        let writer = CapAuditWriter::spawn(rx, tl);
        let _ = writer.await;
    });
}

/// A minimal T2 `SandboxSpec` with no declared scopes + no resource caps (the
/// seccomp filter is what trips SIGSYS on the probe; cgroups/landlock are
/// defense-in-depth). `spirit_id` is informational.
pub fn t2_spec(spirit_id: &str) -> SandboxSpec {
    SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![],
        spirit_id: spirit_id.to_string(),
        output_shape_predicate: None,
    }
}

/// Path to the `forbidden-syscall-probe` binary, building it on first use (the
/// fixture is its own `[workspace]`, so a plain `cargo build --release` in its
/// dir compiles it without touching the main workspace). Cached per test binary
/// via `LazyLock` so parallel tests within one binary don't race the build.
pub fn probe_binary_path() -> PathBuf {
    static PROBE: LazyLock<PathBuf> = LazyLock::new(build_probe);
    PROBE.clone()
}

fn build_probe() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_dir = format!("{manifest_dir}/test-fixtures/forbidden-syscall-probe");
    let binary = format!("{fixture_dir}/target/release/forbidden-syscall-probe");
    if !std::path::Path::new(&binary).exists() {
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&fixture_dir)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => panic!(
                "failed to build forbidden-syscall-probe in {fixture_dir} — \
                 run `cargo build --release` there manually"
            ),
        }
    }
    PathBuf::from(binary)
}

/// Open a fresh on-disk Transparency Log in a temp dir (boot_nonce=1).
pub fn fresh_temp_tl() -> (tempfile::TempDir, PathBuf, std::sync::Arc<TransparencyLogAdapter>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("escape_detector_tl.db");
    let tl = std::sync::Arc::new(
        TransparencyLogAdapter::open(&db_path, 1).expect("open on-disk TL"),
    );
    (dir, db_path, tl)
}

/// Skip gracefully only when the host GENUINELY cannot sandbox — seccomp refused
/// (`EPERM`) or absent (`ENOSYS` → `Unsupported`), or an unsupported platform
/// (`SandboxUnavailable`). Returns `None` to skip; the caller then emits NO
/// measurement marker, so the gate's live legs stay advisory (not vacuous-green).
/// A `SandboxSetup` failure (which can signal a REAL seccomp filter-build
/// regression) and every other error hard-fail (panic). Broadening the skip
/// cannot mask a capable-host enforcement regression: that surfaces as a MISSING
/// kill (the child exits 0 → no violation → no marker → leg RED), never as a
/// spawn refusal, so it never reaches this helper.
pub fn skip_if_sandbox_unavailable<T>(result: Result<T, SpawnError>, test_name: &str) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(SpawnError::SandboxUnavailable { .. }) => {
            eprintln!("SKIP {test_name}: sandbox unavailable on this host");
            None
        }
        Err(SpawnError::Io(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::Unsupported
            ) =>
        {
            eprintln!("SKIP {test_name}: sandbox spawn refused by host ({:?})", e.kind());
            None
        }
        Err(e) => panic!(
            "{test_name}: spawn_sandboxed failed (not an environment-unavailable signal): {e:?}"
        ),
    }
}
