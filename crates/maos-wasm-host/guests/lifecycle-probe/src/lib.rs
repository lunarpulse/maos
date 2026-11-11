//! Scripted `maos:spirit@2.4.0` lifecycle consumer for real runner/session tests.
//!
//! It exports `lifecycle` beside the base world and counts every trigger. The state it
//! snapshots is its frame counter (8 bytes, little-endian); `swap-in` restores it and
//! `migrate` returns it plus 1000, so a successor that continues from `N + 1000` has seen
//! its predecessor's state through the migrator.
//!
//! Input script is UTF-8 in TaskAssign.goal or TelemetryEvent.data, one command per line:
//! - `report` (also the default for an empty script): emit the counts below.
//! - `sleep SECONDS`: this turn stays busy that long (a swap waits for the turn boundary).
//! - `sleep-idle SECONDS`: the next `idle` sleeps that long (a budget-overrun probe).
//! - `fail NAME` / `decline NAME`: the next call of that lifecycle export returns
//!   `failed` / `declined`. NAME is idle, consolidate, telemetry, schedule, pause, resume,
//!   snapshot, swap-in or migrate.
//! - `to SPIRIT`: address the report to this Spirit.
//!
//! Output is one TelemetryEvent (`event-type = lifecycle.probe`) whose data is:
//! `counts idle=I consolidate=C telemetry=T schedule=S pause=P resume=R frames=F`,
//! `state counter=N swap-in=N|- migrate=N|-`, `imports NAME=ok|unavailable ...` (whether
//! the `recall` import answered inside each export) and
//! `last telemetry=TAG@SEQ|- schedule=TEXT|- consolidate=LEN|-`.
wit_bindgen::generate!({ path: "../../../../wit/spirit.wit", world: "spirit-lifecycle" });

use exports::maos::spirit::lifecycle::{Guest as LifecycleGuest, LifecycleError, TelemetryEvent};
use maos::spirit::frames::{FrameKind, FramePayload, TelemetryEventBody};
use maos::spirit::recall::{self, RecallError, RecallFilter};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Default)]
struct State {
    idle: u32,
    consolidate: u32,
    telemetry: u32,
    schedule: u32,
    pause: u32,
    resume: u32,
    frames: u64,
    counter: u64,
    swapped_in: Option<u64>,
    migrated: Option<u64>,
    sleep_idle: u64,
    fail: Vec<(String, bool)>,
    imports: BTreeMap<&'static str, bool>,
    last_telemetry: Option<String>,
    last_schedule: Option<String>,
    last_consolidate: Option<usize>,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

struct LifecycleProbe;

fn imports_answered(name: &'static str) {
    let filter = RecallFilter {
        kind: None,
        since_ns: None,
        until_ns: None,
        limit: 1,
        cursor: None,
        intent_filter: None,
    };
    let answered = !matches!(recall::recall(&filter), Err(RecallError::Unavailable));
    STATE.with(|state| state.borrow_mut().imports.insert(name, answered));
}

/// A scripted refusal for `name`, consumed once.
fn scripted(name: &str) -> Result<(), LifecycleError> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        match state.fail.iter().position(|(n, _)| n == name) {
            Some(at) => {
                let (_, failed) = state.fail.remove(at);
                let reason = format!("scripted {name}");
                Err(if failed {
                    LifecycleError::Failed(reason)
                } else {
                    LifecycleError::Declined(reason)
                })
            }
            None => Ok(()),
        }
    })
}

fn bump(name: &'static str, count: impl FnOnce(&mut State) -> &mut u32) -> Result<(), LifecycleError> {
    STATE.with(|state| *count(&mut state.borrow_mut()) += 1);
    scripted(name)
}

/// Stay busy for `seconds` of the monotonic clock (a poll-based sleep is not available to a
/// guest under the runner; fuel bounds this).
fn busy(seconds: u64) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < until {
        std::hint::black_box(0);
    }
}

fn counter_bytes(counter: u64) -> Vec<u8> {
    counter.to_le_bytes().to_vec()
}

fn counter_of(state: &[u8]) -> Result<u64, LifecycleError> {
    <[u8; 8]>::try_from(state)
        .map(u64::from_le_bytes)
        .map_err(|_| LifecycleError::Failed(format!("state is {} bytes, not 8", state.len())))
}

fn show(value: Option<u64>) -> String {
    value.map_or_else(|| "-".into(), |n| n.to_string())
}

