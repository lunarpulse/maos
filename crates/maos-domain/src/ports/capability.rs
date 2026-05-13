//! Capability Registry port trait per architecture §4.6 + ADR-030 decomposition.
//!
//! The Capability Registry mediates every external call. At v0.1-α this
//! port declares the four ADR-022 universal-arithmetic predicates — the
//! kernel-side surface that fires epistemic halts when a Spirit's
//! tagged-scalar slot crosses a threshold. The full
//! issue/verify/revoke/audit-write surface lands in Story 1b.2; the
//! mailbox-side `iac.send` mediation lands in Story 6.1.

/// Capability Registry — mediates every external call, evaluates ADR-022
/// universal-arithmetic predicates against per-Spirit tagged scalars.
///
/// Per ADR-030: split internally into `cap_tokens` (hot path),
/// `cap_policy`, `cap_audit`, `cap_quota`. The port trait surface at v0.1-α
/// declares only the universal-arithmetic predicates; the cap-tokens
/// hot-path surface (issue/verify/revoke) lands in Story 1b.2.
pub trait CapabilityRegistryPort {
    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value > threshold`. One of the four ADR-022
    /// predicates — the ENTIRE kernel-side computational surface at v0.1.
    /// Spirit-side `[epistemic_policy]` rules reference this predicate
    /// for halt-on-scalar-drift policies (architecture §4.6.1).
    fn on_value_above(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < threshold`. ADR-022 predicate.
    fn on_value_below(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `lower <= value <= upper`. ADR-022 predicate;
    /// inclusive bounds at v0.1-α (open/half-open variants deferred to
    /// Story 4.2 if a Spirit class demands them).
    fn on_value_within(&self, value: f64, lower: f64, upper: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < lower OR value > upper`. ADR-022 predicate.
    fn on_value_outside(&self, value: f64, lower: f64, upper: f64) -> bool;
}
