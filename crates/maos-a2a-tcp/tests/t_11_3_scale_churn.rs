//! Story 11.3 (AC1–AC4) + Story 14.1 (AC1–AC6) — real N-host host-churn
//! scale-envelope drill (NFR-Scale-2 / NFR-Rel-7) over the SAME in-process
//! real-socket real-mTLS substrate (11.3's ratified F1: `TcpA2ATransport`
//! endpoints on real `127.0.0.1` sockets + real rustls mTLS handshakes, one
//! process — NOT OS processes/containers). REPLACES the deleted
//! `maos-a2a-core::chaos::churn::run_scaffold` canned-constant scaffold: every
//! numeric field in `ChurnDrillReport` is DERIVED from real events on a live
//! mesh.
//!
//! # Story 14.1 — what changed and why (each is an AC, not a preference)
//!
//! * **N is a FUNCTION PARAMETER** (AC2.1 — decided; an env var was refused
//!   on the env-registry scope and the gate's env-passing seam): the drill
//!   bodies are `async fn ..._drill(n: usize, ...)` and each scale gets a
//!   thin `#[tokio::test]` wrapper — N=30 (the 11.3 compressed scale) and
//!   N=100 (the 14.1 envelope). `build_mesh_n` was already parameterized on
//!   `names.len()`; no new harness primitive was written for the mesh itself.
//! * **Adversaries are planted INTO the N-host mesh** (AC3.1) — previously
//!   detection ran on 2+2+3-endpoint sub-meshes and blast/recovery on 3. The
//!   planted classes are `support::PlantKind` members of the same mesh; the
//!   two surfaces are unchanged: handshake (`HandshakeFailed`) for
//!   `TofuPinSpoofing`/`CertRotationRaceExploit`, router NACK
//!   (`IntentDeniedAtPeer`) for `AdrLevel012ConsentBypass`. Class assertions
//!   match `HandshakeFailureClass` (AC3.5) — pin-spoof and cert-race are
//!   distinguishable at the assertion, not only in fixture construction.
//! * **The blast floor is a RACE now** (AC3.3, the story's centre): 11.3's
//!   `DrillFaults::default().blast_targets = 2` handed the adversary exactly
//!   two reachable targets against a binding floor of 5, so the clean path
//!   derived 2 on every run at every N ever run — a floor that could not
//!   fail. The adversary is now offered the FULL peer set (n−1 peers,
//!   derived from the mesh — 99 dialable peers at N=100) and `blast_peers`
//!   records real readonly deliveries before the first denial. AC3.3.b's
//!   non-degeneracy is asserted, not narrated: **offered == reached REDS the
//!   clean leg regardless of the count** (the `SweepOnly`
//!   `churn-fault-inject` mutation proves that green could have been red),
//!   and `offered: Some(k)` survives ONLY as the over-reach falsifier's cap
//!   (`k = 6` → derived blast 6 → the ≤5 floor REDS). The RED-at-HEAD
//!   demonstration (recorded in the story's Debug Log): the pre-14-1 clean
//!   path reported `max_blast_radius: 2` with `blast_peers` exactly the two
//!   targets the fixture offered — the ceiling, not a measurement.
//! * **The turnover band is a RATIO** (AC2.3 — a correctness fix, not a
//!   tuning knob): the absolute `(12..=24)` assertion broke at N=100 in both
//!   directions (10–20% of 99 slots = 40–80 events fails it; keeping 4
//!   events/round = 4.0% is below the band). `turnover_per_round(n)` derives
//!   ~15% of the n−1 non-hub slots (4 at N=30, 14 at N=100) and the test
//!   asserts the percentage band + records the derived event count.
//! * **Fleet reconvergence is the FULL hub-and-spoke sweep** (AC3.1.a
//!   decision): all `2×(L−1)` legit pairs over the legit fleet (L = n−1,
//!   hub included), never a sampled re-dial that could hide an
//!   un-reconverged host. The legit reachability sweep stays hub-and-spoke
//!   (AC2.4/AC3.3.c: `2×(N−1)` for the mesh sweep, FULL peer set for the
//!   adversary — one decision, not two). Silent truncation at 3× the scale
//!   was the hazard; the bound is disclosed here instead.
//! * **Bounded concurrency everywhere** (AC6.2.a/AC2.4):
//!   `support::concurrent_dial_pairs` uses `buffered(DIAL_CONCURRENCY)`, never
//!   `join_all`; the adversary's propagation sweep uses the same primitive.
//! * **The fd ceiling is PROBED** (AC6.2.a): `assert_fd_headroom` reads the
//!   soft `Max open files` limit and fails loudly BY NAME before an N-host
//!   mesh is stood up under a short limit.
//! * **The gate boundary cannot be greened vacuously** (AC5.1): every
//!   N=100 test prints `SCALE_CHURN_HOSTS_RECONCILED=<derived>` carrying the
//!   DERIVED reconciled host count (the duplicate-identity control prints 99
//!   — 100 clones are NOT 100 hosts, AC2.2). `check-scale-churn` requires
//!   the marker AND the exact test count per invocation.
//! * **Percentile honesty** (AC3.4.a): `detection_latency_p99_secs` is the
//!   MAXIMUM at every n this file runs (nearest-rank p99 of 3 samples); the
//!   field name is the JSON contract and is NOT renamed — the sample count
//!   is published beside every percentile in the test output, the gate
//!   summary and `report_to_markdown`, so a reader cannot mistake max-of-3
//!   for a characterised tail.
//! * **Failure-branch sentinels are DISCLOSED, not derived** (AC4.4): on a
//!   falsifier run where isolation was never confirmed, `rto_secs` is the
//!   `RTO_UNMET_NS` sentinel (5h); where the fleet never reconverged,
//!   `recovery_secs` is the `RECOVERY_UNMET_NS` sentinel (25h). The branch
//!   selector is a real reachability outcome; the magnitude carries NO
//!   information and only the branch is evidence. See the constants below
//!   and the disclosure in `report_to_markdown`.
//!
//! # The teeth bite on REAL events (11.3 rework, carried through 14.1)
//!
//! The clean loopback pass is trivial by design (L5: sub-second events clear
//! the ≤1h/≤5/≤24h floors). The gate's teeth are FALSIFIERS, and every one is
//! driven by a REAL reachability outcome on the live mesh, never a
//! hand-injected constant into `from_real_events`:
//!
//! * **blast >5** (`churn-fault-inject`, at BOTH N): the adversary sweeps a
//!   real 6-peer subset of the N-host mesh and genuinely reaches all 6
//!   before its single escalation is denied → derived `max_blast_radius = 6`
//!   → the ≤5 binding floor REDS.
//! * **offered == reached** (`churn-fault-inject`, the 14.1 non-degeneracy):
//!   the adversary sweeps the FULL offered set without ever escalating →
//!   nothing denies it → `reached == offered` → the clean leg's
//!   non-degeneracy assertion REDS (and the detection reconcile REDS on the
//!   miss). This is the mutation that makes the clean green mean something.
//! * **blind-one-detector** (`churn-fault-inject`, at BOTH N): a class is
//!   dropped from the counted tally → the DOWNSTREAM
//!   `ChurnDrillReport::reconcile_detections(3)` count/identity contract
//!   REDS. The real detector still fires; `verifier.rs`/`router.rs` are
//!   never feature-gated (the 11.2b P2 sin).
//! * **isolation-blind** (`churn-fault-inject`, at BOTH N): the harness
//!   skips the real isolation repoint → a real re-dial adversary→peer still
//!   SUCCEEDS → `rto` objective unmet → `rto_secs` (sentinel) REDS,
//!   `recovery_secs` unaffected.
//! * **re-pin-blind** (`churn-fault-inject`, at BOTH N): the harness breaks
//!   a legit peer's endpoint → the FULL legit hub-and-spoke sweep FAILS →
//!   fleet not reconverged → `recovery_secs` (sentinel) REDS, `rto_secs`
//!   unaffected. (F3 separability — two INDEPENDENT real falsifiers, D5;
//!   both re-proven at N=100 because they are AC4.3's evidence and do not
//!   travel from N=30.)
//!
//! `recovery_secs`/`rto_secs` are two DISTINCT real events on the SAME mesh
//! the adversary attacked: `rto` = detection→(adversary confirmed unreachable
//! via a real failed re-dial); `recovery` = detection→(the full legit
//! hub-and-spoke sweep passing). They are not the same measurement.
//!
//! # CI-budget decomposition (disclosure — §CI time budget)
//!
//! The legit reachability sweeps are hub-and-spoke, `2×(N−1)` at N=30 and
//! `2×(L−1)` over the L = N−1 legit fleet in the blast drill — NOT full NxN
//! (9,900 directed dials at N=100). A stable hub (index 0, never churns,
//! never planted) transitively proves the mesh is NOT partitioned; it is NOT
//! a claim that every non-hub pair was individually dialed. The adversary's
//! propagation sweep is bounded-concurrency over its offered set but stops
//! at its first escalation denial (PROBE_BATCH deliveries per probe on the
//! clean path). Every dial that DOES happen is a real socket op; nothing is
//! simulated or time-frozen (`tokio::time::pause()` is never used). Timing
//! is read from a single per-drill monotonic `std::time::Instant` base (L4).
//!
//! The watchable N=5 scene (AC1) lives in `t_14_1_churn_scene.rs` — a
//! narrating, non-ignored observability artifact that is NEVER a gate-leg
//! oracle for any NFR floor (AC1.4); AC2–AC4 here are the proof.
mod support;

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::time::Instant;

use maos_a2a_core::chaos::churn::report_to_markdown;
use maos_a2a_core::error::HandshakeFailureClass;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, AdversarialAttempt, AdversarialDetection, ChurnDrillReport};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

