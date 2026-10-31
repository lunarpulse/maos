#![cfg(feature = "spirit_test")]
#![forbid(unsafe_code)]

//! AC5 FR34. The green vector covers shipped hooks, input delivery, halt
//! resolution, manifest self-check and class corpus. The labelled red vector
//! keeps the missing SDK IAC capture visible, not a guarantee of frame I/O.
use maos_spirit_sdk::spirit_test::{
    HaltResolutionKind, RegressionCase, RegressionCorpus, SpiritClass, SpiritTest,
};
use maos_spirit_sdk::{
    spirit, ConsolidatePayload, Ctx, FramePayload, SchedulePayload, SwapInPayload,
    TelemetryEventPayload,
};
use std::sync::Mutex;

#[derive(Default)]
pub struct FixtureSpirit {
    hooks: Mutex<Vec<&'static str>>,
    input: Mutex<Vec<Vec<u8>>>,
}

#[spirit]
impl FixtureSpirit {
    fn on_load(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_load");
    }
    fn on_start(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_start");
    }
    fn on_frame<'a>(&self, _ctx: &mut Ctx, frame: &FramePayload<'a>) {
        self.input.lock().unwrap().push(frame.frame_data.to_vec());
    }
    fn on_idle(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_idle");
    }
    fn on_telemetry_event<'a>(&self, _ctx: &mut Ctx, _payload: &TelemetryEventPayload<'a>) {
        self.hooks.lock().unwrap().push("on_telemetry_event");
    }
    fn on_schedule<'a>(&self, _ctx: &mut Ctx, _payload: &SchedulePayload<'a>) {
        self.hooks.lock().unwrap().push("on_schedule");
    }
    fn on_swap_in<'a>(&self, _ctx: &mut Ctx, _payload: &SwapInPayload<'a>) {
        self.hooks.lock().unwrap().push("on_swap_in");
    }
    fn on_consolidate<'a>(&self, _ctx: &mut Ctx, _payload: &ConsolidatePayload<'a>) {
        self.hooks.lock().unwrap().push("on_consolidate");
    }
    fn on_pause(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_pause");
    }
    fn on_resume(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_resume");
    }
    fn on_unload(&self, _ctx: &mut Ctx) {
        self.hooks.lock().unwrap().push("on_unload");
    }
}

const MANIFEST: &[u8] = br#"
[class]
name = "sdk-fixture"
version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["answer"]
[sandbox]
tier = "T0"
"#;

#[test]
fn sdk_harness_covers_lifecycle_input_halt_manifest_and_class() {
    let spirit = FixtureSpirit::default();
    let mut harness = SpiritTest::new(&spirit, __maos_spirit_vtable_FixtureSpirit());
    let manifest = harness
        .manifest_self_check(MANIFEST)
        .expect("fixture manifest");
    assert_eq!(manifest.class_name, "sdk-fixture");
    assert_eq!(manifest.sandbox_tier, "T0");
    let mut corpus = RegressionCorpus::new(SpiritClass::Generic);
    corpus.cases.push(RegressionCase {
        id: "sdk-fr34".into(),
        fixture_setup: "iac-frame".into(),
        expected_assertions: vec!["replay-context".into()],
    });
    assert_eq!(corpus.class, SpiritClass::Generic);
    assert_eq!(corpus.cases[0].id, "sdk-fr34");
    let fixture = harness.fixture_mut();
    fixture.invoke_on_load = true;
    fixture.invoke_on_start = true;
    fixture.invoke_on_idle = true;
    fixture.invoke_on_pause = true;
    fixture.invoke_on_resume = true;
    fixture.invoke_on_unload = true;
    fixture.frames = corpus
        .cases
        .iter()
        .map(|case| case.fixture_setup.as_bytes().to_vec())
        .collect();
    fixture.telemetry_events = vec![b"telemetry".to_vec()];
    fixture.schedule_payloads = vec![b"schedule".to_vec()];
    fixture.swap_in_payloads = vec![b"predecessor".to_vec()];
    fixture.consolidate_payloads = vec![b"consolidate".to_vec()];
    for (id, kind) in [
        (
            "provided",
            HaltResolutionKind::ProvidedContext {
                context_bytes: b"replay-context".to_vec(),
            },
        ),
        ("accepted", HaltResolutionKind::AcceptedHalt),
        (
            "override",
            HaltResolutionKind::AuthorizedOverride {
                override_marker: b"policy-1".to_vec(),
            },
        ),
    ] {
        harness.resolve_halt(id.into(), kind);
    }
    let report = harness.run();
    assert_eq!(
        *spirit.hooks.lock().unwrap(),
        [
            "on_load",
            "on_start",
            "on_idle",
            "on_telemetry_event",
            "on_schedule",
            "on_swap_in",
            "on_pause",
            "on_resume",
            "on_consolidate",
            "on_unload"
        ]
    );
    assert_eq!(
        *spirit.input.lock().unwrap(),
        corpus
            .cases
            .iter()
            .map(|case| case.fixture_setup.as_bytes().to_vec())
            .collect::<Vec<_>>()
    );
    for hook in [
        "on_load",
        "on_start",
        "on_frame",
        "on_idle",
        "on_telemetry_event",
        "on_schedule",
        "on_swap_in",
        "on_pause",
        "on_resume",
        "on_consolidate",
        "on_unload",
    ] {
        assert_eq!(report.base.hooks_fired.get(hook), Some(&1), "{hook}");
    }
    assert_eq!(report.halt_resolutions.len(), 3);
    assert_eq!(report.halt_resolutions[0].halt_id, "provided");
    assert!(matches!(&report.halt_resolutions[0].kind,
        HaltResolutionKind::ProvidedContext { context_bytes } if context_bytes == corpus.cases[0].expected_assertions[0].as_bytes()));
    assert!(matches!(
        report.halt_resolutions[1].kind,
        HaltResolutionKind::AcceptedHalt
    ));
    assert!(matches!(&report.halt_resolutions[2].kind,
        HaltResolutionKind::AuthorizedOverride { override_marker } if override_marker == b"policy-1"));
    let invalid = String::from_utf8(MANIFEST.to_vec())
        .unwrap()
        .replace("T0", "T99");
    assert!(maos_spirit_sdk::spirit_test::manifest_self_check(invalid.as_bytes()).is_err());
}

#[test]
#[ignore = "red witness: SDK LocalRunner never populates mock_bus_frames (local_runner.rs:43-69,179); ownerless and open, dispositioned by 20-3b AC5.4, trigger-owned by the first story touching the SDK LocalRunner"]
fn sdk_harness_records_iac_frame_io_in_its_capture_report() {
    let spirit = FixtureSpirit::default();
    let mut harness = SpiritTest::new(&spirit, __maos_spirit_vtable_FixtureSpirit());
    harness.fixture_mut().frames = vec![b"iac-frame".to_vec()];
    let report = harness.run();
    assert_eq!(
        *spirit.input.lock().unwrap(),
        [b"iac-frame".to_vec()],
        "the frame really reached the fixture"
    );
    assert_eq!(report.captured_frames.iter().map(|frame| frame.bytes.as_slice()).collect::<Vec<_>>(),
        [b"iac-frame".as_slice()], "FR34 SDK frame I/O must be observable in the harness report; empty capture is not coverage");
}
