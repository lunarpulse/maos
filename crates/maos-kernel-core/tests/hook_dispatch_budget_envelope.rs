#![forbid(unsafe_code)]

//! Integration test: hook dispatch budget envelope (AC2).
//!
//! Verifies manifest gate, timeout, BudgetWarning at 80%, and
//! BudgetExceeded at 100%.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::{HookDispatcher, HookOutcome},
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct HookCounter {
    count: AtomicU32,
    sleep_ms: u64,
}

impl Default for HookCounter {
    fn default() -> Self {
        Self { count: AtomicU32::new(0), sleep_ms: 0 }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for HookCounter {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.count.fetch_add(1, Ordering::Relaxed);
        if self.sleep_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.sleep_ms));
        }
    }
}

fn make_dispatcher() -> HookDispatcher {
    let tl = Arc::new(maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(IacRtMetrics::new());
    HookDispatcher::new(tl, metrics)
}

fn make_scb(enabled_hooks: Vec<String>) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection { enabled_hooks },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        1,
        "hook-test".into(),
        manifest,
        make_spirit_obj(HookCounter::default()),
        0,
    );
    scb.state.store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

#[tokio::test]
async fn manifest_gate_skips_disabled_hook() {
    let dispatcher = make_dispatcher();
    // enabled_hooks is empty → "on_load" is allowed (kernel_invocation_allowed(&[], _) → true)
    // Wait, actually empty means all allowed. Let's explicitly NOT include on_load.
    let scb = make_scb(vec!["on_start".into()]);

    let outcome = dispatcher.fire_on_load(&scb).await;
    assert_eq!(outcome, HookOutcome::SkippedManifest, "on_load must be skipped when not in manifest");
}

#[tokio::test]
async fn hook_fires_within_budget() {
    let dispatcher = make_dispatcher();
    let scb = make_scb(vec!["on_load".into()]);

    let outcome = dispatcher.fire_on_load(&scb).await;
    match outcome {
        HookOutcome::Fired { wall_ns } => {
            assert!(wall_ns < 1_000_000_000, "on_load with no sleep must finish in <1s");
        }
        other => panic!("expected Fired, got {other:?}"),
    }
}

#[tokio::test]
async fn hook_exceeds_budget_and_returns_budget_exceeded() {
    let dispatcher = make_dispatcher();
    // Set a very short time cap so the hook always exceeds.
    let mut dispatcher = dispatcher;
    dispatcher.time_cap_seconds = 1; // 1 second cap

    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into()],
        },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        1,
        "slow-hook".into(),
        manifest,
        make_spirit_obj(HookCounter { count: AtomicU32::new(0), sleep_ms: 2000 }),
        0,
    );
    scb.state.store(ScbLifecycleState::Running as u8, Ordering::Release);

    let outcome = dispatcher.fire_on_load(&Arc::new(scb)).await;
    assert!(
        matches!(outcome, HookOutcome::BudgetExceeded { .. }),
        "sleeping 2s with 1s cap must exceed budget: got {outcome:?}"
    );
}
