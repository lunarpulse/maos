//! Story 11.4b AC3 — producer-wired proven-red.
//!
//! Proves the FULL pipe end-to-end on a REAL seccomp-enforced child (no mock):
//!
//! ```text
//! real seccomp kill (forbidden ptrace syscall)
//!   → SandboxedChild::wait (the real launcher reap)
//!   → classify_exit → Some(SandboxViolation)
//!   → emit_sandbox_block (edge-wired, public kernel-core API)
//!   → real CapAuditWriter → on-disk TL (kind=8, FrameOrigin::Kernel)
//!   → maos_escape_detector::Detector reads the REAL row read-only
//!   → operator-observable anomaly signal
//! ```
//!
//! This is the gap ADR-024 §4 identifies: the producer was unwired (zero
//! production callers); the seam closes it at the composition-root edge, calling
//! only PUBLIC kernel-core API → ZERO kernel-core delta. The CLI-wrapper
//! (`spawn_and_bridge`) is a plain UNSANDBOXED `Command` and is named OUT OF
//! SCOPE in ink (a separate hardening story); the proven-red runs on the real
//! `spawn_sandboxed` T2 path (the 11.1a WASM seam substrate).

#![cfg(target_os = "linux")]

mod common;
use common::*;
use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;
use maos_escape_detector::{format_anomaly_line, Detector, ManifestDeclaration};
use maos_kernel_core::capability::cap_audit::channel;

/// AC3: a REAL seccomp kill produces a REAL `SandboxBlock` TL row through the
/// producer-wired seam, and the detector reads that real row and raises an
/// operator-observable anomaly. NOT the smoke `_probe`, NOT a stderr print, NOT
/// an in-test `classify_exit` assertion — the full pipe.
#[test]
fn real_seccomp_kill_produces_real_tl_row_and_detector_anomaly() {
    let (_dir, db_path, tl) = fresh_temp_tl();
    let (tx, rx) = channel();
    let security = emit_only_security();
    let spirit_pid = 4242u32;

    // Real seccomp kill through the edge-wired seam.
    let spec = t2_spec("test-spirit-producer-wired-e2e");
    let mut cmd = Command::new(probe_binary_path());
    let violation = match skip_if_sandbox_unavailable(
        reap_and_emit_violation(&spec, &mut cmd, &tx, &security, spirit_pid),
        "producer_wired_e2e",
    ) {
        Some(v) => v,
        None => return, // sandbox/seccomp refused on this host — not a vacuous pass
    };
    // The real SIGSYS kill classified as a SandboxViolation — it is the input
    // that drove the emit above (not an in-vacuum assertion).
    let violation =
        violation.expect("a real forbidden-syscall kill MUST classify as Some(SandboxViolation)");
    assert_eq!(
        violation.sandbox_tier,
        SandboxTier::T2,
        "the T2 seccomp kill is attributed to T2"
    );

    // Drain the real cap-audit writer to the on-disk TL.
    drop(tx);
    flush_audit_channel(rx, tl);

    // The detector reads the REAL TL row read-only and raises exactly one
    // anomaly for this unanticipated kill.
    let manifests = vec![ManifestDeclaration {
        spirit_pid: spirit_pid as i64,
        declared_tier: 2,
        anticipated_kill: false,
    }];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect over TL");
    assert_eq!(
        anomalies.len(),
        1,
        "exactly one anomaly for one real unanticipated seccomp kill"
    );
    let anomaly = &anomalies[0];
    assert_eq!(anomaly.spirit_pid, spirit_pid as i64);
    assert_eq!(anomaly.observed_tier, 2);
    assert!(anomaly.rationale.contains("not anticipated"));
    // D8: the anomaly traces to a real, non-empty frame_id.
    assert!(
        !anomaly.frame_id_hex.is_empty(),
        "the anomaly traces to a real TL frame_id"
    );
    // Sally's observability constraint: the signal is an operator-observable line.
    let line = format_anomaly_line(anomaly);
    assert!(line.starts_with("escape-anomaly:"), "operator-observable line: {line}");

    // Derive-and-reconcile (§A7.1): the count is derived from the real TL, not a
    // committed literal. Re-reading the TL yields the same single row.
    let reread =
        Detector::read_kernel_sandbox_blocks(&db_path).expect("re-read kernel sandbox blocks");
    assert_eq!(reread.len(), 1, "idempotent re-read of the real TL");
    assert_eq!(reread[0].origin, maos_escape_detector::FRAME_ORIGIN_KERNEL);

    // Measurement marker — the gate's producer-wired-proven-red leg requires
    // this string to consider the leg GREEN. A silent seccomp-unavailable skip
    // (the `return` above) emits NO marker, so the leg cannot pass vacuously.
    // Only reached after a REAL seccomp kill produced a REAL TL row + anomaly.
    eprintln!(
        "ESCAPE-PRODUCER-WIRED-MEASURED frame={}",
        anomaly.frame_id_hex
    );
}

/// AC3 falsifier (§A7.3 — the flag severs the wiring). Under `escape-fault-inject`
/// the seam SKIPS `emit_sandbox_block`, so a real seccomp kill produces NO TL row
/// → the green assertion above (a real row appears) would RED. This test asserts
/// the severed behavior (no row), proving the row in the green test came from the
/// real wiring, not a canned fixture. Contrast = the green test's row is real.
#[cfg(feature = "escape-fault-inject")]
#[ignore = "requires --features escape-fault-inject; gate-controlled via check-escape-detector"]
#[test]
fn fault_inject_severs_producer_wiring_to_no_row() {
    let (_dir, db_path, tl) = fresh_temp_tl();
    let (tx, rx) = channel();
    let security = emit_only_security();
    let spirit_pid = 9090u32;

    let spec = t2_spec("test-spirit-producer-fault-inject");
    let mut cmd = Command::new(probe_binary_path());
    let reaped = match skip_if_sandbox_unavailable(
        reap_and_emit_violation(&spec, &mut cmd, &tx, &security, spirit_pid),
        "producer_fault_inject",
    ) {
        Some(v) => v, // v: Option<SandboxViolation> — the classified real kill
        None => return, // sandbox unavailable — advisory skip, emits NO marker
    };
    drop(tx);
    flush_audit_channel(rx, tl);

    // Under the falsifier the emit was severed → NO kind=8 row was produced.
    let rows = Detector::read_kernel_sandbox_blocks(&db_path).expect("read TL");
    assert!(
        rows.is_empty(),
        "escape-fault-inject severed the emit — no SandboxBlock row should exist (got {})",
        rows.len()
    );

    // Marker ONLY when a REAL seccomp kill was reaped-and-severed — the genuine
    // §A7.3 contrast vs. the green producer test (real kill → real row). A
    // sandbox-unavailable skip returns above and emits no marker, so the gate's
    // producer-wired leg cannot pass vacuously on a seccomp-blocked host.
    if reaped.is_some() {
        eprintln!("ESCAPE-PRODUCER-FALSIFIER-MEASURED severed_no_row=true");
    }
}
