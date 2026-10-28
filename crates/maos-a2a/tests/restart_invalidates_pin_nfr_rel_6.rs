//! Story 6.3 AC4 — NFR-Rel-6 Spirit-restart TOFU pin invalidation + re-pin
//! consent confirmation — 6-scenario integration test, plus the Story 19-7
//! store vectors for the production decision source (AC2: pending request,
//! supersede, TTL, accept, reject, nonce mismatch, expiry, concurrent
//! detection during a decision, detection sink, rotation-at-detection
//! refusal, monotonic `expired` flag, sink ordering).
//!
//! Not covered here: the consume-time refusals for an absent pin, a pin no
//! longer `Invalidated::SpiritRestarted`, or a pin whose fingerprint differs
//! from the request's. `InMemoryTofuPinStore` exposes no pin-removal or
//! manual-invalidation method and nothing re-fingerprints an invalidated
//! pin, so no public path reaches those states; the guards in
//! `await_repin_consent` are defensive (a consent never resurrects a removed
//! or manually invalidated pin should such an API be added).
//!
//! Scenarios per AC4:
//!   4.1 First-contact pin + subsequent connection succeeds
//!   4.2 Restart → new boot_nonce; pin invalidated; awaiting re-pin
//!   4.3 Operator ACCEPT → re-pin recorded with approval_id; resume
//!   4.4 Operator REJECT → inbound closed; subsequent outbound errors
//!   4.5 Operator timeout (no response) → TimedOut
//!   4.6 Adversarial impersonation (same spirit_id, different cert) → EPinMismatch

use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_a2a::tofu::{
    Invalidated, RepinDecisionError, RepinRequest, RepinSink, RepinVerdict, REPIN_REQUEST_TTL,
};
use maos_a2a::{
    EPinMismatch, InMemoryTofuPinStore, PeerCertFingerprint, PeerId, RePinDecision, TofuPinStore,
};

fn fp(seed: &str) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(seed.as_bytes())
}

const APPROVAL: [u8; 16] = [0x19; 16];

fn approve() -> RepinVerdict {
    RepinVerdict::Approve {
        approval_id: APPROVAL,
    }
}

/// A store holding `peer` pinned to `cert` at boot nonce 1, then a restart
/// detection observing `observed` — the intake path (`router.rs` calls
/// `invalidate_if_boot_nonce_differs`), which records the pending request.
async fn detected(
    store: &InMemoryTofuPinStore,
    peer: &PeerId,
    cert: &PeerCertFingerprint,
    observed: u64,
) {
    store
        .pin_first_contact(peer, cert, cert, 1)
        .await
        .expect("pin");
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(peer, observed)
            .await
            .expect("detect"),
        Some(1)
    );
}

fn recording_sink() -> (RepinSink, Arc<Mutex<Vec<RepinRequest>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let sink: RepinSink = Arc::new(move |request: &RepinRequest| {
        sink_seen.lock().expect("sink log").push(request.clone());
    });
    (sink, seen)
}

#[tokio::test]
async fn scenario_4_1_first_contact_then_verify_succeeds() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let cert = fp("cert-1");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");
    store.verify_pinned(&peer, &cert).await.expect("verify");
}

#[tokio::test]
async fn scenario_4_2_spirit_restart_invalidates_pin() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let cert = fp("cert-1");
    let pin = store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");
    assert_eq!(pin.boot_nonce, 1);

    // Spirit A crashes + restarts (boot_nonce rolls 1 → 2).
    store
        .invalidate_for_restart(&peer, /* prior_boot_nonce */ 1)
        .await
        .expect("invalidate");

    // Now verify_pinned reports `NotPinned` (the pin is invalidated; awaiting re-pin).
    let err = store
        .verify_pinned(&peer, &cert)
        .await
        .expect_err("must be invalidated");
    assert!(matches!(err, EPinMismatch::Invalidated { .. }));
}

