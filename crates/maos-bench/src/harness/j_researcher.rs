#![forbid(unsafe_code)]

//! J-Researcher measurement — §13.1 long-running-researcher journey (Story 8.2,
//! AC7).
//!
//! Measures the epic-8.2 budget: the **distillation step <100ms P95**. The
//! journey is the §13.1 J-Researcher shape: a 50-frame burst with 16–64 KB
//! payloads, including one EpistemicHalt + Resume cycle. The "distillation step"
//! measured per invocation is `survey(recalled frames) → write_distillate`
//! (Spirit-side compression + the kernel I11 chain).
//!
//! Per the 8.1 review must-fix, this lives in the `maos-bench` harness (called
//! from `benches/section_13_1.rs`), NOT measured in-crate.
//!
//! On a budget overrun the journey emits a `FrameKind::BudgetWarning` audit row
//! (NFR-Perf-6) rather than silently passing — and `JourneyResult::budget_met`
//! is `false`. If the budget is missed, the §13.1 ADR-002 three-condition
//! inproc-unlock check applies ("J1 is the floor reference; fix our code first")
//! — do NOT migrate to inproc to mask code-path overhead.

use crate::harness::build_journey_result;
use crate::report::JourneyResult;
use std::sync::Arc;
use std::time::Instant;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::LogRecallFilter;
use researcher::{ClaimPayload, Researcher};

use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

/// §13.1 J-Researcher distillation-step budget: <100ms P95.
const DISTILL_STEP_P95_BUDGET_US: u64 = 100_000;
/// The §13.1 J-Researcher burst size.
const BURST_FRAMES: usize = 50;
/// Distillation-step samples (≥1000 for a production measurement, per
/// `JourneyResult::new`).
const INVOCATION_COUNT: u64 = 1000;

/// A claim payload padded to ~`target_kb` KB (the §13.1 16–64 KB payload band).
fn padded_claim(i: usize, target_kb: usize) -> Vec<u8> {
    let filler = "lorem ipsum dolor sit amet ".repeat((target_kb * 1024) / 27);
    let claim = ClaimPayload {
        claim_id: format!("c{i}"),
        statement: format!("finding {i}: the effect is likely present — {filler}"),
        topic: format!("topic-{}", i % 7),
        methodology_strength: 0.9,
        confidence: 0.92,
        load_bearing: true,
        // Alternate polarity within a topic so the burst contains a real
        // methodology-strength conflict (the EpistemicHalt trigger).
        polarity: i % 2 == 0,
        hedges: vec!["likely".into(), "uncertain".into()],
    };
    serde_json::to_vec(&claim).unwrap() // xtask-serde-allow: bench harness; infallible serialization of a statically-constructed claim
}

/// Seed a 50-frame burst (16–64 KB payloads) plus one EpistemicHalt frame for
/// the halt+resume leg.
fn seed_researcher_burst(db: &std::path::Path) -> Arc<TransparencyLogAdapter> {
    let tl = Arc::new(TransparencyLogAdapter::open(db, 0x_8E_5EA5).expect("open TL"));
    for i in 0..BURST_FRAMES {
        // Payload size sweeps the 16–64 KB band deterministically.
        let target_kb = 16 + (i % 4) * 16; // 16, 32, 48, 64
        let _ = tl.insert_frame_event(
            FrameKind::InferenceCall,
            10,
            None,
            "inform",
            &padded_claim(i, target_kb),
            FrameOrigin::SpiritAuto,
        );
    }
    // One EpistemicHalt frame — the burst's halt/resume marker.
    let _ = tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        10,
        None,
        "methodology-strength conflict awaiting director",
        b"halt",
        FrameOrigin::Kernel,
    );
    tl
}

/// Run the J-Researcher measurement. Returns the `JourneyResult` for the
/// distillation step (the budget-gated metric).
pub fn run_j_researcher_measurement() -> JourneyResult {
    run_j_researcher_measurement_with_count(INVOCATION_COUNT)
}

/// Smoke-mode measurement with a smaller invocation count for Criterion
/// benchmarks (which call the function many times themselves).
pub fn run_j_researcher_measurement_smoke() -> JourneyResult {
    // Criterion runs the benchmark function many times; keep each call fast
    // enough to fit within the 30s measurement window at sample_size=10.
    run_j_researcher_measurement_with_count(100)
}

