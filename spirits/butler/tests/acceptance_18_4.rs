#![forbid(unsafe_code)]

//! Story 18-4 — Butler's acceptance record and option effects, at library
//! level.
//!
//! - AC4.2 (D-18-4-03/04 A; `deferred-work.md:1233`): a pick runs the action
//!   the picked option DECLARES, whatever its position — a completion listing
//!   `snooze` first and `linear_note` second: pick 2 writes the Linear note
//!   with the model's text through a scripted `ButlerMcpPort`, pick 1 snoozes
//!   at the clock + 15 min; an out-of-range pick consumes nothing; with no
//!   MCP port a `linear_note` pick is accepted with its effect refused, typed.
//! - AC1 (D-18-4-01 A): once the 7-day window holds 7 decided offers,
//!   `belief_variance` is H₂ of the pooled rate; before that the conflict
//!   proxy stands.
//! - AC3 (D-18-4-02 A, CR-4): the record is written through the pid-fused
//!   principal port and restored at the pid the host resolves — a Butler at
//!   another pid (a door reload) restores 0 records; each decision's TL
//!   attestation names no work mode or response and lands BEFORE its effect.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use butler::__maos_spirit_vtable_Butler;
use butler::{
    AcceptanceFixture, AcceptanceSeam, Butler, ButlerInferenceBinding, ButlerMcpError,
    ButlerMcpPort, ButlerSeam, CalendarEvent, CommsMessage, EventStatus, MorningDigestRenderer,
    OptionAction, ScenarioInput,
};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;
use maos_spirit_sdk::clock::{Clock, ScenarioClock, NANOS_PER_DAY};
use maos_spirit_sdk::principal::PrincipalMemoryPort;
use maos_spirit_sdk::spirit_test::SpiritTest;

/// `snooze` FIRST, `linear_note` SECOND — position and action disagree.
const REORDERED: &str = r#"{"pattern":"calendar-conflict","confidence":0.82,"evidence":"Board review overlaps Investor call","options":["snooze: Revisit after the board review","linear_note: Track the board/investor clash in Linear"],"user_preference_drift":0.9}"#;

const DAY0: u64 = 20_514; // 2026-03-02

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
    harness.run();
}

struct Scripted(String);

