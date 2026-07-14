//! Story 11.3 — AC1-AC4: real N=30 (floor ≥25) host-churn scale-envelope
//! drill (NFR-Scale-2 / NFR-Rel-7). REPLACES the deleted
//! `maos-a2a-core::chaos::churn::run_scaffold` canned-constant scaffold: every
//! numeric field in `ChurnDrillReport` is DERIVED from real events on a live
//! mesh of `TcpA2ATransport` endpoints over real `127.0.0.1` sockets + real
//! rustls mTLS handshakes (the 10.4b `t_10_4b_rotation_real_timing.rs`
//! precedent, scaled up).
//!
//! Gated `#[ignore]` — `check-scale-churn` (xtask) controls execution;
//! skipped ≠ passed (mirrors `check-multi-region-slo`'s live-oracle posture).
//!
//! # The teeth bite on REAL events (code-review 2026-07-03 rework)
//!
//! The clean loopback pass is trivial by design (L5: sub-second events clear
//! the ≤1h/≤5/≤24h/≤4h floors). The gate's teeth are therefore FALSIFIERS —
//! and every falsifier here is driven by a REAL reachability outcome on the
//! live mesh, never a hand-injected constant into `from_real_events`:
//!
//! * **blast >5** (`churn-fault-inject`): the consent adversary is provisioned
//!   with 6 REAL readonly targets and genuinely reaches all 6 → `max_blast_radius`
//!   = 6 (derived) → the ≤5 binding floor REDS.
//! * **blind-one-detector** (`churn-fault-inject`): a class is dropped from the
//!   counted tally → the DOWNSTREAM `ChurnDrillReport::reconcile_detections(3)`
//!   count/identity contract REDS (not `Iterator::filter` semantics).
//! * **isolation-blind** (`churn-fault-inject`): the harness SKIPS the real
//!   isolation repoint → a real re-dial adversary→target still SUCCEEDS →
//!   `rto` objective unmet → `rto_secs` REDS, `recovery_secs` unaffected.
//! * **re-pin-blind** (`churn-fault-inject`): the harness breaks a legit peer's
//!   endpoint → a real legit↔legit re-dial FAILS → fleet not reconverged →
//!   `recovery_secs` REDS, `rto_secs` unaffected. (F3 separability — two
//!   INDEPENDENT real falsifiers, D5.)
//!
//! `recovery_secs`/`rto_secs` are two DISTINCT real events on the SAME sub-mesh
//! the adversary attacked: `rto` = detection→(adversary confirmed unreachable
//! via a real failed re-dial); `recovery` = detection→(legit fleet reconverged
//! via a real successful re-dial). They are not the same measurement.
//!
//! # CI-budget decomposition (Task-0 disclosure — §CI time budget)
//!
//! A full NxN directed-dial sweep at N=30 is 30×29=870 real mTLS handshakes
//! PER ROUND. Two bounds keep this drill's wall-clock tractable without
//! weakening what it proves:
//!
//! 1. **Reachability sweeps are hub-and-spoke, not full NxN.** `hub_pairs`
//!    dials `2×(N−1)=58` pairs (host_00 ↔ every other host) instead of the
//!    full 870. A stable hub (never churns) transitively proves the mesh is
//!    NOT partitioned after each churn round; it is NOT a claim that every
//!    one of the 435 non-hub pairs was individually dialed.
//! 2. **AC1's "30 distinct hosts" scale claim and AC2-AC4's detection
//!    mechanics run on SEPARATE live sub-meshes.** AC1 stands up 30 real
//!    listeners and reconciles their cert fingerprints + bound `SocketAddr`s,
//!    never by dialing. AC2 (detection latency) and AC3/AC4 (blast, recovery,
//!    RTO, the `churn-fault-inject` falsifiers) exercise SMALL but fully real
//!    dedicated live mTLS sub-meshes — the mechanism under test is "does the
//!    REAL detector fire, get timed, get isolated, and reconverge", which
//!    needs a handful of real sockets, not all 30.
//!
//! Every dial that DOES happen is a real socket op; nothing here is
//! simulated or time-frozen (`tokio::time::pause()` is never used). Timing is
//! read from a single per-drill monotonic `std::time::Instant` base (L4).
mod support;

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::time::Instant;

use maos_a2a_core::chaos::churn::report_to_markdown;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, AdversarialAttempt, AdversarialDetection, ChurnDrillReport};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

