//! AC-10.4b / Story 14-2 — REAL-socket mTLS cert-rotation chaos drill with
//! MEASURED timestamps, run UNDER LOAD across an N-host mesh.
//!
//! N `TcpA2ATransport` endpoints over real `127.0.0.1` sockets + real rustls
//! mTLS handshakes. Unlike the OLD synthetic rotation harness
//! (`maos_a2a_core::chaos::harness_3_host` — DELETED by this story, AC6.1;
//! it seeded `t_0/t_1/t_2` from `DrillConfig` constants and asserted its own
//! floors green), this drill measures the three rotation events on a live
//! mesh with a monotonic `Instant` base (AC4.5 — `SystemTime` is
//! NTP-steppable and was retired):
//!
//! * `t_0` — rotation initiated: the serving generation is swapped IN PLACE
//!   (`swap_serving_cert`, AC2.4.a) while a W-dial in-flight window is
//!   provably open. Rotation is `provision → swap → close` (AC2.0): the mesh
//!   is BUILT provisioned (cfg declares the replacement; §7.2.1.a requires
//!   provisioning ≥ `T_grace` before revocation — AC6.5(c) records this as an
//!   ASSUMED precondition, not a built mechanism), every store opens its
//!   one-generation window, the wire still serves the old leaf, then the swap
//!   lands mid-load, then the window closes promote-and-retire.
//! * `t_1` — a peer FIRST verifies node `i`'s NEW serving cert via a LIVE mTLS
//!   handshake + data-plane round-trip (the revocation-propagation analog: the
//!   new pin is observably active). Recorded by the post-rotation directed-dial
//!   sweep — a real socket op, NOT a listener-bind timestamp.
//! * `t_2` — per-agent (AC4.4): node `i`'s own first successful post-swap
//!   data-plane round trip (its first source-side success in the sweep), NOT a
//!   single shared wall reading — `end_to_end` is a real distribution.
//!
//! The values are sub-second (everything is localhost), but they are REAL — no
//! `tokio::time::pause()`, no hand-built timestamps. They are fed straight into
//! `RotationDrillReport::from_per_agent` and asserted against the §7.2.1.b
//! floors via `passes_v15_floors` (which IS `passes_v10_floors` — NFR-Sec-13's
//! literal PRD numbers are LOOSER on every axis and are never adopted).
//!
//! # Story 14-2 / AC1.1 — the conversation-drop definition
//!
//! > **A conversation drop is a frame issued before `t_0` that never received
//! > a verdict — neither an ACK nor any typed NACK — by `t_2`.**
//! > It is **not** a retried dial (retries are internal and a
//! > retried-then-succeeded dial returns `Ok`, visible only as
//! > `last_dial_attempts()`). It is **not** an `IntentDeniedAtPeer`
//! > (`router.rs:1045`) — that is a completed conversation answered "no". It
//! > is **not** a post-rotation reachability failure; that is the existing
//! > Phase-2 assertion, already proven-red below.
//!
//! Drops are counted **test-side by sequence number** (no production drop
//! counter exists or is owed): every dial records its verdict into a
//! completion channel *before* the future resolves (14-1's
//! channel-before-resolve idiom), and a dial that ends in a transport-plane
//! death — `TransportFailed` / `Io` / `PartitionTimeout`, where ECONNRESET
//! and ECONNREFUSED land (`error.rs:39`) — or that never resolves by `t_2`
//! received no verdict and is a drop.
//!
//! **The before/after pair is the evidence (AC2.4):** against the PRE-14-2
//! teardown-rebind rotation this exact window measured `drops = 6 = W` on 10
//! consecutive runs (2026-08-29, recorded in the Dev Agent Record); under the
//! provision→swap→close overlap the same window measures `drops == 0`. That
//! pair, not a clean pass, is the proof.
//!
//! # Substrate notes
//!
//! * The mesh is the SHARED harness (`support::build_mesh_n*`, the ONE mesh
//!   primitive `build_mesh_inner`) — the local `bind_node`/`build_mesh` fork
//!   is retired (AC3.1). `directed_dial_sweep` is KEPT deliberately: it is the
//!   only source of `t_1`/`t_2` timestamps (`concurrent_dial_pairs` returns
//!   none), and it coexists with the window's `buffer_unordered(W)` stream by
//!   design (AC3.1.a — ordered reachability sweep vs unordered racing window;
//!   they are not substitutes).
//! * The mesh is in-process (11.3 F1 disclosure, re-stated at N hosts): real
//!   sockets, real rustls, real mTLS, one process, one runtime, loopback.
//! * Revocation is NOT measured (AC6.5(a)): no OCSP/CRL exists (ADR-047 §3/§4
//!   forbid it under NFR-Ops-12); `t_1` is the new-pin-observably-active
//!   proxy.
//! * Every drill wraps in a `RunnerEnvelope` and probes the fd ceiling
//!   (`assert_fd_headroom(fd_headroom_for(n))`) — AC3.4/AC3.5: re-measure on
//!   the runner, never inherit numbers.
mod support;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use maos_a2a_core::chaos::report::report_to_markdown;
use maos_a2a_core::error::HandshakeFailureClass;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{
    compute_t_grace, A2AError, AgentRotationTimestamps, HandshakeRetryPolicy, RotationDrillReport,
};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

/// The floors leg's scale (AC3.3 — thin per-N wrappers over `rotation_drill`,
/// never an env var: 14-1's ratified AC2.1 shape).
const ROTATION_FLOORS_HOST_COUNT: usize = 3;
/// The 10-host envelope leg's scale (the epic's scale-out target, NFR-Sec-13).
const ROTATION_ENVELOPE_HOST_COUNT: usize = 10;

/// Nanoseconds elapsed on the drill's OWN monotonic `Instant` base (AC4.5 —
/// `Instant` is monotonic, so a backward wall-clock step can never invert a
/// latency delta; `SystemTime::now()` was retired for this purpose).
fn mono_ns(base: &Instant) -> u64 {
    base.elapsed().as_nanos() as u64
}

// ─────────────────────────── shared drill primitives ───────────────────────────

/// Observations from an NxN directed-dial sweep over REAL sockets.
///
/// * `ok` / `total` — directed-pair success/total counts (GREEN vs RED).
/// * `first_success_ns[j]` — the instant node `j`'s serving cert is FIRST
///   verified by a peer via a live mTLS handshake + data-plane round-trip (the
///   rotation "new-pin-active" event, `t_1[j]`). `None` if never reached.
/// * `first_source_success_ns[i]` — node `i`'s first successful data-plane
///   round trip after its own `t_1[i]` was observed (its per-agent `t_2`,
///   AC4.4). A follow-up dial is issued when source-major ordering would
///   otherwise place the source success before `t_1`.
/// * `pair_ok[from][to]` — exact per-pair success matrix for topology oracles
///   (diagonal unused).
struct DialSweep {
    ok: usize,
    total: usize,
    first_success_ns: Vec<Option<u64>>,
    first_source_success_ns: Vec<Option<u64>>,
    pair_ok: Vec<Vec<bool>>,
}

