#![forbid(unsafe_code)]

//! J0 measurement — Butler conversational + in-proc latency (Story 8.1, AC7).
//!
//! Measures the §13.1 J0 budget:
//! - Conversational <400ms P95 end-to-end (morning_digest path)
//! - Spirit in-proc <60ms (assess() anticipatory-reasoning path)
//!
//! Butler ships in rust-inproc form, so the "IPC" leg is an in-process call.

use crate::harness::build_journey_result;
use crate::report::JourneyResult;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

const CONVERSATIONAL_P95_BUDGET_US: u64 = 400_000; // 400ms
const INPROC_P95_BUDGET_US: u64 = 60_000; // 60ms
const INVOCATION_COUNT: u64 = 1000;

/// Build a file-backed Transparency Log with a realistic ~24h workload.
fn seed_butler_tl(db: &Path) -> Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter> {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

    let tl = Arc::new(TransparencyLogAdapter::open(db, 0xB17_0).expect("open TL"));
    for i in 0..24 {
        let _ = tl.insert_frame_event(
            FrameKind::TaskComplete,
            1,
            None,
            &format!("task {i}"),
            b"done",
            FrameOrigin::SpiritAuto,
        );
    }
    let _ = tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        1,
        None,
        "ambiguous conflict",
        b"halt",
        FrameOrigin::Kernel,
    );
    tl
}

fn p95_us(samples: &mut [u64]) -> u64 {
    assert!(!samples.is_empty(), "p95 requires non-empty samples");
    samples.sort_unstable();
    let n = samples.len();
    let rank = (0.95 * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    samples[idx]
}

pub fn run_j0_measurement() -> JourneyResult {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let db = tmp.path().join("transparency.sqlite");
    let journal = tmp.path().join("journal.ndjson");
    std::fs::write(&journal, "").expect("empty journal");

    let _tl = seed_butler_tl(&db);

    let butler = butler::Butler::new();

    fn now_ns() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            + 1_000_000
    }

    // ── conversational leg: morning_digest ───────────────────────────────────
    let mut conversational_us = Vec::with_capacity(INVOCATION_COUNT as usize);
    for _ in 0..INVOCATION_COUNT {
        let now = now_ns();
        let t0 = Instant::now();
        let _digest = butler
            .morning_digest(&db, &journal, now, &[], 0.25)
            .expect("digest composes");
        conversational_us.push(t0.elapsed().as_micros() as u64);
    }

    // ── in-proc leg: assess() ────────────────────────────────────────────────
    let scenario = butler::ScenarioInput {
        calendar: vec![
            butler::CalendarEvent {
                id: "a".into(),
                title: "Standup".into(),
                start_min: 540,
                end_min: 600,
                status: butler::EventStatus::Confirmed,
            },
            butler::CalendarEvent {
                id: "b".into(),
                title: "Board call".into(),
                start_min: 570,
                end_min: 630,
                status: butler::EventStatus::Confirmed,
            },
        ],
        ..Default::default()
    };
    let mut inproc_us = Vec::with_capacity(INVOCATION_COUNT as usize);
    for _ in 0..INVOCATION_COUNT {
        let t0 = Instant::now();
        let _ = butler.assess(&scenario);
        inproc_us.push(t0.elapsed().as_micros() as u64);
    }

    let conv_p95 = p95_us(&mut conversational_us);
    let inproc_p95 = p95_us(&mut inproc_us);

    // Report the conversational leg as the primary J0 result (it is the
    // end-to-end budget). The in-proc leg is recorded in the journey name.
    let result = build_journey_result(
        &format!("J0 (conv_p95={conv_p95}us inproc_p95={inproc_p95}us)"),
        INVOCATION_COUNT,
        &conversational_us,
        CONVERSATIONAL_P95_BUDGET_US,
    );

    // Sanity-check the in-proc budget inline.
    assert!(
        inproc_p95 <= INPROC_P95_BUDGET_US,
        "J0 in-proc P95 {inproc_p95}us exceeds budget {INPROC_P95_BUDGET_US}us"
    );

    result
}
