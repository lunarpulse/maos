//! Story 21-3 / AC1.b, AC1.d — every pin-lifecycle mutation reaches the
//! durability port exactly once, rotation windows never do, a narrowing write
//! failure degrades the store, a widening one refuses the transition.

use maos_a2a_core::tofu::{
    Invalidated, PinPersistence, PinRecord, RePinDecision, RepinRequest, RepinSink, RepinVerdict,
};
use maos_a2a_core::{InMemoryTofuPinStore, PeerCertFingerprint, PeerId, TofuPinStore};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SEED_NONCE: u64 = 10;
const RESTARTED_NONCE: u64 = 11;

#[derive(Default)]
struct Recording {
    snapshots: Mutex<Vec<Vec<PinRecord>>>,
    fail: AtomicBool,
}

impl Recording {
    fn count(&self) -> usize {
        self.snapshots.lock().unwrap().len()
    }

    fn last(&self) -> Vec<PinRecord> {
        self.snapshots.lock().unwrap().last().cloned().unwrap()
    }
}

impl PinPersistence for Recording {
    fn load(&self) -> Vec<PinRecord> {
        Vec::new()
    }

    fn store(&self, records: &[PinRecord]) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("disk full".into());
        }
        self.snapshots.lock().unwrap().push(records.to_vec());
        Ok(())
    }
}

fn fp(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(&[seed; 8])
}

fn peer() -> PeerId {
    PeerId::new("peer-a")
}

fn fixture() -> (Arc<InMemoryTofuPinStore>, Arc<Recording>) {
    let port = Arc::new(Recording::default());
    let store = InMemoryTofuPinStore::new().with_persistence(port.clone());
    (Arc::new(store), port)
}

async fn pinned() -> (Arc<InMemoryTofuPinStore>, Arc<Recording>) {
    let (store, port) = fixture();
    store
        .pin_first_contact(&peer(), &fp(1), &fp(1), SEED_NONCE)
        .await
        .unwrap();
    (store, port)
}

/// Detect a restart and approve it, leaving the store one write further on.
async fn approve_restart(store: &InMemoryTofuPinStore) -> RePinDecision {
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Approve {
                approval_id: [7; 16],
            },
        )
        .unwrap();
    store
        .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
        .await
}

#[tokio::test]
async fn first_contact_is_one_snapshot_anchored_at_generation_zero() {
    let (_store, port) = pinned().await;
    assert_eq!(port.count(), 1);
    let record = &port.last()[0];
    assert_eq!(record.pin.fingerprint, fp(1));
    assert_eq!(record.anchor, (fp(1), SEED_NONCE));
    assert_eq!(record.generation, 0);
    assert_eq!(record.pending_observed, None);
}

#[tokio::test]
async fn rotation_windows_are_never_persisted_but_a_promotion_is() {
    let (store, port) = pinned().await;
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    assert_eq!(port.count(), 1, "opening a window writes nothing");
    assert_eq!(store.records()[0].pin.fingerprint, fp(1));

    let retired = store.close_rotation_window(&peer());
    assert_eq!(retired, Some(fp(1)));
    assert_eq!(port.count(), 2, "a promotion is exactly one snapshot");
    let record = &port.last()[0];
    assert_eq!(record.pin.fingerprint, fp(2));
    assert_eq!(record.generation, 1);
    assert_eq!(record.anchor, (fp(1), SEED_NONCE), "the anchor is the seed");

    // The conditional close is the second promotion path.
    store.open_rotation_window(&peer(), &fp(3)).unwrap();
    assert_eq!(port.count(), 2);
    assert_eq!(store.close_rotation_window_if(&peer(), &fp(9)), None);
    assert_eq!(
        port.count(),
        2,
        "a refused conditional close writes nothing"
    );
    assert_eq!(store.close_rotation_window_if(&peer(), &fp(3)), Some(fp(2)));
    assert_eq!(port.count(), 3);
    assert_eq!(port.last()[0].generation, 2);

    // An aborted window (never promoted) is not a durable event either.
    store.open_rotation_window(&peer(), &fp(4)).unwrap();
    store.abort_rotation_window(&peer());
    assert_eq!(port.count(), 3);
    assert!(port.last().iter().all(|r| r.pin.fingerprint != fp(4)));
}

