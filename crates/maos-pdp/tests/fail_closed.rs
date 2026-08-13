//! Story 11.4a AC4 — fail-closed posture on PDP unavailability (F4).
//!
//! The security invariant: an enterprise PDP failing OPEN (Allow on
//! unreachable) is a P0. The adapter MUST fail closed — a configured PDP that
//! cannot evaluate degrades to a typed `PolicyDecisionError::Unreachable` /
//! `InvalidPolicy`, never to a permissive default. These are the port-level
//! fail-closed legs; the four distinct reconciler timelines (no-config /
//! configured-down-loud / runtime-freeze / TTL-revert) are exercised at the
//! composition-root + gate level.

use maos_domain::invariants::i1::Scope;
use maos_domain::ports::{
    PolicyDecisionError, PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict,
};
use maos_pdp::{scope_deny_key, CedarPolicyAdapter, FailClosedPosture, FailClosedReconciler};
use std::collections::{HashMap, HashSet};

#[cfg(not(feature = "pdp-fault-inject"))]
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

#[cfg(not(feature = "pdp-fault-inject"))]
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

// A port whose verdicts are keyed by (spirit_pid, capability_key). Distinct
// from FlakyPort: it handles the subject-aware reconcile calls, where the org
// leg submits N requests (spirit_pid 0) and the subject leg submits N * spirit
// requests — a single canned verdict vector would trip the reconciler's
// cardinality-mismatch guard on the subject leg.
struct SubjectKeyedPort {
    denies: HashMap<u32, HashSet<String>>,
    available: bool,
}

impl PolicyDecisionPort for SubjectKeyedPort {
    fn load_policy(&self, _policy_text: &str) -> Result<(), PolicyDecisionError> {
        Ok(())
    }
    fn evaluate(
        &self,
        requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
        if !self.available {
            return Err(PolicyDecisionError::Unreachable {
                reason: "injected PDP failure".into(),
            });
        }
        Ok(requests
            .iter()
            .map(|r| {
                if self
                    .denies
                    .get(&r.spirit_pid)
                    .is_some_and(|denied| denied.contains(&r.capability_key))
                {
                    PolicyVerdict::Deny
                } else {
                    PolicyVerdict::Allow
                }
            })
            .collect())
    }
    fn is_healthy(&self) -> bool {
        self.available
    }
}

#[test]
fn reconcile_with_subjects_freezes_subject_denies_before_ttl() {
    // Story 11.4a — reconcile_with_subjects_at must freeze the last-known-good
    // SUBJECT denies (per_spirit) on a within-TTL runtime drop, not just the
    // global deny_keys. The existing freeze test covers the global leg via
    // reconcile_at (no subjects); this pins the subject leg directly.
    //
    // Two spirits with the deny on spirit 7 ONLY, so fs.read stays genuinely
    // subject-scoped (denied for one of two spirits ⇒ it does NOT fold into
    // MaterializedDenies.global) and the freeze can be observed on per_spirit
    // without ambiguity.
    let fs_read = scope_deny_key(&Scope::FsRead {
        subtree: String::new(),
    });
    let fresh = SubjectKeyedPort {
        denies: HashMap::from([(7, HashSet::from([fs_read.clone()]))]),
        available: true,
    };
    let down = SubjectKeyedPort {
        denies: HashMap::new(),
        available: false,
    };
    let mut reconciler = FailClosedReconciler::new(1000);

    let first = reconciler.reconcile_with_subjects_at(&fresh, &[7, 8], 100);
    assert_eq!(first.posture, FailClosedPosture::Fresh);
    let spirit7 = first
        .subject_denies
        .per_spirit
        .get(&7)
        .expect("spirit 7 subject deny materialized on a fresh eval");
    assert!(
        spirit7.contains(&fs_read),
        "fresh eval applies the subject-scoped deny for spirit 7"
    );
    assert!(
        !first.subject_denies.global.contains(&fs_read),
        "fs.read denied for one of two spirits stays subject-scoped (not global)"
    );

    // PDP drops within TTL — the subject denies must be frozen verbatim.
    let frozen = reconciler.reconcile_with_subjects_at(&down, &[7, 8], 200);
    assert_eq!(frozen.posture, FailClosedPosture::RuntimeFreeze);
    assert_eq!(
        frozen.subject_denies, first.subject_denies,
        "subject denies are frozen verbatim on a within-TTL runtime drop"
    );
    assert!(
        frozen
            .subject_denies
            .per_spirit
            .get(&7)
            .is_some_and(|d| d.contains(&fs_read)),
        "the frozen subject deny for spirit 7 survives the drop"
    );
}