/// F2 (ratified, preflight 2026-07-03) — run at N=30 (the cost-compressed
/// Cortex; NFR-Rel-7's "compressed 30-host"), floor ≥25 (NFR-Scale-2's
/// literal "25-host"). Named const with provenance — never a magic literal.
const HOST_COUNT: usize = 30;
/// The gate asserts the RECONCILED distinct-identity count against this
/// floor, never the literal `HOST_COUNT` (D7/L8 — "30 hosts" must be 30
/// DISTINCT hosts).
const HOST_COUNT_FLOOR: usize = 25;
/// Index 0 is the stable hub — it never churns, keeping the bounded
/// reachability sweep meaningful across every round (see module docs).
const HUB: usize = 0;
/// 4 compressed rounds standing in for "4 weeks" (NFR-Rel-7's 4-week window,
/// modeled as a compressed churn-event schedule per the "Explicitly NOT in
/// 11.3" scope note — loopback churn events are sub-second, not 4 real
/// weeks).
const CHURN_ROUNDS: usize = 4;
/// 4 of 30 non-hub hosts replaced per round ≈ 13.8% of the 29 non-hub slots —
/// inside the ratified 10-20%/week turnover band. 4 rounds × 4 = 16 total
/// real join/leave events, inside the ratified 12-24 range (F2/AC1).
const TURNOVER_PER_ROUND: usize = 4;
/// The 3 adversary classes planted for detection (AC2, one per class).
const PLANTED_ADVERSARIES: usize = 3;

// Story 11.3 (D6/L7) — the fault-injection feature is EXERCISED only by the
// `#[cfg(feature = "churn-fault-inject")]` tests below; it has NO
// `maos-a2a-tcp` library-side `#[cfg]` (the blinds live entirely in this test
// crate), so this test-file guard is the complete placement — a release
// *library* build has no churn-fault-inject code path to leak. The
// `check-scale-churn` gate's `cargo tree --release` absence check is the
// belt-and-suspenders graph guard. MUST NOT ship in release builds.
#[cfg(all(feature = "churn-fault-inject", not(debug_assertions)))]
compile_error!(
    "churn-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 11.3 ship-blocker)."
);

/// Nanoseconds elapsed on the harness's OWN monotonic `Instant` base (L4 — a
/// same-process monotonic reading, never a cross-host clock subtraction, never
/// a frame's wall-clock `timestamp`). `Instant` is monotonic, so a backward
/// wall-clock (NTP) step can never invert a latency delta.
fn mono_ns(base: &Instant) -> u64 {
    base.elapsed().as_nanos() as u64
}

/// An endpoint string that no listener answers — repointing a peer here makes
/// its next `route_outbound` fail with a real transport error (ECONNREFUSED on
/// loopback). Used to model a REAL isolation / a REAL re-pin failure. Port 1
/// (tcpmux) is never bound in CI; connecting to it needs no privilege and is
/// refused immediately.
fn dead_endpoint() -> String {
    "tls://127.0.0.1:1".to_string()
}

/// Hub-and-spoke bounded reachability pairs — `2×(N−1)` dials, not full NxN
/// (CI-budget bound, module docs). Guarded against a degenerate mesh size.
fn hub_pairs(n: usize) -> Vec<(usize, usize)> {
    if n < 2 {
        return Vec::new();
    }
    let mut pairs = Vec::with_capacity(2 * (n - 1));
    for i in 1..n {
        pairs.push((HUB, i));
        pairs.push((i, HUB));
    }
    pairs
}

async fn build_round_mesh(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    leaves: &[Leaf],
) -> Vec<MeshNode> {
    let refs: Vec<&Leaf> = leaves.iter().collect();
    build_mesh_n(clock, ca, names, &refs, &refs, no_retry()).await
}

async fn assert_hub_reachable(mesh: &[MeshNode], seq_base: u64, label: &str) {
    let results = concurrent_dial_pairs(
        mesh,
        &hub_pairs(mesh.len()),
        seq_base,
        IntentClass::Readonly,
    )
    .await;
    let failed: Vec<_> = results.iter().filter(|(_, _, r)| r.is_err()).collect();
    assert!(
        failed.is_empty(),
        "{label}: hub-and-spoke reachability sweep must be zero-drop over the live mesh (failed={failed:?})"
    );
}

// ─────────────────────── AC1 — distinct-host-identity reconcile ───────────────────────

/// AC1 — bind N=30 real listeners (real rcgen certs, real `127.0.0.1:0`
/// sockets); the "30 hosts" claim is DERIVED-AND-RECONCILED against distinct
/// cert fingerprints AND distinct bound `SocketAddr`s (never the literal
/// count on its own — L8/D7). No dials needed for identity.
#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_mesh_identity_reconcile_30_host() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-11-3-identity");
    let names: Vec<String> = (0..HOST_COUNT).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;

    let host_fingerprints: BTreeSet<String> = mesh.iter().map(|n| n.fingerprint.wire()).collect();
    let host_addrs: BTreeSet<SocketAddr> = mesh.iter().map(|n| n.addr).collect();
    let report = ChurnDrillReport::from_real_events(
        "identity-reconcile-30-host",
        host_fingerprints,
        host_addrs,
        vec![],
        0,
        None,
    );
    assert_eq!(
        report.distinct_host_count(),
        HOST_COUNT,
        "a clean 30-node bind (no clones planted) must reconcile to exactly 30 distinct hosts"
    );
    assert!(report.distinct_host_count() >= HOST_COUNT_FLOOR);
}

