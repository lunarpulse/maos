//! Story 11.4a AC4 — fail-closed posture on PDP unavailability (F4).
//!
//! The security invariant: an enterprise PDP failing OPEN (Allow on
//! unreachable) is a P0. The adapter MUST fail closed — a configured PDP that
//! cannot evaluate degrades to a typed `PolicyDecisionError::Unreachable` /
//! `InvalidPolicy`, never to a permissive default. These are the port-level
//! fail-closed legs; the four distinct reconciler timelines (no-config /
//! configured-down-loud / runtime-freeze / TTL-revert) are exercised at the
//! composition-root + gate level.

use maos_domain::ports::{
    PolicyDecisionError, PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict,
};
use maos_pdp::{CedarPolicyAdapter, FailClosedPosture, FailClosedReconciler};

#[test]
fn no_policy_loaded_evaluate_is_unreachable() {
    // AC4 leg 2 (configured-down-at-startup, port-level): evaluating BEFORE a
    // policy is loaded is treated as unreachable — fail closed, never a silent
    // permissive verdict. (Distinct from AC1's "no PDP configured" → the
    // reconciler holds `Option::None` and uses the kernel default. Here the
    // adapter EXISTS but has no usable policy.)
    let adapter = CedarPolicyAdapter::new();
    assert!(!adapter.is_healthy(), "unloaded adapter is not healthy");
    let req = PolicyDecisionRequest {
        spirit_pid: 1,
        capability_key: "FsRead".into(),
        principal_attributes: None,
    };
    let err = adapter.evaluate(&[req]).unwrap_err();
    assert!(
        matches!(err, PolicyDecisionError::Unreachable { .. }),
        "evaluate with no policy must be Unreachable (fail-closed), got {err:?}"
    );
}

#[test]
fn malformed_policy_keeps_adapter_unhealthy() {
    // A configured PDP that FAILS to load its policy stays unhealthy — the
    // reconciler treats this as fail-closed (F4). is_healthy is the hook.
    let adapter = CedarPolicyAdapter::new();
    assert!(adapter.load_policy("@@@ not cedar @@@").is_err());
    assert!(!adapter.is_healthy());
}

#[test]
fn loaded_policy_makes_adapter_healthy() {
    // Positive control: a successfully-loaded policy flips is_healthy true.
    let adapter = CedarPolicyAdapter::new();
    adapter
        .load_policy(r#"permit(principal, action, resource);"#)
        .unwrap();
    assert!(adapter.is_healthy());
}

#[test]
fn evaluate_never_returns_silent_allow_on_error() {
    // The fail-OPEN P0 class: an error condition must NEVER degrade to a
    // silent `Allow`. Every error path is a typed `PolicyDecisionError` (the
    // caller fail-closes); there is no `Ok([Allow, ...])` escape hatch on
    // unreachable/invalid. This test pins the contract by exhausting the
    // error conditions reachable without a loaded policy.
    let adapter = CedarPolicyAdapter::new();
    let reqs = vec![PolicyDecisionRequest {
        spirit_pid: 1,
        capability_key: "FsRead".into(),
        principal_attributes: None,
    }];
    let result = adapter.evaluate(&reqs);
    assert!(
        result.is_err(),
        "no silent Allow on an unloadable PDP — must be a typed error (fail-closed)"
    );
}

struct FlakyPort {
    verdicts: Option<Vec<PolicyVerdict>>,
}

impl PolicyDecisionPort for FlakyPort {
    fn load_policy(&self, _policy_text: &str) -> Result<(), PolicyDecisionError> {
        Ok(())
    }

    fn evaluate(
        &self,
        _requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
        self.verdicts
            .clone()
            .ok_or_else(|| PolicyDecisionError::Unreachable {
                reason: "simulated PDP failure".into(),
            })
    }

    fn is_healthy(&self) -> bool {
        self.verdicts.is_some()
    }
}

#[test]
fn runtime_drop_freezes_last_known_good_before_ttl() {
    let fresh = FlakyPort {
        verdicts: Some(vec![
            PolicyVerdict::Deny;
            maos_pdp::representative_governed_scopes().len()
        ]),
    };
    let down = FlakyPort { verdicts: None };
    let mut reconciler = FailClosedReconciler::new(10);
    let first = reconciler.reconcile_at(&fresh, 100);
    assert_eq!(first.posture, FailClosedPosture::Fresh);
    let frozen = reconciler.reconcile_at(&down, 105);
    assert_eq!(frozen.posture, FailClosedPosture::RuntimeFreeze);
    assert_eq!(frozen.deny_keys, first.deny_keys);
}

#[test]
fn ttl_expiry_reverts_to_all_governed_denies() {
    let fresh = FlakyPort {
        verdicts: Some(vec![
            PolicyVerdict::Allow;
            maos_pdp::representative_governed_scopes().len()
        ]),
    };
    let down = FlakyPort { verdicts: None };
    let mut reconciler = FailClosedReconciler::new(10);
    let first = reconciler.reconcile_at(&fresh, 100);
    assert_eq!(first.posture, FailClosedPosture::Fresh);
    assert!(first.deny_keys.is_empty());
    let expired = reconciler.reconcile_at(&down, 111);
    assert_eq!(expired.posture, FailClosedPosture::TtlExpiredRevert);
    assert_eq!(
        expired.deny_keys.len(),
        maos_pdp::all_governed_deny_keys().len()
    );
}

#[test]
fn startup_unreachable_denies_all_governed_scopes() {
    let down = FlakyPort { verdicts: None };
    let mut reconciler = FailClosedReconciler::new(10);
    let closed = reconciler.reconcile_at(&down, 0);
    assert_eq!(closed.posture, FailClosedPosture::StartupClosed);
    assert_eq!(
        closed.deny_keys.len(),
        maos_pdp::all_governed_deny_keys().len()
    );
}
