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
    panic_message: Option<&'static str>,
}

impl Default for HookCounter {
    fn default() -> Self {
        Self {
            count: AtomicU32::new(0),
            sleep_ms: 0,
            panic_message: None,
        }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for HookCounter {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.count.fetch_add(1, Ordering::Relaxed);
        if self.sleep_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.sleep_ms));
        }
        if let Some(message) = self.panic_message {
            panic!("{message}");
        }
    }
}

fn make_dispatcher() -> HookDispatcher {
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0),
    );
    let metrics = Arc::new(IacRtMetrics::new());
    HookDispatcher::new(tl, metrics)
}

fn make_dispatcher_with_mailbox(
    spirit_id: &str,
) -> (HookDispatcher, maos_kernel_core::iac::SpiritMailboxHandle) {
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0),
    );
    let metrics = Arc::new(IacRtMetrics::new());
    let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
    let handle = mailbox
        .register_spirit(spirit_id)
        .expect("register mailbox");
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        mailbox,
        Arc::clone(&tl),
    ));
    (HookDispatcher::new(tl, metrics).with_iac(iac), handle)
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
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

#[tokio::test]
async fn manifest_gate_skips_disabled_hook() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let dispatcher = make_dispatcher();
    // enabled_hooks is empty → "on_load" is allowed (kernel_invocation_allowed(&[], _) → true)
    // Wait, actually empty means all allowed. Let's explicitly NOT include on_load.
    let scb = make_scb(vec!["on_start".into()]);

    let outcome = dispatcher.fire_on_load(&scb).await;
    assert_eq!(
        outcome,
        HookOutcome::SkippedManifest,
        "on_load must be skipped when not in manifest"
    );
}

#[tokio::test]
async fn hook_fires_within_budget() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let dispatcher = make_dispatcher();
    let scb = make_scb(vec!["on_load".into()]);

    let outcome = dispatcher.fire_on_load(&scb).await;
    match outcome {
        HookOutcome::Fired { wall_ns } => {
            assert!(
                wall_ns < 1_000_000_000,
                "on_load with no sleep must finish in <1s"
            );
        }
        other => panic!("expected Fired, got {other:?}"),
    }
}

#[tokio::test]
async fn hook_exceeds_budget_and_returns_budget_exceeded() {
    let (mut dispatcher, mut mailbox) = make_dispatcher_with_mailbox("slow-hook");
    // Set a very short time cap so the hook always exceeds.
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
        make_spirit_obj(HookCounter {
            count: AtomicU32::new(0),
            sleep_ms: 2000,
            panic_message: None,
        }),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);

    let outcome = dispatcher.fire_on_load(&Arc::new(scb)).await;
    assert!(
        matches!(outcome, HookOutcome::BudgetExceeded { .. }),
        "sleeping 2s with 1s cap must exceed budget: got {outcome:?}"
    );
    let (kind, frame) = tokio::time::timeout(std::time::Duration::from_secs(1), mailbox.recv())
        .await
        .expect("budget frame delivery")
        .expect("budget frame");
    assert_eq!(kind, maos_spirit_abi::identity::FrameKind::BudgetWarning);
    assert!(matches!(
        frame.payload,
        maos_domain::frame::FramePayload::BudgetWarning(
            maos_domain::frame::BudgetEnvelope {
                spirit_pid: 1,
                ref hook_name,
                cap_seconds: 1,
                ..
            }
        ) if hook_name == "on_load"
    ));
    let (kind, frame) = tokio::time::timeout(std::time::Duration::from_secs(1), mailbox.recv())
        .await
        .expect("terminal budget frame delivery")
        .expect("terminal budget frame");
    assert_eq!(kind, maos_spirit_abi::identity::FrameKind::BudgetExceeded);
    assert!(matches!(
        frame.payload,
        maos_domain::frame::FramePayload::BudgetExceeded(
            maos_domain::frame::BudgetEnvelope {
                spirit_pid: 1,
                ref hook_name,
                cap_seconds: 1,
                ..
            }
        ) if hook_name == "on_load"
    ));
}

#[tokio::test]
async fn hook_crosses_eighty_percent_before_timeout() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let (mut dispatcher, mut mailbox) = make_dispatcher_with_mailbox("warning-hook");
    dispatcher.time_cap_seconds = 1;
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into()],
        },
        ..Default::default()
    };
    let scb = Arc::new(SpiritControlBlock::new(
        7,
        "warning-hook".into(),
        manifest,
        make_spirit_obj(HookCounter {
            count: AtomicU32::new(0),
            sleep_ms: 900,
            panic_message: None,
        }),
        0,
    ));
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);

    let outcome = dispatcher.fire_on_load(&scb).await;
    assert!(
        matches!(outcome, HookOutcome::BudgetWarning80 { fired: true, .. }),
        "900ms hook must cross the 800ms warning boundary: {outcome:?}"
    );
    let (kind, frame) = tokio::time::timeout(std::time::Duration::from_secs(1), mailbox.recv())
        .await
        .expect("warning frame delivery")
        .expect("warning frame");
    assert_eq!(kind, maos_spirit_abi::identity::FrameKind::BudgetWarning);
    assert!(matches!(
        frame.payload,
        maos_domain::frame::FramePayload::BudgetWarning(
            maos_domain::frame::BudgetEnvelope {
                spirit_pid: 7,
                ref hook_name,
                cap_seconds: 1,
                ..
            }
        ) if hook_name == "on_load"
    ));
}

#[tokio::test]
async fn panicking_hook_returns_payload_preview() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let dispatcher = make_dispatcher();
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into()],
        },
        ..Default::default()
    };
    let scb = Arc::new(SpiritControlBlock::new(
        9,
        "panic-hook".into(),
        manifest,
        make_spirit_obj(HookCounter {
            count: AtomicU32::new(0),
            sleep_ms: 0,
            panic_message: Some("hook panic sentinel"),
        }),
        0,
    ));
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);

    let outcome = dispatcher.fire_on_load(&scb).await;
    assert!(matches!(
        outcome,
        HookOutcome::Panicked {
            ref panic_payload_preview
        } if panic_payload_preview == "hook panic sentinel"
    ));
}
