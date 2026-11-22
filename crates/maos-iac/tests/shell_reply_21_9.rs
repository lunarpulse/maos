//! An installed Spirit's shell-only answer must not become ordinary mailbox traffic.
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, RuptureReason, TaskCompletePayload,
};
use maos_domain::iac_bus_types::RetractOutcome;
use maos_domain::invariants::i1::{IntentClass, TokenId};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::adapter::mailbox::{
    shell_reply_route, ConsentGate, Mailbox, SHELL_REPLY_DESTINATION,
};
use maos_iac::adapter::transparency_log::{
    FrameFilter, FrameKind as LogKind, TransparencyLogAdapter,
};
use maos_iac::adapter::{DrrScheduler, IacBusAdapter, IacRtMetrics};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

struct RequireShellConsent;

impl ConsentGate for RequireShellConsent {
    fn evaluate(&self, _: &IacFrame, recipient: &FrameAddress) -> Result<(), RuptureReason> {
        if recipient.spirit_id.as_str() == SHELL_REPLY_DESTINATION {
            return Err(RuptureReason::IntentAllowlistMismatch);
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scoped_replies_and_their_retractions_stay_out_of_the_operator_spirits_mailbox() {
    init_monotonic_base();
    for drr in [false, true] {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let mut bus = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&log));
        let (warnings, _warning_rx) = tokio::sync::mpsc::unbounded_channel();
        if drr {
            bus = bus.with_drr_scheduler(DrrScheduler::new(Arc::clone(&log), warnings));
        }
        let mut operator = mailbox.register_spirit("operator").unwrap();
        let guest = SpiritId::from("guest");
        let pid = 41;
        let correlation = [1; 16];
        let reply_id = [2; 16];
        let (route, mut shell) = shell_reply_route(
            correlation,
            guest.clone(),
            pid,
            Instant::now() + Duration::from_secs(30),
        );
        let address = |spirit_id| FrameAddress {
            spirit_id,
            host_id: None,
            role: None,
        };
        let payload = TaskCompletePayload {
            result: "echo: hello".into(),
        };
        let mut frame = IacFrame {
            frame_id: reply_id,
            timestamp_ns: 0,
            logical_clock: 0,
            from: address(guest.clone()),
            to: [address(SpiritId::from(SHELL_REPLY_DESTINATION))]
                .into_iter()
                .collect(),
            kind: FrameKind::TaskComplete,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskComplete(payload.clone()),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: None,
            // In production the host inherits the triggering human request's lineage.
            intent_lineage: IntentLineage::new(vec![A2AIntent::new("standard")]),
        };
        let token = TokenId([3; 16]);
        bus.deliver_shell_reply(frame.clone(), pid, token, &route, correlation)
            .await
            .unwrap();
        let answer = shell.try_recv().expect("missing shell answer");
        assert_eq!(answer.result, "echo: hello");
        let rows = || {
            log.query_frames(FrameFilter {
                spirit_pid: Some(pid),
                ..FrameFilter::default()
            })
            .unwrap()
        };
        let reply = rows()
            .into_iter()
            .find(|row| row.frame_id == reply_id)
            .unwrap();
        assert_eq!(
            reply.correlation_id.as_deref(),
            Some("maos.shell-reply/01010101010101010101010101010101"),
        );
        assert!(matches!(
            bus.retract(reply_id, "superseded".into(), &guest, Some(token))
                .await
                .unwrap(),
            RetractOutcome::Retracted { .. }
        ));
        let retract = rows()
            .into_iter()
            .find(|row| row.kind == LogKind::Retract)
            .unwrap();
        assert_eq!(retract.correlation_id, reply.correlation_id);
        assert!(
            operator.try_recv().unwrap().is_none(),
            "shell traffic reached the real Spirit"
        );

        frame.frame_id = [4; 16];
        frame.to = [address(SpiritId::from("operator"))].into_iter().collect();
        bus.deliver_typed(frame.clone(), pid, Some(token))
            .await
            .unwrap();
        let (_, ordinary) = operator
            .try_recv()
            .unwrap()
            .expect("ordinary reply was not routed");
        assert_eq!(ordinary.frame_id, [4; 16]);

        mailbox.install_transparency_log(Arc::clone(&log)).unwrap();
        mailbox
            .install_consent_gate(Arc::new(RequireShellConsent))
            .unwrap();
        let mut guest_mailbox = mailbox.register_spirit("guest").unwrap();
        let (route, mut shell) = shell_reply_route(
            [5; 16],
            guest,
            pid,
            Instant::now() + Duration::from_secs(30),
        );
        frame.frame_id = [6; 16];
        frame.to = [address(SpiritId::from(SHELL_REPLY_DESTINATION))]
            .into_iter()
            .collect();
        bus.deliver_shell_reply(frame, pid, token, &route, [5; 16])
            .await
            .unwrap();
        assert!(
            shell.try_recv().is_none(),
            "consent denied the private answer"
        );
        let (kind, _) = guest_mailbox
            .try_recv()
            .unwrap()
            .expect("a normal consent refusal notified the sender");
        assert_eq!(kind, FrameKind::ConsentRupture);
        assert!(log
            .query_frames(FrameFilter::default())
            .unwrap()
            .iter()
            .any(|row| row.intent == "consent.rupture.quarantine"));
    }
}
