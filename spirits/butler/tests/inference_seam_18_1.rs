#![forbid(unsafe_code)]

//! Story 18-1 AC1 (D-18-1-03 A, D-18-1-04 A, D-18-1-05 A) — Butler's seam
//! over the Inference Port, at library level.
//!
//! - The completion contract is ONE JSON object; a missing or ill-typed field
//!   is a typed `ButlerError::Proposal` naming the field, never a default.
//! - Bound: one `complete` per pass, a fresh mint per pass, the request names
//!   the bound provider; the notification and `user_preference_drift` are the
//!   completion's, `belief_variance` stays arithmetic.
//! - A failed or malformed completion refuses the pass typed: no
//!   notification, no output-channel write, no scalar write.
//! - Unbound (the deterministic path) emits `options` too.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use butler::__maos_spirit_vtable_Butler;
use butler::{
    parse_proposal, Butler, ButlerError, ButlerInferenceBinding, ButlerMcpError, ButlerMcpPort,
    ButlerSeam, CalendarEvent, EventStatus, MorningDigestRenderer, ScenarioInput,
    DETERMINISTIC_OPTIONS,
};
use maos_domain::halt::HaltReceipt;
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::{EpistemicScalarPort, InferencePort, ScalarPortError};
use maos_spirit_sdk::clock::SystemClock;
use maos_spirit_sdk::spirit_test::SpiritTest;

const GOOD: &str = r#"{"pattern":"calendar-conflict","confidence":0.82,"evidence":"Board review overlaps Investor call","options":["linear_note: note","reminder: remind","snooze: snooze"],"user_preference_drift":0.9}"#;

fn conflict() -> ScenarioInput {
    let event = |id: &str, start, end| CalendarEvent {
        id: id.into(),
        title: id.into(),
        start_min: start,
        end_min: end,
        status: EventStatus::Confirmed,
    };
    ScenarioInput {
        calendar: vec![event("evt-a", 540, 600), event("evt-b", 570, 630)],
        ..Default::default()
    }
}

fn drive_on_idle(spirit: &Butler) {
    let vtable = __maos_spirit_vtable_Butler();
    let mut harness = SpiritTest::new(spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1
    );
}

/// Answers every call with `text`, or with `error` when set; records requests.
struct ScriptedPort {
    text: String,
    error: Option<fn() -> InferenceError>,
    requests: Mutex<Vec<(Option<String>, Vec<String>, String)>>,
}

impl ScriptedPort {
    fn answering(text: &str) -> Arc<Self> {
        Arc::new(Self {
            text: text.into(),
            error: None,
            requests: Mutex::new(Vec::new()),
        })
    }
    fn failing(error: fn() -> InferenceError) -> Arc<Self> {
        Arc::new(Self {
            text: String::new(),
            error: Some(error),
            requests: Mutex::new(Vec::new()),
        })
    }
}

impl InferencePort for ScriptedPort {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.requests.lock().unwrap().push((
            req.provider_id.clone(),
            req.fallback_provider_ids.clone(),
            req.prompt.clone(),
        ));
        if let Some(error) = self.error {
            return Err(error());
        }
        Ok(InferenceResponse {
            text: self.text.clone(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "anthropic".into(),
                endpoint_url: "test://scripted".into(),
                model_id: None,
            },
        })
    }
}

/// Records every scalar write; never halts.
#[derive(Default)]
struct RecordingScalarPort(Mutex<Vec<(String, f64)>>);

impl EpistemicScalarPort for RecordingScalarPort {
    fn write_scalar(
        &self,
        _spirit_pid: u32,
        _spirit_id: &str,
        tag: &str,
        value: f64,
        _derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        self.0.lock().unwrap().push((tag.to_owned(), value));
        Ok(None)
    }
}

struct Bound {
    butler: Butler,
    mints: Arc<AtomicUsize>,
    channel: Arc<Mutex<Option<serde_json::Value>>>,
    scalars: Arc<RecordingScalarPort>,
}

fn bound(port: Arc<ScriptedPort>) -> Bound {
    bound_on(port, conflict())
}