impl InferencePort for Scripted {
    fn complete(&self, _req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        Ok(InferenceResponse {
            text: self.0.clone(),
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

/// Records every Linear write (and its order in `log`).
struct ScriptedMcp {
    notes: Mutex<Vec<(String, String)>>,
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl ButlerMcpPort for ScriptedMcp {
    async fn calendar_events(&self) -> Result<Vec<CalendarEvent>, ButlerMcpError> {
        Ok(vec![])
    }
    async fn comms_messages(&self) -> Result<Vec<CommsMessage>, ButlerMcpError> {
        Ok(vec![])
    }
    async fn write_linear_note(&self, title: &str, content: &str) -> Result<(), ButlerMcpError> {
        self.log.lock().unwrap().push("effect".into());
        self.notes
            .lock()
            .unwrap()
            .push((title.to_owned(), content.to_owned()));
        Ok(())
    }
    async fn fetch_figma_summary(&self) -> Result<serde_json::Value, ButlerMcpError> {
        Ok(serde_json::Value::Null)
    }
}

fn scripted_mcp(log: &Arc<Mutex<Vec<String>>>) -> ScriptedMcp {
    ScriptedMcp {
        notes: Mutex::new(Vec::new()),
        log: Arc::clone(log),
    }
}

fn bound_butler(completion: &str, clock: &Arc<ScenarioClock>) -> Butler {
    let binding = Arc::new(Mutex::new(Some(ButlerInferenceBinding {
        provider_id: "anthropic".into(),
        mint: Arc::new(|| Ok((CapabilityToken::new(TokenId::ZERO, 41, 0, [0u8; 64]), 41))),
    })));
    Butler::with_scenario(conflict())
        .with_deferred_inference_port(Arc::new(Scripted(completion.into())), binding)
        .with_clock(Arc::clone(clock) as Arc<dyn Clock>)
}

/// AC4.2 — pick 2 is `linear_note` by declaration, though second in place.
#[test]
fn a_pick_runs_the_action_the_option_declares_not_its_position() {
    let clock = Arc::new(ScenarioClock::new(
        DAY0 * NANOS_PER_DAY + 8 * 3_600_000_000_000,
        0,
    ));
    let butler = bound_butler(REORDERED, &clock);
    let log = Arc::new(Mutex::new(Vec::new()));
    let mcp = scripted_mcp(&log);

    drive_on_idle(&butler);
    let out_of_range = butler.handle_option_pick(3, "deep", Some(&mcp));
    assert!(
        matches!(
            out_of_range,
            Err(ButlerMcpError::PickOutOfRange {
                pick: 3,
                options: 2
            })
        ),
        "{out_of_range:?}"
    );
    assert!(
        butler.last_notification().is_some(),
        "an out-of-range pick consumes nothing"
    );
    assert!(butler.acceptance_window().offers == 0 && mcp.notes.lock().unwrap().is_empty());

    let note = butler.handle_option_pick(2, "deep", Some(&mcp)).unwrap();
    assert_eq!(note.action, OptionAction::LinearNote);
    assert!(note.linear_note_written && note.accepted);
    assert_eq!(note.effect, "written");
    let notes = mcp.notes.lock().unwrap().clone();
    assert_eq!(notes.len(), 1, "pick 2 writes exactly one Linear note");
    assert_eq!(
        notes[0].1, "Track the board/investor clash in Linear",
        "the note carries the model's text for the option the director picked"
    );
    assert!(
        butler.last_notification().is_none(),
        "the pick consumed the notification"
    );

    drive_on_idle(&butler);
    let snooze = butler.handle_option_pick(1, "shallow", Some(&mcp)).unwrap();
    assert_eq!(snooze.action, OptionAction::Snooze);
    assert!(snooze.snoozed && !snooze.accepted && !snooze.linear_note_written);
    assert_eq!(
        snooze.snooze_until_ns,
        Some(clock.now_ns() + 15 * 60 * 1_000_000_000),
        "the snooze re-checks at the clock + 15 min"
    );
    assert!(snooze.message.contains("08:15 UTC"), "{}", snooze.message);
    assert_eq!(
        mcp.notes.lock().unwrap().len(),
        1,
        "a snooze writes nothing"
    );
}

/// AC4.2 / D-18-4-04 A — no MCP port: the pick is accepted, the effect is
/// refused typed, never "written".
#[test]
fn a_linear_note_pick_without_an_mcp_port_is_refused_typed() {
    let clock = Arc::new(ScenarioClock::new(DAY0 * NANOS_PER_DAY, 0));
    let butler = Butler::with_scenario(conflict()).with_clock(Arc::clone(&clock) as Arc<dyn Clock>);
    drive_on_idle(&butler);
    let outcome = butler.handle_option_pick(1, "deep", None).unwrap();
    assert_eq!(outcome.action, OptionAction::LinearNote);
    assert!(outcome.accepted && !outcome.linear_note_written);
    assert_eq!(outcome.effect, "refused (mcp unconfigured)");
    let reminder = {
        drive_on_idle(&butler);
        butler.handle_option_pick(2, "deep", None).unwrap()
    };
    assert_eq!(
        reminder.effect, "stub (live send v0.4)",
        "the v0.4 stub is labelled"
    );
}

fn fixture(lines: &[(i64, &str, &str)]) -> AcceptanceFixture {
    let mut text =
        String::from(r#"{"principal":"director-ada","start":"2026-03-02","utc_offset":"+00:00"}"#);
    for (day, mode, response) in lines {
        text.push_str(&format!(
            "\n{{\"day\":{day},\"work_mode\":\"{mode}\",\"response\":\"{response}\"}}"
        ));
    }
    AcceptanceFixture::parse(&text).unwrap()
}

/// AC1 (D-18-4-01 A) — the window takes over once it holds 7 decisions.
#[test]
fn the_window_drives_belief_variance_once_it_holds_seven_offers() {
    let all_accepted: Vec<(i64, &str, &str)> = (-6..=0).map(|d| (d, "deep", "pick:1")).collect();
    let fx = fixture(&all_accepted);
    let clock = Arc::new(ScenarioClock::new(fx.day_ns(1), 0));
    let butler = Butler::with_scenario(conflict()).with_clock(Arc::clone(&clock) as Arc<dyn Clock>);
    let cold = butler.assess(&conflict());
    assert!(
        cold.acceptance.is_none() && cold.belief_variance > 0.7,
        "conflict proxy"
    );

    butler.seed_acceptance_history(&fx).unwrap();
    let warm = butler.assess(&conflict());
    let window = warm
        .acceptance
        .clone()
        .expect("the window informs the scalar");
    assert_eq!((window.offers, window.accepted), (7, 7));
    assert_eq!(
        warm.belief_variance, 0.0,
        "a director who always accepts is certain"
    );
    let (tag, value, derived_from) = warm.primary_scalar();
    assert_eq!((tag, value), ("belief_variance", 0.0));
    assert_eq!(derived_from, "acceptance-window:7/7");

    // Two declines among seven: the pooled rate 5/7 ≥ 0.6 but H₂ > 0.7 —
    // the halt leads the falling rate (PRD Journey B :39).
    let mixed: Vec<(i64, &str, &str)> = (-6..=0)
        .map(|d| match d {
            -3 | -1 => (d, "shallow", "pick:3"),
            _ => (d, "deep", "pick:1"),
        })
        .collect();
    let butler = Butler::with_scenario(conflict()).with_clock(Arc::clone(&clock) as Arc<dyn Clock>);
    butler.seed_acceptance_history(&fixture(&mixed)).unwrap();
    let assessment = butler.assess(&conflict());
    let window = assessment.acceptance.expect("informed");
    assert!(window.rate >= 0.6, "{}", window.rate);
    assert!(
        assessment.belief_variance > 0.7,
        "{}",
        assessment.belief_variance
    );
    assert_eq!(window.by_mode["shallow"].rate, 0.0);
    assert_eq!(window.by_mode["deep"].rate, 1.0);
}

/// An in-memory principal store keyed by the FUSED pid.
#[derive(Default)]
struct Store(Mutex<HashMap<(u32, String, String, String), String>>);

struct FusedPort {
    pid: u32,
    store: Arc<Store>,
}

impl PrincipalMemoryPort for FusedPort {
    fn pid(&self) -> u32 {
        self.pid
    }
    fn write(&self, principal: &str, schema: &str, key: &str, md: &str) -> Result<(), String> {
        let k = (self.pid, principal.into(), schema.into(), key.into());
        self.store.0.lock().unwrap().insert(k, md.into());
        Ok(())
    }
    fn read(&self, principal: &str, schema: &str, key: &str) -> Result<Option<String>, String> {
        let k = (self.pid, principal.into(), schema.into(), key.into());
        Ok(self.store.0.lock().unwrap().get(&k).cloned())
    }
}

fn seam(
    store: &Arc<Store>,
    log: &Arc<Mutex<Vec<String>>>,
    clock: &Arc<ScenarioClock>,
) -> ButlerSeam {
    let (store, log) = (Arc::clone(store), Arc::clone(log));
    ButlerSeam {
        inference: None,
        mint: Arc::new(|pid, _: &str| Ok(CapabilityToken::new(TokenId::ZERO, pid, 0, [0u8; 64]))),
        digest: Arc::new(MorningDigestRenderer::new(
            "unused".into(),
            "unused".into(),
            Arc::new(|_, _| {}),
            Arc::clone(clock) as Arc<dyn Clock>,
        )),
        rollover: false,
        clock: Arc::clone(clock) as Arc<dyn Clock>,
        acceptance: Some(AcceptanceSeam {
            principal: "director-ada".into(),
            port_for: Arc::new(move |pid| {
                Arc::new(FusedPort {
                    pid,
                    store: Arc::clone(&store),
                }) as Arc<dyn PrincipalMemoryPort>
            }),
            attest: Arc::new(move |_, payload| {
                log.lock()
                    .unwrap()
                    .push(format!("attest {}", String::from_utf8_lossy(payload)));
                Ok(())
            }),
        }),
    }
}

/// AC3 (D-18-4-02 A, CR-4) — restored at the resolved pid; another pid (a
/// door reload) restores nothing; attestations are non-personal and precede
/// the effect.
#[test]
fn the_record_restores_at_the_resolved_pid_and_attests_before_the_effect() {
    let all_accepted: Vec<(i64, &str, &str)> = (-6..=0).map(|d| (d, "deep", "pick:1")).collect();
    let fx = fixture(&all_accepted);
    let clock = Arc::new(ScenarioClock::new(fx.day_ns(1), 0));
    let (store, log) = (Arc::new(Store::default()), Arc::new(Mutex::new(Vec::new())));
    let seam = seam(&store, &log, &clock);
    let pid = Arc::new(AtomicU32::new(1));
    let pid_of = {
        let pid = Arc::clone(&pid);
        Arc::new(move || Some(pid.load(Ordering::SeqCst)))
    };

    let first = seam
        .attach(Butler::with_scenario(conflict()), || None, pid_of.clone())
        .unwrap();
    first.seed_acceptance_history(&fx).unwrap();
    drive_on_idle(&first);
    let mcp = scripted_mcp(&log);
    first.handle_option_pick(1, "deep", Some(&mcp)).unwrap();
    let log_now = log.lock().unwrap().clone();
    assert_eq!(log_now.len(), 2, "{log_now:?}");
    assert!(
        log_now[0].starts_with("attest "),
        "the attestation precedes the effect: {log_now:?}"
    );
    assert_eq!(log_now[1], "effect");
    for personal in ["deep", "pick:1", "work_mode", "response", "Board"] {
        assert!(
            !log_now[0].contains(personal),
            "the TL attestation names `{personal}`: {}",
            log_now[0]
        );
    }
    assert!(
        log_now[0].contains("\"action\":\"linear_note\""),
        "{}",
        log_now[0]
    );
    let stored = store.0.lock().unwrap();
    let markdown = stored
        .get(&(
            1,
            "director-ada".into(),
            "butler.acceptance/v1".into(),
            "ledger".into(),
        ))
        .expect("the record is in the principal namespace at the fused pid");
    assert_eq!(markdown.lines().filter(|l| l.starts_with('{')).count(), 8);
    assert!(
        markdown.contains("\"work_mode\":\"deep\""),
        "the personal record holds the mode"
    );
    drop(stored);

    // A restart at the same pid restores all 8 records before its first pass.
    clock.set_ns(fx.day_ns(2));
    let resumed = seam
        .attach(Butler::with_scenario(conflict()), || None, pid_of.clone())
        .unwrap();
    drive_on_idle(&resumed);
    assert_eq!(resumed.acceptance_window().offers, 7, "days -5..1 restored");

    // A door reload allocates a new pid: the record is not visible there.
    pid.store(2, Ordering::SeqCst);
    let reloaded = seam
        .attach(Butler::with_scenario(conflict()), || None, pid_of)
        .unwrap();
    drive_on_idle(&reloaded);
    assert_eq!(
        reloaded.acceptance_window().offers,
        0,
        "records: 0 at the new pid"
    );
}