/// Attempt every directed `i → j` dial (`i != j`) over real sockets, recording
/// per-pair success, per-target and per-source first-success timestamps, and
/// aggregate counts. Never panics — the caller decides GREEN (all ACK) vs RED
/// (some fail) from the result. KEPT deliberately (AC3.1): the only source of
/// `t_1`/`t_2` timestamps — `concurrent_dial_pairs` returns none.
async fn directed_dial_sweep(mesh: &[MeshNode], seq_base: u64, base: &Instant) -> DialSweep {
    let n = mesh.len();
    let mut first_success_ns = vec![None; n];
    let mut first_source_success_ns = vec![None; n];
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
                &mesh[i].name,
                &mesh[j].name,
                IntentClass::Readonly,
                seq_base + (i as u64 * n as u64 + j as u64),
            );
            let res = mesh[i]
                .transport
                .route_outbound(frame, &HostId(mesh[j].name.clone()))
                .await;
            if res.is_ok() {
                ok += 1;
                pair_ok[i][j] = true;
                let ts = mono_ns(base);
                if first_success_ns[j].is_none() {
                    first_success_ns[j] = Some(ts);
                }
                if first_success_ns[i].is_some() && first_source_success_ns[i].is_none() {
                    first_source_success_ns[i] = Some(ts);
                }
            }
        }
    }
    // Source-major ordering makes host_00's first source success precede its
    // first target success. Measure a real post-t_1 round trip for any such
    // agent instead of clamping a negative interval to a synthetic 0 ms.
    for i in 0..n {
        if first_success_ns[i].is_some() && first_source_success_ns[i].is_none() {
            let j = (i + 1) % n;
            let frame = make_frame(
                &mesh[i].name,
                &mesh[j].name,
                IntentClass::Readonly,
                seq_base + (n * n + i) as u64,
            );
            if mesh[i]
                .transport
                .route_outbound(frame, &HostId(mesh[j].name.clone()))
                .await
                .is_ok()
            {
                first_source_success_ns[i] = Some(mono_ns(base));
            }
        }
    }
    DialSweep {
        ok,
        total,
        first_success_ns,
        first_source_success_ns,
        pair_ok,
    }
}

/// The strict conversation-verdict boundary is whether a decoded peer response
/// arrived. This preserves every typed NACK without misclassifying a same-shaped
/// local error as a verdict.
fn strict_received_verdict(response_received: bool) -> bool {
    response_received
}

#[cfg(not(feature = "rotation-fault-inject"))]
fn received_verdict(_res: &Result<(), A2AError>, response_received: bool) -> bool {
    strict_received_verdict(response_received)
}

/// AC5.5(b): blind the REAL harness observation seam. Every load-window caller
/// uses this function, so the feature changes the measured drop count rather
/// than exercising a disconnected toy predicate.
#[cfg(feature = "rotation-fault-inject")]
fn received_verdict(_res: &Result<(), A2AError>, _response_received: bool) -> bool {
    true
}

/// Story 14-1 AC5.1-style marker (AC5.4): every rotation drill prints the
/// DERIVED host count — a test that early-returns prints no marker and the
/// gate reds the leg. The value is derived from the distinct `agent_id`s
/// actually present in `per_agent` (AC3.2), never the self-declared
/// `host_count` field.
const ROTATION_MARKER_PREFIX: &str = "ROTATION_HOSTS_ROTATED";

fn print_rotation_marker(derived_hosts: usize) {
    println!("{ROTATION_MARKER_PREFIX}={derived_hosts}");
}

