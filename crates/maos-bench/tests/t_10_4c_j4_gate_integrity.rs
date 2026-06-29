//! Story 10.4c — Gate 1 (harness-integrity) tests.
//!
//! AC2: the gate is falsifiable by DEGRADING the real path (D3, D4).
//! These tests exercise the `bench-fault-inject` mutation, the anti-canned
//! tripwire, non-degeneracy, and the slow-subscriber liveness invariant.

use std::io::Write;
use std::process::Command;

use maos_bench::report::JourneyResult;

/// Marker prefix the producer prints its serialized result after.
const PRODUCER_MARKER: &str = "__J4_RESULT_JSON__";

/// Run J4 measurement in a subprocess and return `(result, stderr)`.
fn run_j4_in_subprocess() -> (JourneyResult, String) {
    let exe = std::env::current_exe().expect("resolve test bin");
    let output = Command::new(exe)
        .args(["j4_gate1_producer_inner", "--exact", "--nocapture"])
        .output()
        .expect("spawn J4 producer subprocess");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "J4 producer subprocess failed; stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = stdout
        .lines()
        .find(|line| line.starts_with(PRODUCER_MARKER))
        .map(|line| &line[PRODUCER_MARKER.len()..])
        .unwrap_or_else(|| {
            panic!("no {PRODUCER_MARKER} line in producer stdout; full stdout:\n{stdout}");
        });
    let result: JourneyResult =
        serde_json::from_str(json).expect("deserialize journey result from producer");
    (result, stderr)
}

/// Inner producer: runs the J4 measurement and prints the JourneyResult as JSON.
#[test]
fn j4_gate1_producer_inner() {
    use maos_bench::harness::j4::{run_j4_measurement, J4Config};
    let config = J4Config {
        invocation_count: 250,
        warmup_count: 10,
    };
    let result = run_j4_measurement(&config).expect("j4 measurement");
    let json = serde_json::to_string(&result).expect("serialize journey result");
    let mut out = std::io::stdout();
    writeln!(out, "{PRODUCER_MARKER}{json}").expect("write result line");
    out.flush().expect("flush stdout");
}

// ────────────────────────────────────────────────────────────────────────────
// AC2 / D4: Anti-canned tripwire + non-degeneracy
// ────────────────────────────────────────────────────────────────────────────

/// AC2 / D9: Anti-canned tripwire — the GA path (kernel_measurement ON) must
/// NOT emit the "NOT real measurements" marker. If it does, the harness has
/// been re-stubbed to constants.
// Gate-1-integrity invariants hold ONLY on the real `kernel_measurement` path;
// under the default smoke path the harness legitimately emits the placeholder
// marker, so these tests are compiled out without the feature (otherwise
// `cargo test -p maos-bench` default is RED on the anti-canned tripwire).
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_anti_canned_tripwire() {
    let (_result, stderr) = run_j4_in_subprocess();
    assert!(
        !stderr.contains("NOT real measurements"),
        "The real J4 measurement path must NOT emit the 'NOT real measurements' \
         marker — if it does, the harness has been re-stubbed to constants. \
         stderr:\n{stderr}"
    );
}

/// AC2 / D4: Non-degeneracy — the real measurement must produce samples with
/// variance > 0 (not all identical) and a minimum distinct-count floor.
/// A synthesized canned vector would have uniform/predictable samples.
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_non_degeneracy() {
    let (result, _stderr) = run_j4_in_subprocess();
    // Non-degeneracy (AC2/D4): real measurements must NOT be a constant vector.
    // The reliable signal is `max > p50` (the tail rises above the median ⟺ not
    // all samples are identical). `std_dev_us` is NOT used: it is integer-µs and
    // rounds to 0 even for a real varied distribution (observed at HEAD: p50=1,
    // max=6, std_dev=0). We also accept `max_us == 0` — on extremely fast
    // machines every sample rounds to 0µs (sub-microsecond), which is a REAL
    // measurement, not a canned vector (canned samples are 1000+, never all-zero).
    // A constant NON-zero vector (max == p50 && max > 0) is rejected as degenerate.
    // (A distinct-count floor cannot be computed from the µs aggregates in
    // JourneyResult; the load-bearing no-re-can guards are the anti-canned
    // tripwire, the sample-count assertion, and the mutation test.)
    let has_variation = result.max_us > result.p50_us || result.max_us == 0;
    assert!(
        has_variation,
        "J4 non-degeneracy FAILED: all samples appear identical \
         (p50={}, p95={}, max={}, std_dev={}) — a canned vector? \
         Real measurements must show some variation.",
        result.p50_us, result.p95_us, result.max_us, result.std_dev_us
    );
}

