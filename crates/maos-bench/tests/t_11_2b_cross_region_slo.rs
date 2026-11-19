#![forbid(unsafe_code)]

//! Story 11.2b — cross-region round-trip SLO (AC2 / D2 / F5).
//!
//! Measures cross-region propagation latency as a **single-clock A→B→A
//! round-trip** over the 11.2a cross-region machinery, fed into the J4
//! percentile/histogram engine (`build_journey_result`). NEVER `recv−emit`
//! across two machines (a foreign-clock subtraction — a proven-rejected
//! category error that can go negative). The metric is named
//! "cross-region round-trip (network + remote-service)" — it includes region
//! B's processing, so "network RTT" / "RTT SLO" would be a dishonest name.
//!
//! The probe uses `maos-loom-lite` as a dev-dep (F7 surface-shrink: the
//! `slo-fault-inject` feature is HERE in maos-bench, keeping loom-lite's
//! manifest free of a leak-risk toggle).
//!
//! ## Run
//!
//! Live (clean GREEN path):
//! ```text
//! MAOS_TEST_POSTGRES_A="..." MAOS_TEST_POSTGRES_B="..." \
//!   cargo test -p maos-bench --test t_11_2b_cross_region_slo \
//!     cross_region_roundtrip_live -- --ignored --nocapture
//! ```
//!
//! Mutation (positive control — fails if the injected delay is not measured):
//! ```text
//! MAOS_TEST_POSTGRES_A="..." MAOS_TEST_POSTGRES_B="..." \
//!   cargo test -p maos-bench --features slo-fault-inject \
//!     --test t_11_2b_cross_region_slo cross_region_roundtrip_mutation \
//!     -- --ignored --nocapture
//! ```

use std::sync::Mutex;
use std::time::Instant;

use maos_bench::harness::build_journey_result;
use maos_bench::harness::cross_region::{slo_inject_delay, MULTI_REGION_SLO_P95_US};
use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::region::Region;
use maos_loom_lite::replication::bundle::{apply_replication_bundle, build_replication_bundle};
use maos_loom_lite::replication::leaf::CollectiveKvLeaf;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};
use tokio_postgres::NoTls;

// Story 13.6e (AC3) — the harness signs its own transcript record; the gate
// only verifies. Shared signer, no new crate and no new dependency.
#[path = "../../../tests/harness/evidence_record.rs"]
mod evidence_record;

/// Serialize the live tests on the shared region DBs.
static REGION_LOCK: Mutex<()> = Mutex::new(());

const BASE_SEED: [u8; 32] = [0x42u8; 32];

fn pg_conn_for(tag: char) -> String {
    let key = match tag {
        'a' => "MAOS_TEST_POSTGRES_A",
        'b' => "MAOS_TEST_POSTGRES_B",
        other => panic!("region tag must be a|b, got {other:?}"),
    };
    std::env::var(key).unwrap_or_else(|_| {
        panic!(
            "{key} must be set for the cross-region round-trip SLO tests (F2 — \
             two real Postgres DBs)"
        )
    })
}

async fn make_store(tag: char, region: &str) -> LoomLiteStore {
    let store = LoomLiteStore::new(StoreConfig {
        connection_string: pg_conn_for(tag),
        home_region: region.to_string(),
        ..StoreConfig::default()
    })
    .await
    .expect("store creation must succeed");
    store.init_schema().await.expect("schema init must succeed");
    store
}

