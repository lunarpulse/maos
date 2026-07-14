//! Story 11.4b (AC1, Task 3) — composition-root wiring of the out-of-kernel
//! sandbox-escape detector as a standalone **read-only** Transparency-Log
//! consumer.
//!
//! This is **NOT** a `maos-kernel-core` adapter and is **NOT** injected into the
//! kernel via a port (L9/L10) — it deliberately stays **out of `api.rs`** so
//! `check-composition-root-completeness` stays GREEN. It reads the TL path
//! read-only, runs `maos_escape_detector::Detector::detect`, and writes each
//! anomaly to the operator-observable surface (stderr — Sally's observability
//! constraint). It **decides nothing** about capability grants (it is not
//! authorization — L10); it only reports structural anomalies.
//!
//! Production posture (the honesty clause): production Spirits today launch via
//! the unsandboxed `spawn_and_bridge` path (CATCH-0 — wiring production
//! sandboxing is a separate hardening story, out of scope). So in production
//! today this consumer finds no `SandboxBlock` rows — it is wired and dormant,
//! ready to surface anomalies the moment a real T2 sandbox kill flows through
//! the edge-wired producer seam. It is exercised here against a synthetic TL.

use std::path::Path;

use maos_escape_detector::{format_anomaly_line, ManifestDeclaration};
/// Run the escape detector over the Transparency Log at `tl_path` read-only and
/// emit each anomaly as an operator-observable line. Returns the anomaly count
/// (derive-and-reconcile: the count comes from the real detector output, never a
/// committed literal).
#[allow(dead_code)] // wired + dormant until production T2 sandboxing lands (CATCH-0, out of scope)
pub fn report_escape_anomalies(
    tl_path: &Path,
    manifests: &[ManifestDeclaration],
) -> Result<usize, maos_escape_detector::DetectorError> {
    // A read-only TL scan failure is NOT "all clear" — propagate it so the caller
    // can distinguish a broken scan from a genuinely clean log. A security-
    // observability surface must never map failure to the benign value (0).
    let anomalies = maos_escape_detector::Detector::detect(tl_path, manifests)?;
    for report in &anomalies {
        eprintln!("{}", format_anomaly_line(report));
    }
    Ok(anomalies.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

    /// The consumer surfaces exactly one anomaly for one real kernel-origin
    /// SandboxBlock row whose manifest did not anticipate the kill, and zero for
    /// an anticipated one — the correlation decision, wired at the composition
    /// root (out of `api.rs`).
    #[test]
    fn consumer_reports_anomaly_for_unanticipated_kill_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("escape_consumer_tl.db");
        let tl = TransparencyLogAdapter::open(&db, 1).expect("open TL");

        // One unanticipated kill (anomaly) + one anticipated kill (no anomaly).
        tl.insert_frame_event(
            FrameKind::SandboxBlock,
            4242,
            None,
            "sandbox.block.unknown",
            b"tier=2",
            FrameOrigin::Kernel,
        );
        tl.insert_frame_event(
            FrameKind::SandboxBlock,
            4243,
            None,
            "sandbox.block.unknown",
            b"tier=2",
            FrameOrigin::Kernel,
        );

        let manifests = vec![
            ManifestDeclaration {
                spirit_pid: 4242,
                declared_tier: 2,
                anticipated_kill: false,
            },
            ManifestDeclaration {
                spirit_pid: 4243,
                declared_tier: 2,
                anticipated_kill: true,
            },
        ];
        let count = report_escape_anomalies(&db, &manifests).expect("read-only TL scan ok");
        assert_eq!(
            count, 1,
            "the consumer surfaces exactly one anomaly (the unanticipated kill); \
             the anticipated kill is correctly NOT an anomaly"
        );
    }
}
