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

use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::notification::{ApprovalClass, NotificationEvent, NotificationLevel};

use crate::iac::transparency_log::{AuditError, TransparencyLogAdapter};
use crate::security::manifest::Posture;
use crate::security::posture::posture_requires_approval;

/// Approval Manager — v0.3-β.
///
/// At v0.3-β returns `Ok(true)` (auto-allow with logged decision).
/// Story 3.3 adds interactive resolution.
#[maos_attrs::i9_exempt(
    reason = "Approval Manager adapter; holds Arc<TransparencyLogAdapter> + AtomicU64 counter — transient/sanctioned state"
)]
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

    /// Posture-aware prompt (Story 3.2, AC5).
    pub fn prompt_with_posture(
        &self,
        posture: Posture,
        class: ApprovalClass,
        capability: String,
        reasoning: Option<String>,
        dispatcher: &NotificationDispatcher,
    ) -> Result<bool, AuditError> {
        // Silent-allow short-circuit when matrix says no approval needed
        if !posture_requires_approval(posture, class) {
            return Ok(true);
        }

        // Otherwise behaves identically to prompt
        self.prompt(class, capability, reasoning, dispatcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct CaptureChannel {
        events: std::sync::Arc<Mutex<Vec<NotificationEvent>>>,
    }

    impl CaptureChannel {
        fn new(events: std::sync::Arc<Mutex<Vec<NotificationEvent>>>) -> Self {
            Self { events }
        }
    }

    impl maos_director_surface::notification::NotificationChannel for CaptureChannel {
        fn surface(&self) -> maos_domain::notification::NotificationSurface {
            maos_domain::notification::NotificationSurface::Terminal
        }
        fn dispatch(
            &self,
            event: &NotificationEvent,
            _level: NotificationLevel,
        ) -> Result<(), maos_director_surface::notification::NotificationError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    fn make_capture_dispatcher() -> (
        std::sync::Arc<Mutex<Vec<NotificationEvent>>>,
        NotificationDispatcher,
    ) {
        let events = std::sync::Arc::new(Mutex::new(Vec::new()));
        let mut dispatcher = NotificationDispatcher::new();
        dispatcher.register(Box::new(CaptureChannel::new(std::sync::Arc::clone(
            &events,
        ))));
        (events, dispatcher)
    }

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

        mgr.prompt(ApprovalClass::ReadonlyScoped, "a".into(), None, &dispatcher)
            .unwrap();
        mgr.prompt(ApprovalClass::Mutating, "b".into(), None, &dispatcher)
            .unwrap();

        let approvals = log.query_approvals(None).unwrap();
        assert_eq!(approvals.len(), 2, "two decisions should be logged");
        assert_ne!(
            approvals[0].capability, approvals[1].capability,
            "decisions should be distinct"
        );
    }

    #[test]
    fn prompt_with_posture_silent_allows_for_matrix_false() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mgr = ApprovalManager::new(Arc::clone(&log));
        let (captured, dispatcher) = make_capture_dispatcher();

        // autonomous-with-halt + Mutating = false in matrix → silent allow
        let result = mgr.prompt_with_posture(
            Posture::AutonomousWithHalt,
            ApprovalClass::Mutating,
            "exec.bash".into(),
            None,
            &dispatcher,
        );
        assert!(result.is_ok());
        assert!(result.unwrap());

        // No approval decision row should be logged (silent allow)
        let approvals = log.query_approvals(None).unwrap();
        assert!(approvals.is_empty(), "silent allow should not log");
        // No notification should be dispatched (silent allow)
        assert!(
            captured.lock().unwrap().is_empty(),
            "silent allow should not dispatch notification"
        );
    }

    #[test]
    fn prompt_with_posture_prompts_for_matrix_true() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mgr = ApprovalManager::new(Arc::clone(&log));
        let (captured, dispatcher) = make_capture_dispatcher();

        // cautious + Mutating = true → requires prompt
        let result = mgr.prompt_with_posture(
            Posture::Cautious,
            ApprovalClass::Mutating,
            "fs.write".into(),
            Some("test".into()),
            &dispatcher,
        );
        assert!(result.is_ok());
        assert!(result.unwrap());

        let approvals = log.query_approvals(None).unwrap();
        assert_eq!(approvals.len(), 1, "should log approval decision");
        assert_eq!(approvals[0].capability, "fs.write");
        // Notification must have been dispatched
        assert_eq!(
            captured.lock().unwrap().len(),
            1,
            "should dispatch notification"
        );
    }

    #[test]
    fn prompt_with_posture_all_cells_match_matrix() {
        use crate::security::posture::POSTURE_APPROVAL_MATRIX;

        for (posture, class, requires_approval) in POSTURE_APPROVAL_MATRIX.iter() {
            let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
            let mgr = ApprovalManager::new(Arc::clone(&log));
            let (captured, dispatcher) = make_capture_dispatcher();

            let result =
                mgr.prompt_with_posture(*posture, *class, "test.cap".into(), None, &dispatcher);
            assert!(result.is_ok());

            let approvals = log.query_approvals(None).unwrap();
            if *requires_approval {
                assert_eq!(
                    approvals.len(),
                    1,
                    "(posture={:?}, class={:?}) should prompt",
                    posture,
                    class
                );
                assert_eq!(
                    captured.lock().unwrap().len(),
                    1,
                    "(posture={:?}, class={:?}) should dispatch notification",
                    posture,
                    class
                );
            } else {
                assert!(
                    approvals.is_empty(),
                    "(posture={:?}, class={:?}) should silent-allow",
                    posture,
                    class
                );
                assert!(
                    captured.lock().unwrap().is_empty(),
                    "(posture={:?}, class={:?}) should not dispatch notification",
                    posture,
                    class
                );
            }
        }
    }
}
