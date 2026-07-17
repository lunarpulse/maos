//! Story 12.1 cohort-mesh coverage — Task 0 preflight + Task 2 production-
//! relevant extension.
//!
//! All sweeps are deliberately all-`i != j` (no self-dials, no hub-and-spoke
//! shortcut). At N=8 that is 56 directed dials and 28 bilateral channels.
//!
//! * **Task 0** — [`t_12_1_n8_full_pairwise_mesh_measurement`]: the real N=8
//!   full-pairwise mTLS measurement (wall-clock over 56 real handshakes).
//! * **Task 2** — the signed-manifest mesh foundation this file now exercises:
//!   - [`t_12_1_pair_counts_derived_from_n`]: the 28/56 counts DERIVED from N
//!     (pure arithmetic, no transport, runs in the default suite).
//!   - [`t_12_1_n3_mesh_smoke`]: N=3 hermetic real-transport smoke settling
//!     under 2s (watchable measurement printout).
//!   - [`t_12_1_n8_distinct_identity_reconciliation`]: over a real N=8 mesh,
//!     distinct §7.2 cert fingerprints + distinct bound `SocketAddr`s, and the
//!     bound fingerprints reconcile with the declared leaf pins.
//!   - [`t_12_1_duplicate_fingerprint_detected`]: negative control — a cloned
//!     identity (two nodes serving the SAME leaf) collapses the distinct-
//!     fingerprint count while the `SocketAddr`s stay distinct, proving the
//!     distinctness invariant is real, not vacuous.
//!
//! The mesh is built with the parameterized [`build_mesh_n`] helper (never the
//! hardwired 3-host `build_mesh`): each node's N−1 peer configs + pins are
//! derived from the OTHER nodes' leaves, exactly the projection
//! `CohortManifest::peer_configs_for` produces, then reconciled with the real
//! bound `SocketAddr`s via `set_peer_endpoint`.

mod support;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ed25519_dalek::SigningKey;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_cohort::{
    CohortAuthority, CohortClock, CohortDistributor, CohortManifest, CohortManifestState,
    CohortMember, ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink, ManifestSignature,
    PinnedAuthorityKeys, COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::FrameAddress;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::{HostId, SpiritId};
use support::*;

const HOST_COUNT: usize = 8;

fn all_directed_pairs(n: usize) -> Vec<(usize, usize)> {
    (0..n)
        .flat_map(|from| {
            (0..n)
                .filter(move |&to| to != from)
                .map(move |to| (from, to))
        })
        .collect()
}

#[tokio::test]
#[ignore = "Story 12.1 — check-cohort-mesh owns the real N=8 full-pairwise sweep"]
async fn t_12_1_n8_full_pairwise_mesh_measurement() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-1-full-pairwise");
    let names: Vec<String> = (0..HOST_COUNT).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let refs: Vec<&Leaf> = leaves.iter().collect();
    let mesh = build_mesh_n(&clock, &ca, &names, &refs, &refs, no_retry()).await;
    let pairs = all_directed_pairs(mesh.len());

    assert_eq!(pairs.len(), mesh.len() * (mesh.len() - 1));
    assert!(pairs.iter().all(|(from, to)| from != to));

    let started = Instant::now();
    let results = concurrent_dial_pairs(&mesh, &pairs, 12_100, IntentClass::Readonly).await;
    let elapsed = started.elapsed();
    let failures: Vec<_> = results
        .iter()
        .filter(|(_, _, result)| result.is_err())
        .collect();

    assert!(
        failures.is_empty(),
        "full-pairwise sweep failures: {failures:?}"
    );
    assert_eq!(results.len(), mesh.len() * (mesh.len() - 1));
    eprintln!(
        "Story 12.1 Task 0: N={} channels={} directed_dials={} wall_clock_ms={}",
        mesh.len(),
        mesh.len() * (mesh.len() - 1) / 2,
        results.len(),
        elapsed.as_millis(),
    );
}

