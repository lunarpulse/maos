#![forbid(unsafe_code)]

//! Story 6.2 AC2 — FR21: Orchestrator dispatches against distillates, not raw output.
//!
//! Runtime gate invoked from `IacBusAdapter::deliver_typed` BEFORE the I13
//! lineage check and AFTER the kind-routing check. Closes the raw-output
//! context-overflow loophole described in FR21:
//!
//! > Orchestrator dispatches subsequent tasks against the distillate of prior
//! > Worker output, not the raw output.
//!
//! ### When the check fires
//!
//! 1. `frame.kind == FrameKind::TaskAssign`
//! 2. `frame.from.role == Some(SpiritRole::Orchestrator)`
//! 3. A `FrameKind::TaskComplete` frame from a Worker is present in the
//!    Transparency Log within the last `window_ns` (default 60s — v0.5 floor).
//! 4. `task_assign.prior_distillate_ref == None`
//!    *or* `prior_distillate_ref` resolves to a frame that is NOT a
//!    `FrameKind::Distillate` row.
//!
//! ### Effect
//!
//! Returns `IacBusError::EOrchestratorDispatchRawOutput`. The bus REJECTS the
//! frame and the Transparency Log row is NOT written — this is a permission
//! check, distinct from the I13 lineage check which fires AFTER log-before-deliver.

use std::sync::Arc;

use maos_domain::frame::{FramePayload, IacFrame};
use maos_domain::iac_bus_types::IacBusError;
use maos_spirit_abi::identity::{FrameKind, SpiritRole};

use super::transparency_log::{FrameFilter, FrameKind as TlFrameKind, TransparencyLogAdapter};

/// Default Orchestrator-dispatch follow-up window. Operator-configurable via
/// `MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS` (composition root surfaces it on
/// the daemon). 60s per AC2 spec at v0.5-α; tightened in v0.8 per the §13.1
/// runner-tier calibration window.
pub const DEFAULT_ORCHESTRATOR_DISPATCH_WINDOW_NS: u64 = 60_000_000_000;

/// Returns the current monotonic-wall-clock-adjacent nanosecond timestamp
/// for window comparisons. We use `SystemTime` because TL rows record
/// `timestamp_ns` derived from the same epoch (see `transparency_log.rs`).
fn now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// FR21 runtime gate — see module docs.
///
/// Returns `Ok(())` when:
///   * The frame is NOT an Orchestrator-emitted `TaskAssign`, OR
///   * No prior `TaskComplete` exists in the window (this is the first
///     dispatch in a fan-out), OR
///   * `prior_distillate_ref` is `Some(ref)` AND `ref.digest_frame_id`
///     resolves to a `FrameKind::Distillate` row in the TL.
///
/// Returns `Err(EOrchestratorDispatchRawOutput)` otherwise.
pub(crate) fn check_orchestrator_distillate_required(
    frame: &IacFrame,
    transparency_log: &Arc<TransparencyLogAdapter>,
    window_ns: u64,
) -> Result<(), IacBusError> {
    // 1. Only Orchestrator-emitted TaskAssign frames are gated.
    if frame.kind != FrameKind::TaskAssign {
        return Ok(());
    }
    if frame.from.role != Some(SpiritRole::Orchestrator) {
        return Ok(());
    }

    // Extract goal/task_id for the error message — best-effort; goal serves
    // as a stable task identifier at v0.5-α (full TaskId surface is Story 7.x).
    let (prior_ref, goal) = match &frame.payload {
        FramePayload::TaskAssign(p) => (p.prior_distillate_ref.clone(), p.goal.clone()),
        _ => return Ok(()), // payload mismatch — let the type system handle it
    };

    // 2. Look for any TaskComplete frames in the window from any Worker.
    let now = now_ns();
    // Fail-open when the system clock is untrustworthy (pre-epoch or broken):
    // a `now` of 0 saturates to 0 for `since`, which would match ALL frames
    // since epoch — incorrectly rejecting legitimate first dispatches.
    if now == 0 {
        return Ok(());
    }
    let since = now.saturating_sub(window_ns);
    let filter = FrameFilter {
        kind: Some(TlFrameKind::TaskComplete),
        since_ns: Some(since),
        until_ns: Some(now),
        limit: Some(1), // we only need to know whether ANY exist
        ..Default::default()
    };
    let prior_completions = transparency_log
        .query_frames(filter)
        .map_err(|e| IacBusError::SerializationFailed(e.to_string()))?;

    // No predecessor — first dispatch in a fan-out. Accept regardless of
    // `prior_distillate_ref`.
    if prior_completions.is_empty() {
        return Ok(());
    }

    // 3. Predecessor present — `prior_distillate_ref` MUST be Some(ref) and
    //    ref MUST resolve to a `FrameKind::Distillate` row.
    let Some(ref_) = prior_ref else {
        return Err(IacBusError::EOrchestratorDispatchRawOutput {
            orchestrator: frame.from.spirit_id.as_str().to_string(),
            task_id: goal,
        });
    };

    // Lookup the referenced row and assert it is Distillate.
    let referenced = transparency_log
        .query_frame_by_id(ref_.digest_frame_id)
        .map_err(|e| IacBusError::SerializationFailed(e.to_string()))?;
    let referenced = match referenced {
        Some(row) if matches!(row.kind, TlFrameKind::Distillate) => row,
        _ => {
            return Err(IacBusError::EOrchestratorDispatchRawOutput {
                orchestrator: frame.from.spirit_id.as_str().to_string(),
                task_id: goal,
            });
        }
    };

    // Basic depth sanity: distillation_depth must be > 0 for a valid
    // distillate. Full cross-check of depth + intent_lineage against the
    // DistillationReceipt payload is deferred to Story 6.5/7.x when the
    // DistillateWriter deserialization surface stabilizes (currently the
    // TL stores opaque blobs; schema-versioned payload parse requires
    // registry-side type resolution not yet plumbed into kernel-core).
    if ref_.distillation_depth == 0 {
        return Err(IacBusError::EOrchestratorDispatchRawOutput {
            orchestrator: frame.from.spirit_id.as_str().to_string(),
            task_id: goal,
        });
    }

    Ok(())
}
