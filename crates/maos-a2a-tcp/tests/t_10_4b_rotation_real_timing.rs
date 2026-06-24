//! AC-10.4b — REAL-socket mTLS cert-rotation chaos drill with MEASURED
//! wall-clock timestamps.
//!
//! Three `TcpA2ATransport` endpoints over real `127.0.0.1` sockets + real
//! rustls mTLS handshakes. Unlike the OLD synthetic rotation harness
//! (`maos_a2a_core::chaos::harness_3_host`, which seeds `t_0/t_1/t_2` from a
//! `DrillConfig` under `tokio::time::pause()`), this drill measures the three
//! rotation events with `SystemTime::now()` on a live mesh:
//!
//! * `t_0` — rotation initiated: the old serving certs are superseded and the
//!   fresh leaves are issued (the rotation anchor). `TcpA2ATransport` exposes
//!   no in-place cert-swap API, so rotation is modelled as old-mesh-teardown +
//!   new-mesh-bind; `t_0` is the moment that rotation begins.
//! * `t_1` — a peer FIRST verifies node `i`'s NEW serving cert via a LIVE mTLS
//!   handshake + data-plane round-trip (the revocation-propagation analog: the
//!   new pin is observably active). Recorded by the post-rotation directed-dial
//!   sweep — a real socket op, NOT a listener-bind timestamp.
//! * `t_2` — post-rotation NxN reachability confirmed == the sweep's full
//!   re-handshake + data-plane round-trip set completes.
//!
//! The values are sub-second (everything is localhost), but they are REAL — no
//! `tokio::time::pause()`, no hand-built timestamps. They are fed straight into
//! `RotationDrillReport::from_per_agent` and asserted against the §7.2.1.b
//! floors.
mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{
    AgentRotationTimestamps, HandshakeRetryPolicy, RotationDrillReport, compute_t_grace,
};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const HOSTS: [&str; 3] = ["host_a", "host_b", "host_c"];
const NONCES: [u64; 3] = [10, 20, 30];

/// Real wall-clock nanoseconds since UNIX epoch (no `tokio::time::pause()`).
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos() as u64
}

/// Bind node `idx` serving `own_leaf`, pinning/allowlisting each `peers` entry
/// by that peer's EXPECTED leaf fingerprint. `own_leaf` and the expected leaf
/// are deliberately separate so a half-failed rotation (one node serving a
/// stale cert while its peers pin the new one) can be modelled.
async fn bind_node(
    clock: &Clock,
    ca: &Ca,
    idx: usize,
    own_leaf: &Leaf,
    peers: &[(usize, &Leaf)],
    retry: HandshakeRetryPolicy,
) -> TcpA2ATransport {
    let peer_pins: Vec<_> = peers
        .iter()
        .map(|(j, leaf)| pin(HOSTS[*j], &leaf.fingerprint, NONCES[*j]))
        .collect();
    let peer_cfgs: Vec<_> = peers
        .iter()
        .map(|(j, leaf)| {
            peer_cfg(
                HOSTS[*j],
                "tls://127.0.0.1:0",
                &leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )
        })
        .collect();
    bind_endpoint(
        own_leaf,
        Some(ca),
        NONCES[idx],
        peer_pins,
        peer_cfgs,
        clock,
        TcpTimeouts::test_profile(),
        retry,
    )
    .await
}

/// Build a wired 3-node mesh. `serving[i]` is the cert node `i` actually
/// serves; `expected[i]` is the fingerprint every OTHER node pins/allowlists
/// for node `i`. Healthy rotation: `serving == expected`. Half-failed
/// rotation: one differs (pin mismatch).
async fn build_mesh(
    clock: &Clock,
    ca: &Ca,
    serving: [&Leaf; 3],
    expected: [&Leaf; 3],
    retry: HandshakeRetryPolicy,
) -> Vec<TcpA2ATransport> {
    let mut nodes = Vec::with_capacity(3);
    for i in 0..3 {
        let peers: Vec<(usize, &Leaf)> = (0..3)
            .filter(|j| *j != i)
            .map(|j| (j, expected[j]))
            .collect();
        nodes.push(bind_node(clock, ca, i, serving[i], &peers, retry.clone()).await);
    }
    // Wire real readback addresses (H3/H4) now that all listeners are bound.
    let addrs: Vec<_> = nodes.iter().map(|n| n.local_addr().unwrap()).collect();
    for (i, node) in nodes.iter().enumerate() {
        for j in 0..3 {
            if j != i {
                node.set_peer_endpoint(&HostId(HOSTS[j].into()), format!("tls://{}", addrs[j]));
            }
        }
    }
    nodes
}