// ── Task 2 ──────────────────────────────────────────────────────────────────

/// The mesh-wide pair counts are DERIVED from N — never a hardcoded 56/28.
/// At N=8 the derivation yields 56 directed dials (N·(N−1)) and 28 bilateral
/// channels (N·(N−1)/2); the asserts below sanity-check that the derivation
/// reproduces those known values, not that a literal was asserted in place of
/// the formula. Pure arithmetic — no transport, so it runs in the default suite.
#[test]
fn t_12_1_pair_counts_derived_from_n() {
    for &n in &[2usize, 3, 8] {
        let pairs = all_directed_pairs(n);
        // Derived from N — the production formula, not a literal count.
        let directed = n * (n - 1);
        let bilateral = directed / 2;
        assert_eq!(pairs.len(), directed, "N={n}: directed dial count");
        // No self-dial ever appears in an all-`i != j` mesh.
        assert!(
            pairs.iter().all(|&(from, to)| from != to),
            "N={n}: no self-dials"
        );
        assert_eq!(
            bilateral * 2,
            directed,
            "N={n}: channels are half the directed dials"
        );
    }

    // The Task-2 fleet size: the derivation reproduces 56 dials / 28 channels.
    let n = HOST_COUNT;
    let directed = n * (n - 1);
    let bilateral = directed / 2;
    assert_eq!(directed, 56, "N=8 derives 56 directed dials");
    assert_eq!(bilateral, 28, "N=8 derives 28 bilateral channels");
}

/// N=3 hermetic real-transport mesh smoke: every directed pair dials
/// successfully over real mTLS, settling under the 2s budget. Watchable via the
/// measurement printout (`--nocapture`).
#[tokio::test]
#[ignore = "Story 12.1 Task 2 — N=3 hermetic real-transport mesh smoke (<2s)"]
async fn t_12_1_n3_mesh_smoke() {
    let n = 3;
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-1-n3-smoke");
    let names: Vec<String> = (0..n).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let refs: Vec<&Leaf> = leaves.iter().collect();
    let mesh = build_mesh_n(&clock, &ca, &names, &refs, &refs, no_retry()).await;
    let pairs = all_directed_pairs(mesh.len());

    let started = Instant::now();
    let results = concurrent_dial_pairs(&mesh, &pairs, 20_000, IntentClass::Readonly).await;
    let elapsed = started.elapsed();

    let failures: Vec<_> = results.iter().filter(|(_, _, r)| r.is_err()).collect();
    assert!(failures.is_empty(), "N=3 mesh dial failures: {failures:?}");
    // Directed-dial count derived from N, not a literal.
    assert_eq!(results.len(), n * (n - 1), "N=3 directed dial count");
    assert!(
        elapsed.as_millis() < 2000,
        "N=3 mesh must settle under 2s (got {} ms)",
        elapsed.as_millis()
    );
    eprintln!(
        "Story 12.1 Task 2 N=3 smoke: directed_dials={} channels={} wall_clock_ms={}",
        results.len(),
        n * (n - 1) / 2,
        elapsed.as_millis(),
    );
}

