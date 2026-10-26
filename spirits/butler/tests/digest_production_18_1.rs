#![forbid(unsafe_code)]

//! Story 18-1 code review (2026-10-25) — the production FR17 digest.
//!
//! - Review ruling D-18-1-R3: (d) is yesterday's predicate-fire rate —
//!   `epistemic.halt` frames whose intent is an evaluated tag over the
//!   `epistemic.evaluated:<tag>` frames the composition root writes per scalar
//!   evaluation.
//! - Review ruling D-18-1-R5 / EC-7: the renderer renders once per day and a
//!   clock stepped back across midnight never renders a duplicate.
//! - Story 18-4 (D-18-4-06 A, AC2.3): the day is the injected `Clock`'s
//!   LOCAL day — a `ScenarioClock` stepped across local midnight at a
//!   non-zero offset renders once per local day, never twice, never after a
//!   step back. (D-18-4-07 A, AC5.2): (c) reads the window's
//!   `anomaly.flagged:` frames through the ≥ 0.6 floor, each citing its frame.

use std::path::Path;
use std::sync::{Arc, Mutex};

use butler::{
    yesterday_predicate_fire_rate, Butler, MorningDigestRenderer, PredicateFireRate,
    INTENT_ANOMALY_FLAGGED, INTENT_PREDICATE_EVALUATED, NANOS_PER_DAY,
};
use maos_kernel_core::iac::transparency_log::FrameKind;
use maos_spirit_sdk::clock::{Clock, ScenarioClock, SystemClock};

const HOUR_NS: u64 = 3_600_000_000_000;
/// Today's UTC day number (any fixed day well after the epoch).
const TODAY: u64 = 20_000;

/// A Transparency Log with explicit frame ids and timestamps (the schema
/// `maos-audit` reads; `spirits/butler/tests/hallucination.rs` precedent).
fn seed(db: &Path, frames: &[(u64, FrameKind, &str)]) {
    let with_payload: Vec<(u64, FrameKind, &str, &str)> =
        frames.iter().map(|(ts, k, i)| (*ts, *k, *i, "x")).collect();
    seed_payloads(db, &with_payload);
}

fn seed_payloads(db: &Path, frames: &[(u64, FrameKind, &str, &str)]) {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transparency_log (
            frame_id BLOB NOT NULL PRIMARY KEY,
            timestamp_ns INTEGER NOT NULL,
            spirit_pid INTEGER NOT NULL,
            boot_nonce INTEGER NOT NULL,
            capability_token BLOB,
            kind INTEGER NOT NULL,
            intent TEXT NOT NULL,
            payload_redacted BLOB NOT NULL,
            origin INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS approval_decision_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ns INTEGER NOT NULL DEFAULT 0,
            actor TEXT NOT NULL,
            target TEXT NOT NULL,
            capability TEXT NOT NULL,
            intent TEXT NOT NULL,
            decision INTEGER NOT NULL DEFAULT 1,
            reasoning TEXT DEFAULT ''
        );",
    )
    .unwrap();
    for (i, (ts, kind, intent, payload)) in frames.iter().enumerate() {
        let mut id = [0u8; 16];
        id[0] = 0x18;
        id[15] = i as u8;
        conn.execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, 1, 1, ?3, ?4, ?5, 1)",
            rusqlite::params![&id[..], *ts as i64, *kind as i64, intent, payload.as_bytes()],
        )
        .unwrap();
    }
}

fn evaluated(tag: &str) -> String {
    format!("{INTENT_PREDICATE_EVALUATED}{tag}")
}

