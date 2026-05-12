//! Validation for secret-redaction items.
//!
//! # Load-bearing P0 false-negative detector
//!
//! An item surface a `FalseNegativeRisk` when its `raw` form contains a
//! secret-shaped pattern that, given the seed's `pattern_regex`, would match
//! in a real run but the item's `expected_redacted` does NOT contain a
//! `<REDACTED:...>` marker.  This catches seeds that are mis-classified or
//! whose synthetic raw form does not actually embed the synthetic secret.

use super::{SecretRedactionItem, SecretRedactionSeed};
use crate::ValidationOutcome;

/// Validate a single expanded item against its originating seed.
pub fn validate_item(item: &SecretRedactionItem, seeds: &[SecretRedactionSeed]) -> ValidationOutcome {
    let Some(seed) = seeds.iter().find(|s| s.id == item.seed_id) else {
        return ValidationOutcome::Invalid {
            reason: format!("seed_id '{}' not found in seed corpus", item.seed_id),
        };
    };

    // --- Structural checks ---

    if item.class != seed.class {
        return ValidationOutcome::FalseNegativeRisk {
            detail: format!(
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

    if item.raw.is_empty() {
        return ValidationOutcome::Invalid {
            reason: "empty raw field".to_string(),
        };
    }

    // --- False-negative risk: raw does not contain a redactable pattern ---
    // At v0.1-α we don't execute the regex; we check that `expected_redacted`
    // is well-formed and that `raw` contains a synthetic-secret-like token.
    if !item.expected_redacted.starts_with("<REDACTED:type=") {
        return ValidationOutcome::FalseNegativeRisk {
            detail: format!(
                "expected_redacted doesn't start with '<REDACTED:type=': {}",
                &item.expected_redacted[..item.expected_redacted.len().min(80)]
            ),
        };
    }

    // Heuristic: all v0.1-α synthetic secrets contain "test" (case-insensitive).
    // This is the synthetic indicator that a real redactor should recognize.
    let has_synthetic_indicator = item.raw.to_lowercase().contains("test");

    if !has_synthetic_indicator {
        return ValidationOutcome::FalseNegativeRisk {
            detail: format!(
                "raw does not contain synthetic indicator ('-TEST-' etc.): '{}'. A real redactor might not detect this as synthetic.",
                &item.raw[..item.raw.len().min(100)]
            ),
        };
    }

    ValidationOutcome::Valid
}
