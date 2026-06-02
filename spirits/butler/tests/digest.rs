//! AC5 — morning digest (FR17, Spirit-side) composed from the Story 3.4
//! `ranged_recall` log-composition primitive and persisted through the Story
//! 4.4 distillation port (I11 audit chain).
//!
//! ONE file-backed Transparency Log backs both the read path (`ranged_recall` +
//! `maos_audit::query` over the db file) and the write path (`DistillateWriter`
//! resolving `source_log_ref` against the same adapter), so the cited frame ids
//! are exactly the ids the kernel resolves.

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use butler::Butler;

use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::notification::NotificationEvent;
use maos_domain::ports::DistillationPort;

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0xB17_D16E5;

fn now_ns() -> u64 {
    // 1ms ahead so the just-inserted (wall-clock-stamped) frames fall strictly
    // inside the half-open last-24h window [now-24h, now).
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        + 1_000_000
}

/// Seed a file-backed TL with two completions, one open halt, and a noise
/// task.assign (which the digest must ignore). Returns (Arc<TL>, completion
/// ids, halt id).
fn seed_tl(db: &std::path::Path) -> (Arc<TransparencyLogAdapter>, [u8; 16], [u8; 16], [u8; 16]) {
    let tl = Arc::new(TransparencyLogAdapter::open(db, NONCE).expect("open file-backed TL"));

    let _ = tl.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "ship release notes",
        b"done",
        FrameOrigin::SpiritAuto,
    );
    let c1 = tl.last_frame_id();

    let _ = tl.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "review PR 42",
        b"done",
        FrameOrigin::SpiritAuto,
    );
    let c2 = tl.last_frame_id();

    let _ = tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        1,
        None,
        "ambiguous calendar conflict awaiting director",
        b"halt",
        FrameOrigin::Kernel,
    );
    let h1 = tl.last_frame_id();

    // Noise: a task.assign that is NEITHER a completion NOR a halt.
    let _ = tl.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        None,
        "noise assignment",
        b"x",
        FrameOrigin::HumanAuthored,
    );

    (tl, c1, c2, h1)
}

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn ac5_morning_digest_composes_cites_and_persists_within_budget() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("transparency.sqlite");
    let journal = tmp.path().join("journal.ndjson");
    let (tl, c1, c2, h1) = seed_tl(&db);

    let now = now_ns();
    std::fs::write(
        &journal,
        format!("{{\"timestamp\":{},\"lifecycle_event\":\"Pause\",\"spirit_id\":\"butler\"}}\n", now - 5_000),
    )
    .unwrap();

    let anomalies = vec![
        NotificationEvent::anomaly_flagged("observer", "butler", "unusual after-hours access", 0.82)
            .unwrap(),
        NotificationEvent::anomaly_flagged("observer", "butler", "low-confidence blip", 0.40).unwrap(),
    ];

    let butler = Butler::new();

    // FR17 30-second generation budget.
    let started = Instant::now();
    let digest = butler
        .morning_digest(&db, &journal, now, &anomalies, 0.25)
        .expect("digest composes");
    assert!(
        started.elapsed().as_millis() < 30_000,
        "FR17 30s generation budget must be respected"
    );

    // (a) completed tasks with outcome tags + cited source_log_ref.
    assert_eq!(digest.completed.len(), 2, "two task.complete frames");
    let cited: std::collections::BTreeSet<String> =
        digest.completed.iter().map(|c| c.source_log_ref.clone()).collect();
    assert!(cited.contains(&hex(&c1)) && cited.contains(&hex(&c2)), "both completions cite their frame id");
    assert!(digest.completed.iter().all(|c| c.outcome == "succeeded"));
    assert!(digest.completed.iter().all(|c| c.source_log_ref.len() == 32), "every cite is a 16-byte frame id");

    // (b) open halts.
    assert_eq!(digest.open_halts.len(), 1, "one open halt");
    assert_eq!(digest.open_halts[0].source_log_ref, hex(&h1));

    // (c) anomalies ≥ 0.6 only.
    assert_eq!(digest.anomalies.len(), 1, "only the ≥0.6-confidence anomaly is included");
    assert!((digest.anomalies[0].confidence - 0.82).abs() < 1e-6);

    // (d) trust bar = 1 - predicate-fire-rate.
    assert!((digest.trust_bar - 0.75).abs() < 1e-6);

    // Persist through the I11 distillation port.
    let request = butler.digest_to_distillation_request(&digest).expect("request builds");
    assert_eq!(request.source_log_ref.len(), 3, "2 completions + 1 halt");
    assert!(request.distillation_depth >= 1, "I11: depth ≥ 1");
    assert_eq!(request.source_log_ref[0], c1);
    assert_eq!(request.source_log_ref[2], h1);

    let memory: Arc<dyn std::any::Any + Send + Sync> = Arc::new(0u8);
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);
    let receipt = writer.write_distillate(1, request).expect("write succeeds — all refs resolve");
    assert!(receipt.effective_distillation_depth >= 1);
    assert!(!receipt.effective_source_log_ref.is_empty(), "kernel computed a non-empty audit chain");
    assert!(!receipt.digest_frame_id.iter().all(|b| *b == 0), "digest frame id minted");
}

#[test]
fn ac5_missing_audit_chain_is_rejected_kernel_side() {
    // A digest whose distillation request carries NO source_log_ref must be
    // rejected by the kernel writer with EDigestAuditChainMissing — the I11
    // enforcement is NOT bypassed Spirit-side.
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("transparency.sqlite");
    let (tl, _c1, _c2, _h1) = seed_tl(&db);

    let memory: Arc<dyn std::any::Any + Send + Sync> = Arc::new(0u8);
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);

    // Struct literal bypasses DistillationRequest::new's author-side guard, so
    // the WRITER's (kernel-side) enforcement is what we exercise.
    let request = DistillationRequest {
        source_log_ref: vec![],
        distillation_depth: 1,
        digest_payload: DigestPayload::Text("hallucinated digest with no backing".into()),
        segment_hint: None,
    };
    let err = writer.write_distillate(1, request).unwrap_err();
    assert!(
        matches!(err, DistillationError::AuditChainMissing { .. }),
        "EDigestAuditChainMissing expected, got {err:?}"
    );
}

#[test]
fn empty_digest_rejected_by_distillation_request() {
    // An empty digest (no completions, no halts) must be rejected at the
    // DistillationRequest::new layer with AuditChainMissing — the normal API
    // path is untested elsewhere (only the struct-literal bypass is tested
    // in the negative test above).
    let digest = butler::MorningDigest {
        completed: vec![],
        open_halts: vec![],
        anomalies: vec![],
        trust_bar: 1.0,
    };
    let butler = Butler::new();
    let err = butler.digest_to_distillation_request(&digest).unwrap_err();
    assert!(
        matches!(
            err,
            butler::ButlerError::Distillation(
                maos_domain::distillation::DistillationError::AuditChainMissing { .. }
            )
        ),
        "empty digest must fail with AuditChainMissing, got {err:?}"
    );
}
