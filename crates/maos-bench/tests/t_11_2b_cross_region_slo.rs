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
//! Mutation (fault-inject — must RED):
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
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, build_replication_bundle, verify_replication_bundle,
};
use maos_loom_lite::replication::leaf::CollectiveKvLeaf;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};
use tokio_postgres::NoTls;

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

/// Run `n` single-clock A→B→A round-trips, returning the per-sample microsecond
/// latencies. `build@A → apply(dest=B) → build@B → apply(dest=A)`, all wrapped
/// in ONE `Instant` on region A (F5). A constant-size probe row is updated each
/// iteration so the per-sample cost is stable (no row-count growth).
async fn roundtrip_samples(
    client_a: &tokio_postgres::Client,
    client_b: &tokio_postgres::Client,
    store_a: &LoomLiteStore,
    store_b: &LoomLiteStore,
    n: usize,
) -> Vec<u64> {
    let region_a = Region::canonicalize("region-a").unwrap();
    let region_b = Region::canonicalize("region-b").unwrap();
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let ts = 1_700_000_000_000_000_000i64 + i as i64;
        // t0 on region A's monotonic clock — the SOLE clock for this round-trip.
        let t0 = Instant::now();
        // Write the probe row to A (update — same key, monotonically higher ts).
        store_a
            .write_with_source(
                1,
                &MemoryNamespace::Default,
                "rtt-probe",
                MemoryValue::Text(format!("v{i}")),
                maos_loom_lite::store::WriteSource {
                    ts: ts,
                    region: "region-a",
                    log_ref: "",
                    team: None,
                },
            )
            .await
            .expect("write probe to A");
        // build@A → apply(dest=B)
        let leaves_a = read_all_leaves(client_a).await;
        let bundle_ab = build_replication_bundle(leaves_a, &region_a, &BASE_SEED);
        verify_replication_bundle(&bundle_ab, &BASE_SEED).expect("verify A->B bundle");
        apply_replication_bundle(&bundle_ab, store_b, "region-b", None, &BASE_SEED)
            .await
            .expect("apply A->B");
        // build@B → apply(dest=A) — the return leg.
        let leaves_b = read_all_leaves(client_b).await;
        let bundle_ba = build_replication_bundle(leaves_b, &region_b, &BASE_SEED);
        verify_replication_bundle(&bundle_ba, &BASE_SEED).expect("verify B->A bundle");
        apply_replication_bundle(&bundle_ba, store_a, "region-a", None, &BASE_SEED)
            .await
            .expect("apply B->A");
        // F7 fault-inject: inject INSIDE the measured span (no-op without the
        // slo-fault-inject feature — the clean GREEN path).
        slo_inject_delay();
        // t1 on the SAME region-A clock. rtt_us = t1 − t0 (single-clock — valid).
        let t1 = Instant::now();
        samples.push(t1.duration_since(t0).as_micros() as u64);
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
    let _g = REGION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store_a = make_store('a', "region-a").await;
    let store_b = make_store('b', "region-b").await;
    let client_a = persistent_client('a').await;
    let client_b = persistent_client('b').await;
    reset('a').await;
    reset('b').await;

    // Warmup (discarded) — mirror the j4.rs warmup-discard idiom.
    let _ = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, 10).await;

    // N≥200 post-warmup sample floor (mirror j4.rs:238-245).
    const N: usize = 200;
    let samples = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, N).await;

    let result = build_journey_result(
        "J-crossregion-rt",
        N as u64,
        &samples,
        MULTI_REGION_SLO_P95_US,
    );

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

    eprintln!(
        "cross-region round-trip LIVE: p50={}µs p95={}µs p99={}µs max={}µs \
         mean={}µs (budget={}µs, met={}, n={})",
        result.p50_us,
        result.p95_us,
        result.p99_us,
        result.max_us,
        result.mean_us,
        MULTI_REGION_SLO_P95_US,
        result.budget_met,
        N
    );
}

/// AC2 / F7 (Arm-1, latency): The `slo-fault-inject` mutation. With the feature
/// active, a fixed 15ms is injected INSIDE the measured A→B→A span, so the p95
/// MUST cross the loopback floor. This is the "stranger's falsifier" (D4): if
/// anyone re-stubs the probe to constants, the injection cannot move the number
/// → this test goes green-when-it-must-be-red.
#[cfg(feature = "slo-fault-inject")]
#[tokio::test]
#[ignore = "requires two live Postgres + --features slo-fault-inject"]
async fn cross_region_roundtrip_mutation() {
    let _g = REGION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store_a = make_store('a', "region-a").await;
    let store_b = make_store('b', "region-b").await;
    let client_a = persistent_client('a').await;
    let client_b = persistent_client('b').await;
    reset('a').await;
    reset('b').await;

    // Warmup with injection active (discarded).
    let _ = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, 5).await;

    const N: usize = 60;
    let samples = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, N).await;
    let result = build_journey_result(
        "J-crossregion-rt-mutation",
        N as u64,
        &samples,
        MULTI_REGION_SLO_P95_US,
    );

    // The injected ≥15ms delay must break the loopback budget.
    assert!(
        !result.budget_met,
        "mutation: budget_met must be FALSE with slo-fault-inject — p95={}µs \
         did not cross the {}µs floor. If the injection cannot move the number, \
         the probe has been re-stubbed to constants (F7 ship-blocker).",
        result.p95_us, MULTI_REGION_SLO_P95_US
    );
    assert!(
        result.p95_us >= 14_000,
        "mutation: p95={}µs is below 14_000µs — the 15ms injection should add \
         ≥14_000µs to each sample. If the number didn't move, the injection may \
         not be inside the measured span.",
        result.p95_us
    );

    eprintln!(
        "cross-region round-trip MUTATION: p95={}µs (>= 14_000µs — injection \
         moves the number through the gate's own comparator, budget broken)",
        result.p95_us
    );
}
