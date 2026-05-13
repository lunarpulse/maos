//! I1: Spirits cannot bypass the Capability Registry.
//!
//! Every tool, network call, file op, or sub-Spirit spawn from a Spirit
//! MUST flow through the Capability Registry. The kernel's only API
//! surface returned to a Spirit at load-time is the typed capability
//! mediation layer; there is no Spirit-visible short-circuit.
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — Capability Registry mediation is the only
//!   public function path returning side-effects to Spirits.
//! - **v0.3 / v0.5 / v0.9**: `runtime` (unchanged).
//! - **v1.0 / v1.5**: `fuzz` — the 80-scenario red-team corpus (NFR-Sec-10)
//!   beats on capability-confusion paths.
//!
//! # Invariant statement (doctest)
//!
//! The marker type below codifies I1 at the type level. Calling it requires
//! a `CapabilityToken`; a Spirit cannot construct a `CapabilityToken`
//! outside the registry. This is the type-level expression of the I1 contract.
//!
//! ```
//! use maos_domain::invariants::i1::{InvariantI1, CapabilityToken};
//!
//! // The marker type exists and is the contract anchor for I1.
//! let _marker: InvariantI1 = InvariantI1;
//!
//! // Capability tokens are private-constructor — no Spirit-visible `new`
//! // function exists at the domain layer. The kernel's
//! // `cap_tokens::issue(spirit_id, scope)` is the ONLY constructor.
//! // (Trying to construct one here would fail to compile by design;
//! // the doctest documents the contract, it does NOT exercise a violation.)
//! # let _ = std::mem::size_of::<CapabilityToken>();  // proves the type exists
//! ```

/// I1 marker type — Spirits cannot bypass the Capability Registry.
///
/// This zero-size type exists to anchor I1 in the type system. Its
/// presence in a function signature documents that the function operates
/// under the I1 contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI1;

/// Capability token — short-lived authorization to invoke a specific
/// Capability with specific arguments under a specific posture (per §3.1
/// vocabulary + ADR-023).
///
/// Constructor is private-to-the-crate at v0.1-α (the actual kernel-side
/// issuance lands in Story 1b.2 inside `maos-kernel-core::capability::cap_tokens`;
/// this `maos-domain` type is the wire-stable shape Spirits see).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CapabilityToken {
    // Fields are crate-private at v0.1-α; Story 1a.3 expands when
    // CryptoProvider lands. The structure exists at v0.1-α to nail down
    // the type identity for ABI continuity.
    _placeholder: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_token_exists() {
        let t = CapabilityToken { _placeholder: () };
        assert_eq!(t._placeholder, ());
    }

    #[test]
    fn invariant_i1_marker_is_zst() {
        assert_eq!(std::mem::size_of::<InvariantI1>(), 0);
    }
}