#[tokio::test]
async fn a_restart_detection_persists_the_invalidation_and_its_request() {
    let (store, port) = pinned().await;
    let prior = store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    assert_eq!(prior, Some(SEED_NONCE));
    assert_eq!(port.count(), 2);
    let record = &port.last()[0];
    assert_eq!(
        record.pin.invalidated,
        Some(Invalidated::SpiritRestarted {
            prior_boot_nonce: SEED_NONCE
        })
    );
    assert_eq!(record.pending_observed, Some(RESTARTED_NONCE));
    // No verdict is ever part of a record.
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Reject {
                reason: "no".into(),
            },
        )
        .unwrap();
    assert_eq!(port.count(), 2, "recording a verdict writes nothing");
}

#[tokio::test]
async fn the_direct_invalidation_persists_once() {
    let (store, port) = pinned().await;
    store
        .invalidate_for_restart(&peer(), SEED_NONCE)
        .await
        .unwrap();
    assert_eq!(port.count(), 2);
    let record = &port.last()[0];
    assert!(matches!(
        record.pin.invalidated,
        Some(Invalidated::SpiritRestarted { .. })
    ));
    assert_eq!(record.pending_observed, None);
}

#[tokio::test]
async fn an_approved_repin_persists_the_new_nonce_and_clears_the_request() {
    let (store, port) = pinned().await;
    let decision = approve_restart(&store).await;
    assert_eq!(
        decision,
        RePinDecision::AcceptedByOperator {
            approval_id: [7; 16]
        }
    );
    assert_eq!(port.count(), 3, "first contact, detection, approval");
    let record = &port.last()[0];
    assert_eq!(record.pin.boot_nonce, RESTARTED_NONCE);
    assert_eq!(record.pin.invalidated, None);
    assert_eq!(record.pin.repin_approval_id, Some([7; 16]));
    assert_eq!(record.pending_observed, None);
    assert_eq!(
        record.generation, 0,
        "a nonce-only re-pin keeps the generation"
    );
}

#[tokio::test]
async fn a_restored_invalidated_pin_stays_invalid_and_rearms_its_request_with_a_fresh_ttl() {
    let (store, port) = pinned().await;
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    let saved = port.last();

    let rebooted = InMemoryTofuPinStore::new().with_repin_ttl(Duration::from_secs(600));
    for record in saved {
        rebooted.restore(record).unwrap();
    }
    assert!(rebooted.verify_pinned_sync(&peer(), &fp(1)).is_err());
    assert_eq!(rebooted.find_active_pin_by_fingerprint(&fp(1)), None);
    let pending = rebooted.pending_repins();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].observed_boot_nonce, RESTARTED_NONCE);
    assert_eq!(pending[0].prior_boot_nonce, SEED_NONCE);
    assert_eq!(pending[0].pinned_fingerprint, fp(1));
    assert!(!pending[0].expired);
    assert_eq!(
        pending[0]
            .expires_at
            .duration_since(pending[0].detected_at)
            .unwrap(),
        Duration::from_secs(600)
    );

    // The request predates the sink: installing it journals the re-armed row.
    let seen: Arc<Mutex<Vec<RepinRequest>>> = Arc::default();
    let sink_seen = Arc::clone(&seen);
    let sink: RepinSink =
        Arc::new(move |request: &RepinRequest| sink_seen.lock().unwrap().push(request.clone()));
    assert!(rebooted.install_repin_sink(sink));
    assert_eq!(seen.lock().unwrap().len(), 1);

    // And the consent door can approve it.
    rebooted
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Approve {
                approval_id: [9; 16],
            },
        )
        .unwrap();
    assert!(matches!(
        rebooted
            .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
            .await,
        RePinDecision::AcceptedByOperator { .. }
    ));
    assert!(rebooted.verify_pinned_sync(&peer(), &fp(1)).is_ok());
}

#[tokio::test]
async fn restore_never_trusts_a_collision_or_a_duplicate() {
    let (store, port) = pinned().await;
    let record = port.last()[0].clone();
    assert!(store.restore(record.clone()).is_err(), "duplicate peer");
    let mut other = record;
    other.pin.peer = PeerId::new("peer-b");
    assert!(store.restore(other).is_err(), "fingerprint held by peer-a");
}

