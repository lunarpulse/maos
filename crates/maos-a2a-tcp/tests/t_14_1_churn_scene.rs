//! Story 14.1 AC1 — the watchable N=5 host-churn scene: **eviction →
//! two-surface detection → reconvergence** as ONE continuous observable
//! scene, on the same in-process real-socket real-mTLS substrate as the
//! N=30/N=100 drills (`t_11_3_scale_churn.rs`).
//!
//! # Where it lives, and why (AC1.5 — decided by arithmetic)
//!
//! This is a narrating test under `crates/maos-a2a-tcp/tests/` (uncharged by
//! `kloc-check`), NOT a new `xtask/src/demo_*.rs`: `xtask/src` has a
//! 35-line pre-14.1 headroom, `demo_j1.rs` is 1179 charged lines, and no
//! measured delta justifies opening that door. It is NOT `#[ignore]`-gated:
//! it runs in the normal test lane and is deliberately NOT a
//! `check-scale-churn` leg — AC1.4 forbids the scene from becoming a
//! gate-leg oracle for any NFR floor. AC2–AC4 (the real drills) are the
//! proof; this file is the observability artifact a human can look at.
//!
//! # The AC1.4 non-substitution guard
//!
//! Epic-13 retro §5 risk 1: *"ensure the small watchable scenes do not
//! substitute for fleet proof."* Forbidding the sentence is weaker than
//! making it unsayable, so the scene renders the SAME beat-set at BOTH
//! scales side by side, and every beat it did NOT execute renders **`ABSENT`
//! in its own column** — never a trailing footnote. The concrete failure
//! mode being closed: someone screenshots the five-host scene for a deck and
//! the footnote is outside the crop. Here the N=100 column says `ABSENT` in
//! every cell of this artifact, because this artifact did not measure N=100
//! — the N=100 evidence lives in the gate's legs, and the gate's marker
//! (`SCALE_CHURN_HOSTS_RECONCILED`) is what carries it.
//!
//! # Evidence states (AC1.3)
//!
//! Beats render the `EvidenceState` wire vocabulary from
//! `xtask/src/gate_common.rs` — `PROVEN_BLOCKING` for a beat this scene
//! actually executed and asserted, `ABSENT` for a beat it did not. The bare
//! product-claim word "PROVEN" never appears: an evidence state is what the
//! harness OBSERVED, distinct from BindingClass, and `ABSENT` never becomes
//! green.
//!
//! # The eviction is lifted, not invented (AC1.2)
//!
//! The primitive is 11.3's, buried in the 3-node consent drill
//! (`t_11_3_scale_churn.rs` at `run_consent_reachability_drill`, pre-14.1):
//! a REAL repoint-to-dead-port through the PUBLIC `set_peer_endpoint`,
//! confirmed by a REAL failed re-dial, followed by a REAL legit↔legit
//! reconvergence. 14.1 lifted it into `support::evict_and_confirm` — a
//! mesh-level operation usable at any N. The churn loop's drop-and-rebuild
//! is NOT an eviction and is never narrated as one here.
mod support;
use std::collections::BTreeSet;
use std::net::SocketAddr;

use maos_a2a_core::error::HandshakeFailureClass;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{A2AError, ChurnDrillReport};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

/// The scene scale: five hosts — small enough to read, real enough that
/// every beat is a live socket op.
const SCENE_N: usize = 5;

/// Wire spellings of the `EvidenceState` vocabulary (`gate_common.rs`) —
/// kept as literals because this crate cannot depend on `xtask`, and the
/// vocabulary is the epic-13 wire contract (`epic-13:200`), not a type.
const PROVEN_BLOCKING: &str = "PROVEN_BLOCKING";
const ABSENT: &str = "ABSENT";

/// One scene beat: what happened, and the evidence state it earned at each
/// rendered scale.
struct Beat {
    name: &'static str,
    observed: &'static str,
    /// The N=100 column: this scene did not execute the beat at the envelope
    /// scale, so it renders `ABSENT` — in its own cell, never a footnote.
    envelope: &'static str,
}