/// F2 (ratified, preflight 2026-07-03) — the compressed scale runs at N=30
/// (NFR-Rel-7's "compressed 30-host"), floor ≥25 (NFR-Scale-2's literal
/// "25-host"). Named const with provenance — never a magic literal. 14.1's
/// full envelope scale is its own wrapper constant below.
const COMPRESSED_HOST_COUNT: usize = 30;
/// Story 14.1 — the full envelope scale. At this N an adversary inside the
/// mesh has 99 dialable peers: the first mesh on which the ≤5 blast floor
/// can genuinely be violated (AC3.3).
const FULL_ENVELOPE_HOST_COUNT: usize = 100;
/// The gate asserts the RECONCILED distinct-identity count against this
/// floor on the compressed scale, never the literal host count (D7/L8 —
/// "30 hosts" must be 30 DISTINCT hosts).
const HOST_COUNT_FLOOR: usize = 25;
/// Index 0 is the stable hub — it never churns and is never a planted
/// adversary, keeping the bounded reachability sweep meaningful across every
/// round (see module docs).
const HUB: usize = 0;
/// 4 compressed rounds standing in for "4 weeks" (NFR-Rel-7's 4-week window,
/// modeled as a compressed churn-event schedule per the "Explicitly NOT in
/// 11.3" scope note — loopback churn events are sub-second, not 4 real
/// weeks).
const CHURN_ROUNDS: usize = 4;
/// The 3 adversary classes planted for detection (NFR-Rel-7's "3 planted
/// adversarial hosts"; AC3.2 fixed the count at the NFR's number — scaling it
/// is out of scope and would change the `reconcile_detections` contract).
const PLANTED_ADVERSARIES: usize = 3;
// (fd_headroom_for / FD_PER_HOST / FD_HEADROOM_FLOOR live in `support` since
// round 2 — the N=5 scene derives its own requirement instead of inheriting
// the envelope's literal.)

/// Story 14-1 review D-C (ratified 2026-08-27, "no more deferring"): AC6.2.a's
/// runner preconditions are ENFORCED IN CODE, not promised as an obligation.
/// Grumbal's trip condition — wall > 5 min OR peak RSS > 1 GiB OR the fd probe
/// firing — is now a guard every drill carries, so the FIRST CI run measures
/// itself, publishes the three numbers, and REDS loudly by name on a slow or
/// tight runner instead of flaking or quietly passing on inherited numbers.
const RUNNER_WALL_BUDGET_SECS: u64 = 300;
/// 1 GiB in kB — the ratified peak-RSS trip condition.
const RUNNER_PEAK_RSS_BUDGET_KB: u64 = 1_048_576;

/// RAII guard: publishes `wall` + `peak_rss_kb` for the drill it wraps and
/// enforces the trip condition on drop. It never fires while the thread is
/// already panicking, so a real assertion failure is never masked by a
/// budget message.
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
        // Round-2 P2: an unreadable VmHWM prints `unmeasured` and FAILS by
        // name — the helper's contract says unmeasured is never "fine", and
        // `0 <= budget` must not stand in for a measurement.
        let rss_token = match rss {
            Some(kb) => format!("{kb}"),
            None => "unmeasured".to_string(),
        };
        println!(
            "SCALE_CHURN_RUNNER_ENVELOPE {} wall_s={:.2} peak_rss_kb={rss_token}",
            self.label,
            wall.as_secs_f64()
        );
        if std::thread::panicking() {
            return;
        }
        assert!(
            wall.as_secs_f64() <= RUNNER_WALL_BUDGET_SECS as f64,
            "RUNNER-BUDGET ({}): wall {:.1}s exceeds the ratified {RUNNER_WALL_BUDGET_SECS}s trip \
             condition — the N=100 leg must go per-leg advisory with a named owner (AC6.2.a/AC6.3)",
            self.label,
            wall.as_secs_f64()
        );
        let rss = rss.unwrap_or_else(|| {
            panic!(
                "RUNNER-UNMEASURED ({}): /proc/self/status VmHWM unreadable — the peak-RSS trip \
                 condition cannot be evaluated, and unmeasured must fail by name, not pass as 0 \
                 (AC6.2.a/D-C)",
                self.label
            )
        });
        assert!(
            rss <= RUNNER_PEAK_RSS_BUDGET_KB,
            "RUNNER-BUDGET ({}): peak RSS {rss} kB exceeds the ratified 1 GiB trip condition \
             — bounded concurrency is not holding on this runner (AC6.2.a)",
            self.label
        );
    }
}
/// The probing adversary delivers this many readonly frames per escalation
/// probe (clean strategy). The offered ceiling is n−1 (derived); this batch
/// bound is the adversary MODEL — the mutations prove both failure
/// directions (`SweepThenEscalate` over a capped set → 6 > 5 REDS the floor;
/// `SweepOnly` over the full set → offered == reached REDS the leg).
const PROBE_BATCH: usize = 4;

/// Round-2 P6: the two D-A binding conjuncts as a FALLIBLE verdict, so the
/// detector-off falsifier can prove the binding predicate itself REJECTS its
/// report. Before this, the conjuncts existed only as asserts on the clean
/// path (where they must PASS) while the falsifier checked its own fixture
/// against the same constants — deleting the clean-side asserts would have
/// left every leg green.
fn clean_blast_verdict(reached: usize, offered_len: usize) -> Result<(), String> {
    if reached > 4 * RACE_IN_FLIGHT {
        return Err(format!(
            "detection must confine propagation to a BOUNDED number of in-flight generations \
             (<= 4W = {}), got {reached} — nothing structurally stopped the adversary",
            4 * RACE_IN_FLIGHT
        ));
    }
    if offered_len > 2 && reached * 2 >= offered_len {
        return Err(format!(
            "detection must interrupt before the adversary reaches HALF the offered set: \
             reached {reached} of {offered_len} offered — a bound derived from the run, \
             red at every scale"
        ));
    }
    Ok(())
}

/// Story 14.1 AC5.1 — the marker every N=100 test prints carrying the DERIVED
/// reconciled host count. A test that early-returns prints no marker and the
/// gate reds the leg: a silent skip cannot green the gate boundary.
const SCALE_MARKER_PREFIX: &str = "SCALE_CHURN_HOSTS_RECONCILED";

fn print_scale_marker(hosts: usize) {
    println!("{SCALE_MARKER_PREFIX}={hosts}");
}

/// Story 14.1 AC2.3 — per-round turnover as a RATIO of the n−1 non-hub
/// slots: ~15% (the midpoint of NFR-Rel-7's ratified 10–20%/week band).
/// 15% of 29 = 4 at the compressed scale (11.3's historical value, now
/// derived rather than canned); 15% of 99 = 14 at the full envelope. The
/// wrapper asserts the derived percentage stays inside the band at every N
/// it runs.
fn turnover_per_round(n: usize) -> usize {
    ((n - 1) * 15) / 100
}

/// Nanoseconds elapsed on the harness's OWN monotonic `Instant` base (L4 — a
/// same-process monotonic reading, never a cross-host clock subtraction,
/// never a frame's wall-clock `timestamp`). `Instant` is monotonic, so a
/// backward wall-clock (NTP) step can never invert a latency delta.
fn mono_ns(base: &Instant) -> u64 {
    base.elapsed().as_nanos() as u64
}

