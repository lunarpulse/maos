#![cfg(all(feature = "network", unix))]
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::{YankEntry, YankOrigin};
use maos_domain::revocation::RevocationAction;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::scheduler::{
    control_block::{ScbLifecycleState, SpiritManifestBundle},
    SpiritSchedulerAdapter,
};
use maos_kernel_core::security::{
    manifest::{ClassSection, OnRevocationSection},
    RingCryptoProvider,
};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use maos_registry::yank::{YankObserver, YankPolicy};
use std::sync::{Arc, Mutex};
static ENV: Mutex<()> = Mutex::new(());
struct Spirit;
impl maos_spirit_abi::lifecycle::Spirit for Spirit {}
struct World {
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
    tl: Arc<TransparencyLogAdapter>,
    memory: Arc<MemoryManagerAdapter>,
}
fn world(path: &std::path::Path) -> World {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x201b));
    let telemetry = Arc::new(IacRtMetrics::new());
    let (tx, mut rx) = cap_audit::channel();
    std::thread::spawn(move || while rx.blocking_recv().is_some() {});
    let cap = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x21; 32]),
        0x201b,
        Arc::new(PolicyTable::new()),
        tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    std::fs::create_dir_all(path).unwrap();
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(path.join("private"), 4)),
        Arc::new(SharedMemoryStore::open(&path.join("memory.db")).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&path.join("memory.db")).unwrap()),
        tl.clone(),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(telemetry.clone())),
        tl.clone(),
    ));
    let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        tl.clone(),
        cap.clone(),
        memory.clone(),
        iac.clone(),
        halt.clone(),
        telemetry.clone(),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&path.join("journal.ndjson")).unwrap(),
    );
    let applier = Arc::new(maos_kernel_core::revocation::RevocationApplier::new(
        scheduler.scbs(),
        cap,
        scheduler.clone(),
        iac,
        halt,
        tl.clone(),
        journal,
        telemetry,
    ));
    World {
        scheduler,
        applier,
        tl,
        memory,
    }
}
fn manifest() -> SpiritManifestBundle {
    SpiritManifestBundle {
        class: Some(ClassSection {
            name: "running".into(),
            version: "1.0.0".into(),
            abi: "1.0".into(),
            manifest_schema_version: 1,
            min_substrate_version: "0.1.0".into(),
            forms: vec!["rust-inproc".into()],
            trust_tier: "local".into(),
            description: "test-only running actor".into(),
            artifact: None,
        }),
        on_revocation: Some(OnRevocationSection {
            action: RevocationAction::TerminateImmediately,
        }),
        ..Default::default()
    }
}
fn install_fixture(home: &std::path::Path, promotion: bool) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::env::set_var("HOME", home);
    let root = home.join(".local/share/maos/registry");
    let directory = root.join("installed/running/1.0.0-test");
    std::fs::create_dir_all(&directory).unwrap();
    let pkg = trust::package("running", "1.0.0", &trust::PUBLISHER);
    std::fs::write(
        directory.join("signed_package.json"),
        serde_json::to_vec(&pkg).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("manifest.toml"), &pkg.manifest_toml).unwrap();
    std::fs::write(
        root.join("installed-index.json"),
        b"{\"running\":\"running/1.0.0-test\"}",
    )
    .unwrap();
    let key = home.join("operator.key");
    std::fs::write(&key, hex::encode(trust::OPERATOR)).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::env::set_var("MAOS_AUDIT_KEY", key);
    if promotion {
        let keyring = home.join("vetters.cbor");
        std::fs::write(&keyring, serde_cbor::to_vec(&trust::keyring()).unwrap()).unwrap();
        let metadata = maos_registry::install::InstalledVetting {
            attestation: trust::attestation(&pkg, 10, 1000),
            keyring_path: keyring,
        };
        std::fs::write(
            directory.join("vetting.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
    }
    directory
}
fn entries(tl: &TransparencyLogAdapter) -> Vec<serde_json::Value> {
    tl.query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
        kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::SpiritRevoked),
        ..Default::default()
    })
    .unwrap()
    .into_iter()
    .map(|e| serde_json::from_slice(&e.payload_redacted).unwrap())
    .collect()
}
#[test]
fn signed_yank_decisions_warn_request_quarantine_and_apply_crl_ac8() {
    let _lock = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for policy in [
            YankPolicy::Warn,
            YankPolicy::Quarantine,
            YankPolicy::AutoRevoke,
        ] {
            let tmp = tempfile::tempdir().unwrap();
            install_fixture(tmp.path(), false);
            let world = world(&tmp.path().join("world"));
            let pid = world
                .scheduler
                .load("running", manifest(), Spirit, 1)
                .await
                .unwrap();
            world.scheduler.start(pid).await.unwrap();
            let observer = maos_bin::yank_policy::YankRuntime::new(
                world.tl.clone(),
                world.scheduler.clone(),
                world.applier.clone(),
                policy,
            );
            let mut entry = YankEntry::new("running".into(), "1.0.0".into(), 0, "withdrawn".into());
            entry.sequence = 1;
            entry.origin = Some(YankOrigin::Publisher);
            entry.authorization = maos_registry::yank_auth::YankAuthorization::sign(
                &trust::PUBLISHER,
                "running",
                "1.0.0",
                "withdrawn",
                YankOrigin::Publisher,
                String::new(),
                10,
            )
            .unwrap();
            let mut forged = entry.clone();
            forged.reason.push('!');
            assert!(observer.on_yank(&forged).is_err());
            assert!(entries(&world.tl).is_empty());
            observer.on_yank(&entry).unwrap();
            let rows = entries(&world.tl);
            let outcome = match policy {
                YankPolicy::Warn => "warned",
                YankPolicy::Quarantine => "quarantine-requested",
                YankPolicy::AutoRevoke => "auto-revoke-requested",
            };
            assert!(rows.iter().any(|r| r["outcome"] == outcome), "{rows:?}");
            if policy == YankPolicy::AutoRevoke {
                assert!(!world.scheduler.scbs().read().unwrap().contains_key(&pid));
                assert!(
                    rows.iter()
                        .any(|r| r["revocation_origin"] == "registry_yank"
                            && r["action"] == "terminate-immediately"),
                    "{rows:?}"
                );
            } else {
                assert_eq!(
                    world.scheduler.scbs().read().unwrap()[&pid].current_state(),
                    ScbLifecycleState::Running
                );
            }
            drop(observer);
            drop(world);
        }
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}
#[test]
fn expiry_and_vetter_revocation_observe_once_and_request_policy_ac9() {
    let _lock = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for revoked in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            install_fixture(tmp.path(), true);
            let world = world(&tmp.path().join("world"));
            let pid = world
                .scheduler
                .load("running", manifest(), Spirit, 1)
                .await
                .unwrap();
            world.scheduler.start(pid).await.unwrap();
            let observer = maos_bin::yank_policy::YankRuntime::new(
                world.tl.clone(),
                world.scheduler.clone(),
                world.applier.clone(),
                YankPolicy::Quarantine,
            );
            observer.reevaluate_at(500).unwrap();
            assert!(entries(&world.tl).is_empty());
            if revoked {
                let mut keyring = trust::keyring();
                keyring.push(maos_compliance::vetting::keyring::issue_event(
                    &trust::OPERATOR,
                    &maos_compliance::VetterKeyEventClaim {
                        kind: maos_compliance::VetterKeyEventKind::Revoke,
                        vetter_key_id: "vetter".into(),
                        vetter_pubkey: trust::public(&trust::VETTER),
                        predecessor_pubkey: None,
                        effective_at_unix_ms: 600,
                        journal_sequence: 2,
                        journaled_at_unix_ms: 600,
                        note: "test revocation".into(),
                    },
                ));
                std::fs::write(
                    tmp.path().join("vetters.cbor"),
                    serde_cbor::to_vec(&keyring).unwrap(),
                )
                .unwrap();
            }
            let now = if revoked { 700 } else { 1000 };
            observer.reevaluate_at(now).unwrap();
            observer.reevaluate_at(now + 1).unwrap();
            let rows = entries(&world.tl);
            let cause = if revoked {
                "vetting-revocation"
            } else {
                "expiry-lapse"
            };
            assert_eq!(
                rows.iter()
                    .filter(|r| r["cause"] == cause && r["disposition"] == "refuse-at-next-load")
                    .count(),
                1,
                "{rows:?}"
            );
            assert_eq!(
                rows.iter()
                    .filter(|r| r["outcome"] == "quarantine-requested")
                    .count(),
                1,
                "{rows:?}"
            );
            assert_eq!(
                world.scheduler.scbs().read().unwrap()[&pid].current_state(),
                ScbLifecycleState::Running
            );
            drop(observer);
            drop(world);
        }
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}
#[test]
fn registry_publishers_cannot_share_principal_spill_ac10() {
    use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
    use maos_domain::ports::MemoryManagerPort;
    let _lock = ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let text = maos_spirit_hello::MANIFEST_TOML;
    let manifest_path = tmp.path().join("manifest.toml");
    std::fs::write(&manifest_path, text).unwrap();
    let mut first = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
    let bare = first.spirit_id.clone();
    let mut imported = trust::package(&bare, &first.class_section.version, &trust::PUBLISHER);
    imported.manifest_toml = text.as_bytes().to_vec();
    trust::sign(&mut imported, &trust::PUBLISHER);
    std::fs::write(
        tmp.path().join("signed_package.json"),
        serde_json::to_vec(&imported).unwrap(),
    )
    .unwrap();
    first.bind_registry_package_at(&manifest_path).unwrap();
    let ns = MemoryNamespace::principal("director-test", "registry/v1").unwrap();
    {
        let world = world(tmp.path());
        world
            .memory
            .bind_storage_identity(1, &first.storage_identity)
            .unwrap();
        world
            .memory
            .write(
                1,
                MemoryTier::Private,
                &ns,
                "secret",
                MemoryValue::Markdown("publisher one".into()),
            )
            .unwrap();
    }
    // Force a differently signed imported sidecar past the supply-side pin.
    trust::sign(&mut imported, &trust::VETTER);
    std::fs::write(
        tmp.path().join("signed_package.json"),
        serde_json::to_vec(&imported).unwrap(),
    )
    .unwrap();
    let mut second = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
    second.bind_registry_package_at(&manifest_path).unwrap();
    let world = world(tmp.path());
    world
        .memory
        .bind_storage_identity(1, &second.storage_identity)
        .unwrap();
    assert_eq!(
        world
            .memory
            .read(1, MemoryTier::Private, &ns, "secret")
            .unwrap(),
        None
    );
    assert_ne!(first.storage_identity, second.storage_identity);
    let local = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
    assert_eq!(local.storage_identity, bare);
    let mut long = local;
    long.spirit_id = "x".repeat(128);
    long.bind_registry_publisher(&trust::public(&trust::PUBLISHER));
    assert!(3 + long.storage_identity.len() * 2 <= 255);
}
