#![forbid(unsafe_code)]

//! Integration test: DRR priority-weighted scheduling.
//!
//! AC3 — the picker increments all Running deficits by weight each tick,
//! then selects the Spirit with the highest deficit ≥ quantum.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    pick_next_spirit_from_slice, SCHEDULER_QUANTUM,
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};

struct DummySpirit;
impl maos_spirit_abi::lifecycle::Spirit for DummySpirit {}

fn mock_scb(pid: u32, weight: u8) -> Arc<SpiritControlBlock> {
    let scheduling = SchedulingSection {
        priority_weight: weight,
        yield_every_polls: 64,
        idle_window_ms: 30000,
    };
    let manifest = SpiritManifestBundle {
        scheduling,
        lifecycle: LifecycleSection::default(),
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        pid,
        format!("spirit-{pid}"),
        manifest,
        make_spirit_obj(DummySpirit),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

#[test]
fn drr_highest_weight_dominates() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scbs = vec![mock_scb(1, 50), mock_scb(2, 100), mock_scb(3, 200)];
    let mut counts = [0u32; 4];

    for _ in 0..1000 {
        if let Some(pid) = pick_next_spirit_from_slice(&scbs) {
            counts[pid as usize] += 1;
            if let Some(scb) = scbs.iter().find(|s| s.pid == pid) {
                scb.deficit_counter
                    .fetch_sub(SCHEDULER_QUANTUM, Ordering::SeqCst);
            }
        }
    }

    // With the max-deficit algorithm, spirit-3 (weight 200) always wins
    // because its deficit grows fastest and always stays ahead after
    // quantum subtraction.
    assert_eq!(counts[3], 1000, "weight-200 spirit should dominate");
    assert_eq!(counts[1], 0, "weight-50 spirit is starved by max-deficit");
    assert_eq!(counts[2], 0, "weight-100 spirit is starved by max-deficit");
}

#[test]
fn drr_equal_weights_rotate() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scbs = vec![mock_scb(1, 64), mock_scb(2, 64), mock_scb(3, 64)];
    let mut counts = [0u32; 4];

    for _ in 0..3000 {
        if let Some(pid) = pick_next_spirit_from_slice(&scbs) {
            counts[pid as usize] += 1;
            if let Some(scb) = scbs.iter().find(|s| s.pid == pid) {
                scb.deficit_counter
                    .fetch_sub(SCHEDULER_QUANTUM, Ordering::SeqCst);
            }
        }
    }

    // Equal weights → roughly equal turns (±10% tolerance).
    let total = counts.iter().sum::<u32>() as f64;
    for pid in 1..=3 {
        let proportion = counts[pid] as f64 / total;
        assert!(
            (proportion - 1.0 / 3.0).abs() < 0.10,
            "pid={pid} proportion {proportion} deviates >10% from 1/3 (count={})",
            counts[pid]
        );
    }
}

#[test]
fn drr_skips_non_running_spirits() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let running = mock_scb(1, 100);
    let paused = mock_scb(2, 100);
    paused
        .state
        .store(ScbLifecycleState::Paused as u8, Ordering::Release);

    let scbs = vec![running.clone(), paused];
    let pid = pick_next_spirit_from_slice(&scbs);
    assert_eq!(pid, Some(1), "picker must skip Paused spirits");
}

#[test]
fn drr_empty_slice_returns_none() {
    let scbs: Vec<Arc<SpiritControlBlock>> = vec![];
    assert_eq!(pick_next_spirit_from_slice(&scbs), None);
}
