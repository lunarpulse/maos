#![forbid(unsafe_code)]

//! Story 18-1 code review (2026-10-25) — the production FR17 digest.
//!
//! - Review ruling D-18-1-R3: (d) is yesterday's (UTC day) predicate-fire
//!   rate — `epistemic.halt` frames whose intent is an evaluated tag over the
//!   `epistemic.evaluated:<tag>` frames the composition root writes per scalar
//!   evaluation; (c) is an explicit unknown, never an empty list.
//! - Review ruling D-18-1-R5 / EC-7: the renderer renders once per UTC day and
//!   a wall clock stepped back across midnight never renders a duplicate. (The
//!   process-level rollover test waits for 18-4's injectable `Clock`.)

use std::path::Path;
use std::sync::{Arc, Mutex};

use butler::{
    yesterday_predicate_fire_rate, Butler, MorningDigestRenderer, PredicateFireRate,
    ANOMALY_FEED_UNKNOWN, INTENT_PREDICATE_EVALUATED, NANOS_PER_DAY,
};
use maos_kernel_core::iac::transparency_log::FrameKind;

const HOUR_NS: u64 = 3_600_000_000_000;
/// Today's UTC day number (any fixed day well after the epoch).
const TODAY: u64 = 20_000;

/// A Transparency Log with explicit frame ids and timestamps (the schema
/// `maos-audit` reads; `spirits/butler/tests/hallucination.rs` precedent).
fn seed(db: &Path, frames: &[(u64, FrameKind, &str)]) {
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
    for (i, (ts, kind, intent)) in frames.iter().enumerate() {
        let mut id = [0u8; 16];
        id[0] = 0x18;
        id[15] = i as u8;
        conn.execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, 1, 1, ?3, ?4, ?5, 1)",
            rusqlite::params![&id[..], *ts as i64, *kind as i64, intent, b"x" as &[u8]],
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

    let basis = yesterday_predicate_fire_rate(&db, now).unwrap();
    assert_eq!(
        basis,
        PredicateFireRate {
            day_utc: TODAY - 1,
            fires: 1,
            evaluations: 4,
        }
    );
    let digest = Butler::new()
        .production_morning_digest(&db, &journal, now)
        .unwrap();
    assert_eq!(
        digest["trust_bar"],
        serde_json::json!(0.75),
        "1 fire / 4 evaluations → trust 0.75: {digest}"
    );
    assert_eq!(
        digest["trust_bar_basis"],
        serde_json::json!({"day_utc": TODAY - 1, "fires": 1, "evaluations": 4})
    );
    assert_eq!(
        digest["anomalies"],
        serde_json::json!(ANOMALY_FEED_UNKNOWN),
        "(c) is an explicit unknown, not an empty list"
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
    let basis = yesterday_predicate_fire_rate(&db, TODAY * NANOS_PER_DAY + HOUR_NS).unwrap();
    assert_eq!((basis.fires, basis.evaluations), (0, 0));
    assert_eq!(basis.rate(), 0.0);
}

#[test]
fn the_renderer_renders_once_per_utc_day_and_never_on_a_clock_step_back() {
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
    );
    let at = |day: u64, hour: u64| day * NANOS_PER_DAY + hour * HOUR_NS;

    let first = renderer.render_at(at(TODAY, 8), 7).unwrap();
    assert_eq!(first.as_ref().unwrap()["event"], "morning_digest");
    assert!(
        renderer.render_at(at(TODAY, 9), 7).unwrap().is_none(),
        "the same UTC day renders once"
    );
    assert!(
        renderer.render_at(at(TODAY - 1, 23), 7).unwrap().is_none(),
        "a wall clock stepped back across midnight must not render a duplicate"
    );
    assert!(
        renderer.render_at(at(TODAY + 1, 0), 7).unwrap().is_some(),
        "the next UTC day renders"
    );
    let frames = frames.lock().unwrap();
    assert_eq!(frames.len(), 2, "one TL frame per rendered digest");
    assert_eq!(frames[0].0, 7, "the frame carries the Spirit's pid");
    let event: serde_json::Value = serde_json::from_str(&frames[0].1).unwrap();
    assert_eq!(event, first.unwrap(), "the TL frame is the stdout event");
}
