//! Approval prompt end-to-end integration test (AC6).
//!
//! Verifies the full path: ApprovalManager::prompt dispatches a
//! NotificationEvent::ApprovalPrompt and the decision is persisted
//! in the Approval Decision Log (distinct from Transparency Log).

use std::sync::{Arc, Mutex};

use maos_director_surface::notification::{NotificationChannel, NotificationDispatcher, NotificationError};
use maos_domain::notification::{ApprovalClass, NotificationEvent, NotificationLevel};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::security::approval::ApprovalManager;

struct CaptureChannel {
    events: Arc<Mutex<Vec<NotificationEvent>>>,
}

impl CaptureChannel {
    fn new(events: Arc<Mutex<Vec<NotificationEvent>>>) -> Self {
        Self { events }
    }
}

impl NotificationChannel for CaptureChannel {
    fn surface(&self) -> maos_domain::notification::NotificationSurface {
        maos_domain::notification::NotificationSurface::Terminal
    }
    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

#[test]
fn approval_prompt_e2e_dispatches_and_logs() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mgr = ApprovalManager::new(Arc::clone(&log));

    let captured = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = NotificationDispatcher::new();
    dispatcher.register(Box::new(CaptureChannel::new(Arc::clone(&captured))));

    let result = mgr.prompt(
        ApprovalClass::ReadonlyScoped,
        "fs.read".into(),
        Some("user requested file read".into()),
        &dispatcher,
    );

    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify the notification was dispatched to our capture channel
    {
        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1, "exactly one notification should be dispatched");
        match &events[0] {
            NotificationEvent::ApprovalPrompt { capability, .. } => {
                assert_eq!(capability, "fs.read");
            }
            _ => panic!("expected ApprovalPrompt event"),
        }
    }

    let approvals = log.query_approvals(None).unwrap();
    assert!(
        !approvals.is_empty(),
        "Approval Decision Log should contain at least one row"
    );

    let last = approvals.last().unwrap();
    assert_eq!(last.capability, "fs.read");
    assert!(last.decision, "v0.3-β auto-allows");
}

#[test]
fn approval_prompt_is_distinct_from_transparency_log() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mgr = ApprovalManager::new(Arc::clone(&log));

    let dispatcher = NotificationDispatcher::new();

    mgr.prompt(
        ApprovalClass::Mutating,
        "exec.bash".into(),
        None,
        &dispatcher,
    )
    .unwrap();

    let approvals = log.query_approvals(None).unwrap();
    assert!(!approvals.is_empty());

    // Verify the approval_decision_log row exists and matches
    assert_eq!(approvals.last().unwrap().capability, "exec.bash");

    // Verify transparency_log does NOT contain the approval decision
    // (they are distinct tables per I4 + NFR-Obs-5)
    let frames = log
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskAssign),
            ..Default::default()
        })
        .unwrap();

    let has_approval_in_tl = frames.iter().any(|f| {
        let payload_str = String::from_utf8_lossy(&f.payload_redacted);
        payload_str.contains("exec.bash")
    });

    assert!(!has_approval_in_tl,
        "approval decision should not appear as a frame in transparency_log (I4 + NFR-Obs-5: distinct tables)");
}