#[tokio::test]
async fn scenario_4_3_operator_accept_repin() {
    // The store method keeps its general semantics (D-19-7-04: the door
    // constrains the fingerprint, the store does not) — a new cert re-pins.
    let approval_id = [0xAB; 16];
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let cert_v1 = fp("cert-v1");
    let cert_v2 = fp("cert-v2");
    detected(&store, &peer, &cert_v1, 7).await;
    store
        .record_repin_decision(&peer, 7, RepinVerdict::Approve { approval_id })
        .expect("record accept");

    let decision = store.await_repin_consent(&peer, &cert_v2, 7).await;
    match decision {
        RePinDecision::AcceptedByOperator { approval_id: aid } => {
            assert_eq!(aid, approval_id);
        }
        _ => panic!("expected accept"),
    }

    // Subsequent verify against v2 succeeds; v1 fails.
    store
        .verify_pinned(&peer, &cert_v2)
        .await
        .expect("verify v2");
    let err = store
        .verify_pinned(&peer, &cert_v1)
        .await
        .expect_err("must mismatch v1");
    assert!(matches!(err, EPinMismatch::Mismatch { .. }));

    // The new pin record carries the approval_id.
    let pin = store.get_pin(&peer).await.expect("get pin");
    assert_eq!(pin.repin_approval_id, Some(approval_id));
}

#[tokio::test]
async fn scenario_4_4_operator_reject_repin_closes_stream() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let cert_v1 = fp("cert-v1");
    let cert_v2 = fp("cert-v2");
    detected(&store, &peer, &cert_v1, 7).await;
    store
        .record_repin_decision(
            &peer,
            7,
            RepinVerdict::Reject {
                reason: "not approved".into(),
            },
        )
        .expect("record reject");

    let decision = store.await_repin_consent(&peer, &cert_v2, 7).await;
    assert!(matches!(decision, RePinDecision::RejectedByOperator { .. }));

    // No pin materialized — subsequent verify still Invalidated.
    let err = store
        .verify_pinned(&peer, &cert_v2)
        .await
        .expect_err("must not pin");
    assert!(matches!(err, EPinMismatch::Invalidated { .. }));
}

#[tokio::test]
async fn scenario_4_5_operator_timeout() {
    // No pending request (invalidated outside intake detection) ⇒ TimedOut.
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let cert_v1 = fp("cert-v1");
    let cert_v2 = fp("cert-v2");
    store
        .pin_first_contact(&peer, &cert_v1, &cert_v1, 1)
        .await
        .expect("pin v1");
    store
        .invalidate_for_restart(&peer, 1)
        .await
        .expect("invalidate");
    let decision = store.await_repin_consent(&peer, &cert_v2, 7).await;
    assert!(matches!(decision, RePinDecision::TimedOut));
}

#[tokio::test]
async fn scenario_4_6_adversarial_impersonation_pin_mismatch() {
    // An adversarial peer presents a cert with the SAME peer_id but a
    // DIFFERENT cert fingerprint (impersonation). The pin store fires
    // `EPinMismatch::Mismatch` BEFORE the boot_nonce check.
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-b");
    let legitimate = fp("cert-legitimate");
    let adversary = fp("cert-adversarial");
    store
        .pin_first_contact(&peer, &legitimate, &legitimate, 1)
        .await
        .expect("pin legitimate");
    let err = store
        .verify_pinned(&peer, &adversary)
        .await
        .expect_err("must mismatch");
    assert!(matches!(err, EPinMismatch::Mismatch { .. }));
}

// ── Story 19-7 (AC2) — the production decision source ────────────────────

#[tokio::test]
async fn repin_19_7_detection_records_one_pending_request_and_no_decision_times_out() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;

    let pending = store.pending_repins();
    assert_eq!(pending.len(), 1);
    let request = &pending[0];
    assert_eq!(request.peer, peer);
    assert_eq!(request.pinned_fingerprint, cert);
    assert_eq!(
        (request.prior_boot_nonce, request.observed_boot_nonce),
        (1, 7)
    );
    assert_eq!(
        request
            .expires_at
            .duration_since(request.detected_at)
            .expect("expiry after detection"),
        REPIN_REQUEST_TTL
    );
    assert_eq!(REPIN_REQUEST_TTL, Duration::from_secs(300));

    // CR-1: no recorded decision ⇒ TimedOut; the pin stays invalidated and
    // the request stays pending.
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    assert!(matches!(
        store.verify_pinned(&peer, &cert).await,
        Err(EPinMismatch::Invalidated { .. })
    ));
    assert_eq!(store.pending_repins().len(), 1);
}

