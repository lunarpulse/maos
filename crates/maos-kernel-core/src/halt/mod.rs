#![forbid(unsafe_code)]

//! Halt protocol mechanism scaffold.
//!
//! Story 3.3 LANDS: the `HaltResolver` trait + `MockHaltResolver` +
//! resolution-receiving glue. Story 4.1 LANDS: `invoke_halt`,
//! halt-receipt production, `HaltState` lifecycle, I14
//! halt-continuity validation, halt-recall/precision floors.
//!
//! See `crates/maos-director-surface/src/halt_ui.rs` for the
//! director-side resolution submission path.

pub mod resolver;
pub use resolver::{MockHaltResolver, FailingHaltResolver};
pub use maos_domain::halt::{HaltResolver, ResolveError};

use maos_domain::halt::{HaltId, HaltJournal, HaltJournalError, Resolution};
use maos_domain::invariants::i4::ApprovalDecision;
use crate::iac::transparency_log::TransparencyLogAdapter;

/// Journal a halt resolution to the Approval Decision Log (Story 3.3, AC4).
///
/// Mirrors `crate::security::posture::journal_posture_shift` (Story 3.2
/// AC4) — one canonical surface for director-action audit rows. The
/// `actor` is the director identity (`"director"` at v0.3-β; Story 9.x
/// wires identity propagation from the control-plane session).
///
/// Returns `Err(AuditError::SqliteWriteFatal)` if the SQLite write
/// fails AT THE LOG ADAPTER LEVEL — note that `insert_frame_event`
/// PANICS on log write failure per I2 (`transparency_log.rs:296-307`)
/// but `insert_approval_decision` returns a `Result` per the existing
/// signature at `transparency_log.rs:322-345`. The caller (`halt_ui`)
/// MUST surface the error to the director rather than silently dropping.
pub fn journal_halt_resolution(
    log: &TransparencyLogAdapter,
    actor: &str,
    spirit_id: &str,
    halt_id: &HaltId,
    resolution: &Resolution,
) -> Result<(), HaltJournalError> {
    let reasoning = match resolution {
        Resolution::ProvidedContext { text } => Some(format!("halt={}: provided_context: {text}", halt_id.as_str())),
        Resolution::AcceptedHalt => Some(format!("halt={}: accepted_halt", halt_id.as_str())),
        Resolution::AuthorizedOverride { operator_policy_ref } => {
            Some(format!("halt={}: authorized_override: operator_policy_ref={operator_policy_ref}", halt_id.as_str()))
        }
    };
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: "halt.resolve".into(),
        intent: resolution.kind_label().into(),
        decision: true,
        reasoning,
    })
    .map_err(|e| HaltJournalError::WriteFailed(e.to_string()))
}

impl HaltJournal for TransparencyLogAdapter {
    fn journal_halt_resolution(
        &self,
        actor: &str,
        spirit_id: &str,
        halt_id: &HaltId,
        resolution: &Resolution,
    ) -> Result<(), HaltJournalError> {
        journal_halt_resolution(self, actor, spirit_id, halt_id, resolution)
    }
}