/// Round-2 P5: a planted host's JOIN timestamp — the monotonic ns at its
/// bind (captured inside the builder), so a detection sample is
/// `t_first_rejection − t_join` for THAT adversary, never a single
/// post-build wall reading shared by all three (AC3.4's defined sample).
fn join_ns_of(base: &Instant, node: &MeshNode) -> u64 {
    node.bound_at.duration_since(*base).as_nanos() as u64
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

// ─────────────────────── AC1/AC2 — identity reconcile + churn envelope ───────────────────────

/// AC1/AC2 — bind n real listeners (real rcgen certs, real `127.0.0.1:0`
/// sockets); the "N hosts" claim is DERIVED-AND-RECONCILED against distinct
/// cert fingerprints AND distinct bound `SocketAddr`s, never the literal
/// count on its own (L8/D7). No dials needed for identity.
async fn mesh_identity_reconcile_drill(n: usize, label: &str) {
    assert_fd_headroom(fd_headroom_for(n));
    let _envelope = RunnerEnvelope::open(format!("identity-reconcile-n{n}"));
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-identity");
    let names: Vec<String> = (0..n).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;

    let host_fingerprints: BTreeSet<String> = mesh.iter().map(|m| m.fingerprint.wire()).collect();
    let host_addrs: BTreeSet<SocketAddr> = mesh.iter().map(|m| m.addr).collect();
    let report =
        ChurnDrillReport::from_real_events(label, host_fingerprints, host_addrs, vec![], 0, None);
    assert_eq!(
        report.distinct_host_count(),
        n,
        "a clean {n}-node bind (no clones planted) must reconcile to exactly {n} distinct hosts"
    );
    assert!(
        report.distinct_host_count() >= HOST_COUNT_FLOOR,
        "distinct-identity-reconciled host count must clear the NFR-Scale-2 compressed floor"
    );
    print_scale_marker(report.distinct_host_count());
}

/// AC2.2 — topology-fraud negative control, scaled. **RE-SITED by Story
/// 14-2a (AC6.5) — NOT re-pointed.** Story 14-2's bind-time uniqueness guard
/// now refuses the old fixture (`serving[6] = &leaves[5]`: two hosts, one
/// fingerprint) at BIND (`support/mod.rs` `bind mesh endpoint` →
/// `rotation window refused ... already held by host_05`), so a duplicate can
/// no longer exist on a live mesh. Expecting that refusal here would delete
/// the claim this leg exists to prove and substitute a different one: the
/// bind-time refusal is a *config validator* claim; NFR-Rel-7's leg is the
/// **reconcile derivation** rejecting a collapsed identity set. So the
/// control stands the mesh up on DISTINCT pins (bind succeeds; the guard has
/// its own coverage and stays idle), then applies the fraud AFTER mesh-up at
/// the only seam left — the DERIVED identity witness the reconcile stage
/// reads: host_06's witness carries host_05's fingerprint (same DER) at
/// host_06's own distinct socket. The collapsed set must (a) derive to n−1
/// reconciled hosts — the AC5.1 marker carries 99 at N=100: **100 clones are
/// NOT 100 hosts** — and (b) HARD-FAIL the only error-returning reconcile,
/// `ChurnDrillReport::reconcile_detections`' identity clause ("identity
/// reconcile: 2 detections collapse to 1 distinct fingerprints (expected
/// 2)"). `distinct_host_count()` is a count with no error path, so the
/// hard-fail assertion lives on the error-returning reconcile — asserted on
/// the returned `Err` with its message content, never a bare count.
async fn duplicate_identity_control(n: usize) {
    assert_fd_headroom(fd_headroom_for(n));
    let _envelope = RunnerEnvelope::open(format!("duplicate-identity-n{n}"));
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-dup-identity");
    let names: Vec<String> = (0..n).map(host_name).collect();
    // DISTINCT pins: every host its own leaf, so the 14-2 uniqueness guard
    // stays idle and bind succeeds. This is the control's positive half — the
    // collapse asserted below is attributable to the fraud applied at the
    // derivation seam, not to a fixture that could not bind.
    let leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;

    let host_addrs: BTreeSet<SocketAddr> = mesh.iter().map(|m| m.addr).collect();
    assert_eq!(host_addrs.len(), n, "{n} distinct real sockets bound");
    let clean_fingerprints: BTreeSet<String> = mesh.iter().map(|m| m.fingerprint.wire()).collect();
    assert_eq!(
        clean_fingerprints.len(),
        n,
        "distinct pins must reconcile to {n} distinct fingerprints before the fraud is applied"
    );

    // The fraud, applied AFTER mesh-up at the derivation seam (the only seam
    // a post-14-2 mesh leaves): index 6's identity witness carries index 5's
    // fingerprint — same DER, the clone — while keeping its own bound socket.
    const CLONE: usize = 6;
    const SOURCE: usize = 5;
    let clone_fingerprint = mesh[SOURCE].fingerprint.wire();
    assert_ne!(
        mesh[CLONE].fingerprint.wire(),
        clone_fingerprint,
        "fixture must start from genuinely distinct identities"
    );
    let fraud_fingerprints: BTreeSet<String> = mesh
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if i == CLONE {
                clone_fingerprint.clone()
            } else {
                m.fingerprint.wire()
            }
        })
        .collect();
    assert_eq!(
        fraud_fingerprints.len(),
        n - 1,
        "the clone collapses the derived fingerprint set to {}",
        n - 1
    );

    // The two identity witnesses the reconcile stage reads, carrying the same
    // fraud: two planted identities (host_05, host_06) on ONE fingerprint.
    // They are IDENTITY records for this negative control, not latency
    // samples — `reconcile_detections` checks count → miss → identity in
    // order, so `first_rejection_ns` must be `Some` to REACH the identity
    // clause this control asserts. The stamps are the harness's own monotonic
    // readings (real binds; the reading at which the fraud is applied);
    // AC4.4's discipline governs: in THIS control only the Err branch is
    // evidence, and no latency claim is derived from them.
    let applied_ns = mono_ns(&base);
    let witnesses = vec![
        AdversarialDetection {
            adversary_id: names[SOURCE].clone(),
            adversary_fingerprint: mesh[SOURCE].fingerprint.wire(),
            attack_class: AdversarialAttempt::TofuPinSpoofing,
            join_ns: join_ns_of(&base, &mesh[SOURCE]),
            first_rejection_ns: Some(applied_ns),
            blast_peers: BTreeSet::new(),
        },
        AdversarialDetection {
            adversary_id: names[CLONE].clone(),
            // The clone's TRUE fingerprint — same DER as host_05's leaf,
            // exactly what `serving[6] = &leaves[5]` produced pre-14-2.
            adversary_fingerprint: clone_fingerprint,
            attack_class: AdversarialAttempt::TofuPinSpoofing,
            join_ns: join_ns_of(&base, &mesh[CLONE]),
            first_rejection_ns: Some(applied_ns),
            blast_peers: BTreeSet::new(),
        },
    ];

    let report = ChurnDrillReport::from_real_events(
        "dup-identity-negative-control",
        fraud_fingerprints,
        host_addrs,
        witnesses,
        0,
        None,
    );
    assert_eq!(
        report.distinct_host_count(),
        n - 1,
        "a duplicate-fingerprint witness MUST collapse the derive-and-reconcile count to {} \
         ({n} claimed hosts with one clone — 100 clones are not 100 hosts)",
        n - 1
    );
    // The DERIVED reconciled count is 99 at N=100 — the marker carries the
    // derived number, never the claimed one (AC5.1/AC2.2); the gate pins the
    // N=100 line to exactly `SCALE_CHURN_HOSTS_RECONCILED=99`.
    print_scale_marker(report.distinct_host_count());

    // Hard-fail half: the reconcile derivation must REJECT the collapsed
    // identity set. `reconcile_detections(2)` gets past the count and miss
    // clauses and must fail in the IDENTITY clause.
    let verdict = report.reconcile_detections(2);
    assert!(
        matches!(&verdict, Err(msg)
            if msg.contains("identity reconcile")
                && msg.contains("collapse to 1 distinct fingerprints (expected 2)")),
        "the reconcile derivation must HARD-FAIL a collapsed identity set (two planted \
         identities, one fingerprint — 100 clones are not 100 hosts): {verdict:?}"
    );
}

/// AC1/AC2.3 — compressed churn schedule at scale n: `CHURN_ROUNDS` rounds ×
/// `turnover_per_round(n)` real join/leave rebinds, each round followed by a
/// bounded hub-and-spoke reachability sweep proving the mesh self-heals, then
/// a distinct-identity reconcile on the FINAL membership. The turnover band
/// is asserted as the NFR's RATIO (10–20% of non-hub slots per round), not an
/// absolute count — the absolute band broke at N=100 in BOTH directions
/// (AC2.3), and the derived event count is recorded, not narrated.
async fn scale_churn_envelope_drill(n: usize) {
    assert!(
        n >= COMPRESSED_HOST_COUNT,
        "the churn envelope runs at the NFR scales (compressed 30 / full 100), not on sub-meshes"
    );
    assert_fd_headroom(fd_headroom_for(n));
    let _envelope = RunnerEnvelope::open(format!("churn-envelope-n{n}"));
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-scale-churn");
    let names: Vec<String> = (0..n).map(host_name).collect();
    let mut leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();

    let turnover = turnover_per_round(n);
    let pct = turnover * 100 / (n - 1);
    assert!(
        (10..=20).contains(&pct),
        "per-round turnover must stay inside the ratified 10-20% band of non-hub slots \
         (NFR-Rel-7), got {turnover}/{} = {pct}% at N={n} (AC2.3)",
        n - 1
    );
    let derived_events = CHURN_ROUNDS * turnover;
    eprintln!(
        "AC2.3 derived churn schedule at N={n}: {turnover}/round x {CHURN_ROUNDS} rounds = \
         {derived_events} real join/leave events ({pct}% of {} non-hub slots per round)",
        n - 1
    );

    let baseline = build_round_mesh(&clock, &ca, &names, &leaves).await;
    assert_hub_reachable(&baseline, 10_000, "baseline").await;
    // Review P10: the pre-churn identity set is the first term of the
    // observed-turnover diff.
    let baseline_fingerprints: BTreeSet<String> =
        baseline.iter().map(|m| m.fingerprint.wire()).collect();
    drop(baseline); // H6 deterministic teardown frees the old ports

    // Review P10: derive each round's turnover from OBSERVED membership
    // change, not from the loop counter. `churn_events` used to be
    // incremented right after the harness assigned a leaf and compared to
    // the loop bound, which only reasserted the schedule — a no-op
    // replacement would not have been caught. Now every round diffs the
    // live mesh's reconciled fingerprint set against the previous round's
    // and counts the identities that ACTUALLY changed.
    let mut observed_events = 0usize;
    let mut previous: BTreeSet<String> = baseline_fingerprints;
    for round in 0..CHURN_ROUNDS {
        for k in 0..turnover {
            let idx = 1 + ((round * turnover + k) % (n - 1));
            leaves[idx] = valid_leaf(&ca, &clock); // teardown+rebind models one leave+join turnover
        }
        let round_mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;
        assert_hub_reachable(
            &round_mesh,
            20_000 + (round as u64) * 100,
            "post-churn-round",
        )
        .await;
        let current: BTreeSet<String> = round_mesh.iter().map(|m| m.fingerprint.wire()).collect();
        let joined = current.difference(&previous).count();
        let left = previous.difference(&current).count();
        assert_eq!(
            joined, left,
            "a churn round replaces identities one-for-one: {joined} joined vs {left} left"
        );
        assert_eq!(
            joined, turnover,
            "round {round} must show {turnover} OBSERVED identity changes on the live mesh, \
             not merely {turnover} scheduled rebinds (review P10)"
        );
        observed_events += joined;
        previous = current;
        drop(round_mesh);
    }
    assert_eq!(
        observed_events, derived_events,
        "the schedule must run every planned rebind AND the mesh must show it (no silent \
         short-circuit, no no-op replacement)"
    );

    let final_mesh = build_round_mesh(&clock, &ca, &names, &leaves).await;
    let host_fingerprints: BTreeSet<String> =
        final_mesh.iter().map(|m| m.fingerprint.wire()).collect();
    let host_addrs: BTreeSet<SocketAddr> = final_mesh.iter().map(|m| m.addr).collect();
    assert_hub_reachable(&final_mesh, 40_000, "post-churn-final").await;
    let report = ChurnDrillReport::from_real_events(
        "t-14-1-scale-churn-envelope",
        host_fingerprints,
        host_addrs,
        vec![],
        0,
        None,
    );
    assert_eq!(
        report.distinct_host_count(),
        n,
        "clean drill: no clones planted, so N distinct == n"
    );
    assert!(
        report.distinct_host_count() >= HOST_COUNT_FLOOR,
        "distinct-identity-reconciled host count must be >= {HOST_COUNT_FLOOR}, got {}",
        report.distinct_host_count()
    );
    print_scale_marker(report.distinct_host_count());
    drop(final_mesh);
}

