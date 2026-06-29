//! Story 9.4b AC-5 / AC-9 — the single region-enforcement chokepoint for the
//! working-memory store (re-ratification R1, Option A).
//!
//! Under Option A memory rows are **plaintext** and region-bound by *governance*
//! (region-verified audit frames + region-pinned read/write paths), NOT by
//! per-row encryption (the at-rest-confidentiality residual is recorded in the
//! story's Honest Risk Register; per-row crypto is deferred to 9.4c).  So the
//! enforcement here is the **sole safety case** for the memory layer — there is
//! no cryptographic backstop the way an AEAD-at-rest design would have (Murat).
//!
//! Therefore [`WriteEntryPoint`] is a **non-wildcard** enum and [`enforce_region`]
//! matches it with **no `_ =>` arm**: adding a new write path (e.g. a future 9.6
//! `ScheduledWrite`) FAILS TO COMPILE until its region provenance is handled
//! (AC-9 forward-coupling). The `xtask bypass-scan` gate is the runtime
//! companion — it reds if a raw store-write becomes reachable without routing
//! through this module.

use maos_domain::region::{Region, RegionError};

/// Every store-write path is exactly one of these variants.
///
/// **No wildcard match arm may ever be written against this enum** — that is the
/// AC-9 type-system guard (R-RG3).  New variants are reserved ahead of their
/// live paths precisely so the path cannot land un-enforced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteEntryPoint {
    /// First-party local write — originates in the home jurisdiction by
    /// construction, so its region provenance IS the configured home region.
    DirectWrite,
    /// Write applied from a replayed audit bundle (ADR-028).  Carries the source
    /// bundle's region provenance.  **No live path at v1.0** — reserved + guarded
    /// so a replay-to-store path cannot land without a region decision.
    ReplayApply {
        /// The region the replayed bundle was signed under (`None` = untagged).
        source_region: Option<Region>,
    },
    /// Write applied while restoring region-bound data from backup (Story 9.4).
    /// Carries the backup's region provenance.  **No live path at v1.0** —
    /// reserved + guarded.
    BackupRestore {
        /// The region the backup artifact was bound to (`None` = untagged).
        source_region: Option<Region>,
    },
}

impl WriteEntryPoint {
    /// The region provenance of the data this entry point is about to write.
    ///
    /// **This match has NO wildcard arm (AC-9).** A new variant forces a
    /// compile error here until its provenance is declared.
    fn source_region<'a>(&'a self, home: Option<&'a Region>) -> Option<&'a Region> {
        match self {
            // Local origin == home jurisdiction by construction.
            WriteEntryPoint::DirectWrite => home,
            WriteEntryPoint::ReplayApply { source_region } => source_region.as_ref(),
            WriteEntryPoint::BackupRestore { source_region } => source_region.as_ref(),
        }
    }
}

/// Fail-closed region enforcement at the single write chokepoint (AC-5 / AC-9).
///
/// * `home == None` → region pinning **disabled** → always `Ok` (legacy /
///   default-region semantics, AC-11).
/// * data region == home → `Ok`.
/// * data region != home → [`RegionError::ERegionViolation`] (fail-closed).
/// * foreign-origin data with **no** region tag under a pinned home →
///   `ERegionViolation` (untagged cross-region data is rejected, not trusted).
pub fn enforce_region(entry: &WriteEntryPoint, home: Option<&Region>) -> Result<(), RegionError> {
    let Some(home) = home else {
        return Ok(()); // region pinning disabled
    };
    match entry.source_region(Some(home)) {
        Some(src) if src == home => Ok(()),
        Some(src) => Err(RegionError::ERegionViolation {
            expected: home.as_str().to_string(),
            found: src.as_str().to_string(),
            detail: "cross-region store write rejected at WriteEntryPoint",
        }),
        None => Err(RegionError::ERegionViolation {
            expected: home.as_str().to_string(),
            found: "<unverifiable>".to_string(),
            detail: "untagged foreign-origin write rejected under region pinning",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(tag: &str) -> Region {
        Region::canonicalize(tag).unwrap()
    }

    /// R-RG1 (MERGE-BLOCKING) — same-input-opposite-verdict: identical write
    /// shapes differing only by region tag → home ALLOW, foreign VIOLATION.
    #[test]
    fn r_rg1_same_input_opposite_verdict() {
        let home = region("eu");
        let foreign = region("us");

        let home_replay = WriteEntryPoint::ReplayApply {
            source_region: Some(home.clone()),
        };
        let foreign_replay = WriteEntryPoint::ReplayApply {
            source_region: Some(foreign.clone()),
        };

        assert!(enforce_region(&home_replay, Some(&home)).is_ok());
        assert!(matches!(
            enforce_region(&foreign_replay, Some(&home)),
            Err(RegionError::ERegionViolation { .. })
        ));
    }

    /// R-RG2 (MERGE-BLOCKING, anti-stub) — parametrized over ALL non-home write
    /// entry points: this test DIES if `enforce_region` is replaced with a
    /// fail-OPEN stub (`Ok(())`).  Murat's line in the sand.
    #[test]
    fn r_rg2_anti_stub_all_entry_points_fail_closed() {
        let home = region("eu");
        let foreign = region("us");

        // Every foreign / untagged entry point MUST be rejected.
        let must_reject = [
            WriteEntryPoint::ReplayApply {
                source_region: Some(foreign.clone()),
            },
            WriteEntryPoint::BackupRestore {
                source_region: Some(foreign.clone()),
            },
            WriteEntryPoint::ReplayApply {
                source_region: None,
            },
            WriteEntryPoint::BackupRestore {
                source_region: None,
            },
        ];
        for ep in &must_reject {
            assert!(
                enforce_region(ep, Some(&home)).is_err(),
                "fail-OPEN regression: {ep:?} was not rejected (a `true` stub would pass this)"
            );
        }

        // DirectWrite is home-by-construction → allowed under pinning.
        assert!(enforce_region(&WriteEntryPoint::DirectWrite, Some(&home)).is_ok());
        // The matching-region replay is allowed (proves the guard isn't a
        // reject-everything stub either).
        assert!(enforce_region(
            &WriteEntryPoint::ReplayApply {
                source_region: Some(home.clone())
            },
            Some(&home)
        )
        .is_ok());
    }

    /// R-RG3 / AC-9 — exhaustiveness: every variant yields a defined verdict
    /// (the compile-time proof is the non-wildcard match; this asserts runtime
    /// totality + that pinning-disabled is uniformly permissive).  If a future
    /// variant is added, `source_region`'s match fails to compile — which is the
    /// real guard; this test additionally exercises each known variant.
    #[test]
    fn r_rg3_exhaustive_defined_verdict_per_variant() {
        let home = region("eu");
        let all = [
            WriteEntryPoint::DirectWrite,
            WriteEntryPoint::ReplayApply {
                source_region: Some(home.clone()),
            },
            WriteEntryPoint::ReplayApply {
                source_region: None,
            },
            WriteEntryPoint::BackupRestore {
                source_region: Some(home.clone()),
            },
            WriteEntryPoint::BackupRestore {
                source_region: None,
            },
        ];
        for ep in &all {
            // Disabled pinning → every variant is permitted (legacy semantics).
            assert!(
                enforce_region(ep, None).is_ok(),
                "disabled pinning must allow {ep:?}"
            );
        }
    }

    #[test]
    fn disabled_pinning_allows_foreign() {
        let foreign = region("us");
        let ep = WriteEntryPoint::ReplayApply {
            source_region: Some(foreign),
        };
        assert!(enforce_region(&ep, None).is_ok());
    }
}
