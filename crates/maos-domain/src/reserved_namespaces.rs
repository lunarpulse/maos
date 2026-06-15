//! Story 9.4b AC-7 (re-ratification D7/D8) — reserved namespace identifiers for
//! the v1.5+ multi-operator tenancy seam.
//!
//! Per D7, the generic per-record tenancy reservation was **CUT as theater**:
//! the only load-bearing persisted reservation is `deployment_operator_id`,
//! stamped on the AC-6 provenance governance event (Task 2), not a generic
//! per-record tenant field.
//!
//! Per D8, this namespace reservation is held **OUTSIDE the NFR-Test-11
//! grammar-lock hash** — it is NOT a [`crate::memory::MemoryNamespace`] variant.
//! Adding a `MemoryNamespace` variant is a future story's *intentional* re-roll
//! of the grammar-lock hash; reserving string identifiers here lets v1.5+
//! multi-operator work claim names without touching the hashed surface now.
//!
//! Full multi-operator implementation + the multi-Spirit acceptance demo are
//! deferred to v1.5+ (Epic 10), gated by Story 9.6 — there is no live
//! multi-tenant code path at v1.0.
//!
//! ## CI collision guard (D8)
//!
//! The test below asserts no current `MemoryNamespace` variant label collides
//! with a reserved identifier.  Coupled with the NFR-Test-11 grammar-lock test
//! (which reds on ANY enum change and forces review), this enforces D8's rule:
//! *a future variant addition must remove its name from `RESERVED` in the same
//! diff* — otherwise this guard reds.

/// Reserved namespace identifiers for the v1.5+ multi-operator tenancy seam.
///
/// These are NOT live `MemoryNamespace` variants and carry no runtime behavior
/// at v1.0 — they exist solely to prevent a future variant from silently
/// colliding with a name multi-operator tenancy intends to claim.
pub const RESERVED_NAMESPACE_IDENTIFIERS: &[&str] = &["operator", "tenant"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryNamespace;

    /// D8 CI collision guard — no live `MemoryNamespace` variant label may
    /// collide with a reserved identifier.  RED-able: add a variant whose label
    /// is in `RESERVED` (and forget to remove it) and this fails.
    #[test]
    fn reserved_do_not_collide_with_current_variants() {
        // Current variant labels (kept in lockstep with the closed enum; the
        // NFR-Test-11 grammar-lock test reds on any enum change, forcing this
        // list — and the RESERVED set — to be revisited in the same diff).
        let current: Vec<&'static str> = vec![
            MemoryNamespace::Default.kind_label(),
            MemoryNamespace::Coordination.kind_label(),
            MemoryNamespace::Forgotten.kind_label(),
            MemoryNamespace::principal("p", "s").unwrap().kind_label(),
        ];
        for reserved in RESERVED_NAMESPACE_IDENTIFIERS {
            assert!(
                !current.contains(reserved),
                "reserved identifier {reserved:?} collides with a live MemoryNamespace variant \
                 — per D8 a new variant must be REMOVED from RESERVED in the same diff"
            );
        }
    }

    #[test]
    fn reserved_are_unique_and_nonempty() {
        assert!(!RESERVED_NAMESPACE_IDENTIFIERS.is_empty());
        let mut seen = std::collections::HashSet::new();
        for r in RESERVED_NAMESPACE_IDENTIFIERS {
            assert!(seen.insert(*r), "duplicate reserved identifier {r:?}");
        }
    }
}
