#![forbid(unsafe_code)]

//! ADR-036 precheck reporter — pure-function path that does NOT mutate
//! kernel state. Returns a `PrecheckVerdict` with exit-code semantics.
//!
//! Used by `maosctl spirit hot-swap-precheck` (Story 5.2, REPORTING-ONLY
//! at v0.3-β). Story 5.4's `maosctl spirit upgrade` calls this internally
//! before the actual swap.

/// Precheck verdict outcome.
pub use maos_domain::hot_swap::PrecheckOutcome;

/// Schema compatibility classification.
pub use maos_domain::hot_swap::SchemaCompat;

/// Verdict returned by the precheck function.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrecheckVerdict {
    pub verdict: PrecheckOutcome,
    pub predecessor_halt_protocol_version: u32,
    pub successor_accepted_versions: Vec<u32>,
    pub drained_count: Option<usize>,
    pub migrated_count: Option<usize>,
    pub schema_compat: SchemaCompat,
    pub auto_revert_window_seconds: u32,
}

/// The precheck function — pure, does NOT mutate kernel state.
pub struct HotSwapPrecheck;

impl HotSwapPrecheck {
    /// Run a dry-run precheck for a prospective hot-swap.
    ///
    /// Reads predecessor state and successor manifest declaratively,
    /// then calls `validate_swap_halt_continuity_dry_run` to produce
    /// a verdict without mutating the halt registry.
    pub fn check(
        halt_registry: &crate::halt::HaltRegistry,
        _predecessor_pid: u32,
        predecessor_halt_protocol_version: u32,
        successor_accepted_versions: &[u32],
        predecessor_state_schema_version: u32,
        successor_state_schema_version: u32,
    ) -> PrecheckVerdict {
        let schema_compat = super::state_codec::detect_compat(
            predecessor_state_schema_version,
            successor_state_schema_version,
        );

        let version_list = successor_accepted_versions.to_vec();
        let pending_count = halt_registry.pending_halt_ids().len();

        let verdict = match schema_compat {
            SchemaCompat::Breaking => PrecheckOutcome::SchemaIncompatible,
            SchemaCompat::SameMajor | SchemaCompat::CrossMajor => {
                // Check halt continuity dry-run style (no mutation).
                match crate::halt::validate_halt_set(
                    &halt_registry.pending_halt_ids(),
                    predecessor_halt_protocol_version,
                    Some(successor_accepted_versions),
                ) {
                    Ok(()) => {
                        if pending_count == 0 {
                            PrecheckOutcome::SafeDrained
                        } else {
                            PrecheckOutcome::SafeMigrated
                        }
                    }
                    Err(_) => PrecheckOutcome::HaltContinuityViolation,
                }
            }
        };

        // Story 5.2 review backfill: drained_count and migrated_count are
        // mutually exclusive per AC7's JSON shape. Populate exactly the field
        // that matches the verdict; the other is None.
        let (drained_count, migrated_count) = match verdict {
            PrecheckOutcome::SafeDrained => (Some(pending_count), None),
            PrecheckOutcome::SafeMigrated => (Some(0), Some(pending_count)),
            PrecheckOutcome::HaltContinuityViolation
            | PrecheckOutcome::SchemaIncompatible
            | PrecheckOutcome::EMigratorMissing => (None, None),
        };

        PrecheckVerdict {
            verdict,
            predecessor_halt_protocol_version,
            successor_accepted_versions: version_list,
            drained_count,
            migrated_count,
            schema_compat,
            auto_revert_window_seconds: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halt::HaltRegistry;
    use maos_domain::halt::{HaltId, HaltState};

    fn make_registry() -> HaltRegistry {
        HaltRegistry::new()
    }

    #[test]
    fn precheck_empty_halt_set_returns_safe_drained() {
        let registry = make_registry();
        let verdict = HotSwapPrecheck::check(&registry, 1, 1, &[1, 2], 1, 1);
        assert_eq!(verdict.verdict, PrecheckOutcome::SafeDrained);
        assert_eq!(verdict.schema_compat, SchemaCompat::SameMajor);
    }

    #[test]
    fn precheck_cross_major_detected() {
        let registry = make_registry();
        let verdict = HotSwapPrecheck::check(&registry, 1, 1, &[1, 2], 0x0001_0000, 0x0002_0000);
        assert_eq!(verdict.schema_compat, SchemaCompat::CrossMajor);
    }

    #[test]
    fn precheck_halt_continuity_violation() {
        let registry = make_registry();
        let hid = HaltId::new("halt-001").unwrap();
        registry.insert_pending(hid, HaltState::PendingResolution);

        let verdict = HotSwapPrecheck::check(
            &registry,
            1,
            1,
            &[/* empty — no compatible versions */],
            1,
            1,
        );
        assert_eq!(verdict.verdict, PrecheckOutcome::HaltContinuityViolation);
    }
}
