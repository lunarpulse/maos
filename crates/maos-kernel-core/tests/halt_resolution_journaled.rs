use std::sync::Arc;

use maos_director_surface::halt_ui::HaltFlow;
use maos_domain::halt::{HaltId, HaltJournal, Resolution};
use maos_kernel_core::halt::{FailingHaltResolver, MockHaltResolver};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_director_surface::notification::NotificationDispatcher;

#[test]
fn halt_resolution_three_variants_journaled() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));

    let mock = Arc::new(MockHaltResolver::new());
    let dispatcher = Arc::new(NotificationDispatcher::new());
    let journal: Arc<dyn HaltJournal> = Arc::clone(&log) as Arc<dyn HaltJournal>;
    let flow = HaltFlow::new(mock.clone(), dispatcher, journal);

    let hid1 = HaltId::new("halt-001").unwrap();
    let res1 = Resolution::ProvidedContext {
        text: "the issue is X".into(),
    };
    flow.submit_resolution(hid1.clone(), res1.clone(), "spirit-1").unwrap();

    let hid2 = HaltId::new("halt-002").unwrap();
    let res2 = Resolution::AcceptedHalt;
    flow.submit_resolution(hid2.clone(), res2.clone(), "spirit-1").unwrap();

    let hid3 = HaltId::new("halt-003").unwrap();
    let res3 = Resolution::AuthorizedOverride {
        operator_policy_ref: "policy://override/2026-05".into(),
    };
    flow.submit_resolution(hid3.clone(), res3.clone(), "spirit-1").unwrap();

    let approvals = log.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 3, "expected 3 approval decision rows");

    for row in &approvals {
        assert_eq!(row.capability, "halt.resolve");
        assert_eq!(row.actor, "director");
        assert_eq!(row.target, "spirit-1");
        assert!(row.decision);
        assert!(row.reasoning.as_deref().unwrap().contains("halt="));
    }

    let intents: Vec<&str> = approvals.iter().map(|r| r.intent.as_str()).collect();
    assert!(intents.contains(&"provided_context"));
    assert!(intents.contains(&"accepted_halt"));
    assert!(intents.contains(&"authorized_override"));

    let reasoning_texts: Vec<&str> = approvals
        .iter()
        .filter_map(|r| r.reasoning.as_deref())
        .collect();
    assert!(reasoning_texts.iter().any(|r| r.contains("the issue is X")));
    assert!(reasoning_texts.iter().any(|r| r.contains("accepted_halt")));
    assert!(reasoning_texts
        .iter()
        .any(|r| r.contains("authorized_override: operator_policy_ref=policy://override/2026-05")));

    let calls = mock.calls();
    assert_eq!(calls.len(), 3, "mock resolver should have 3 calls");
    assert_eq!(calls[0].0.as_str(), "halt-001");
    assert_eq!(calls[1].0.as_str(), "halt-002");
    assert_eq!(calls[2].0.as_str(), "halt-003");

    let frames = log
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            ..Default::default()
        })
        .unwrap();
    let halt_in_tl_count = frames.iter().filter(|f| {
        let payload_str = String::from_utf8_lossy(&f.payload_redacted);
        payload_str.contains("halt.resolve")
    }).count();
    assert_eq!(
        halt_in_tl_count, 0,
        "halt resolution must not appear in transparency_log"
    );
}

#[test]
fn failing_resolver_prevents_journal_write() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));

    let fail_resolver = Arc::new(FailingHaltResolver);
    let dispatcher = Arc::new(NotificationDispatcher::new());
    let journal: Arc<dyn HaltJournal> = Arc::clone(&log) as Arc<dyn HaltJournal>;
    let flow = HaltFlow::new(fail_resolver, dispatcher, journal);

    let hid = HaltId::new("halt-fail").unwrap();
    let res = Resolution::AcceptedHalt;

    let result = flow.submit_resolution(hid.clone(), res.clone(), "spirit-1");
    assert!(result.is_err(), "expected resolver error");
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("unknown halt_id"), "error should mention unknown halt_id: {err_str}");

    let approvals = log.query_approvals(None).unwrap();
    assert!(approvals.is_empty(), "no approval row should be written on resolver failure");
}