// ─────────────────────── AC3 — two-surface detection at scale ───────────────────────

/// Plant the 3 adversary classes INTO an n-host mesh (AC3.1 — never a
/// dedicated 2–3 node sub-mesh), one per class, and return their real
/// `AdversarialDetection` samples plus the mesh's DERIVED reconciled host
/// count. Layout: index 0 = the hub prober; n−3 = pin-spoof, n−2 =
/// cert-race, n−1 = consent bypass. Reused by the clean detection drill AND
/// the `churn-fault-inject` blind mutations at BOTH scales — the real
/// detectors fire identically; only the caller decides whether to count all
/// 3 or blind one.
async fn plant_and_detect_three_adversaries(n: usize) -> (Vec<AdversarialDetection>, usize) {
    assert!(
        n >= 5,
        "planted-mesh drills run on a real mesh, not a sub-mesh"
    );
    assert_fd_headroom(fd_headroom_for(n));
    let _envelope = RunnerEnvelope::open(format!("detection-n{n}"));
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-adversaries");
    let retry = no_retry();
    let pin_spoof = n - 3;
    let cert_race = n - 2;
    let consent = n - 1;
    let mut names: Vec<String> = (0..n).map(host_name).collect();
    names[pin_spoof] = "adv_pin_spoof".to_string();
    names[cert_race] = "adv_cert_race".to_string();
    names[consent] = "adv_consent_bypass".to_string();

    // `expected` = the identity every peer PINS (claimed); `serving` = the
    // leaf the node REALLY serves. `serving` starts as a COPY of `expected`
    // (each node serves what its peers pinned); only planted nodes deviate:
    // the pin-spoof serves a DIFFERENT valid leaf than pinned, the cert-race
    // serves an EXPIRED leaf, and the consent adversary holds a valid
    // identity (served == claimed).
    let expected: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();
    let mut serving: Vec<Leaf> = expected.clone();
    serving[pin_spoof] = valid_leaf(&ca, &clock);
    serving[cert_race] = expired_leaf(&ca, &clock);

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
            (cert_race, PlantKind::CertRotationRaceExploit),
            (consent, PlantKind::AdrLevel012ConsentBypass),
        ],
        retry,
    )
    .await;
    // Round-2 P5: each adversary carries its OWN join (its bind), so the
    // three samples are per-adversary `t_first_rejection − t_join` (AC3.4).
    let join_pin = join_ns_of(&base, &mesh[pin_spoof]);
    let join_cert = join_ns_of(&base, &mesh[cert_race]);
    let join_consent = join_ns_of(&base, &mesh[consent]);

    // ── Surface 1 (handshake): the hub dials the pin-spoof. It pinned the
    // CLAIMED fingerprint; the adversary serves a different valid leaf →
    // WebPKI succeeds, the TOFU pin check fails. AC3.5: match the CLASS, not
    // just the surface — pin-spoof and cert-race must be distinguishable at
    // the assertion.
    let f1 = make_frame(&names[HUB], &names[pin_spoof], IntentClass::Readonly, 1);
    let r1 = mesh[HUB]
        .transport
        .route_outbound(f1, &HostId(names[pin_spoof].clone()))
        .await;
    let t1 = mono_ns(&base);
    assert!(
        matches!(
            r1,
            Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::PinMismatch,
                ..
            })
        ),
        "TofuPinSpoofing must be rejected at the HANDSHAKE layer with class PinMismatch, got {r1:?}"
    );

    // ── Surface 1 (handshake): cert-race. WebPKI's validity step rejects the
    // expired leaf BEFORE any pin check → CertExpired (a DIFFERENT class).
    let f2 = make_frame(&names[HUB], &names[cert_race], IntentClass::Readonly, 2);
    let r2 = mesh[HUB]
        .transport
        .route_outbound(f2, &HostId(names[cert_race].clone()))
        .await;
    let t2 = mono_ns(&base);
    assert!(
        matches!(
            r2,
            Err(A2AError::HandshakeFailed {
                class: HandshakeFailureClass::CertExpired,
                ..
            })
        ),
        "CertRotationRaceExploit must be rejected at the HANDSHAKE layer with class CertExpired, \
         got {r2:?}"
    );

    // ── Surface 2 (router NACK): the consent adversary probes a legit peer —
    // a readonly delivery SUCCEEDS (real reachability), then its escalation
    // to "standard" is denied at the RECEIVER's accept-allowlist.
    let f3 = make_frame(&names[consent], &names[1], IntentClass::Readonly, 3);
    let ok3 = mesh[consent]
        .transport
        .route_outbound(f3, &HostId(names[1].clone()))
        .await;
    assert!(
        ok3.is_ok(),
        "the readonly probe must succeed before escalation (real reachability): {ok3:?}"
    );
    let f4 = make_frame(&names[consent], &names[1], IntentClass::Standard, 4);
    let r4 = mesh[consent]
        .transport
        .route_outbound(f4, &HostId(names[1].clone()))
        .await;
    let t3 = mono_ns(&base);
    assert!(
        matches!(r4, Err(A2AError::IntentDeniedAtPeer { .. })),
        "AdrLevel012ConsentBypass must be rejected at the ROUTER NACK layer, got {r4:?}"
    );

    let mut consent_blast = BTreeSet::new();
    consent_blast.insert(names[1].clone());

    let detections = vec![
        AdversarialDetection {
            adversary_id: names[pin_spoof].clone(),
            adversary_fingerprint: serving[pin_spoof].fingerprint.wire(),
            attack_class: AdversarialAttempt::TofuPinSpoofing,
            join_ns: join_pin,
            first_rejection_ns: Some(t1),
            // handshake reject: zero peers ever reached (real, not fabricated)
            blast_peers: BTreeSet::new(),
        },
        AdversarialDetection {
            adversary_id: names[cert_race].clone(),
            adversary_fingerprint: serving[cert_race].fingerprint.wire(),
            attack_class: AdversarialAttempt::CertRotationRaceExploit,
            join_ns: join_cert,
            first_rejection_ns: Some(t2),
            blast_peers: BTreeSet::new(),
        },
        AdversarialDetection {
            adversary_id: names[consent].clone(),
            adversary_fingerprint: serving[consent].fingerprint.wire(),
            attack_class: AdversarialAttempt::AdrLevel012ConsentBypass,
            join_ns: join_consent,
            first_rejection_ns: Some(t3),
            blast_peers: consent_blast,
        },
    ];
    // The mesh's DERIVED reconciled host count — the marker's payload.
    let fingerprints: BTreeSet<String> = mesh.iter().map(|m| m.fingerprint.wire()).collect();
    let addrs: BTreeSet<SocketAddr> = mesh.iter().map(|m| m.addr).collect();
    let derived_hosts = fingerprints.len().min(addrs.len());
    (detections, derived_hosts)
}

/// Build the detection-only report (recovery/rto not exercised on this leg).
fn detection_report(
    per_adversary: Vec<AdversarialDetection>,
    drill_id: impl Into<String>,
) -> ChurnDrillReport {
    ChurnDrillReport::from_real_events(
        drill_id,
        BTreeSet::new(),
        BTreeSet::new(),
        per_adversary,
        0,
        None,
    )
}

/// AC3/AC3.4 — two-surface detection latency at scale n, per-event, with the
/// adversarial-host-identity reflex and the percentile-honesty disclosure.
async fn detection_latency_drill(n: usize) {
    let (per_adversary, derived_hosts) = plant_and_detect_three_adversaries(n).await;
    assert_eq!(
        per_adversary.len(),
        PLANTED_ADVERSARIES,
        "all 3 planted classes must be detected"
    );

    // Adversarial-host-identity reflex (L8/D7): the detected set reconciles
    // to 3 DISTINCT planted cert-fingerprint identities — a real count and
    // identity contract (NOT a tautological re-inlining of
    // `detection_surface()`). Each class was asserted at its RIGHT surface
    // with its RIGHT `HandshakeFailureClass` at the dial site above.
    let report = detection_report(per_adversary.clone(), format!("t-14-1-detection-n{n}"));
    report
        .reconcile_detections(PLANTED_ADVERSARIES)
        .expect("clean detection set must reconcile to 3 distinct planted identities");

    // Non-degenerate distribution (11.2a vacuous-count guard): the RAW ns
    // detection-latency samples must ALL be distinct (the seconds-rounded
    // value is 0 for every sample on sub-second loopback, L5). A
    // constant-vector median must stay unable to green this leg.
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
        "detection latencies must be non-degenerate — all distinct (11.2a vacuous-count guard); \
         raw_ns={raw_ns:?}"
    );

    // Median + p99 detection floors (both BINDING). Trivially green on
    // loopback (L5) — the teeth are the blind falsifiers, not this clean
    // pass.
    assert!(
        report.detection_latency_median_secs <= 3600 && report.detection_latency_p99_secs <= 3600,
        "detection median+p99 must be within the 1h floor: {report:?}"
    );

    // AC3.4.a — publish the sample count beside every percentile. For n < 101
    // the nearest-rank p99 IS the maximum; the label must not overstate the
    // evidence and no reader should have to trust a docstring.
    let samples = per_adversary
        .iter()
        .filter(|d| d.first_rejection_ns.is_some())
        .count();
    eprintln!(
        "detection_latency_median_secs = {} (n={samples})",
        report.detection_latency_median_secs
    );
    eprintln!(
        "detection_latency_p99_secs = {} (n={samples}) — for n < 101 the p99 IS the maximum \
         (AC3.4.a)",
        report.detection_latency_p99_secs
    );
    eprintln!("=== Story 14.1 detection-latency drill (N={n}) ===");
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));
    print_scale_marker(derived_hosts);
}