/// Distinct §7.2 cert fingerprints AND distinct bound `SocketAddr`s across a
/// real N=8 mesh, with the bound fingerprints reconciling against the declared
/// leaf pins (the trust material `peer_configs_for` would project). This is the
/// live-transport reflex that the manifest-derived peer edges feed.
#[tokio::test]
#[ignore = "Story 12.1 Task 2 — distinct fingerprint/SocketAddr reconciliation over a real N=8 mesh"]
async fn t_12_1_n8_distinct_identity_reconciliation() {
    let n = HOST_COUNT;
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-1-reconcile");
    let names: Vec<String> = (0..n).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let refs: Vec<&Leaf> = leaves.iter().collect();
    let mesh = build_mesh_n(&clock, &ca, &names, &refs, &refs, no_retry()).await;

    // Distinct §7.2 fingerprints — exactly N, one per host (the trust pins).
    let fp_set: HashSet<_> = mesh.iter().map(|node| node.fingerprint.clone()).collect();
    let fp_total = mesh.len();
    assert_eq!(
        fp_set.len(),
        fp_total,
        "N={n}: cert fingerprints must be pairwise distinct"
    );

    // Distinct bound SocketAddrs (H3/H4 readback) — exactly N, one per host.
    let mut addrs: Vec<_> = mesh.iter().map(|node| node.addr).collect();
    let addr_total = addrs.len();
    addrs.sort_unstable();
    addrs.dedup();
    assert_eq!(
        addrs.len(),
        addr_total,
        "N={n}: bound SocketAddrs must be pairwise distinct"
    );

    // Reconciliation: the bound node identity (served cert) equals the declared
    // leaf pin, in declaration order — declared §7.2 pin == live served identity.
    for (node, leaf) in mesh.iter().zip(leaves.iter()) {
        assert_eq!(
            node.fingerprint, leaf.fingerprint,
            "bound fingerprint must reconcile with the declared leaf pin"
        );
    }
}

/// Negative control: a CLONED identity (two nodes serving the SAME leaf) binds
/// at DISTINCT `SocketAddr`s but reports a DUPLICATE §7.2 fingerprint. The
/// distinct-fingerprint invariant therefore COLLAPSES (distinct < total) —
/// proving the reconciliation check is real and would catch a cloned identity,
/// not pass vacuously.
#[tokio::test]
#[ignore = "Story 12.1 Task 2 — duplicate fingerprint negative control over a real mesh"]
async fn t_12_1_duplicate_fingerprint_detected() {
    let n = 3;
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-1-dup-fp");
    let names: Vec<String> = (0..n).map(host_name).collect();
    // Two distinct leaves for host_00 / host_01; host_02 REUSES host_00's leaf
    // → a cloned identity (distinct addr, duplicate fingerprint).
    let leaf0 = valid_leaf(&ca, &clock);
    let leaf1 = valid_leaf(&ca, &clock);
    let serving: Vec<&Leaf> = vec![&leaf0, &leaf1, &leaf0];
    let mesh = build_mesh_n(&clock, &ca, &names, &serving, &serving, no_retry()).await;

    let total = mesh.len();

    // Fingerprints: the clone collapses the distinct count to total − 1.
    let fp_set: HashSet<_> = mesh.iter().map(|node| node.fingerprint.clone()).collect();
    assert_eq!(
        fp_set.len(),
        total - 1,
        "duplicate fingerprint MUST reduce the distinct count (got {} distinct of {})",
        fp_set.len(),
        total
    );
    // SocketAddrs: the cloned-identity nodes still bind at distinct addresses
    // (the listener readback is per-bind, independent of the served cert).
    let mut addrs: Vec<_> = mesh.iter().map(|node| node.addr).collect();
    addrs.sort_unstable();
    addrs.dedup();
    assert_eq!(
        addrs.len(),
        total,
        "cloned-identity nodes still bind distinct SocketAddrs"
    );
}

struct TestCohortClock(AtomicU64);

impl TestCohortClock {
    fn new(now: u64) -> Self {
        Self(AtomicU64::new(now))
    }

    fn set(&self, now: u64) {
        self.0.store(now, Ordering::Release);
    }
}

impl CohortClock for TestCohortClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}

fn signed_cohort_manifest(
    version: u64,
    signer: &SigningKey,
    host_a_fp: &maos_a2a_core::PeerCertFingerprint,
    host_b_fp: &maos_a2a_core::PeerCertFingerprint,
) -> String {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-12-1-recovery".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signer.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: host_a_fp.wire(),
                roles: vec!["worker".into()],
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: host_b_fp.wire(),
                roles: vec!["worker".into()],
            },
        ],
        consent: ConsentMatrix {
            send: vec![
                ConsentTuple {
                    peer: "host_a".into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                },
                ConsentTuple {
                    peer: "host_b".into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                },
            ],
            accept: vec![
                ConsentTuple {
                    peer: "host_a".into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                },
                ConsentTuple {
                    peer: "host_b".into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                },
            ],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 30,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
    }
    .signed_with(signer);
    toml::to_string(&manifest).expect("cohort recovery manifest serializes")
}

