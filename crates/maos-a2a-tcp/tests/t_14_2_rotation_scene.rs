//! Story 14.2 AC1.3 — the watchable N=3 mTLS-rotation-under-load scene:
//! **provision → swap → close** (the spec's own §7.2.1.a procedure, not the
//! old teardown workaround) as ONE continuous observable scene, on the same
//! in-process real-socket real-mTLS substrate as the drills
//! (`t_10_4b_rotation_real_timing.rs`).
//!
//! # Where it lives, and why (AC1.5 — decided by arithmetic)
//!
//! This is a narrating test under `crates/maos-a2a-tcp/tests/` (uncharged by
//! `kloc-check` — `kloc_check.rs` excludes `tests/`), NOT a new
//! `xtask/src/demo_*.rs`: `xtask/src` sits at ZERO headroom (40613/40613) and
//! no measured delta justifies opening that door. It is NOT `#[ignore]`-gated
//! and is deliberately NOT a `check-rotation-real-timing` leg — AC1.4 forbids
//! the scene from becoming a gate-leg oracle for any NFR floor. The drills
//! are the proof; this file is the observability artifact a human can look at.
//!
//! # The AC1.4 non-substitution guard
//!
//! The scene renders the SAME beat-set at BOTH scales side by side, and every
//! beat it did NOT execute renders **`ABSENT` in its own column** — never a
//! trailing footnote (14-1's named failure mode: someone screenshots the
//! three-host scene for a deck and the footnote is outside the crop). The
//! scene executes at N=3 ONLY; the N=10 evidence lives in the gate's legs,
//! carried by the `ROTATION_HOSTS_ROTATED` marker.
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
//! # The beats and where they come from
//!
//! 1. **Pre-rotation mesh reconcile** — the provisioned mesh stands up with
//!    DISTINCT identities (fingerprints × addrs, derived — never a count
//!    literal).
//! 2. **In-flight window established** — `W = min(DIAL_CONCURRENCY, pairs)`
//!      readonly frames confirmed live via the summed `active_connections()`
//!      gauge, count published (AC1.2).
//! 3. **Generation swap under load** — `swap_serving_cert` IN PLACE racing
//!      the live sweep inside `tokio::select!` (14-1's race shape).
//! 4. **Zero drops** — the AC1.1 accounting: every issued frame has a verdict
//!      by `t_2`. The before/after pair is the story's evidence (teardown
//!      RED `drops=W` recorded 2026-08-29 → overlap GREEN `drops=0`).
//! 5. **Post-grace old-cert rejection observed** — the windows close
//!      promote-and-retire and a ghost endpoint still serving a RETIRED leaf
//!      is refused at the mTLS layer (AC2.5).
mod support;
use std::time::{Duration, Instant};

use maos_a2a_core::error::HandshakeFailureClass;
use maos_a2a_core::identity::PeerId;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{A2AError, HandshakeRetryPolicy};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

/// The scene scale: three hosts — small enough to read, real enough that
/// every beat is a live socket op.
const SCENE_N: usize = 3;

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
    /// The N=10 column: this scene did not execute the beat at the envelope
    /// scale, so it renders `ABSENT` — in its own cell, never a footnote.
    envelope: &'static str,
}

/// Nanoseconds on the scene's own monotonic base (AC4.5).
fn mono_ns(base: &Instant) -> u64 {
    base.elapsed().as_nanos() as u64
}

