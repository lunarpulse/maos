//! Scripted `maos:spirit@2.2.0` recall consumer for real runner/session tests.
//!
//! Input script is UTF-8 in TaskAssign.goal, TelemetryEvent.data or
//! TaskComplete.result. One command per line; blank lines are ignored:
//! - `recall [limit=N] [kind=LABEL] [since=NS] [until=NS] [intent=HEX]`
//!   `[cursor=NS:HEX]`: default limit 16; HEX is lowercase/uppercase hex of
//!   UTF-8 intent bytes or the 16-byte cursor id. `cursor=last` reuses the last
//!   page's next cursor. Labels are passed unchanged, including unknown ones.
//! - `fetch self|last|HEX`: inbound id, last returned recall entry id, or an
//!   explicit id (arbitrary lengths permit validation tests).
//! - `recalls N`: N recalls with the default filter (including zero).
//! - `context N`: Story 18-5 — `context.report(N)`; reports
//!   `context ok USED WINDOW LEVEL CROSSED|-` or `context err CODE`.
//! - `to SPIRIT`: address the emitted report to this Spirit instead of the input
//!   recipients. Session send authorization sees these claims; telemetry delivery
//!   itself uses the bus's broadcast subscription.
//!
//! Output is one TelemetryEvent (`event-type = recall.probe`), preserving the
//! input envelope/id except explicit `to`, with LF-separated data:
//! `recall ok COUNT cursor=none|NS:HEX`, followed by
//! `entry IDHEX NS KIND INTENTHEX PAYLOAD_AVAILABLE` for each entry;
//! `fetch ok IDHEX NS KIND INTENTHEX PAYLOADHEX ORIGIN`; or
//! `recall err CODE` / `fetch err CODE`. An empty hex field is `-`.
//! An inbound kernel `context-pressure`/`context-limit` frame (Story 18-5) is
//! reported as `context-frame KIND PID USED WINDOW PERMILLE`.
//! Script syntax errors emit `script err invalid-request` (no host call).
//! Before Ready and during shutdown the guest also calls recall and asserts
//! `unavailable`; those calls must produce no transport records.
wit_bindgen::generate!({ path: "../../../../wit/spirit.wit", world: "spirit" });

use maos::spirit::frames::{FrameKind, FrameOrigin, FramePayload, TelemetryEventBody};
use maos::spirit::recall::{self, RecallCursor, RecallError, RecallFilter};
use std::fmt::Write;
struct RecallProbe;

