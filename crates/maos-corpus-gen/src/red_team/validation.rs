//! Validation for red-team items.
//!
//! Checks structural well-formedness: all 8 binding fields present and
//! non-empty, class matches seed class, canonical_assertion matches.

use super::{RedTeamItem, RedTeamSeed};
use crate::ValidationOutcome;

/// Validate a single expanded red-team item against its originating seed.
pub fn validate_item(item: &RedTeamItem, seeds: &[RedTeamSeed]) -> ValidationOutcome {
    let Some(seed) = seeds.iter().find(|s| s.id == item.seed_id) else {
        return ValidationOutcome::Invalid {
            reason: format!("seed_id '{}' not found in seed corpus", item.seed_id),
        };
    };

    // Structural checks
    if item.class != seed.class {
        return ValidationOutcome::Invalid {
            reason: format!(
                "item class '{}' does not match seed class '{}'",
                item.class, seed.class
            ),
        };
    }

    if item.id.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty item id".to_string(),
        };
    }

    if item.scenario_description.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty scenario_description".to_string(),
        };
    }

    if item.expected_kernel_response.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty expected_kernel_response".to_string(),
        };
    }

    if item.expected_audit_signal.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty expected_audit_signal".to_string(),
        };
    }

    if item.canonical_assertion.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty canonical_assertion".to_string(),
        };
    }

    // Check canonical_assertion matches seed
    if item.canonical_assertion != seed.canonical_assertion {
        return ValidationOutcome::Invalid {
            reason: format!(
                "canonical_assertion mismatch: item has '{}', seed has '{}'",
                item.canonical_assertion, seed.canonical_assertion
            ),
        };
    }

    // Check class is one of the 8 binding identifiers
    const VALID_CLASSES: &[&str] = &[
        "capability_confusion",
        "iac_frame_injection",
        "distillation_poisoning",
        "ledger_tampering",
        "cross_spirit_privilege_escalation",
        "resource_exhaustion",
        "side_channel_timing",
        "kernel_syscall_abuse",
    ];

    if !VALID_CLASSES.contains(&item.class.as_str()) {
        return ValidationOutcome::Invalid {
            reason: format!("unknown class '{}' — must be one of the 8 §8.1 classes", item.class),
        };
    }

    ValidationOutcome::Valid
}
