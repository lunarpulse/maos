#![forbid(unsafe_code)]

//! Integration test: five lifecycle verbs (load / start / pause / resume / unload)
//! routed through the real SpiritSchedulerAdapter (AC1).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_domain::lifecycle::LifecycleError;
use maos_kernel_core::scheduler::{
    control_block::SpiritManifestBundle, scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct TestSpirit {
    on_load: AtomicU32,
    on_start: AtomicU32,
    on_pause: AtomicU32,
    on_resume: AtomicU32,
    on_unload: AtomicU32,
}

impl Default for TestSpirit {
    fn default() -> Self {
        Self {
            on_load: AtomicU32::new(0),
            on_start: AtomicU32::new(0),
            on_pause: AtomicU32::new(0),
            on_resume: AtomicU32::new(0),
            on_unload: AtomicU32::new(0),
        }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_load.fetch_add(1, Ordering::Relaxed);
    }
    fn on_start(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_start.fetch_add(1, Ordering::Relaxed);
    }
    fn on_pause(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_pause.fetch_add(1, Ordering::Relaxed);
    }
    fn on_resume(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_resume.fetch_add(1, Ordering::Relaxed);
    }
    fn on_unload(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.on_unload.fetch_add(1, Ordering::Relaxed);
    }
}

fn make_scheduler() -> Arc<SpiritSchedulerAdapter> {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("audit.db");
    let memory_root = tmp.path().join("memory");

    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0xBEEF),
    );
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
            0xBEEF,
            Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            memory_root,
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap(),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let telemetry = Arc::new(IacRtMetrics::new());

    Arc::new(SpiritSchedulerAdapter::new(
        tl,
        capability,
        memory,
        iac,
        halt_registry,
        telemetry,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ))
}

#[tokio::test]
async fn five_verb_lifecycle_routes_through_scheduler() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scheduler = make_scheduler();
    let spirit = TestSpirit::default();
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec![
                "on_load".into(),
                "on_start".into(),
                "on_pause".into(),
                "on_resume".into(),
                "on_unload".into(),
            ],
        },
        ..Default::default()
    };

    // 1. Load
    let pid = scheduler
        .load("test-spirit", manifest, spirit, 0xBEEF)
        .await
        .expect("load");
    assert_eq!(scheduler.resolve_pid("test-spirit"), Some(pid), "pid must resolve to the loaded spirit");

    // 2. Start
    scheduler.start(pid).await.expect("start");

    // 3. Pause
    scheduler.pause(pid).await.expect("pause");

    // 4. Resume
    scheduler.resume(pid).await.expect("resume");

    // 5. Unload
    scheduler.unload(pid).await.expect("unload");

    // Verify SCB is removed.
    assert_eq!(
        scheduler.resolve_pid("test-spirit"),
        None,
        "SCB must be removed after unload"
    );
}

#[tokio::test]
async fn unload_is_idempotent() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scheduler = make_scheduler();
    let spirit = TestSpirit::default();
    let manifest = SpiritManifestBundle::default();

    let pid = scheduler
        .load("idempotent-spirit", manifest, spirit, 0)
        .await
        .expect("load");
    scheduler.start(pid).await.expect("start");
    scheduler.unload(pid).await.expect("first unload");
    scheduler
        .unload(pid)
        .await
        .expect("second unload must be idempotent");
}

#[tokio::test]
async fn invalid_state_transition_rejected() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scheduler = make_scheduler();
    let spirit = TestSpirit::default();
    let manifest = SpiritManifestBundle::default();

    let pid = scheduler
        .load("transition-spirit", manifest, spirit, 0)
        .await
        .expect("load");

    // Start without load transition is invalid — but load already sets Loaded.
    // Try pause on Loaded (not Running).
    let err = scheduler.pause(pid).await.unwrap_err();
    assert!(
        matches!(err, LifecycleError::InvalidStateTransition { .. }),
        "pause on Loaded must fail: {err}"
    );
}
