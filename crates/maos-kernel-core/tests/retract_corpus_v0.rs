#![forbid(unsafe_code)]

//! Story 6.1 — retract corpus integration test (v0).
//!
//! Loads scenarios from `crates/maos-eval/fixtures/retract-corpus-v0/`
//! and asserts the expected outcomes.

use std::path::Path;
use std::sync::Arc;

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, RetractPayload};
use maos_domain::iac_bus_types::RetractOutcome;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::ports::IacBusPort;
use maos_kernel_core::iac::{
    IacBusAdapter, Mailbox, TransparencyLogAdapter,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

fn hex_to_frame_id(hex: &str) -> [u8; 16] {
    let bytes = hex::decode(hex).expect("invalid hex frame_id_hex in fixture");
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes[..16.min(bytes.len())]);
    arr
}

fn kind_from_str(s: &str) -> FrameKind {
    match s {
        "TaskAssign" => FrameKind::TaskAssign,
        "TaskComplete" => FrameKind::TaskComplete,
        "DecisionDispatch" => FrameKind::DecisionDispatch,
        "EpistemicHalt" => FrameKind::EpistemicHalt,
        "TelemetryEvent" => FrameKind::TelemetryEvent,
        "ConsentRequest" => FrameKind::ConsentRequest,
        "Retract" => FrameKind::Retract,
        _ => FrameKind::TaskAssign,
    }
}

fn make_frame_for_scenario(
    fid: [u8; 16],
    from: &str,
    to: &str,
    kind: FrameKind,
    payload_size: usize,
) -> IacFrame {
    let payload = match kind {
        FrameKind::DecisionDispatch => FramePayload::DecisionDispatch(
            maos_domain::frame::DecisionDispatchPayload {
                decision_id: 42,
                approved: true,
                working_memory_digest_refs: Default::default(),
            },
        ),
        FrameKind::ConsentRequest => FramePayload::ConsentRequest(
            maos_domain::frame::ConsentRequestPayload {
                capability: "test.cap".into(),
            },
        ),
        FrameKind::EpistemicHalt => FramePayload::EpistemicHalt(
            maos_domain::frame::EpistemicHaltPayload::new(
                "halt-1".into(), "tag".into(), 0.5, None, "pol".into(), "src".into(),
            ).unwrap(),
        ),
        FrameKind::TelemetryEvent => FramePayload::TelemetryEvent(
            maos_domain::frame::TelemetryEventPayload {
                event_type: "test".into(),
                data: "x".repeat(payload_size.min(4096)),
            },
        ),
        _ => FramePayload::TaskAssign(maos_domain::frame::TaskAssignPayload {
            goal: "x".repeat(payload_size.min(4096)),
            scope: vec![],
            success_criteria: "done".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
    };
    IacFrame {
        frame_id: fid,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(from),
            host_id: None,
            role: None,
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from(to),
            host_id: None,
            role: None,
        }],
        kind,
        intent: IntentClass::Standard,
        payload,
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

#[tokio::test]
async fn retract_corpus_fixtures() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("maos-eval/fixtures/retract-corpus-v0");
    let corpus = maos_eval::retract_corpus::RetractCorpus::load_from(&corpus_dir)
        .expect("failed to load retract corpus");

    assert!(corpus.len() >= 30, "expected >=30 scenarios, got {}", corpus.len());

    let mut results = Vec::new();

    for scenario in &corpus.scenarios {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(
            maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
        )));
        let adapter = IacBusAdapter::new(mailbox.clone(), tl.clone());

        let from_spirit = &scenario.original_frame.from_spirit;
        let to_spirit = &scenario.original_frame.to_spirit;

        // Register both spirits
        let _h_from = adapter.register_spirit_typed(&SpiritId::from(from_spirit.as_str())).ok();
        let _h_to = adapter.register_spirit_typed(&SpiritId::from(to_spirit.as_str())).ok();

        let frame_id = hex_to_frame_id(&scenario.original_frame.frame_id_hex);
        let kind = kind_from_str(&scenario.original_frame.kind);
        let frame = make_frame_for_scenario(frame_id, from_spirit, to_spirit, kind, scenario.original_frame.payload_size_bytes);

        let _ = adapter.deliver_typed(frame).await.unwrap();

        let result = adapter
            .retract(
                frame_id,
                scenario.retract_request.reason.clone(),
                &SpiritId::from(scenario.retract_request.retracting_spirit.as_str()),
            )
            .await;

        let expected = &scenario.expected_outcome;
        let passed = match (expected.success, &expected.outcome_variant[..]) {
            (true, "Retracted") => matches!(result, Ok(RetractOutcome::Retracted { .. })),
            (true, "Already") => {
                // Need a second retract for Already — first one gives Retracted
                let _first = adapter
                    .retract(
                        frame_id,
                        "first".into(),
                        &SpiritId::from(scenario.retract_request.retracting_spirit.as_str()),
                    )
                    .await;
                let second = adapter
                    .retract(
                        frame_id,
                        scenario.retract_request.reason.clone(),
                        &SpiritId::from(scenario.retract_request.retracting_spirit.as_str()),
                    )
                    .await;
                matches!(second, Ok(RetractOutcome::Already { .. }))
            }
            (true, "OriginalNotFound") => matches!(result, Ok(RetractOutcome::OriginalNotFound)),
            (false, "Error") => {
                matches!(result, Err(ref e) if {
                    let err_str = format!("{:?}", e);
                    expected.error_variant.as_deref().map_or(false, |ev| err_str.contains(ev))
                })
            }
            _ => false,
        };

        results.push((scenario.scenario_id.clone(), passed));
        if !passed {
            eprintln!(
                "FAIL {} ({}): expected success={} variant={} error={:?}, got {:?}",
                scenario.scenario_id,
                scenario.category,
                expected.success,
                expected.outcome_variant,
                expected.error_variant,
                result,
            );
        }
    }

    let total = results.len();
    let passed = results.iter().filter(|(_, p)| *p).count();
    eprintln!("retract corpus: {passed}/{total} passed");
    for (id, p) in &results {
        if !p {
            eprintln!("  FAIL: {id}");
        }
    }
    assert_eq!(passed, total, "all 30 retract corpus scenarios must pass");
}
