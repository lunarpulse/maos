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
) -> u64 {
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
    // build@A → apply(dest=B)
    //
    // FIDELITY (2026-08-04): do not re-add an explicit
    // `verify_replication_bundle` here. Story 13.2 moved verification inside
    // `apply_replication_bundle`; production calls `apply` directly, and a
    // verification failure makes the `.expect` below panic.
    let leaves_a = read_all_leaves(client_a).await;
    let bundle_ab = build_replication_bundle(leaves_a, region_a, &BASE_SEED);
    apply_replication_bundle(&bundle_ab, store_b, "region-b", None, &BASE_SEED)
        .await
        .expect("apply A->B (verifies internally)");
    // build@B → apply(dest=A) — the return leg.
    let leaves_b = read_all_leaves(client_b).await;
    let bundle_ba = build_replication_bundle(leaves_b, region_b, &BASE_SEED);
    apply_replication_bundle(&bundle_ba, store_a, "region-a", None, &BASE_SEED)
        .await
        .expect("apply B->A (verifies internally)");
    // F7 fault-inject: inject INSIDE the measured span (no-op without the
    // slo-fault-inject feature).
    if inject {
        slo_inject_delay();
    }
    // t1 on the SAME region-A clock. rtt_us = t1 − t0 (single-clock — valid).
    Instant::now().duration_since(t0).as_micros() as u64
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
) -> Vec<u64> {
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
    let samples = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, N, true).await;

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
    eprintln!(
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

/// AC2 / F7 (Arm-1, latency): The `slo-fault-inject` mutation. With the feature
/// active, a fixed 15ms is injected INSIDE the measured A→B→A span, so the p95
/// MUST cross the loopback floor. This is the "stranger's falsifier" (D4): if
/// anyone re-stubs the probe to constants, the injection cannot move the number
/// → this test goes green-when-it-must-be-red.
///
/// Story 13.6e (T5) repaired the old absolute assertion with paired samples.
/// Clean and injected measurements alternate order within each pair, then the
/// median per-pair delta must carry at least 14ms of the fixed 15ms injection.
/// This measures the injected contribution directly instead of subtracting two
/// sequential batch percentiles that can drift independently.
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

    // Warmup with injection active (discarded).
    let _ = roundtrip_samples(&client_a, &client_b, &store_a, &store_b, 5, true).await;

    const N: usize = 60;
    let region_a = Region::canonicalize("region-a").unwrap();
    let region_b = Region::canonicalize("region-b").unwrap();
    let mut clean_samples = Vec::with_capacity(N);
    let mut injected_samples = Vec::with_capacity(N);

    // Pair adjacent samples and alternate order. This prevents a systematic
    // second-batch speedup/slowdown from masquerading as the fixed injection.
    for pair in 0..N {
        let sequence = 10 + pair * 2;
        let (clean_us, injected_us) = if pair % 2 == 0 {
            let clean = roundtrip_sample(
                &client_a, &client_b, &store_a, &store_b, &region_a, &region_b, sequence, false,
            )
            .await;
            let injected = roundtrip_sample(
                &client_a,
                &client_b,
                &store_a,
                &store_b,
                &region_a,
                &region_b,
                sequence + 1,
                true,
            )
            .await;
            (clean, injected)
        } else {
            let injected = roundtrip_sample(
                &client_a, &client_b, &store_a, &store_b, &region_a, &region_b, sequence, true,
            )
            .await;
            let clean = roundtrip_sample(
                &client_a,
                &client_b,
                &store_a,
                &store_b,
                &region_a,
                &region_b,
                sequence + 1,
                false,
            )
            .await;
            (clean, injected)
        };
        clean_samples.push(clean_us);
        injected_samples.push(injected_us);
    }
    let paired_deltas = paired_injection_deltas(&clean_samples, &injected_samples);

    let clean = build_journey_result(
        "J-crossregion-rt-mutation-clean",
        N as u64,
        &clean_samples,
        MULTI_REGION_SLO_P95_US,
    );
    let result = build_journey_result(
        "J-crossregion-rt-mutation",
        N as u64,
        &injected_samples,
        MULTI_REGION_SLO_P95_US,
    );
    let delta = build_journey_result(
        "J-crossregion-rt-mutation-paired-delta",
        N as u64,
        &paired_deltas,
        u64::MAX,
    );

    eprintln!(
        "cross-region round-trip MUTATION: injected p95={}µs, clean p95={}µs, \
         paired delta p50={}µs p95={}µs (budget={}µs, injected met={}, n={})",
        result.p95_us,
        clean.p95_us,
        delta.p50_us,
        delta.p95_us,
        MULTI_REGION_SLO_P95_US,
        result.budget_met,
        N
    );

    assert!(
        !result.budget_met,
        "mutation: budget_met must be FALSE with slo-fault-inject — p95={}µs \
         did not cross the {}µs floor. If the injection cannot move the number, \
         the probe has been re-stubbed to constants (F7 ship-blocker).",
        result.p95_us, MULTI_REGION_SLO_P95_US
    );
    assert!(
        delta.p50_us >= 14_000,
        "mutation: paired injection delta p50={}µs is below 14 000µs \
         (p95={}µs). The {}µs injection is not landing inside the measured \
         span for a majority of adjacent pairs.",
        delta.p50_us,
        delta.p95_us,
        maos_bench::harness::cross_region::SLO_FAULT_INJECT_DELAY_US
    );
}

fn paired_injection_deltas(clean: &[u64], injected: &[u64]) -> Vec<u64> {
    assert_eq!(
        clean.len(),
        injected.len(),
        "paired clean/injected samples must have equal length"
    );
    clean
        .iter()
        .zip(injected)
        .map(|(clean, injected)| injected.saturating_sub(*clean))
        .collect()
}

#[test]
fn paired_delta_oracle_requires_the_injection_on_a_majority_of_pairs() {
    let clean = vec![20_000; 60];
    let absent = paired_injection_deltas(&clean, &vec![20_000; 60]);
    let injected = paired_injection_deltas(&clean, &vec![35_000; 60]);
    let absent_result = build_journey_result("absent", 60, &absent, u64::MAX);
    let injected_result = build_journey_result("injected", 60, &injected, u64::MAX);
    assert!(absent_result.p50_us < 14_000);
    assert!(injected_result.p50_us >= 14_000);
}
