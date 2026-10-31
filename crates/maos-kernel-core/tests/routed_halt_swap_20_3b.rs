#![forbid(unsafe_code)]

//! AC5 FR53: an already-active halt, not a successor re-fire. The real swap
//! currently drains the halt and metadata. Repair belongs to 20-3c member 12.
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::HaltState;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::iac::{transparency_log::TransparencyLogAdapter, IacBusAdapter, Mailbox};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::scheduler::hook_dispatch::HookDispatcher;
use maos_kernel_core::security::manifest::LifecycleSection;
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use maos_spirit_abi::ctx::Ctx;
use maos_spirit_abi::lifecycle::{Spirit, SwapInPayload};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

struct ReplaySpirit(Arc<Mutex<Vec<u8>>>);
impl Spirit for ReplaySpirit {
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
    fn on_swap_in<'a>(&self, _ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        *self.0.lock().unwrap() = payload.predecessor_state.to_vec();
    }
}

fn replay_context(registry: &HaltRegistry) -> Vec<(String, u32, String, u64, Vec<u8>)> {
    registry
        .halt_metadata_for_spirit(7, 0)
        .into_iter()
        .map(|(id, meta)| {
            (
                id.as_str().to_owned(),
                meta.spirit_pid,
                meta.spirit_id,
                meta.fired_ns,
                serde_json::to_vec(&meta.payload).expect("halt payload"),
            )
        })
        .collect()
}

#[tokio::test]
#[ignore = "owned by 20-3c: member 12 swap-halt receipt/continuity; validate_swap_halt_continuity drains the active halt and replay metadata before swap (20-3b AC5.4, deferred-work)"]
async fn active_halt_keeps_identity_and_replay_context_across_hot_swap() {
    cap_tokens::init_monotonic_base();
    let tmp = tempfile::tempdir().unwrap();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x203b));
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap());
    let registry = Arc::new(HaltRegistry::new());
    let metrics = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0; 32]),
        0x203b,
        Arc::new(PolicyTable::new()),
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let manifest = SpiritManifestBundle {
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["snapshot".into(), "on_swap_out".into(), "on_swap_in".into()],
        },
        ..Default::default()
    };
    let before_state = b"replay:frame:origin-203b;policy:clarify;context:original".to_vec();
    let scb = Arc::new(SpiritControlBlock::new(
        7,
        "halt-witness".into(),
        manifest.clone(),
        make_spirit_obj(ReplaySpirit(Arc::new(Mutex::new(before_state.clone())))),
        0x203b,
    ));
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    let spirits = Arc::new(RwLock::new(BTreeMap::from([(7, Arc::clone(&scb))])));
    let payload = EpistemicHaltPayload::new(
        "halt-fr53-original".into(),
        "claim.ambiguity".into(),
        0.9,
        Some(0.8),
        "policy:clarify".into(),
        "frame:origin-203b".into(),
    )
    .unwrap();
    let receipt =
        invoke_halt(&tl, &journal, &registry, payload, 7, "halt-witness", 0x203b).unwrap();
    assert_eq!(
        registry.lookup_state(&receipt.halt_id),
        Some(HaltState::PendingResolution)
    );
    let before = replay_context(&registry);
    assert_eq!(before.len(), 1, "the active halt has real replay metadata");
    let coordinator = HotSwapCoordinator::new(
        Arc::clone(&spirits),
        journal,
        Arc::clone(&tl),
        Arc::clone(&registry),
        capability,
        iac,
        Arc::new(HookDispatcher::new(tl, Arc::clone(&metrics))),
        metrics,
        tmp.path().join("archives"),
    );
    let successor = Arc::new(Mutex::new(Vec::new()));
    coordinator
        .initiate_swap(
            "halt-witness",
            &manifest,
            make_spirit_obj(ReplaySpirit(Arc::clone(&successor))),
        )
        .await
        .expect("real same-major hot swap");
    assert_eq!(
        *successor.lock().unwrap(),
        before_state,
        "successor replay snapshot"
    );
    assert!(Arc::ptr_eq(spirits.read().unwrap().get(&7).unwrap(), &scb));
    assert_eq!(
        (registry.pending_halt_ids(), replay_context(&registry)),
        (vec![receipt.halt_id.clone()], before),
        "FR53 preserves the ORIGINAL active halt id, source-frame/policy replay payload, owner and fire time; re-firing is not continuity",
    );
    assert_eq!(
        registry.lookup_state(&receipt.halt_id),
        Some(HaltState::PendingResolution)
    );
}