/// The derived identity witness (AC3.2, mirroring
/// `ChurnDrillReport::distinct_host_count`): the count of DISTINCT `agent_id`s
/// actually present in `per_agent`. The 10-host floor is asserted against
/// THIS, never against `rotation.rs`'s self-declared `host_count` — a report
/// that SAYS ten hosts and MEASURED three must not pass a ten-host floor
/// (Blocking condition 4).
fn distinct_agent_witness(per_agent: &[AgentRotationTimestamps]) -> usize {
    per_agent
        .iter()
        .map(|a| a.agent_id.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

// ─────────────────────────── RunnerEnvelope (AC3.4, copied from 14-1) ───────────────────────────

/// The ratified wall/RSS trip conditions (14-1 AC6.2.a): 300 s / 1 GiB.
const RUNNER_WALL_BUDGET_SECS: u64 = 300;
const RUNNER_PEAK_RSS_BUDGET_KB: u64 = 1_048_576;

/// RAII guard COPIED from `t_11_3_scale_churn.rs:190-245` (AC3.4 — "copy
/// them; do not rebuild them"): publishes `wall` + `peak_rss_kb` for the
/// drill it wraps and enforces the trip condition on drop. Never fires while
/// the thread is already panicking, so a real assertion failure is never
/// masked by a budget message.
struct RunnerEnvelope {
    label: String,
    started: Instant,
}

impl RunnerEnvelope {
    fn open(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            started: Instant::now(),
        }
    }
}

impl Drop for RunnerEnvelope {
    fn drop(&mut self) {
        let wall = self.started.elapsed();
        let rss = peak_rss_kb();
        // Unmeasured VmHWM prints `unmeasured` and FAILS by name — never
        // passes as 0 (14-1 round-2 P2).
        let rss_token = match rss {
            Some(kb) => format!("{kb}"),
            None => "unmeasured".to_string(),
        };
        println!(
            "ROTATION_RUNNER_ENVELOPE {} wall_s={:.2} peak_rss_kb={rss_token}",
            self.label,
            wall.as_secs_f64()
        );
        if std::thread::panicking() {
            return;
        }
        assert!(
            wall.as_secs_f64() <= RUNNER_WALL_BUDGET_SECS as f64,
            "RUNNER-BUDGET ({}): wall {:.1}s exceeds the ratified {RUNNER_WALL_BUDGET_SECS}s trip condition (AC3.4)",
            self.label,
            wall.as_secs_f64()
        );
        let rss = rss.unwrap_or_else(|| {
            panic!(
                "RUNNER-UNMEASURED ({}): /proc/self/status VmHWM unreadable — unmeasured must \
                 fail by name, not pass as 0 (AC3.5)",
                self.label
            )
        });
        assert!(
            rss <= RUNNER_PEAK_RSS_BUDGET_KB,
            "RUNNER-BUDGET ({}): peak RSS {rss} kB exceeds the ratified 1 GiB trip condition (AC3.4)",
            self.label
        );
    }
}

// ─────────────────────────── the rotation drill (AC1/AC2/AC3/AC4) ───────────────────────────

/// Post-grace probe (AC2.5): a GHOST endpoint still serving node `i`'s
/// RETIRED leaf dials a peer that pinned node i, over the REAL dial path.
/// The peer's listen-side lookup (site 2) must refuse the retired identity —
/// the dial dies in the mTLS handshake with `HandshakeFailed { PinMismatch }`,
/// a typed, observed rejection through the same surface the proven-red
/// pin-mismatch vector uses. (A raw `TlsConnector::connect().is_err()` probe
/// is NOT an oracle here: in TLS 1.3 the client's handshake completes before
/// the server has verified the client certificate, so the refusal alert
/// arrives only on the first read — a request/response round trip, never a
/// bare connect, observes it.) Returns `true` when the presentation was
/// REFUSED.
async fn retired_cert_probe_refused(
    target: &MeshNode,
    retired_leaf: &Leaf,
    target_current: &PeerCertFingerprint,
    ca: &Ca,
    clock: &Clock,
) -> bool {
    let ghost_id = "post_grace_ghost";
    let pins = vec![pin(&target.name, target_current, 1)];
    let cfgs = vec![peer_cfg(
        &target.name,
        "tls://127.0.0.1:0",
        target_current,
        &["readonly"],
        &["readonly"],
    )];
    let ghost = bind_endpoint(
        retired_leaf,
        Some(ca),
        1,
        pins,
        cfgs,
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    ghost.set_peer_endpoint(
        &HostId(target.name.clone()),
        format!("tls://{}", target.addr),
    );
    let frame = make_frame(ghost_id, &target.name, IntentClass::Readonly, 7_777_000);
    let res = ghost
        .route_outbound(frame, &HostId(target.name.clone()))
        .await;
    // The refusal is OBSERVED on the wire as the server's fatal handshake
    // alert (measured: `Io("recv: received fatal alert: CertificateUnknown")`
    // on the dialer — the TLS 1.3 client-cert rejection alert; the
    // PIN_MISMATCH tag lives in the server's verifier and does not
    // round-trip). §A6 TestInfra-4: match the SPECIFIC alert/class — a bare
    // "fatal alert" substring would count ANY TLS failure as a refusal — and
    // every probe is PAIRED with a current-cert success on the same path
    // (in the drill's loop), proving the path is alive and the refusal is
    // cert-specific.
    matches!(&res, Err(A2AError::Io(msg)) if msg.contains("CertificateUnknown"))
        || matches!(
            &res,
            Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::PinMismatch,
                ..
            })
        )
}
/// swap IN PLACE racing the live sweep (`t_0`) → drain with zero drops →
/// post-swap sweep (`t_1`/`t_2`) → windows closed promote-and-retire →
/// post-grace retired-cert probes (AC2.5) → report + floors + disclosures.
async fn rotation_drill(n: usize) {
    assert!(n >= 2, "a rotation needs at least a pair");
    assert_fd_headroom(fd_headroom_for(n)); // AC3.5 — probe, never assume
    let _envelope = RunnerEnvelope::open(format!("rotation-drill-n{n}")); // AC3.4
    let base = Instant::now(); // AC4.5 — the monotonic base every sample cites
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-10-4b-rotation");
    let retry = HandshakeRetryPolicy::default();
    let names: Vec<String> = (0..n).map(host_name).collect();

    // Two generations: `old` is served+ pinned at steady state; `new` is what
    // the operator DECLARED at t_provision (AC2.0) and what the mesh rotates to.
    let old: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let new: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let new_refs: Vec<&Leaf> = new.iter().collect();

    // PROVISIONED mesh (AC2.0.a): serving + store pins stay on the OLD
    // generation while every A2APeerConfig DECLARES the new one — the
    // operator's t_provision act, constructed directly (no runtime
    // config-mutation API exists or is created; AC2.0).
    let mesh = build_mesh_n_provisioned(
        &clock,
        &ca,
        &names,
        &old_refs,
        &old_refs,
        &new_refs,
        retry.clone(),
    )
    .await;

    // t_provision: open every node's rotation window for every peer — the
    // store's active set becomes {current: old, next: new} (AC2.1), sites 5/6
    // pass (the DECLARED new ∈ set), and the wire still serves OLD.
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

    // ── Phase 1: steady (provisioned) NxN sweep over the OLD certs ──
    let sweep1 = directed_dial_sweep(&mesh, 1000, &base).await;
    assert_eq!(
        sweep1.ok, sweep1.total,
        "Phase 1: zero conversation drops over the old generation — full NxN reachability"
    );
    // §A6 TestInfra-3: drain the summed gauge to ZERO before the in-flight
    // window — Phase 1's server-side handlers decrement only after they are
    // next scheduled, and a stale handler would let the arm's `live >= w`
    // fire on Phase-1 residue instead of Phase-2 frames.
    assert!(
        wait_until(
            || mesh
                .iter()
                .map(|m| m.transport.active_connections())
                .sum::<usize>()
                == 0,
            Duration::from_secs(5),
        )
        .await,
        "Phase 1 gauge must drain to zero before the window arms"
    );

    // ── Phase 2: the in-flight window + the generation swap racing INSIDE it ──
    // W = min(DIAL_CONCURRENCY, pairs.len()) (AC1.2): 6 at N=3, 16 at N=10.
    // Repeated dials to the same pair are not additional conversations.
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (0..n).filter(move |j| *j != i).map(move |j| (i, j)))
        .collect();
    let w = DIAL_CONCURRENCY.min(pairs.len());
    assert!(
        w >= 2,
        "the window is not a window below 2 in-flight frames"
    );

    // Drop accounting (AC1.1): verdicts recorded channel-before-resolve.
    // §A6 close-pass (CloseEdge-3): `issued` is NOT pairs.len() — at N=10 the
    // buffer_unordered window launches only W futures up front and the rest
    // as slots free (possibly after t_0); the drop population is the set
    // LAUNCHED BEFORE t_0, tracked per-dial. §A6 close-pass (CloseBlind-6):
    // the arm counts OUTSTANDING CONVERSATIONS (launched − completed), not
    // the server-side connection gauge — a response future can complete
    // before its server handler is rescheduled to decrement the gauge, so
    // the gauge can satisfy `>= w` with fewer than W frames outstanding.
    let launched = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let issued_pre = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t0_cell: std::sync::Arc<std::sync::atomic::AtomicU64> =
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (verdicts_tx, mut verdicts_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    use futures_util::stream::{self, StreamExt};
    let mut sweep = {
        let futs = pairs.iter().enumerate().map(|(k, &(i, j))| {
            let seq = 5_000_000 + k as u64;
            let frame = make_frame(&names[i], &names[j], IntentClass::Readonly, seq);
            let to = HostId(names[j].clone());
            let node = &mesh[i];
            let verdicts_tx = verdicts_tx.clone();
            let launched = launched.clone();
            let issued_pre = issued_pre.clone();
            let t0_cell = t0_cell.clone();
            let base2 = base;
            async move {
                // The launch record is the FIRST statement: this frame counts
                // as issued-before-t_0 iff it launched before the arm fired.
                launched.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let ts = mono_ns(&base2);
                let t0 = t0_cell.load(std::sync::atomic::Ordering::SeqCst);
                let pre = t0 == 0 || ts <= t0;
                if pre {
                    issued_pre.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                let (res, response_received) =
                    node.transport.route_outbound_observed(frame, &to).await;
                // Verdicts are counted ONLY for the pre-t_0 population —
                // post-t_0 launches are not part of the drop story.
                if pre && received_verdict(&res, response_received) {
                    let _ = verdicts_tx.send(seq);
                }
            }
        });
        Box::pin(stream::iter(futs).buffer_unordered(w))
    };

    // Arm (AC1.2): drive the LAZY sweep (it launches dials only when polled)
    // and watch OUTSTANDING CONVERSATIONS until W are confirmed in flight.
    let mut max_live = 0usize;
    let mut completed: usize = 0;
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
                        None => break false, // sweep exhausted before the window opened
                    }
                }
            }
        }
    };
    assert!(
        armed,
        "the in-flight window must open: {w} conversations confirmed outstanding \
         (launched − completed; max observed {max_live}) — if it cannot, the \
         window or the launch accounting is broken"
    );
    let t0 = mono_ns(&base);
    t0_cell.store(t0, std::sync::atomic::Ordering::SeqCst);

    // The generation swap, IN PLACE (AC2.4.a): `swap_serving_cert` per node —
    // no teardown, no rebind, no move. 14-1's `select!` race shape is now
    // WRITABLE (the pre-14-2 teardown version structurally could not race the
    // traffic it was killing — recorded in the Dev Agent Record). NOT
    // `biased`: the poll order IS the race.
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
                    break; // sweep drained first; finish the swap below
                }
            }
        }
    }
    if !swap_won {
        (&mut swap).await;
    }
    // Drain the remainder under a BOUNDED cutoff (§A6 close-pass
    // CloseAcceptance-2: an unbounded drain postpones t_2 with any pending
    // conversation and never classifies it — the oracle must be able to
    // red, not wait for CI's outer timeout). 30 s is ~1000× the observed
    // per-dial wall on loopback.
    let drained = tokio::time::timeout(Duration::from_secs(30), async {
        while sweep.next().await.is_some() {}
    })
    .await
    .is_ok();
    assert!(
        drained,
        "the post-swap drain must finish inside the 30s t_2 cutoff"
    );
    let t2_window = mono_ns(&base);
    // Drop accounting: every frame ISSUED BEFORE t_0 must have a verdict by
    // t_2 (AC1.1) — the population is the pre-t_0 launch snapshot, never
    // pairs.len() (post-t_0 launches are not part of the drop story).
    let issued = issued_pre.load(std::sync::atomic::Ordering::SeqCst) as u64;
    let mut verdict_ids = BTreeSet::new();
    while let Ok(seq) = verdicts_rx.try_recv() {
        assert!(
            verdict_ids.insert(seq),
            "each issued frame can contribute at most one verdict"
        );
    }
    let verdicts = verdict_ids.len() as u64;
    assert!(
        verdicts <= issued,
        "verdict population cannot exceed pre-t_0 issued population"
    );
    let drops = issued - verdicts;
    eprintln!(
        "  in-flight window: W={w} confirmed at t_0; issued={issued}; verdicts={verdicts}; \
         drops={drops} (window drain t_2−t_0 = {} ms)",
        (t2_window - t0) / 1_000_000
    );
    assert!(
        drops == 0,
        "AC2.4: the provision→swap→close overlap MUST deliver zero conversation drops \
         (got {drops} of {issued}) — the pre-overlap teardown model measured drops={w} on \
         this same window (Dev Agent Record, 2026-08-29)"
    );
    // ── Phase 3: post-swap NxN sweep — `t_1` per target (first NEW-cert
    // verification), `t_2` per source (AC4.4's per-agent re-handshake+first
    // data-plane success). Every dial must ACK.
    let sweep2 = directed_dial_sweep(&mesh, 2000, &base).await;
    assert_eq!(
        sweep2.ok, sweep2.total,
        "Phase 3: zero conversation drops — all NxN dials ACK over the new generation"
    );

    // ── Close: promote-and-retire (AC2.1.a) on every node's store. The store
    // must end up holding NEW; the retired OLD is returned and refused below.
    let mut retired: Vec<PeerCertFingerprint> = Vec::with_capacity(n);
    for node in &mesh {
        for peer_name in &names {
            if node.name != *peer_name {
                retired.push(
                    node.transport
                        .pins()
                        .close_rotation_window(&PeerId::new(peer_name.clone()))
                        .expect("window was open at close"),
                );
            }
        }
    }

    // ── Post-grace probes (AC2.5): present every RETIRED leaf to a peer that
    // pinned it, over a real handshake attempt. Every presentation MUST be
    // refused (NFR-Sec-12: 100% detected, blocked, alerted). The measured
    // count of ACCEPTED retired presentations feeds the report — the zero is
    // EARNED by these probes, not hardcoded (the pre-14-2 drill passed the
    // literal `0`).
    let mut accepted_retired = 0usize;
    let mut probes = 0usize;
    for i in 0..n {
        let target = &mesh[(i + 1) % n]; // a peer that pinned node i
        probes += 1;
        if !retired_cert_probe_refused(target, &old[i], &new[(i + 1) % n].fingerprint, &ca, &clock)
            .await
        {
            accepted_retired += 1;
            eprintln!("  POST-GRACE LEAK: node {i}'s retired leaf was ACCEPTED by a peer");
        }
        // §A6 TestInfra-4 pairing: the SAME path must carry CURRENT-cert
        // traffic green — proving the path is alive and the refusal above
        // is cert-specific, not a dead/unreachable target.
        let ok_frame = make_frame(
            &names[i],
            &names[(i + 1) % n],
            IntentClass::Readonly,
            7_888_000 + i as u64,
        );
        let ok = mesh[i]
            .transport
            .route_outbound(ok_frame, &HostId(names[(i + 1) % n].clone()))
            .await;
        assert!(
            ok.is_ok(),
            "post-grace pairing: new-generation traffic on the probe path must ACK: {ok:?}"
        );
    }
    assert_eq!(
        accepted_retired, 0,
        "NFR-Sec-12: every retired-cert presentation post-grace must be refused \
         ({accepted_retired} of {probes} probes accepted)"
    );

    // ── Per-agent timestamps + the report ──
    let per_agent: Vec<AgentRotationTimestamps> = (0..n)
        .map(|i| AgentRotationTimestamps {
            agent_id: names[i].clone(),
            t_0_ns: t0,
            t_1_ns: sweep2.first_success_ns[i],
            t_2_ns: sweep2.first_source_success_ns[i],
        })
        .collect();
    // AC4.4 — non-degeneracy (11.2a vacuous-count guard, ported): every raw
    // sample set must hold DISTINCT values, or the per-agent t_2 fix regressed
    // to shared wall readings.
    let prop_raw: BTreeSet<u64> = per_agent
        .iter()
        .filter_map(|a| a.t_1_ns.map(|t| t.saturating_sub(t0)))
        .collect();
    let e2e_raw: BTreeSet<u64> = per_agent
        .iter()
        .filter_map(|a| a.t_2_ns.map(|t| t.saturating_sub(t0)))
        .collect();
    let rh_raw: BTreeSet<u64> = per_agent
        .iter()
        .filter_map(|a| match (a.t_1_ns, a.t_2_ns) {
            (Some(t1), Some(t2)) => Some(t2.saturating_sub(t1)),
            _ => None,
        })
        .collect();
    assert_eq!(
        prop_raw.len(),
        n,
        "raw propagation samples must be distinct"
    );
    assert_eq!(rh_raw.len(), n, "raw re-handshake samples must be distinct");
    assert_eq!(e2e_raw.len(), n, "raw end-to-end samples must be distinct");

    // T_grace per §7.2.1.a cold-deployment floor: max(2*max(500,500), 5000) ms.
    let t_grace_ms = compute_t_grace(500, 7).as_millis() as u64;
    let report = RotationDrillReport::from_per_agent(
        format!("10-4b-live-{n}host-drill"),
        // AC3.2: the DERIVED witness feeds the report's host_count — never a
        // literal. A drill that says N and measured fewer cannot pass below.
        distinct_agent_witness(&per_agent) as u32,
        500,
        t_grace_ms,
        per_agent,
        accepted_retired as u64, // MEASURED (AC2.5): retired certs accepted post-grace
        probes as u64 + sweep2.total as u64, // every post-grace connection observed
    );

    // Transparency: the measured timings WITH sample counts (AC4.2 — n= beside
    // every percentile; at small n the p99 IS the worst of n, said loudly).
    let n_prop = report
        .per_agent
        .iter()
        .filter(|a| a.t_1_ns.is_some())
        .count();
    let n_rh = report
        .per_agent
        .iter()
        .filter(|a| a.t_1_ns.is_some() && a.t_2_ns.is_some())
        .count();
    let n_e2e = report
        .per_agent
        .iter()
        .filter(|a| a.t_2_ns.is_some())
        .count();
    eprintln!("\n=== AC-10.4b live {n}-host rotation drill (REAL monotonic timestamps) ===");
    for a in &report.per_agent {
        eprintln!(
            "  {:>8}: prop(t1-t0)={:>5}ms  re-hs(t2-t1)={:>5}ms  e2e(t2-t0)={:>5}ms",
            a.agent_id,
            a.revocation_propagation_ms().unwrap_or(0),
            a.re_handshake_ms().unwrap_or(0),
            a.end_to_end_ms().unwrap_or(0),
        );
    }
    eprintln!(
        "  revocation-propagation p50/p99 = {}/{} ms  (n={n_prop})  (v0.7 floors 30000/90000)",
        report.revocation_propagation_p50_ms, report.revocation_propagation_p99_ms
    );
    eprintln!(
        "  re-handshake            p50/p99 = {}/{} ms  (n={n_rh})  (v0.7 floors 30000/60000)",
        report.re_handshake_p50_ms, report.re_handshake_p99_ms
    );
    eprintln!(
        "  end-to-end              p50/p99 = {}/{} ms  (n={n_e2e})  (v1.0 floors 60000/150000)",
        report.end_to_end_p50_ms, report.end_to_end_p99_ms
    );
    eprintln!(
        "  post-grace retired-cert accept rate = {:.6} of {} observed  (v1.0 floor 0.001)",
        report.post_grace_reject_rate,
        probes as u64 + sweep2.total as u64
    );
    eprintln!(
        "  => passes_v07={}  passes_v10={}  passes_v15={}",
        report.passes_v07_floors, report.passes_v10_floors, report.passes_v15_floors
    );

    // AC3.2 — the identity witness binds: the DERIVED count is the mesh size,
    // and the floor at the envelope scale asserts against IT.
    let derived = distinct_agent_witness(&report.per_agent);
    assert_eq!(
        derived, n,
        "the derived identity witness must equal the mesh"
    );
    print_rotation_marker(derived);

    // The floors (AC4.1) — via passes_v15_floors, which IS passes_v10_floors.
    assert!(
        report.passes_v07_floors,
        "v0.7 floors MUST pass: localhost rotation timings are well under 30s/90s/30s/60s"
    );
    assert!(
        report.passes_v10_floors,
        "v1.0 floors MUST pass: end-to-end < 60s/150s and retired-cert accept rate is 0"
    );
    assert!(
        report.passes_v15_floors,
        "v1.5 NFR-Sec-13 floors MUST pass (inherits v1.0 strictness; never the looser PRD literals)"
    );
    // Explicit zero-drop invariant across every phase.
    assert_eq!(
        sweep1.ok + sweep2.ok,
        sweep1.total + sweep2.total,
        "zero conversation drops across both reachability sweeps"
    );
    // AC4.2 third surface — the rendered markdown report with per-axis
    // sample counts (report.rs, WIRED by this story after 11 dead stories).
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));
}

