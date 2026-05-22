#![forbid(unsafe_code)]

//! FR50 dead-Spirit task disposition policy enforcement.
//!
//! Story 5.3 — AC5.

use std::sync::Arc;

use maos_domain::supervision::{DispositionOutcome, OnCrashAction, ReplicaResolver};

/// Enforce the `[on_crash]` disposition policy for a crashed Spirit.
///
/// v0.3-β: `ReassignToReplica` falls through to escalation when no replica
/// is available (`NullReplicaResolver`). Multi-instance hosting lands at
/// Story 6.1 + 8.4.
pub fn enforce_disposition(
    action: OnCrashAction,
    drained_tasks: &[maos_domain::ports::task::TaskAssignmentRecord],
    iac: &Arc<crate::iac::IacBusAdapter>,
    replica: Option<&dyn ReplicaResolver>,
) -> DispositionOutcome {
    let total = drained_tasks.len();
    let mut nacked = 0usize;
    let mut reassigned = 0usize;
    let mut escalated = 0usize;
    let mut reassignment_failed = 0usize;

    match action {
        OnCrashAction::Nack => {
            for task in drained_tasks {
                iac.emit_task_complete_nack(task);
                nacked += 1;
            }
        }
        OnCrashAction::ReassignToReplica => {
            for task in drained_tasks {
                let intent_str = task.intent_class.as_str();
                let target = replica.and_then(|r| r.find_replica(intent_str));
                if let Some(_replica_pid) = target {
                    // v0.3-β: reassign task to replica
                    iac.reassign_task_to(task, &task.originator_spirit_id);
                    reassigned += 1;
                } else {
                    // v0.3-β — no replica available; fall through to escalation
                    // per Story 5.3 forward-compat note.
                    iac.emit_task_complete_escalated(task);
                    escalated += 1;
                    reassignment_failed += 1;
                }
            }
        }
        OnCrashAction::EscalateToOperator => {
            for task in drained_tasks {
                iac.emit_task_complete_escalated(task);
                escalated += 1;
            }
        }
        #[allow(unreachable_patterns)]
        _ => {}
    }

    DispositionOutcome {
        nacked,
        reassigned,
        escalated,
        reassignment_failed,
    }
}
