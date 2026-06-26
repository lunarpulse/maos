//! Story 9.4b R1-COND / AC-9 — the single region-enforcement chokepoint for
//! the working-memory store **read** path.
//!
//! Mirror of [`super::write_entry_point`] for the read side.  Under Option A
//! (governance-only, no per-row AEAD), the region-verified adapter path IS the
//! safety case: raw store methods (`read`, `scan`, `lookup`) are
//! `pub(in crate::memory)` and this module's [`enforce_region`] is the
//! mandatory gate on every public read path through the adapter.
//!
//! [`ReadEntryPoint`] is a **non-wildcard** enum: adding a new read path
//! (e.g. a future bulk-export or CDC read) FAILS TO COMPILE until its region
//! provenance is handled (AC-9 forward-coupling).

use maos_domain::region::{Region, RegionError};

/// Every store-read path is exactly one of these variants.
///
/// **No wildcard match arm may ever be written against this enum** — that is
/// the AC-9 type-system guard on the read side (R1-COND).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadEntryPoint {
    /// First-party local read — the reader's context is the home jurisdiction
    /// by construction, so its region provenance IS the configured home region.
    DirectRead,
    /// Read applied while replaying an audit bundle (ADR-028).  Carries the
    /// source bundle's region provenance.  **No live path at v1.0** — reserved
    /// + guarded so a replay-read path cannot land without a region decision.
    ReplayRead { source_region: Option<Region> },
    /// Read applied while verifying a region-bound backup (Story 9.4).
    /// Carries the backup's region provenance.  **No live path at v1.0** —
    /// reserved + guarded.
    BackupVerify { source_region: Option<Region> },
}

impl ReadEntryPoint {
    /// The region provenance of the context that drives this read.
    ///
    /// **This match has NO wildcard arm (AC-9).** A new variant forces a
    /// compile error here until its provenance is declared.
    fn source_region<'a>(&'a self, home: Option<&'a Region>) -> Option<&'a Region> {
        match self {
            ReadEntryPoint::DirectRead => home,
            ReadEntryPoint::ReplayRead { source_region } => source_region.as_ref(),
            ReadEntryPoint::BackupVerify { source_region } => source_region.as_ref(),
        }
    }
}

