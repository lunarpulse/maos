#![forbid(unsafe_code)]

//! Integration test: HookDispatcher swap hooks (smoke test per AC6).

use std::sync::Arc;

use maos_kernel_core::scheduler::control_block::{SpiritControlBlock, SpiritManifestBundle, make_spirit_obj};
use maos_kernel_core::scheduler::hook_dispatch::{HookDispatcher, HookOutcome};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_spirit_abi::lifecycle::{MigratorError, Spirit};

struct HookDummySpirit;

impl Spirit for HookDummySpirit {}

fn test_dispatcher() -> HookDispatcher {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xCAFE));
    let metrics = Arc::new(IacRtMetrics::new());
    HookDispatcher::new(tl, metrics)
}

fn test_scb() -> SpiritControlBlock {
    let obj = make_spirit_obj(HookDummySpirit);
    SpiritControlBlock::new(
        1,
        "hook-test".into(),
        SpiritManifestBundle::default(),
        obj,
        0xCAFE,
    )
}

#[tokio::test]
async fn default_on_swap_out_fires_noop() {
    let dispatcher = test_dispatcher();
    let scb = test_scb();
    let outcome = dispatcher.fire_on_swap_out(&scb).await;
    assert!(matches!(
        outcome,
        HookOutcome::Fired { .. } | HookOutcome::SkippedManifest
    ));
}

#[tokio::test]
async fn default_snapshot_returns_empty_vec() {
    let dispatcher = test_dispatcher();
    let scb = test_scb();
    let result = dispatcher.fire_snapshot(&scb).await;
    assert!(result.is_ok(), "default snapshot should succeed");
    assert_eq!(result.unwrap(), Vec::<u8>::new());
}

#[tokio::test]
async fn default_migrate_returns_not_implemented() {
    let dispatcher = test_dispatcher();
    let scb = test_scb();
    let result = dispatcher.fire_migrate(&scb, b"test-payload").await;
    assert!(result.is_err(), "default migrate should error");
    assert!(matches!(result.unwrap_err(), MigratorError::NotImplemented));
}
