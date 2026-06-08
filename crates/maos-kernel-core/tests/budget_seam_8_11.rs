#![forbid(unsafe_code)]

//! Story 8.11 · AC3 — per-Spirit budget threading (the authorized kernel-core delta).
//!
//! Proves the parsed-but-previously-dead `[budget].time_cap_seconds` now drives
//! the dispatcher **per loaded Spirit** (not the global 30s default), and that a
//! Spirit which omits `[budget]` still falls back to the dispatcher default.
//!
//! **Provider-independent (Murat's AC7 trip-wire):** no inference, no LLM, no
//! network — the cap is exercised with a deterministic sleeping hook so the
//! reproducible crate carries zero inference variance.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_kernel_core::iac::{FrameFilter, FrameKind};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::{HookDispatcher, HookOutcome},
};
use maos_kernel_core::security::manifest::{Budget, LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

/// A hook that sleeps `sleep_ms` inside `on_load` so we can drive it past a cap.
struct SleepyHook {
    count: AtomicU32,
    sleep_ms: u64,
}

impl maos_spirit_abi::lifecycle::Spirit for SleepyHook {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.count.fetch_add(1, Ordering::Relaxed);
        if self.sleep_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.sleep_ms));
        }
    }
}

fn make_tl() -> Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter> {
    Arc::new(maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0))
}

fn make_dispatcher_with_default(
    tl: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    default_cap: u64,
) -> HookDispatcher {
    let metrics = Arc::new(IacRtMetrics::new());
    let mut d = HookDispatcher::new(tl, metrics);
    d.time_cap_seconds = default_cap;
    d
}

fn make_scb(budget: Option<Budget>, sleep_ms: u64) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into()],
        },
        budget,
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        7,
        "budget-test".into(),
        manifest,
        make_spirit_obj(SleepyHook {
            count: AtomicU32::new(0),
            sleep_ms,
        }),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

/// The manifest `[budget].time_cap_seconds` overrides the dispatcher default.
/// Dispatcher default is a large 30s; the manifest declares 1s; a 2s hook must
/// exceed the **manifest** cap (1s), proving the per-Spirit budget governed.
#[tokio::test]
async fn manifest_time_cap_overrides_default() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = make_tl();
    let dispatcher = make_dispatcher_with_default(Arc::clone(&tl), 30);
    let scb = make_scb(
        Some(Budget {
            context_window_size: 4096,
            time_cap_seconds: 1,
        }),
        2000,
    );

    let outcome = dispatcher.fire_on_load(&scb).await;
    match outcome {
        HookOutcome::BudgetExceeded { cap_seconds, .. } => {
            assert_eq!(
                cap_seconds, 1,
                "the manifest [budget].time_cap_seconds=1 must govern, NOT the 30s default"
            );
        }
        other => panic!("expected BudgetExceeded with manifest cap=1, got {other:?}"),
    }
}

/// A Spirit that omits `[budget]` falls back to the dispatcher default.
/// Dispatcher default is 1s; budget is None; a 2s hook exceeds the **default**.
#[tokio::test]
async fn absent_budget_falls_back_to_default() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = make_tl();
    let dispatcher = make_dispatcher_with_default(Arc::clone(&tl), 1);
    let scb = make_scb(None, 2000);

    let outcome = dispatcher.fire_on_load(&scb).await;
    match outcome {
        HookOutcome::BudgetExceeded { cap_seconds, .. } => {
            assert_eq!(
                cap_seconds, 1,
                "with no [budget], the dispatcher default (1s here) must govern"
            );
        }
        other => panic!("expected BudgetExceeded with default cap=1, got {other:?}"),
    }
}

/// The 80% `BudgetWarning` (already emitted from production) fires at the
/// **manifest** cap, then `BudgetExceeded` at the cap — both recorded to the TL
/// with the manifest's `cap_seconds`, proving the per-Spirit cap drives the
/// production warn-emit path.
#[tokio::test]
async fn slow_hook_emits_budget_warning_then_exceeded_at_manifest_cap() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = make_tl();
    let dispatcher = make_dispatcher_with_default(Arc::clone(&tl), 30);
    // manifest cap = 1s → warn at 0.8s, exceed at 1s. Hook sleeps 3s.
    let scb = make_scb(
        Some(Budget {
            context_window_size: 4096,
            time_cap_seconds: 1,
        }),
        3000,
    );

    let outcome = dispatcher.fire_on_load(&scb).await;
    assert!(
        matches!(outcome, HookOutcome::BudgetExceeded { cap_seconds: 1, .. }),
        "3s hook with manifest cap=1 must exceed at the manifest cap: got {outcome:?}"
    );

    let warnings = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::BudgetWarning),
            ..Default::default()
        })
        .expect("query BudgetWarning frames");
    assert!(
        !warnings.is_empty(),
        "the 80% BudgetWarning must be emitted to the TL for a slow hook under the manifest cap"
    );

    let exceeded = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::BudgetExceeded),
            ..Default::default()
        })
        .expect("query BudgetExceeded frames");
    assert!(
        !exceeded.is_empty(),
        "BudgetExceeded must be emitted to the TL when the hook overruns the manifest cap"
    );
}