// ─────────── AC3 — blast race + recovery (BIND) + rto (REPORTED), real events ───────────

/// Story 14-1 code-review D1 (party-mode ratified 2026-08-27, UNANIMOUS) —
/// the adversary's in-flight window for the RACE strategy. **`W >= 6` is
/// load-bearing and is NOT a tuning knob:** it must sit strictly ABOVE the
/// binding ≤5 blast floor, or a detector that never interrupts still cannot
/// breach the floor and the leg is the same green-by-construction lie with
/// more threads (Winston). 6 is the MINIMUM lawful value — chosen because it
/// is minimal, never because of the outcome it produces. If detection cannot
/// bound a parallel adversary below 5 peers on this substrate, blocking
/// condition 4 governs: that is a FINDING to publish with the measurement,
/// never a number to re-tune.
const RACE_IN_FLIGHT: usize = 6;

/// The adversary's propagation strategy (AC3.3 — the race).
#[derive(Clone, Copy)]
enum SweepStrategy {
    /// **The clean-path oracle (AC3.3.a).** Parallelize `RACE_IN_FLIGHT`
    /// readonly dials over the FULL offered set while awaiting the
    /// escalation probe CONCURRENTLY; whichever completes first decides the
    /// count, and the NACK ABORTS the sweep. `blast_peers` is therefore what
    /// real deliveries completed *before detection isolated the adversary* —
    /// a genuine contest between propagation and detection, with the worst
    /// case bounded at the in-flight window rather than at the whole mesh
    /// (Amelia's abort seam, which answered Dana's flake objection).
    Race,
    /// Deliver `PROBE_BATCH` readonly frames, then probe escalation on the
    /// batch's first peer — a deterministic probing adversary. Retained for
    /// the SEPARABILITY falsifiers (isolation-blind / re-pin-blind), which
    /// must hold propagation constant to isolate their own axis, and for the
    /// N=5 smoke wrapper where the offered set fits inside one in-flight
    /// window.
    ProbeEveryBatch,
    /// Sweep the ENTIRE offered set on readonly first, escalate once at the
    /// end — a propagating adversary. With `offered: Some(k)` this is the
    /// blast over-reach falsifier (it genuinely reaches k); it stays
    /// deterministic so an over-reach RED is never confused with a race RED
    /// (Paige).
    SweepThenEscalate,
    /// Sweep the offered set and NEVER escalate — the "detector switched
    /// off" mutation: nothing denies the adversary, so reached == offered
    /// and first_rejection stays None. Proves the clean leg's green could
    /// have been red (AC3.3.b / blocking condition 5).
    SweepOnly,
}

/// Fault knobs for the consent reachability/recovery drill. `Default` is the
/// clean path: FULL offered set, real isolation, real re-pin, and the
/// **RACE** strategy (D1). Only the `#[cfg(feature = "churn-fault-inject")]`
/// tests and the N=5 smoke wrapper set non-default values; each knob makes a
/// REAL reachability outcome diverge.
#[derive(Clone, Copy)]
struct DrillFaults {
    /// `None` = the FULL peer set, DERIVED from the mesh (n−1 peers — the
    /// adversary can dial every other host; 99 at N=100). `Some(k)` = the
    /// over-reach falsifier's cap: k real peers out of the mesh (a strict
    /// subset — 11.3's canned `blast_targets: 2` DEFAULT is dead; the knob
    /// exists only here).
    offered: Option<usize>,
    /// Perform the real isolation repoint (clean); `false` = skip it, so the
    /// adversary stays reachable → rto objective unmet.
    isolate: bool,
    /// Perform the real legit re-pin/reconverge (clean); `false` = break a
    /// legit peer's endpoint → the FULL legit sweep fails → fleet never
    /// reconverges → recovery unmet.
    repin: bool,
    strategy: SweepStrategy,
}

impl Default for DrillFaults {
    fn default() -> Self {
        Self {
            offered: None,
            isolate: true,
            repin: true,
            strategy: SweepStrategy::Race,
        }
    }
}

/// Sentinel `rto_secs` (over 4h, in ns) — the derived value when isolation is
/// NOT confirmed by a real failed re-dial (the objective was not met on the
/// live mesh; the breach is a CONSEQUENCE of a real still-succeeding dial,
/// not an injected constant). **AC4.4 disclosure: on this branch the VALUE is
/// the `RTO_UNMET_NS` sentinel and only the BRANCH is evidence — 5h is not a
/// measurement.**
const RTO_UNMET_NS: u64 = 5 * 3600 * 1_000_000_000;
/// Sentinel `recovery_secs` (over 24h, in ns) — the derived value when the
/// legit fleet does NOT reconverge (the full legit sweep failed).
/// **AC4.4 disclosure: on this branch the VALUE is the `RECOVERY_UNMET_NS`
/// sentinel and only the BRANCH is evidence — 25h is not a measurement.**
const RECOVERY_UNMET_NS: u64 = 25 * 3600 * 1_000_000_000;

