//! AC2 — the participant-scoped `log.recall` walker, PROVEN against the real
//! Story 4.4 `LogRecallAdapter` driven as a dev-dependency (Butler's resolved
//! kernel-adapter-as-dev-dep pattern; the researcher lib itself never reaches
//! into the kernel — Story 0.2).
//!
//! The explicit 8.1→8.2 contract: Researcher walks the SCOPED `LogRecallPort`
//! (results limited to the calling Spirit's emitter frames; a cross-Spirit
//! `fetch` → `LogRecallError::ScopeViolation`). It does NOT use Butler's
//! unscoped `ranged_recall` — the researcher crate has NO `maos-audit`
//! dependency, so `ranged_recall` is unreachable at compile time.

use std::sync::Arc;

use researcher::{ClaimPayload, Researcher};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{LogRecallError, LogRecallFilter};
use maos_domain::ports::LogRecallPort;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0x_8E5_EA5C;

fn claim_bytes(claim_id: &str, conf: f32) -> Vec<u8> {
    let claim = ClaimPayload {
        claim_id: claim_id.into(),
        statement: "the effect is likely present".into(),
        topic: "fusion".into(),
        methodology_strength: 0.9,
        confidence: conf,
        load_bearing: true,
        polarity: true,
        hedges: vec!["likely".into(), "uncertain".into()],
    };
    serde_json::to_vec(&claim).unwrap()
}

/// Seed `count` claim frames emitted by `pid`. Returns the minted frame ids.
fn seed_claims(tl: &Arc<TransparencyLogAdapter>, pid: u32, count: usize) -> Vec<[u8; 16]> {
    let mut ids = Vec::new();
    for i in 0..count {
        let _ = tl.insert_frame_event(
            FrameKind::InferenceCall,
            pid,
            None,
            "inform",
            &claim_bytes(&format!("claim-{pid}-{i}"), 0.9),
            FrameOrigin::SpiritAuto,
        );
        ids.push(tl.last_frame_id());
    }
    ids
}

#[test]
fn walker_is_participant_scoped_and_paginates() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    seed_claims(&tl, 10, 5); // researcher pid
    seed_claims(&tl, 20, 4); // a different Spirit
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));

    let researcher = Researcher::new();

    // Walk pid 10 with a small page size so recall_all must follow next_cursor
    // across multiple pages — all 5 of pid 10's frames, NONE of pid 20's.
    let frames = researcher
        .walk(
            &adapter,
            10,
            LogRecallFilter::new(None, None, None, 2, None, None),
        )
        .expect("walk succeeds");
    assert_eq!(
        frames.len(),
        5,
        "exactly the 5 emitter-scoped frames of pid 10"
    );
    // Every payload is a real claim the survey can parse (walker fetched it).
    for f in &frames {
        let claim: ClaimPayload =
            serde_json::from_slice(&f.payload).expect("each recalled frame carries a claim");
        assert!(
            claim.claim_id.starts_with("claim-10-"),
            "only pid-10 claims"
        );
    }

    // Walk pid 20 — sees ONLY its own 4 frames (scope isolation, both directions).
    let other = researcher
        .walk(&adapter, 20, LogRecallFilter::default())
        .expect("walk succeeds");
    assert_eq!(other.len(), 4);
    for f in &other {
        let claim: ClaimPayload = serde_json::from_slice(&f.payload).unwrap();
        assert!(claim.claim_id.starts_with("claim-20-"));
    }
}

#[test]
fn cross_spirit_fetch_is_a_scope_violation() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0x1));
    let pid10 = seed_claims(&tl, 10, 1);
    let frame_id = pid10[0];
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));

    // pid 20 fetching pid 10's frame → ScopeViolation (the negative the
    // walker's participant-scoping rests on; driven through the REAL adapter).
    let err = adapter.fetch(20, frame_id).unwrap_err();
    match err {
        LogRecallError::ScopeViolation {
            frame_id: fid,
            requested_pid,
            owner_pid,
        } => {
            assert_eq!(fid, frame_id);
            assert_eq!(requested_pid, 20);
            assert_eq!(owner_pid, 10);
        }
        other => panic!("expected ScopeViolation, got {other:?}"),
    }

    // The emitter itself fetches fine.
    assert!(
        adapter.fetch(10, frame_id).is_ok(),
        "emitter may fetch its own frame"
    );
}

#[test]
fn empty_log_walks_to_empty() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0x2));
    let adapter = LogRecallAdapter::new(tl);
    let frames = Researcher::new()
        .walk(&adapter, 10, LogRecallFilter::default())
        .expect("walk succeeds on empty log");
    assert!(frames.is_empty());
}
