//! Story 22-8 (D-22-8-05 A) — an unresolved halt survives a restart of the standing cast.
//!
//! Every Spirit termination writes a halt receipt (NFR-Rel-11), and the pending-halt registry is
//! in memory, so after any restart `maosctl halt resolve` answered `halt_not_pending` for a
//! decision the operator never made. Once per boot, after the cast is admitted, [`carry_over`]
//! reads the halts a cast member's admitted identity raised that no `halt.resolve` decision
//! ever closed (`maos_audit::halts`, across sealed segments and the live log), journals one
//! `halt.carried` row for each, and re-admits it as `PendingResolution` at the member's new pid
//! under the same halt id. The old incarnation's receipt stays: the incarnation ended, the
//! decision did not. Resolution stays what it was: through the operator door only.

use maos_audit::halts::{carry_correlation, CARRIED_INTENT};
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState};
use maos_kernel_core::halt::{HaltRegistry, PendingHaltMetadata};
use maos_kernel_core::iac::transparency_log::FrameKind;
use maos_kernel_core::iac::TransparencyLogAdapter;
use std::path::Path;

/// One halt re-admitted at a new incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    pub halt_id: String,
    pub spirit_id: String,
    pub from_boot: u64,
    pub from_pid: u32,
    pub to_pid: u32,
}

/// What a carry-over did, and every reason it may be incomplete.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CarryReport {
    pub carried: Vec<Carried>,
    pub gaps: Vec<String>,
}

/// Carry the unresolved halts of every `(spirit id, pid)` in `cast` into `registry`.
pub fn carry_over(
    audit_db_path: &Path,
    log: &TransparencyLogAdapter,
    registry: &HaltRegistry,
    cast: &[(String, u32)],
    boot_nonce: u64,
) -> Result<CarryReport, String> {
    let ids: Vec<&str> = cast.iter().map(|(id, _)| id.as_str()).collect();
    let (unresolved, gaps) = maos_audit::halts::unresolved_halts(audit_db_path, &ids, boot_nonce)?;
    let mut report = CarryReport {
        carried: Vec::new(),
        gaps,
    };
    for halt in unresolved {
        let Some((spirit_id, to_pid)) = cast.iter().find(|(id, _)| *id == halt.spirit_id) else {
            continue;
        };
        let carried = Carried {
            halt_id: halt.payload.halt_id.clone(),
            spirit_id: spirit_id.clone(),
            from_boot: halt.boot,
            from_pid: halt.pid,
            to_pid: *to_pid,
        };
        match admit(log, registry, &carried, halt.payload, boot_nonce) {
            Ok(true) => report.carried.push(carried),
            Ok(false) => {}
            Err(gap) => report.gaps.push(gap),
        }
    }
    Ok(report)
}

/// Journal the carry row, then re-admit the halt as pending at the new pid. `false`: the
/// registry already holds the halt id, so nothing was written.
fn admit(
    log: &TransparencyLogAdapter,
    registry: &HaltRegistry,
    carried: &Carried,
    payload: EpistemicHaltPayload,
    boot_nonce: u64,
) -> Result<bool, String> {
    let halt_id = HaltId::new(carried.halt_id.clone()).map_err(|e| e.to_string())?;
    if registry.lookup_state(&halt_id).is_some() {
        return Ok(false);
    }
    // A kind-3 (EpistemicHalt) row at the new pid carrying the original halt payload plus the
    // carry fields: `maosctl halt list` reads it as the raised halt it is, and the uninstall
    // scan closes it like any other raise.
    let mut row = serde_json::to_value(&payload).map_err(|e| e.to_string())?;
    row["spirit_id"] = carried.spirit_id.clone().into();
    row["from_boot"] = carried.from_boot.into();
    row["from_pid"] = carried.from_pid.into();
    row["to_boot"] = boot_nonce.into();
    row["to_pid"] = carried.to_pid.into();
    let _ = log
        .insert_frame_event_with_correlation(
            FrameKind::EpistemicHalt,
            carried.to_pid,
            None,
            &carry_correlation(&carried.halt_id),
            CARRIED_INTENT,
            row.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        )
        .into_inner();
    registry
        .insert_pending_with_metadata(
            halt_id,
            HaltState::PendingResolution,
            PendingHaltMetadata {
                spirit_pid: carried.to_pid,
                spirit_id: carried.spirit_id.clone(),
                payload,
                fired_ns: 0,
            },
        )
        .map(|()| true)
        .map_err(|e| format!("halt {} not re-admitted: {e}", carried.halt_id))
}