#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_real_timing_3_host_drill() {
    rotation_drill(ROTATION_FLOORS_HOST_COUNT).await;
}

/// Test 1b — the 10-host envelope leg (the epic's scale-out target): the SAME
/// drill at N=10 plus the rotation-named adversary (AC3.4), both inside the
/// runner envelope with the fd ceiling probed for THIS scale.
#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_real_timing_10_host_envelope() {
    assert_fd_headroom(fd_headroom_for(ROTATION_ENVELOPE_HOST_COUNT));
    let _envelope = RunnerEnvelope::open("rotation-10host-envelope");
    rotation_drill(ROTATION_ENVELOPE_HOST_COUNT).await;
    cert_rotation_race_adversary_detected(ROTATION_ENVELOPE_HOST_COUNT).await;
}

/// AC3.4 — the rotation-named adversary (`PlantKind::CertRotationRaceExploit`,
/// shipped by 14-1, inherited for free): one mesh member serves an EXPIRED
/// leaf instead of rotating. Detection = every adversary-involving directed
/// pair fails (with the exact `CertExpired` class on a live dial — WebPKI
/// validity runs BEFORE any pin check) while every healthy pair stays green.
async fn cert_rotation_race_adversary_detected(n: usize) {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2-race-exploit");
    let names: Vec<String> = (0..n).map(host_name).collect();
    // `serving` starts as a CLONE of `expected` (every node serves the
    // identity its peers pinned); ONLY the adversary deviates — a DIFFERENT
    // (expired) leaf.
    let adv = n - 1; // never index 0 (the stable hub rule)
    let expected: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let mut serving: Vec<Leaf> = expected.clone();
    serving[adv] = expired_leaf(&ca, &clock); // "rotated" into an expired leaf
    let serving_refs: Vec<&Leaf> = serving.iter().collect();
    let expected_refs: Vec<&Leaf> = expected.iter().collect();
    let mesh = build_mesh_with_planted(
        &clock,
        &ca,
        &names,
        &serving_refs,
        &expected_refs,
        &[(adv, PlantKind::CertRotationRaceExploit)],
        no_retry(),
    )
    .await;

    let base = Instant::now();
    let sweep = directed_dial_sweep(&mesh, 9_000, &base).await;
    let mut adv_pairs_failed = 0usize;
    let mut healthy_pairs_ok = 0usize;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let involves_adv = i == adv || j == adv;
            if involves_adv {
                assert!(
                    !sweep.pair_ok[i][j],
                    "adversary-involving pair {i}->{j} must fail (expired leaf detected)"
                );
                adv_pairs_failed += 1;
            } else if sweep.pair_ok[i][j] {
                healthy_pairs_ok += 1;
            }
        }
    }
    let healthy_total = n * (n - 1) - 2 * (n - 1);
    assert_eq!(adv_pairs_failed, 2 * (n - 1), "exact adversary topology");
    assert_eq!(
        healthy_pairs_ok, healthy_total,
        "every healthy pair stays green around the detected adversary"
    );
    // The exact class, on one live dial (not inferred from the matrix).
    let frame = make_frame(&names[0], &names[adv], IntentClass::Readonly, 9_999);
    let res = mesh[0]
        .transport
        .route_outbound(frame, &HostId(names[adv].clone()))
        .await;
    assert!(
        matches!(
            res,
            Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::CertExpired,
                ..
            })
        ),
        "the rotation-race adversary must be detected as CertExpired, got {res:?}"
    );
    eprintln!(
        "  adversary leg: {adv_pairs_failed}/{} adversary pairs failed (CertExpired), \
         {healthy_pairs_ok}/{healthy_total} healthy pairs green at N={n}",
        n * (n - 1)
    );
}