#[tokio::test]
async fn repin_19_7_accept_repins_the_current_fingerprint_with_the_observed_nonce() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;

    // The door path (D-19-7-04/05 A): record, then call the trait method
    // with the request's pinned fingerprint and observed nonce.
    let request = store
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");
    assert_eq!(
        store
            .await_repin_consent(
                &peer,
                &request.pinned_fingerprint,
                request.observed_boot_nonce
            )
            .await,
        RePinDecision::AcceptedByOperator {
            approval_id: APPROVAL
        }
    );
    // The consumed decision cannot be replayed: the SAME nonce re-awaited
    // right after the accept is not accepted a second time.
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    let pin = store.get_pin(&peer).await.expect("re-pinned");
    assert_eq!(pin.fingerprint, cert);
    assert_eq!(pin.boot_nonce, 7);
    assert_eq!(pin.invalidated, None);
    assert_eq!(pin.repin_approval_id, Some(APPROVAL));
    assert_eq!(store.rotation_next(&peer), None);
    assert!(store.pending_repins().is_empty());
    store.verify_pinned(&peer, &cert).await.expect("verify");

    // Restart detection is re-armed on the NEW nonce: the consented nonce
    // passes, a further restart is detected against it.
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 7)
            .await
            .expect("same nonce"),
        None
    );
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 8)
            .await
            .expect("next restart"),
        Some(7)
    );
    // The re-armed detection carries no verdict of its own.
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 8).await,
        RePinDecision::TimedOut
    );
}

#[tokio::test]
async fn repin_19_7_reject_consumes_the_request_and_leaves_the_pin_invalidated() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    store
        .record_repin_decision(
            &peer,
            7,
            RepinVerdict::Reject {
                reason: "unknown restart".into(),
            },
        )
        .expect("record reject");
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::RejectedByOperator {
            reason: "unknown restart".into()
        }
    );
    assert!(store.pending_repins().is_empty());
    assert!(matches!(
        store.verify_pinned(&peer, &cert).await,
        Err(EPinMismatch::Invalidated { .. })
    ));
    assert_eq!(
        store.record_repin_decision(&peer, 7, approve()),
        Err(RepinDecisionError::NotPending {
            peer: "host-a".into()
        })
    );

    // A reject followed by a fresh detection opens a new request.
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 7)
            .await
            .expect("re-detect"),
        Some(1)
    );
    assert_eq!(store.pending_repins().len(), 1);
}

#[tokio::test]
async fn repin_19_7_nonce_mismatch_is_typed_and_changes_nothing() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    assert_eq!(
        store.record_repin_decision(&peer, 8, approve()),
        Err(RepinDecisionError::NonceMismatch {
            peer: "host-a".into(),
            observed: 7,
            supplied: 8,
        })
    );
    // Nothing was recorded: the observed nonce still times out.
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );

    // A recorded decision is not applied under another nonce.
    store
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 8).await,
        RePinDecision::TimedOut
    );
    let pin = store.get_pin(&peer).await.expect("pin");
    assert!(pin.invalidated.is_some());
    assert_eq!(pin.boot_nonce, 1);
}

#[tokio::test]
async fn repin_19_7_no_pending_request_is_typed() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let not_pending = Err(RepinDecisionError::NotPending {
        peer: "host-a".into(),
    });
    assert_eq!(
        store.record_repin_decision(&peer, 7, approve()),
        not_pending
    );
    let cert = fp("cert-a");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");
    assert_eq!(
        store.record_repin_decision(&peer, 7, approve()),
        not_pending
    );
    assert!(store.pending_repins().is_empty());
}

#[tokio::test]
async fn repin_19_7_expired_request_is_typed_and_times_out() {
    let store = InMemoryTofuPinStore::new().with_repin_ttl(Duration::ZERO);
    let (sink, seen) = recording_sink();
    assert!(store.install_repin_sink(sink));
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;

    // Expired requests stay listed so a decision reads Expired, not 404.
    assert_eq!(store.pending_repins().len(), 1);
    assert_eq!(
        store.record_repin_decision(&peer, 7, approve()),
        Err(RepinDecisionError::Expired {
            peer: "host-a".into()
        })
    );
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    assert!(store
        .get_pin(&peer)
        .await
        .expect("pin")
        .invalidated
        .is_some());

    // The same nonce arriving after expiry is a NEW detection.
    store
        .invalidate_if_boot_nonce_differs(&peer, 7)
        .await
        .expect("re-detect");
    assert_eq!(seen.lock().expect("sink log").len(), 2);
}