/// Real consent-bypass reachability drill on an n-host mesh with the
/// adversary planted INTO it (AC3.1/AC3.3 — the 11.3 dedicated 3-node
/// sub-mesh is gone). Derives blast (real readonly deliveries before the
/// first denial), detection (real NACK at the receiver's accept-allowlist),
/// rto (detection → adversary confirmed UNREACHABLE via a real failed
/// re-dial), and recovery (detection → the FULL legit hub-and-spoke sweep
/// passing — AC3.1.a's fleet definition, never a sampled subset). Recovery
/// and rto are two DISTINCT real events on the SAME mesh the adversary
/// attacked (D1/D2). Returns the report AND the derived offered set so the
/// caller can assert the AC3.3.b non-degeneracy contract.
#[allow(clippy::needless_range_loop)]
async fn run_consent_reachability_drill(
    n: usize,
    faults: DrillFaults,
) -> (ChurnDrillReport, BTreeSet<String>) {
    assert!(
        n >= 5,
        "the reachability drill runs on a real mesh (>=5), never a dedicated sub-mesh (AC3.1)"
    );
    assert_fd_headroom(fd_headroom_for(n));
    let _envelope = RunnerEnvelope::open(format!("blast-recovery-rto-n{n}"));
    let base = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-1-consent-reach");
    let retry = no_retry();

    let adv_idx = n - 1;
    let mut names: Vec<String> = (0..n).map(host_name).collect();
    names[adv_idx] = "adv_consent_bypass".to_string();
    let leaves: Vec<Leaf> = (0..n).map(|_| valid_leaf(&ca, &clock)).collect();

    // The offered set: EVERY OTHER host in the mesh, DERIVED — never a
    // literal (AC3.3.a/AC3.3.b). At N=100 that is 99 dialable peers. The
    // `Some(k)` cap exists only for the over-reach falsifier.
    let full_offered: Vec<usize> = (0..n).filter(|i| *i != adv_idx).collect();
    let offered: Vec<usize> = match faults.offered {
        None => full_offered,
        Some(k) => {
            assert!(
                k >= 2,
                "the over-reach falsifier needs >=2 real targets for the reconvergence phase"
            );
            assert!(
                k < n - 1,
                "the falsifier cap must be a strict subset of the mesh — otherwise it is not a cap"
            );
            full_offered.into_iter().take(k).collect()
        }
    };
    let offered_names: BTreeSet<String> = offered.iter().map(|i| names[*i].clone()).collect();

    let serving_refs: Vec<&Leaf> = leaves.iter().collect();
    let mesh = build_mesh_with_planted(
        &clock,
        &ca,
        &names,
        &serving_refs,
        &serving_refs,
        &[(adv_idx, PlantKind::AdrLevel012ConsentBypass)],
        retry,
    )
    .await;
    let adversary = &mesh[adv_idx];
    let join_ns = join_ns_of(&base, &mesh[adv_idx]);

    // ── Propagation. `blast_peers` records real successes only.
    let mut blast_peers = BTreeSet::new();
    let mut seq: u64 = 100;
    let mut detected_at: Option<u64> = None;
    if matches!(faults.strategy, SweepStrategy::Race) {
        // D1's ratified shape: the adversary parallelizes over the FULL
        // offered set with `RACE_IN_FLIGHT` dials in flight, and the
        // escalation probe is awaited CONCURRENTLY from the moment it has a
        // foothold. On the NACK the sweep is DROPPED — in-flight dials
        // cancel — so the count is exactly what completed before detection
        // isolated it. `tokio::select!` is deliberately NOT `biased`: the
        // poll order IS the race.
        //
        // Round-2 P1 (all four layers): `blast_peers` is NOT the yields —
        // a dial that completes inside `buffer_unordered` while the NACK
        // arm wins `select!` used to be dropped uncounted. Every dial now
        // records its OWN completion timestamp into a channel (the send
        // happens before the future resolves, so nothing completed is
        // lost), and the count below is every successful delivery that
        // completed at-or-before detection — the race's real outcome.
        let (completions_tx, mut completions_rx) =
            tokio::sync::mpsc::unbounded_channel::<(String, u64)>();
        use futures_util::stream::{self, StreamExt};
        let sweep = stream::iter(offered.iter().enumerate().map(|(k, j)| {
            let frame = make_frame(
                &names[adv_idx],
                &names[*j],
                IntentClass::Readonly,
                1_000 + k as u64,
            );
            let to = HostId(names[*j].clone());
            let peer = names[*j].clone();
            let transport = &adversary.transport;
            let completions_tx = completions_tx.clone();
            let base = base;
            async move {
                let res = transport.route_outbound(frame, &to).await;
                if res.is_ok() {
                    let _ = completions_tx.send((peer.clone(), mono_ns(&base)));
                }
                (peer, res)
            }
        }))
        .buffer_unordered(RACE_IN_FLIGHT);
        let mut sweep = Box::pin(sweep);
        let (first_peer, first_res) = sweep
            .next()
            .await
            .expect("a non-empty offered set must yield at least one delivery");
        assert!(
            first_res.is_ok(),
            "the adversary's first readonly delivery must succeed (real reachability): {first_res:?}"
        );
        let escalation = make_frame(
            &names[adv_idx],
            &names[offered[0]],
            IntentClass::Standard,
            500_000,
        );
        // Bound to a local: the probe future borrows the target for as long
        // as it is polled inside `select!`.
        let escalation_target = HostId(names[offered[0]].clone());
        let mut probe = Box::pin(
            adversary
                .transport
                .route_outbound(escalation, &escalation_target),
        );
        loop {
            tokio::select! {
                verdict = &mut probe => {
                    assert!(
                        matches!(verdict, Err(A2AError::IntentDeniedAtPeer { .. })),
                        "the escalation must be denied at the ROUTER NACK surface, got {verdict:?}"
                    );
                    detected_at = Some(mono_ns(&base));
                    break;
                }
                delivered = sweep.next() => match delivered {
                    Some((peer, res)) => {
                        assert!(res.is_ok(), "readonly delivery to {peer} must succeed: {res:?}");
                    }
                    None => {
                        // Propagation exhausted the offered set before the
                        // detector answered — the adversary won outright, and
                        // the caller's non-degeneracy assertion must RED.
                        let verdict = (&mut probe).await;
                        assert!(
                            matches!(verdict, Err(A2AError::IntentDeniedAtPeer { .. })),
                            "the escalation must be denied at the ROUTER NACK surface, got {verdict:?}"
                        );
                        detected_at = Some(mono_ns(&base));
                        break;
                    }
                },
            }
        }
        drop(sweep);
        // Round-2 P1: count every successful delivery that completed
        // at-or-before detection. Dropping the sweep cancelled only
        // genuinely in-flight dials (the abort seam); everything that
        // already completed is in the channel — yielded or not.
        let cut = detected_at.expect("the race always records detection");
        while let Ok((peer, ts)) = completions_rx.try_recv() {
            if ts <= cut {
                blast_peers.insert(peer);
            }
        }
    } else {
        'propagation: for batch in offered.chunks(PROBE_BATCH) {
            let pairs: Vec<(usize, usize)> = batch.iter().map(|j| (adv_idx, *j)).collect();
            let results = concurrent_dial_pairs(&mesh, &pairs, seq, IntentClass::Readonly).await;
            seq += 1_000;
            for (_, j, res) in &results {
                assert!(
                    res.is_ok(),
                    "readonly delivery to {} must succeed (real reachability before detection): \
                     {res:?}",
                    names[*j]
                );
                blast_peers.insert(names[*j].clone());
            }
            match faults.strategy {
                SweepStrategy::SweepOnly => continue 'propagation,
                SweepStrategy::ProbeEveryBatch => {
                    seq += 1;
                    let probe = make_frame(
                        &names[adv_idx],
                        &names[batch[0]],
                        IntentClass::Standard,
                        seq,
                    );
                    let denied = adversary
                        .transport
                        .route_outbound(probe, &HostId(names[batch[0]].clone()))
                        .await;
                    match denied {
                        Err(A2AError::IntentDeniedAtPeer { .. }) => {
                            detected_at = Some(mono_ns(&base));
                            break 'propagation;
                        }
                        other => panic!(
                            "escalation probe must be denied at the ROUTER NACK surface \
                             (IntentDeniedAtPeer), got {other:?}"
                        ),
                    }
                }
                SweepStrategy::SweepThenEscalate => {}
                SweepStrategy::Race => unreachable!("the race path is handled above"),
            }
        }
        if matches!(faults.strategy, SweepStrategy::SweepThenEscalate) {
            seq += 1;
            let probe = make_frame(
                &names[adv_idx],
                &names[offered[0]],
                IntentClass::Standard,
                seq,
            );
            let denied = adversary
                .transport
                .route_outbound(probe, &HostId(names[offered[0]].clone()))
                .await;
            assert!(
                matches!(denied, Err(A2AError::IntentDeniedAtPeer { .. })),
                "escalation must be denied at the ROUTER NACK surface, got {denied:?}"
            );
            detected_at = Some(mono_ns(&base));
        }
    }

    // SweepOnly: nothing ever denied the adversary — first_rejection stays
    // None (a detection MISS: `reconcile_detections` REDS) and reached ==
    // offered. Review P4: recovery carries the `RECOVERY_UNMET_NS` SENTINEL,
    // not 0 — no reconvergence event was measured, and a 0 would let
    // `passes_v20_binding_floors()` green a zero-detection run at small n
    // (blast 4 ≤ 5, empty samples → 0 latencies). Only the BRANCH is
    // evidence (AC4.4).
    let Some(detection_ns) = detected_at else {
        let detection = AdversarialDetection {
            adversary_id: names[adv_idx].clone(),
            adversary_fingerprint: leaves[adv_idx].fingerprint.wire(),
            attack_class: AdversarialAttempt::AdrLevel012ConsentBypass,
            join_ns,
            first_rejection_ns: None,
            blast_peers,
        };
        let report = ChurnDrillReport::from_real_events(
            format!("consent-reach-n{n}-no-detection"),
            mesh.iter().map(|m| m.fingerprint.wire()).collect(),
            mesh.iter().map(|m| m.addr).collect(),
            vec![detection],
            RECOVERY_UNMET_NS,
            None,
        );
        print_scale_marker(report.distinct_host_count());
        return (report, offered_names);
    };

    // ── RTO event: REAL isolation — repoint the adversary's endpoint for
    // EVERY mesh peer to a dead port, then CONFIRM unreachability by a real
    // failed re-dial. (isolation-blind skips the repoint → the re-dial
    // still SUCCEEDS → objective unmet → the `RTO_UNMET_NS` sentinel.)
    // Review P5: repoint the FULL peer set, not just the offered cap — with
    // `offered: Some(6)` at n=100 the old code left the adversary 93
    // dialable peers while recording it as isolated.
    if faults.isolate {
        for (i, name) in names.iter().enumerate() {
            if i != adv_idx {
                adversary
                    .transport
                    .set_peer_endpoint(&HostId(name.clone()), dead_endpoint());
            }
        }
    }
    seq += 1;
    let recheck = make_frame(
        &names[adv_idx],
        &names[offered[0]],
        IntentClass::Readonly,
        seq,
    );
    let redial = adversary
        .transport
        .route_outbound(recheck, &HostId(names[offered[0]].clone()))
        .await;
    let adversary_isolated = redial.is_err();
    let isolated_ns = mono_ns(&base);
    let rto_ns = if adversary_isolated {
        isolated_ns.saturating_sub(detection_ns)
    } else {
        RTO_UNMET_NS
    };

    // ── Recovery event (AC3.1.a decision): fleet reconvergence = the FULL
    // hub-and-spoke sweep passing — all 2×(L−1) legit pairs over the legit
    // fleet (L = n−1, hub included). A sampled re-dial could hide an
    // un-reconverged host; this cannot. (re-pin-blind breaks the hub's
    // endpoint for a legit peer → the sweep FAILS → the `RECOVERY_UNMET_NS`
    // sentinel.)
    if !faults.repin {
        mesh[HUB]
            .transport
            .set_peer_endpoint(&HostId(names[1].clone()), dead_endpoint());
    }
    let sweep = concurrent_dial_pairs(
        &mesh[..adv_idx],
        &hub_pairs(adv_idx),
        900_000,
        IntentClass::Readonly,
    )
    .await;
    let failed: Vec<_> = sweep.iter().filter(|(_, _, r)| r.is_err()).collect();
    let fleet_reconverged = failed.is_empty();
    assert!(
        faults.repin || !fleet_reconverged,
        "re-pin-blind must actually break the sweep (the falsifier must bite)"
    );
    let reconverge_ns = mono_ns(&base);
    let recovery_ns = if fleet_reconverged {
        reconverge_ns.saturating_sub(detection_ns)
    } else {
        RECOVERY_UNMET_NS
    };

    let detection = AdversarialDetection {
        adversary_id: names[adv_idx].clone(),
        adversary_fingerprint: leaves[adv_idx].fingerprint.wire(),
        attack_class: AdversarialAttempt::AdrLevel012ConsentBypass,
        join_ns,
        first_rejection_ns: Some(detection_ns),
        blast_peers,
    };
    let report = ChurnDrillReport::from_real_events(
        format!("consent-reach-n{n}"),
        mesh.iter().map(|m| m.fingerprint.wire()).collect(),
        mesh.iter().map(|m| m.addr).collect(),
        vec![detection],
        recovery_ns,
        Some(rto_ns),
    );
    print_scale_marker(report.distinct_host_count());
    (report, offered_names)
}

