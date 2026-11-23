pub mod abi_diff;
pub mod build_test_fixtures;
pub mod check_empty_kernel;
pub mod check_error_catalog;
pub mod check_service_boundary;
pub mod clippy_ratchet;
mod fs_walk;
pub mod invariant_lock;
// Story 20-12 T1 — the escape-detector gate's `Report` → `emit` writer, exposed
// so `kernel_pin_content_hash_16_0` proves its JSON stdout over a stub `Report`
// instead of running all nine nested legs (1,259 s of the 1,871 s suite).
pub mod check_escape_detector;
pub mod check_fkcs;
pub mod check_host_surface;
pub mod check_kernel_baseline;
pub mod check_mock_not_in_release;
// Story 17-5 — the model-pin currency gate's audit, exposed so `xtask/tests/` drive
// the REAL `audit()` over planted trees (a parallel copy would prove nothing).
pub mod check_model_currency;
// Story 20-12 — the test-cost gates, exposed so their proven-red vectors in
// `xtask/tests/` drive the REAL `audit` over planted trees and junit reports.
pub mod check_test_budget;
pub mod check_test_cost;
pub mod check_trial_attestation;
// Story 20-7 — the performance budget file and the verdict / policy / count
// gates over it, exposed so `xtask/tests/` drive the REAL gates over fixtures.
pub mod check_perf_counts;
pub mod check_perf_runner_policy;
pub mod check_perf_verdict;
pub mod perf_budgets;
pub mod perf_ledger;
pub mod release_dry_run;
pub mod render_packages;
// corpus_types + gate_common exposed so the lib-compiled gate modules
// (check_fkcs, check_trial_attestation) can resolve `crate::gate_common`
// (Option C leg-binding, Epic 12 retro B1). corpus_types is self-contained;
// gate_common depends only on it + chrono.
pub mod corpus_types;
pub mod gate_common;
