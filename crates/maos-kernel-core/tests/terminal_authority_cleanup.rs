#![forbid(unsafe_code)]

//! Terminal release owners (scheduler unload, crash handler) withdraw only the
//! releasing pid's own principal-write grant and promotion registration.

use std::sync::Arc;

use maos_domain::invariants::i13::AllowedPromotionSet;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_domain::supervision::CrashCause;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::scheduler::SpiritManifestBundle;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct Quiet;
impl maos_spirit_abi::lifecycle::Spirit for Quiet {}

struct World {
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    crash: Arc<maos_kernel_core::supervision::CrashDetector>,
    memory: Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
    _tmp: tempfile::TempDir,
}

fn world() -> World {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xCAFE));
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
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("audit.db");
    let memory = Arc::new(
        maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                tmp.path().join("memory"),
                4096,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path)
                    .unwrap(),
            ),
            Arc::clone(&tl),
        )
        .with_principal_write_enforcement(),
    );
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
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&tmp.path().join("journal.ndjson"))
            .expect("journal open"),
    );
    let crash = Arc::new(
        maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            tl,
            halt_registry,
            capability,
            Arc::clone(&iac),
            metrics,
            journal,
        )
        .with_memory(Arc::clone(&memory)),
    );
    World {
        scheduler,
        crash,
        memory,
        iac,
        _tmp: tmp,
    }
}

fn can_write(memory: &maos_kernel_core::memory::MemoryManagerAdapter, pid: u32) -> bool {
    let ns = MemoryNamespace::Principal {
        principal_id: "p".into(),
        schema: "s".into(),
    };
    memory
        .write(
            pid,
            MemoryTier::Private,
            &ns,
            "k",
            MemoryValue::Text("v".into()),
        )
        .is_ok()
}

/// Loads `id`, grants write authority and registers an owned promotion set,
/// proving both are live before any terminal release runs.
async fn load_authorized(w: &World, id: &str, boot: u64) -> u32 {
    let pid = w
        .scheduler
        .load(id, SpiritManifestBundle::default(), Quiet, boot)
        .await
        .expect("load");
    assert!(
        !can_write(&w.memory, pid),
        "no write authority before grant"
    );
    w.memory.authorize_principal_writes(pid);
    w.iac
        .register_promotion_set_for_pid(id, pid, Some(AllowedPromotionSet::new()));
    assert!(can_write(&w.memory, pid), "grant makes writes succeed");
    assert!(w.iac.has_promotion_set(id), "promotion registered");
    pid
}

#[tokio::test]
async fn crash_deauthorizes_writes_and_withdraws_owned_promotion() {
    let w = world();
    let pid = load_authorized(&w, "crash-id", 1).await;
    let bystander = load_authorized(&w, "bystander-id", 2).await;

    w.crash
        .handle_crash(pid, CrashCause::Voluntary)
        .await
        .expect("crash handled");

    assert!(
        !can_write(&w.memory, pid),
        "crashed pid loses write authority"
    );
    assert!(
        !w.iac.has_promotion_set("crash-id"),
        "owned promotion withdrawn"
    );
    assert!(can_write(&w.memory, bystander), "other pid keeps its grant");
    assert!(w.iac.has_promotion_set("bystander-id"));
}

#[tokio::test]
async fn unload_deauthorizes_writes_and_withdraws_owned_promotion() {
    let w = world();
    let pid = load_authorized(&w, "unload-id", 1).await;
    let bystander = load_authorized(&w, "bystander-id", 2).await;

    w.scheduler.unload(pid).await.expect("unload");

    assert!(
        !can_write(&w.memory, pid),
        "unloaded pid loses write authority"
    );
    assert!(
        !w.iac.has_promotion_set("unload-id"),
        "owned promotion withdrawn"
    );
    assert!(can_write(&w.memory, bystander), "other pid keeps its grant");
    assert!(w.iac.has_promotion_set("bystander-id"));
    w.scheduler.unload(pid).await.expect("idempotent unload");
    assert!(
        can_write(&w.memory, bystander),
        "repeat unload spares others"
    );
}

/// A predecessor's stale release (same spirit id, newer owner pid) must not
/// remove the successor's registrations. The scheduler never dual-loads one
/// id, so this drives the pid-owned adapter methods the release paths call.
#[test]
fn stale_pid_release_spares_new_owner_registrations() {
    let w = world();
    let (old, new) = (101_u32, 102_u32);
    w.memory.authorize_principal_writes(old);
    w.memory.authorize_principal_writes(new);
    w.iac
        .register_promotion_set_for_pid("same-id", old, Some(AllowedPromotionSet::new()));
    // Successor takes over the id.
    w.iac
        .register_promotion_set_for_pid("same-id", new, Some(AllowedPromotionSet::new()));

    w.memory.deauthorize_principal_writes(old);
    w.iac.unregister_promotion_set_for_pid("same-id", old);

    assert!(!can_write(&w.memory, old), "stale pid loses its grant");
    assert!(can_write(&w.memory, new), "new owner grant untouched");
    assert!(
        w.iac.has_promotion_set("same-id"),
        "new owner promotion survives stale release"
    );

    w.memory.deauthorize_principal_writes(new);
    w.iac.unregister_promotion_set_for_pid("same-id", new);
    assert!(!can_write(&w.memory, new));
    assert!(!w.iac.has_promotion_set("same-id"));
}