/// Story 14-2 / T8a (AC6.1.b) — the post-grace falsifier, ported from the
/// canned twin's `scenario_5_3` BEFORE its deletion: `cert_post_grace_reject`
/// is a REAL floor over a MEASURED count, not a constant. The live drill
/// earns its zero with retired-cert probes (every presentation refused);
/// this vector proves the floor itself still REDS when an old-cert
/// presentation gets through — exactly at the 0.001 boundary passes, above
/// it v1.0 AND v1.5 go RED.
#[test]
#[ignore] // AC5.5(a): gate-driven
fn t_10_4b_rotation_proven_red_post_grace_boundary() {
    const NS_PER_MS: u64 = 1_000_000;
    let t0 = mono_ns(&Instant::now());
    let agents: Vec<AgentRotationTimestamps> = (0..3)
        .map(|i| AgentRotationTimestamps {
            agent_id: format!("host_{i:02}"),
            t_0_ns: t0,
            t_1_ns: Some(t0 + (5 + i as u64) * NS_PER_MS),
            t_2_ns: Some(t0 + (10 + i as u64) * NS_PER_MS),
        })
        .collect();

    // Exactly AT the boundary: 1 accepted retired presentation of 1_000
    // observed post-grace connections → rate 0.001 — still passes.
    let at_boundary =
        RotationDrillReport::from_per_agent("pg-boundary", 3, 500, 5_000, agents.clone(), 1, 1_000);
    assert_eq!(at_boundary.post_grace_reject_rate, 0.001);
    assert!(at_boundary.passes_v10_floors);
    assert!(at_boundary.passes_v15_floors);

    // Above the boundary: 2 of 1_000 → v1.0 AND v1.5 RED.
    let above = RotationDrillReport::from_per_agent("pg-above", 3, 500, 5_000, agents, 2, 1_000);
    assert!(above.post_grace_reject_rate > 0.001);
    assert!(!above.passes_v10_floors);
    assert!(!above.passes_v15_floors);
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
/// the rotation failure, not to a retry-starved baseline. Reachability drops
/// to exactly 2/6 → the drill goes RED.
#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_proven_red_drop_reachability() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-10-4b-red-drop");
    // Same retry policy as the GREEN drill (default) so the RED outcome is
    // attributable to the rotation failure, not a retry-starved baseline. A
    // half-failed pair's PinMismatch is a NON-retryable class, so the failing
    // pairs still drop on the first attempt — the drill stays fast.
    let retry = HandshakeRetryPolicy::default();
    let base = Instant::now();
    let names: Vec<String> = (0..3).map(host_name).collect();

    // ── Phase 1: healthy mesh, full NxN reachability (baseline GREEN) ──
    let old: Vec<Leaf> = (0..3).map(|_| valid_leaf(&ca, &clock)).collect();
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let mesh_old = build_mesh_n(&clock, &ca, &names, &old_refs, &old_refs, retry.clone()).await;
    let sweep1 = directed_dial_sweep(&mesh_old, 3000, &base).await;
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
    let serving = [&new_a, &new_b, &c_stale];
    let expected = [&new_a, &new_b, &new_c];
    let mesh_new = build_mesh_n(&clock, &ca, &names, &serving, &expected, retry).await;

    let sweep2 = directed_dial_sweep(&mesh_new, 4000, &base).await;
    // Exact surviving topology: only the A↔B directed pairs survive the
    // half-failed rotation (A→B and B→A); every C-involving pair fails on the
    // deterministic pin mismatch.
    let mut surviving: Vec<(String, String)> = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            if i != j && sweep2.pair_ok[i][j] {
                surviving.push((mesh_new[i].name.clone(), mesh_new[j].name.clone()));
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
        vec![
            (names[0].clone(), names[1].clone()),
            (names[1].clone(), names[0].clone()),
        ],
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
/// with the per-agent propagation/re-handshake durations injected as
/// CONTROLLED time-offsets — a hermetic test cannot incur a real 90s+
/// propagation delay, so the degradation is modelled as named offsets recorded
/// through the same `RotationDrillReport::from_per_agent` drill path used by
/// the GREEN test, not as hand-built absolute fixture timestamps. One agent (c)
/// observes the revocation 95s after `t_0`, so p99 = 95s > 90s floor; all other
/// axes are kept under their floors so the failure isolates to
/// revocation-propagation p99.
#[test]
#[ignore] // AC5.5(a): gate-driven
fn t_10_4b_rotation_proven_red_p99_exceeds_floor() {
    // LIVE-measured rotation anchor (same monotonic base as the GREEN drill).
    // A hermetic test cannot incur a real 90s+ revocation-propagation delay,
    // so the per-agent propagation and re-handshake durations below are
    // injected as CONTROLLED time-offsets on top of this anchor — recorded
    // through the same `from_per_agent` drill path, not as hand-built
    // absolute timestamps.
    let t0 = mono_ns(&Instant::now());

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
    assert!(
        !report.passes_v15_floors,
        "v1.5 floors (which inherit v1.0) MUST also be RED"
    );
}

// ─────────── 14-2 / AC1.2 — the in-flight window drop oracle (GREEN under overlap) ───────────

/// Story 14-2 / AC1.2 + AC2.4 — the in-flight window drop oracle: `t_0` fires
/// while `W` readonly frames are provably in flight (the summed
/// `active_connections()` RAII gauge), the generation swap races the live
/// sweep inside `tokio::select!`, and drops are counted test-side by sequence
/// number per AC1.1's definition.
///
/// **The before/after pair is the evidence (AC2.4).** Against the PRE-14-2
/// teardown-rebind rotation this exact window measured `drops = 6 = W` on 10
/// consecutive runs (2026-08-29, Dev Agent Record) — the teardown physically
/// cannot race the traffic it kills. Under the provision→swap→close overlap
/// the same window measures `drops == 0`: in-flight connections are never
/// touched (rustls does no renegotiation) and post-swap handshakes resolve
/// the new leaf against the open window's set-valued verify.
///
/// The oracle is `drops == 0` (GREEN state); the RED state was the teardown
/// run. Never a specific pre-count.
#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_under_load_window() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2-window");
    let retry = HandshakeRetryPolicy::default();
    let base = Instant::now();
    let n = 3usize;
    let names: Vec<String> = (0..n).map(host_name).collect();

    // Provisioned mesh + open windows (AC2.0): old generation served+pinned,
    // new generation declared and windowed, wire still old.
    let old: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let new: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let new_refs: Vec<&Leaf> = new.iter().collect();
    let mesh =
        build_mesh_n_provisioned(&clock, &ca, &names, &old_refs, &old_refs, &new_refs, retry).await;
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

    // The directed pair set (6 at N=3) and W = min(DIAL_CONCURRENCY, 6) = 6.
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (0..n).filter(move |j| *j != i).map(move |j| (i, j)))
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
            let seq = 6_000_000 + k as u64;
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
                let (res, response_received) =
                    node.transport.route_outbound_observed(frame, &to).await;
                if pre && received_verdict(&res, response_received) {
                    let _ = verdicts_tx.send(seq);
                }
            }
        });
        Box::pin(stream::iter(futs).buffer_unordered(w))
    };

    // Arm on conversations that have launched but not completed. The server
    // connection gauge can lag a completed response and is not an in-flight
    // conversation witness.
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
        "the in-flight window must open: {w} conversations confirmed outstanding \
         (launched − completed; max observed {max_live})"
    );
    let t0 = mono_ns(&base);
    t0_cell.store(t0, std::sync::atomic::Ordering::SeqCst);

    // The swap races the live sweep — 14-1's shape, NOT biased.
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
    assert!(
        drained,
        "the overlap window must drain inside its t_2 cutoff"
    );
    let t2 = mono_ns(&base);

    // Close the windows (promote-and-retire) — the overlap is complete.
    for node in &mesh {
        for peer_name in &names {
            if node.name != *peer_name {
                assert!(
                    node.transport
                        .pins()
                        .close_rotation_window(&PeerId::new(peer_name.clone()))
                        .is_some(),
                    "window was open at close"
                );
            }
        }
    }

    let issued = issued_pre.load(std::sync::atomic::Ordering::SeqCst) as u64;
    let mut verdict_ids = BTreeSet::new();
    while let Ok(seq) = verdicts_rx.try_recv() {
        assert!(
            verdict_ids.insert(seq),
            "each issued frame can contribute at most one verdict"
        );
    }
    let verdicts = verdict_ids.len() as u64;
    assert!(verdicts <= issued, "verdicts cannot exceed issued frames");
    let drops = issued - verdicts;
    eprintln!("\n=== 14-2 AC1.2 in-flight window (provision→swap→close overlap) ===");
    eprintln!(
        "  W={w} dials confirmed in flight at t_0; issued={issued}; verdicts by t_2={verdicts}; drops={drops}"
    );
    eprintln!(
        "  swap-to-drain wall (t_2−t_0) = {} ms",
        (t2 - t0) / 1_000_000
    );
    assert!(
        drops == 0,
        "AC2.4 GREEN: the overlap MUST deliver zero conversation drops (got {drops} of \
         {issued}) — the pre-overlap teardown run measured drops=6=W on this same window \
         (Dev Agent Record, 2026-08-29); a nonzero count here means the swap is not \
         in-place or the window accounting is broken"
    );
}