/// Open a persistent raw `tokio_postgres::Client` (reused across all probe
/// samples). A real mediator does not open a fresh TCP connection per probe;
/// connection-setup cost would otherwise dominate the measurement and mask the
/// machinery latency the loopback floor is meant to regression-guard.
async fn persistent_client(tag: char) -> tokio_postgres::Client {
    let conn_str = pg_conn_for(tag);
    let (client, conn) = tokio_postgres::connect(&conn_str, NoTls)
        .await
        .expect("raw connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
}

async fn reset(tag: char) {
    let client = persistent_client(tag).await;
    client
        .execute("DELETE FROM collective_memory", &[])
        .await
        .expect("DELETE must succeed");
}

async fn read_all_leaves(client: &tokio_postgres::Client) -> Vec<CollectiveKvLeaf> {
    let rows = LoomLiteStore::read_all_rows_from(client)
        .await
        .expect("read_all_rows_from");
    rows.iter().map(CollectiveKvLeaf::from_row).collect()
}

/// The measured span's stages, in order. Printed as per-stage p50 by the live
/// test (21-2 ruling, 2026-09-24): the 2026-09-21 floor breach needed a
/// throwaway timer to attribute; a breach must now attribute itself.
const STAGES: [&str; 5] = [
    "write_a",
    "read_build_a",
    "apply_ab",
    "read_build_b",
    "apply_ba",
];

/// One round-trip: the single-clock total plus its per-stage split. The stage
/// stamps are `Instant::now()` reads between stages — the total is still
/// `t0 → end` on the one region-A clock, unchanged by them.
struct RoundTrip {
    total_us: u64,
    #[cfg(feature = "slo-fault-inject")]
    before_injection_us: u64,
    stages_us: [u64; STAGES.len()],
}

/// Run one single-clock A→B→A round-trip. The region identities are prepared
/// outside the measured span so the sample covers only the production path.
async fn roundtrip_sample(
    client_a: &tokio_postgres::Client,
    client_b: &tokio_postgres::Client,
    store_a: &LoomLiteStore,
    store_b: &LoomLiteStore,
    region_a: &Region,
    region_b: &Region,
    sequence: usize,
    inject: bool,
) -> RoundTrip {
    let ts = 1_700_000_000_000_000_000i64 + sequence as i64;
    // t0 on region A's monotonic clock — the SOLE clock for this round-trip.
    let t0 = Instant::now();
    // Write the probe row to A (update — same key, monotonically higher ts).
    store_a
        .write_with_source(
            1,
            &MemoryNamespace::Default,
            "rtt-probe",
            MemoryValue::Text(format!("v{sequence}")),
            maos_loom_lite::store::WriteSource {
                ts,
                region: "region-a",
                log_ref: "",
                team: None,
                distillation_depth: None,
                intent_lineage: None,
            },
        )
        .await
        .expect("write probe to A");
    let written = Instant::now();
    // build@A → apply(dest=B)
    //
    // FIDELITY (2026-08-04): do not re-add an explicit
    // `verify_replication_bundle` here. Story 13.2 moved verification inside
    // `apply_replication_bundle`; production calls `apply` directly, and a
    // verification failure makes the `.expect` below panic.
    let leaves_a = read_all_leaves(client_a).await;
    let bundle_ab = build_replication_bundle(leaves_a, region_a, &BASE_SEED);
    let built_a = Instant::now();
    apply_replication_bundle(&bundle_ab, store_b, "region-b", None, &BASE_SEED)
        .await
        .expect("apply A->B (verifies internally)");
    let applied_ab = Instant::now();
    // build@B → apply(dest=A) — the return leg.
    let leaves_b = read_all_leaves(client_b).await;
    let bundle_ba = build_replication_bundle(leaves_b, region_b, &BASE_SEED);
    let built_b = Instant::now();
    apply_replication_bundle(&bundle_ba, store_a, "region-a", None, &BASE_SEED)
        .await
        .expect("apply B->A (verifies internally)");
    let applied_ba = Instant::now();
    // F7 fault-inject: inject INSIDE the measured span (no-op without the
    // slo-fault-inject feature).
    if inject {
        slo_inject_delay();
    }
    // End on the SAME region-A clock. rtt_us = end − t0 (single-clock — valid).
    let end = Instant::now();
    let us = |from: Instant, to: Instant| to.duration_since(from).as_micros() as u64;
    RoundTrip {
        total_us: us(t0, end),
        #[cfg(feature = "slo-fault-inject")]
        before_injection_us: us(t0, applied_ba),
        stages_us: [
            us(t0, written),
            us(written, built_a),
            us(built_a, applied_ab),
            us(applied_ab, built_b),
            us(built_b, applied_ba),
        ],
    }
}

/// Run `n` round-trips. `inject: false` supplies a same-build clean arm for the
/// mutation test; without the feature the injection call is a no-op.
async fn roundtrip_samples(
    client_a: &tokio_postgres::Client,
    client_b: &tokio_postgres::Client,
    store_a: &LoomLiteStore,
    store_b: &LoomLiteStore,
    n: usize,
    inject: bool,
) -> Vec<RoundTrip> {
    let region_a = Region::canonicalize("region-a").unwrap();
    let region_b = Region::canonicalize("region-b").unwrap();
    let mut samples = Vec::with_capacity(n);
    for sequence in 0..n {
        samples.push(
            roundtrip_sample(
                client_a, client_b, store_a, store_b, &region_a, &region_b, sequence, inject,
            )
            .await,
        );
    }
    samples
}

/// AC2 / D2 (F5/F6): Live cross-region round-trip SLO — single-clock A→B→A
/// round-trip measured against the loopback regression floor
/// [`MULTI_REGION_SLO_P95_US`]. The floor is a regression tripwire, NOT a
/// geo-SLO (F6/L7 — CI Postgres is co-located). Anti-hardcoded-count tooth:
/// `samples.len() == count == N`; non-degenerate-distribution check (a constant
/// vector → `p95 == p50` → RED).
#[tokio::test]
#[ignore = "requires two live Postgres (set MAOS_TEST_POSTGRES_{A,B})"]
async fn cross_region_roundtrip_live() {
    let _evidence = evidence_record::attest("cross_region_roundtrip_live");
    let _g = REGION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store_a = make_store('a', "region-a").await;
    let store_b = make_store('b', "region-b").await;
    let client_a = persistent_client('a').await;
    let client_b = persistent_client('b').await;
    reset('a').await;
    reset('b').await;

    // Warmup (discarded) — mirror the j4.rs warmup-discard idiom.
    let _ = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, 10, true).await;

    // N≥200 post-warmup sample floor (mirror j4.rs:238-245).
    const N: usize = 200;
    let trips = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, N, true).await;
    let samples: Vec<u64> = trips.iter().map(|trip| trip.total_us).collect();

    let result = build_journey_result(
        "J-crossregion-rt",
        N as u64,
        &samples,
        MULTI_REGION_SLO_P95_US,
    );

    // Diagnostics BEFORE the asserts (2026-08-04): every assert below panics, so
    // emitting the distribution afterwards meant a RED run reported p95 and
    // nothing else — the shape of the failure was unmeasurable by construction.
    // A floor breach is a FINDING, and a finding you cannot characterise is a
    // guess. `--nocapture` (which both gate legs already pass) surfaces this.
    let live_line = format!(
        "cross-region round-trip LIVE: p50={}µs p95={}µs p99={}µs max={}µs \
         mean={}µs std_dev={}µs (budget={}µs, met={}, n={})",
        result.p50_us,
        result.p95_us,
        result.p99_us,
        result.max_us,
        result.mean_us,
        result.std_dev_us,
        MULTI_REGION_SLO_P95_US,
        result.budget_met,
        N
    );
    // Per-stage p50 / p95 / max (2026-09-24). p50 alone attributed a FLAT
    // breach (the 2026-09-21 fsync cost) but not a TAIL one: CI 67bf70fa had
    // p50 6120µs and p95 41300µs, max 341211µs, and a p50-only stage line
    // could not say where those samples went.
    let stage_line = format!(
        "cross-region round-trip STAGES p50/p95/max (µs): {}",
        STAGES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mut stage: Vec<u64> = trips.iter().map(|trip| trip.stages_us[i]).collect();
                stage.sort_unstable();
                format!(
                    "{name}={}/{}/{}",
                    stage[stage.len() / 2],
                    stage[stage.len() * 95 / 100],
                    stage[stage.len() - 1]
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    );
    // For every sample over the floor: where did most of its time go? `other`
    // is the span NOT covered by the five stage stamps (total − their sum) —
    // the fault-inject delay, or a stall between stages — so a stall there is
    // never blamed on the largest measured stage.
    let mut dominant = [0usize; STAGES.len() + 1];
    let over_floor = trips
        .iter()
        .filter(|trip| trip.total_us > MULTI_REGION_SLO_P95_US)
        .inspect(|trip| {
            let other = trip
                .total_us
                .saturating_sub(trip.stages_us.iter().sum::<u64>());
            let (worst, _) = trip
                .stages_us
                .iter()
                .copied()
                .chain([other])
                .enumerate()
                .max_by_key(|(_, us)| *us)
                .expect("six buckets");
            dominant[worst] += 1;
        })
        .count();
    let tail_line = format!(
        "cross-region round-trip TAIL: {over_floor}/{N} samples over {}µs; dominant stage: {}",
        MULTI_REGION_SLO_P95_US,
        STAGES
            .iter()
            .copied()
            .chain(["other"])
            .zip(dominant)
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| format!("{name}×{count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    for line in [&live_line, &stage_line, &tail_line] {
        eprintln!("{line}");
    }
    // Also publish to the CI step summary, so a GREEN run leaves its
    // distribution too: the gate prints a leg's transcript only when the leg
    // fails, so six consecutive greens left no numbers to measure variance by.
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(summary) {
            let _ = writeln!(file, "```\n{live_line}\n{stage_line}\n{tail_line}\n```");
        }
    }

    // Anti-hardcoded-count tooth: samples.len() == count == N.
    assert_eq!(
        samples.len(),
        N,
        "sample count ({}) must equal the invocation count ({N})",
        samples.len()
    );
    assert_eq!(
        result.invocation_count, N as u64,
        "JourneyResult.invocation_count must equal N"
    );

    // Non-degenerate-distribution check (a constant vector → p95==p50 → RED).
    assert!(
        result.p95_us > result.p50_us || result.std_dev_us > 0,
        "non-degenerate distribution required: p95={}µs p50={}µs std_dev={}µs — \
         a constant vector would make p95==p50 (the 11.2a vacuous re-stub trap)",
        result.p95_us,
        result.p50_us,
        result.std_dev_us
    );

    // Budget met (loopback floor, NOT a geo-SLO — F6/L7).
    assert!(
        result.budget_met,
        "cross-region round-trip p95={}µs must be <= the loopback regression \
         floor MULTI_REGION_SLO_P95_US={}µs (NOT a geo-SLO — CI Postgres is \
         co-located). If this regressed, the machinery/convergence slowed.",
        result.p95_us, MULTI_REGION_SLO_P95_US
    );
}

