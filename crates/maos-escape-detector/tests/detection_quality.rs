//! Story 11.4b AC4 — detection quality on a LIVE-syscall proven-red (no mock).
//!
//! TP-floor + FP-ceiling measured on the DETECTOR's **correlation decision**,
//! NOT on the deterministic `classify_exit` map (Murat: a TP-rate over
//! `classify_exit` is vacuous — every SIGSYS→T2 identically). The corpus reuses
//! the 11.1a real-seccomp substrate (`spawn_sandboxed` + the forbidden-syscall
//! probe):
//!
//! - **TP set**: real seccomp kills whose manifest did NOT declare the tripping
//!   scope (a true escape signature) → the detector MUST raise an anomaly.
//! - **FP set**: benign completions (no kill, no row) **+** real seccomp kills
//!   the manifest DID anticipate → the detector MUST NOT raise an anomaly.
//!
//! The SAME real seccomp kill correlated against an anticipated vs. unanticipated
//! manifest yields a different outcome — that contrast is what makes the metric
//! non-vacuous. Counts are DERIVED per-run from real detector output over real
//! children (never committed literals); an empty corpus reports N/A, not a pass.

#![cfg(target_os = "linux")]

mod common;
use common::*;
use std::process::Command;

use maos_escape_detector::{Detector, ManifestDeclaration};
use maos_kernel_core::capability::cap_audit::channel;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::security::sandbox::{classify_exit, spawn_sandboxed};

// Measure-then-pin thresholds (the 11.3 / 11.4a idiom). A correct detector on a
// clean real-seccomp corpus hits TP=1.0 / FP=0.0; the bars carry margin so a
// single spurious result does not flip the leg, and the `escape-fault-inject`
// falsifier (below) breaks them decisively.
const TP_FLOOR: f64 = 0.9;
const FP_CEILING: f64 = 0.1;

/// Drive one real seccomp kill for `spirit_pid` through the edge-wired seam,
/// emitting a real `SandboxBlock` TL row. Returns true if a real violation was
/// classified + emitted (the producer half of a TP/anticipated-FP case).
fn emit_real_kill(
    tl: &std::sync::Arc<TransparencyLogAdapter>,
    spirit_pid: u32,
    spirit_id: &str,
) -> bool {
    let (tx, rx) = channel();
    let security = emit_only_security();
    let spec = t2_spec(spirit_id);
    let mut cmd = Command::new(probe_binary_path());
    let violation = match skip_if_sandbox_unavailable(
        reap_and_emit_violation(&spec, &mut cmd, &tx, &security, spirit_pid),
        "detection_quality_emit_real_kill",
    ) {
        Some(v) => v,
        None => return false,
    };
    let emitted = violation.is_some();
    drop(tx);
    flush_audit_channel(rx, tl.clone());
    emitted
}