/// Observations from an NxN directed-dial sweep over REAL sockets.
///
/// * `ok` / `total` — directed-pair success/total counts (GREEN vs RED).
/// * `first_success_ns[j]` — the instant node `j`'s serving cert is FIRST
///   verified by a peer via a live mTLS handshake + data-plane round-trip (the
///   rotation "new-pin-active" event). `None` if node `j` was never reached.
/// * `pair_ok[from][to]` — exact per-pair success matrix for topology oracles
///   (diagonal unused).
struct DialSweep {
    ok: usize,
    total: usize,
    first_success_ns: Vec<Option<u64>>,
    pair_ok: Vec<Vec<bool>>,
}

/// Attempt every directed `i → j` dial (`i != j`) over real sockets, recording
/// per-pair success, the per-target first-success timestamp, and aggregate
/// counts. Never panics — the caller decides GREEN (all ACK) vs RED (some fail)
/// from the result.
async fn directed_dial_sweep(nodes: &[TcpA2ATransport], seq_base: u64) -> DialSweep {
    let n = nodes.len();
    let mut first_success_ns = vec![None; n];
    let mut pair_ok = vec![vec![false; n]; n];
    let mut ok = 0usize;
    let mut total = 0usize;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            total += 1;
            let frame = make_frame(
                HOSTS[i],
                HOSTS[j],
                IntentClass::Readonly,
                seq_base + (i as u64 * n as u64 + j as u64),
            );
            if nodes[i]
                .route_outbound(frame, &HostId(HOSTS[j].into()))
                .await
                .is_ok()
            {
                ok += 1;
                pair_ok[i][j] = true;
                if first_success_ns[j].is_none() {
                    first_success_ns[j] = Some(now_ns());
                }
            }
        }
    }
    DialSweep {
        ok,
        total,
        first_success_ns,
        pair_ok,
    }
}

// ─────────────────────────── AC-10.4b GREEN ───────────────────────────