/// Task 4 real-wire recovery: ordinary work is denied while stale; a reserved
/// pull crosses real TCP/mTLS, the authority-side distributor sends the exact
/// signed current artifact, and only an explicit caller resubmit succeeds.
#[tokio::test]
#[ignore = "Story 12.1 Task 4 — real TCP stale→pull→signed-push→explicit-resubmit"]
async fn t_12_1_stale_pull_push_resubmit_real_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-1-recovery");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[42u8; 32]);
    let pins =
        PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).expect("authority pins");
    let manifest_v1 =
        signed_cohort_manifest(1, &authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let manifest_v2 =
        signed_cohort_manifest(2, &authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let clock_a = Arc::new(TestCohortClock::new(0));
    let clock_b = Arc::new(TestCohortClock::new(0));
    let state_a = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host_a".into()),
            &manifest_v2,
            pins.clone(),
            Arc::new(InMemoryCohortAuditSink::default()),
            clock_a,
        )
        .expect("authority state"),
    );
    let state_b = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host_b".into()),
            &manifest_v1,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
            clock_b.clone(),
        )
        .expect("stale member state"),
    );

    let pems_a = write_pem(&leaf_a, Some(&ca));
    let tcp_a = tcp_config(
        &pems_a,
        vec![pin("host_b", &leaf_b.fingerprint, 2)],
        std::time::Duration::from_secs(30),
    );
    let peer_a = peer_cfg(
        "host_b",
        "tls://127.0.0.1:0",
        &leaf_b.fingerprint,
        &["readonly"],
        &["readonly"],
    );
    let transport_a = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_a,
            vec![peer_a],
            1,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_a.clone()),
        )
        .await
        .expect("bind authority transport"),
    );
    let addr_a = transport_a.local_addr().expect("authority address");

    let pems_b = write_pem(&leaf_b, Some(&ca));
    let tcp_b = tcp_config(
        &pems_b,
        vec![pin("host_a", &leaf_a.fingerprint, 1)],
        std::time::Duration::from_secs(30),
    );
    let peer_b = peer_cfg(
        "host_a",
        &format!("tls://{addr_a}"),
        &leaf_a.fingerprint,
        &["readonly"],
        &["readonly"],
    );
    let transport_b = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_b,
            vec![peer_b],
            2,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_b.clone()),
        )
        .await
        .expect("bind member transport"),
    );
    let addr_b = transport_b.local_addr().expect("member address");
    transport_a.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{addr_b}"));

    clock_b.set(31);
    let ordinary = make_frame("host_b", "host_a", IntentClass::Readonly, 30_001);
    assert!(
        transport_b
            .route_outbound(ordinary.clone(), &HostId("host_a".into()))
            .await
            .is_err(),
        "stale ordinary work must fail before recovery"
    );

    let router_b: Arc<dyn A2APeerRouter> = transport_b.clone();
    let distributor_b = CohortDistributor::new(
        state_b.clone(),
        router_b,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host_b".into())),
            role: None,
        },
    );
    distributor_b
        .pull_from(&HostId("host_a".into()))
        .await
        .expect("reserved pull crosses live wire");

    let router_a: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor_a = CohortDistributor::new(
        state_a,
        router_a,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host_a".into())),
            role: None,
        },
    );
    assert_eq!(
        distributor_a
            .service_pending_pulls()
            .await
            .expect("signed push"),
        1,
        "one verified pull must produce one signed push"
    );
    assert_eq!(state_b.version().expect("member version"), 2);
    assert!(state_b.is_fresh(), "signed push refreshes the stale member");

    transport_b
        .route_outbound(ordinary, &HostId("host_a".into()))
        .await
        .expect("explicit resubmit succeeds after verified recovery");
}
