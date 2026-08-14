#![forbid(unsafe_code)]

//! Halt resolution UX surface — three-tap mobile flow per FR15 +
//! architecture §4.6.1 §7.4. The Spirit's halt mechanism owner
//! (Story 4.1) emits halts via `invoke_halt`; this module is the
//! director's-surface side of the loop.

use crate::notification::{DispatchReport, NotificationDispatcher, NotificationError};
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{
    HaltId, HaltJournal, HaltJournalError, HaltResolver, Resolution, ResolutionError, ResolveError,
};
use maos_domain::notification::{NotificationEvent, NotificationLevel};
use std::sync::Arc;

/// The three taps the director performs in the worst case. Used by
/// `resolve_flow` to bound the click-path; the unit test
/// `resolve_flow_completes_in_at_most_three_taps` asserts the state
/// machine never advances past `Tap3Submit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowState {
    /// Tap 1 — notification surfaced; director sees halt + reasoning chain.
    Tap1Acknowledge,
    /// Tap 2 — director selects the resolution kind (one of three).
    Tap2SelectKind,
    /// Tap 3 — director confirms (and supplies text or operator_policy_ref
    /// if the kind requires it). After this tap the resolution submits.
    Tap3Submit,
    /// Terminal — resolution submitted and journaled. No further taps.
    Done,
}

/// Director-surface flow object. Holds a reference to the wired
/// `HaltResolver` and the dispatcher used to surface the halt itself.
pub struct HaltFlow<R: HaltResolver> {
    resolver: Arc<R>,
    dispatcher: Arc<NotificationDispatcher>,
    journal: Arc<dyn HaltJournal>,
}

impl<R: HaltResolver> HaltFlow<R> {
    pub fn new(
        resolver: Arc<R>,
        dispatcher: Arc<NotificationDispatcher>,
        journal: Arc<dyn HaltJournal>,
    ) -> Self {
        Self {
            resolver,
            dispatcher,
            journal,
        }
    }

    /// Surface a halt to the director through the wired dispatcher.
    /// Returns the `DispatchReport` from the dispatcher so callers
    /// (composition root, integration tests) can verify fan-out.
    pub fn dispatch_halt(
        &self,
        halt_id: HaltId,
        payload: EpistemicHaltPayload,
    ) -> Result<DispatchReport, NotificationError> {
        let event = NotificationEvent::Halt { payload };
        self.dispatcher
            .dispatch(event, NotificationLevel::Immediate)
    }

    /// Submit a resolution — validates the payload, resolves via the
    /// HaltResolver, then journals to the Approval Decision Log
    /// (fail-closed: no resolver call and no journal row if the payload
    /// is invalid; no journal row if the resolver rejects). Structural
    /// fail-closed guarantee means every caller gets correct sequencing
    /// automatically, including callers wired to a resolver that does
    /// not validate (e.g. `MockHaltResolver`).
    pub fn submit_resolution(
        &self,
        halt_id: HaltId,
        resolution: Resolution,
        spirit_id: &str,
    ) -> Result<(), HaltUiError> {
        resolution.validate()?;
        self.resolver.resolve(&halt_id, resolution.clone())?;
        self.journal
            .journal_halt_resolution("director", spirit_id, &halt_id, &resolution)?;
        Ok(())
    }