/// Yesterday: 4 evaluations of `belief_variance`, 1 of its halts (a fire), a
/// planned-unload receipt and a halt of a tag never evaluated (neither is a
/// fire). Today and two days ago carry fires and evaluations that must not
/// count.
fn seeded_tl(dir: &Path) -> std::path::PathBuf {
    let db = dir.join("tl.sqlite");
    let yesterday = (TODAY - 1) * NANOS_PER_DAY;
    let today = TODAY * NANOS_PER_DAY;
    let before = (TODAY - 2) * NANOS_PER_DAY;
    let bv = evaluated("belief_variance");
    seed(
        &db,
        &[
            (before + HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (
                before + 2 * HOUR_NS,
                FrameKind::EpistemicHalt,
                "belief_variance",
            ),
            (yesterday + HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (yesterday + 2 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (
                yesterday + 2 * HOUR_NS + 1,
                FrameKind::EpistemicHalt,
                "belief_variance",
            ),
            (yesterday + 3 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (yesterday + 4 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (
                yesterday + 5 * HOUR_NS,
                FrameKind::EpistemicHalt,
                "planned_unload",
            ),
            (
                yesterday + 6 * HOUR_NS,
                FrameKind::EpistemicHalt,
                "diagnostic_confidence",
            ),
            (today + HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (
                today + 2 * HOUR_NS,
                FrameKind::EpistemicHalt,
                "belief_variance",
            ),
        ],
    );
    db
}

#[test]
fn the_trust_bar_is_yesterdays_predicate_fire_rate() {
    let dir = tempfile::tempdir().unwrap();
    let db = seeded_tl(dir.path());
    let journal = dir.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    let now = TODAY * NANOS_PER_DAY + 12 * HOUR_NS;

    let basis = yesterday_predicate_fire_rate(&db, now, 0).unwrap();
    assert_eq!(
        basis,
        PredicateFireRate {
            local_day: TODAY - 1,
            fires: 1,
            evaluations: 4,
        }
    );
    let digest = Butler::new()
        .production_morning_digest(&db, &journal, now, 0)
        .unwrap();
    assert_eq!(
        digest["trust_bar"],
        serde_json::json!(0.75),
        "1 fire / 4 evaluations → trust 0.75: {digest}"
    );
    assert_eq!(
        digest["trust_bar_basis"],
        serde_json::json!({"local_day": TODAY - 1, "fires": 1, "evaluations": 4})
    );
    assert_eq!(digest["utc_offset_secs"], serde_json::json!(0));
    assert_eq!(
        digest["anomalies_basis"],
        serde_json::json!({"wired": ["escape-detector"], "unwired": ["observer"]}),
        "(c) names the wired and the unwired producers"
    );
}

/// Story 18-4 (D-18-4-06 A) — at +09:00 the local day before 01:00 local is
/// the UTC day before: the basis covers the local yesterday, not the UTC one.
#[test]
fn the_trust_bar_basis_is_the_local_yesterday() {
    let dir = tempfile::tempdir().unwrap();
    let db = seeded_tl(dir.path());
    // 20:00 UTC on TODAY - 1 is 05:00 local on TODAY at +09:00.
    let now = (TODAY - 1) * NANOS_PER_DAY + 20 * HOUR_NS;
    let basis = yesterday_predicate_fire_rate(&db, now, 9 * 3600).unwrap();
    assert_eq!(basis.local_day, TODAY - 1);
    // Local TODAY - 1 runs 15:00 UTC TODAY - 2 → 15:00 UTC TODAY - 1: the
    // TODAY - 2 halt at 02:00 UTC is before it, the yesterday frames up to
    // 14:59 UTC are inside it.
    assert_eq!((basis.fires, basis.evaluations), (1, 4));
}

/// Review TIA-4 — frames placed so the UTC yesterday and the +09:00 local
/// yesterday hold DIFFERENT frames: at 01:00 UTC on TODAY (10:00 local), the
/// local yesterday is [TODAY-2 15:00, TODAY-1 15:00) UTC. Two evaluations
/// at TODAY-2 16:00/17:00 UTC are local-only; an evaluation and its fire at
/// TODAY-1 16:00 UTC are UTC-only; one at TODAY-1 10:00 UTC is in both.
#[test]
fn the_trust_bar_basis_differs_between_the_utc_and_the_local_yesterday() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tl.sqlite");
    let (before, yesterday) = ((TODAY - 2) * NANOS_PER_DAY, (TODAY - 1) * NANOS_PER_DAY);
    let bv = evaluated("belief_variance");
    seed(
        &db,
        &[
            (before + 16 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (before + 17 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (yesterday + 10 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (yesterday + 16 * HOUR_NS, FrameKind::TelemetryEvent, &bv),
            (
                yesterday + 16 * HOUR_NS + 1,
                FrameKind::EpistemicHalt,
                "belief_variance",
            ),
        ],
    );
    let now = TODAY * NANOS_PER_DAY + HOUR_NS;
    let utc = yesterday_predicate_fire_rate(&db, now, 0).unwrap();
    let local = yesterday_predicate_fire_rate(&db, now, 9 * 3600).unwrap();
    assert_eq!(
        (utc.local_day, utc.fires, utc.evaluations),
        (TODAY - 1, 1, 2)
    );
    assert_eq!(
        (local.local_day, local.fires, local.evaluations),
        (TODAY - 1, 0, 3),
        "the +09:00 basis reads the local yesterday, not the UTC day"
    );
}

/// Story 18-4 (D-18-4-07 A, AC5.2) — a seeded `anomaly.flagged:` frame at
/// 0.9 is listed with its frame ref; one at 0.4 is omitted; one outside the
/// last 24 h is omitted.
#[test]
fn the_digest_lists_seeded_anomalies_above_the_floor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tl.sqlite");
    let journal = dir.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    let now = TODAY * NANOS_PER_DAY + 12 * HOUR_NS;
    let intent = format!("{INTENT_ANOMALY_FLAGGED}escape-detector");
    let flagged = |summary: &str, confidence: f32| {
        serde_json::json!({
            "observer": "escape-detector",
            "subject": "spirit_pid=4",
            "summary": summary,
            "confidence": confidence,
        })
        .to_string()
    };
    let (high, low, old) = (
        flagged("seccomp kill at T2", 0.9),
        flagged("weak signal", 0.4),
        flagged("two days ago", 1.0),
    );
    seed_payloads(
        &db,
        &[
            (now - HOUR_NS, FrameKind::TelemetryEvent, &intent, &high),
            (now - 2 * HOUR_NS, FrameKind::TelemetryEvent, &intent, &low),
            (now - 48 * HOUR_NS, FrameKind::TelemetryEvent, &intent, &old),
        ],
    );
    let digest = Butler::new()
        .production_morning_digest(&db, &journal, now, 0)
        .unwrap();
    let anomalies = digest["anomalies"].as_array().expect("(c) is a list");
    assert_eq!(
        anomalies.len(),
        1,
        "only the ≥ 0.6 anomaly in the window: {digest}"
    );
    assert_eq!(anomalies[0]["summary"], "seccomp kill at T2");
    assert_eq!(anomalies[0]["observer"], "escape-detector");
    assert_eq!(
        anomalies[0]["source_log_ref"],
        format!("18{}00", "0".repeat(28)),
        "the item cites its anomaly.flagged frame"
    );
}

#[test]
fn a_day_with_no_evaluation_reads_full_trust() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tl.sqlite");
    seed(
        &db,
        &[(
            (TODAY - 1) * NANOS_PER_DAY + HOUR_NS,
            FrameKind::EpistemicHalt,
            "planned_unload",
        )],
    );
    let basis = yesterday_predicate_fire_rate(&db, TODAY * NANOS_PER_DAY + HOUR_NS, 0).unwrap();
    assert_eq!((basis.fires, basis.evaluations), (0, 0));
    assert_eq!(basis.rate(), 0.0);
}

#[test]
fn the_renderer_renders_once_per_day_and_never_on_a_clock_step_back() {
    let dir = tempfile::tempdir().unwrap();
    let db = seeded_tl(dir.path());
    let journal = dir.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    let frames = Arc::new(Mutex::new(Vec::<(u32, String)>::new()));
    let sink = Arc::clone(&frames);
    let renderer = MorningDigestRenderer::new(
        db,
        journal,
        Arc::new(move |pid, line: &[u8]| {
            sink.lock()
                .unwrap()
                .push((pid, String::from_utf8(line.to_vec()).unwrap()));
        }),
        Arc::new(SystemClock::UTC),
    );
    let at = |day: u64, hour: u64| day * NANOS_PER_DAY + hour * HOUR_NS;

    let first = renderer.render_at(at(TODAY, 8), 7).unwrap();
    assert_eq!(first.as_ref().unwrap()["event"], "morning_digest");
    assert!(
        renderer.render_at(at(TODAY, 9), 7).unwrap().is_none(),
        "the same day renders once"
    );
    assert!(
        renderer.render_at(at(TODAY - 1, 23), 7).unwrap().is_none(),
        "a wall clock stepped back across midnight must not render a duplicate"
    );
    assert!(
        renderer.render_at(at(TODAY + 1, 0), 7).unwrap().is_some(),
        "the next day renders"
    );
    let frames = frames.lock().unwrap();
    assert_eq!(frames.len(), 2, "one TL frame per rendered digest");
    assert_eq!(frames[0].0, 7, "the frame carries the Spirit's pid");
    let event: serde_json::Value = serde_json::from_str(&frames[0].1).unwrap();
    assert_eq!(event, first.unwrap(), "the TL frame is the stdout event");
}

/// Story 18-4 (AC2.3, D-18-4-06 A; `deferred-work.md:1232`) — the renderer
/// reads its injected clock: a `ScenarioClock` at +09:00 stepped across UTC
/// midnight (same local day) does not render, across LOCAL midnight renders
/// once, never twice, and never after a step back.
#[test]
fn a_scenario_clock_rolls_the_digest_over_on_the_local_day() {
    let dir = tempfile::tempdir().unwrap();
    let db = seeded_tl(dir.path());
    let journal = dir.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    let offset = 9 * 3600;
    let clock = Arc::new(ScenarioClock::new(0, offset));
    let frames = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let sink = Arc::clone(&frames);
    let renderer = MorningDigestRenderer::new(
        db,
        journal,
        Arc::new(move |_, line: &[u8]| {
            sink.lock()
                .unwrap()
                .push(serde_json::from_slice(line).unwrap());
        }),
        Arc::clone(&clock) as Arc<dyn Clock>,
    );
    // Local hour `h` of local day `day` at +09:00, in UTC ns.
    let local = |day: u64, hour: u64| day * NANOS_PER_DAY + hour * HOUR_NS - 9 * HOUR_NS;
    let steps = [
        (local(TODAY, 8), true, "first local morning renders"),
        (local(TODAY, 8) + HOUR_NS, false, "same local day"),
        (
            TODAY * NANOS_PER_DAY + 30 * 60_000_000_000,
            false,
            "UTC midnight is not local midnight",
        ),
        (
            local(TODAY + 1, 0) + 60_000_000_000,
            true,
            "local midnight renders",
        ),
        (
            local(TODAY + 1, 0) + 2 * 60_000_000_000,
            false,
            "never twice",
        ),
        (local(TODAY, 23), false, "a step back never renders"),
        (local(TODAY + 2, 8), true, "the next local day renders"),
    ];
    for (now, renders, why) in steps {
        clock.set_ns(now);
        let before = frames.lock().unwrap().len();
        renderer.render_if_new_day(3).unwrap();
        assert_eq!(
            frames.lock().unwrap().len() - before,
            usize::from(renders),
            "{why}"
        );
    }
    let frames = frames.lock().unwrap();
    assert_eq!(
        frames
            .iter()
            .map(|f| f["now_ns"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![
            local(TODAY, 8),
            local(TODAY + 1, 0) + 60_000_000_000,
            local(TODAY + 2, 8)
        ],
        "each digest carries the scenario instant, never the wall clock"
    );
    assert_eq!(
        frames[0]["digest"]["utc_offset_secs"],
        serde_json::json!(offset)
    );
}