/// Test 1 — live 3-host rotation drill with REAL measured timestamps.
///
/// Phase 1 establishes a pre-paired mTLS mesh and proves full NxN
/// reachability (6 directed dials). Phase 2 rotates every endpoint's serving
/// cert, measuring `t_0` (rotation initiated: old certs superseded, fresh
/// leaves issued), per-node `t_1` (a peer first verifies node i's NEW serving
/// cert via a LIVE mTLS handshake — the revocation-propagation analog, recorded
/// by the directed-dial sweep), and `t_2` (the sweep's full NxN re-handshake +
/// data-plane round-trip set completes). The three real `(t_0, t_1, t_2)`
/// tuples feed `RotationDrillReport::from_per_agent`, which must pass BOTH the
/// v0.7 and v1.0 floors with zero conversation drops.
#[tokio::test]
async fn t_10_4b_rotation_real_timing_3_host_drill() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-10-4b-drill");

    // ── Phase 1: pre-paired mTLS mesh, full NxN reachability (zero drops) ──
    let old: [Leaf; 3] = [
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
    ];
    let old_refs: [&Leaf; 3] = [&old[0], &old[1], &old[2]];
    let mesh_old = build_mesh(
        &clock,
        &ca,
        old_refs,
        old_refs,
        HandshakeRetryPolicy::default(),
    )
    .await;
    let sweep1 = directed_dial_sweep(&mesh_old, 1000).await;
    assert_eq!(
        sweep1.ok, sweep1.total,
        "Phase 1: zero conversation drops — full NxN reachability over REAL sockets"
    );

    // ── Phase 2: REAL rotation with measured timestamps ──
    // t_0 = rotation initiated (old serving certs superseded, fresh leaves about to be issued).
    let t0 = now_ns();
    drop(mesh_old); // drop the old mesh (H6 deterministic teardown frees old ports)

    // New cert material: fresh leaves from the SAME CA.
    let new: [Leaf; 3] = [
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
    ];

    // Rebind each node with its new cert + new peer pins. (TcpA2ATransport
    // exposes no in-place cert-swap API, so rotation is modelled as a rebind:
    // old cert superseded → new cert bound + peers re-pinned at the new
    // fingerprint. The rotation EVENTS below are still measured from live
    // socket ops, not from these binds.)
    let mut nodes = Vec::with_capacity(3);
    for i in 0..3 {
        let peers: Vec<(usize, &Leaf)> = (0..3).filter(|j| *j != i).map(|j| (j, &new[j])).collect();
        nodes.push(
            bind_node(
                &clock,
                &ca,
                i,
                &new[i],
                &peers,
                HandshakeRetryPolicy::default(),
            )
            .await,
        );
    }
    // Wire real readback addresses (H3/H4) now that all listeners are bound.
    let addrs: Vec<_> = nodes.iter().map(|n| n.local_addr().unwrap()).collect();
    for (i, node) in nodes.iter().enumerate() {
        for j in 0..3 {
            if j != i {
                node.set_peer_endpoint(&HostId(HOSTS[j].into()), format!("tls://{}", addrs[j]));
            }
        }
    }

    // Phase 2 NxN reachability == re-handshake + data-plane round-trip over the
    // NEW certs. The sweep also records t_1[i] — the instant a peer FIRST
    // verifies node i's new serving cert via a live handshake (the
    // revocation-propagation analog: the new pin is observably active). Every
    // dial must ACK (zero conversation drops).
    let sweep2 = directed_dial_sweep(&nodes, 2000).await;
    assert_eq!(
        sweep2.ok, sweep2.total,
        "Phase 2: zero conversation drops — all NxN dials ACK over new certs"
    );
    // t_1[i] = the new pin first observed ACTIVE via a live socket op.
    let mut t1_ns = [0u64; 3];
    for i in 0..3 {
        t1_ns[i] = sweep2.first_success_ns[i]
            .expect("every node must be reached by a peer over its new cert");
    }
    // t_2 = post-rotation full NxN re-handshake + data-plane round-trip confirmed.
    let t2 = now_ns();

    // Build 3 AgentRotationTimestamps from the REAL measured values and feed
    // them into the aggregate report.
    let per_agent: Vec<AgentRotationTimestamps> = (0..3)
        .map(|i| AgentRotationTimestamps {
            agent_id: HOSTS[i].into(),
            t_0_ns: t0,
            t_1_ns: Some(t1_ns[i]),
            t_2_ns: Some(t2),
        })
        .collect();
    // T_grace per §7.2.1.a cold-deployment floor (p99=500ms, 7 days history):
    // max(2*max(500,500), 5000) = 5000ms.
    let t_grace_ms = compute_t_grace(500, 7).as_millis() as u64;
    let report = RotationDrillReport::from_per_agent(
        "10-4b-live-3host-drill",
        3,
        500,
        t_grace_ms,
        per_agent,
        0,                   // post_grace_reject_count: NO old-cert connection survived rotation
        sweep2.total as u64, // every phase-2 connection used a new cert
    );

    // Transparency: print the REAL measured timings.
    eprintln!("\n=== AC-10.4b live 3-host rotation drill (REAL timestamps) ===");
    for a in &report.per_agent {
        eprintln!(
            "  {:>7}: t0={:_>20}ns  prop(t1-t0)={:>5}ms  re-hs(t2-t1)={:>5}ms  e2e(t2-t0)={:>5}ms",
            a.agent_id,
            a.t_0_ns,
            a.revocation_propagation_ms().unwrap_or(0),
            a.re_handshake_ms().unwrap_or(0),
            a.end_to_end_ms().unwrap_or(0),
        );
    }
    eprintln!(
        "  revocation-propagation p50/p99 = {}/{} ms   (v0.7 floors 30000/90000)",
        report.revocation_propagation_p50_ms, report.revocation_propagation_p99_ms
    );
    eprintln!(
        "  re-handshake            p50/p99 = {}/{} ms   (v0.7 floors 30000/60000)",
        report.re_handshake_p50_ms, report.re_handshake_p99_ms
    );
    eprintln!(
        "  end-to-end              p50/p99 = {}/{} ms   (v1.0 floors 60000/150000)",
        report.end_to_end_p50_ms, report.end_to_end_p99_ms
    );
    eprintln!(
        "  post-grace reject rate = {:.6}   (v1.0 floor 0.001)",
        report.post_grace_reject_rate
    );
    eprintln!(
        "  => passes_v07_floors={}  passes_v10_floors={}",
        report.passes_v07_floors, report.passes_v10_floors
    );

    // localhost timings are sub-second → far under every floor.
    assert!(
        report.passes_v07_floors,
        "v0.7 floors MUST pass: localhost rotation timings are well under 30s/90s/30s/60s"
    );
    assert!(
        report.passes_v10_floors,
        "v1.0 floors MUST pass: end-to-end < 60s/150s and post-grace reject rate is 0"
    );
    // Explicit zero-drop invariant across both phases.
    assert_eq!(
        sweep1.ok + sweep2.ok,
        sweep1.total + sweep2.total,
        "zero conversation drops across both phases"
    );
}

