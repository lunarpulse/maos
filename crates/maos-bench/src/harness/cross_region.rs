#![forbid(unsafe_code)]

//! Story 11.2b — Cross-region round-trip SLO source (AC2 / D2 / F5).
//!
//! The J4 percentile/histogram engine ([`super::build_journey_result`]) is
//! journey-agnostic — it takes any `&[u64]` microsecond samples + a budget. The
//! cross-region SLO feeds it samples drawn from a **single-clock A→B→A
//! round-trip** over the 11.2a cross-region machinery (F5), NOT the in-process
//! `run_j4_kernel` tap (which assumes one process / one monotonic clock —
//! invalid across regions, L2). A foreign-clock `B.instant − A.instant`
//! subtraction is a proven-rejected category error (can go negative).
//!
//! # The budget is a LOOPBACK regression floor — NOT a geo-SLO (F6/L7)
//!
//! CI Postgres is co-located, so a real geo-RTT (San-Francisco↔Frankfurt) is
//! physically unobservable on a GitHub runner. [`MULTI_REGION_SLO_P95_US`] is a
//! **loopback-calibrated regression floor**: it binds the *machinery +
//! convergence + regression* (the round-trip did not regress), NOT a
//! geo-latency figure. Pinning a geo number as a *pass condition* is the 10.2
//! trap (loopback ~1ms passes trivially). The absolute geo-SLO is authored as
//! NFR text marked "validated in pilot" — a release-gate artifact, un-gated
//! here. The budget's teeth come from the [`slo_inject_delay`] falsifier (which
//! moves p95 through the gate's own comparator), not from a geo claim.
//!
//! # Fault injection (AC2 / F7 — Arm-1, latency only)
//!
//! With the `slo-fault-inject` feature, [`slo_inject_delay`] sleeps a fixed
//! 15ms INSIDE the measured A→B→A span (between `t0` and `t1`), so a mutation
//! test asserts `p95_us >= 14_000 && !budget_met` read from the SAME
//! [`super::build_journey_result`] the gate's `oracle_green` consumes. The
//! feature is release-guarded ([`compile_error!`]) + checked absent from the
//! release feature graph by the gate's `cargo tree --release` tripwire.

// Story 11.2b (F7): slo-fault-inject MUST NOT exist in release binaries. A
// release build with the fault-inject feature active is a ship-blocker — it
// would inject spurious latency into a production measurement path.
#[cfg(all(feature = "slo-fault-inject", not(debug_assertions)))]
compile_error!(
    "slo-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 11.2b ship-blocker)."
);

use std::time::Duration;

/// Cross-region round-trip P95 budget — a **loopback-calibrated regression
/// floor**, NOT a geo-SLO (F6/L7).
///
/// ⚠️ **NOT a geo-SLO.** Authority: measure-then-pin on the 11.2b 3-region
/// pilot. **Measured loopback p95 ≈ 21ms** (debug `cargo test` build, two
/// co-located Postgres on the dev rig: ~5 SQL ops + 2 Ed25519 sign/verify per
/// round-trip). Pinned at **30_000µs (30ms)** — ~43% headroom over the measured
/// p95 to absorb CI-runner variance without flaking, tight enough to catch a
/// ≥1.5× machinery/convergence regression. CI Postgres is co-located, so a real
/// geo-RTT is physically unobservable; pinning a geo number as a pass condition
/// is the 10.2 trap (loopback passes trivially). This floor binds **machinery +
/// convergence + regression** — the round-trip did not regress — and its teeth
/// come from the `slo-fault-inject` falsifier (which moves p95 through the
/// gate's own comparator), not from a geo-latency claim. The absolute geo-SLO
/// is a separately-tracked release-gate pilot artifact, NOT this constant.
///
/// Do NOT bump silently: record the measured p95 + rig + build mode in any
/// change (the `J4_P95_BUDGET_US` idiom). This is distinct from
/// `J4_P95_BUDGET_US` (10_000µs, kernel cross-task delivery) — same order of
/// magnitude, different measurement source + semantics.
pub const MULTI_REGION_SLO_P95_US: u64 = 30_000;
/// The fixed delay injected INSIDE the measured A→B→A span when the
/// `slo-fault-inject` feature is active (F7, Arm-1/latency only). 15ms — large
/// enough that the mutation test's `p95 >= 14_000µs` assertion is unambiguous
/// even with sub-ms real latency, mirroring the J4 `bench-fault-inject` budget.
pub const SLO_FAULT_INJECT_DELAY_US: u64 = 15_000;

/// Sleep the fault-inject delay INSIDE the measured span when the
/// `slo-fault-inject` feature is active; a no-op on the clean GREEN path.
///
/// The caller wraps the entire A→B→A round-trip (`build@A → apply(B) →
/// build@B → apply(A)`) in ONE `Instant` and calls this between `t0` and `t1`,
/// so the injected latency lands in the SAME span the histogram consumes.
#[allow(clippy::unused_self)] // no self; kept as a named seam for the F7 contract
pub fn slo_inject_delay() {
    // Feature-gated: only the `slo-fault-inject` build injects. The clean path
    // (no feature) is a true no-op so the GREEN budget_met assertion is honest.
    #[cfg(feature = "slo-fault-inject")]
    std::thread::sleep(Duration::from_micros(SLO_FAULT_INJECT_DELAY_US));
    #[cfg(not(feature = "slo-fault-inject"))]
    {
        // Clean path: no injection. The `Duration` import is still used by the
        // feature-on branch above; keep the binding to avoid an unused-import
        // warning on the clean build.
        let _ = Duration::from_micros(0);
    }
}
