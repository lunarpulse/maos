#![forbid(unsafe_code)]

//! Integration test: in-process panic → CrashDetector → SCB removal + halt receipt.
//!
//! Story 5.3 — AC1.

use std::sync::Arc;

use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::SpiritManifestBundle;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct PanicSpirit;
impl maos_spirit_abi::lifecycle::Spirit for PanicSpirit {
    fn on_start(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        panic!("crash-detector-test: synthetic panic");
    }
}

#[tokio::test]
async fn panic_hook_triggers_crash_detector() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();

    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEEF));
    let metrics = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(
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
    );
    let tmp_mem = tempfile::TempDir::new().unwrap();
    let db_path = tmp_mem.path().join("audit.db");
    let memory_root = tmp_mem.path().join("memory");
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
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        Arc::clone(&memory),
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&metrics),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));

    let mut scheduler_mut = scheduler;
    let journal_path = std::env::temp_dir().join(format!(
        "maos-crash-detector-test-{}.ndjson",
        std::process::id()
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&journal_path).expect("journal open"),
    );
    let crash_detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
        scheduler_mut.scbs(),
        Arc::clone(&tl),
        Arc::clone(&halt_registry),
        Arc::clone(&capability),
        Arc::clone(&iac),
        Arc::clone(&metrics),
        journal,
    ));
    Arc::get_mut(&mut scheduler_mut)
        .unwrap()
        .set_crash_detector(crash_detector);

    let manifest = SpiritManifestBundle::default();
    let pid = scheduler_mut
        .load("crash-detector-panic-test", manifest, PanicSpirit, 12345)
        .await
        .unwrap();

    // start() catches the panic and spawns the crash handler
    let _ = scheduler_mut.start(pid).await;

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // SCB should be removed
    let spirits = scheduler_mut.scbs();
    let map = spirits.read().unwrap();
    assert!(map.get(&pid).is_none(), "SCB must be removed after panic");
    drop(map);

    // At least one EpistemicHalt frame (halt receipt) should exist
    let receipts = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap();
    assert!(
        receipts.len() >= 1,
        "expected >=1 halt receipt, got {}",
        receipts.len()
    );
}
