#![forbid(unsafe_code)]

//! AC2 integration test — exercises the policy runtime evaluator across
//! all four predicate forms × Halt/Flag/VerbalizeOnly actions (12
//! combinations) plus rule-not-matching-tag pass-through.
//!
//! Uses `EpistemicPolicySection::from_toml_str` for manifest parsing
//! (does NOT bypass the parser with struct literals — per the
//! regression A3 closed in Story 4.1).

use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::CapabilityRegistryPort;
use maos_kernel_core::capability::working_memory::policy_runtime::{
    evaluate_after_set_scalar, PolicyEvaluationOutcome,
};
use maos_kernel_core::capability::{
    cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::security::manifest::EpistemicPolicySection;

/// Real adapter — required for CapabilityRegistryPort dispatch in
/// the evaluator (accesses the four predicate methods).
fn make_adapter() -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _) = maos_kernel_core::capability::cap_audit::channel();
    let quota = CapQuotaTracker::new();
    let working_memory = Arc::new(WorkingMemoryStore::new());
    let telemetry = Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default());
    CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xCAFE,
        policy,
        audit_tx,
        quota,
        working_memory,
        telemetry,
    )
}

fn make_policy(toml_str: &str) -> EpistemicPolicySection {
    EpistemicPolicySection::from_toml_str(toml_str).unwrap()
}

#[test]
fn on_value_above_halt_fires() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_above = { threshold = 0.7 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Halt(_))));
}

#[test]
fn on_value_above_flag_returns_flag() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "flag"
on_value_above = { threshold = 0.7 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Flag(_))));
}

#[test]
fn on_value_above_verbalize_only_returns_verbalize() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "verbalize_only"
on_value_above = { threshold = 0.7 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Some(PolicyEvaluationOutcome::VerbalizeOnly)
    ));
}

#[test]
fn on_value_below_halt_fires() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_below = { threshold = 0.3 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.2,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Halt(_))));
}

#[test]
fn on_value_within_halt_fires() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_within = { lower = 0.4, upper = 0.6 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.5,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Halt(_))));
}

#[test]
fn on_value_outside_halt_fires() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_outside = { lower = 0.3, upper = 0.7 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Halt(_))));
}

#[test]
fn on_value_within_flag_returns_flag() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "flag"
on_value_within = { lower = 0.3, upper = 0.6 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.45,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Flag(_))));
}

#[test]
fn on_value_outside_verbalize_only_returns_verbalize() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "verbalize_only"
on_value_outside = { lower = 0.2, upper = 0.5 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.9,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Some(PolicyEvaluationOutcome::VerbalizeOnly)
    ));
}

#[test]
fn rule_not_matching_tag_returns_none() {
    let policy = make_policy(
        r#"[[rules]]
tag = "other_tag"
action = "halt"
on_value_above = { threshold = 0.5 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.9,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(outcome.is_none());
}

#[test]
fn on_value_below_flag_returns_flag() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "flag"
on_value_below = { threshold = 0.3 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.1,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Flag(_))));
}

#[test]
fn on_value_below_verbalize_only_returns_verbalize() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "verbalize_only"
on_value_below = { threshold = 0.5 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.3,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Some(PolicyEvaluationOutcome::VerbalizeOnly)
    ));
}

#[test]
fn on_value_within_verbalize_only_returns_verbalize() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "verbalize_only"
on_value_within = { lower = 0.3, upper = 0.6 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.5,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Some(PolicyEvaluationOutcome::VerbalizeOnly)
    ));
}

#[test]
fn on_value_outside_flag_returns_flag() {
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "flag"
on_value_outside = { lower = 0.3, upper = 0.6 }"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.9,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Flag(_))));
}

#[test]
fn on_confidence_below_desugar_halt_fires() {
    // Story 3.2 backward compat — on_confidence_below desugars to Below
    let policy = make_policy(
        r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_confidence_below = 0.5"#,
    );
    let adapter = make_adapter();
    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.3,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();
    assert!(matches!(outcome, Some(PolicyEvaluationOutcome::Halt(_))));
}
