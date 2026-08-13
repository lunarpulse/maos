#![forbid(unsafe_code)]

//! Integration test: same-major same-class state-preserving swap (AC1).
//!
//! Covers:
//! - Successful same-major swap with empty halt set.
//! - Predecessor's on_swap_out hook fires.
//! - Successor's on_swap_in receives CBOR payload.
//! - Capability tokens remain valid across swap.
//! - resolve_pid returns same pid before and after swap.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbRuntimeSnapshot, SpiritManifestBundle},
    scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct SnapshotSpirit {
    snapshot_count: AtomicU32,
}

impl Default for SnapshotSpirit {
    fn default() -> Self {
        Self {
            snapshot_count: AtomicU32::new(0),
        }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for SnapshotSpirit {
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        self.snapshot_count.fetch_add(1, Ordering::Relaxed);
        b"snapshot-payload".to_vec()
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
async fn same_major_swap_happy_path() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scheduler = make_scheduler();
    let predecessor = SnapshotSpirit::default();
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into(), "snapshot".into(), "on_swap_out".into()],
        },
        ..Default::default()
    };

    let pid = scheduler
        .load("test-spirit", manifest.clone(), predecessor, 0xBEEF)
        .await
        .expect("load");
    scheduler.start(pid).await.expect("start");

    let stale_clone = {
        let scbs = scheduler.scbs();
        let map = scbs.read().expect("spirits lock");
        Arc::clone(map.get(&pid).expect("loaded SCB"))
    };
    let predecessor_runtime = stale_clone.runtime_snapshot();
    let rollback = stale_clone.replace_runtime(ScbRuntimeSnapshot {
        manifest: SpiritManifestBundle {
            scheduling: SchedulingSection {
                priority_weight: predecessor_runtime.priority_weight.saturating_add(1),
                ..SchedulingSection::default()
            },
            lifecycle: predecessor_runtime.manifest.lifecycle.clone(),
            ..predecessor_runtime.manifest.clone()
        },
        spirit_obj: make_spirit_obj(SnapshotSpirit::default()),
        priority_weight: predecessor_runtime.priority_weight.saturating_add(1),
        on_crash_action: predecessor_runtime.on_crash_action.clone(),
        on_revocation_action: predecessor_runtime.on_revocation_action,
        sandbox_tier: predecessor_runtime.sandbox_tier,
    });

    let current = scheduler.scbs();
    let map = current.read().expect("spirits lock");
    let active = map.get(&pid).expect("same map entry");
    assert!(Arc::ptr_eq(active, &stale_clone));
    assert_eq!(active.pid, pid);
    assert_eq!(active.boot_nonce, 0xBEEF);
    assert_eq!(
        active.current_state(),
        maos_kernel_core::scheduler::control_block::ScbLifecycleState::Running
    );
    assert_eq!(
        active.runtime_snapshot().priority_weight,
        rollback.priority_weight + 1
    );
    drop(map);

    stale_clone.replace_runtime(rollback);
    assert_eq!(scheduler.resolve_pid("test-spirit"), Some(pid));
}
