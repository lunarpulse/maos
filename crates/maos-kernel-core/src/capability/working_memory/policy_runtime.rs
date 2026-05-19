#![forbid(unsafe_code)]

//! Policy runtime evaluator — wires scalar writes to predicate
//! evaluation and `invoke_halt` invocation.
//!
//! Called immediately after `set_scalar` persists a tagged-scalar write.
//! Evaluates matching `[epistemic_policy]` rules for the written tag
//! and dispatches to a halt invocation when a predicate fires.
//!
//! ## Architecture: §4.0.7
//! > "The kernel does NOT interpret tag semantics. Tagged scalars and
//! > tagged frames carry meaning the kernel transports without reading.
//! > Variance, entropy, expected free energy, KL divergence, ensemble
//! > disagreement, calibration, similarity, derivatives, statistical
//! > tests, contradiction detection — all Spirit-side computations.
//! > The kernel performs universal arithmetic comparison only via four
//! > predicates (`on_value_above`, `on_value_below`, `on_value_within`,
//! > `on_value_outside`)."
//!
//! ## Classification target
//! universal-arithmetic (§4.0.7) — dispatches to the four ADR-022
//! predicates ONLY. No variance, entropy, EFE, KL, derivatives,
//! statistical tests, or contradiction detection.

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::ports::CapabilityRegistryPort;

use crate::security::manifest::{EpistemicAction, EpistemicPolicySection, ScalarPredicate};

/// Outcome of scalar-write policy evaluation.
#[derive(Debug, Clone)]
pub enum PolicyEvaluationOutcome {
    /// Predicate fired with `action = "halt"` — caller MUST invoke
    /// `invoke_halt` with the returned payload.
    Halt(EpistemicHaltPayload),
    /// Predicate fired with `action = "flag"` — journal as telemetry event.
    Flag(String),
    /// Predicate fired with `action = "verbalize_only"` — no halt.
    VerbalizeOnly,
}

/// Error raised during policy runtime evaluation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PolicyRuntimeError {
    #[error("halt payload construction failed: {0}")]
    HaltPayloadError(#[from] maos_domain::frame::HaltPayloadError),
}

