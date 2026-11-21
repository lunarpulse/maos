//! Story 21-3 / AC1.c — the D-21-3-02 anchor rule at the seeding seam
//! (`TcpA2AConfig::build_pin_store`), one test per branch. The durable record
//! is hand-built; nothing is dialed.

use maos_a2a_core::tofu::{Invalidated, PinPersistence, PinRecord, TofuPin};
use maos_a2a_core::{InMemoryTofuPinStore, PeerCertFingerprint, PeerId};
use maos_a2a_tcp::{PinnedFingerprint, TcpA2AConfig, TcpTransportError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const NONCE: u64 = 10;

/// Serves a fixed record set and remembers every snapshot it was handed.
struct Port {
    loaded: Vec<PinRecord>,
    stored: Mutex<Vec<Vec<PinRecord>>>,
}

impl Port {
    fn with(loaded: Vec<PinRecord>) -> Arc<Self> {
        Arc::new(Self {
            loaded,
            stored: Mutex::new(Vec::new()),
        })
    }
}

impl PinPersistence for Port {
    fn load(&self) -> Vec<PinRecord> {
        self.loaded.clone()
    }

    fn store(&self, records: &[PinRecord]) -> Result<(), String> {
        self.stored.lock().unwrap().push(records.to_vec());
        Ok(())
    }
}

fn fp(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(&[seed; 8])
}

fn config(pins: &[(&str, u8, u64)]) -> TcpA2AConfig {
    TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        own_cert_chain: "/nonexistent/chain.pem".into(),
        own_private_key: "/nonexistent/key.pem".into(),
        peer_pins: pins
            .iter()
            .map(|(peer, seed, nonce)| PinnedFingerprint {
                peer_id: PeerId::new(*peer),
                fingerprint: fp(*seed),
                boot_nonce: *nonce,
            })
            .collect(),
        handshake_timeout: Duration::from_secs(30),
        ca_roots: None,
    }
}

/// A durable record for `peer`: seeded from `(anchor_seed, anchor_nonce)`,
/// currently pinned at `fingerprint_seed` with `boot_nonce`.
fn record(
    peer: &str,
    (anchor_seed, anchor_nonce): (u8, u64),
    fingerprint_seed: u8,
    boot_nonce: u64,
    generation: u64,
) -> PinRecord {
    PinRecord {
        pin: TofuPin {
            peer: PeerId::new(peer),
            fingerprint: fp(fingerprint_seed),
            boot_nonce,
            pinned_at_ns: 0,
            invalidated: None,
            repin_approval_id: None,
        },
        anchor: (fp(anchor_seed), anchor_nonce),
        generation,
        pending_observed: None,
    }
}

fn last_snapshot(store: &InMemoryTofuPinStore, port: &Port) -> Vec<PinRecord> {
    store.flush().unwrap();
    let stored = port.stored.lock().unwrap();
    assert_eq!(
        stored.len(),
        1,
        "the reconciled set is written exactly once"
    );
    stored[0].clone()
}