fn report() -> String {
    STATE.with(|state| {
        let s = state.borrow();
        let mut out = String::new();
        let _ = writeln!(
            out,
            "counts idle={} consolidate={} telemetry={} schedule={} pause={} resume={} frames={}",
            s.idle, s.consolidate, s.telemetry, s.schedule, s.pause, s.resume, s.frames
        );
        let _ = writeln!(
            out,
            "state counter={} swap-in={} migrate={}",
            s.counter,
            show(s.swapped_in),
            show(s.migrated)
        );
        out.push_str("imports");
        for (name, answered) in &s.imports {
            let _ = write!(out, " {name}={}", if *answered { "ok" } else { "unavailable" });
        }
        out.push('\n');
        let _ = writeln!(
            out,
            "last telemetry={} schedule={} consolidate={}",
            s.last_telemetry.as_deref().unwrap_or("-"),
            s.last_schedule.as_deref().unwrap_or("-"),
            show(s.last_consolidate.map(|len| len as u64))
        );
        out
    })
}

impl Guest for LifecycleProbe {
    fn handle_frame(mut frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        let script = match &frame.payload {
            FramePayload::TaskAssign(body) => body.goal.clone(),
            FramePayload::TelemetryEvent(body) => body.data.clone(),
            _ => String::new(),
        };
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.frames += 1;
            state.counter += 1;
        });
        for line in script.lines().filter(|line| !line.trim().is_empty()) {
            let mut words = line.split_whitespace();
            match (words.next(), words.next()) {
                (Some("sleep"), Some(seconds)) => {
                    busy(seconds.parse().unwrap_or(0));
                }
                (Some("sleep-idle"), Some(seconds)) => {
                    let seconds = seconds.parse().unwrap_or(0);
                    STATE.with(|state| state.borrow_mut().sleep_idle = seconds);
                }
                (Some(verb @ ("fail" | "decline")), Some(name)) => STATE.with(|state| {
                    state.borrow_mut().fail.push((name.to_owned(), verb == "fail"));
                }),
                (Some("to"), Some(id)) => {
                    frame.to = vec![maos::spirit::frames::FrameAddress {
                        spirit_id: id.into(),
                        host_id: None,
                        role: None,
                    }];
                }
                _ => {}
            }
        }
        frame.kind = FrameKind::TelemetryEvent;
        frame.payload = FramePayload::TelemetryEvent(TelemetryEventBody {
            event_type: "lifecycle.probe".into(),
            data: report(),
        });
        Ok(vec![frame])
    }
    fn on_start() -> Result<(), Halt> {
        Ok(())
    }
    fn on_shutdown() {}
}

impl LifecycleGuest for LifecycleProbe {
    fn idle() -> Result<(), LifecycleError> {
        imports_answered("idle");
        let sleep = STATE.with(|state| std::mem::take(&mut state.borrow_mut().sleep_idle));
        if sleep > 0 {
            busy(sleep);
        }
        bump("idle", |s| &mut s.idle)
    }
    fn consolidate(batch: Vec<u8>) -> Result<(), LifecycleError> {
        imports_answered("consolidate");
        STATE.with(|state| state.borrow_mut().last_consolidate = Some(batch.len()));
        bump("consolidate", |s| &mut s.consolidate)
    }
    fn telemetry(event: TelemetryEvent) -> Result<(), LifecycleError> {
        imports_answered("telemetry");
        STATE.with(|state| {
            state.borrow_mut().last_telemetry = Some(format!("{}@{}", event.tag, event.seq));
        });
        bump("telemetry", |s| &mut s.telemetry)
    }
    fn schedule(payload: Vec<u8>) -> Result<(), LifecycleError> {
        imports_answered("schedule");
        STATE.with(|state| {
            state.borrow_mut().last_schedule = Some(String::from_utf8_lossy(&payload).into_owned());
        });
        bump("schedule", |s| &mut s.schedule)
    }
    fn pause() -> Result<(), LifecycleError> {
        imports_answered("pause");
        bump("pause", |s| &mut s.pause)
    }
    fn resume() -> Result<(), LifecycleError> {
        imports_answered("resume");
        bump("resume", |s| &mut s.resume)
    }
    fn snapshot() -> Result<Vec<u8>, LifecycleError> {
        imports_answered("snapshot");
        scripted("snapshot")?;
        Ok(counter_bytes(STATE.with(|state| state.borrow().counter)))
    }
    fn swap_in(state: Vec<u8>) -> Result<(), LifecycleError> {
        imports_answered("swap-in");
        scripted("swap-in")?;
        let counter = counter_of(&state)?;
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.counter = counter;
            s.swapped_in = Some(counter);
        });
        Ok(())
    }
    fn migrate(state: Vec<u8>) -> Result<Vec<u8>, LifecycleError> {
        imports_answered("migrate");
        scripted("migrate")?;
        let migrated = counter_of(&state)? + 1000;
        STATE.with(|s| s.borrow_mut().migrated = Some(migrated));
        Ok(counter_bytes(migrated))
    }
}

export!(LifecycleProbe);