/// AC2 / F7 (Arm-1, latency): verify the real 15 ms injection is included in
/// the total passed to the live SLO histogram. Compare two readings from the
/// same trip, not two database transactions. Database and scheduler latency
/// before the injection is common to both readings and cancels exactly.
/// `thread::sleep` may oversleep, so only the requested minimum matters.
/// The clean live test remains the sole check of the 30 ms p95 budget.
#[cfg(feature = "slo-fault-inject")]
#[tokio::test]
#[ignore = "requires two live Postgres + --features slo-fault-inject"]
async fn cross_region_roundtrip_mutation() {
    let _evidence = evidence_record::attest("cross_region_roundtrip_mutation");
    let _g = REGION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store_a = make_store('a', "region-a").await;
    let store_b = make_store('b', "region-b").await;
    let client_a = persistent_client('a').await;
    let client_b = persistent_client('b').await;
    reset('a').await;
    reset('b').await;

    use maos_bench::harness::cross_region::SLO_FAULT_INJECT_DELAY_US;

    const N: usize = 3;
    let trips = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, N, true).await;
    for (sample, trip) in trips.into_iter().enumerate() {
        let included_us = trip.total_us.saturating_sub(trip.before_injection_us);
        eprintln!(
            "cross-region round-trip MUTATION {sample}: pre-injection={}µs total={}µs included={}µs minimum={}µs",
            trip.before_injection_us, trip.total_us, included_us, SLO_FAULT_INJECT_DELAY_US
        );
        assert!(
            included_us >= SLO_FAULT_INJECT_DELAY_US,
            "round-trip sample {sample} omitted the in-span delay: \
             before={}µs, reported total={}µs, need at least {}µs more",
            trip.before_injection_us,
            trip.total_us,
            SLO_FAULT_INJECT_DELAY_US
        );
    }
}