#[tokio::test]
async fn repin_19_7_decision_that_outlives_the_request_times_out() {
    let store = InMemoryTofuPinStore::new().with_repin_ttl(Duration::from_millis(500));
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    store
        .record_repin_decision(&peer, 7, approve())
        .expect("recorded before expiry");
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    assert!(store
        .get_pin(&peer)
        .await
        .expect("pin")
        .invalidated
        .is_some());
}

#[tokio::test]
async fn repin_19_7_newer_detection_during_a_decision_supersedes_it() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    store
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");

    // The peer restarts again before the decision is applied.
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 9)
            .await
            .expect("second restart"),
        Some(1)
    );
    let pending = store.pending_repins();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].observed_boot_nonce, 9);
    assert_eq!(pending[0].prior_boot_nonce, 1);

    // The old waiter times out; the superseding request has no decision.
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 9).await,
        RePinDecision::TimedOut
    );
    assert_eq!(
        store.record_repin_decision(&peer, 7, approve()),
        Err(RepinDecisionError::NonceMismatch {
            peer: "host-a".into(),
            observed: 9,
            supplied: 7,
        })
    );
    assert!(store
        .get_pin(&peer)
        .await
        .expect("pin")
        .invalidated
        .is_some());
}

#[tokio::test]
async fn repin_19_7_repeat_detection_of_the_same_nonce_keeps_the_decision() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    let first = store.pending_repins();
    store
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");

    // The holding sender re-sends (D-19-7-09): same nonce, same request.
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 7)
            .await
            .expect("re-send"),
        Some(1)
    );
    assert_eq!(store.pending_repins(), first);
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::AcceptedByOperator {
            approval_id: APPROVAL
        }
    );
}

#[tokio::test]
async fn repin_19_7_withdraw_drops_only_the_decision() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    store
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");
    assert!(store.withdraw_repin_decision(&peer));
    assert!(!store.withdraw_repin_decision(&peer));
    assert_eq!(
        store.await_repin_consent(&peer, &cert, 7).await,
        RePinDecision::TimedOut
    );
    assert_eq!(store.pending_repins().len(), 1);
    assert!(store
        .get_pin(&peer)
        .await
        .expect("pin")
        .invalidated
        .is_some());
}

#[tokio::test]
async fn repin_19_7_detection_sink_is_set_once_and_fires_per_new_detection() {
    let store = InMemoryTofuPinStore::new();
    let (sink, seen) = recording_sink();
    assert!(store.install_repin_sink(sink));
    let (second, second_seen) = recording_sink();
    assert!(!store.install_repin_sink(second), "the sink is set-once");

    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    detected(&store, &peer, &cert, 7).await;
    store
        .invalidate_if_boot_nonce_differs(&peer, 7)
        .await
        .expect("repeat");
    store
        .invalidate_if_boot_nonce_differs(&peer, 9)
        .await
        .expect("supersede");

    let seen = seen.lock().expect("sink log");
    let nonces: Vec<(u64, u64)> = seen
        .iter()
        .map(|r| (r.prior_boot_nonce, r.observed_boot_nonce))
        .collect();
    assert_eq!(nonces, vec![(1, 7), (1, 9)]);
    assert_eq!(seen[1], store.pending_repins()[0]);
    assert!(second_seen.lock().expect("unused sink log").is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn repin_19_7_concurrent_detections_record_one_request() {
    let store = Arc::new(InMemoryTofuPinStore::new());
    let (sink, seen) = recording_sink();
    assert!(store.install_repin_sink(sink));
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");

    let tasks: Vec<_> = (0..32)
        .map(|_| {
            let store = Arc::clone(&store);
            let peer = peer.clone();
            tokio::spawn(async move { store.invalidate_if_boot_nonce_differs(&peer, 7).await })
        })
        .collect();
    for task in tasks {
        assert_eq!(task.await.expect("join").expect("detect"), Some(1));
    }
    assert_eq!(seen.lock().expect("sink log").len(), 1);
    assert_eq!(store.pending_repins().len(), 1);
}

/// Edge F4 — a restart detected while a rotation window was open: the
/// detection reads the window state BEFORE clearing it, the approval is
/// recorded, and the store refuses to materialize it; the request is
/// consumed and the pin is left exactly as the detection left it.
#[tokio::test]
async fn repin_19_7_approval_of_a_restart_detected_mid_rotation_is_refused_by_the_store() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    let next = fp("cert-a-next");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");
    store
        .open_rotation_window(&peer, &next)
        .expect("window opens on the active pin");
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 7)
            .await
            .expect("detect"),
        Some(1)
    );
    assert_eq!(
        store.rotation_next(&peer),
        None,
        "detection clears the window"
    );

    let request = store
        .record_repin_decision(&peer, 7, approve())
        .expect("the verdict itself is recorded");
    assert!(matches!(
        store
            .await_repin_consent(&peer, &request.pinned_fingerprint, 7)
            .await,
        RePinDecision::RejectedByStore {
            error: EPinMismatch::Invalidated { .. }
        }
    ));
    assert!(store.pending_repins().is_empty(), "the request is consumed");
    let pin = store.get_pin(&peer).await.expect("pin kept");
    assert_eq!(pin.fingerprint, cert);
    assert_eq!(pin.boot_nonce, 1);
    assert_eq!(
        pin.invalidated,
        Some(Invalidated::SpiritRestarted {
            prior_boot_nonce: 1
        })
    );
    assert_eq!(pin.repin_approval_id, None);
    assert_eq!(store.rotation_next(&peer), None);
}