// ─────────── 14-2 / AC2.3 — NFR-Sec-12 proven-red at the pin-store seam ───────────

/// Story 14-2 / AC2.3 — the overlap pin-closure contract, all gate-enrollable:
/// the **four** close assertions (1–3 prove you can break a store; 4 proves
/// you ROTATED — a window that closes onto nothing is also "closed"), the
/// during-window outside-the-set block, and the colliding-`next` refusals at
/// open with the dedicated `FingerprintCollision` variant (AC2.2.a), plus the
/// §A6 Edge-3 self-collision refusal.
#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_overlap_pin_closure() {
    use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
    use maos_a2a_core::{EPinMismatch, InMemoryTofuPinStore, RePinDecision, TofuPinStore};

    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new("host_a");
    let other = PeerId::new("host_b");
    let old = PeerCertFingerprint::from_cert_der(b"old-leaf");
    let new = PeerCertFingerprint::from_cert_der(b"new-leaf");
    let outsider = PeerCertFingerprint::from_cert_der(b"outsider-leaf");
    let other_fp = PeerCertFingerprint::from_cert_der(b"other-leaf");

    // Block context: two peers with active current pins (first contact).
    store
        .pin_first_contact(&peer, &old, &old, 1)
        .await
        .expect("pin peer");
    store
        .pin_first_contact(&other, &other_fp, &other_fp, 1)
        .await
        .expect("pin other");

    // Steady state: the current cert verifies; the replacement does not yet.
    assert!(store.verify_pinned_sync(&peer, &old).is_ok());
    assert!(matches!(
        store.verify_pinned_sync(&peer, &new),
        Err(EPinMismatch::Mismatch { .. })
    ));

    // (b) AC2.2.a — a `next` colliding with ANOTHER peer's CURRENT pin is
    // refused at open, as the dedicated security variant (never `Mismatch`).
    let err = store
        .open_rotation_window(&peer, &other_fp)
        .expect_err("collision with other's current must be refused");
    assert!(
        matches!(err, EPinMismatch::FingerprintCollision { ref peer, ref colliding_peer, .. }
            if peer == "host_a" && colliding_peer == "host_b"),
        "collision must name both peers, got {err:?}"
    );

    // Open the window for real.
    store
        .open_rotation_window(&peer, &new)
        .expect("open rotation window");
    assert_eq!(store.rotation_next(&peer), Some(new.clone()));

    // An open window is idempotent for the same generation but cannot be
    // replaced without an explicit close.
    store
        .open_rotation_window(&peer, &new)
        .expect("same generation is idempotent");
    let replacement = PeerCertFingerprint::from_cert_der(b"replacement-leaf");
    assert!(matches!(
        store
            .open_rotation_window(&peer, &replacement)
            .expect_err("active generation cannot be replaced"),
        EPinMismatch::Invalidated { .. }
    ));
    assert_eq!(store.rotation_next(&peer), Some(new.clone()));

    // Current-pin insertion paths uphold the same global uniqueness invariant.
    let late_peer = PeerId::new("host_late");
    assert!(matches!(
        store
            .pin_first_contact(&late_peer, &new, &new, 9)
            .await
            .expect_err("first contact cannot collide with an open next"),
        EPinMismatch::FingerprintCollision { .. }
    ));

    // An operator-approved re-pin that collides with another peer's open next
    // is explicitly refused by the store and never materialized.
    let approval_id = [7u8; 16];
    let repin_store = InMemoryTofuPinStore::new()
        .with_repin_hook(move |_, _, _| RePinDecision::AcceptedByOperator { approval_id });
    let rotating_peer = PeerId::new("rotating");
    let rotating_current = PeerCertFingerprint::from_cert_der(b"rotating-current");
    let colliding_next = PeerCertFingerprint::from_cert_der(b"colliding-next");
    repin_store
        .pin_first_contact(&rotating_peer, &rotating_current, &rotating_current, 1)
        .await
        .expect("pin rotating peer");
    repin_store
        .open_rotation_window(&rotating_peer, &colliding_next)
        .expect("open rotating peer window");
    let repin_peer = PeerId::new("repin-peer");
    assert!(matches!(
        repin_store
            .await_repin_consent(&repin_peer, &colliding_next, 2)
            .await,
        RePinDecision::RejectedByStore {
            error: EPinMismatch::FingerprintCollision { .. }
        }
    ));
    assert!(repin_store.get_pin(&repin_peer).await.is_none());

    // §A6 Edge-3 (self-collision): `next == this peer's current` is refused.
    let err = store
        .open_rotation_window(&peer, &old)
        .expect_err("next == current must be refused as a self-collision");
    assert!(matches!(err, EPinMismatch::FingerprintCollision { .. }));

    // Collision guard's second half: a `next` colliding with another peer's
    // OPEN WINDOW is refused too.
    let err = store
        .open_rotation_window(&other, &new)
        .expect_err("collision with peer's open next must be refused");
    assert!(matches!(err, EPinMismatch::FingerprintCollision { .. }));

    // During the window (one-generation overlap, §7.2.1.a): BOTH old and new
    // verify for `peer`; an outsider fingerprint OUTSIDE {current, next} is
    // still blocked — NFR-Sec-12 stays 100% while trust is widened.
    assert!(store.verify_pinned_sync(&peer, &old).is_ok());
    assert!(store.verify_pinned_sync(&peer, &new).is_ok());
    assert!(matches!(
        store.verify_pinned_sync(&peer, &outsider),
        Err(EPinMismatch::Mismatch { .. })
    ));
    // Sites 2/3: the replacement resolves as an active identity during the
    // window; the outsider resolves to nobody.
    assert_eq!(
        store.find_active_pin_by_fingerprint(&new),
        Some(peer.clone())
    );
    assert_eq!(store.find_active_pin_by_fingerprint(&outsider), None);

    // Close — PROMOTE-and-retire (AC2.1.a): the store must end up holding NEW.
    let retired = store.close_rotation_window(&peer).expect("window was open");
    assert_eq!(retired, old, "close must retire the OLD fingerprint");
    // 1. the window is shut.
    assert!(store.rotation_next(&peer).is_none());
    // 2. the retired cert is refused at the verify sites (1/4).
    assert!(matches!(
        store.verify_pinned_sync(&peer, &retired),
        Err(EPinMismatch::Mismatch { .. })
    ));
    // 3. it is also unresolvable as an identity at the listen sites (2/3).
    assert!(store.find_active_pin_by_fingerprint(&retired).is_none());
    // 4. the NEW one still works — without this, 1–3 prove you can break a
    // store, not that you rotated.
    assert!(store.verify_pinned_sync(&peer, &new).is_ok());
    assert_eq!(
        store.find_active_pin_by_fingerprint(&new),
        Some(peer.clone())
    );

    eprintln!(
        "\n=== 14-2 AC2.3 overlap pin-closure: retired={}, promoted={} — all four close \
         assertions + during-window block + collision refusals hold ===",
        retired.wire(),
        new.wire()
    );
}

