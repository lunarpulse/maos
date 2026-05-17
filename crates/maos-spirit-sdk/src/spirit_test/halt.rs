#![forbid(unsafe_code)]

//! Halt resolution simulator — forward-anchor for Story 4.1.
//!
//! Architecture §6.3 + Epic 3 line 92 commit to the 3 resolution kinds:
//! `provided_context` (operator supplied additional context the Spirit
//! should consult before continuing), `accepted_halt` (operator agreed
//! with the halt; Spirit unloads), `authorized_override` (operator
//! authorized the action despite the halt; Spirit continues with an
//! override marker added to subsequent output for `output_shape`
//! predicates).
//!
//! Story 2.4 ships the ENUM SHAPE as the forward-anchor contract.
//! Story 4.1 ships the runtime mechanism (HaltResolver trait + 99.9%
//! HaltReceipt + I14 hot-swap halt-continuity enforcement).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaltResolutionKind {
    /// Operator supplied additional context — Spirit should consult
    /// the context bytes before continuing.
    ProvidedContext { context_bytes: Vec<u8> },
    /// Operator agreed with the halt — Spirit unloads.
    AcceptedHalt,
    /// Operator authorized the action despite the halt — Spirit
    /// continues with the override marker added to subsequent output.
    AuthorizedOverride { override_marker: Vec<u8> },
}

/// Record of a halt resolution the simulator surfaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaltResolutionRecord {
    pub halt_id: String,
    pub kind: HaltResolutionKind,
}
