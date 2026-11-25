//! Story 23-7 — the durable join (`promoted`, D-23-7-07 A), the manifest-version floor
//! (D-23-7-09 A) and the narrowing write path (D-23-7-08 A) at the pin-store seam, over a
//! recording port.

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{
    InMemoryTofuPinStore, PinPersistence, PinRecord, RePinDecision, RepinVerdict, TofuPinStore,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Port {
    /// `(records, floor, narrowing)` per write.
    writes: Mutex<Vec<(Vec<PinRecord>, u64, bool)>>,
    floor: AtomicU64,
}

impl Port {
    fn last(&self) -> (Vec<PinRecord>, u64, bool) {
        self.writes.lock().unwrap().last().cloned().unwrap()
    }
    fn count(&self) -> usize {
        self.writes.lock().unwrap().len()
    }
}

impl PinPersistence for Port {
    fn load(&self) -> Vec<PinRecord> {
        Vec::new()
    }
    fn load_floor(&self) -> u64 {
        self.floor.load(Ordering::SeqCst)
    }
    fn store(&self, records: &[PinRecord], floor: u64) -> Result<(), String> {
        self.writes
            .lock()
            .unwrap()
            .push((records.to_vec(), floor, false));
        Ok(())
    }
    fn store_narrowing(&self, records: &[PinRecord], floor: u64) -> Result<(), String> {
        self.writes
            .lock()
            .unwrap()
            .push((records.to_vec(), floor, true));
        Ok(())
    }
    fn fence(&self) -> Result<(), String> {
        Ok(())
    }
}

fn fp(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(&[seed; 8])
}

fn peer() -> PeerId {
    PeerId::new("peer-a")
}

async fn pinned() -> (Arc<InMemoryTofuPinStore>, Arc<Port>) {
    let port = Arc::new(Port::default());
    let store = Arc::new(InMemoryTofuPinStore::new().with_persistence(port.clone()));
    store
        .pin_first_contact(&peer(), &fp(1), &fp(1), 7)
        .await
        .unwrap();
    (store, port)
}

#[tokio::test]
async fn both_promotion_closes_mark_the_pin_promoted_and_write_narrowing() {
    let (store, port) = pinned().await;
    assert!(
        !store.is_promoted(&peer()),
        "a first contact is not a promotion"
    );
    assert!(!port.last().0[0].promoted);

    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    assert!(
        !store.is_promoted(&peer()),
        "an open window is not a promotion"
    );
    let before = port.count();
    store.close_rotation_window_if(&peer(), &fp(2)).unwrap();
    assert!(store.is_promoted(&peer()));
    assert_eq!(port.count(), before + 1);
    let (records, _, narrowing) = port.last();
    assert!(
        narrowing,
        "a promotion narrows trust: it takes the fenced write path"
    );
    assert!(records[0].promoted);
    assert_eq!(records[0].generation, 1);

    // The unconditional close marks it too.
    store.open_rotation_window(&peer(), &fp(3)).unwrap();
    store.close_rotation_window(&peer()).unwrap();
    assert!(port.last().0[0].promoted);
    assert_eq!(port.last().0[0].generation, 2);
}

#[tokio::test]
async fn an_abort_is_not_a_promotion() {
    let (store, port) = pinned().await;
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    store.abort_rotation_window(&peer());
    assert!(!store.is_promoted(&peer()));
    assert!(!port.last().0[0].promoted);
}

#[tokio::test]
async fn a_restored_record_keeps_its_promotion() {
    let (store, port) = pinned().await;
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    store.close_rotation_window_if(&peer(), &fp(2)).unwrap();
    let records = port.last().0;

    let fresh = InMemoryTofuPinStore::new();
    for record in records {
        fresh.restore(record).unwrap();
    }
    assert!(fresh.is_promoted(&peer()));
}

#[tokio::test]
async fn a_repair_to_another_fingerprint_clears_the_promotion_and_a_nonce_repin_keeps_it() {
    let (store, port) = pinned().await;
    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    store.close_rotation_window_if(&peer(), &fp(2)).unwrap();

    // A restart detection + consented re-pin on the SAME fingerprint (new nonce): still promoted.
    store
        .invalidate_if_boot_nonce_differs(&peer(), 8)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            8,
            RepinVerdict::Approve {
                approval_id: [1; 16],
            },
        )
        .unwrap();
    assert!(matches!(
        store.await_repin_consent(&peer(), &fp(2), 8).await,
        RePinDecision::AcceptedByOperator { .. }
    ));
    assert!(store.is_promoted(&peer()));
    assert!(port.last().0[0].promoted);

    // A consented re-pin to a DIFFERENT fingerprint is a re-pair: not a promotion.
    store
        .invalidate_if_boot_nonce_differs(&peer(), 9)
        .await
        .unwrap();
    store
        .record_repin_decision(
            &peer(),
            9,
            RepinVerdict::Approve {
                approval_id: [2; 16],
            },
        )
        .unwrap();
    assert!(matches!(
        store.await_repin_consent(&peer(), &fp(9), 9).await,
        RePinDecision::AcceptedByOperator { .. }
    ));
    assert!(!store.is_promoted(&peer()));
    assert!(!port.last().0[0].promoted);
}