/// Fail-closed region enforcement at the single read chokepoint (R1-COND / AC-9).
///
/// Semantics mirror [`super::write_entry_point::enforce_region`]:
/// * `home == None` → region pinning **disabled** → always `Ok`.
/// * data region == home → `Ok`.
/// * data region != home → [`RegionError::ERegionViolation`] (fail-closed).
/// * foreign-origin read with **no** region tag under a pinned home →
///   `ERegionViolation`.
pub fn enforce_region(entry: &ReadEntryPoint, home: Option<&Region>) -> Result<(), RegionError> {
    let Some(home) = home else {
        return Ok(()); // region pinning disabled
    };
    match entry.source_region(Some(home)) {
        Some(src) if src == home => Ok(()),
        Some(src) => Err(RegionError::ERegionViolation {
            expected: home.as_str().to_string(),
            found: src.as_str().to_string(),
            detail: "cross-region store read rejected at ReadEntryPoint",
        }),
        None => Err(RegionError::ERegionViolation {
            expected: home.as_str().to_string(),
            found: "<unverifiable>".to_string(),
            detail: "untagged foreign-origin read rejected under region pinning",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore};

    /// R1-COND proof: the `ReadEntryPoint` enum is exhaustive (no wildcard),
    /// and every variant is handled with an explicit region-provenance decision.
    /// Adding a new variant will fail compilation here AND in `source_region`.
    #[test]
    fn read_entry_point_enum_is_exhaustive() {
        let home = Region::canonicalize("eu-west-1").unwrap();
        let cases: Vec<(ReadEntryPoint, bool)> = vec![
            // DirectRead — home by construction → Ok
            (ReadEntryPoint::DirectRead, true),
            // ReplayRead same region → Ok
            (
                ReadEntryPoint::ReplayRead {
                    source_region: Some(home.clone()),
                },
                true,
            ),
            // ReplayRead different region → Err
            (
                ReadEntryPoint::ReplayRead {
                    source_region: Some(Region::canonicalize("us-east-1").unwrap()),
                },
                false,
            ),
            // ReplayRead no region → Err
            (
                ReadEntryPoint::ReplayRead {
                    source_region: None,
                },
                false,
            ),
            // BackupVerify same region → Ok
            (
                ReadEntryPoint::BackupVerify {
                    source_region: Some(home.clone()),
                },
                true,
            ),
            // BackupVerify different region → Err
            (
                ReadEntryPoint::BackupVerify {
                    source_region: Some(Region::canonicalize("ap-south-1").unwrap()),
                },
                false,
            ),
            // BackupVerify no region → Err
            (
                ReadEntryPoint::BackupVerify {
                    source_region: None,
                },
                false,
            ),
        ];
        for (entry, expect_ok) in &cases {
            let result = enforce_region(entry, Some(&home));
            assert_eq!(
                result.is_ok(),
                *expect_ok,
                "entry {entry:?} expected ok={expect_ok}, got {result:?}"
            );
        }
    }

    /// R1-COND proof: when region pinning is disabled (home == None) every
    /// read path succeeds unconditionally.
    #[test]
    fn read_enforcement_disabled_when_no_home() {
        let entries = vec![
            ReadEntryPoint::DirectRead,
            ReadEntryPoint::ReplayRead {
                source_region: Some(Region::canonicalize("eu-west-1").unwrap()),
            },
            ReadEntryPoint::BackupVerify {
                source_region: None,
            },
        ];
        for entry in &entries {
            assert!(
                enforce_region(entry, None).is_ok(),
                "pinning disabled should always succeed: {entry:?}"
            );
        }
    }

    /// R1-COND / AC-9 proof: enumerate every raw store read method across all 3
    /// backends and prove each is `pub(in crate::memory)` (i.e., unreachable from
    /// outside the memory module).
    ///
    /// The raw store read methods are:
    ///   - `PrivateMemoryStore::read`   — `pub(in crate::memory)`
    ///   - `PrivateMemoryStore::scan`   — `pub(in crate::memory)`
    ///   - `SharedMemoryStore::read`    — `pub(in crate::memory)`
    ///   - `SharedMemoryStore::scan`    — `pub(in crate::memory)`
    ///   - `PrincipalNamespaceIndex::lookup` — `pub(in crate::memory)`
    ///
    /// The **only** public read surface is `MemoryManagerAdapter` (via the
    /// `MemoryManagerPort` trait impl), which routes every read through this
    /// module's `enforce_region` chokepoint.
    ///
    /// This test is a *compile-time* proof by construction: it calls every raw
    /// store read method, proving they are reachable from within the memory
    /// module.  If any of these methods were accidentally promoted to `pub`
    /// (unrestricted), the `xtask bypass-scan` gate would catch it.  If a NEW
    /// read method is added to any store without being routed through the
    /// adapter, the bypass-scan gate's exhaustive public-surface check will
    /// fail.
    #[test]
    fn raw_store_read_methods_callable_from_memory_module() {
        // This test lives inside the `memory` module, so `pub(in crate::memory)`
        // methods are reachable.  The test proves they exist and are correctly
        // scoped.  We don't need to run real I/O — just verify the methods
        // resolve at compile time.
        //
        // We use function-pointer coercions: if the signature changes or the
        // method is removed/renamed, this fails to compile.

        // PrivateMemoryStore read methods
        let _: fn(
            &PrivateMemoryStore,
            u32,
            &maos_domain::memory::MemoryNamespace,
            &str,
        ) -> Result<
            Option<maos_domain::memory::MemoryValue>,
            maos_domain::memory::MemoryError,
        > = PrivateMemoryStore::read;

        let _: fn(
            &PrivateMemoryStore,
            u32,
            &maos_domain::memory::MemoryNamespace,
            &str,
            usize,
        ) -> Result<
            Vec<maos_domain::memory::MemoryEntry>,
            maos_domain::memory::MemoryError,
        > = PrivateMemoryStore::scan;

        // SharedMemoryStore read methods
        let _: fn(
            &SharedMemoryStore,
            u32,
            &maos_domain::memory::MemoryNamespace,
            &str,
        ) -> Result<
            Option<maos_domain::memory::MemoryValue>,
            maos_domain::memory::MemoryError,
        > = SharedMemoryStore::read;

        let _: fn(
            &SharedMemoryStore,
            u32,
            &maos_domain::memory::MemoryNamespace,
            &str,
            usize,
        ) -> Result<
            Vec<maos_domain::memory::MemoryEntry>,
            maos_domain::memory::MemoryError,
        > = SharedMemoryStore::scan;

        // PrincipalNamespaceIndex lookup method
        let _: fn(
            &PrincipalNamespaceIndex,
            &str,
        ) -> Result<
            Vec<maos_domain::memory::PrincipalIndexRow>,
            maos_domain::memory::MemoryError,
        > = PrincipalNamespaceIndex::lookup;
    }
}
