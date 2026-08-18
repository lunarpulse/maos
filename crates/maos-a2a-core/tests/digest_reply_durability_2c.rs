//! Story `j1-crosshost-2c` AC3.5 / `deferred-work.md:819` — **nothing is
//! `Duplicate` until something is durable**, asserted at the router seam.
//!
//! The digest-reply path recorded its dedup and only THEN handed the frame to
//! the intake sink. A sender that retried after a dropped-receiver NACK was
//! answered `Duplicate` — an ACK carrying `delivered: true` — while the frame was
//! still gone. That is the same durability lie `j1-crosshost-2b` fixed one layer
//! over (*"an ACK here is a lie about durability, and the sender has no other way
//! to learn the frame is gone"*), reached through a different door.
//!
//! The fix makes the intake push the reply's **commit guard**: it runs after every
//! authorization check and before any dedup state is published. These legs hold
//! the ROUTER to that ordering with a port that mirrors the real
//! `CohortManifestState` contract; the state machine itself is held by
//! `maos-cohort`'s `a_failed_commit_leaves_the_digest_reply_retryable_never_duplicate`.

use std::collections::HashSet;
use std::sync::Arc;

use maos_a2a_core::router::A2ARouterCore;
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_INTERNAL, METHOD_IAC_DELIVER,
};
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, ConsentAllowlists, DigestFrameClass, DigestReadPort,
    DigestReplyObservation, InMemoryTofuPinStore, PeerCertFingerprint, PeerId, TofuPinStore,
    COHORT_INTENT_DIGEST_READ,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use parking_lot::Mutex;
use smallvec::smallvec;

const PEER: &str = "host-b";
const REQUEST_ID: &str = "host-a:0001";

/// A digest-read port that honours the `observe_reply_guarded` contract exactly as
/// `CohortManifestState` does: run the guard after authorization, publish the
/// dedup record only if it succeeded.
#[derive(Default)]
struct GuardedPort {
    recorded: Mutex<HashSet<String>>,
    /// How many times the router asked us to observe a reply.
    observations: Mutex<usize>,
}

impl DigestReadPort for GuardedPort {
    fn classify(&self, _frame: &IacFrame) -> DigestFrameClass {
        DigestFrameClass::Reply {
            request_id: REQUEST_ID.to_string(),
        }
    }

    fn note_admitted_request_guarded(
        &self,
        _requester: &HostId,
        _request_id: &str,
        _frame: &IacFrame,
        _before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        Ok(())
    }

    fn authorize_reply_send(&self, _peer: &HostId, _request_id: &str) -> bool {
        true
    }

    fn observe_reply_guarded(
        &self,
        _peer: &HostId,
        _frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<DigestReplyObservation, String> {
        *self.observations.lock() += 1;
        let mut recorded = self.recorded.lock();
        if recorded.contains(REQUEST_ID) {
            return Ok(DigestReplyObservation::Duplicate);
        }
        // Authorization passed. NOTHING is published before the guard.
        before_commit()?;
        recorded.insert(REQUEST_ID.to_string());
        Ok(DigestReplyObservation::Accepted)
    }
}

fn peer_cfg() -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(PEER),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: PeerCertFingerprint::from_cert_der(b"peer-b"),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(COHORT_INTENT_DIGEST_READ)],
            accept_allowlist: vec![A2AIntent::new(COHORT_INTENT_DIGEST_READ)],
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

async fn core(port: Arc<GuardedPort>) -> A2ARouterCore {
    let cfg = peer_cfg();
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new(PEER),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    A2ARouterCore::new(vec![cfg], tofu).with_digest_read_port(port)
}

/// A `cohort:digest-read` Reply frame from `host-b`.
fn reply_frame() -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-control"),
        host_id: Some(HostId(PEER.into())),
        role: None,
    };
    IacFrame {
        frame_id: [0x2c; 16],
        timestamp_ns: 0,
        logical_clock: 1,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host-a".into())),
            role: None,
        }],
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(TelemetryEventPayload {
            event_type: "maos.cohort-digest-read.v1".into(),
            data: format!("{{\"request_id\":\"{REQUEST_ID}\"}}"),
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(COHORT_INTENT_DIGEST_READ),
        )),
        intent_lineage: IntentLineage::default(),
    }
}

