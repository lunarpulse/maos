#![forbid(unsafe_code)]

//! later-registry-quarantine-closure (FR13) — kernel witnesses: a quarantined pid's own effects
//! are refused with typed outcomes, and kernel-origin work is not. This fences effects; it is NOT
//! process or container isolation (ruling `q`).
//!
//! Planted reds (orchestrator runs them; restore after each, expect the named test to fail):
//!   * memory: delete the `is_quarantined` guard in `MemoryManagerPort::write`
//!     (`crates/maos-kernel-core/src/memory/mod.rs`)            -> `..._memory_write_...`
//!   * memory: make `KernelHaltResolver` call the fenced `write` instead of `write_kernel_origin`,
//!     or the marker call the fenced `set_scalar`               -> `..._memory_write_...`
//!   * scalar: delete the guard in `CapabilityRegistryAdapter::set_scalar` -> `..._scalar_write_...`
//!   * hooks: drop `FENCED_HOOKS` handling in `fire_payload_hook`           -> `..._event_hooks_...`
//!   * sender: delete `set_sender_fenced(.., true)` in the CRL Quarantine arm, or the clear in
//!     `unload`                                             -> `crl_quarantine_sets_and_unload_...`
//!
//! The poller witness `poller_reports_an_error_class_change_once_and_counts_repeats` lives in
//! `src/revocation/poller.rs` (it exercises the crate-private reporter).
//!
//! Run: `cargo nextest run -p maos-kernel-core --test quarantine_closure_lrqc`

use std::sync::atomic::Ordering;
use std::sync::Arc;

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltResolver, Resolution};
use maos_domain::memory::{MemoryError, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::SetScalarError;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry, KernelHaltResolver, OutputMarkerRegistry};
use maos_kernel_core::iac::Mailbox;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::{HookDispatcher, HookOutcome},
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

#[path = "support/quarantine_world.rs"]
mod quarantine_world;
use quarantine_world::{crl_naming, fixture, manifest, TestSpirit};