#[tokio::test]
async fn no_record_seeds_from_config_at_generation_zero() {
    let port = Port::with(Vec::new());
    let store = config(&[("peer-a", 1, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!((pin.fingerprint, pin.boot_nonce), (fp(1), NONCE));
    let written = last_snapshot(&store, &port);
    assert_eq!(written.len(), 1);
    assert_eq!(
        (written[0].generation, written[0].anchor.clone()),
        (0, (fp(1), NONCE))
    );
}

#[tokio::test]
async fn config_equal_to_the_anchor_restores_the_record() {
    let mut saved = record("peer-a", (1, NONCE), 1, 99, 0);
    saved.pin.invalidated = Some(Invalidated::SpiritRestarted {
        prior_boot_nonce: 99,
    });
    saved.pin.boot_nonce = 99;
    saved.pending_observed = Some(100);
    let port = Port::with(vec![saved]);
    let store = config(&[("peer-a", 1, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!(
        pin.boot_nonce, 99,
        "the consented nonce, not the config seed"
    );
    assert!(
        pin.invalidated.is_some(),
        "invalidation survives the restart"
    );
    assert_eq!(store.pending_repins().len(), 1);
    assert_eq!(last_snapshot(&store, &port)[0].pending_observed, Some(100));
}

#[tokio::test]
async fn a_promotion_not_carried_into_config_is_a_typed_boot_refusal() {
    let port = Port::with(vec![record("peer-a", (1, NONCE), 2, NONCE, 1)]);
    let error = config(&[("peer-a", 1, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .err()
        .expect("boot must be refused");
    assert_eq!(
        error,
        TcpTransportError::PinPromotionNotCarried {
            peer: "peer-a".into(),
            configured: fp(1).wire(),
            promoted: fp(2).wire(),
        }
    );
    let text = error.to_string();
    assert!(
        text.contains(&fp(1).wire()) && text.contains(&fp(2).wire()),
        "{text}"
    );
    assert!(
        port.stored.lock().unwrap().is_empty(),
        "a refused boot writes nothing"
    );
}

#[tokio::test]
async fn a_promotion_carried_into_config_restores_with_its_generation() {
    // The operator moved config to the promoted fingerprint: the record's
    // state is kept and only the anchor moves.
    let port = Port::with(vec![record("peer-a", (1, NONCE), 2, NONCE, 1)]);
    let store = config(&[("peer-a", 2, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!(pin.fingerprint, fp(2));
    let written = last_snapshot(&store, &port);
    assert_eq!(
        written[0].generation, 1,
        "same fingerprint as the record: kept"
    );
    assert_eq!(written[0].anchor, (fp(2), NONCE), "the anchor is re-seeded");
}

#[tokio::test]
async fn a_repaired_nonce_wins_over_an_invalidated_record() {
    let mut saved = record("peer-a", (1, NONCE), 1, NONCE, 4);
    saved.pin.invalidated = Some(Invalidated::SpiritRestarted {
        prior_boot_nonce: NONCE,
    });
    saved.pending_observed = Some(11);
    let port = Port::with(vec![saved]);
    let store = config(&[("peer-a", 1, 12)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!((pin.boot_nonce, pin.invalidated), (12, None));
    assert!(store.pending_repins().is_empty());
    let written = last_snapshot(&store, &port);
    assert_eq!(
        written[0].generation, 4,
        "nonce-only change keeps the generation"
    );
    assert_eq!(written[0].anchor, (fp(1), 12));
}

#[tokio::test]
async fn a_repair_to_a_different_fingerprint_bumps_the_generation() {
    let port = Port::with(vec![record("peer-a", (1, NONCE), 1, NONCE, 4)]);
    let store = config(&[("peer-a", 3, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let written = last_snapshot(&store, &port);
    assert_eq!(written[0].pin.fingerprint, fp(3));
    assert_eq!(written[0].generation, 5);
}

#[tokio::test]
async fn a_nonce_only_config_edit_cannot_resurrect_the_retired_fingerprint() {
    // The record was promoted 1 -> 2; config still names 1 but with a new nonce.
    let port = Port::with(vec![record("peer-a", (1, NONCE), 2, NONCE, 1)]);
    let error = config(&[("peer-a", 1, 12)])
        .build_pin_store(Some(port))
        .await
        .err()
        .expect("boot must be refused");
    assert!(matches!(
        error,
        TcpTransportError::PinPromotionNotCarried { .. }
    ));
}

#[tokio::test]
async fn a_carried_promotion_keeps_the_consented_nonce_and_invalidation() {
    let mut saved = record("peer-a", (1, NONCE), 2, 99, 1);
    saved.pin.invalidated = Some(Invalidated::SpiritRestarted {
        prior_boot_nonce: 99,
    });
    saved.pending_observed = Some(100);
    let port = Port::with(vec![saved]);
    // Config now names the promoted fingerprint (with its own seed nonce).
    let store = config(&[("peer-a", 2, 12)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!((pin.fingerprint, pin.boot_nonce), (fp(2), 99));
    assert!(pin.invalidated.is_some());
    assert_eq!(store.pending_repins().len(), 1);
    let written = last_snapshot(&store, &port);
    assert_eq!(
        (written[0].generation, written[0].anchor.clone()),
        (1, (fp(2), 12))
    );
}

#[tokio::test]
async fn the_file_never_adds_a_fingerprint_the_config_does_not_name() {
    let port = Port::with(vec![
        record("peer-a", (1, NONCE), 1, NONCE, 0),
        record("peer-gone", (7, NONCE), 7, NONCE, 0),
    ]);
    let store = config(&[("peer-a", 1, NONCE), ("peer-new", 5, NONCE)])
        .build_pin_store(Some(port.clone()))
        .await
        .unwrap();
    assert!(store.get_pin_sync(&PeerId::new("peer-gone")).is_none());
    assert_eq!(store.find_active_pin_by_fingerprint(&fp(7)), None);
    let written = last_snapshot(&store, &port);
    let mut peers: Vec<_> = written.iter().map(|r| r.pin.peer.as_str()).collect();
    peers.sort_unstable();
    assert_eq!(peers, ["peer-a", "peer-new"], "the omitted peer is dropped");
    assert!(written
        .iter()
        .all(|r| r.pin.fingerprint == fp(1) || r.pin.fingerprint == fp(5)));
}

#[tokio::test]
async fn without_a_port_the_config_seeds_exactly_as_before() {
    let store = config(&[("peer-a", 1, NONCE)])
        .build_pin_store(None)
        .await
        .unwrap();
    let pin = store.get_pin_sync(&PeerId::new("peer-a")).unwrap();
    assert_eq!((pin.fingerprint, pin.boot_nonce), (fp(1), NONCE));
    assert!(!store.is_degraded());
}