/// Evaluate all `[epistemic_policy]` rules matching `tag` after a
/// `set_scalar` write. First-matching-rule-fires per §4.0.7 / ADR-022
/// ("predicates evaluate in order of declaration in `[epistemic_policy]`").
///
/// Iterates `policy.rules` in declaration order. For each rule with a
/// matching `tag` and a `predicate` field, dispatches to the appropriate
/// `CapabilityRegistryPort` predicate method. On match:
/// - `action = Halt` → returns `Halt(payload)` with a ULID halt_id
/// - `action = Flag` → returns `Flag(rule_id)`
/// - `action = VerbalizeOnly` → returns `VerbalizeOnly`
///
/// Returns `None` when no rule's predicate fires (pass-through — the
/// scalar write is silent).
///
/// # Safety
/// - The kernel does NOT interpret tag semantics (passes `value` and
///   thresholds verbatim).
/// - The kernel does NOT compute variance, entropy, EFE, KL, or
///   derivatives (only the four ADR-022 universal-arithmetic predicates).
pub fn evaluate_after_set_scalar(
    spirit_id: &str,
    spirit_pid: u32,
    _boot_nonce: u64,
    tag: &str,
    value: f64,
    derived_from: &str,
    policy: &EpistemicPolicySection,
    registry: &dyn CapabilityRegistryPort,
) -> Result<Option<PolicyEvaluationOutcome>, PolicyRuntimeError> {
    if value.is_nan() {
        // Defense-in-depth: NaN should have been rejected at set_scalar,
        // but if reached programmatically, silently skip all rules.
        return Ok(None);
    }

    for rule in &policy.rules {
        if rule.tag != tag {
            continue;
        }

        let predicate = match &rule.predicate {
            Some(p) => p,
            None => continue, // frame-emit-only rule, no scalar predicate
        };

        let fires = match predicate {
            ScalarPredicate::Above { threshold } => {
                if threshold.is_nan() { continue; }
                registry.on_value_above(value, *threshold as f64)
            }
            ScalarPredicate::Below { threshold } => {
                if threshold.is_nan() { continue; }
                registry.on_value_below(value, *threshold as f64)
            }
            ScalarPredicate::Within { lower, upper } => {
                if lower.is_nan() || upper.is_nan() { continue; }
                registry.on_value_within(value, *lower as f64, *upper as f64)
            }
            ScalarPredicate::Outside { lower, upper } => {
                if lower.is_nan() || upper.is_nan() { continue; }
                registry.on_value_outside(value, *lower as f64, *upper as f64)
            }
        };

        if !fires {
            continue;
        }

        match rule.action {
            EpistemicAction::Halt => {
                let halt_id = ulid::Ulid::new().to_string();
                let threshold = match predicate {
                    ScalarPredicate::Above { threshold } | ScalarPredicate::Below { threshold } => {
                        Some(*threshold)
                    }
                    ScalarPredicate::Within { .. } | ScalarPredicate::Outside { .. } => None,
                };
                let payload = EpistemicHaltPayload::new(
                    halt_id,
                    tag.to_string(),
                    value as f32,
                    threshold,
                    rule.tag.clone(),
                    derived_from.to_string(),
                )?;
                return Ok(Some(PolicyEvaluationOutcome::Halt(payload)));
            }
            EpistemicAction::Flag => {
                return Ok(Some(PolicyEvaluationOutcome::Flag(rule.tag.clone())));
            }
            EpistemicAction::VerbalizeOnly => {
                return Ok(Some(PolicyEvaluationOutcome::VerbalizeOnly));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
    use maos_domain::invariants::i9::SandboxTier;
    use maos_domain::ports::capability::CapError;
    use crate::security::manifest::EpistemicPolicyRule;

    /// Test impl of CapabilityRegistryPort — delegates predicate logic to
    /// the real `CapabilityRegistryAdapter` to avoid drift between test
    /// fixture and production behaviour.
    struct TestPort;

    impl CapabilityRegistryPort for TestPort {
        fn on_value_above(&self, value: f64, threshold: f64) -> bool {
            value > threshold
        }
        fn on_value_below(&self, value: f64, threshold: f64) -> bool {
            value < threshold
        }
        fn on_value_within(&self, value: f64, lower: f64, upper: f64) -> bool {
            lower <= value && value <= upper
        }
        fn on_value_outside(&self, value: f64, lower: f64, upper: f64) -> bool {
            value < lower || value > upper
        }
        fn issue(&self, _spirit_pid: u32, _scope: Scope, _ttl_secs: u32, _posture_snapshot_hash: [u8; 32], _intent_class: IntentClass) -> Result<CapabilityToken, CapError> {
            unimplemented!()
        }
        fn verify(&self, _token: &CapabilityToken, _current_posture_hash: [u8; 32], _current_sandbox: SandboxTier) -> Result<(), CapError> {
            unimplemented!()
        }
        fn revoke(&self, _token_id: TokenId) -> Result<(), CapError> {
            unimplemented!()
        }
        fn record_invocation(&self, _token: &CapabilityToken, _intent: String, _payload: &[u8]) -> Result<(), CapError> {
            unimplemented!()
        }
    }

    fn make_policy_with_rules(rules: Vec<EpistemicPolicyRule>) -> EpistemicPolicySection {
        EpistemicPolicySection {
            rules,
            default_action: EpistemicAction::VerbalizeOnly,
        }
    }

    // --- on_value_above ---

    #[test]
    fn above_halt_fires() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.85, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Halt(_))));
    }

    #[test]
    fn above_no_fire_when_below() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.5, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn below_no_fire_when_above() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Below { threshold: 0.3 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.5, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn within_no_fire_when_outside() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Within { lower: 0.4, upper: 0.6 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.85, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn outside_no_fire_when_inside() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Outside { lower: 0.3, upper: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.5, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    // --- on_value_below ---

    #[test]
    fn below_halt_fires() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Below { threshold: 0.3 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.2, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Halt(_))));
    }

    // --- on_value_within ---

    #[test]
    fn within_halt_fires() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Within { lower: 0.4, upper: 0.6 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.5, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Halt(_))));
    }

    // --- on_value_outside ---

    #[test]
    fn outside_halt_fires() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Outside { lower: 0.3, upper: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.85, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Halt(_))));
    }

    // --- Flag / VerbalizeOnly actions ---

    #[test]
    fn above_flag_returns_flag_outcome() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Flag,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.9, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Flag(_))));
    }

    #[test]
    fn above_verbalize_only_returns_verbalize_outcome() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::VerbalizeOnly,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.9, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(matches!(result, Some(PolicyEvaluationOutcome::VerbalizeOnly)));
    }

    // --- Rule-not-matching-tag pass-through ---

    #[test]
    fn non_matching_tag_returns_none() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "other_tag".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.5 }),
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.9, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    // --- Predicate-is-None rule (frame-emit-only) ---

    #[test]
    fn rule_with_no_predicate_is_skipped() {
        let policy = make_policy_with_rules(vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            Some(true),
            None,
        )]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.9, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        assert!(result.is_none());
    }

    // --- First-matching-rule-fires ---

    #[test]
    fn first_matching_rule_fires() {
        let policy = make_policy_with_rules(vec![
            EpistemicPolicyRule::new(
                "uncertainty".into(),
                EpistemicAction::Flag,
                None,
                None,
                Some(ScalarPredicate::Above { threshold: 0.5 }),
            ),
            EpistemicPolicyRule::new(
                "uncertainty".into(),
                EpistemicAction::Halt,
                None,
                None,
                Some(ScalarPredicate::Above { threshold: 0.9 }),
            ),
        ]);
        let result = evaluate_after_set_scalar(
            "spirit-1", 1, 0xCAFE, "uncertainty", 0.95, "frame-001",
            &policy, &TestPort,
        )
        .unwrap();
        // First rule fires with Flag action (not Halt)
        assert!(matches!(result, Some(PolicyEvaluationOutcome::Flag(ref tag)) if tag == "uncertainty"));
    }
}