fn bound_on(port: Arc<ScriptedPort>, scenario: ScenarioInput) -> Bound {
    let mints = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&mints);
    let binding = Arc::new(Mutex::new(Some(ButlerInferenceBinding {
        provider_id: "anthropic".into(),
        mint: Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok((CapabilityToken::new(TokenId::ZERO, 41, 0, [0u8; 64]), 41))
        }),
    })));
    let channel = Arc::new(Mutex::new(None));
    let scalars = Arc::new(RecordingScalarPort::default());
    let butler = Butler::with_scenario(scenario)
        .with_output_channel(Arc::clone(&channel))
        .with_scalar_port(Arc::clone(&scalars) as Arc<dyn EpistemicScalarPort>)
        .with_deferred_inference_port(port, binding);
    Bound {
        butler,
        mints,
        channel,
        scalars,
    }
}

#[test]
fn a_complete_proposal_parses() {
    let proposal = parse_proposal(GOOD).expect("the contract parses");
    assert_eq!(proposal.pattern, "calendar-conflict");
    assert_eq!(proposal.options.len(), 3);
    assert!((proposal.user_preference_drift - 0.9).abs() < f32::EPSILON);
}

#[test]
fn a_missing_field_is_a_typed_refusal_naming_the_field() {
    for field in [
        "pattern",
        "confidence",
        "evidence",
        "options",
        "user_preference_drift",
    ] {
        let mut value: serde_json::Value = serde_json::from_str(GOOD).unwrap();
        value.as_object_mut().unwrap().remove(field);
        let err = parse_proposal(&value.to_string()).expect_err("a missing field must refuse");
        assert!(matches!(err, ButlerError::Proposal(_)), "{err:?}");
        assert!(err.to_string().contains(field), "{err} must name `{field}`");
    }
}

#[test]
fn ill_typed_or_out_of_range_values_refuse() {
    for text in [
        GOOD.replace("0.82", "\"high\""),
        GOOD.replace("0.82", "1.5"),
        GOOD.replace("0.9}", "-0.1}"),
        GOOD.replace(
            r#"["linear_note: note","reminder: remind","snooze: snooze"]"#,
            "[]",
        ),
        "I notice a pattern: prose is not a proposal".to_owned(),
    ] {
        let err = parse_proposal(&text).expect_err("must refuse");
        assert!(matches!(err, ButlerError::Proposal(_)), "{text}: {err:?}");
    }
}

#[test]
fn bound_pass_takes_the_proposal_from_the_completion() {
    let port = ScriptedPort::answering(GOOD);
    let b = bound(Arc::clone(&port));
    drive_on_idle(&b.butler);

    assert!(b.butler.take_pass_error().is_none());
    let notification = b.butler.last_notification().expect("a notification");
    assert_eq!(notification.pattern, "calendar-conflict");
    assert!((notification.confidence - 0.82).abs() < f32::EPSILON);
    assert_eq!(notification.evidence, "Board review overlaps Investor call");
    assert_eq!(
        notification.options,
        vec!["linear_note: note", "reminder: remind", "snooze: snooze"]
    );
    let emitted = b.channel.lock().unwrap().clone().expect("channel written");
    assert_eq!(emitted["options"], serde_json::json!(notification.options));

    let assessment = b.butler.last_assessment().expect("assessment stored");
    assert!((assessment.user_preference_drift - 0.9).abs() < f32::EPSILON);
    assert!(
        assessment.belief_variance > 0.7,
        "belief_variance stays arithmetic"
    );
    assert_eq!(
        b.scalars
            .0
            .lock()
            .unwrap()
            .first()
            .map(|(tag, _)| tag.clone()),
        Some("belief_variance".to_owned()),
        "the belief_variance scalar is written as today"
    );

    let requests = port.requests.lock().unwrap();
    assert_eq!(requests.len(), 1, "one complete per pass");
    assert_eq!(requests[0].0.as_deref(), Some("anthropic"));
    assert!(requests[0].1.is_empty(), "no fallback chain");
    assert!(
        requests[0].2.contains("evt-a"),
        "the prompt carries the scenario"
    );
}

#[test]
fn every_pass_mints_its_own_token() {
    let port = ScriptedPort::answering(GOOD);
    let b = bound(port);
    drive_on_idle(&b.butler);
    drive_on_idle(&b.butler);
    assert_eq!(b.mints.load(Ordering::SeqCst), 2, "one mint per pass");
}