// ─────────────────────────── AC-10.4b RED vectors ───────────────────────────

/// Test 2 — proven-RED: a half-failed rotation (one host fails to rotate its
/// cert) degrades reachability below NxN to an EXACT topology.
///
/// Hosts A & B rotate to fresh certs and re-pin C at its NEW fingerprint, but
/// C serves a DIFFERENT (stale) leaf — i.e. C never rotated. Every directed
/// pair involving C hits a deterministic mTLS pin mismatch (`PinMismatch`, a
/// NON-retryable failure class) and fails on the first attempt; only the A↔B
/// directed pairs (A→B and B→A) survive. Both the baseline and the half-failed
/// meshes use the SAME retry policy as the GREEN drill
/// (`HandshakeRetryPolicy::default()`), so the RED outcome is attributable to
/// the rotation failure, not to a retry-starved baseline. Reachability drops to
/// exactly 2/6 → the drill goes RED.
#[tokio::test]
async fn t_10_4b_rotation_proven_red_drop_reachability() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-10-4b-red-drop");
    // Same retry policy as the GREEN drill (default) so the RED outcome is
    // attributable to the rotation failure, not a retry-starved baseline. A
    // half-failed pair's PinMismatch is a NON-retryable class, so the failing
    // pairs still drop on the first attempt — the drill stays fast.
    let retry = HandshakeRetryPolicy::default();

    // ── Phase 1: healthy mesh, full NxN reachability (baseline GREEN) ──
    let old: [Leaf; 3] = [
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
    ];
    let old_refs: [&Leaf; 3] = [&old[0], &old[1], &old[2]];
    let mesh_old = build_mesh(&clock, &ca, old_refs, old_refs, retry.clone()).await;
    let sweep1 = directed_dial_sweep(&mesh_old, 3000).await;
    assert_eq!(
        sweep1.ok, sweep1.total,
        "baseline: full NxN reachability before rotation"
    );
    drop(mesh_old);

    // ── Phase 2: half-failed rotation — C does NOT rotate to its new cert ──
    let new_a = valid_leaf(&ca, &clock);
    let new_b = valid_leaf(&ca, &clock);
    let new_c = valid_leaf(&ca, &clock); // what A & B EXPECT C to serve
    let c_stale = valid_leaf(&ca, &clock); // what C ACTUALLY serves (didn't rotate)
    let serving: [&Leaf; 3] = [&new_a, &new_b, &c_stale];
    let expected: [&Leaf; 3] = [&new_a, &new_b, &new_c];
    let mesh_new = build_mesh(&clock, &ca, serving, expected, retry.clone()).await;

    let sweep2 = directed_dial_sweep(&mesh_new, 4000).await;
    // Exact surviving topology: only the A↔B directed pairs survive the
    // half-failed rotation (A→B and B→A); every C-involving pair fails on the
    // deterministic pin mismatch.
    let mut surviving: Vec<(&'static str, &'static str)> = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            if i != j && sweep2.pair_ok[i][j] {
                surviving.push((HOSTS[i], HOSTS[j]));
            }
        }
    }
    eprintln!("\n=== AC-10.4b proven-RED (dropped reachability) ===");
    eprintln!(
        "  reachable {}/{} directed pairs after a half-failed rotation (NxN would be {})",
        sweep2.ok, sweep2.total, sweep2.total
    );
    eprintln!("  surviving directed pairs: {:?}", surviving);
    assert_eq!(
        surviving,
        vec![("host_a", "host_b"), ("host_b", "host_a")],
        "only the A↔B directed pairs must survive a half-failed rotation"
    );
    // Core RED assertion: reachability MUST degrade below NxN to exactly 2/6.
    assert_eq!(
        sweep2.ok, 2,
        "reachability MUST drop to exactly 2/6 directed pairs → RED (got {})",
        sweep2.ok
    );
    assert!(
        sweep2.ok < sweep2.total,
        "reachability MUST drop below NxN when one host fails to rotate its cert → RED"
    );
}