    /// Pure state-machine step. Given the current `FlowState` and a
    /// tap event, return the next `FlowState`. Total function — every
    /// input pair has a defined output. The three-tap budget is
    /// enforced structurally: `Tap1Acknowledge → Tap2SelectKind →
    /// Tap3Submit → Done` is the only path; `Done` is absorbing.
    pub fn resolve_flow(state: FlowState, tap: TapEvent) -> FlowState {
        match (state, tap) {
            (FlowState::Tap1Acknowledge, TapEvent::Acknowledge) => FlowState::Tap2SelectKind,
            (FlowState::Tap2SelectKind, TapEvent::SelectKind) => FlowState::Tap3Submit,
            (FlowState::Tap3Submit, TapEvent::Submit) => FlowState::Done,
            (FlowState::Done, _) => FlowState::Done,
            (s, _) => s,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapEvent {
    Acknowledge,
    SelectKind,
    Submit,
}

#[derive(Debug, thiserror::Error)]
pub enum HaltUiError {
    #[error("halt resolver rejected: {0}")]
    Resolver(#[from] ResolveError),
    #[error("invalid resolution payload: {0}")]
    InvalidResolution(#[from] ResolutionError),
    #[error("notification dispatch failed: {0}")]
    Dispatch(#[from] NotificationError),
    #[error("audit journal write failed: {0}")]
    Audit(#[from] HaltJournalError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::halt::HaltJournalError;
    use std::sync::Mutex as StdMutex;

    /// Records every journal write so tests can prove the fail-closed
    /// contract: nothing lands in the Approval Decision Log unless the
    /// payload validated AND the resolver accepted.
    #[derive(Default)]
    struct MockJournal {
        writes: StdMutex<Vec<(String, Resolution)>>,
    }

    impl MockJournal {
        fn writes(&self) -> Vec<(String, Resolution)> {
            self.writes.lock().unwrap().clone()
        }
    }

    impl HaltJournal for MockJournal {
        fn journal_halt_resolution(
            &self,
            _actor: &str,
            _spirit_id: &str,
            halt_id: &HaltId,
            resolution: &Resolution,
        ) -> Result<(), HaltJournalError> {
            self.writes
                .lock()
                .unwrap()
                .push((halt_id.as_str().to_owned(), resolution.clone()));
            Ok(())
        }
    }

    /// A simple in-process `HaltResolver` that captures calls for unit tests.
    /// Mirrors `MockHaltResolver` shape but lives here to avoid the
    /// kernel-core circular dependency.
    struct TestResolver {
        calls: StdMutex<Vec<(HaltId, Resolution)>>,
    }

    impl TestResolver {
        fn new() -> Self {
            Self {
                calls: StdMutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(HaltId, Resolution)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl HaltResolver for TestResolver {
        fn resolve(&self, halt_id: &HaltId, resolution: Resolution) -> Result<(), ResolveError> {
            self.calls
                .lock()
                .unwrap()
                .push((halt_id.clone(), resolution));
            Ok(())
        }
    }

    /// A resolver that always returns UnknownHalt.
    struct FailingResolver;

    impl HaltResolver for FailingResolver {
        fn resolve(&self, halt_id: &HaltId, _: Resolution) -> Result<(), ResolveError> {
            Err(ResolveError::UnknownHalt(halt_id.as_str().into()))
        }
    }

    /// A CaptureChannel for notification tests in halt_ui context.
    /// Mirrors the shape from `approval_prompt_e2e.rs:14-36`.
    struct CaptureChannel {
        events: Arc<std::sync::Mutex<Vec<maos_domain::notification::NotificationEvent>>>,
    }

    impl CaptureChannel {
        fn new(
            events: Arc<std::sync::Mutex<Vec<maos_domain::notification::NotificationEvent>>>,
        ) -> Self {
            Self { events }
        }
    }

    impl crate::notification::NotificationChannel for CaptureChannel {
        fn surface(&self) -> maos_domain::notification::NotificationSurface {
            maos_domain::notification::NotificationSurface::Terminal
        }
        fn dispatch(
            &self,
            event: &maos_domain::notification::NotificationEvent,
            _level: maos_domain::notification::NotificationLevel,
        ) -> Result<(), crate::notification::NotificationError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn dispatch_halt_emits_one_halt_event_per_registered_channel() {
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = CaptureChannel::new(events.clone());

        let mut dispatcher = NotificationDispatcher::new();
        dispatcher.register(Box::new(capture));

        let journal = Arc::new(MockJournal::default());
        let resolver = Arc::new(TestResolver::new());
        let flow = HaltFlow::new(resolver, Arc::new(dispatcher), journal);

        let payload = EpistemicHaltPayload::new(
            "halt-001".into(),
            "tag".into(),
            0.3,
            Some(0.5),
            "pol".into(),
            "d".into(),
        )
        .unwrap();

        let report = flow
            .dispatch_halt(HaltId::new("halt-001").unwrap(), payload)
            .unwrap();
        assert_eq!(report.delivered, 1);
        assert_eq!(report.errors, 0);

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert!(matches!(&captured[0], NotificationEvent::Halt { .. }));
    }

    #[test]
    fn submit_provided_context_calls_resolver_with_text() {
        let journal = Arc::new(MockJournal::default());
        let resolver = Arc::new(TestResolver::new());
        let flow = HaltFlow::new(resolver, Arc::new(NotificationDispatcher::new()), journal);

        let hid = HaltId::new("halt-001").unwrap();
        let res = Resolution::ProvidedContext {
            text: "missing context".into(),
        };
        flow.submit_resolution(hid.clone(), res.clone(), "hello-spirit")
            .unwrap();

        let calls = flow.resolver.as_ref().calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, hid);
        assert!(
            matches!(&calls[0].1, Resolution::ProvidedContext { text } if text == "missing context")
        );
    }

    #[test]
    fn submit_accepted_halt_calls_resolver_with_correct_kind() {
        let journal = Arc::new(MockJournal::default());
        let resolver = Arc::new(TestResolver::new());
        let flow = HaltFlow::new(resolver, Arc::new(NotificationDispatcher::new()), journal);

        let hid = HaltId::new("halt-002").unwrap();
        flow.submit_resolution(hid, Resolution::AcceptedHalt, "hello-spirit")
            .unwrap();

        let calls = flow.resolver.as_ref().calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(calls[0].1, Resolution::AcceptedHalt));
    }

    #[test]
    fn submit_authorized_override_carries_operator_policy_ref() {
        let journal = Arc::new(MockJournal::default());
        let resolver = Arc::new(TestResolver::new());
        let flow = HaltFlow::new(resolver, Arc::new(NotificationDispatcher::new()), journal);

        let hid = HaltId::new("halt-003").unwrap();
        let res = Resolution::AuthorizedOverride {
            operator_policy_ref: "policy://x".into(),
        };
        flow.submit_resolution(hid, res, "hello-spirit").unwrap();

        let calls = flow.resolver.as_ref().calls();
        assert_eq!(calls.len(), 1);
        assert!(
            matches!(&calls[0].1, Resolution::AuthorizedOverride { operator_policy_ref } if operator_policy_ref == "policy://x")
        );
    }

    #[test]
    fn submit_resolution_surfaces_resolver_error_and_writes_no_journal_row() {
        let journal = Arc::new(MockJournal::default());
        let resolver = Arc::new(FailingResolver);
        let flow = HaltFlow::new(
            resolver,
            Arc::new(NotificationDispatcher::new()),
            journal.clone(),
        );

        let hid = HaltId::new("halt-001").unwrap();
        let result = flow.submit_resolution(hid, Resolution::AcceptedHalt, "hello-spirit");
        let err = result.expect_err("failing resolver must surface an error");
        assert!(matches!(
            err,
            HaltUiError::Resolver(ResolveError::UnknownHalt(_))
        ));
        assert!(
            journal.writes().is_empty(),
            "fail-closed: a rejected resolution must not be journaled"
        );
    }

    /// Story 3.3 review closure (High) — the director surface refuses an
    /// empty payload BEFORE the resolver is touched, so the guarantee holds
    /// even against a resolver that performs no validation of its own.
    #[test]
    fn submit_resolution_rejects_empty_payload_before_resolver_and_journal() {
        for invalid in [
            Resolution::ProvidedContext {
                text: String::new(),
            },
            Resolution::ProvidedContext { text: "  ".into() },
            Resolution::AuthorizedOverride {
                operator_policy_ref: String::new(),
            },
        ] {
            let journal = Arc::new(MockJournal::default());
            let resolver = Arc::new(TestResolver::new());
            let flow = HaltFlow::new(
                resolver.clone(),
                Arc::new(NotificationDispatcher::new()),
                journal.clone(),
            );

            let hid = HaltId::new("halt-empty").unwrap();
            let err = flow
                .submit_resolution(hid, invalid.clone(), "hello-spirit")
                .expect_err("empty payload must be refused");
            assert!(
                matches!(err, HaltUiError::InvalidResolution(_)),
                "{invalid:?}: expected InvalidResolution, got {err:?}"
            );
            assert!(
                resolver.calls().is_empty(),
                "{invalid:?}: resolver must not be invoked for an invalid payload"
            );
            assert!(
                journal.writes().is_empty(),
                "{invalid:?}: nothing may be journaled for an invalid payload"
            );
        }
    }

    #[test]
    fn resolve_flow_advances_through_three_taps_and_absorbs_done() {
        use FlowState::*;
        use TapEvent::*;

        let s0 = Tap1Acknowledge;
        let s1 = HaltFlow::<TestResolver>::resolve_flow(s0, Acknowledge);
        assert_eq!(s1, Tap2SelectKind);
        let s2 = HaltFlow::<TestResolver>::resolve_flow(s1, SelectKind);
        assert_eq!(s2, Tap3Submit);
        let s3 = HaltFlow::<TestResolver>::resolve_flow(s2, Submit);
        assert_eq!(s3, Done);

        // Done is absorbing
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Done, Acknowledge),
            Done
        );
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Done, SelectKind),
            Done
        );
        assert_eq!(HaltFlow::<TestResolver>::resolve_flow(Done, Submit), Done);
    }

    #[test]
    fn resolve_flow_total_function_all_12_pairs() {
        use FlowState::*;
        use TapEvent::*;

        // All 12 pairs: 4 states × 3 events — every pair has a defined output.
        // Tap1Acknowledge
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap1Acknowledge, Acknowledge),
            Tap2SelectKind
        );
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap1Acknowledge, SelectKind),
            Tap1Acknowledge
        ); // stays — wrong tap
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap1Acknowledge, Submit),
            Tap1Acknowledge
        ); // stays — wrong tap

        // Tap2SelectKind
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap2SelectKind, Acknowledge),
            Tap2SelectKind
        ); // stays
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap2SelectKind, SelectKind),
            Tap3Submit
        );
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap2SelectKind, Submit),
            Tap2SelectKind
        ); // stays

        // Tap3Submit
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap3Submit, Acknowledge),
            Tap3Submit
        ); // stays
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap3Submit, SelectKind),
            Tap3Submit
        ); // stays
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Tap3Submit, Submit),
            Done
        );

        // Done — absorbing
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Done, Acknowledge),
            Done
        );
        assert_eq!(
            HaltFlow::<TestResolver>::resolve_flow(Done, SelectKind),
            Done
        );
        assert_eq!(HaltFlow::<TestResolver>::resolve_flow(Done, Submit), Done);
    }
}