#[test]
fn a_malformed_completion_refuses_the_pass_without_side_effects() {
    let port = ScriptedPort::answering(&GOOD.replace(
        r#","options":["linear_note: note","reminder: remind","snooze: snooze"]"#,
        "",
    ));
    let b = bound(port);
    drive_on_idle(&b.butler);
    let err = b.butler.take_pass_error().expect("the pass is refused");
    assert!(err.to_string().contains("options"), "{err}");
    assert!(b.butler.last_notification().is_none(), "no notification");
    assert!(
        b.channel.lock().unwrap().is_none(),
        "no output-channel write"
    );
    assert!(b.scalars.0.lock().unwrap().is_empty(), "no scalar write");
}

#[test]
fn an_inference_error_refuses_the_pass_typed() {
    let b = bound(ScriptedPort::failing(|| {
        InferenceError::MalformedResponse("cassette replay: exhausted".into())
    }));
    drive_on_idle(&b.butler);
    let err = b.butler.take_pass_error().expect("the pass is refused");
    assert!(matches!(err, ButlerError::Inference(_)), "{err:?}");
    assert!(b.butler.last_notification().is_none());
    assert!(b.scalars.0.lock().unwrap().is_empty());
}

#[test]
fn an_unready_binding_refuses_the_pass_typed() {
    let butler = Butler::with_scenario(conflict())
        .with_deferred_inference_port(ScriptedPort::answering(GOOD), Arc::new(Mutex::new(None)));
    drive_on_idle(&butler);
    let err = butler.take_pass_error().expect("the pass is refused");
    assert!(matches!(err, ButlerError::Unbound(_)), "{err:?}");
}

#[test]
fn the_deterministic_path_emits_options() {
    let channel = Arc::new(Mutex::new(None));
    let butler = Butler::with_scenario(conflict()).with_output_channel(Arc::clone(&channel));
    drive_on_idle(&butler);
    assert!(!butler.is_inference_bound());
    let notification = butler.last_notification().expect("a notification");
    assert_eq!(notification.options, DETERMINISTIC_OPTIONS.to_vec());
    let emitted = channel.lock().unwrap().clone().expect("channel written");
    for field in ["pattern", "confidence", "evidence", "options"] {
        assert!(emitted.get(field).is_some(), "missing `{field}`");
    }
}

