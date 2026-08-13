#![cfg(not(feature = "pdp-fault-inject"))]

//! Story 11.4a AC2 — decisions come from REAL policy evaluation, not a
//! hardcoded map (the anti-canned thesis).
//!
//! These tests prove the Cedar adapter forwards each `evaluate` call to the
//! real engine. The central control is a **policy-swap-flips-verdict**: the
//! SAME request yields `Allow` under policy-A and `Deny` under policy-B — a
use maos_domain::ports::{PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict};
use maos_pdp::CedarPolicyAdapter;

fn req(pid: u32, key: &str) -> PolicyDecisionRequest {
    PolicyDecisionRequest {
        spirit_pid: pid,
        capability_key: key.to_string(),
        principal_attributes: None,
    }
}

/// Policy A: PERMIT the `FsRead` action for any principal.
const POLICY_A_PERMIT: &str = r#"permit(principal, action == Action::"FsRead", resource);"#;

/// Policy B: FORBID the `FsRead` action for any principal (Cedar `forbid`).
const POLICY_B_FORBID: &str = r#"forbid(principal, action == Action::"FsRead", resource);"#;

#[test]
fn policy_swap_flips_verdict_allow_to_deny() {
    // AC2 — the SAME request yields Allow under policy-A and Deny under
    // policy-B. A hardcoded map cannot flip when only the policy text
    // changes; this is the engine re-evaluating against a new policy set.
    let adapter = CedarPolicyAdapter::new();
    let request = req(42, "FsRead");

    adapter
        .load_policy(POLICY_A_PERMIT)
        .expect("policy A parses");
    let under_a = adapter.evaluate(&[request.clone()]).expect("eval A");
    assert_eq!(under_a, vec![PolicyVerdict::Allow]);

    // SWAP to the forbid policy — same request, opposite verdict.
    adapter
        .load_policy(POLICY_B_FORBID)
        .expect("policy B parses");
    let under_b = adapter.evaluate(&[request.clone()]).expect("eval B");
    assert_eq!(
        under_b,
        vec![PolicyVerdict::Deny],
        "policy swap MUST flip the verdict — a hardcoded map cannot"
    );
}

#[test]
fn swap_forces_re_evaluation_not_memoized_cache() {
    // CARRIED (Murat) — the swap forces the engine to RE-evaluate. A cache
    // that returned the first `Allow` would pass a naive swap test; this test
    // swaps A→B→A and asserts the verdict tracks the policy each time (Allow,
    // Deny, Allow), proving each call is a DISTINCT engine evaluation, not a
    // memoized first result.
    let adapter = CedarPolicyAdapter::new();
    let request = req(7, "FsRead");

    adapter.load_policy(POLICY_A_PERMIT).unwrap();
    assert_eq!(
        adapter.evaluate(&[request.clone()]).unwrap(),
        vec![PolicyVerdict::Allow]
    );

    adapter.load_policy(POLICY_B_FORBID).unwrap();
    assert_eq!(
        adapter.evaluate(&[request.clone()]).unwrap(),
        vec![PolicyVerdict::Deny]
    );

    // Swap BACK to permit — a cache would still return the Deny from the
    // prior call. The engine re-evaluates → Allow.
    adapter.load_policy(POLICY_A_PERMIT).unwrap();
    assert_eq!(
        adapter.evaluate(&[request.clone()]).unwrap(),
        vec![PolicyVerdict::Allow],
        "swap-back must re-evaluate (Allow), not serve a cached Deny"
    );
}

#[test]
fn canned_map_negative_control_empty_policy_denies() {
    // AC2 — a canned-map negative control: with an empty policy (no rules),
    // the adapter has NO baked-in verdict. Cedar's default is DENY (no permit
    // → deny), so an empty policy denies everything — the adapter does NOT
    // silently return a hardcoded `Allow`. (Contrast: a `HashMap` literal
    // stand-in would return its baked verdict regardless of the policy.)
    let adapter = CedarPolicyAdapter::new();
    adapter
        .load_policy("")
        .expect("empty policy parses to an empty PolicySet");
    let requests = [req(1, "FsRead"), req(2, "ProcExec"), req(3, "NetHttps")];
    let verdicts = adapter.evaluate(&requests).expect("eval on empty policy");
    assert!(
        verdicts.iter().all(|v| *v == PolicyVerdict::Deny),
        "empty policy must deny all (Cedar default-deny) — no baked-in Allow: {verdicts:?}"
    );
}

#[test]
fn derive_and_reconcile_count_vs_request_cardinality() {
    // AC2 — the allow/deny count is DERIVED per-request from actual engine
    // calls and reconciled against the request-set cardinality (never a
    // committed/hardcoded literal — the 11.2a vacuous-count lesson).
    let adapter = CedarPolicyAdapter::new();
    // Policy: permit FsRead + NetHttps; forbid ProcExec.
    let policy = r#"
        permit(principal, action == Action::"FsRead", resource);
        permit(principal, action == Action::"NetHttps", resource);
        forbid(principal, action == Action::"ProcExec", resource);
    "#;
    adapter.load_policy(policy).unwrap();

    let requests = [
        req(1, "FsRead"),
        req(1, "NetHttps"),
        req(1, "ProcExec"),
        req(2, "FsRead"),
    ];
    let verdicts = adapter.evaluate(&requests).expect("eval");
    // Reconcile: one verdict per request (cardinality match — derive-and-
    // reconcile, not a committed count).
    assert_eq!(verdicts.len(), requests.len(), "one verdict per request");
    let allow_count = verdicts
        .iter()
        .filter(|v| **v == PolicyVerdict::Allow)
        .count();
    let deny_count = verdicts
        .iter()
        .filter(|v| **v == PolicyVerdict::Deny)
        .count();
    // Derived from the real engine output: FsRead×2 + NetHttps×1 = 3 Allow;
    // ProcExec×1 = 1 Deny. Empty-policy N/A does NOT appear here (non-vacuous).
    assert_eq!(
        allow_count, 3,
        "derived allow count from real engine output"
    );
    assert_eq!(deny_count, 1, "derived deny count from real engine output");
    assert_eq!(allow_count + deny_count, requests.len());
}

#[test]
fn malformed_policy_is_invalid_not_silent_allow() {
    // Fail-loud: a malformed policy returns InvalidPolicy (a configured PDP
    // that can't load its policy is treated as unreachable → fail-closed,
    // F4). It does NOT silently degrade to a permissive canned verdict.
    let adapter = CedarPolicyAdapter::new();
    let err = adapter.load_policy("this is not cedar policy syntax @@@");
    assert!(
        err.is_err(),
        "malformed policy must be rejected, not silently accepted"
    );
    // is_healthy stays false (no valid policy loaded).
    assert!(!adapter.is_healthy());
}
