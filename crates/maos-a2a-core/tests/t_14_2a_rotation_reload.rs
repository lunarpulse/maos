//! Story 14-2a — the two primitives the production peer-cert reload needs, and
//! the AC2.2 invariant they exist to keep.
//!
//! **The invariant, by name:** for every peer `p` at every observable instant,
//! `peers[p].cert_fingerprint ∈ {pins[p].fingerprint, rotation_next[p]}`.
//!
//! Sites 5 and 6 (`router.rs:991-994` `prepare_outbound` → `A2AError::PinMismatch`,
//! `router.rs:1309-1315` `handle_intake_inner` → NACK `CODE_PIN_MISMATCH_NOT_PINNED`)
//! read plane A (`A2ARouterCore.peers[p].cert_fingerprint`) and plane B
//! (`InMemoryTofuPinStore`) with no shared lock and no snapshot. The reload is
//! therefore made observably atomic by ORDER rather than by a new lock: plane B's
//! rotation window is opened FIRST, so the accepted set is a SUPERSET
//! `{old, next}` for the whole transition, and plane A's single-word move can
//! never be observed against a set that does not contain it. The rollback runs
//! the same order in reverse (plane A restored BEFORE the window is aborted).
//!
//! Every test below asserts the invariant through the SAME call the production
//! sites use — `verify_pinned` — never through a re-implementation of it.

use std::sync::Arc;

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::A2ARouterCore;
use maos_a2a_core::tofu::{InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_core::{A2APeerConfig, A2AProfile, ConsentAllowlists};
use maos_spirit_abi::identity::HostId;

fn fingerprint(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", format!("{seed:02x}").repeat(32)))
        .expect("fixture fingerprint is well formed")
}

fn peer_config(peer: &str, fingerprint: PeerCertFingerprint) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(peer),
        endpoint: format!("tls://{peer}:0"),
        cert_fingerprint: fingerprint,
        profile: A2AProfile::CrossHost,
        allowlists: ConsentAllowlists {
            send_allowlist: Vec::new(),
            accept_allowlist: Vec::new(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

/// Plane A's value, verified against plane B exactly as sites 5/6 do it.
async fn plane_a_is_accepted_by_plane_b(
    core: &A2ARouterCore,
    pins: &InMemoryTofuPinStore,
    peer: &str,
) -> bool {
    let cfg = core
        .lookup_peer(&HostId(peer.to_string()))
        .expect("peer is declared in plane A");
    pins.verify_pinned(&cfg.peer_id, &cfg.cert_fingerprint)
        .await
        .is_ok()
}

async fn pinned_core(
    peer: &str,
    current: &PeerCertFingerprint,
) -> (Arc<A2ARouterCore>, Arc<InMemoryTofuPinStore>) {
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config(peer, current.clone())], tofu)
            .expect("single peer config is valid"),
    );
    pins.pin_first_contact(&PeerId::new(peer), current, current, 7)
        .await
        .expect("first contact pins the operator-declared fingerprint");
    (core, pins)
}

#[tokio::test]
async fn abort_discards_next_and_leaves_the_current_pin_serving() {
    let old = fingerprint(0x11);
    let next = fingerprint(0x22);
    let (_core, pins) = pinned_core("host-b", &old).await;

    pins.open_rotation_window(&PeerId::new("host-b"), &next)
        .expect("window opens on a live pin");
    assert_eq!(
        pins.rotation_next(&PeerId::new("host-b")),
        Some(next.clone())
    );

    let discarded = pins.abort_rotation_window(&PeerId::new("host-b"));

    assert_eq!(
        discarded,
        Some(next.clone()),
        "abort returns the discarded generation so the caller can audit what it refused"
    );
    assert_eq!(
        pins.rotation_next(&PeerId::new("host-b")),
        None,
        "abort leaves no widened trust set behind"
    );
    assert_eq!(
        pins.get_pin_sync(&PeerId::new("host-b"))
            .expect("pin survives an abort")
            .fingerprint,
        old,
        "abort is DISCARD-and-keep; close is promote-and-retire (14-2 AC2.1.a is not reopened)"
    );
    assert!(
        pins.verify_pinned(&PeerId::new("host-b"), &next)
            .await
            .is_err(),
        "the aborted generation is no longer accepted"
    );
    assert!(
        pins.abort_rotation_window(&PeerId::new("host-b")).is_none(),
        "abort is idempotent: a second abort discards nothing"
    );
}

#[tokio::test]
async fn plane_a_mutator_reports_the_fingerprint_it_replaced() {
    let old = fingerprint(0x33);
    let next = fingerprint(0x44);
    let (core, _pins) = pinned_core("host-b", &old).await;

    let replaced = core.set_peer_cert_fingerprint(&HostId("host-b".into()), next.clone());

    assert_eq!(
        replaced,
        Some(old),
        "the reload needs the retired value for its audit row and its rollback"
    );
    assert_eq!(
        core.lookup_peer(&HostId("host-b".into()))
            .expect("peer stays declared")
            .cert_fingerprint,
        next
    );
    assert_eq!(
        core.set_peer_cert_fingerprint(&HostId("ghost".into()), fingerprint(0x55)),
        None,
        "a peer plane A never declared is reported, never inserted"
    );
    assert!(
        core.lookup_peer(&HostId("ghost".into())).is_err(),
        "the mutator must not create a peer the operator never declared"
    );
}