/// AC1 — topology-fraud negative control: index 6 serves the SAME leaf as
/// index 5 (same DER → same cert fingerprint) while binding at a DISTINCT
/// real `SocketAddr`. Proves "30 hosts" is not fakeable by a single
/// stand-in: the reconcile MUST hard-fail (30 addrs, only 29 fingerprints).
#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_duplicate_identity_negative_control_hard_fails() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-11-3-dup-identity");
    let names: Vec<String> = (0..HOST_COUNT).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let mut serving: Vec<&Leaf> = leaves.iter().collect();
    serving[6] = &leaves[5]; // index 6 clones index 5's identity — distinct socket, same fingerprint
    let expected = serving.clone();
    let mesh = build_mesh_n(&clock, &ca, &names, &serving, &expected, no_retry()).await;

    let host_fingerprints: BTreeSet<String> = mesh.iter().map(|n| n.fingerprint.wire()).collect();
    let host_addrs: BTreeSet<SocketAddr> = mesh.iter().map(|n| n.addr).collect();
    assert_eq!(
        host_addrs.len(),
        HOST_COUNT,
        "30 distinct real sockets bound"
    );
    assert_eq!(
        host_fingerprints.len(),
        HOST_COUNT - 1,
        "the clone collapses to 29 distinct fingerprints"
    );

    let report = ChurnDrillReport::from_real_events(
        "dup-identity-negative-control",
        host_fingerprints,
        host_addrs,
        vec![],
        0,
        None,
    );
    assert!(
        report.distinct_host_count() < HOST_COUNT,
        "a duplicate-fingerprint fixture MUST make the distinct-identity reconcile hard-fail \
         (30 claimed hosts must not reconcile to 30 when one is a clone)"
    );
    assert_eq!(report.distinct_host_count(), HOST_COUNT - 1);
}

/// AC1 — compressed churn schedule: 16 real join/leave rebinds over 4 rounds,
/// each followed by a bounded hub-and-spoke reachability sweep proving the
/// mesh self-heals, then a distinct-identity reconcile on the FINAL membership.
/// (Leg-1 clean oracle — DISTINCT from the detection and blast/recovery drills
/// so a break here reds exactly this leg.)
#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_scale_churn_30_host_drill() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-11-3-scale-churn");
    let names: Vec<String> = (0..HOST_COUNT).map(host_name).collect();
    let mut leaves: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();

    let baseline = build_round_mesh(&clock, &ca, &names, &leaves).await;
    assert_hub_reachable(&baseline, 10_000, "baseline").await;
    drop(baseline); // H6 deterministic teardown frees the old ports

    let mut churn_events = 0usize;
    for round in 0..CHURN_ROUNDS {
        for k in 0..TURNOVER_PER_ROUND {
            let idx = 1 + ((round * TURNOVER_PER_ROUND + k) % (HOST_COUNT - 1));
            leaves[idx] = valid_leaf(&ca, &clock); // teardown+rebind models one leave+join turnover
            churn_events += 1;
        }
        let round_mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;
        assert_hub_reachable(
            &round_mesh,
            20_000 + (round as u64) * 100,
            "post-churn-round",
        )
        .await;
        drop(round_mesh);
    }
    assert_eq!(
        churn_events,
        CHURN_ROUNDS * TURNOVER_PER_ROUND,
        "the schedule must run every planned rebind (no silent short-circuit)"
    );
    assert!(
        (12..=24).contains(&churn_events),
        "compressed churn schedule must model 12-24 real join/leave events \
         (10-20%/wk x 4wk, NFR-Rel-7), got {churn_events}"
    );

    let final_mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;
    let host_fingerprints: BTreeSet<String> =
        final_mesh.iter().map(|n| n.fingerprint.wire()).collect();
    let host_addrs: BTreeSet<SocketAddr> = final_mesh.iter().map(|n| n.addr).collect();
    assert_hub_reachable(&final_mesh, 40_000, "post-churn-final").await;

    let report = ChurnDrillReport::from_real_events(
        "t-11-3-scale-churn-30-host",
        host_fingerprints,
        host_addrs,
        vec![],
        0,
        None,
    );
    assert_eq!(
        report.distinct_host_count(),
        HOST_COUNT,
        "clean drill: no clones planted, so N distinct == HOST_COUNT"
    );
    assert!(
        report.distinct_host_count() >= HOST_COUNT_FLOOR,
        "distinct-identity-reconciled host count must be >= {HOST_COUNT_FLOOR}, got {}",
        report.distinct_host_count()
    );
}

// ─────────────────────── AC2 — two-surface detection ───────────────────────