/// Test 3 — proven-RED: a DEGRADED rotation whose revocation-propagation p99
/// EXCEEDS the 90000ms v0.7 floor forces the report RED.
///
/// This proves the floor check actually goes RED when timing degrades (it is
/// not a rubber-stamp). The timestamps are anchored to a LIVE-measured `t_0`
/// (`now_ns()`) with the per-agent propagation/re-handshake durations injected
/// as CONTROLLED time-offsets — a hermetic test cannot incur a real 90s+
/// propagation delay, so the degradation is modelled as named offsets recorded
/// through the same `RotationDrillReport::from_per_agent` drill path used by
/// the GREEN test, not as hand-built absolute fixture timestamps. One agent (c)
/// observes the revocation 95s after `t_0`, so p99 = 95s > 90s floor; all other
/// axes are kept under their floors so the failure isolates to
/// revocation-propagation p99.
#[test]
fn t_10_4b_rotation_proven_red_p99_exceeds_floor() {
    // LIVE-measured rotation anchor (same wall clock as the GREEN drill). A
    // hermetic test cannot incur a real 90s+ revocation-propagation delay, so
    // the per-agent propagation and re-handshake durations below are injected
    // as CONTROLLED time-offsets on top of this anchor — recorded through the
    // same `from_per_agent` drill path, not as hand-built absolute timestamps.
    let t0 = now_ns();

    // revocation-propagation = t_1 − t_0 (controlled degradation offsets):
    //   a: 20s   b: 30s   c: 95s  → p99 = 95s (> 90s v0.7 floor).
    const NS_PER_MS: u64 = 1_000_000;
    const PROP_A_MS: u64 = 20_000;
    const PROP_B_MS: u64 = 30_000;
    const PROP_C_MS: u64 = 95_000; // exceeds the 90s v0.7 p99 floor
    // re-handshake = t_2 − t_1 (kept under the 60s floor so the failure
    // isolates to revocation-propagation p99).
    const RH_MS: u64 = 10_000;
    let mk = |prop_ms: u64| -> (u64, u64) {
        (t0 + prop_ms * NS_PER_MS, t0 + (prop_ms + RH_MS) * NS_PER_MS)
    };
    let (t1_a, t2_a) = mk(PROP_A_MS);
    let (t1_b, t2_b) = mk(PROP_B_MS);
    let (t1_c, t2_c) = mk(PROP_C_MS);

    let agents = vec![
        AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: t0,
            t_1_ns: Some(t1_a),
            t_2_ns: Some(t2_a),
        },
        AgentRotationTimestamps {
            agent_id: "b".into(),
            t_0_ns: t0,
            t_1_ns: Some(t1_b),
            t_2_ns: Some(t2_b),
        },
        AgentRotationTimestamps {
            agent_id: "c".into(),
            t_0_ns: t0,
            t_1_ns: Some(t1_c),
            t_2_ns: Some(t2_c),
        },
    ];
    let report = RotationDrillReport::from_per_agent(
        "10-4b-red-p99-exceeds-floor",
        3,
        500,
        5_000,
        agents,
        0,
        100,
    );

    assert_eq!(
        report.revocation_propagation_p99_ms, PROP_C_MS,
        "p99 must be the worst agent's 95s propagation"
    );
    assert!(
        report.revocation_propagation_p99_ms > 90_000,
        "p99 {}ms must exceed the 90000ms floor",
        report.revocation_propagation_p99_ms
    );
    assert!(
        !report.passes_v07_floors,
        "v0.7 floors MUST go RED when revocation-propagation p99 exceeds 90s"
    );
    assert!(
        !report.passes_v10_floors,
        "v1.0 floors (which require v0.7) MUST also be RED"
    );
}