#[tokio::test]
async fn t_14_1_churn_watchable_scene_n5() {
    assert_fd_headroom(fd_headroom_for(SCENE_N));
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-scene");
    let retry = no_retry();

    // Layout: 0 = hub (eviction witness), 1–2 = legit, 3 = pin-spoof
    // (handshake surface), 4 = consent bypass (router-NACK surface).
    let pin_spoof = 3;
    let consent = 4;
    let mut names: Vec<String> = (0..SCENE_N).map(host_name).collect();
    names[pin_spoof] = "scene_pin_spoof".to_string();
    names[consent] = "scene_consent_bypass".to_string();

    let expected: Vec<Leaf> = (0..SCENE_N).map(|_| valid_leaf(&ca, &clock)).collect();
    // `serving` starts as a COPY of `expected` (each node serves the identity
    // its peers pinned); only the planted pin-spoof deviates (a DIFFERENT
    // valid leaf). The consent adversary keeps serving == expected: a VALID
    // identity, denied on intent, not on identity.
    let mut serving: Vec<Leaf> = expected.clone();
    serving[pin_spoof] = valid_leaf(&ca, &clock);

    let serving_refs: Vec<&Leaf> = serving.iter().collect();
    let expected_refs: Vec<&Leaf> = expected.iter().collect();
    let mesh = build_mesh_with_planted(
        &clock,
        &ca,
        &names,
        &serving_refs,
        &expected_refs,
        &[
            (pin_spoof, PlantKind::TofuPinSpoofing),
            (consent, PlantKind::AdrLevel012ConsentBypass),
        ],
        retry,
    )
    .await;

    // ── Beat 1: the mesh stands up and reconciles (distinct fingerprints AND
    // distinct bound addrs — never the literal count alone).
    let fingerprints: BTreeSet<String> = mesh.iter().map(|m| m.fingerprint.wire()).collect();
    let addrs: BTreeSet<SocketAddr> = mesh.iter().map(|m| m.addr).collect();
    let report = ChurnDrillReport::from_real_events(
        "scene-n5-mesh-reconciled",
        fingerprints.clone(),
        addrs.clone(),
        vec![],
        0,
        None,
    );
    assert_eq!(
        report.distinct_host_count(),
        SCENE_N,
        "beat 1: the five-host scene must reconcile to five DISTINCT identities"
    );
    let beat_mesh = PROVEN_BLOCKING;

    // ── Beat 2: a REAL eviction — the hub repoints its view of host 1 to a
    // dead port through the PUBLIC `set_peer_endpoint`, and the eviction is
    // CONFIRMED by a real failed re-dial. No loop counter stands in for it.
    let eviction = evict_and_confirm(&mesh, 1, 0, 7).await;
    assert!(
        eviction.redial_result.is_err(),
        "beat 2: the eviction must be confirmed by a REAL failed re-dial, got {:?}",
        eviction.redial_result
    );
    let beat_eviction = PROVEN_BLOCKING;

    // ── Beat 3: two-surface detection, one beat per surface. Handshake
    // first: the hub dials the pin-spoof it pinned by claimed fingerprint.
    let f_hs = make_frame(&names[0], &names[pin_spoof], IntentClass::Readonly, 11);
    let hs = mesh[0]
        .transport
        .route_outbound(f_hs, &HostId(names[pin_spoof].clone()))
        .await;
    assert!(
        matches!(
            hs,
            Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::PinMismatch,
                ..
            })
        ),
        "beat 3 (handshake surface): pin-spoof must fail with class PinMismatch, got {hs:?}"
    );
    // Router NACK: the consent adversary probes a legit peer readonly (real
    // reachability), then escalates and is denied at the receiver.
    let f_ro = make_frame(&names[consent], &names[2], IntentClass::Readonly, 12);
    let ro = mesh[consent]
        .transport
        .route_outbound(f_ro, &HostId(names[2].clone()))
        .await;
    assert!(
        ro.is_ok(),
        "beat 3 (router surface): readonly probe must succeed before escalation: {ro:?}"
    );
    let f_esc = make_frame(&names[consent], &names[2], IntentClass::Standard, 13);
    let esc = mesh[consent]
        .transport
        .route_outbound(f_esc, &HostId(names[2].clone()))
        .await;
    assert!(
        matches!(esc, Err(A2AError::IntentDeniedAtPeer { .. })),
        "beat 3 (router surface): escalation must be denied with IntentDeniedAtPeer, got {esc:?}"
    );
    let beat_detection = PROVEN_BLOCKING;

    // ── Beat 4: REAL reconvergence — the eviction heals (the hub re-pins the
    // victim's REAL bound address, the same public primitive) and the full
    // hub-and-spoke sweep over the legit fleet passes zero-drop.
    mesh[0]
        .transport
        .set_peer_endpoint(&HostId(names[1].clone()), format!("tls://{}", mesh[1].addr));
    let legit: &[MeshNode] = &mesh[..pin_spoof.min(3)]; // hosts 0..3 are the legit fleet
    let pairs: Vec<(usize, usize)> = (1..legit.len())
        .flat_map(|i| [(0usize, i), (i, 0usize)])
        .collect();
    let results = concurrent_dial_pairs(legit, &pairs, 21, IntentClass::Readonly).await;
    let failed: Vec<_> = results.iter().filter(|(_, _, r)| r.is_err()).collect();
    assert!(
        failed.is_empty(),
        "beat 4: reconvergence sweep must be zero-drop over the live legit fleet (failed={failed:?})"
    );
    let beat_reconvergence = PROVEN_BLOCKING;

    // ── The beat table (AC1.4): the SAME beat-set at BOTH scales, side by
    // side. This artifact executed N=5 only — the N=100 column renders
    // `ABSENT` per beat, in its own cell. It is an observability artifact
    // and NEVER a gate-leg oracle for any NFR floor (AC1.4); the N=100
    // evidence is the gate's marked legs.
    let beats = [
        Beat {
            name: "mesh stood up; identity reconciled (fingerprints x addrs)",
            observed: beat_mesh,
            envelope: ABSENT,
        },
        Beat {
            name: "eviction: victim repointed to dead port; re-dial FAILED (confirmed)",
            observed: beat_eviction,
            envelope: ABSENT,
        },
        Beat {
            name: "two-surface detection: handshake PinMismatch + router NACK",
            observed: beat_detection,
            envelope: ABSENT,
        },
        Beat {
            name: "reconvergence: eviction healed; legit hub sweep zero-drop",
            observed: beat_reconvergence,
            envelope: ABSENT,
        },
    ];
    eprintln!("\n=== Story 14.1 watchable churn scene (AC1) ===");
    eprintln!("{:=<72}", "");
    eprintln!(
        "| {:<58} | {:<16} | {:<16} |",
        "beat (asserted, not narrated)", "N=5 (observed)", "N=100"
    );
    eprintln!("|{:-<60}|{:-<18}|{:-<18}|", "", "", "");
    for beat in &beats {
        eprintln!(
            "| {:<58} | {:<16} | {:<16} |",
            beat.name, beat.observed, beat.envelope
        );
    }
    eprintln!("{:=<72}", "");
    eprintln!(
        "EvidenceState wire vocabulary (gate_common.rs): {} = executed and asserted here; \
         {} = not executed by THIS artifact (never a footnote; never a gate-leg oracle — \
         AC1.4). The N=100 proof is check-scale-churn's marked legs.",
        PROVEN_BLOCKING, ABSENT
    );
}