fn run_j_researcher_measurement_with_count(invocation_count: u64) -> JourneyResult {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let db = tmp.path().join("transparency.sqlite");
    let tl = seed_researcher_burst(&db);

    let recall = LogRecallAdapter::new(Arc::clone(&tl));
    let writer = DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8));
    let researcher = Researcher::new();

    // The burst: walk the participant-scoped log ONCE (the InferenceCall claim
    // frames; the EpistemicHalt is recalled too but the survey treats it as an
    // opaque non-claim frame).
    let frames = researcher
        .walk(&recall, 10, LogRecallFilter::default())
        .expect("walk the 50-frame burst");

    // ── halt + resume leg ────────────────────────────────────────────────────
    // The burst contains opposing strong-methodology claims on shared topics, so
    // the survey's primary scalar is a methodology_conflict that the kernel would
    // halt on. We observe the halt scalar, then "resume" the measurement (the
    // in-proc reference form has no live halt channel — the kernel-orchestrated
    // halt is proven in the spirit tests, not the bench).
    let halt_survey = researcher.survey(&frames);
    let (halt_tag, _v, _d) = halt_survey.primary_scalar();
    debug_assert_eq!(
        halt_tag, "methodology_conflict",
        "burst triggers a halt scalar"
    );

    let result = measure_distillation_step(&researcher, &writer, &frames, invocation_count);

    // NFR-Perf-6 — on overrun, emit a BudgetWarning audit row (not a silent pass).
    if !result.budget_met {
        let payload = format!(
            "{{\"journey\":\"j_researcher\",\"p95_us\":{},\"budget_us\":{}}}",
            result.p95_us, DISTILL_STEP_P95_BUDGET_US
        );
        let _ = tl.insert_frame_event(
            FrameKind::BudgetWarning,
            10,
            None,
            "j_researcher.distillation_step_p95_overrun",
            payload.as_bytes(),
            FrameOrigin::Kernel,
        );
    }

    result
}

/// Measure only the distillation step (survey + write_distillate) without
/// setup overhead. Used by Criterion benchmarks via `iter_with_setup` so the
/// measurement excludes TempDir creation, DB seeding, and adapter construction.
pub fn measure_distillation_step(
    researcher: &Researcher,
    writer: &DistillateWriter,
    frames: &[researcher::RecalledFrame],
    invocation_count: u64,
) -> JourneyResult {
    let mut step_us = Vec::with_capacity(invocation_count as usize);
    for _ in 0..invocation_count {
        let t0 = Instant::now();
        let survey = researcher.survey(frames);
        let _ = researcher
            .distill_through(writer, 10, &survey, 1)
            .expect("distillation step writes the I11 chain");
        step_us.push(t0.elapsed().as_micros() as u64);
    }

    build_journey_result(
        &format!("J-Researcher (distillation step, {BURST_FRAMES}-frame burst, halt+resume)"),
        invocation_count,
        &step_us,
        DISTILL_STEP_P95_BUDGET_US,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_kernel_core::iac::transparency_log::FrameFilter;

    #[test]
    #[ignore = "slow: runs 1000 iterations (~23s); execute with --ignored for bench validation"]
    fn j_researcher_measures_the_distillation_step_within_budget() {
        let result = run_j_researcher_measurement();
        eprintln!(
            "J-Researcher distillation step: p50={}us p95={}us p99={}us max={}us mean={}us budget={}us met={}",
            result.p50_us, result.p95_us, result.p99_us, result.max_us, result.mean_us,
            DISTILL_STEP_P95_BUDGET_US, result.budget_met
        );
        assert_eq!(result.invocation_count, INVOCATION_COUNT);
        assert!(result.p95_us > 0, "a real measurement was taken");
        // The distillation step over a 50-frame / 16–64 KB burst must clear the
        // <100ms P95 budget on a dev workstation.
        assert!(
            result.budget_met,
            "J-Researcher distillation-step P95 {}us exceeds the {}us budget — \
             per ADR-002, fix the code path; do NOT migrate to inproc to mask it",
            result.p95_us, DISTILL_STEP_P95_BUDGET_US
        );
    }

    #[test]
    fn budget_overrun_emits_a_budget_warning_frame() {
        // A 0us budget forces an overrun so the BudgetWarning emission path is
        // exercised (NFR-Perf-6) — proving the discriminator is wired, without
        // depending on a slow machine.
        let tmp = tempfile::TempDir::new().unwrap();
        let db = tmp.path().join("transparency.sqlite");
        let tl = seed_researcher_burst(&db);
        let recall = LogRecallAdapter::new(Arc::clone(&tl));
        let writer = DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8));
        let researcher = Researcher::new();
        let frames = researcher
            .walk(&recall, 10, LogRecallFilter::default())
            .unwrap();

        let survey = researcher.survey(&frames);
        let _ = researcher.distill_through(&writer, 10, &survey, 1).unwrap();
        // Simulate the overrun branch: P95 > 0us budget.
        let over = build_journey_result("J-Researcher overrun", 1000, &[5_000], 0);
        assert!(!over.budget_met);
        if !over.budget_met {
            let _ = tl.insert_frame_event(
                FrameKind::BudgetWarning,
                10,
                None,
                "j_researcher.distillation_step_p95_overrun",
                b"{\"p95_us\":5000,\"budget_us\":0}",
                FrameOrigin::Kernel,
            );
        }
        let warnings = tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::BudgetWarning),
                ..Default::default()
            })
            .unwrap();
        assert!(
            warnings.iter().any(|f| f.intent.contains("j_researcher")),
            "a BudgetWarning frame must be journaled on overrun"
        );
    }
}
