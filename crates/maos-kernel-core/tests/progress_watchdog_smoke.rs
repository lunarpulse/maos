#![forbid(unsafe_code)]

//! Integration test: ProgressWatchdog emits TaskStalled on hung Spirit.
//!
//! Story 5.3 — AC2.

use std::sync::Arc;

use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::SpiritManifestBundle;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct IdleSpirit;
impl maos_spirit_abi::lifecycle::Spirit for IdleSpirit {}

#[tokio::test]
async fn progress_watchdog_emits_task_stalled() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    std::env::set_var("MAOS_SUPERVISION_FAST", "1");

    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD));
    let metrics = Arc::new(IacRtMetrics::new());
    let scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
                0,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                maos_kernel_core::capability::cap_audit::channel().0,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ),
        ),
        Arc::new({
            let tmp_mem = tempfile::TempDir::new().unwrap();
            let db_path = tmp_mem.path().join("audit.db");
            let memory_root = tmp_mem.path().join("memory");
            maos_kernel_core::memory::MemoryManagerAdapter::new(
                Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                    memory_root,
                    4,
                )),
                Arc::new(
                    maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap(),
                ),
                Arc::new(
                    maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path)
                        .unwrap(),
                ),
                Arc::clone(&tl),
            )
        }),
        Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
            Arc::clone(&tl),
        )),
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        Arc::clone(&metrics),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));

    let manifest = SpiritManifestBundle::default();
    let pid = scheduler
        .load("progress-watchdog-test", manifest, IdleSpirit, 12346)
        .await
        .unwrap();
    scheduler.start(pid).await.unwrap();

    // Inject fake in-flight task + backdate progress
    {
        let spirits = scheduler.scbs();
        let map = spirits.read().unwrap();
        let scb = map.get(&pid).cloned().unwrap();
        drop(map);
        let mut tasks = scb.task_assignments_in_flight.lock().unwrap();
        tasks.push(maos_domain::ports::task::TaskAssignmentRecord {
            task_id: "hung-task-001".into(),
            capability_token: maos_domain::invariants::i1::TokenId([0u8; 16]),
            ttl_deadline_ns: u64::MAX,
            intent_class: maos_domain::invariants::i1::IntentClass::Standard,
            originator_spirit_id: "progress-watchdog-test".into(),
        });
        let now = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
        scb.last_progress_iac_ns.store(
            now.saturating_sub(30_000_000_000),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    // Spawn local watchdog
    let cancel = tokio_util::sync::CancellationToken::new();
    let _watchdog = Arc::new(maos_kernel_core::supervision::ProgressWatchdog::new(
        scheduler.scbs(),
        Arc::clone(&tl),
        Arc::clone(&metrics),
        Arc::new(maos_director_surface::notification::NotificationDispatcher::new()),
    ))
    .spawn(cancel.child_token());

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    cancel.cancel();

    let stalled = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::TaskStalled),
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap();
    assert!(
        stalled.len() >= 1,
        "expected >=1 TaskStalled frame, got {}",
        stalled.len()
    );
}