fn request() -> A2AJsonRpcRequest {
    A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, reply_frame(), 1)
}

fn nack_message(response: &A2AJsonRpcResponse) -> String {
    match response {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(n.error.code, CODE_INTERNAL, "expected an internal NACK");
            n.error.message.clone()
        }
        A2AJsonRpcResponse::Ack(a) => panic!(
            "expected a NACK, got an ACK claiming delivered={}",
            a.result.delivered
        ),
    }
}

/// **The `:819` counterexample, now closed.** With a dead consumer, the first
/// reply NACKs — and so must the retry. Before the fix the retry was answered
/// with an ACK claiming `delivered: true` while the frame was still gone.
#[tokio::test]
async fn a_retry_after_a_dropped_receiver_nack_is_never_reported_duplicate() {
    let port = Arc::new(GuardedPort::default());
    let core = core(port.clone()).await;

    let (tx, rx) = tokio::sync::mpsc::channel::<IacFrame>(64);
    core.install_intake_sink(tx).await;
    drop(rx); // the consumer is gone — every push must fail

    let first = core.handle_intake(request()).await;
    let message = nack_message(&first);
    assert!(
        message.contains("digest reply NOT delivered"),
        "the first attempt must say the frame was not delivered: {message}"
    );

    let retry = core.handle_intake(request()).await;
    let retry_message = nack_message(&retry);
    assert!(
        retry_message.contains("digest reply NOT delivered"),
        "the retry must repeat the refusal, not claim a duplicate delivery: {retry_message}"
    );

    assert_eq!(
        *port.observations.lock(),
        2,
        "both attempts must reach the correlation port"
    );
    assert!(
        port.recorded.lock().is_empty(),
        "no dedup record may exist for a frame that was never handed over"
    );
}

/// The happy path still commits exactly once, and the SECOND delivery is a real
/// `Duplicate` — the idempotency AC3 relies on is preserved, not weakened.
#[tokio::test]
async fn a_delivered_reply_commits_once_and_then_reports_duplicate() {
    let port = Arc::new(GuardedPort::default());
    let core = core(port.clone()).await;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<IacFrame>(64);
    core.install_intake_sink(tx).await;

    match core.handle_intake(request()).await {
        A2AJsonRpcResponse::Ack(a) => assert!(a.result.delivered),
        other => panic!("a live consumer must accept the reply: {other:?}"),
    }
    let handed_over = rx.try_recv().expect("the frame must reach the consumer");
    assert_eq!(handed_over.frame_id, [0x2c; 16]);
    assert_eq!(port.recorded.lock().len(), 1);

    // The replay is idempotent and does NOT re-deliver.
    match core.handle_intake(request()).await {
        A2AJsonRpcResponse::Ack(a) => assert!(a.result.delivered),
        other => panic!("a replay must be acknowledged idempotently: {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "a duplicate must not be handed to the consumer twice"
    );
    assert_eq!(port.recorded.lock().len(), 1);
}

/// Backpressure, not just a dead consumer: a FULL channel is the same verdict
/// (2b's D2 contract), and it too must leave the reply retryable.
#[tokio::test]
async fn a_full_intake_channel_also_leaves_the_reply_retryable() {
    let port = Arc::new(GuardedPort::default());
    let core = core(port.clone()).await;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<IacFrame>(1);
    tx.try_send(reply_frame()).expect("fill the single slot");
    core.install_intake_sink(tx).await;

    let full = core.handle_intake(request()).await;
    assert!(nack_message(&full).contains("digest reply NOT delivered"));
    assert!(
        port.recorded.lock().is_empty(),
        "backpressure must not publish a dedup record"
    );

    // Drain, then the retry succeeds — proving the refusal was transient rather
    // than a permanently-lost frame recorded as delivered.
    rx.try_recv().expect("drain");
    match core.handle_intake(request()).await {
        A2AJsonRpcResponse::Ack(a) => assert!(a.result.delivered),
        other => panic!("after draining, the retry must land: {other:?}"),
    }
    assert_eq!(port.recorded.lock().len(), 1);
}