#[test]
fn quarantined_pid_memory_write_is_refused_typed_and_kernel_origin_write_is_not() {
    let fx = fixture();
    let (pid, bystander) = (7u32, 8u32);
    let write = |pid: u32, tier: MemoryTier| {
        fx.memory.write(
            pid,
            tier,
            &MemoryNamespace::Default,
            "k",
            MemoryValue::Text("v".into()),
        )
    };
    write(pid, MemoryTier::Private).expect("an unfenced pid writes");
    fx.capability.set_quarantined(pid, true);
    for tier in [MemoryTier::Private, MemoryTier::Shared] {
        let refused = write(pid, tier);
        assert!(
            matches!(refused, Err(MemoryError::Quarantined { spirit_pid }) if spirit_pid == pid),
            "{tier:?}: {refused:?}"
        );
    }
    write(bystander, MemoryTier::Private).expect("a bystander is untouched");

    // Kernel-origin: the operator's halt context still reaches the quarantined Spirit's private
    // tier, and its marker scalar is published (neither goes through the fenced entries).
    let registry = Arc::new(HaltRegistry::new());
    let metrics = Arc::new(IacRtMetrics::new());
    let resolver = KernelHaltResolver::new(
        Arc::clone(&registry),
        Arc::clone(&fx.tl),
        Arc::new(OutputMarkerRegistry::new()),
        Arc::new(Mailbox::new(metrics)),
        0xBEEF,
        Arc::clone(&fx.memory),
        Arc::new(WorkingMemoryOrchestrator::new(
            Arc::clone(&fx.capability),
            Arc::clone(&registry),
        )),
    );
    let journal = JournalAdapter::open(&fx._tmp.path().join("halt-journal.ndjson")).unwrap();
    let payload = EpistemicHaltPayload::new(
        "halt-q".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    invoke_halt(
        &fx.tl,
        &journal,
        &registry,
        payload,
        pid,
        "quarantined",
        0xBEEF,
    )
    .unwrap();
    resolver
        .resolve(
            &HaltId::new("halt-q").unwrap(),
            Resolution::ProvidedContext {
                text: "operator context".into(),
            },
        )
        .expect("the operator's halt resolution is not refused by the quarantine");
    let context = fx
        .memory
        .read(
            pid,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "halt_context::halt-q",
        )
        .unwrap();
    assert!(
        matches!(&context, Some(MemoryValue::Text(text)) if text == "operator context"),
        "{context:?}"
    );
    fx.capability.set_quarantined(pid, false);
    write(pid, MemoryTier::Private).expect("release restores the Spirit's own writes");
}

#[test]
fn quarantined_pid_scalar_write_is_refused_and_publishes_no_tap() {
    let fx = fixture();
    let pid = 11u32;
    let set = |value: f64| {
        fx.capability
            .set_scalar(pid, "s", "uncertainty", value, "frame")
    };
    set(1.0).expect("an unfenced pid sets a scalar");
    fx.capability.set_quarantined(pid, true);
    let refused = set(2.0);
    assert!(
        matches!(&refused, Err(SetScalarError::Quarantined { spirit_pid }) if *spirit_pid == pid),
        "{refused:?}"
    );
    assert_eq!(
        fx.capability
            .working_memory()
            .get_scalar(pid, "uncertainty")
            .map(|slot| slot.0),
        Some(1.0),
        "the refused write changed nothing (and so emitted no scalar.tap)"
    );
    fx.capability.set_quarantined(pid, false);
    set(3.0).expect("release restores scalar writes");
}

#[tokio::test]
async fn quarantined_pid_event_hooks_are_refused_without_a_strike() {
    let fx = fixture();
    let dispatcher = HookDispatcher::new(Arc::clone(&fx.tl), Arc::new(IacRtMetrics::new()))
        .with_capability(Arc::clone(&fx.capability));
    let scb = SpiritControlBlock::new(
        21,
        "hooked".into(),
        SpiritManifestBundle::default(),
        make_spirit_obj(TestSpirit),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    fx.capability.set_quarantined(21, true);
    let outcomes = [
        dispatcher.fire_on_frame(&scb, b"{}").await,
        dispatcher.fire_on_telemetry_event(&scb, b"{}").await,
        dispatcher.fire_on_schedule(&scb, b"{}").await,
        dispatcher.fire_on_idle(&scb).await,
        dispatcher.fire_on_consolidate(&scb, b"{}").await,
    ];
    for outcome in outcomes {
        assert_eq!(outcome, HookOutcome::SkippedQuarantined);
    }
    // Operator lifecycle hooks still fire; a refusal is never a crash or a strike.
    assert!(matches!(
        dispatcher.fire_on_pause(&scb).await,
        HookOutcome::Fired { .. }
    ));
    fx.capability.set_quarantined(21, false);
    assert!(matches!(
        dispatcher.fire_on_frame(&scb, b"{}").await,
        HookOutcome::Fired { .. }
    ));
}

#[tokio::test]
async fn crl_quarantine_sets_and_unload_clears_the_sender_fence() {
    let fx = fixture();
    let mut pids = Vec::new();
    for (name, boot) in [("fenced-class", 1u64), ("bystander-class", 2)] {
        let pid = fx
            .scheduler
            .load(name, manifest(name), TestSpirit, boot)
            .await
            .unwrap();
        fx.scheduler.start(pid).await.unwrap();
        pids.push(pid);
    }
    let (target, bystander) = (pids[0], pids[1]);
    assert!(!fx.iac.is_sender_fenced(target));
    fx.applier
        .apply_crl(crl_naming("fenced-class"))
        .await
        .unwrap();
    assert!(fx.capability.is_quarantined(target));
    assert!(
        fx.iac.is_sender_fenced(target),
        "the CRL arm fences the sender"
    );
    assert!(!fx.iac.is_sender_fenced(bystander));
    // A director pause is not a quarantine: it never sets the sender fence.
    fx.capability.set_paused(bystander, true);
    assert!(!fx.iac.is_sender_fenced(bystander));
    // The CRL itself still matches the identity (release is refused while it does).
    assert_eq!(fx.applier.matching_crls("fenced-class", "1.2.3").len(), 1);
    assert!(fx
        .applier
        .matching_crls("bystander-class", "1.2.3")
        .is_empty());
    fx.scheduler.unload(target).await.unwrap();
    assert!(
        !fx.iac.is_sender_fenced(target),
        "unload ends the sender fence"
    );
    assert!(!fx.capability.is_quarantined(target));
}