#[tokio::test]
async fn a_widening_write_failure_refuses_first_contact() {
    let (store, port) = fixture();
    port.fail.store(true, Ordering::SeqCst);
    let error = store
        .pin_first_contact(&peer(), &fp(1), &fp(1), SEED_NONCE)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("disk full"), "{error}");
    assert!(store.get_pin(&peer()).await.is_none());
    assert!(!store.is_degraded(), "a refused widening applied nothing");
}

#[tokio::test]
async fn a_widening_write_failure_leaves_the_pin_invalidated_and_the_request_pending() {
    let (store, port) = pinned().await;
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Approve {
                approval_id: [7; 16],
            },
        )
        .unwrap();
    port.fail.store(true, Ordering::SeqCst);
    let decision = store
        .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
        .await;
    assert!(matches!(decision, RePinDecision::PersistFailed { .. }));
    let pin = store.get_pin(&peer()).await.unwrap();
    assert!(pin.invalidated.is_some());
    assert_eq!(pin.boot_nonce, SEED_NONCE);
    assert_eq!(
        store.pending_repins().len(),
        1,
        "the request is still pending"
    );

    // The verdict was dropped: a fresh decision, on a healthy disk, applies.
    port.fail.store(false, Ordering::SeqCst);
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Approve {
                approval_id: [8; 16],
            },
        )
        .unwrap();
    assert!(matches!(
        store
            .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
            .await,
        RePinDecision::AcceptedByOperator { .. }
    ));
}

#[tokio::test]
async fn a_narrowing_write_failure_applies_and_degrades_until_a_write_succeeds() {
    let (store, port) = pinned().await;
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    port.fail.store(true, Ordering::SeqCst);
    assert_eq!(store.close_rotation_window(&peer()), Some(fp(1)));
    assert_eq!(store.get_pin_sync(&peer()).unwrap().fingerprint, fp(2));
    assert!(store.is_degraded());

    // A restart detection while degraded still applies in memory.
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    assert!(store.get_pin_sync(&peer()).unwrap().invalidated.is_some());
    assert!(store.is_degraded());

    // The next successful write clears it and carries the whole current state.
    port.fail.store(false, Ordering::SeqCst);
    store
        .pin_first_contact(&PeerId::new("peer-b"), &fp(5), &fp(5), SEED_NONCE)
        .await
        .unwrap();
    assert!(!store.is_degraded());
    let records = port.last();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].pin.fingerprint, fp(2));
    assert!(records[0].pin.invalidated.is_some());
}

#[tokio::test]
async fn a_consumed_request_is_dropped_durably_but_the_pin_stays_invalidated() {
    let (store, port) = pinned().await;
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Reject {
                reason: "no".into(),
            },
        )
        .unwrap();
    assert_eq!(port.last()[0].pending_observed, Some(RESTARTED_NONCE));
    let decision = store
        .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
        .await;
    assert!(matches!(decision, RePinDecision::RejectedByOperator { .. }));
    assert_eq!(
        port.count(),
        3,
        "first contact, detection, the consumed request"
    );
    let record = &port.last()[0];
    assert_eq!(record.pending_observed, None, "a reboot must not re-arm it");
    assert!(record.pin.invalidated.is_some());

    // A failing write while consuming only degrades the store.
    let (store, port) = pinned().await;
    store
        .invalidate_if_boot_nonce_differs(&peer(), RESTARTED_NONCE)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            RESTARTED_NONCE,
            RepinVerdict::Reject {
                reason: "no".into(),
            },
        )
        .unwrap();
    port.fail.store(true, Ordering::SeqCst);
    store
        .await_repin_consent(&peer(), &fp(1), RESTARTED_NONCE)
        .await;
    assert!(store.is_degraded());
    assert!(store.healthy_records().is_none());
}

#[tokio::test]
async fn a_store_without_a_port_behaves_as_before() {
    let store = InMemoryTofuPinStore::new();
    store
        .pin_first_contact(&peer(), &fp(1), &fp(1), SEED_NONCE)
        .await
        .unwrap();
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    assert_eq!(store.close_rotation_window(&peer()), Some(fp(1)));
    assert!(!store.is_degraded());
    assert_eq!(store.records()[0].generation, 1);
}