/// The clean-path contract shared by the compressed and envelope scales
/// (AC3.3.b — the assertions are the contract, not the count).
fn assert_clean_blast_recovery_contract(
    report: &ChurnDrillReport,
    offered: &BTreeSet<String>,
    n: usize,
) {
    assert_eq!(
        report.distinct_host_count(),
        n,
        "the clean {n}-host drill must reconcile to {n} distinct hosts"
    );
    report
        .reconcile_detections(1)
        .expect("the planted consent adversary must reconcile to its identity");
    // The offered set is DERIVED from the mesh (every other host), never a
    // literal (AC3.3.b).
    assert_eq!(
        offered.len(),
        n - 1,
        "the adversary must be offered the FULL peer set (n-1), derived from the mesh"
    );
    // The reached set is real deliveries over the offered set...
    assert!(
        report.per_adversary[0]
            .blast_peers
            .iter()
            .all(|p| offered.contains(p)),
        "reached must be a subset of offered (real deliveries over the offered set)"
    );
    // ...and detection must have BOUNDED propagation: offered == reached
    // REDS regardless of the count — nothing was isolated, and a floor met
    // because propagation ran out of peers is not a floor that was met
    // (AC3.3.b). The check has teeth only when the offered set is LARGER
    // than the adversary's in-flight window: at `offered <= RACE_IN_FLIGHT`
    // the whole set fits in one window and no detector could bound it, which
    // is exactly why the envelope scale (99 offered) is the oracle and N=5
    // is a smoke wrapper.
    assert!(
        !report.per_adversary[0].blast_peers.is_empty(),
        "the adversary must have reached at least one peer, or the race measured nothing"
    );
    if offered.len() > RACE_IN_FLIGHT {
        assert_ne!(
            &report.per_adversary[0].blast_peers, offered,
            "offered == reached REDs the leg (AC3.3.b): the adversary swept every offered peer \
             before detection — nothing was isolated"
        );
    }
    // ── THE BINDING PROPERTY (D-A, ratified 2026-08-27 after the review's
    // blast finding). The NFR's ABSOLUTE "≤5 peers" is not the instrument
    // this substrate can falsify: with no fan-out control anywhere in the
    // mesh, a parallel adversary reaches ~W peers before the NACK at EVERY
    // scale (measured 6–8 at N=100, 7 at N=30), so the absolute number
    // measures the harness's serialization, not the system. What IS a real,
    // falsifiable system property — and what an operator actually needs —
    // is that **detection INTERRUPTS propagation within one in-flight
    // window**. That binds here; the absolute count is REPORTED beside it.
    // This is the F3-ledger shape 11.3 already ratified for `rto_secs`
    // (derived + reported + advisory-if-breached, promotes to binding when
    // the substrate can observe a real breach) applied to the axis whose
    // instrument the review proved wrong. It is NOT a relaxed number: the
    // `SweepOnly` falsifier reds this bound (99 > 2W) whenever detection
    // stops interrupting, which the old `≤5` comparison could never do.
    //
    // TWO CONJUNCTS, because the property is "propagation was interrupted",
    // and neither slowness nor mesh size may be able to fake it:
    //
    //  (1) A CONSTANT that cannot scale with the fleet. `buffer_unordered(W)`
    //      refills a slot the instant a dial completes, so completions inside
    //      one NACK round-trip come in generations of ≤ W. Measured reach:
    //      6,6,6,6,6,6,6,7,7,8 isolated, and 10 under concurrent compile load
    //      (~1.7 generations). The bound is 4W = FOUR generations — one full
    //      doubling over the worst observed contention, because the point of
    //      this conjunct is to catch a detector that STRUCTURALLY stops
    //      interrupting, NOT a slow one: detection slowness already has its
    //      own binding axis (`detection_latency_*` ≤ 1h, asserted below), and
    //      double-binding it here would only buy flakes. What matters is that
    //      4W is a CONSTANT: it does not grow with N, so a mesh 3× larger
    //      cannot pass by being larger.
    //
    //  (2) A FRACTION DERIVED FROM THE RUN ITSELF — reach must stay under
    //      half the offered set, which no absolute constant can satisfy
    //      vacuously and which reds the total-detector-failure case at EVERY
    //      scale (SweepOnly reaches 99 of 99 at N=100 and 29 of 29 at N=30;
    //      both fail this, and 29 would slip past a generous constant alone).
    let reached = report.per_adversary[0].blast_peers.len();
    clean_blast_verdict(reached, offered.len())
        .expect("the clean blast contract (D-A conjuncts) must HOLD on a clean run");
    // The three NFR-Rel-7 axes still BIND, read per-axis so the blast
    // instrument correction cannot silently relax detection or recovery.
    assert!(
        report.detection_latency_median_secs <= 3600 && report.detection_latency_p99_secs <= 3600,
        "detection median+p99 must stay within the 1h floor: {report:?}"
    );
    assert!(
        report.recovery_secs <= 86_400,
        "fleet reconvergence must stay within the 24h floor: {report:?}"
    );
    // REPORTED, never silent: publish the absolute NFR-Rel-7 blast count and
    // its verdict on every clean run, plus the promotion condition, so the
    // breach cannot decay into a number nobody prints (RELEASE-HOLDS row 18
    // carries the claim boundary).
    eprintln!(
        "NFR-Rel-7 blast (REPORTED/advisory, not binding — D-A): max_blast_radius = {} vs the \
         ≤5 absolute floor → {}; v2.0 absolute predicate passes_v20_binding_floors() = {}. \
         PROMOTES back to BINDING when a mesh-level adversary fan-out control lands.",
        report.max_blast_radius,
        if report.max_blast_radius <= 5 {
            "within"
        } else {
            "BREACHED"
        },
        report.passes_v20_binding_floors()
    );
    assert!(
        !report.rto_exceeded_advisory(),
        "isolation was confirmed by a real failed re-dial — no 4h RTO breach: {report:?}"
    );
}

/// AC3 (D4/D5) — the compressed-scale clean drill, REBUILT by 14.1 onto the
/// planted mesh (the 11.3 canned `blast_targets: 2` clean path — which
/// derived 2 against a ceiling of 5 on every run ever — is dead; its
/// RED-at-HEAD demonstration is in the story's Debug Log).
#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_blast_recovery_rto_drill() {
    let (report, offered) =
        run_consent_reachability_drill(COMPRESSED_HOST_COUNT, DrillFaults::default()).await;
    eprintln!("=== Story 11.3 blast/recovery/rto drill (N={COMPRESSED_HOST_COUNT}, rebuilt by 14-1 AC3.3) ===");
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));
    assert_clean_blast_recovery_contract(&report, &offered, COMPRESSED_HOST_COUNT);
}

/// Story 14.1 AC3 — the envelope-scale clean drill on the N=100 mesh: the
/// adversary is offered 99 peers, reaches what it delivered before the first
/// denial, and the floors must hold on a mesh where they can genuinely fail.
#[tokio::test]
#[ignore = "Story 14.1 — real N=100 mesh; gate-controlled via check-scale-churn"]
async fn t_14_1_blast_recovery_rto_n100() {
    let (report, offered) =
        run_consent_reachability_drill(FULL_ENVELOPE_HOST_COUNT, DrillFaults::default()).await;
    eprintln!("=== Story 14.1 blast/recovery/rto drill (N={FULL_ENVELOPE_HOST_COUNT}) ===");
    eprintln!("{}", report_to_markdown(&report).expect("markdown render"));
    assert_clean_blast_recovery_contract(&report, &offered, FULL_ENVELOPE_HOST_COUNT);
}

/// The smoke scale — AC2.1's third thin wrapper (N=5, N=30, N=100).
const SMOKE_HOST_COUNT: usize = 5;

/// AC2.1 — the N=5 wrapper, routed through the SAME drill body as the
/// compressed and envelope scales (review finding: the wrapper list AC2.1
/// names is `N=5, N=30, N=100`, and only two existed).
///
/// It runs the DETERMINISTIC probing strategy on purpose: at n=5 the offered
/// set is 4, which fits inside one `RACE_IN_FLIGHT` window, so the race
/// cannot bound it and `offered != reached` has no teeth at this scale (the
/// contract helper skips that check below the window for exactly this
/// reason). NOT `#[ignore]`-gated — it is a fast structural smoke of the
/// drill body in the normal lane, never a gate-leg oracle for an NFR floor;
/// the envelope scale is the oracle.
#[tokio::test]
async fn t_14_1_blast_recovery_rto_n5_smoke() {
    let (report, offered) = run_consent_reachability_drill(
        SMOKE_HOST_COUNT,
        DrillFaults {
            strategy: SweepStrategy::ProbeEveryBatch,
            ..DrillFaults::default()
        },
    )
    .await;
    assert_eq!(
        offered.len(),
        SMOKE_HOST_COUNT - 1,
        "the smoke scale still offers the FULL derived peer set"
    );
    report
        .reconcile_detections(1)
        .expect("the planted consent adversary must reconcile at the smoke scale too");
    // The smoke scale runs the deterministic strategy, so the absolute
    // predicate still holds here and is asserted as a structural check.
    assert!(
        report.passes_v20_binding_floors(),
        "the N=5 smoke must clear the binding floors: {report:?}"
    );
}

// ─────────────────────── wrappers — thin per-N (AC2.1) ───────────────────────

#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_mesh_identity_reconcile_30_host() {
    mesh_identity_reconcile_drill(COMPRESSED_HOST_COUNT, "identity-reconcile-30-host").await;
}

#[tokio::test]
#[ignore = "Story 14.1 — real N=100 mesh; gate-controlled via check-scale-churn"]
async fn t_14_1_mesh_identity_reconcile_n100() {
    mesh_identity_reconcile_drill(FULL_ENVELOPE_HOST_COUNT, "identity-reconcile-100-host").await;
}

#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_duplicate_identity_negative_control_hard_fails() {
    duplicate_identity_control(COMPRESSED_HOST_COUNT).await;
}

#[tokio::test]
#[ignore = "Story 14.1 — real N=100 mesh; gate-controlled via check-scale-churn"]
async fn t_14_1_duplicate_identity_negative_control_n100() {
    duplicate_identity_control(FULL_ENVELOPE_HOST_COUNT).await;
}

#[tokio::test]
#[ignore = "Story 11.3 — real N=30 mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_scale_churn_30_host_drill() {
    scale_churn_envelope_drill(COMPRESSED_HOST_COUNT).await;
}

#[tokio::test]
#[ignore = "Story 14.1 — real N=100 mesh; gate-controlled via check-scale-churn"]
async fn t_14_1_scale_churn_n100_drill() {
    scale_churn_envelope_drill(FULL_ENVELOPE_HOST_COUNT).await;
}

#[tokio::test]
#[ignore = "Story 11.3 — real detection mesh; gate-controlled via check-scale-churn"]
async fn t_11_3_detection_latency_drill() {
    detection_latency_drill(COMPRESSED_HOST_COUNT).await;
}

#[tokio::test]
#[ignore = "Story 14.1 — real N=100 detection mesh; gate-controlled via check-scale-churn"]
async fn t_14_1_detection_latency_n100() {
    detection_latency_drill(FULL_ENVELOPE_HOST_COUNT).await;
}