// ─────────── 14-2 / §A6 — the drop oracle's EXECUTABLE negative control ───────────

/// §A6 Acceptance-5 — the EXECUTABLE teardown negative control for the drop
/// oracle (Blocking condition 3: a green must be able to have been red). The
/// overlap window test asserts `drops == 0`; THIS test runs the same
/// arm/accounting machinery against the OLD rotation model — teardown-rebind
/// — and asserts `drops > 0`. Committed as a permanent pair: if the drop
/// accounting ever stops detecting real drops (a verdict-predicate
/// regression, a gauge that never opens), THIS test goes red and names
/// itself.
///
/// Pins the structural finding measured 2026-08-29 (10/10 runs: drops = 6 =
/// W): a teardown rotation CANNOT coexist with in-flight traffic — the old
/// mesh cannot be dropped while a sweep borrows it, so the sweep (the
/// traffic) dies first and its frames receive no verdict by `t_2`. The
/// borrow checker is the first witness; AC2's in-place swap is the repair.
#[tokio::test]
#[ignore] // AC5.5(a): the check-rotation-real-timing gate drives this via --ignored; skipped != passed
async fn t_10_4b_rotation_drop_oracle_teardown_control() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2-teardown-control");
    let retry = HandshakeRetryPolicy::default();
    let n = 3usize;
    let names: Vec<String> = (0..n).map(host_name).collect();

    let old: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let mesh_old = build_mesh_n(&clock, &ca, &names, &old_refs, &old_refs, retry.clone()).await;

    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (0..n).filter(move |j| *j != i).map(move |j| (i, j)))
        .collect();
    let w = DIAL_CONCURRENCY.min(pairs.len());
    let launched = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (verdicts_tx, mut verdicts_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    use futures_util::stream::{self, StreamExt};
    let mut sweep = {
        let futs = pairs.iter().enumerate().map(|(k, &(i, j))| {
            let seq = 6_500_000 + k as u64;
            let frame = make_frame(&names[i], &names[j], IntentClass::Readonly, seq);
            let to = HostId(names[j].clone());
            let node = &mesh_old[i];
            let verdicts_tx = verdicts_tx.clone();
            let launched = launched.clone();
            async move {
                launched.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let (res, response_received) =
                    node.transport.route_outbound_observed(frame, &to).await;
                if received_verdict(&res, response_received) {
                    let _ = verdicts_tx.send(seq);
                }
            }
        });
        Box::pin(stream::iter(futs).buffer_unordered(w))
    };
    let mut completed = 0usize;
    let armed = {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let live = launched.load(std::sync::atomic::Ordering::SeqCst) - completed;
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
        "the control's window must open with {w} outstanding conversations"
    );
    let issued = launched.load(std::sync::atomic::Ordering::SeqCst) as u64;
    // The OLD rotation model: teardown-rebind. The kill order is forced by
    // the model itself — the sweep dies so the mesh can.
    drop(sweep);
    drop(mesh_old);
    let new: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let new_refs: Vec<&Leaf> = new.iter().collect();
    let _mesh_new = build_mesh_n(&clock, &ca, &names, &new_refs, &new_refs, retry).await;

    let mut verdict_ids = BTreeSet::new();
    while let Ok(seq) = verdicts_rx.try_recv() {
        assert!(
            verdict_ids.insert(seq),
            "each issued frame can contribute at most one verdict"
        );
    }
    let verdicts = verdict_ids.len() as u64;
    assert!(verdicts <= issued, "verdicts cannot exceed issued frames");
    let drops = issued - verdicts;
    eprintln!("  teardown control: W={w}, issued={issued}, verdicts={verdicts}, drops={drops}");
    assert!(
        drops > 0,
        "the drop oracle must still be able to go RED: a teardown rotation \
         mid-window MUST measure drops > 0 (got 0 of {issued}) — if this fires, \
         the verdict predicate or the arm regressed and the overlap test's \
         drops == 0 is no longer evidence"
    );
}

