//! Integration test: Capability policy default rules for Story 4.4 capabilities.

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use maos_kernel_core::capability::cap_policy::decision::{
    ApprovalClass, Capability, Intent, PolicyDecision, TrustTier,
};
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};

fn make_policy_table() -> PolicyTable {
    let table = PolicyTable::new();
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        1,
        ManifestCapabilityScope {
            scopes: vec![Scope::LogRecall, Scope::LogFetch, Scope::DistillateWrite],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    table.update(inner);
    table
}

#[test]
fn log_recall_policy_decision_is_allow() {
    let table = make_policy_table();
    let decision = table.evaluate(
        1,
        &Capability {
            scope: Scope::LogRecall,
        },
        &Intent::LogRecall,
    );
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn log_fetch_policy_decision_is_allow() {
    let table = make_policy_table();
    let decision = table.evaluate(
        1,
        &Capability {
            scope: Scope::LogFetch,
        },
        &Intent::LogFetch,
    );
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn distillate_write_policy_decision_is_require_approval() {
    let table = make_policy_table();
    let decision = table.evaluate(
        1,
        &Capability {
            scope: Scope::DistillateWrite,
        },
        &Intent::DistillateWrite,
    );
    assert_eq!(
        decision,
        PolicyDecision::RequireApproval {
            class: ApprovalClass::Interactive,
        }
    );
}
