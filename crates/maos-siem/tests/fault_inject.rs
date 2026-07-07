//! Story 11.4c — `siem-fault-inject` redaction-BYPASS gate.
//!
//! Runs ONLY via
//! `cargo test -p maos-siem --features siem-fault-inject -- --ignored`.
//! The whole file is `#![cfg(feature = "siem-fault-inject")]` and the test is
//! `#[ignore]`d, so it is absent from the default suite; the `compile_error!`
//! guard in `lib.rs` blocks `--features siem-fault-inject` in release builds.
//!
//! # Why this is the achievable inversion
//!
//! The `siem-fault-inject` feature makes `export_from_tl` BYPASS
//! `query_with_redaction` and use plain `query(...)` instead — the real
//! behavioral branch this test exercises.
//!
//! MAOS scrubs secret-class payloads at INSERT time (the TL stores only
//! `payload_redacted`; there is no raw column), AND both `query` and
//! `query_with_redaction` read that SAME `payload_redacted` column. So a raw
//! secret can never reach the projection in EITHER path — the bypass's
//! observable effect is NOT raw-secret leakage but the ABSENCE of the
//! `redaction` provenance metadata that `query_with_redaction` is the sole
//! sanctioned populator of. This test asserts that provenance is DROPPED under
//! the bypass — the faithful inversion of the redaction guarantee that the
//! feature actually produces (and the exact signature `redaction_before_forward`
//! pins as PRESENT on the sanctioned path).

#![cfg(feature = "siem-fault-inject")]
#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_audit::AuditFilter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::export_from_tl;
use tempfile::TempDir;

/// A secret-class fragment the kernel redaction policy scrubs at INSERT time
/// (canonical `aws_access_key` prefix match). It is replaced with
/// `<REDACTED:type=aws_access_key,...>` before the row is persisted, so a raw
/// secret never reaches EITHER read path — see the module doc.
const SECRET_FRAGMENT: &str = "AKIAIOSFODNN7EXAMPLE";

/// Seed an isolated TL with one secret-class frame, then drop the writer so the
/// read-only export path sees a quiesced / checkpointed DB.
fn seed_tl_with_secret() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 1).unwrap());

    let payload = format!("invocation notes key={SECRET_FRAGMENT} endpoint=prod");

    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1,
        None,
        "test.capability.invoke",
        payload.as_bytes(),
        FrameOrigin::Kernel,
    );

    drop(tl); // quiesce / checkpoint the writer for deterministic export
    (dir, db_path)
}

#[test]
#[ignore]
fn fault_inject_bypass_drops_redaction_provenance_so_a_leak_can_invert() {
    let (_dir, db_path) = seed_tl_with_secret();

    // export_from_tl under siem-fault-inject routes through plain query().
    let records = export_from_tl(&db_path, AuditFilter::default())
        .expect("export_from_tl must succeed for a real, quiesced TL");

    assert!(
        !records.is_empty(),
        "the seeded TL row must be exported — an empty result would be a vacuous pass"
    );

    let ndjson = records
        .iter()
        .map(|r| r.ndjson.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // BYPASS signature: plain query() leaves AuditEntry.redaction = None, and
    // the serde model skips the key entirely, so the forwarded NDJSON carries
    // NO `redaction` provenance. The sanctioned path (query_with_redaction)
    // populates it — see `redaction_before_forward.rs`.
    assert!(
        !ndjson.contains("\"redaction\""),
        "under the siem-fault-inject BYPASS the redaction provenance MUST be \
         absent (this is the inversion the feature produces): {ndjson}"
    );

    // Sanity: the bypass still forwards the (insert-time-scrubbed) payload — the
    // row is real, not a no-op. The raw secret fragment never appears because
    // it was never persisted; the redaction marker is what survives.
    assert!(
        ndjson.contains("REDACTED"),
        "the bypassed export must still carry the insert-time redaction marker \
         (proving the row was read, not dropped): {ndjson}"
    );
}