#[tokio::test]
async fn the_manifest_floor_is_monotonic_durable_and_loaded_from_the_port() {
    let port = Arc::new(Port::default());
    port.floor.store(4, Ordering::SeqCst);
    let store = InMemoryTofuPinStore::new().with_persistence(port.clone());
    assert_eq!(
        store.manifest_floor(),
        4,
        "the durable floor survives a restart"
    );

    store.raise_manifest_floor(6);
    assert_eq!(store.manifest_floor(), 6);
    let (_, floor, narrowing) = port.last();
    assert_eq!(
        (floor, narrowing),
        (6, true),
        "a raise is a narrowing write carrying the floor"
    );

    let writes = port.count();
    store.raise_manifest_floor(5);
    store.raise_manifest_floor(6);
    assert_eq!(store.manifest_floor(), 6, "never lowered");
    assert_eq!(port.count(), writes, "a no-op raise writes nothing");

    // The floor rides every other write too.
    store
        .pin_first_contact(&peer(), &fp(1), &fp(1), 7)
        .await
        .unwrap();
    assert_eq!(port.last().1, 6);
}

#[test]
fn without_a_port_the_floor_is_still_enforced_in_memory() {
    let store = InMemoryTofuPinStore::new();
    store.raise_manifest_floor(3);
    assert_eq!(store.manifest_floor(), 3);
}

/// A port that records, in one ordered log, every fence and write together with the pin a
/// lock-free verifier could read at that instant.
#[derive(Default)]
struct OrderPort {
    store: std::sync::OnceLock<Arc<InMemoryTofuPinStore>>,
    events: Mutex<Vec<String>>,
}

impl OrderPort {
    fn observed(&self) -> String {
        self.store
            .get()
            .and_then(|store| store.get_pin_sync(&peer()))
            .map_or_else(|| "none".into(), |pin| pin.fingerprint.wire())
    }
}

impl PinPersistence for OrderPort {
    fn load(&self) -> Vec<PinRecord> {
        Vec::new()
    }
    fn store(&self, _: &[PinRecord], _: u64) -> Result<(), String> {
        Ok(())
    }
    fn store_narrowing(&self, _: &[PinRecord], _: u64) -> Result<(), String> {
        let seen = self.observed();
        self.events.lock().unwrap().push(format!("write:{seen}"));
        Ok(())
    }
    fn fence(&self) -> Result<(), String> {
        let seen = self.observed();
        self.events.lock().unwrap().push(format!("fence:{seen}"));
        Ok(())
    }
}

// Review A2 — the fence is durable BEFORE the retirement is observable: at the instant of the
// fence a verifier still reads the OLD pin, so no crash can follow an observable retirement
// without the fence in place. Planted red: dropping `fence_before_retirement` leaves only the
// write (which a verifier already sees as retired).
#[tokio::test]
async fn both_promotion_closes_fence_before_the_retirement_is_observable() {
    let port = Arc::new(OrderPort::default());
    let store = Arc::new(InMemoryTofuPinStore::new().with_persistence(port.clone()));
    assert!(port.store.set(store.clone()).is_ok());
    store
        .pin_first_contact(&peer(), &fp(1), &fp(1), 7)
        .await
        .unwrap();
    let (old, newer, newest) = (fp(1).wire(), fp(2).wire(), fp(3).wire());

    store.open_rotation_window(&peer(), &fp(2)).unwrap();
    port.events.lock().unwrap().clear();
    store.close_rotation_window_if(&peer(), &fp(2)).unwrap();
    assert_eq!(
        *port.events.lock().unwrap(),
        vec![format!("fence:{old}"), format!("write:{newer}")]
    );

    store.open_rotation_window(&peer(), &fp(3)).unwrap();
    port.events.lock().unwrap().clear();
    store.close_rotation_window(&peer()).unwrap();
    assert_eq!(
        *port.events.lock().unwrap(),
        vec![format!("fence:{newer}"), format!("write:{newest}")]
    );
}
