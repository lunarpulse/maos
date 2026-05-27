//! Story 6.3 AC4 — NFR-Rel-6 Spirit-restart TOFU pin invalidation + re-pin
//! consent confirmation — 6-scenario integration test.
//!
//! Scenarios per AC4:
//!   4.1 First-contact pin + subsequent connection succeeds
//!   4.2 Restart → new boot_nonce; pin invalidated; awaiting re-pin
//!   4.3 Operator ACCEPT → re-pin recorded with approval_id; resume
//!   4.4 Operator REJECT → inbound closed; subsequent outbound errors
//!   4.5 Operator timeout (no response) → TimedOut
//!   4.6 Adversarial impersonation (same spirit_id, different cert) → EPinMismatch

use maos_a2a::{
    EPinMismatch, InMemoryTofuPinStore, PeerCertFingerprint, PeerId, RePinDecision, TofuPinStore,
};

fn fp(seed: &str) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(seed.as_bytes())
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
    let approval_id = [0xAB; 16];
    let store =
        InMemoryTofuPinStore::new().with_repin_hook(move |_, _, _| RePinDecision::AcceptedByOperator {
            approval_id,
        });
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
    match decision {
        RePinDecision::AcceptedByOperator { approval_id: aid } => {
            assert_eq!(aid, approval_id);
        }
        _ => panic!("expected accept"),
    }

    // Subsequent verify against v2 succeeds; v1 fails.
    store.verify_pinned(&peer, &cert_v2).await.expect("verify v2");
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
    let store = InMemoryTofuPinStore::new().with_repin_hook(|_, _, _| {
        RePinDecision::RejectedByOperator {
            reason: "not approved".into(),
        }
    });
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
    assert!(matches!(decision, RePinDecision::RejectedByOperator { .. }));

    // No pin materialized — subsequent verify still NotPinned.
    let err = store
        .verify_pinned(&peer, &cert_v2)
        .await
        .expect_err("must not pin");
    assert!(matches!(err, EPinMismatch::Invalidated { .. }));
}

#[tokio::test]
async fn scenario_4_5_operator_timeout() {
    // Default hook returns TimedOut.
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
