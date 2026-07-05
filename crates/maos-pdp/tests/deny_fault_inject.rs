//! Story 11.4a AC3 — deny-rule proven-red: a real policy `forbid` actually
//! blocks a capability, and the `pdp-fault-inject` falsifier stubs the engine
//! to a canned `Allow` and reds the deny (proving the deny is engine-derived,
//! not a constant — §A7.3 "feature-flag ≠ measurement" done right: the flag
//! REMOVES the real engine, and the verdict flips).
//!
//! # The two-test contrast
//!
//! - `deny_forbid_returns_deny_through_port` (no feature) — the REAL engine
//!   evaluates a `forbid` rule → `Deny`.
//! - `fault_inject_stubs_deny_to_allow` (`pdp-fault-inject`, `#[ignore]`) —
//!   the SAME `forbid` policy under the stub → `Allow`. The verdict FLIPPED
//!   because the real engine was removed → the deny is engine-derived.
//!
//! The `check-enterprise-pdp` gate runs both and asserts the contrast (real →
//! Deny, fault-inject → Allow); the gate-controlled `#[ignore]` keeps the
//! fault-inject test out of the default `cargo test` run.

use maos_domain::ports::{PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict};
use maos_pdp::CedarPolicyAdapter;

fn req(pid: u32, key: &str) -> PolicyDecisionRequest {
    PolicyDecisionRequest {
        spirit_pid: pid,
        capability_key: key.to_string(),
        principal_attributes: None,
    }
}

const POLICY_FORBID_FSREAD: &str =
    r#"forbid(principal, action == Action::"FsRead", resource);"#;

#[test]
fn deny_forbid_returns_deny_through_port() {
    // AC3 (real engine) — a `forbid` rule produces `Deny` via the real Cedar
    // authorizer. This is the verdict the reconciler materializes into the
    // bounded `per_capability_deny` kernel layer (F2), which the mediated
    // `issue_with_mediation` path then enforces as `Err(PolicyDenied)`.
    let adapter = CedarPolicyAdapter::new();
    adapter.load_policy(POLICY_FORBID_FSREAD).unwrap();
    let verdicts = adapter.evaluate(&[req(42, "FsRead")]).unwrap();
    assert_eq!(
        verdicts,
        vec![PolicyVerdict::Deny],
        "real engine forbid MUST return Deny"
    );
}

#[test]
fn permit_returns_allow_contrast_with_forbid() {
    // Contrast leg — under a PERMIT policy the SAME request returns Allow.
    // (Together with `deny_forbid_returns_deny_through_port` this is the
    // policy-swap-flips-verdict at the port level — the deny is the engine's
    // verdict, not a constant Deny.)
    let adapter = CedarPolicyAdapter::new();
    adapter
        .load_policy(r#"permit(principal, action == Action::"FsRead", resource);"#)
        .unwrap();
    let verdicts = adapter.evaluate(&[req(42, "FsRead")]).unwrap();
    assert_eq!(verdicts, vec![PolicyVerdict::Allow]);
}

// ───────────────────── pdp-fault-inject falsifier (dev/CI only) ─────────────────────
//
// The feature stubs the engine to a canned `Allow`. With it ON, the SAME
// `forbid` policy returns `Allow` (not `Deny`) — the deny leg REDS, proving
// the deny is engine-derived. The `compile_error!` in `src/lib.rs` blocks any
// release build with this feature; the gate runs this via `--features
// pdp-fault-inject --ignored`.

#[cfg(feature = "pdp-fault-inject")]
#[ignore = "requires --features pdp-fault-inject; gate-controlled via check-enterprise-pdp"]
#[test]
fn fault_inject_stubs_deny_to_allow() {
    // Under pdp-fault-inject the real engine is REMOVED — `evaluate` returns a
    // canned `Allow` regardless of the loaded `forbid` policy. This is the
    // falsifier: the verdict flipped from Deny (real engine) to Allow (stub)
    // because the engine was taken out, proving the deny is engine-derived.
    let adapter = CedarPolicyAdapter::new();
    adapter.load_policy(POLICY_FORBID_FSREAD).unwrap();
    let verdicts = adapter.evaluate(&[req(42, "FsRead")]).unwrap();
    assert_eq!(
        verdicts,
        vec![PolicyVerdict::Allow],
        "pdp-fault-inject stub MUST return canned Allow (the deny reds)"
    );
}
