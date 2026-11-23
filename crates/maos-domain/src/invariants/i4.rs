//! I4: Every approval captures intent, not just decision.
//!
//! `(actor, target, capability, intent, decision, reasoning_if_any)` lands
//! in the Approval Decision Log. Audit trail must answer "why did the user
//! approve this?", not just "did they?".
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — Approval Manager writes structured record at
//!   every prompt resolution.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i4::{InvariantI4, ApprovalDecision};
//!
//! let _marker: InvariantI4 = InvariantI4;
//! let decision = ApprovalDecision {
//!     actor: "user-1".into(),
//!     target: "spirit-nash".into(),
//!     capability: "fs.write".into(),
//!     intent: "diagnosis-handoff".into(),
//!     decision: true,
//!     reasoning: Some("Patient data requires update".into()),
//! };
//! assert!(decision.decision);
//! ```

/// I4 marker type — Every approval captures intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI4;

/// Structured approval decision record — the I4 contract at the type level.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalDecision {
    /// Who made the decision (user identifier).
    pub actor: String,
    /// Which Spirit or resource is the target.
    pub target: String,
    /// Which capability was requested.
    pub capability: String,
    /// Typed intent under which the capability was requested.
    pub intent: String,
    /// `true` = approved; `false` = denied.
    pub decision: bool,
    /// Optional human-readable reasoning.
    pub reasoning: Option<String>,
}

/// I4 observation: the approvals that were granted (by target) and the Approval Decision Log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub granted: Vec<String>,
    pub log: Vec<ApprovalDecision>,
}

/// I4 predicate: every log row carries actor, target, capability and intent, and every
/// granted approval has a granting row for its target.
pub fn check(obs: &Observation) -> Result<(), crate::invariants::Violation> {
    use crate::invariants::ensure;
    for row in &obs.log {
        let fields = [&row.actor, &row.target, &row.capability, &row.intent];
        ensure(
            fields.iter().all(|f| !f.trim().is_empty()),
            "I4",
            "approval row missing actor, target, capability or intent",
        )?;
    }
    for target in &obs.granted {
        ensure(
            obs.log.iter().any(|r| r.decision && &r.target == target),
            "I4",
            "granted approval without a decision row",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_decision_construction() {
        let d = ApprovalDecision {
            actor: "a".into(),
            target: "t".into(),
            capability: "c".into(),
            intent: "i".into(),
            decision: false,
            reasoning: None,
        };
        assert!(!d.decision);
    }
}
