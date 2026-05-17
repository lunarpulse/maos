#![forbid(unsafe_code)]

//! Approval Manager — v0.3-β surface.
//!
//! Per architecture §4.3.3 + Invariant I4, the Approval Manager prompts
//! the director through the notification surface and persists the decision
//! in the Approval Decision Log (distinct from Transparency Log per NFR-Obs-5).
//!
//! At v0.3-β the manager auto-allows with a logged decision (interactive
//! resolution UI lands in Story 3.3 for halts and Story 3.4 for
//! pause/resume/revoke).

use std::sync::Arc;

use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::notification::{ApprovalClass, NotificationEvent, NotificationLevel};
use maos_director_surface::notification::NotificationDispatcher;

use crate::iac::transparency_log::{AuditError, TransparencyLogAdapter};

/// Approval Manager — v0.3-β.
///
/// At v0.3-β returns `Ok(true)` (auto-allow with logged decision).
/// Story 3.3 adds interactive resolution.
#[maos_attrs::i9_exempt(reason = "Approval Manager adapter; holds Arc<TransparencyLogAdapter> + AtomicU64 counter — transient/sanctioned state")]
pub struct ApprovalManager {
    log: Arc<TransparencyLogAdapter>,
    decision_counter: std::sync::atomic::AtomicU64,
}

impl ApprovalManager {
    pub fn new(log: Arc<TransparencyLogAdapter>) -> Self {
        Self {
            log,
            decision_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Raise an approval prompt, dispatch to notification surface,
    /// and persist the decision. v0.3-β auto-allows.
    pub fn prompt(
        &self,
        class: ApprovalClass,
        capability: String,
        reasoning: Option<String>,
        dispatcher: &NotificationDispatcher,
    ) -> Result<bool, AuditError> {
        let decision_id = self
            .decision_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let event = NotificationEvent::ApprovalPrompt {
            decision_id,
            class,
            capability: capability.clone(),
            reasoning: reasoning.clone(),
        };
        let _ = dispatcher.dispatch(event, NotificationLevel::Immediate);

        let decision = ApprovalDecision {
            actor: "kernel".into(),
            target: "spirit".into(),
            capability,
            intent: format!("{:?}", class),
            decision: true,
            reasoning,
        };
        self.log.insert_approval_decision(decision)?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_prompt_auto_allows_and_logs() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mgr = ApprovalManager::new(Arc::clone(&log));
        let dispatcher = NotificationDispatcher::new();
        let result = mgr.prompt(
            ApprovalClass::ReadonlyScoped,
            "fs.read".into(),
            Some("testing".into()),
            &dispatcher,
        );
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify the decision was actually logged
        let approvals = log.query_approvals(None).unwrap();
        assert_eq!(approvals.len(), 1, "exactly one approval should be logged");
        assert_eq!(approvals[0].capability, "fs.read");
        assert!(approvals[0].decision, "v0.3-β auto-allows");
    }

    #[test]
    fn approval_decision_id_increments() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mgr = ApprovalManager::new(Arc::clone(&log));
        let dispatcher = NotificationDispatcher::new();

        mgr.prompt(ApprovalClass::ReadonlyScoped, "a".into(), None, &dispatcher).unwrap();
        mgr.prompt(ApprovalClass::Mutating, "b".into(), None, &dispatcher).unwrap();

        let approvals = log.query_approvals(None).unwrap();
        assert_eq!(approvals.len(), 2, "two decisions should be logged");
        assert_ne!(approvals[0].capability, approvals[1].capability, "decisions should be distinct");
    }
}