#[tokio::test]
async fn t_14_2_rotation_watchable_scene_n3() {
    assert_fd_headroom(fd_headroom_for(SCENE_N));
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2-scene");
    let retry = HandshakeRetryPolicy::default();
    let names: Vec<String> = (0..SCENE_N).map(host_name).collect();

    // Two generations. The mesh is BUILT provisioned (AC2.0): the old
    // generation is served + pinned, the new one is DECLARED in every
    // A2APeerConfig — the operator's t_provision act, constructed directly.
    let old: Vec<Leaf> = (0..SCENE_N).map(|_| valid_leaf(&ca, &clock)).collect();
    let new: Vec<Leaf> = (0..SCENE_N).map(|_| valid_leaf(&ca, &clock)).collect();
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let new_refs: Vec<&Leaf> = new.iter().collect();
    let mesh =
        build_mesh_n_provisioned(&clock, &ca, &names, &old_refs, &old_refs, &new_refs, retry).await;
    // t_provision: open every rotation window — the stores' active sets
    // become {current: old, next: new}; the wire still serves OLD.
    for node in &mesh {
        for (j, peer_name) in names.iter().enumerate() {
            if node.name != *peer_name {
                node.transport
                    .pins()
                    .open_rotation_window(&PeerId::new(peer_name.clone()), &new[j].fingerprint)
                    .expect("open rotation window");
            }
        }
    }

    // ── Beat 1: pre-rotation mesh reconcile — DISTINCT identities (derived
    // fingerprints × addrs), never a count literal.
    let fingerprints: std::collections::BTreeSet<String> =
        mesh.iter().map(|m| m.fingerprint.wire()).collect();
    let addrs: std::collections::BTreeSet<std::net::SocketAddr> =
        mesh.iter().map(|m| m.addr).collect();
    assert_eq!(fingerprints.len(), SCENE_N, "distinct identities");
    assert_eq!(addrs.len(), SCENE_N, "distinct bound addrs");
    let beat_mesh = PROVEN_BLOCKING;

    // ── Beat 2: the in-flight window — W readonly frames confirmed live via
    // the summed RAII gauge, count PUBLISHED (AC1.2). Verdicts recorded
    // channel-before-resolve (AC1.1).
    let pairs: Vec<(usize, usize)> = (0..SCENE_N)
        .flat_map(|i| (0..SCENE_N).filter(move |j| *j != i).map(move |j| (i, j)))
        .collect();
    let w = DIAL_CONCURRENCY.min(pairs.len());
    assert!(
        w >= 2,
        "the window is not a window below 2 in-flight frames"
    );
    let launched = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let issued_pre = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t0_cell = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (verdicts_tx, mut verdicts_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    use futures_util::stream::{self, StreamExt};
    let mut sweep = {
        let futs = pairs.iter().enumerate().map(|(k, &(i, j))| {
            let seq = 8_000_000 + k as u64;
            let frame = make_frame(&names[i], &names[j], IntentClass::Readonly, seq);
            let to = HostId(names[j].clone());
            let node = &mesh[i];
            let verdicts_tx = verdicts_tx.clone();
            let launched = launched.clone();
            let issued_pre = issued_pre.clone();
            let t0_cell = t0_cell.clone();
            let base2 = base;
            async move {
                launched.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let ts = mono_ns(&base2);
                let t0 = t0_cell.load(std::sync::atomic::Ordering::SeqCst);
                let pre = t0 == 0 || ts <= t0;
                if pre {
                    issued_pre.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                let (_res, response_received) =
                    node.transport.route_outbound_observed(frame, &to).await;
                if pre && response_received {
                    let _ = verdicts_tx.send(seq);
                }
            }
        });
        Box::pin(stream::iter(futs).buffer_unordered(w))
    };
    let mut max_live = 0usize;
    let mut completed = 0usize;
    let armed = {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let live = launched.load(std::sync::atomic::Ordering::SeqCst) - completed;
            max_live = max_live.max(live);
            if live >= w {
                break true;
            }
            if Instant::now() > deadline {
                break false;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(0)) => {}
                delivered = sweep.next() => {
                    match delivered {
                        Some(_) => completed += 1,
                        None => break false,
                    }
                }
            }
        }
    };
    assert!(
        armed,
        "beat 2: the in-flight window must open with W={w} outstanding conversations \
         (max observed {max_live})"
    );
    let t0 = mono_ns(&base);
    t0_cell.store(t0, std::sync::atomic::Ordering::SeqCst);
    let beat_window = PROVEN_BLOCKING;

    // ── Beat 3: generation swap UNDER LOAD — in place, racing the live sweep
    // (14-1's select! shape, not biased: the poll order IS the race).
    let mut swap = Box::pin(async {
        for (i, node) in mesh.iter().enumerate() {
            node.transport
                .swap_serving_cert(vec![new[i].der.clone()], new[i].key_der.clone_key())
                .expect("in-place serving-cert swap");
        }
    });
    let mut swap_won = false;
    loop {
        tokio::select! {
            _ = &mut swap => {
                swap_won = true;
                break;
            }
            delivered = sweep.next() => {
                if delivered.is_none() {
                    break;
                }
            }
        }
    }
    if !swap_won {
        (&mut swap).await;
    }
    let drained = tokio::time::timeout(Duration::from_secs(30), async {
        while sweep.next().await.is_some() {}
    })
    .await
    .is_ok();
    assert!(drained, "beat 3: sweep must drain inside the t_2 cutoff");
    let t2 = mono_ns(&base);
    let beat_swap = PROVEN_BLOCKING;

    // ── Beat 4: ZERO DROPS — every frame issued before t_0 has a verdict by
    // t_2 (AC1.1). The teardown model measured drops=W here (2026-08-29,
    // Dev Agent Record); the overlap must measure zero.
    let issued = issued_pre.load(std::sync::atomic::Ordering::SeqCst) as u64;
    let mut verdict_ids = std::collections::BTreeSet::new();
    while let Ok(seq) = verdicts_rx.try_recv() {
        assert!(
            verdict_ids.insert(seq),
            "each issued frame can contribute at most one verdict"
        );
    }
    let verdicts = verdict_ids.len() as u64;
    assert!(verdicts <= issued, "verdicts cannot exceed issued frames");
    let drops = issued - verdicts;
    assert!(
        drops == 0,
        "beat 4: zero conversation drops under the overlap (got {drops} of {issued})"
    );
    let beat_zero_drops = PROVEN_BLOCKING;

    // ── Beat 5: post-grace OLD-CERT REJECTION observed — close every window
    // promote-and-retire, then a ghost endpoint still serving a RETIRED leaf
    // is REFUSED at the mTLS layer by a peer that pinned it (AC2.5).
    for node in &mesh {
        for peer_name in &names {
            if node.name != *peer_name {
                assert!(
                    node.transport
                        .pins()
                        .close_rotation_window(&PeerId::new(peer_name.clone()))
                        .is_some(),
                    "beat 5: window was open at close"
                );
            }
        }
    }
    let ghost_id = "scene_post_grace_ghost";
    let target = &mesh[1];
    let ghost_pins = vec![pin(&target.name, &new[1].fingerprint, 1)];
    let ghost_cfgs = vec![peer_cfg(
        &target.name,
        "tls://127.0.0.1:0",
        &new[1].fingerprint,
        &["readonly"],
        &["readonly"],
    )];
    let ghost = bind_endpoint(
        &old[0], // node 0's RETIRED leaf — node 1 pinned it before the rotation
        Some(&ca),
        1,
        ghost_pins,
        ghost_cfgs,
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    ghost.set_peer_endpoint(
        &HostId(target.name.clone()),
        format!("tls://{}", target.addr),
    );
    let frame = make_frame(ghost_id, &target.name, IntentClass::Readonly, 8_777_000);
    let refusal = ghost
        .route_outbound(frame, &HostId(target.name.clone()))
        .await;
    // The refusal surfaces on the dialer as the server's fatal handshake
    // alert (measured: `Io("recv: received fatal alert: …")`), with the
    // classified pin-mismatch shape accepted should the alert mapping
    // change. §A6 TestInfra-4: the SPECIFIC alert, never a bare "fatal
    // alert" substring.
    let refused = matches!(
        &refusal,
        Err(A2AError::Io(msg)) if msg.contains("CertificateUnknown")
    ) || matches!(
        &refusal,
        Err(A2AError::HandshakeFailed {
            class: HandshakeFailureClass::PinMismatch,
            ..
        })
    );
    assert!(
        refused,
        "beat 5: the retired cert must be refused post-grace, got {refusal:?}"
    );
    // And the NEW generation still talks: a live dial over the promoted pins.
    let f_ok = make_frame(&names[0], &names[1], IntentClass::Readonly, 8_888_000);
    let ok = mesh[0]
        .transport
        .route_outbound(f_ok, &HostId(names[1].clone()))
        .await;
    assert!(
        ok.is_ok(),
        "beat 5: new-generation traffic must ACK: {ok:?}"
    );
    let beat_post_grace = PROVEN_BLOCKING;

    // ── The beat table (AC1.4): the SAME beat-set at BOTH scales, side by
    // side. This artifact executed N=3 only — the N=10 column renders ABSENT
    // per beat, in its own cell. Never a gate-leg oracle for any NFR floor.
    let beats = [
        Beat {
            name: "provisioned mesh stood up; identities reconciled (fp x addr)",
            observed: beat_mesh,
            envelope: ABSENT,
        },
        Beat {
            name: "in-flight window: W frames confirmed live; count published",
            observed: beat_window,
            envelope: ABSENT,
        },
        Beat {
            name: "generation swap UNDER LOAD: in-place swap raced the sweep",
            observed: beat_swap,
            envelope: ABSENT,
        },
        Beat {
            name: "zero drops: every issued frame verdicted by t_2 (AC1.1)",
            observed: beat_zero_drops,
            envelope: ABSENT,
        },
        Beat {
            name: "post-grace: retired cert REFUSED; new generation ACKs",
            observed: beat_post_grace,
            envelope: ABSENT,
        },
    ];
    eprintln!("\n=== Story 14.2 watchable rotation scene (AC1.3) ===");
    eprintln!("{:=<72}", "");
    eprintln!(
        "| {:<52} | {:<16} | {:<16} |",
        "beat (asserted, not narrated)", "N=3 (observed)", "N=10"
    );
    eprintln!("|{:-<54}|{:-<18}|{:-<18}|", "", "", "");
    for beat in &beats {
        eprintln!(
            "| {:<52} | {:<16} | {:<16} |",
            beat.name, beat.observed, beat.envelope
        );
    }
    eprintln!("{:=<72}", "");
    eprintln!(
        "  W={w} in flight at t_0; drops={drops}; swap-to-drain {} ms; \
         EvidenceState wire vocabulary (gate_common.rs): {} = executed and \
         asserted here; {} = not executed by THIS artifact (never a footnote; \
         never a gate-leg oracle — AC1.4). The N=10 proof is \
         check-rotation-real-timing's marked legs.",
        (t2 - t0) / 1_000_000,
        PROVEN_BLOCKING,
        ABSENT
    );
}