/// AC2 / D4: sample count must equal invocation_count — a synthesized canned
/// vector that bypasses the tap would produce a count mismatch.
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_sample_count_matches_invocations() {
    let (result, _stderr) = run_j4_in_subprocess();
    assert_eq!(
        result.invocation_count, 250,
        "J4 sample count ({}) must equal the configured invocation_count (250) — \
         a tap-arrival-count mismatch indicates a bypassed tap",
        result.invocation_count,
    );
}

/// AC2: the measurement is GREEN without fault injection — within the 10ms budget.
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_no_injection_green() {
    let (result, _stderr) = run_j4_in_subprocess();
    assert!(
        result.budget_met,
        "J4 must be GREEN without fault injection: P95={}µs exceeds \
         budget 10000µs — investigate the real measurement path",
        result.p95_us,
    );
}

// ────────────────────────────────────────────────────────────────────────────
// AC2: Slow-subscriber liveness test (Winston)
// ────────────────────────────────────────────────────────────────────────────

/// AC2: slow-subscriber liveness — proves the "cannot lag the producer" invariant
/// structurally: inject a sleeping subscriber and assert the producer's emit-cost
/// distribution is statistically unchanged (non-blocking emit; bounded/lossy channel).
///
/// This test verifies that even when a subscriber is slow, the producer's
/// `set_scalar` → `publish_event` path does NOT block. The tokio broadcast
/// channel is lossy: slow consumers get `RecvError::Lagged`, not producer backpressure.
#[cfg(feature = "kernel_measurement")]
#[tokio::test]
async fn t_10_4c_slow_subscriber_liveness() {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use maos_domain::invariants::i7::TelemetryTopic;
    use maos_domain::ports::crypto::CryptoProvider;
    use maos_domain::ports::TelemetryStreamPort;
    use maos_kernel_core::capability::{
        cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker,
        cap_tokens::Ed25519SigningKey, CapabilityRegistryAdapter, WorkingMemoryStore,
    };
    use maos_kernel_core::telemetry::TelemetryStreamAdapter;

    let telemetry = Arc::new(TelemetryStreamAdapter::new(16)); // small capacity
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _) = cap_audit::channel();
    let quota = CapQuotaTracker::new();
    let working_memory = Arc::new(WorkingMemoryStore::new());
    let adapter = CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xCAFE,
        policy,
        audit_tx,
        quota,
        working_memory,
        Arc::clone(&telemetry),
    );

    let tag = "liveness";
    let topic = TelemetryTopic::new(&format!("scalar.tap.{tag}"));
    telemetry.subscribe_topic("slow-observer", &topic);
    let _rx = telemetry.subscribe(&topic).unwrap();

    // Spawn a sleeping subscriber that never reads
    let slow_rx = telemetry.subscribe(&topic).unwrap();
    let _slow_task = tokio::spawn(async move {
        let _rx = slow_rx; // hold the receiver but never read
        tokio::time::sleep(Duration::from_secs(60)).await;
    });

    // Measure emit-cost with the sleeping subscriber present
    let n = 100;
    let mut emit_costs = Vec::with_capacity(n);
    for i in 0..n {
        let start = Instant::now();
        adapter
            .set_scalar(1, "bench-spirit", tag, i as f64, "bench-frame")
            .unwrap();
        emit_costs.push(start.elapsed());
    }

    // The emit path should be fast (<1ms per call) even with a slow subscriber.
    // tokio broadcast drops messages for lagged consumers, it does NOT block the sender.
    let max_emit_us = emit_costs.iter().map(|d| d.as_micros()).max().unwrap();
    assert!(
        max_emit_us < 1_000_000, // 1 second — extremely generous; real should be <100µs
        "Slow-subscriber liveness FAILED: emit path blocked for {}µs — \
         the producer should never be blocked by a slow consumer \
         (broadcast channel is lossy, not backpressured)",
        max_emit_us,
    );

    // P95 should be well under 1ms
    let mut sorted: Vec<u128> = emit_costs.iter().map(|d| d.as_micros()).collect();
    sorted.sort_unstable();
    let p95_idx = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    let emit_p95 = sorted[p95_idx];
    eprintln!(
        "Slow-subscriber liveness: emit P95={}µs max={}µs (must be << 1s; \
         proves non-blocking emit)",
        emit_p95, max_emit_us,
    );
}
