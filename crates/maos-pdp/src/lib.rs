#![forbid(unsafe_code)]

//! `maos-pdp` — out-of-kernel enterprise Policy Decision Point adapter.
//!
//! Story 11.4a, ADR-050 / NFR-Sec-17. Implements the `maos-domain`
//! `PolicyDecisionPort` with a **real in-process Cedar engine** (F3). The
//! kernel mediates capability-authorization via the injected port trait; the
//! engine + its dependencies live HERE ONLY (never in `maos-kernel-core` or
//! `maos-domain` — enforced by `check-dependency-closure`). Nothing depends
//! on this crate except `maos-bin` — that is what keeps the Cedar dependency
//! at the binary edge.
//!
//! # Kernel delta
//!
//! ZERO kernel-core source delta from this crate. The bounded
//! `OperatorPolicyConfig.per_capability_deny` forbid layer (F2) lives in
//! `cap_policy/mod.rs`; the reconciler in `maos-bin` evaluates the operator
//! policy via this adapter and materializes `Deny` verdicts into that field
//! through the public `PolicyTable::update()` CoW swap (ADR-030 hot-path
//! preserved — the PDP is NEVER called on the token-verify hot path).
//!
//! # `pdp-fault-inject` (AC3 falsifier — dev/CI only)
//!
//! The `pdp-fault-inject` feature stubs the real Cedar engine to a canned
//! `Allow`. With it ON, a real `forbid` rule returns `Allow` → the deny test
//! goes RED, proving the deny is engine-derived (the anti-canned thesis). It
//! MUST NOT ship in release builds — the `compile_error!` below hard-fails any
//! release build with the feature on, and the `check-enterprise-pdp` gate's
//! `cargo tree --release` absence leg is the belt-and-suspenders graph guard.

// Story 11.4a AC3 — `pdp-fault-inject` is dev/CI-only. A release build
// (`not(debug_assertions)`) with this feature enabled MUST NOT compile.
// Mirrors the `churn-fault-inject` / `slo-fault-inject` ship-blocker idiom.
#[cfg(all(feature = "pdp-fault-inject", not(debug_assertions)))]
compile_error!(
    "pdp-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 11.4a ship-blocker)."
);
pub mod adapter;
pub mod reconcile;

pub use adapter::CedarPolicyAdapter;
pub use reconcile::{
    all_governed_deny_keys, reconcile_org_denies, reconcile_subject_denies,
    representative_governed_scopes, scope_deny_key, FailClosedOutcome, FailClosedPosture,
    FailClosedReconciler, MaterializedDenies,
};