/// AC4: TP-rate ≥ floor AND FP-rate ≤ ceiling, measured LIVE over real seccomp
/// enforcement on the detector's correlation decision.
#[cfg(not(feature = "escape-fault-inject"))]
#[test]
fn detection_quality_meets_floor_and_ceiling_on_real_seccomp() {
    let (_dir, db_path, tl) = fresh_temp_tl();

    // TP set: two real seccomp kills, manifests UNANTICIPATED.
    let tp_pids = [5001u32, 5002u32];
    let mut tp_kills_observed = 0u32;
    for &pid in &tp_pids {
        if emit_real_kill(&tl, pid, &format!("test-spirit-tp-{pid}")) {
            tp_kills_observed += 1;
        }
    }
    // FP set (anticipated): one real seccomp kill, manifest ANTICIPATED.
    let anticipated_pid = 6001u32;
    let mut anticipated_kills_observed = 0u32;
    if emit_real_kill(&tl, anticipated_pid, "test-spirit-fp-anticipated") {
        anticipated_kills_observed += 1;
    }
    // FP set (benign): a clean T2 completion — no kill, no row.
    let mut benign_observed = false;
    {
        let spec = t2_spec("test-spirit-fp-benign");
        let mut cmd = Command::new("/bin/true");
        if let Some(mut child) = skip_if_sandbox_unavailable(
            spawn_sandboxed(&spec, &mut cmd),
            "detection_quality_benign",
        ) {
            let status = child.wait().expect("wait benign");
            assert!(status.success(), "benign /bin/true must complete under T2");
            assert!(
                classify_exit(status).is_none(),
                "clean exit is not a violation"
            );
            benign_observed = true;
        }
    }

    // DERIVE fp_cases early (§A7.1) + INDEPENDENT corpus guards (P3): the TP set
    // AND the FP set must EACH be non-empty to measure. A degenerate corpus (host
    // refused some/all sandbox spawns) reports N/A — emitting NO marker, so the
    // gate's detection-quality-live leg stays advisory, never a vacuous pass.
    let fp_cases = anticipated_kills_observed as f64 + if benign_observed { 1.0 } else { 0.0 };
    if tp_kills_observed == 0 || fp_cases == 0.0 {
        eprintln!(
            "SKIP detection_quality: degenerate real-seccomp corpus \
             (tp_kills={tp_kills_observed}, fp_cases={fp_cases}) — N/A, not a vacuous pass"
        );
        return;
    }

    let manifests = vec![
        ManifestDeclaration {
            spirit_pid: 5001,
            declared_tier: 2,
            anticipated_kill: false,
        },
        ManifestDeclaration {
            spirit_pid: 5002,
            declared_tier: 2,
            anticipated_kill: false,
        },
        // The anticipated kill: the manifest declared this kill was expected.
        ManifestDeclaration {
            spirit_pid: 6001,
            declared_tier: 2,
            anticipated_kill: true,
        },
    ];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect over real-seccomp TL");

    // DERIVE-AND-RECONCILE (§A7.1): TP/FP counts come from the detector's real
    // correlation output, reconciled against the corpus cardinality.
    let tp_anomalies = anomalies
        .iter()
        .filter(|a| tp_pids.iter().any(|&p| i64::from(p) == a.spirit_pid))
        .count() as f64;
    let tp_rate = tp_anomalies / tp_kills_observed as f64;
    let fp_anomalies = anomalies
        .iter()
        .filter(|a| !tp_pids.iter().any(|&p| i64::from(p) == a.spirit_pid))
        .count() as f64;
    let fp_rate = fp_anomalies / fp_cases;

    assert!(
        tp_rate >= TP_FLOOR,
        "TP-rate {tp_rate:.3} below floor {TP_FLOOR}"
    );
    assert!(
        fp_rate <= FP_CEILING,
        "FP-rate {fp_rate:.3} above ceiling {FP_CEILING}"
    );

    // Measurement marker — the gate's detection-quality leg requires this string
    // to consider the leg GREEN (a silent seccomp-unavailable skip emits none).
    eprintln!(
        "ESCAPE-DETECTOR-QUALITY-MEASURED tp_rate={tp_rate:.3} fp_rate={fp_rate:.3} \
         tp_kills={tp_kills_observed} fp_cases={fp_cases}"
    );
}

/// AC4 falsifier (§A7.3 — the flag stubs the DETECTOR, never the enforcement).
/// Under `escape-fault-inject`, `correlate()` returns a canned anomaly for EVERY
/// real frame regardless of the manifest. So a real anticipated kill (which the
/// green test correctly does NOT anomalize) NOW anomalizes → the FP-ceiling
/// breaks. This test asserts the severed behavior, proving the FP-avoidance in
/// the green test is the REAL correlation, not a constant.
#[cfg(feature = "escape-fault-inject")]
#[ignore = "requires --features escape-fault-inject; gate-controlled via check-escape-detector"]
#[test]
fn fault_inject_stubs_detector_to_canned_tp() {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::FrameKind;
    let (_dir, db_path, tl) = fresh_temp_tl();

    // A REAL Kernel-origin SandboxBlock row (genuine structural fact — the
    // enforcement data is real; only the DETECTOR is stubbed by the feature).
    tl.insert_frame_event(
        FrameKind::SandboxBlock,
        6001,
        None,
        "sandbox.block.unknown",
        b"tier=2",
        FrameOrigin::Kernel,
    );

    // Anticipated manifest → in the green test this is NoAnomaly. Under the
    // canned-TP stub the detector anomalizes it anyway → FP-avoidance broke.
    let manifests = vec![ManifestDeclaration {
        spirit_pid: 6001,
        declared_tier: 2,
        anticipated_kill: true,
    }];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect under fault-inject");
    assert_eq!(
        anomalies.len(),
        1,
        "escape-fault-inject stubbed the detector to canned-TP — an anticipated \
         kill anomalizes (FP-avoidance broke, proving the green metric is real)"
    );
    assert!(anomalies[0].rationale.contains("escape-fault-inject"));
}

