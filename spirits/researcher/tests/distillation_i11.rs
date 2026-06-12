//! AC3 — Researcher's distillate is written through the kernel-enforced I11
//! audit chain, PROVEN end-to-end against the real `DistillateWriter` +
//! `LogRecallAdapter` (dev-deps). Researcher walks the scoped log, surveys the
//! recalled frames, and persists the digest; the kernel computes the
//! transitively-flattened `effective_source_log_ref` and the `intent_lineage`
//! (I13 — NEVER Spirit-self-reported). A missing audit chain is rejected
//! kernel-side, not bypassed Spirit-side.

use std::sync::Arc;

use researcher::Researcher;

use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::LogRecallFilter;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0x_D15_7111;

fn claim_bytes(claim_id: &str, conf: f32, topic: &str, polarity: bool) -> Vec<u8> {
    let claim = researcher::ClaimPayload {
        claim_id: claim_id.into(),
        statement: "the effect is likely present".into(),
        topic: topic.into(),
        methodology_strength: 0.9,
        confidence: conf,
        load_bearing: true,
        polarity,
        hedges: vec!["likely".into()],
    };
    serde_json::to_vec(&claim).unwrap()
}

/// Seed claim frames for `pid` with the given intents (one frame each).
fn seed(tl: &Arc<TransparencyLogAdapter>, pid: u32, intents: &[&str]) -> Vec<[u8; 16]> {
    let mut ids = Vec::new();
    for (i, intent) in intents.iter().enumerate() {
        let _ = tl.insert_frame_event(
            FrameKind::InferenceCall,
            pid,
            None,
            intent,
            &claim_bytes(&format!("c{pid}-{i}"), 0.9, "fusion", i % 2 == 0),
            FrameOrigin::SpiritAuto,
        );
        ids.push(tl.last_frame_id());
    }
    ids
}

fn memory() -> Arc<dyn std::any::Any + Send + Sync> {
    Arc::new(0u8)
}

#[test]
fn researcher_distillate_resolves_the_i11_chain_with_kernel_intent_lineage() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    // Three source frames with distinct intents: lineage = sorted union.
    let raw = seed(&tl, 10, &["consult", "inform", "verify"]);
    let recall = LogRecallAdapter::new(Arc::clone(&tl));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory());

    let researcher = Researcher::new();

    // Walk → survey → distill, all over the REAL adapters.
    let frames = researcher
        .walk(&recall, 10, LogRecallFilter::default()) // pid 10 = researcher
        .expect("walk");
    assert_eq!(frames.len(), 3);
    let survey = researcher.survey(&frames);
    let receipt = researcher
        .distill_through(&writer, 10, &survey, 1) // depth 1 = direct digest over raw frames
        .expect("write_distillate succeeds — every cited ref resolves");

    // Non-empty audit chain, digest frame minted.
    assert!(!receipt.effective_source_log_ref.is_empty());
    assert!(!receipt.digest_frame_id.iter().all(|b| *b == 0));
    assert_eq!(receipt.effective_distillation_depth, 1);

    // effective_source_log_ref == the three raw frames (exact set).
    let mut got = receipt.effective_source_log_ref.clone();
    got.sort();
    let mut want = raw.clone();
    want.sort();
    assert_eq!(got, want, "depth-1 digest over raws flattens to the raws");

    // Kernel-computed intent_lineage = sorted union of source intents.
    let lineage: Vec<&str> = receipt
        .intent_lineage
        .as_slice()
        .iter()
        .map(|i| i.as_str())
        .collect();
    assert_eq!(lineage, vec!["consult", "inform", "verify"]);
}

#[test]
fn digest_of_digest_flattens_to_original_raw_frames() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0xA));
    let raw = seed(&tl, 10, &["consult", "inform"]);
    let recall = LogRecallAdapter::new(Arc::clone(&tl));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory());
    let researcher = Researcher::new();

    // Hop 1: digest over the raws.
    let frames = researcher
        .walk(&recall, 10, LogRecallFilter::default())
        .unwrap();
    let survey = researcher.survey(&frames);
    let r1 = researcher.distill_through(&writer, 10, &survey, 1).unwrap();

    // Hop 2: a digest whose only source is the hop-1 digest. The kernel must
    // transitively flatten back to the ORIGINAL raw frames (I11).
    let req2 = DistillationRequest::new(
        vec![r1.digest_frame_id],
        2,
        DigestPayload::Text("digest-of-digest".into()),
        None,
    )
    .unwrap();
    let r2 = writer.write_distillate(10, req2).unwrap();

    let mut got = r2.effective_source_log_ref.clone();
    got.sort();
    let mut want = raw.clone();
    want.sort();
    assert_eq!(got, want, "digest-of-digest flattens to the original raws");
    assert_eq!(r2.effective_distillation_depth, 2);
}

#[test]
fn missing_audit_chain_is_rejected_kernel_side() {
    // A digest with an EMPTY source_log_ref must be rejected by the kernel
    // writer with EDigestAuditChainMissing — I11 is not bypassed Spirit-side.
    // The struct literal bypasses DistillationRequest::new's author guard, so
    // the WRITER's (kernel) enforcement is what we exercise.
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0xB));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory());

    let bad = DistillationRequest {
        source_log_ref: vec![],
        distillation_depth: 1,
        digest_payload: DigestPayload::Text("hallucinated digest with no backing".into()),
        segment_hint: None,
    };
    let err = writer.write_distillate(10, bad).unwrap_err();
    assert!(
        matches!(err, DistillationError::AuditChainMissing { .. }),
        "EDigestAuditChainMissing expected, got {err:?}"
    );
}

#[test]
fn fabricated_source_ref_fails_loud() {
    // A cited frame id that does not exist in the TL must fail loud
    // (SourceFrameNotFound), never silently produce an unbacked digest.
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0xC));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory());
    let req = DistillationRequest::new(
        vec![[0xDE; 16]],
        1,
        DigestPayload::Text("digest of a fabricated ref".into()),
        None,
    )
    .unwrap();
    let err = writer.write_distillate(10, req).unwrap_err();
    assert!(
        matches!(err, DistillationError::SourceFrameNotFound { .. }),
        "fabricated ref must fail loud, got {err:?}"
    );
}

#[test]
fn empty_survey_is_rejected_at_the_request_layer() {
    // Researcher's own author-guard: a survey citing no frames cannot even
    // build a DistillationRequest (AuditChainMissing before the kernel).
    let researcher = Researcher::new();
    let empty = researcher.survey(&[]);
    let err = researcher.to_distillation_request(&empty, 1).unwrap_err();
    assert!(matches!(
        err,
        researcher::ResearcherError::Distillation(DistillationError::AuditChainMissing { .. })
    ));
}