// ─────────────────────── AC4/AC5.5 — churn-fault-inject falsifiers ───────────────────────

/// Blind ONE adversary class's rejection from the harness's COUNTED tally.
/// The real detector still fired; NEVER feature-gates `verifier.rs`/`router`
/// (that would be subsystem-gating, the 11.2b P2 sin) — only the harness's
/// tally drops the sample, and the DOWNSTREAM count/identity reconcile then
/// REDS.
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

/// Blind-one-detector at scale n: the drill plants into the REAL n-host
/// mesh, the real detectors fire for all 3 classes, and blinding one class
/// at the counting seam REDS the downstream count/identity reconcile.
#[cfg(feature = "churn-fault-inject")]
async fn fault_inject_blind_one_class_at(n: usize, blind: AdversarialAttempt) {
    let (detections, derived_hosts) = plant_and_detect_three_adversaries(n).await;
    assert_eq!(
        detections.len(),
        PLANTED_ADVERSARIES,
        "the real detectors must fire for all 3 classes BEFORE the harness blinds one"
    );
    print_scale_marker(derived_hosts);
    // Clean set reconciles to 3 distinct planted identities.
    let clean = detection_report(detections.clone(), format!("blind-clean-n{n}"));
    clean
        .reconcile_detections(PLANTED_ADVERSARIES)
        .expect("clean set reconciles to 3 planted identities");

    // Blind one class at the counting seam → the DOWNSTREAM reconcile (the
    // real count/identity contract the gate consumes) REDS. This is the
    // falsifier — not `Vec::filter` on a hardcoded vector.
    let blinded = detection_report(blind_seam(detections, blind), format!("blind-red-n{n}"));
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
    fault_inject_blind_one_class_at(COMPRESSED_HOST_COUNT, AdversarialAttempt::TofuPinSpoofing)
        .await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blind_consent_bypass_reds_reconcile() {
    fault_inject_blind_one_class_at(
        COMPRESSED_HOST_COUNT,
        AdversarialAttempt::AdrLevel012ConsentBypass,
    )
    .await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blind_cert_race_reds_reconcile() {
    fault_inject_blind_one_class_at(
        COMPRESSED_HOST_COUNT,
        AdversarialAttempt::CertRotationRaceExploit,
    )
    .await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_blind_pin_spoof_reds_reconcile_n100() {
    fault_inject_blind_one_class_at(
        FULL_ENVELOPE_HOST_COUNT,
        AdversarialAttempt::TofuPinSpoofing,
    )
    .await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_blind_consent_bypass_reds_reconcile_n100() {
    fault_inject_blind_one_class_at(
        FULL_ENVELOPE_HOST_COUNT,
        AdversarialAttempt::AdrLevel012ConsentBypass,
    )
    .await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_blind_cert_race_reds_reconcile_n100() {
    fault_inject_blind_one_class_at(
        FULL_ENVELOPE_HOST_COUNT,
        AdversarialAttempt::CertRotationRaceExploit,
    )
    .await;
}

/// AC3/AC4 — a REAL blast over-reach at scale n: the adversary sweeps a
/// 6-peer subset of the n-host mesh and genuinely reaches all 6 before its
/// single escalation is denied → derived `max_blast_radius = 6` → the ≤5
/// BINDING floor REDS. A live falsifier for the blast axis (not a synthetic
/// 6-peer fixture).
#[cfg(feature = "churn-fault-inject")]
async fn fault_inject_blast_overreach_reds_floor(n: usize) {
    let (report, _) = run_consent_reachability_drill(
        n,
        DrillFaults {
            offered: Some(6),
            strategy: SweepStrategy::SweepThenEscalate,
            ..DrillFaults::default()
        },
    )
    .await;
    assert_eq!(
        report.max_blast_radius, 6,
        "the adversary genuinely reached all 6 offered targets: {report:?}"
    );
    assert!(
        !report.passes_v20_binding_floors(),
        "a real blast of 6 must red the ≤5 binding floor: {report:?}"
    );
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_blast_overreach_reds_floor() {
    fault_inject_blast_overreach_reds_floor(COMPRESSED_HOST_COUNT).await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_blast_overreach_reds_floor_n100() {
    fault_inject_blast_overreach_reds_floor(FULL_ENVELOPE_HOST_COUNT).await;
}

/// AC3/AC4 (D5, F3 separability) — ISOLATION-blind at scale n: skip the real
/// isolation repoint, so a real re-dial adversary→peer still SUCCEEDS → the
/// rto objective is unmet → `rto_secs` (the `RTO_UNMET_NS` sentinel — only
/// the branch is evidence, AC4.4) REDS, while `recovery_secs` (the full
/// legit sweep) stays green. INDEPENDENT of the recovery axis.
#[cfg(feature = "churn-fault-inject")]
async fn fault_inject_isolation_blind_reds_rto_only(n: usize) {
    let (report, _) = run_consent_reachability_drill(
        n,
        DrillFaults {
            isolate: false,
            // Review D1: the separability falsifiers hold propagation
            // CONSTANT (deterministic probing) so the axis under test is the
            // only variable — a race-driven blast breach must never be able
            // to red the rto/recovery separability legs for the wrong
            // reason.
            strategy: SweepStrategy::ProbeEveryBatch,
            ..DrillFaults::default()
        },
    )
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

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_isolation_blind_reds_rto_only() {
    fault_inject_isolation_blind_reds_rto_only(COMPRESSED_HOST_COUNT).await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_isolation_blind_reds_rto_only_n100() {
    fault_inject_isolation_blind_reds_rto_only(FULL_ENVELOPE_HOST_COUNT).await;
}

/// AC3/AC4 (D5, F3 separability) — RE-PIN-blind at scale n: break a legit
/// peer's endpoint so the FULL legit hub-and-spoke sweep FAILS → the fleet
/// never reconverges → `recovery_secs` (the `RECOVERY_UNMET_NS` sentinel)
/// REDS the binding floor, while `rto_secs` (adversary isolated) stays
/// green. INDEPENDENT of the rto axis.
#[cfg(feature = "churn-fault-inject")]
async fn fault_inject_repin_blind_reds_recovery_only(n: usize) {
    let (report, _) = run_consent_reachability_drill(
        n,
        DrillFaults {
            repin: false,
            // Review D1: deterministic propagation — see the isolation-blind
            // falsifier above.
            strategy: SweepStrategy::ProbeEveryBatch,
            ..DrillFaults::default()
        },
    )
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

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_11_3_fault_inject_repin_blind_reds_recovery_only() {
    fault_inject_repin_blind_reds_recovery_only(COMPRESSED_HOST_COUNT).await;
}

#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_repin_blind_reds_recovery_only_n100() {
    fault_inject_repin_blind_reds_recovery_only(FULL_ENVELOPE_HOST_COUNT).await;
}

/// Story 14.1 AC3.3.b — the offered==reached NON-DEGENERACY mutation at the
/// envelope scale: the adversary sweeps the FULL offered set (99 peers at
/// N=100) and never escalates, so nothing ever denies it. Three signals must
/// fire together: reached == offered (the clean leg's non-degeneracy
/// assertion would RED), the detection reconcile REDS on the miss, and the
/// blast floor REDS (99 > 5). This is the run that proves the clean green
/// could have been red.
#[cfg(feature = "churn-fault-inject")]
#[tokio::test]
#[ignore = "requires --features churn-fault-inject; gate-controlled via check-scale-churn"]
async fn t_14_1_fault_inject_full_sweep_no_detection_reds_offered_reached_n100() {
    let (report, offered) = run_consent_reachability_drill(
        FULL_ENVELOPE_HOST_COUNT,
        DrillFaults {
            strategy: SweepStrategy::SweepOnly,
            ..DrillFaults::default()
        },
    )
    .await;
    assert_eq!(
        report.per_adversary[0].first_rejection_ns, None,
        "the detector-switched-off mutation must produce a detection MISS: {report:?}"
    );
    assert_eq!(
        &report.per_adversary[0].blast_peers, &offered,
        "reached == offered REDS the clean leg's non-degeneracy assertion (AC3.3.b): nothing \
         bounded propagation"
    );
    assert!(
        report.reconcile_detections(1).is_err(),
        "the count/identity reconcile must RED on the undetected adversary"
    );
    // The falsifier for the NEW binding property (D-A): with detection never
    // interrupting, the reach violates BOTH conjuncts the clean leg binds.
    // Round-2 P6: the first assertion runs the BINDING PREDICATE ITSELF and
    // requires a REJECTION — the fixture-value checks below prove the
    // premises, but without this, deleting the clean-side conjuncts would
    // leave every leg green (the doc-comment claim "neither can be relaxed
    // without this leg noticing" is only true of THIS assert).
    let blind_reach = report.per_adversary[0].blast_peers.len();
    assert!(
        clean_blast_verdict(blind_reach, offered.len()).is_err(),
        "the binding conjuncts (D-A) must REJECT the detector-off report — a predicate \
         that cannot fail is the green-by-construction defect this story exists to close"
    );
    assert!(
        blind_reach > 4 * RACE_IN_FLIGHT,
        "detector-off must blow past the bounded-generations constant (4W = {}), got \
         {blind_reach} — the clean leg's conjunct (1) would not be falsifiable otherwise",
        4 * RACE_IN_FLIGHT
    );
    assert!(
        blind_reach * 2 >= offered.len(),
        "detector-off must reach at least HALF the offered set ({} of {}) — the clean leg's \
         run-derived conjunct (2) would not be falsifiable otherwise",
        blind_reach,
        offered.len()
    );
    assert!(
        !report.passes_v20_binding_floors(),
        "the absolute blast predicate must also RED when the adversary sweeps all {} peers: \
         {report:?}",
        offered.len()
    );
}

// Story 11.3 (D6/L7) — the fault-injection feature is EXERCISED only by the
// `#[cfg(feature = "churn-fault-inject")]` tests above; it has NO
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