/// AC4 (correlation decision, no seccomp required): the detector's TP/FP metric
/// on the CORRELATION (declared-vs-actual manifest), validated with real
/// Kernel-origin SandboxBlock rows inserted directly. This runs GREEN on every
/// host — the live-seccomp substrate (the test above) is the additional "no
/// mock" enforcement tripwire on seccomp-capable runners. Together they make the
/// metric non-vacuous: the correlation logic is proven here, the real-enforcement
/// signal on capable hosts.
#[cfg(not(feature = "escape-fault-inject"))]
#[test]
fn correlation_quality_on_structural_rows() {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::FrameKind;
    let (_dir, db_path, tl) = fresh_temp_tl();

    // TP rows: real Kernel-origin SandboxBlock rows for Spirits whose manifest
    // did NOT anticipate the kill → the detector MUST raise anomalies.
    let tp_pids = [8001u32, 8002u32, 8003u32];
    for &pid in &tp_pids {
        tl.insert_frame_event_with_id(
            Some([pid as u8; 16]),
            FrameKind::SandboxBlock,
            pid,
            "",
            "",
            None,
            "sandbox.block.unknown",
            b"tier=2",
            FrameOrigin::Kernel,
        );
    }
    // FP row (anticipated): a real Kernel-origin kill the manifest DID
    // anticipate → the detector MUST NOT raise an anomaly.
    let anticipated_pid = 8004u32;
    tl.insert_frame_event_with_id(
        Some([0x84; 16]),
        FrameKind::SandboxBlock,
        anticipated_pid,
        "",
        "",
        None,
        "sandbox.block.unknown",
        b"tier=2",
        FrameOrigin::Kernel,
    );
    // A non-kernel-origin row that MUST be excluded (synthesized source).
    tl.insert_frame_event_with_id(
        Some([0x99; 16]),
        FrameKind::SandboxBlock,
        8005,
        "",
        "",
        None,
        "sandbox.block.unknown",
        b"tier=2",
        FrameOrigin::HumanAuthored,
    );

    let manifests = vec![
        ManifestDeclaration {
            spirit_pid: 8001,
            declared_tier: 2,
            anticipated_kill: false,
        },
        ManifestDeclaration {
            spirit_pid: 8002,
            declared_tier: 2,
            anticipated_kill: false,
        },
        ManifestDeclaration {
            spirit_pid: 8003,
            declared_tier: 2,
            anticipated_kill: false,
        },
        ManifestDeclaration {
            spirit_pid: 8004,
            declared_tier: 2,
            anticipated_kill: true,
        },
        ManifestDeclaration {
            spirit_pid: 8005,
            declared_tier: 2,
            anticipated_kill: false,
        },
    ];
    let anomalies = Detector::detect(&db_path, &manifests).expect("detect over structural rows");

    // DERIVE-AND-RECONCILE (§A7.1): counts come from the detector's real
    // correlation output. The FP denominator is the set of Kernel-origin rows the
    // correlation actually EVALUATED (non-TP) — NOT the source-identity-filtered
    // non-kernel row (8005), which detect() excludes before correlation.
    let is_tp = |pid: i64| tp_pids.iter().any(|&p| i64::from(p) == pid);
    let frames = Detector::read_kernel_sandbox_blocks(&db_path).expect("read frames");
    let tp_anomalies = anomalies.iter().filter(|a| is_tp(a.spirit_pid)).count() as f64;
    let tp_rate = tp_anomalies / tp_pids.len() as f64;
    let fp_cases = frames
        .iter()
        .filter(|f| f.origin == maos_escape_detector::FRAME_ORIGIN_KERNEL && !is_tp(f.spirit_pid))
        .count() as f64;
    let fp_anomalies = anomalies.iter().filter(|a| !is_tp(a.spirit_pid)).count() as f64;
    let fp_rate = fp_anomalies / fp_cases;

    assert!(
        tp_rate >= TP_FLOOR,
        "TP-rate {tp_rate:.3} below floor {TP_FLOOR}"
    );
    assert!(
        fp_rate <= FP_CEILING,
        "FP-rate {fp_rate:.3} above ceiling {FP_CEILING}"
    );

    // Measurement marker (AFTER the asserts — only a genuine pass emits it). The
    // gate's detection-quality leg requires this string to consider the leg GREEN.
    eprintln!(
        "ESCAPE-DETECTOR-QUALITY-MEASURED tp_rate={tp_rate:.3} fp_rate={fp_rate:.3} \
         tp_kills={} fp_cases={fp_cases}",
        tp_pids.len()
    );
}