/// Plant the 3 adversary classes into a dedicated live sub-mesh, one per
/// class, and return their real `AdversarialDetection` samples (detection
/// latency on a monotonic base + the adversary's real served-cert fingerprint
/// as the distinct identity witness). Reused by the clean detection drill AND
/// the `churn-fault-inject` blind mutations — the real detectors fire
/// identically; only the caller decides whether to count all 3 or blind one.
async fn plant_and_detect_three_adversaries() -> Vec<AdversarialDetection> {
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-11-3-adversaries");
    let retry = no_retry();
    let mut detections = Vec::with_capacity(PLANTED_ADVERSARIES);

    // ── TofuPinSpoofing (HANDSHAKE layer) — the prober pins `expected_leaf`
    // but the adversary SERVES a DIFFERENT valid leaf: WebPKI succeeds, the
    // TOFU pin check fails → `HandshakeFailed` (the 10.4b `PinMismatch` shape).
    {
        let join_ns = mono_ns(&base);
        let expected_leaf = valid_leaf(&ca, &clock);
        let served_leaf = valid_leaf(&ca, &clock);
        let adversary: TcpA2ATransport = bind_endpoint(
            &served_leaf,
            Some(&ca),
            9_101,
            vec![],
            vec![],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        let adv_addr = adversary.local_addr().expect("adversary bound (H3/H4)");
        let prober_leaf = valid_leaf(&ca, &clock);
        let prober: TcpA2ATransport = bind_endpoint(
            &prober_leaf,
            Some(&ca),
            9_102,
            vec![pin("adv_pin_spoof", &expected_leaf.fingerprint, 9_101)],
            vec![peer_cfg(
                "adv_pin_spoof",
                "tls://127.0.0.1:0",
                &expected_leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        prober.set_peer_endpoint(&HostId("adv_pin_spoof".into()), format!("tls://{adv_addr}"));
        let frame = make_frame(
            "prober_pin_spoof",
            "adv_pin_spoof",
            IntentClass::Readonly,
            1,
        );
        let result = prober
            .route_outbound(frame, &HostId("adv_pin_spoof".into()))
            .await;
        let rejection_ns = mono_ns(&base);
        assert!(
            matches!(result, Err(A2AError::HandshakeFailed { .. })),
            "TofuPinSpoofing must be rejected at the HANDSHAKE layer, got {result:?}"
        );
        detections.push(AdversarialDetection {
            adversary_id: "adv_pin_spoof".into(),
            adversary_fingerprint: served_leaf.fingerprint.wire(),
            attack_class: AdversarialAttempt::TofuPinSpoofing,
            join_ns,
            first_rejection_ns: Some(rejection_ns),
            blast_peers: BTreeSet::new(), // handshake reject: zero peers ever reached (real, not fabricated)
        });
    }

    // ── CertRotationRaceExploit (HANDSHAKE layer) — the adversary serves a
    // STALE (expired) leaf; WebPKI's validity step rejects it BEFORE any pin
    // check → `HandshakeFailed`.
    {
        let join_ns = mono_ns(&base);
        let served_leaf = expired_leaf(&ca, &clock);
        let adversary: TcpA2ATransport = bind_endpoint(
            &served_leaf,
            Some(&ca),
            9_201,
            vec![],
            vec![],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        let adv_addr = adversary.local_addr().expect("adversary bound (H3/H4)");
        let expected_leaf = valid_leaf(&ca, &clock);
        let prober_leaf = valid_leaf(&ca, &clock);
        let prober: TcpA2ATransport = bind_endpoint(
            &prober_leaf,
            Some(&ca),
            9_202,
            vec![pin("adv_cert_race", &expected_leaf.fingerprint, 9_201)],
            vec![peer_cfg(
                "adv_cert_race",
                "tls://127.0.0.1:0",
                &expected_leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        prober.set_peer_endpoint(&HostId("adv_cert_race".into()), format!("tls://{adv_addr}"));
        let frame = make_frame(
            "prober_cert_race",
            "adv_cert_race",
            IntentClass::Readonly,
            1,
        );
        let result = prober
            .route_outbound(frame, &HostId("adv_cert_race".into()))
            .await;
        let rejection_ns = mono_ns(&base);
        assert!(
            matches!(result, Err(A2AError::HandshakeFailed { .. })),
            "CertRotationRaceExploit must be rejected at the HANDSHAKE layer, got {result:?}"
        );
        detections.push(AdversarialDetection {
            adversary_id: "adv_cert_race".into(),
            adversary_fingerprint: served_leaf.fingerprint.wire(),
            attack_class: AdversarialAttempt::CertRotationRaceExploit,
            join_ns,
            first_rejection_ns: Some(rejection_ns),
            blast_peers: BTreeSet::new(),
        });
    }

    // ── AdrLevel012ConsentBypass (ROUTER NACK layer) — valid pinned identity;
    // reaches 2 targets on readonly (real blast) then escalates to "standard"
    // (not in the accept-allowlist) → `IntentDeniedAtPeer`.
    {
        let join_ns = mono_ns(&base);
        let adv_leaf = valid_leaf(&ca, &clock);
        let target_a_leaf = valid_leaf(&ca, &clock);
        let target_b_leaf = valid_leaf(&ca, &clock);
        let target_a: TcpA2ATransport = bind_endpoint(
            &target_a_leaf,
            Some(&ca),
            9_302,
            vec![pin("adv_consent_bypass", &adv_leaf.fingerprint, 9_301)],
            vec![peer_cfg(
                "adv_consent_bypass",
                "tls://127.0.0.1:0",
                &adv_leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        let target_b: TcpA2ATransport = bind_endpoint(
            &target_b_leaf,
            Some(&ca),
            9_303,
            vec![pin("adv_consent_bypass", &adv_leaf.fingerprint, 9_301)],
            vec![peer_cfg(
                "adv_consent_bypass",
                "tls://127.0.0.1:0",
                &adv_leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        let target_a_addr = target_a.local_addr().expect("bound (H3/H4)");
        let target_b_addr = target_b.local_addr().expect("bound (H3/H4)");
        let adversary: TcpA2ATransport = bind_endpoint(
            &adv_leaf,
            Some(&ca),
            9_301,
            vec![
                pin("consent_target_a", &target_a_leaf.fingerprint, 9_302),
                pin("consent_target_b", &target_b_leaf.fingerprint, 9_303),
            ],
            vec![
                peer_cfg(
                    "consent_target_a",
                    "tls://127.0.0.1:0",
                    &target_a_leaf.fingerprint,
                    &["readonly", "standard"],
                    &["readonly"],
                ),
                peer_cfg(
                    "consent_target_b",
                    "tls://127.0.0.1:0",
                    &target_b_leaf.fingerprint,
                    &["readonly", "standard"],
                    &["readonly"],
                ),
            ],
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        adversary.set_peer_endpoint(
            &HostId("consent_target_a".into()),
            format!("tls://{target_a_addr}"),
        );
        adversary.set_peer_endpoint(
            &HostId("consent_target_b".into()),
            format!("tls://{target_b_addr}"),
        );

        let mut blast_peers = BTreeSet::new();
        for (idx, target_host) in ["consent_target_a", "consent_target_b"]
            .into_iter()
            .enumerate()
        {
            let frame = make_frame(
                "adv_consent_bypass",
                target_host,
                IntentClass::Readonly,
                20 + idx as u64,
            );
            let ok = adversary
                .route_outbound(frame, &HostId(target_host.into()))
                .await;
            assert!(
                ok.is_ok(),
                "the readonly-allowed frame must succeed (real reachability before isolation): {ok:?}"
            );
            blast_peers.insert(target_host.to_string());
        }
        let escalated = make_frame(
            "adv_consent_bypass",
            "consent_target_a",
            IntentClass::Standard,
            30,
        );
        let result = adversary
            .route_outbound(escalated, &HostId("consent_target_a".into()))
            .await;
        let rejection_ns = mono_ns(&base);
        assert!(
            matches!(result, Err(A2AError::IntentDeniedAtPeer { .. })),
            "AdrLevel012ConsentBypass must be rejected at the ROUTER NACK layer, got {result:?}"
        );
        detections.push(AdversarialDetection {
            adversary_id: "adv_consent_bypass".into(),
            adversary_fingerprint: adv_leaf.fingerprint.wire(),
            attack_class: AdversarialAttempt::AdrLevel012ConsentBypass,
            join_ns,
            first_rejection_ns: Some(rejection_ns),
            blast_peers,
        });
    }

    detections
}

/// Build the detection-only report (recovery/rto not exercised on this leg).
fn detection_report(per_adversary: Vec<AdversarialDetection>) -> ChurnDrillReport {
    ChurnDrillReport::from_real_events(
        "t-11-3-detection-latency",
        BTreeSet::new(),
        BTreeSet::new(),
        per_adversary,
        0,
        None,
    )
}

/// AC2 — two-surface detection latency, per-event, with the
/// adversarial-host-identity reflex. (Leg-2 clean oracle.)
#[tokio::test]
#[ignore = "Story 11.3 — real detection sub-mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_detection_latency_drill() {
    let per_adversary = plant_and_detect_three_adversaries().await;
    assert_eq!(
        per_adversary.len(),
        PLANTED_ADVERSARIES,
        "all 3 planted classes must be detected"
    );

    // Adversarial-host-identity reflex (L8/D7): the detected set reconciles to
    // 3 DISTINCT planted cert-fingerprint identities — a real count/identity
    // contract (NOT a tautological re-inlining of `detection_surface()`).
    let report = detection_report(per_adversary.clone());
    report
        .reconcile_detections(PLANTED_ADVERSARIES)
        .expect("clean detection set must reconcile to 3 distinct planted identities");

    // Non-degenerate distribution (11.2a vacuous-count guard): the RAW ns
    // detection-latency samples must ALL be distinct (the seconds-rounded
    // value is 0 for every sample on sub-second loopback, L5).
    let raw_ns: Vec<u64> = per_adversary
        .iter()
        .map(|d| {
            d.detection_latency_ns()
                .expect("every planted adversary must be detected")
        })
        .collect();
    let distinct_raw: BTreeSet<u64> = raw_ns.iter().copied().collect();
    assert_eq!(
        distinct_raw.len(),
        raw_ns.len(),
        "detection latencies must be non-degenerate — all distinct (11.2a vacuous-count guard); raw_ns={raw_ns:?}"
    );

    // Median + p99 detection floors (both BINDING at v2.0 now; p99 is the
    // stricter tail companion). Trivially green on loopback (L5) — the teeth
    // are the blind falsifiers, not this clean pass.
    assert!(
        report.detection_latency_median_secs <= 3600 && report.detection_latency_p99_secs <= 3600,
        "detection median+p99 must be within the 1h floor: {report:?}"
    );

    eprintln!("=== Story 11.3 detection-latency drill ===");
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));
}

// ─────────── AC3 — blast + recovery (BIND) + rto (REPORTED), real events ───────────

/// Fault knobs for the consent reachability/recovery drill. `Default` is the
/// clean path (2 targets, real isolation, real re-pin). Only the
/// `#[cfg(feature = "churn-fault-inject")]` tests set non-default values; each
/// knob makes a REAL reachability outcome diverge (a real over-reach, a real
/// still-reachable adversary, a real broken legit dial).
#[derive(Clone, Copy)]
struct DrillFaults {
    /// Distinct legit targets the adversary reaches on readonly before denial.
    blast_targets: usize,
    /// Perform the real isolation repoint (clean); `false` = skip it, so the
    /// adversary stays reachable → rto objective unmet.
    isolate: bool,
    /// Perform the real legit re-pin/reconverge (clean); `false` = break a
    /// legit peer's endpoint → fleet never reconverges → recovery unmet.
    repin: bool,
}

impl Default for DrillFaults {
    fn default() -> Self {
        Self {
            blast_targets: 2,
            isolate: true,
            repin: true,
        }
    }
}

/// Sentinel `rto_secs` (over 4h, in ns) — the derived value when isolation is
/// NOT confirmed by a real failed re-dial (the objective was not met on the
/// live mesh; the breach is a CONSEQUENCE of a real still-succeeding dial, not
/// an injected constant).
const RTO_UNMET_NS: u64 = 5 * 3600 * 1_000_000_000;
/// Sentinel `recovery_secs` (over 24h, in ns) — the derived value when the
/// legit fleet does NOT reconverge (a real legit↔legit dial failed).
const RECOVERY_UNMET_NS: u64 = 25 * 3600 * 1_000_000_000;

/// Real consent-bypass reachability drill on ONE dedicated live sub-mesh:
/// adversary + `faults.blast_targets` legit targets (fully pinned to each
/// other for reconvergence). Derives blast (real deliveries), detection (real
/// NACK), rto (detection → adversary confirmed UNREACHABLE via a real failed
/// re-dial), and recovery (detection → legit fleet reconverged via a real
/// successful re-dial) — recovery and rto are two DISTINCT real events on the
/// SAME mesh the adversary attacked (D1/D2).
// The drill indexes several PARALLEL arrays (`target_leaves`, `target_names`,
// `targets`, `target_addrs`) by the same `i`/`j`; range loops are clearer than
// nested `enumerate()`/`zip()` here (the 10.4b mesh-builder precedent).
#[allow(clippy::needless_range_loop)]
async fn run_consent_reachability_drill(faults: DrillFaults) -> ChurnDrillReport {
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-11-3-consent-reach");
    let retry = no_retry();
    let k = faults.blast_targets;
    assert!(
        k >= 2,
        "need >=2 legit targets for a reconvergence measurement"
    );

    let adv_leaf = valid_leaf(&ca, &clock);
    let target_leaves: Vec<Leaf> = (0..k).map(|_| valid_leaf(&ca, &clock)).collect();
    let target_names: Vec<String> = (0..k).map(|i| format!("reach_target_{i}")).collect();

    // Each target pins the adversary + every OTHER target (readonly), so the
    // legit fleet can reconverge target↔target after the adversary is isolated.
    let mut targets: Vec<TcpA2ATransport> = Vec::with_capacity(k);
    for i in 0..k {
        let mut pins = vec![pin("adv_consent_bypass", &adv_leaf.fingerprint, 9_301)];
        let mut cfgs = vec![peer_cfg(
            "adv_consent_bypass",
            "tls://127.0.0.1:0",
            &adv_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )];
        for j in 0..k {
            if j != i {
                pins.push(pin(
                    &target_names[j],
                    &target_leaves[j].fingerprint,
                    9_400 + j as u64,
                ));
                cfgs.push(peer_cfg(
                    &target_names[j],
                    "tls://127.0.0.1:0",
                    &target_leaves[j].fingerprint,
                    &["readonly"],
                    &["readonly"],
                ));
            }
        }
        let t = bind_endpoint(
            &target_leaves[i],
            Some(&ca),
            9_400 + i as u64,
            pins,
            cfgs,
            &clock,
            TcpTimeouts::test_profile(),
            retry.clone(),
        )
        .await;
        targets.push(t);
    }
    let target_addrs: Vec<SocketAddr> = targets
        .iter()
        .map(|t| t.local_addr().expect("target bound (H3/H4)"))
        .collect();
    // Wire the legit target↔target endpoints (real readback addrs).
    for i in 0..k {
        for j in 0..k {
            if j != i {
                targets[i].set_peer_endpoint(
                    &HostId(target_names[j].clone()),
                    format!("tls://{}", target_addrs[j]),
                );
            }
        }
    }

    // Adversary pins every target; send-allowlist includes "standard" (it will
    // try to escalate) but the targets accept only "readonly".
    let adv_pins: Vec<_> = (0..k)
        .map(|i| {
            pin(
                &target_names[i],
                &target_leaves[i].fingerprint,
                9_400 + i as u64,
            )
        })
        .collect();
    let adv_cfgs: Vec<_> = (0..k)
        .map(|i| {
            peer_cfg(
                &target_names[i],
                "tls://127.0.0.1:0",
                &target_leaves[i].fingerprint,
                &["readonly", "standard"],
                &["readonly"],
            )
        })
        .collect();
    let adversary: TcpA2ATransport = bind_endpoint(
        &adv_leaf,
        Some(&ca),
        9_301,
        adv_pins,
        adv_cfgs,
        &clock,
        TcpTimeouts::test_profile(),
        retry.clone(),
    )
    .await;
    for i in 0..k {
        adversary.set_peer_endpoint(
            &HostId(target_names[i].clone()),
            format!("tls://{}", target_addrs[i]),
        );
    }

    let join_ns = mono_ns(&base);

    // Real blast: the adversary genuinely reaches every target on readonly.
    let mut blast_peers = BTreeSet::new();
    for (i, name) in target_names.iter().enumerate() {
        let frame = make_frame(
            "adv_consent_bypass",
            name,
            IntentClass::Readonly,
            100 + i as u64,
        );
        let ok = adversary.route_outbound(frame, &HostId(name.clone())).await;
        assert!(
            ok.is_ok(),
            "readonly delivery to {name} must succeed (real blast): {ok:?}"
        );
        blast_peers.insert(name.clone());
    }

    // Detection: escalate to "standard" on target 0 → router NACK.
    let escalated = make_frame(
        "adv_consent_bypass",
        &target_names[0],
        IntentClass::Standard,
        200,
    );
    let denied = adversary
        .route_outbound(escalated, &HostId(target_names[0].clone()))
        .await;
    let detection_ns = mono_ns(&base);
    assert!(
        matches!(denied, Err(A2AError::IntentDeniedAtPeer { .. })),
        "consent escalation must be denied at the router NACK layer, got {denied:?}"
    );

    // ── RTO event: isolate the adversary and CONFIRM unreachability by a real
    // re-dial that must now fail. (isolation-blind skips the repoint → the
    // re-dial still SUCCEEDS → objective unmet.)
    if faults.isolate {
        for name in &target_names {
            adversary.set_peer_endpoint(&HostId(name.clone()), dead_endpoint());
        }
    }
    let recheck = make_frame(
        "adv_consent_bypass",
        &target_names[0],
        IntentClass::Readonly,
        300,
    );
    let redial = adversary
        .route_outbound(recheck, &HostId(target_names[0].clone()))
        .await;
    let adversary_isolated = redial.is_err();
    let isolated_ns = mono_ns(&base);
    let rto_ns = if adversary_isolated {
        isolated_ns.saturating_sub(detection_ns)
    } else {
        RTO_UNMET_NS
    };

    // ── Recovery event: the legit fleet reconverges (target_0 ↔ target_1 real
    // re-dial). (re-pin-blind breaks target_0's endpoint for target_1 → the
    // legit dial FAILS → fleet not reconverged → objective unmet.)
    if !faults.repin {
        targets[0].set_peer_endpoint(&HostId(target_names[1].clone()), dead_endpoint());
    }
    let ab = targets[0]
        .route_outbound(
            make_frame(
                &target_names[0],
                &target_names[1],
                IntentClass::Readonly,
                400,
            ),
            &HostId(target_names[1].clone()),
        )
        .await;
    let ba = targets[1]
        .route_outbound(
            make_frame(
                &target_names[1],
                &target_names[0],
                IntentClass::Readonly,
                401,
            ),
            &HostId(target_names[0].clone()),
        )
        .await;
    let fleet_reconverged = ab.is_ok() && ba.is_ok();
    let reconverge_ns = mono_ns(&base);
    let recovery_ns = if fleet_reconverged {
        reconverge_ns.saturating_sub(detection_ns)
    } else {
        RECOVERY_UNMET_NS
    };

    let detection = AdversarialDetection {
        adversary_id: "adv_consent_bypass".into(),
        adversary_fingerprint: adv_leaf.fingerprint.wire(),
        attack_class: AdversarialAttempt::AdrLevel012ConsentBypass,
        join_ns,
        first_rejection_ns: Some(detection_ns),
        blast_peers,
    };
    ChurnDrillReport::from_real_events(
        "t-11-3-blast-recovery-rto",
        target_leaves.iter().map(|l| l.fingerprint.wire()).collect(),
        target_addrs.into_iter().collect(),
        vec![detection],
        recovery_ns,
        Some(rto_ns),
    )
}

/// AC3 (D4/D5) — blast (BINDING ≤5), recovery (BINDING ≤24h), rto (REPORTED
/// ≤4h) derived from real reachability events on the mesh the adversary
/// attacked. (Leg-3 clean oracle — DISTINCT from the detection drill.)
#[tokio::test]
#[ignore = "Story 11.3 — real recovery sub-mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_blast_recovery_rto_drill() {
    let report = run_consent_reachability_drill(DrillFaults::default()).await;
    eprintln!("=== Story 11.3 blast/recovery/rto drill ===");
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));

    assert_eq!(
        report.max_blast_radius, 2,
        "clean drill reaches exactly 2 targets"
    );
    report
        .reconcile_detections(1)
        .expect("the planted consent adversary must reconcile to its identity");
    assert!(
        report.passes_v20_binding_floors(),
        "clean loopback drill must pass the binding v2.0 floors (blast ≤5, recovery ≤24h): {report:?}"
    );
    assert!(
        !report.rto_exceeded_advisory(),
        "clean loopback drill must not breach the 4h reported RTO floor: {report:?}"
    );
}

// ─────────────────────── AC4 — churn-fault-inject falsifiers ───────────────────────

/// Blind ONE adversary class's rejection from the harness's COUNTED tally. The
/// real detector still fired; NEVER feature-gates `verifier.rs`/`router` (that
/// would be subsystem-gating, the 11.2b P2 sin) — only the harness's tally
/// drops the sample, and the DOWNSTREAM count/identity reconcile then REDS.
#[cfg(feature = "churn-fault-inject")]
fn blind_seam(
    detections: Vec<AdversarialDetection>,
    blind: AdversarialAttempt,
) -> Vec<AdversarialDetection> {
    detections
        .into_iter()
        .filter(|d| d.attack_class != blind)
        .collect()
}

#[cfg(feature = "churn-fault-inject")]
async fn fault_inject_blind_one_class(blind: AdversarialAttempt) {
    let detections = plant_and_detect_three_adversaries().await;
    assert_eq!(
        detections.len(),
        PLANTED_ADVERSARIES,
        "the real detectors must fire for all 3 classes BEFORE the harness blinds one"
    );
    // Clean set reconciles to 3 distinct planted identities.
    let clean = detection_report(detections.clone());
    clean
        .reconcile_detections(PLANTED_ADVERSARIES)
        .expect("clean set reconciles to 3 planted identities");

    // Blind one class at the counting seam → the DOWNSTREAM reconcile (the
    // real count/identity contract the gate consumes) REDS. This is the
    // falsifier — not `Vec::filter` on a hardcoded vector.
    let blinded = detection_report(blind_seam(detections, blind));
    let verdict = blinded.reconcile_detections(PLANTED_ADVERSARIES);
    assert!(
        verdict.is_err(),
        "blinding {blind:?} must red the detected-count/identity reconcile (3→2): {verdict:?}"
    );
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blind_pin_spoof_reds_reconcile() {
    fault_inject_blind_one_class(AdversarialAttempt::TofuPinSpoofing).await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blind_consent_bypass_reds_reconcile() {
    fault_inject_blind_one_class(AdversarialAttempt::AdrLevel012ConsentBypass).await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blind_cert_race_reds_reconcile() {
    fault_inject_blind_one_class(AdversarialAttempt::CertRotationRaceExploit).await;
}

/// AC3/AC4 — a REAL blast over-reach: the adversary reaches 6 legit targets
/// before denial → derived `max_blast_radius = 6` → the ≤5 BINDING floor REDS.
/// A live falsifier for the blast axis (not a synthetic 6-peer fixture).
#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blast_overreach_reds_floor() {
    let report = run_consent_reachability_drill(DrillFaults {
        blast_targets: 6,
        ..DrillFaults::default()
    })
    .await;
    assert_eq!(
        report.max_blast_radius, 6,
        "the adversary genuinely reached 6 targets: {report:?}"
    );
    assert!(
        !report.passes_v20_binding_floors(),
        "a real blast of 6 must red the ≤5 binding floor: {report:?}"
    );
}

/// AC3/AC4 (D5, F3 separability) — ISOLATION-blind: skip the real isolation
/// repoint, so a real re-dial adversary→target still SUCCEEDS → the rto
/// objective is unmet → `rto_secs` REDS, while `recovery_secs` (the legit
/// fleet reconverged) stays green. INDEPENDENT of the recovery axis.
#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_isolation_blind_reds_rto_only() {
    let report = run_consent_reachability_drill(DrillFaults {
        isolate: false,
        ..DrillFaults::default()
    })
    .await;
    assert!(
        report.rto_exceeded_advisory(),
        "isolation-blind: adversary still reachable → rto objective unmet → rto REDS: {report:?}"
    );
    assert!(
        report.passes_v20_binding_floors(),
        "isolation-blind MUST NOT red recovery/blast (independent falsifier, D5): {report:?}"
    );
}

/// AC3/AC4 (D5, F3 separability) — RE-PIN-blind: break a legit peer's endpoint
/// so a real legit↔legit re-dial FAILS → the fleet never reconverges →
/// `recovery_secs` REDS (the binding floor), while `rto_secs` (adversary
/// isolated) stays green. INDEPENDENT of the rto axis.
#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_repin_blind_reds_recovery_only() {
    let report = run_consent_reachability_drill(DrillFaults {
        repin: false,
        ..DrillFaults::default()
    })
    .await;
    assert!(
        !report.passes_v20_binding_floors(),
        "re-pin-blind: legit fleet did not reconverge → recovery REDS (binding floor): {report:?}"
    );
    assert!(
        !report.rto_exceeded_advisory(),
        "re-pin-blind MUST NOT red rto (independent falsifier, D5): {report:?}"
    );
}