#[test]
fn the_post_pass_hook_runs_after_every_pass_refused_or_not() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    let butler = Butler::with_scenario(conflict())
        .with_deferred_inference_port(ScriptedPort::answering("{}"), Arc::new(Mutex::new(None)))
        .with_post_pass_hook(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
    drive_on_idle(&butler);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

// ── Story 18-1 code review (2026-10-25) ─────────────────────────────────────

/// Review EC-6: a blank `pattern`, `evidence` or option is a typed refusal,
/// not a shape-valid notification.
#[test]
fn blank_fields_are_refused() {
    for (field, text) in [
        ("pattern", GOOD.replace("\"calendar-conflict\"", "\"  \"")),
        (
            "evidence",
            GOOD.replace("\"Board review overlaps Investor call\"", "\"\""),
        ),
        ("options", GOOD.replace("\"reminder: remind\"", "\" \"")),
    ] {
        let err = parse_proposal(&text).expect_err("a blank field must refuse");
        assert!(matches!(err, ButlerError::Proposal(_)), "{err:?}");
        assert!(err.to_string().contains(field), "{err} must name `{field}`");
    }
}

/// Story 18-4 (D-18-4-03 A, AC4.1) — an option naming an unknown action, no
/// action, or an action already offered is a typed refusal: the director is
/// never shown an option whose effect Butler cannot run as declared.
#[test]
fn options_outside_the_closed_vocabulary_are_refused() {
    for (case, text, says) in [
        (
            "unknown",
            GOOD.replace("reminder: remind", "email_ceo: remind"),
            "unknown action",
        ),
        (
            "missing",
            GOOD.replace("reminder: remind", "remind"),
            "names no action",
        ),
        (
            "duplicate",
            GOOD.replace("reminder: remind", "snooze: again"),
            "twice",
        ),
        (
            "blank text",
            GOOD.replace("reminder: remind", "reminder:  "),
            "no text",
        ),
    ] {
        let err = parse_proposal(&text).expect_err(case);
        assert!(matches!(err, ButlerError::Proposal(_)), "{case}: {err:?}");
        assert!(err.to_string().contains(says), "{case}: {err}");
    }
    let port = ScriptedPort::answering(&GOOD.replace("reminder: remind", "email_ceo: remind"));
    let b = bound(port);
    drive_on_idle(&b.butler);
    assert!(b.butler.take_pass_error().is_some(), "the pass is refused");
    assert!(
        b.butler.last_notification().is_none(),
        "nothing to pick from"
    );
}

/// Story 18-4 (D-18-4-03 A) — the prompt declares the closed vocabulary.
#[test]
fn the_prompt_declares_the_option_actions() {
    let port = ScriptedPort::answering(GOOD);
    let b = bound(Arc::clone(&port));
    drive_on_idle(&b.butler);
    let requests = port.requests.lock().unwrap();
    assert!(
        requests[0]
            .2
            .contains("one of linear_note, reminder, snooze, each at most once"),
        "{}",
        requests[0].2
    );
}

/// Review BH-1/EC-5: the prompt states which end of the drift scale is good.
#[test]
fn the_prompt_states_the_drift_scale() {
    let port = ScriptedPort::answering(GOOD);
    let b = bound(Arc::clone(&port));
    drive_on_idle(&b.butler);
    let requests = port.requests.lock().unwrap();
    assert!(
        requests[0]
            .2
            .contains("1.0 = fully aligned with the director's preferences; lower = more drift"),
        "the prompt must state the scale: {}",
        requests[0].2
    );
}

/// Review ruling D-18-1-R6 + TIA-10: a bound pass with no conflict still
/// calls the model — its `user_preference_drift` is the scalar written — but
/// stores and emits no notification (as the deterministic path).
#[test]
fn a_bound_no_conflict_pass_writes_the_models_drift_and_emits_nothing() {
    let calm = ScenarioInput {
        calendar: vec![CalendarEvent {
            id: "evt-solo".into(),
            title: "Standup".into(),
            start_min: 540,
            end_min: 560,
            status: EventStatus::Confirmed,
        }],
        ..Default::default()
    };
    let port = ScriptedPort::answering(&GOOD.replace("0.9}", "0.42}"));
    let b = bound_on(Arc::clone(&port), calm);
    drive_on_idle(&b.butler);
    assert!(b.butler.take_pass_error().is_none());
    assert_eq!(
        port.requests.lock().unwrap().len(),
        1,
        "the model is called"
    );
    assert!(b.butler.last_notification().is_none(), "no notification");
    assert!(b.channel.lock().unwrap().is_none(), "nothing emitted");
    assert_eq!(
        b.scalars.0.lock().unwrap().clone(),
        vec![("user_preference_drift".to_owned(), f64::from(0.42f32))],
        "the written drift scalar is the completion's value"
    );
}

/// Review BH-3: a refused pass clears the previous pass's notification and
/// output channel, so nothing stale stays pending.
#[test]
fn a_refused_pass_clears_the_previous_notification() {
    let b = bound(ScriptedPort::answering(GOOD));
    drive_on_idle(&b.butler);
    assert!(b.butler.last_notification().is_some());
    assert!(b.channel.lock().unwrap().is_some());

    let binding = Arc::new(Mutex::new(Some(ButlerInferenceBinding {
        provider_id: "anthropic".into(),
        mint: Arc::new(|| Ok((CapabilityToken::new(TokenId::ZERO, 41, 0, [0u8; 64]), 41))),
    })));
    let refusing = b.butler.clone().with_deferred_inference_port(
        ScriptedPort::failing(|| InferenceError::MalformedResponse("exhausted".into())),
        binding,
    );
    drive_on_idle(&refusing);
    assert!(refusing.take_pass_error().is_some(), "the pass is refused");
    assert!(
        b.butler.last_notification().is_none(),
        "the stale notification is cleared"
    );
    assert!(
        b.channel.lock().unwrap().is_none(),
        "the stale output is cleared"
    );
}

/// Review TIA-5: the only production combination — a bound Butler with an
/// MCP port. The MCP scenario reaches the prompt, the arithmetic
/// `belief_variance` is written, and the notification is the completion's.
#[test]
fn a_bound_butler_reasons_over_the_mcp_scenario() {
    struct FakeMcp;
    #[async_trait::async_trait]
    impl ButlerMcpPort for FakeMcp {
        async fn calendar_events(&self) -> Result<Vec<CalendarEvent>, ButlerMcpError> {
            let event = |id: &str, start, end| CalendarEvent {
                id: id.into(),
                title: id.into(),
                start_min: start,
                end_min: end,
                status: EventStatus::Confirmed,
            };
            Ok(vec![event("mcp-a", 540, 600), event("mcp-b", 570, 630)])
        }
        async fn comms_messages(&self) -> Result<Vec<butler::CommsMessage>, ButlerMcpError> {
            Ok(Vec::new())
        }
        async fn write_linear_note(&self, _: &str, _: &str) -> Result<(), ButlerMcpError> {
            Ok(())
        }
        async fn fetch_figma_summary(&self) -> Result<serde_json::Value, ButlerMcpError> {
            Ok(serde_json::json!({}))
        }
    }
    let port = ScriptedPort::answering(GOOD);
    let b = bound_on(Arc::clone(&port), ScenarioInput::default());
    let butler = b
        .butler
        .clone()
        .with_mcp_port(Arc::new(FakeMcp) as Arc<dyn ButlerMcpPort>);
    drive_on_idle(&butler);
    assert!(butler.take_pass_error().is_none());
    let prompt = port.requests.lock().unwrap()[0].2.clone();
    assert!(
        prompt.contains("mcp-a") && prompt.contains("mcp-b"),
        "the MCP scenario reaches the prompt: {prompt}"
    );
    assert_eq!(
        b.scalars
            .0
            .lock()
            .unwrap()
            .first()
            .map(|(tag, _)| tag.clone()),
        Some("belief_variance".to_owned()),
        "the arithmetic belief_variance over the MCP conflict is written"
    );
    assert_eq!(
        butler.last_notification().expect("a notification").evidence,
        "Board review overlaps Investor call",
        "the notification is the completion's"
    );
}

fn seam(
    registered: &[&str],
    mints: Arc<Mutex<Vec<(u32, String)>>>,
    port: Arc<ScriptedPort>,
) -> ButlerSeam {
    ButlerSeam {
        inference: Some((
            Arc::new(move || Some(Arc::clone(&port) as Arc<dyn InferencePort + Send + Sync>)),
            registered.iter().map(|id| (*id).to_owned()).collect(),
        )),
        mint: Arc::new(move |pid, provider: &str| {
            mints.lock().unwrap().push((pid, provider.to_owned()));
            Ok(CapabilityToken::new(TokenId::ZERO, pid, 0, [0u8; 64]))
        }),
        digest: Arc::new(MorningDigestRenderer::new(
            "unused-tl".into(),
            "unused-journal".into(),
            Arc::new(|_, _| {}),
            Arc::new(SystemClock::UTC),
        )),
        rollover: false,
        clock: Arc::new(SystemClock::UTC),
        acceptance: None,
        scene_butler: Arc::default(),
    }
}

/// Review ruling D-18-1-R1 — the seam every host Butler attaches binds the
/// port, names the declared provider and mints per pass at the pid the host
/// resolves (a successor's, a door-loaded Butler's, or the boot pid).
#[test]
fn the_seam_binds_the_declared_provider_at_the_resolved_pid() {
    let mints = Arc::new(Mutex::new(Vec::new()));
    let port = ScriptedPort::answering(GOOD);
    let butler = seam(
        &["anthropic", "ollama"],
        Arc::clone(&mints),
        Arc::clone(&port),
    )
    .attach(
        Butler::with_scenario(conflict()),
        || Some("anthropic".into()),
        Arc::new(|| Some(77)),
    )
    .expect("a registered declared provider binds");
    assert!(butler.is_inference_bound());
    drive_on_idle(&butler);
    drive_on_idle(&butler);
    assert_eq!(
        mints.lock().unwrap().clone(),
        vec![(77, "anthropic".to_owned()), (77, "anthropic".to_owned())],
        "one mint per pass, at the resolved pid, for the declared provider"
    );
    assert_eq!(
        port.requests.lock().unwrap()[0].0.as_deref(),
        Some("anthropic")
    );
}

/// Review ruling D-18-1-R2 — a declared provider the router does not
/// register is refused when the Butler is built (boot or load), typed.
#[test]
fn the_seam_refuses_an_unregistered_declared_provider() {
    let err = seam(&["ollama"], Arc::default(), ScriptedPort::answering(GOOD))
        .attach(
            Butler::with_scenario(conflict()),
            || Some("anthropic".into()),
            Arc::new(|| Some(77)),
        )
        .expect_err("an unregistered declared provider must refuse");
    assert!(
        err.contains("no inference provider is configured") && err.contains("anthropic"),
        "{err}"
    );
}

// ── Story 18-8 AC5 (review ruling D-18-8-R4) — both MCP fetches failing ──

/// An MCP port whose calendar and comms fetches answer as scripted.
struct ScriptedMcp {
    calendar: fn() -> Result<Vec<CalendarEvent>, ButlerMcpError>,
    comms: fn() -> Result<Vec<butler::CommsMessage>, ButlerMcpError>,
}

#[async_trait::async_trait]
impl ButlerMcpPort for ScriptedMcp {
    async fn calendar_events(&self) -> Result<Vec<CalendarEvent>, ButlerMcpError> {
        (self.calendar)()
    }
    async fn comms_messages(&self) -> Result<Vec<butler::CommsMessage>, ButlerMcpError> {
        (self.comms)()
    }
    async fn write_linear_note(&self, _: &str, _: &str) -> Result<(), ButlerMcpError> {
        Ok(())
    }
    async fn fetch_figma_summary(&self) -> Result<serde_json::Value, ButlerMcpError> {
        Ok(serde_json::json!({}))
    }
}

fn failed(server: &str) -> ButlerMcpError {
    ButlerMcpError::CallFailed {
        server: server.into(),
        tool: "t".into(),
        cause: maos_domain::ports::mcp::McpError::Transport("unauthorized (401)".into()),
    }
}

fn unconfigured() -> ButlerMcpError {
    ButlerMcpError::CallFailed {
        server: "slack".into(),
        tool: "list_messages".into(),
        cause: maos_domain::ports::mcp::McpError::Unconfigured,
    }
}

/// A bound Butler over `mcp`, after one pass, with its model port.
fn bound_over(mcp: ScriptedMcp) -> (Bound, Arc<ScriptedPort>) {
    let port = ScriptedPort::answering(GOOD);
    let mut b = bound_on(Arc::clone(&port), ScenarioInput::default());
    b.butler = b.butler.clone().with_mcp_port(Arc::new(mcp));
    drive_on_idle(&b.butler);
    (b, port)
}

#[test]
fn both_mcp_fetches_failing_refuses_the_pass_before_any_model_call() {
    let (b, port) = bound_over(ScriptedMcp {
        calendar: || Err(failed("calendar")),
        comms: || Err(failed("slack")),
    });
    let err = b.butler.take_pass_error().expect("the pass is refused");
    assert!(
        matches!(err, ButlerError::McpUnavailable { .. }),
        "typed MCP refusal: {err:?}"
    );
    assert!(err.to_string().contains("MCP data unavailable"), "{err}");
    assert!(port.requests.lock().unwrap().is_empty(), "zero model calls");
    assert_eq!(b.mints.load(Ordering::SeqCst), 0, "no token minted");
    assert!(b.butler.last_notification().is_none(), "no notification");
    assert!(
        b.channel.lock().unwrap().is_none(),
        "no output-channel write"
    );
    assert!(b.scalars.0.lock().unwrap().is_empty(), "no scalar write");
}

/// "Both" is two real failures: one unconfigured server plus one failed
/// fetch, or one failure beside data, proceeds on the partial data.
#[test]
fn a_partial_mcp_failure_is_not_both_and_the_pass_proceeds() {
    let partial: [ScriptedMcp; 3] = [
        ScriptedMcp {
            calendar: || Err(failed("calendar")),
            comms: || Err(unconfigured()),
        },
        ScriptedMcp {
            calendar: || Ok(Vec::new()),
            comms: || Err(failed("slack")),
        },
        ScriptedMcp {
            calendar: || Err(unconfigured()),
            comms: || Err(unconfigured()),
        },
    ];
    for mcp in partial {
        let (b, port) = bound_over(mcp);
        assert!(b.butler.take_pass_error().is_none(), "not refused");
        assert_eq!(
            port.requests.lock().unwrap().len(),
            1,
            "the model is called"
        );
    }
}

/// The deterministic (unbound) path is unchanged: no refusal on MCP failure.
#[test]
fn an_unbound_butler_does_not_refuse_on_mcp_failure() {
    let butler = Butler::with_scenario(conflict()).with_mcp_port(Arc::new(ScriptedMcp {
        calendar: || Err(failed("calendar")),
        comms: || Err(failed("slack")),
    }));
    drive_on_idle(&butler);
    assert!(butler.take_pass_error().is_none());
}
