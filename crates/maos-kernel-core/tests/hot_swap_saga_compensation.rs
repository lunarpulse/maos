#![forbid(unsafe_code)]

//! Regression proof that compensation restores a runtime snapshot without
//! replacing the scheduler's stable SCB Arc.

use std::sync::Arc;

use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, ScbRuntimeSnapshot, SpiritControlBlock,
    SpiritManifestBundle,
};

struct Predecessor;
impl maos_spirit_abi::lifecycle::Spirit for Predecessor {}
struct Successor;
impl maos_spirit_abi::lifecycle::Spirit for Successor {}

#[test]
fn swap_in_failure_restores_the_predecessor_runtime_on_the_same_control_block() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scb = Arc::new(SpiritControlBlock::new(
        42,
        "saga-spirit".into(),
        SpiritManifestBundle::default(),
        make_spirit_obj(Predecessor),
        0xCAFE,
    ));
    scb.state.store(
        ScbLifecycleState::Running as u8,
        std::sync::atomic::Ordering::Release,
    );
    let stale_clone = Arc::clone(&scb);
    let predecessor = scb.runtime_snapshot();
    let replacement = ScbRuntimeSnapshot {
        manifest: predecessor.manifest.clone(),
        spirit_obj: make_spirit_obj(Successor),
        priority_weight: predecessor.priority_weight,
        on_crash_action: predecessor.on_crash_action.clone(),
        on_revocation_action: predecessor.on_revocation_action,
        sandbox_tier: predecessor.sandbox_tier,
    };

    let rollback = scb.replace_runtime(replacement);
    assert!(!Arc::ptr_eq(
        &predecessor.spirit_obj,
        &stale_clone.runtime_snapshot().spirit_obj,
    ));
    stale_clone.replace_runtime(rollback);

    assert!(Arc::ptr_eq(&scb, &stale_clone));
    assert_eq!(stale_clone.pid, 42);
    assert_eq!(stale_clone.current_state(), ScbLifecycleState::Running);
    assert!(Arc::ptr_eq(
        &predecessor.spirit_obj,
        &stale_clone.runtime_snapshot().spirit_obj,
    ));
}