fn default_filter() -> RecallFilter {
    RecallFilter {
        kind: None,
        since_ns: None,
        until_ns: None,
        limit: 16,
        cursor: None,
        intent_filter: None,
    }
}
fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".into();
    }
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").unwrap();
    }
    out
}
fn unhex(value: &str) -> Option<Vec<u8>> {
    if value == "-" {
        return Some(vec![]);
    }
    if value.len() % 2 != 0 {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |b: u8| (b as char).to_digit(16).map(|v| v as u8);
            Some(digit(pair[0])? * 16 + digit(pair[1])?)
        })
        .collect()
}
fn code(error: RecallError) -> &'static str {
    match error {
        RecallError::ScopeViolation => "scope-violation",
        RecallError::FrameNotFound => "frame-not-found",
        RecallError::InvalidRequest(_) => "invalid-request",
        RecallError::LimitExceeded(_) => "limit-exceeded",
        RecallError::CapabilityDenied => "capability-denied",
        RecallError::ConsentWithheld => "consent-withheld",
        RecallError::ReplyTooLarge(_) => "reply-too-large",
        RecallError::Unavailable => "unavailable",
    }
}
fn run_recall(
    filter: RecallFilter,
    report: &mut String,
    last: &mut Option<Vec<u8>>,
    cursor: &mut Option<RecallCursor>,
) {
    match recall::recall(&filter) {
        Ok(page) => {
            let resume = page
                .next_cursor
                .as_ref()
                .map(|c| format!("{}:{}", c.last_timestamp_ns, hex(&c.last_frame_id)))
                .unwrap_or_else(|| "none".into());
            writeln!(report, "recall ok {} cursor={resume}", page.entries.len()).unwrap();
            for entry in page.entries {
                writeln!(
                    report,
                    "entry {} {} {} {} {}",
                    hex(&entry.frame_id),
                    entry.timestamp_ns,
                    entry.kind,
                    hex(entry.intent.as_bytes()),
                    entry.payload_available
                )
                .unwrap();
                *last = Some(entry.frame_id);
            }
            *cursor = page.next_cursor;
        }
        Err(error) => {
            writeln!(report, "recall err {}", code(error)).unwrap();
        }
    }
}
fn filter<'a>(
    args: impl Iterator<Item = &'a str>,
    last: &Option<RecallCursor>,
) -> Option<RecallFilter> {
    let mut filter = default_filter();
    for arg in args {
        let (key, value) = arg.split_once('=')?;
        match key {
            "limit" => filter.limit = value.parse().ok()?,
            "kind" => filter.kind = Some(value.into()),
            "since" => filter.since_ns = Some(value.parse().ok()?),
            "until" => filter.until_ns = Some(value.parse().ok()?),
            "intent" => filter.intent_filter = Some(String::from_utf8(unhex(value)?).ok()?),
            "cursor" if value == "last" => filter.cursor = last.clone(),
            "cursor" => {
                let (ns, id) = value.split_once(':')?;
                filter.cursor = Some(RecallCursor {
                    last_timestamp_ns: ns.parse().ok()?,
                    last_frame_id: unhex(id)?,
                });
            }
            _ => return None,
        }
    }
    Some(filter)
}
impl Guest for RecallProbe {
    fn handle_frame(mut frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        let mut report = String::new();
        if let FramePayload::ContextPressure(e) | FramePayload::ContextLimit(e) = &frame.payload {
            let kind = if frame.kind == FrameKind::ContextPressure { "pressure" } else { "limit" };
            let (pid, used, size, permille) =
                (e.spirit_pid, e.context_used, e.context_window_size, e.threshold_permille);
            writeln!(report, "context-frame {kind} {pid} {used} {size} {permille}").unwrap();
        }
        let script = match &frame.payload {
            FramePayload::TaskAssign(body) => body.goal.clone(),
            FramePayload::TelemetryEvent(body) => body.data.clone(),
            FramePayload::TaskComplete(body) => body.result_text.clone(),
            _ => String::new(),
        };
        let mut last = None;
        let mut cursor = None;
        for line in script.lines().filter(|line| !line.trim().is_empty()) {
            let mut words = line.split_whitespace();
            match words.next() {
                Some("to") => match words.next().filter(|_| words.clone().next().is_none()) {
                    Some(id) => {
                        frame.to = vec![maos::spirit::frames::FrameAddress {
                            spirit_id: id.into(),
                            host_id: None,
                            role: None,
                        }];
                    }
                    None => report.push_str("script err invalid-request\n"),
                },
                Some("recall") => match filter(words, &cursor) {
                    Some(filter) => run_recall(filter, &mut report, &mut last, &mut cursor),
                    None => report.push_str("script err invalid-request\n"),
                },
                Some("recalls") => match words
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .filter(|_| words.next().is_none())
                {
                    Some(count) => {
                        for _ in 0..count {
                            run_recall(default_filter(), &mut report, &mut last, &mut cursor);
                        }
                    }
                    None => report.push_str("script err invalid-request\n"),
                },
                Some("context") => match words.next().and_then(|n| n.parse::<u32>().ok()) {
                    Some(used) => match maos::spirit::context::report(used) {
                        Ok(s) => {
                            let name = |l: maos::spirit::context::ContextLevel| format!("{l:?}");
                            let crossed = s.crossed.map(name).unwrap_or_else(|| "-".into());
                            let (used, window, level) = (s.used, s.window, name(s.level));
                            writeln!(report, "context ok {used} {window} {level} {crossed}").unwrap();
                        }
                        Err(e) => writeln!(report, "context err {e:?}").unwrap(),
                    },
                    None => report.push_str("script err invalid-request\n"),
                },
                Some("fetch") => {
                    let id = match words.next() {
                        Some("self") => Some(frame.frame_id.clone()),
                        Some("last") => last.clone(),
                        Some(id) => unhex(id),
                        None => None,
                    };
                    if let Some(id) = id.filter(|_| words.next().is_none()) {
                        match recall::fetch(&id) {
                            Ok(row) => {
                                let origin = match row.origin {
                                    FrameOrigin::HumanAuthored => "human-authored",
                                    FrameOrigin::SpiritAuto => "spirit-auto",
                                    FrameOrigin::SpiritDraftedHumanApproved => {
                                        "spirit-drafted-human-approved"
                                    }
                                    FrameOrigin::Kernel => "kernel",
                                };
                                writeln!(
                                    report,
                                    "fetch ok {} {} {} {} {} {}",
                                    hex(&row.frame_id),
                                    row.timestamp_ns,
                                    row.kind,
                                    hex(row.intent.as_bytes()),
                                    hex(&row.payload),
                                    origin
                                )
                                .unwrap();
                            }
                            Err(error) => {
                                writeln!(report, "fetch err {}", code(error)).unwrap();
                            }
                        }
                    } else {
                        report.push_str("script err invalid-request\n");
                    }
                }
                _ => report.push_str("script err invalid-request\n"),
            }
        }
        frame.kind = FrameKind::TelemetryEvent;
        frame.payload = FramePayload::TelemetryEvent(TelemetryEventBody {
            event_type: "recall.probe".into(),
            data: report,
        });
        Ok(vec![frame])
    }
    fn on_start() -> Result<(), Halt> {
        assert!(matches!(
            recall::recall(&default_filter()),
            Err(RecallError::Unavailable)
        ));
        Ok(())
    }
    fn on_shutdown() {
        assert!(matches!(
            recall::recall(&default_filter()),
            Err(RecallError::Unavailable)
        ));
    }
}
export!(RecallProbe);