#[tokio::test]
async fn open_then_move_keeps_the_invariant_at_every_observable_instant() {
    let old = fingerprint(0x66);
    let next = fingerprint(0x77);
    let (core, pins) = pinned_core("host-b", &old).await;
    let peer = PeerId::new("host-b");

    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "precondition: the mesh is coherent before the reload"
    );

    // Phase 1 — plane B first: the accepted set becomes {old, next}.
    pins.open_rotation_window(&peer, &next)
        .expect("window opens");
    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "plane A still holds `old`, which is still the serving pin"
    );

    // Phase 2 — plane A second, into a set that already contains `next`.
    core.set_peer_cert_fingerprint(&HostId("host-b".into()), next.clone());
    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "plane A holds `next`, which the open window pins equally (§7.2.1.a overlap)"
    );

    // Terminal — promote-and-retire.
    assert_eq!(
        pins.close_rotation_window(&peer),
        Some(old.clone()),
        "close retires the old generation and returns it"
    );
    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "after promotion plane A holds the only pinned generation"
    );
    assert!(
        pins.verify_pinned(&peer, &old).await.is_err(),
        "the retired generation is refused once the grace has closed"
    );
}

#[tokio::test]
async fn rollback_restores_plane_a_before_aborting_the_window() {
    let old = fingerprint(0x88);
    let next = fingerprint(0x99);
    let (core, pins) = pinned_core("host-b", &old).await;
    let peer = PeerId::new("host-b");

    pins.open_rotation_window(&peer, &next)
        .expect("window opens");
    core.set_peer_cert_fingerprint(&HostId("host-b".into()), next.clone());

    // The ONLY order that keeps the invariant: plane A back to `old` FIRST,
    // then the window closes. Aborting first would leave plane A holding a
    // fingerprint plane B no longer accepts — the torn state sites 5/6 report
    // as PinMismatch on every frame in both directions.
    core.set_peer_cert_fingerprint(&HostId("host-b".into()), old.clone());
    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "plane A is back on the serving pin while the window is still open"
    );
    pins.abort_rotation_window(&peer);
    assert!(
        plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "a rolled-back reload leaves the mesh exactly as it found it"
    );

    // Proven-red vector for the ordering claim: abort BEFORE restoring plane A
    // is observably torn, and this asserts that it is — so the ordering above
    // is load-bearing rather than decorative.
    pins.open_rotation_window(&peer, &next)
        .expect("window reopens");
    core.set_peer_cert_fingerprint(&HostId("host-b".into()), next.clone());
    pins.abort_rotation_window(&peer);
    assert!(
        !plane_a_is_accepted_by_plane_b(&core, &pins, "host-b").await,
        "aborting before restoring plane A MUST tear — this is why the order is fixed"
    );
}

/// §A6 close pass (F6) — the reachable half: a conditional close transitions
/// ONLY the exact window generation it was armed for.
///
/// A stale closer (a grace timer whose window was already superseded) must
/// neither promote a foreign generation nor close the window a newer
/// generation legitimately owns; only the owner's generation promotes and
/// retires. (The companion fail-closed half — clearing the window when the
/// promotion target has vanished — is unreachable through the public API:
/// `open_rotation_window` requires a live pin and no public mutator ever
/// removes one, so that contract is pinned by a module-private unit test in
/// `tofu.rs` rather than manufactured here.)
#[tokio::test]
async fn a_conditional_close_promotes_only_the_generation_it_opened() {
    let old = fingerprint(0xaa);
    let next = fingerprint(0xbb);
    let foreign = fingerprint(0xcc);
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let peer = PeerId::new("host-b");
    pins.pin_first_contact(&peer, &old, &old, 7)
        .await
        .expect("first contact pins");
    pins.open_rotation_window(&peer, &next)
        .expect("window opens");

    // A closer armed for a DIFFERENT generation (a superseded window's timer)
    // must not fire: no promotion, no close, the live window stays intact.
    assert_eq!(
        pins.close_rotation_window_if(&peer, &foreign),
        None,
        "a closer may transition only the exact window instance it owns"
    );
    assert_eq!(
        pins.rotation_next(&peer),
        Some(next.clone()),
        "the refusing close must NOT clear the window its owner still holds"
    );
    assert_eq!(
        pins.get_pin_sync(&peer)
            .expect("pin survives a refused close")
            .fingerprint,
        old.clone(),
        "and it must promote nothing"
    );

    // The owner's close, for the exact generation, promotes-and-retires.
    let retired = pins
        .close_rotation_window_if(&peer, &next)
        .expect("the exact generation is closable by its owner");
    assert_eq!(
        retired, old,
        "the retired fingerprint is the OLD serving pin"
    );
    assert_eq!(pins.rotation_next(&peer), None, "the window is closed");
    assert_eq!(
        pins.get_pin_sync(&peer)
            .expect("pin survives a close")
            .fingerprint,
        next,
        "close is promote-and-retire: the store ends holding NEW"
    );
    assert!(
        pins.verify_pinned(&peer, &next).await.is_ok(),
        "the promoted generation is the accepted identity, as sites 5/6 observe"
    );
    assert!(
        pins.verify_pinned(&peer, &old).await.is_err(),
        "the retired generation is no longer accepted once the window is closed"
    );
    assert!(
        pins.close_rotation_window_if(&peer, &next).is_none(),
        "a second close of the same generation finds no window"
    );
}