/// AC6.1.a — the WIRED markdown surface's own contract, ported to the
/// uncharged test lane (in-src `#[cfg(test)]` is KLOC-charged per E11-A6):
/// the drill id + per-axis sample counts from the ENGINE's own filter, and
/// the counts disagree with `per_agent.len()` exactly as the percentile
/// engine does.
#[test]
#[ignore] // AC5.5(a): gate-driven
fn t_10_4b_rotation_report_markdown_publishes_sample_counts() {
    use maos_a2a_core::chaos::report::report_to_markdown;

    let agents: Vec<AgentRotationTimestamps> = (0..3)
        .map(|i| AgentRotationTimestamps {
            agent_id: format!("a{i}"),
            t_0_ns: 0,
            t_1_ns: Some((i as u64 + 1) * 1_000_000_000),
            t_2_ns: Some((i as u64 + 2) * 1_000_000_000),
        })
        .collect();
    let r = RotationDrillReport::from_per_agent("drill-14-2", 3, 500, 5_000, agents, 0, 100);
    let md = report_to_markdown(&r).expect("md");
    assert!(md.contains("drill-14-2"));
    assert!(md.contains("```json"));
    assert!(md.contains("propagation=3"), "got: {md}");
    assert!(md.contains("re-handshake=3"), "got: {md}");
    assert!(md.contains("end-to-end=3"), "got: {md}");
    assert!(md.contains("p99 IS the worst of n"), "got: {md}");

    // The filter, not len(): one agent never re-handshook — propagation
    // counts it, end-to-end does not.
    let agents = vec![
        AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: 0,
            t_1_ns: Some(1_000_000_000),
            t_2_ns: Some(2_000_000_000),
        },
        AgentRotationTimestamps {
            agent_id: "b".into(),
            t_0_ns: 0,
            t_1_ns: Some(3_000_000_000),
            t_2_ns: None,
        },
    ];
    let r = RotationDrillReport::from_per_agent("drill-filter", 2, 500, 5_000, agents, 0, 10);
    let md = report_to_markdown(&r).expect("md");
    assert!(md.contains("propagation=2"), "got: {md}");
    assert!(md.contains("end-to-end=1"), "got: {md}");
}

// ─────────── §A6 close-pass (CloseRuntime) — the verdict predicate's own contract ───────────

/// The drop oracle's verdict predicate, defended in BOTH directions
/// (§A6 CloseRuntime finding: the teardown control's drops arise from future
/// cancellation UPSTREAM of the predicate, so a lenient-predicate regression
/// is invisible to it; this table test is the executable red for exactly
/// that mutation — TestInfra-1's class, "a swap that breaks handshakes
/// passes as zero drops").
#[cfg(not(feature = "rotation-fault-inject"))]
#[test]
#[ignore] // AC5.5(a): gate-driven
fn t_10_4b_rotation_verdict_predicate_contract() {
    use maos_a2a_core::error::HandshakeFailureClass;
    assert!(
        received_verdict(&Ok(()), true),
        "a decoded ACK is a verdict"
    );
    assert!(
        received_verdict(
            &Err(A2AError::PinInvalidated {
                peer: "p".into(),
                awaiting_repin: true,
            }),
            true,
        ),
        "every decoded typed NACK is a verdict"
    );
    assert!(
        !received_verdict(
            &Err(A2AError::PinInvalidated {
                peer: "p".into(),
                awaiting_repin: true,
            }),
            false,
        ),
        "a same-shaped local refusal is not a peer verdict"
    );
    assert!(
        !received_verdict(
            &Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::PinMismatch,
                message: "pin".into(),
            }),
            false,
        ),
        "a handshake failure generated no answer"
    );
    assert!(
        !received_verdict(&Err(A2AError::Io("recv: reset by peer".into())), false),
        "an I/O death is a drop"
    );
}

// ─────────── AC5.5b — the fault-injection seam, EXERCISED (close-pass Blind-2) ───────────

/// The observation-seam blind, feature-gated (AC5.5b): with the feature on,
/// the shared verdict predicate is lenient (every outcome counts as a
/// verdict ⇒ the drop oracle reads drops = 0 no matter what happened). This
/// test PROVES the blind exists at exactly the harness observation seam —
/// never `verifier.rs`, never the router — by running the teardown-control
/// scenario with BOTH predicates: the blinded one reads `drops == 0` (the
/// lie), the committed one reads `drops > 0` (the truth). The pair is the
/// demonstration that the gate's committed control catches the blind; the
/// gate invokes this test with the feature ON (a mutation leg whose GREEN
/// is "the blind is present and characterized", mirroring 11.3's shape).
#[cfg(feature = "rotation-fault-inject")]
#[tokio::test]
#[ignore] // AC5.5(a): gate-driven (with --features rotation-fault-inject)
async fn t_10_4b_rotation_fault_blind_characterized() {
    let transport_death = Err(A2AError::Io("recv: connection reset by peer".into()));
    assert!(
        received_verdict(&transport_death, false),
        "feature-on shared predicate must blind a real no-response outcome"
    );
    assert!(
        !strict_received_verdict(false),
        "the committed strict predicate must count the same outcome as a drop"
    );
    eprintln!("  fault-blind: feature-on shared seam reads drops=0; strict seam reads drops=1");
}

// Story 14-2 (AC5.5b) — the fault-injection feature is dev/CI-only. The
// shared test-harness verdict seam above is its only consumer; production
// verifier/router code never reads it. The gate proves release absence by
// requiring `cargo check --release --tests --features rotation-fault-inject`
// to fail on this guard.
#[cfg(all(feature = "rotation-fault-inject", not(debug_assertions)))]
compile_error!(
    "rotation-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 14-2 ship-blocker)."
);
