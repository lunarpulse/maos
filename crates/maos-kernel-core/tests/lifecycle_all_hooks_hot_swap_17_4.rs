#![forbid(unsafe_code)]

//! Explicit registration of every ABI hook must permit state-preserving hot-swap.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, RwLock,
};

use maos_domain::hot_swap::SchemaCompat;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
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

#[derive(Default)]
struct Calls {
    snapshot: AtomicU32,
    swap_out: AtomicU32,
    swap_in: AtomicU32,
    restored: Mutex<Vec<u8>>,
}

struct StatefulSpirit(Arc<Calls>);

impl Spirit for StatefulSpirit {
    fn on_swap_out(&self, _ctx: &mut Ctx) {
        self.0.swap_out.fetch_add(1, Ordering::SeqCst);
    }

    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        self.0.snapshot.fetch_add(1, Ordering::SeqCst);
        // CBOR unsigned integer 42, owned and interpreted by the Spirit.
        vec![0x18, 0x2a]
    }

    fn on_swap_in<'a>(&self, _ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        self.0.swap_in.fetch_add(1, Ordering::SeqCst);
        *self.0.restored.lock().expect("restored state lock") = payload.predecessor_state.to_vec();
    }
}

#[tokio::test]
async fn validated_all_fourteen_hooks_hot_swap_and_invoke_snapshot() {
    cap_tokens::init_monotonic_base();
    let manifest_toml = r#"
[lifecycle]
enabled_hooks = [
    "on_load", "on_start", "on_frame", "on_idle", "on_telemetry_event",
    "on_schedule", "on_swap_in", "on_pause", "on_resume", "on_unload",
    "on_consolidate", "on_swap_out", "snapshot", "migrate"
]
"#;
    let document: toml::Value = toml::from_str(manifest_toml).expect("manifest TOML");
    let lifecycle_toml = toml::to_string(&document["lifecycle"]).expect("lifecycle TOML");
    let lifecycle = LifecycleSection::from_toml_str(&lifecycle_toml)
        .expect("all fourteen ABI hooks must pass the real manifest validator before hot-swap");
    assert_eq!(
        lifecycle.enabled_hooks.len(),
        maos_spirit_abi::count_hooks!()
    );
    let manifest = SpiritManifestBundle {
        lifecycle,
        ..Default::default()
    };

    let tmp = tempfile::TempDir::new().expect("hot-swap tempdir");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x174));
    let metrics = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0; 32]),
        0x174,
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
    let dispatcher = Arc::new(HookDispatcher::new(Arc::clone(&tl), Arc::clone(&metrics)));
    let journal = Arc::new(
        JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("hot-swap journal"),
    );
    let predecessor_calls = Arc::new(Calls::default());
    let predecessor_scb = Arc::new(SpiritControlBlock::new(
        7,
        "explicit-hooks".into(),
        manifest.clone(),
        make_spirit_obj(StatefulSpirit(Arc::clone(&predecessor_calls))),
        0x174,
    ));
    predecessor_scb
        .state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    let spirits = Arc::new(RwLock::new(BTreeMap::from([(
        7,
        Arc::clone(&predecessor_scb),
    )])));
    let coordinator = HotSwapCoordinator::new(
        Arc::clone(&spirits),
        journal,
        tl,
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        capability,
        iac,
        dispatcher,
        metrics,
        tmp.path().join("archives"),
    );
    let successor_calls = Arc::new(Calls::default());
    let result = coordinator
        .initiate_swap(
            "explicit-hooks",
            &manifest,
            make_spirit_obj(StatefulSpirit(Arc::clone(&successor_calls))),
        )
        .await
        .expect("a validated all-hook manifest must complete the real hot-swap path");

    assert_eq!(result.schema_compat, SchemaCompat::SameMajor);
    assert_eq!(
        predecessor_calls.snapshot.load(Ordering::SeqCst),
        1,
        "snapshot must be invoked, not SkippedManifest"
    );
    assert_eq!(predecessor_calls.swap_out.load(Ordering::SeqCst), 1);
    assert_eq!(successor_calls.swap_in.load(Ordering::SeqCst), 1);
    assert_eq!(
        successor_calls
            .restored
            .lock()
            .expect("restored state lock")
            .as_slice(),
        [0x18, 0x2a]
    );
    let active = spirits.read().expect("SCB map");
    let scb = active.get(&7).expect("successor SCB");
    assert!(Arc::ptr_eq(scb, &predecessor_scb));
    assert_eq!(scb.current_state(), ScbLifecycleState::Running);
    assert_eq!(
        scb.runtime_snapshot().manifest.lifecycle,
        manifest.lifecycle
    );
}
