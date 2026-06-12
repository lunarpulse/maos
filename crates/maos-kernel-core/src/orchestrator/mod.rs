#![forbid(unsafe_code)]

//! Orchestrator instruction buffer — kernel checkpoint/resume primitive
//! per FR20 + epic-3 Story 3.4 AC.
//!
//! Story 3.4 LANDS: the per-Spirit `OrchestratorBuffer` + `OrchestratorBufferRegistry`,
//! the `enqueue` / `dequeue_at_safe_point` / `recall_all_pending` operations,
//! and the bounded-queue backpressure semantics.
//!
//! **What the kernel does NOT decide:** WHEN a safe sequence point fires.
//! The Orchestrator-class Spirit calls `dequeue_at_safe_point` from its
//! own task-completion handler; the kernel only enforces the queue
//! ordering + bounded capacity. Per §4.0.7 the kernel does not embed an
//! orchestration policy.
//!
//! See `crates/maos-cli/src/cli.rs::OrchestratorOp` for the director-side
//! CLI surface that enqueues into this buffer via `MAOS_ONE_SHOT=orchestrator-queue`.

pub mod buffer;
pub mod echo_gateway;
pub mod gateway_dispatcher;
pub mod registry;

pub use buffer::{OrchestratorBuffer, OrchestratorBufferError};
pub use echo_gateway::{EchoGatewayFactory, EchoGatewaySubmodule};
pub use gateway_dispatcher::{
    GatewayDispatcher, GatewayInstance, GatewaySubmoduleFactory, GatewaySubmoduleRegistry,
};
pub use registry::OrchestratorBufferRegistry;

use crate::iac::transparency_log::TransparencyLogAdapter;
use maos_domain::halt::HaltJournalError;
use maos_domain::invariants::i4::ApprovalDecision;

/// Journal an orchestrator enqueue to the Approval Decision Log (Story 3.4 AC2).
/// Mirrors `crate::halt::journal_halt_resolution` (Story 3.3 AC4) — one canonical
/// shape for director-action audit rows. `actor` is `"director"` at v0.3-β.
pub fn journal_orchestrator_queue(
    log: &TransparencyLogAdapter,
    actor: &str,
    spirit_id: &str,
    instruction: &maos_domain::orchestrator::OrchestratorInstruction,
) -> Result<(), HaltJournalError> {
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: "orchestrator.queue".into(),
        intent: "queue".into(),
        decision: true,
        reasoning: Some(format!(
            "id={}: queued instruction (len={}): {}",
            instruction.id.0,
            instruction.goal.len(),
            instruction.goal,
        )),
    })
    .map_err(|e| HaltJournalError::WriteFailed(e.to_string()))
}

/// Journal a director pause/resume to the Approval Decision Log.
/// `action` is one of `"pause"` / `"resume"` — stable labels.
pub fn journal_director_lifecycle_action(
    log: &TransparencyLogAdapter,
    actor: &str,
    spirit_id: &str,
    action: &str,
) -> Result<(), HaltJournalError> {
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: format!("lifecycle.{action}"),
        intent: action.into(),
        decision: true,
        reasoning: None,
    })
    .map_err(|e| HaltJournalError::WriteFailed(e.to_string()))
}

/// Journal a director-initiated capability token revocation. Per FR42
/// the row carries director identity (`actor = "director"` at v0.3-β)
/// + optional reason. `capability` is the stable label `"token.revoke"`.
pub fn journal_token_revocation(
    log: &TransparencyLogAdapter,
    actor: &str,
    token_id_hex: &str,
    reason: Option<&str>,
) -> Result<(), HaltJournalError> {
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: format!("token:{token_id_hex}"),
        capability: "token.revoke".into(),
        intent: "revoke".into(),
        decision: true,
        reasoning: reason.map(|r| format!("token={token_id_hex}: {r}")),
    })
    .map_err(|e| HaltJournalError::WriteFailed(e.to_string()))
}