/// Edge F4 — the rotation-at-detection mark survives a superseding
/// detection (the first detection already cleared the window).
#[tokio::test]
async fn repin_19_7_rotation_mark_survives_a_superseding_detection() {
    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");
    store
        .open_rotation_window(&peer, &fp("cert-a-next"))
        .expect("window opens");
    store
        .invalidate_if_boot_nonce_differs(&peer, 7)
        .await
        .expect("detect mid-rotation");
    store
        .invalidate_if_boot_nonce_differs(&peer, 9)
        .await
        .expect("supersede");
    store
        .record_repin_decision(&peer, 9, approve())
        .expect("record accept");
    assert!(matches!(
        store.await_repin_consent(&peer, &cert, 9).await,
        RePinDecision::RejectedByStore {
            error: EPinMismatch::Invalidated { .. }
        }
    ));
    assert!(store
        .get_pin(&peer)
        .await
        .expect("pin")
        .invalidated
        .is_some());
}

/// Blind#6 / TI-8 — `expired` is stamped by `pending_repins()` from the
/// store's monotonic deadline; the sink and `record_repin_decision` values
/// carry `false`.
#[tokio::test]
async fn repin_19_7_pending_requests_carry_the_monotonic_expired_flag() {
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");

    let lapsed = InMemoryTofuPinStore::new().with_repin_ttl(Duration::ZERO);
    let (sink, seen) = recording_sink();
    assert!(lapsed.install_repin_sink(sink));
    detected(&lapsed, &peer, &cert, 7).await;
    let pending = lapsed.pending_repins();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].expired, "TTL 0 lapses at detection");
    assert!(!seen.lock().expect("sink log")[0].expired);

    let live = InMemoryTofuPinStore::new().with_repin_ttl(Duration::from_secs(3600));
    detected(&live, &peer, &cert, 7).await;
    assert!(!live.pending_repins()[0].expired);
    let request = live
        .record_repin_decision(&peer, 7, approve())
        .expect("record accept");
    assert!(!request.expired);
    assert_eq!(live.pending_repins()[0], request);
}

/// Blind#5 — the sink runs under the store's lifecycle lock, so its rows
/// follow detection order: under concurrent superseding detections the
/// last row delivered is the request that ended up pending.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn repin_19_7_sink_rows_follow_detection_order() {
    let store = Arc::new(InMemoryTofuPinStore::new());
    let (sink, seen) = recording_sink();
    assert!(store.install_repin_sink(sink));
    let peer = PeerId::new("host-a");
    let cert = fp("cert-a");
    store
        .pin_first_contact(&peer, &cert, &cert, 1)
        .await
        .expect("pin");

    let tasks: Vec<_> = (2..34)
        .map(|nonce| {
            let store = Arc::clone(&store);
            let peer = peer.clone();
            tokio::spawn(async move { store.invalidate_if_boot_nonce_differs(&peer, nonce).await })
        })
        .collect();
    for task in tasks {
        assert_eq!(task.await.expect("join").expect("detect"), Some(1));
    }
    let seen = seen.lock().expect("sink log");
    assert_eq!(seen.len(), 32, "every distinct nonce supersedes");
    assert_eq!(seen.last(), store.pending_repins().first());
}
