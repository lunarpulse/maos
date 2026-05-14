//! Security Manager port trait per architecture §4.3.
//!
//! Enforces sandbox tiers, secret isolation, and approval-class
//! mediation. Story 1b.3 lands the T0/T1/T2 tier enforcement.

use crate::invariants::i4::ApprovalDecision;
use crate::invariants::i9::SandboxTier;

/// Security Manager — sandbox, secret, and approval mediation.
///
/// Per §4.3: "The Security Manager enforces the sandbox tier floor
/// for every Spirit before it is allowed to execute external calls."
pub trait SecurityManagerPort {
    /// Class: supervision
    ///
    /// Returns the minimum sandbox tier (`SandboxTier`) that a Spirit
    /// must satisfy before external calls are permitted. At v0.1-α
    /// this is a structural placeholder; Story 1b.3 lands T0–T2
    /// enforcement with per-Spirit resource caps.
    fn sandbox_tier_floor(&self, spirit_id: &str) -> SandboxTier;

    /// Class: supervision
    ///
    /// Returns the effective sandbox tier for a given Spirit pid,
    /// or `None` if the Spirit is not known.
    fn effective_sandbox_tier(&self, spirit_pid: u32) -> Option<SandboxTier>;

    /// Class: supervision
    ///
    /// Returns the approval class for a given capability request.
    /// The approval class determines whether human-in-the-loop
    /// approval is required, auto-approved, or denied.
    fn approval_class(&self, capability: &str) -> ApprovalDecision;
}
