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
    cap: Arc<CapabilityRegistryAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
    tl: Arc<TransparencyLogAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    security: Arc<maos_kernel_core::security::SecurityManagerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    pids: Arc<std::sync::RwLock<std::collections::BTreeMap<String, u32>>>,
}
fn world(path: &std::path::Path) -> World {
    static CLOCK: std::sync::Once = std::sync::Once::new();
    CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x201b));
    let telemetry = Arc::new(IacRtMetrics::new());
    let (tx, mut rx) = cap_audit::channel();
    std::thread::spawn(move || while rx.blocking_recv().is_some() {});
    let policy = Arc::new(PolicyTable::new());
    let security = Arc::new(maos_kernel_core::security::SecurityManagerAdapter::new(
        policy.clone(),
    ));
    let cap = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x21; 32]),
        0x201b,
        policy,
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
        cap.clone(),
        scheduler.clone(),
        iac,
        halt,
        tl.clone(),
        journal.clone(),
        telemetry,
    ));
    World {
        cap,
        scheduler,
        applier,
        tl,
        memory,
        security,
        journal,
        pids: Arc::new(std::sync::RwLock::new(std::collections::BTreeMap::new())),
    }
}
impl World {
    async fn admit(&self, gated: &maos_bin::admission::GatedManifest) -> u32 {
        maos_bin::admission::load_admit_start(
            &maos_bin::admission::AdmissionDeps {
                scheduler: &self.scheduler,
                security: &self.security,
                transparency_log: &self.tl,
                journal: &self.journal,
                memory: &self.memory,
                pid_by_spirit_id: &self.pids,
                hooks: None,
                providers: None,
                iac: None,
            },
            gated,
            maos_kernel_core::scheduler::make_spirit_obj(Spirit),
            0x201b,
            maos_bin::admission::StartPolicy::LeaveLoaded,
            &mut |_| {},
        )
        .await
        .unwrap()
        .pid
    }
}
fn manifest() -> SpiritManifestBundle {
    manifest_named("running")
}
fn manifest_named(name: &str) -> SpiritManifestBundle {
    SpiritManifestBundle {
        class: Some(ClassSection {
            name: name.into(),
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
    std::fs::write(directory.join("artifact.bin"), &pkg.artifact_bytes).unwrap();
    std::fs::write(
        directory.join("origin.json"),
        serde_json::to_vec(&maos_registry::origin::RegistryOrigin::Remote {
            uri: "http://127.0.0.1:20101/mcp".into(),
        })
        .unwrap(),
    )
    .unwrap();
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
                world.cap.clone(),
                world.tl.clone(),
                world.scheduler.clone(),
                world.applier.clone(),
                policy,
                "http://127.0.0.1:20101/mcp".into(),
            );
            let mut entry = YankEntry::new("running".into(), "1.0.0".into(), 0, "withdrawn".into());
            entry.sequence = 1;
            entry.origin = Some(YankOrigin::Publisher);
            entry.authorization = maos_registry::yank_auth::YankAuthorization::sign(
                &trust::PUBLISHER,
                maos_registry::yank_auth::YankScope {
                    name: "running",
                    version: "1.0.0",
                    reason: "withdrawn",
                    registry_uri: "http://127.0.0.1:20101/mcp",
                },
                YankOrigin::Publisher,
                String::new(),
                10,
            )
            .unwrap();
            let mut forged = entry.clone();
            forged.reason.push('!');
            assert!(matches!(
                observer.on_yank(&forged).unwrap(),
                maos_registry::yank::YankDisposition::Rejected(_)
            ));
            assert!(entries(&world.tl).is_empty());
            observer.on_yank(&entry).unwrap();
            let rows = entries(&world.tl);
            let outcome = match policy {
                YankPolicy::Warn => "warned",
                YankPolicy::Quarantine => "quarantine-requested",
                YankPolicy::AutoRevoke => "auto-revoke-requested",
            };
            assert!(rows.iter().any(|r| r["outcome"] == outcome), "{rows:?}");
            assert_eq!(
                rows.iter()
                    .filter(|r| r["cause"] == "registry-yank"
                        && r["disposition"] == "refuse-at-next-load")
                    .count(),
                1,
                "remote withdrawal must retain its running-Spirit observation: {rows:?}"
            );
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
                assert_eq!(
                    world.cap.is_paused(pid),
                    policy == YankPolicy::Quarantine,
                    "only the quarantine policy fences the still-loaded Spirit"
                );
            }
            drop(observer);
            drop(world);
        }
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}
const REGISTRY: &str = "http://127.0.0.1:20101/mcp";
fn signed_yank(name: &str, version: &str, uri: &str, signer: &[u8; 32]) -> YankEntry {
    let mut entry = YankEntry::new(name.into(), version.into(), 0, "withdrawn".into());
    entry.sequence = 1;
    entry.origin = Some(YankOrigin::Publisher);
    entry.authorization = maos_registry::yank_auth::YankAuthorization::sign(
        signer,
        maos_registry::yank_auth::YankScope {
            name,
            version,
            reason: "withdrawn",
            registry_uri: uri,
        },
        YankOrigin::Publisher,
        String::new(),
        10,
    )
    .unwrap();
    entry
}
fn receipts(tl: &TransparencyLogAdapter, pid: u32) -> usize {
    tl.query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
        kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::CapabilityInvocation),
        spirit_pid: Some(pid),
        ..Default::default()
    })
    .unwrap()
    .into_iter()
    .filter(|frame| frame.intent == "spirit.quarantined")
    .count()
}
#[test]
fn quarantine_policy_fences_only_the_signed_yank_target_and_stays_sticky_ac8() {
    let _lock = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        install_fixture(tmp.path(), false);
        let world = world(&tmp.path().join("world"));
        let mut pids = Vec::new();
        for name in ["running", "other"] {
            let pid = world
                .scheduler
                .load(name, manifest_named(name), Spirit, 1)
                .await
                .unwrap();
            world.scheduler.start(pid).await.unwrap();
            pids.push(pid);
        }
        let (target, bystander) = (pids[0], pids[1]);
        let observer = maos_bin::yank_policy::YankRuntime::new(
            world.cap.clone(),
            world.tl.clone(),
            world.scheduler.clone(),
            world.applier.clone(),
            YankPolicy::Quarantine,
            REGISTRY.into(),
        );
        let fenced = |pid| world.cap.is_paused(pid);

        // A hostile registry cannot quarantine anything: a forged row, a row
        // signed by an attacker, a row for a loaded but uninstalled name, and a
        // row signed for another registry are all refused; a validly signed row
        // for a version the Spirit is not running is verified but not actionable.
        let mut forged = signed_yank("running", "1.0.0", REGISTRY, &trust::PUBLISHER);
        forged.reason.push('!');
        let attacker = signed_yank("running", "1.0.0", REGISTRY, &trust::VETTER);
        let uninstalled = signed_yank("other", "1.0.0", REGISTRY, &trust::VETTER);
        let rebound = signed_yank(
            "running",
            "1.0.0",
            "http://evil.invalid/mcp",
            &trust::PUBLISHER,
        );
        for hostile in [&forged, &attacker, &uninstalled, &rebound] {
            assert!(
                matches!(
                    observer.on_yank(hostile).unwrap(),
                    maos_registry::yank::YankDisposition::Rejected(_)
                ),
                "{hostile:?}"
            );
        }
        let other_version = signed_yank("running", "2.0.0", REGISTRY, &trust::PUBLISHER);
        assert!(matches!(
            observer.on_yank(&other_version).unwrap(),
            maos_registry::yank::YankDisposition::Verified
        ));
        assert!(!fenced(target) && !fenced(bystander));
        assert_eq!(
            receipts(&world.tl, target) + receipts(&world.tl, bystander),
            0
        );

        // The authentic publisher yank fences exactly the yanked running Spirit.
        let entry = signed_yank("running", "1.0.0", REGISTRY, &trust::PUBLISHER);
        observer.on_yank(&entry).unwrap();
        observer.on_yank(&entry).unwrap();
        assert!(
            world.cap.is_quarantined(target),
            "the yanked Spirit is quarantined"
        );
        assert!(!fenced(bystander), "an unrelated Spirit is untouched");
        assert_ne!(
            world.cap.issue_with_mediation(
                bystander,
                maos_domain::invariants::i1::Scope::FsRead {
                    subtree: "/tmp".into()
                },
                60,
                [0; 32],
                maos_domain::invariants::i1::IntentClass::Standard
            ),
            Err(maos_domain::ports::capability::CapError::Revoked)
        );
        assert_eq!(
            world.cap.issue_with_mediation(
                target,
                maos_domain::invariants::i1::Scope::FsRead {
                    subtree: "/tmp".into()
                },
                60,
                [0; 32],
                maos_domain::invariants::i1::IntentClass::Standard
            ),
            Err(maos_domain::ports::capability::CapError::Revoked),
            "the next mediated call of the quarantined Spirit is refused"
        );
        assert_eq!(
            receipts(&world.tl, target),
            1,
            "one typed receipt; the replay is deduplicated"
        );
        assert_eq!(receipts(&world.tl, bystander), 0);
        let scbs = world.scheduler.scbs();
        assert_eq!(
            scbs.read().unwrap()[&target].current_state(),
            ScbLifecycleState::Running,
            "quarantine isolates and never kills"
        );

        // A director pause -> resume cannot lift it; unload ends it.
        world.scheduler.pause(target).await.unwrap();
        world.scheduler.resume(target).await.unwrap();
        assert!(
            world.cap.is_quarantined(target),
            "resume leaves the quarantine"
        );
        world.scheduler.unload(target).await.unwrap();
        assert!(!fenced(target), "unload clears the fence");
        assert!(!fenced(bystander));
        drop(observer);
        drop(world);
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
                world.cap.clone(),
                world.tl.clone(),
                world.scheduler.clone(),
                world.applier.clone(),
                YankPolicy::Quarantine,
                "http://127.0.0.1:20101/mcp".into(),
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
    // Two actual processes restart the global PID allocator. A control file in
    // the child's private cwd avoids introducing a production environment knob.
    let cwd = std::env::current_dir().unwrap();
    let phase = match std::fs::read_to_string(cwd.join("spill-boot.phase")) {
        Ok(phase) => phase,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let tmp = tempfile::tempdir().unwrap();
            for phase in ["first", "second"] {
                std::fs::write(tmp.path().join("spill-boot.phase"), phase).unwrap();
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .current_dir(tmp.path())
                    .args([
                        "--exact",
                        "registry_publishers_cannot_share_principal_spill_ac10",
                        "--nocapture",
                    ])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{phase} boot: {}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        }
        Err(error) => panic!("read spill child control: {error}"),
    };
    assert!(phase == "first" || phase == "second");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        std::env::set_var("HOME", &cwd);
        let text = maos_spirit_hello::MANIFEST_TOML;
        let manifest_path = cwd.join("manifest.toml");
        std::fs::write(&manifest_path, text).unwrap();
        let mut gated = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
        let bare = gated.spirit_id.clone();
        let signer = if phase == "first" {
            &trust::PUBLISHER
        } else {
            &trust::VETTER
        };
        // Hostile imported-sidecar replacement deliberately bypasses the supply
        // pin so the second, independent principal-spill boundary is exercised.
        let mut imported = trust::package(&bare, &gated.class_section.version, signer);
        imported.manifest_toml = text.as_bytes().to_vec();
        trust::sign(&mut imported, signer);
        std::fs::write(cwd.join("artifact.bin"), &imported.artifact_bytes).unwrap();
        std::fs::write(
            cwd.join("signed_package.json"),
            serde_json::to_vec(&imported).unwrap(),
        )
        .unwrap();
        gated.bind_registry_package_at(&manifest_path).unwrap();
        let ns = MemoryNamespace::principal("director-test", "registry/v1").unwrap();
        let world = world(&cwd.join("world"));
        let pid = world.admit(&gated).await;
        if phase == "first" {
            world
                .memory
                .write(
                    pid,
                    MemoryTier::Private,
                    &ns,
                    "secret",
                    MemoryValue::Markdown("publisher one".into()),
                )
                .unwrap();
            std::fs::write(cwd.join("spill-boot.pid"), pid.to_string()).unwrap();
            std::fs::write(cwd.join("spill-boot.identity"), &gated.storage_identity).unwrap();
        } else {
            let previous_pid: u32 = std::fs::read_to_string(cwd.join("spill-boot.pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                pid, previous_pid,
                "the second process must reuse the scheduler PID"
            );
            assert_eq!(
                world
                    .memory
                    .read(pid, MemoryTier::Private, &ns, "secret")
                    .unwrap(),
                None
            );
            assert_ne!(
                gated.storage_identity,
                std::fs::read_to_string(cwd.join("spill-boot.identity")).unwrap()
            );
            let local = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
            assert_eq!(local.storage_identity, bare);
            let mut long = local;
            long.spirit_id = "x".repeat(128);
            long.bind_registry_publisher(&trust::public(&trust::PUBLISHER));
            assert!(3 + long.storage_identity.len() * 2 <= 255);
        }
    });
}

#[test]
fn deployed_artifact_cannot_hide_behind_genuine_signed_sidecar_p9() {
    let _env = ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let text = maos_spirit_hello::MANIFEST_TOML;
    let manifest = tmp.path().join("manifest.toml");
    std::fs::write(&manifest, text).unwrap();
    let mut gated = maos_bin::admission::gate_manifest(text, &|_| true).unwrap();
    let mut pkg = trust::package(
        &gated.spirit_id,
        &gated.class_section.version,
        &trust::PUBLISHER,
    );
    pkg.manifest_toml = text.as_bytes().to_vec();
    trust::sign(&mut pkg, &trust::PUBLISHER);
    std::fs::write(
        tmp.path().join("signed_package.json"),
        serde_json::to_vec(&pkg).unwrap(),
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("artifact.bin"),
        b"modified deployed component",
    )
    .unwrap();
    assert!(
        gated.bind_registry_package_at(&manifest).is_err(),
        "signed metadata must not vouch for modified deployed bytes"
    );
}

#[test]
fn failing_auto_revoke_retries_policy_without_repeating_observation_p12() {
    let _env = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        install_fixture(tmp.path(), true);
        let world = world(&tmp.path().join("world"));
        let pid = world
            .scheduler
            .load("running", manifest(), Spirit, 1)
            .await
            .unwrap();
        world.scheduler.start(pid).await.unwrap();
        let scb = world.scheduler.scbs().read().unwrap()[&pid].clone();
        let fault = std::thread::spawn(move || {
            let _guard = scb.task_assignments_in_flight.lock().unwrap();
            panic!("test-only poisoned assignment state rejects CRL application");
        })
        .join();
        assert!(fault.is_err());
        let observer = maos_bin::yank_policy::YankRuntime::new(
            world.cap.clone(),
            world.tl.clone(),
            world.scheduler.clone(),
            world.applier.clone(),
            YankPolicy::AutoRevoke,
            "http://127.0.0.1:20101/mcp".into(),
        );
        for now in [1000, 1001, 1002] {
            assert!(observer.reevaluate_at(now).is_err());
        }
        let rows = entries(&world.tl);
        assert_eq!(
            rows.iter()
                .filter(|row| row["cause"] == "expiry-lapse"
                    && row["disposition"] == "refuse-at-next-load")
                .count(),
            1,
            "failed policy application must not duplicate its observation: {rows:?}"
        );
        assert!(world.scheduler.scbs().read().unwrap().contains_key(&pid));
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}

struct FixtureYanks(Vec<YankEntry>);
impl maos_registry::yank::YankSource for FixtureYanks {
    fn fetch_yanks(
        &self,
        _: u64,
    ) -> Result<maos_domain::ports::registry::YankList, maos_domain::ports::registry::RegistryError>
    {
        Ok(maos_domain::ports::registry::YankList::new(self.0.clone()))
    }
}
fn signed_entry(sequence: u64, origin: YankOrigin) -> YankEntry {
    let mut entry = YankEntry::new("running".into(), "1.0.0".into(), 0, "withdrawn".into());
    entry.sequence = sequence;
    entry.origin = Some(origin);
    let seed = if origin == YankOrigin::Vetter {
        &trust::VETTER
    } else {
        &trust::PUBLISHER
    };
    entry.authorization = maos_registry::yank_auth::YankAuthorization::sign(
        seed,
        maos_registry::yank_auth::YankScope {
            name: "running",
            version: "1.0.0",
            reason: "withdrawn",
            registry_uri: "http://127.0.0.1:20101/mcp",
        },
        origin,
        "vetter".into(),
        10,
    )
    .unwrap();
    entry
}

#[test]
fn rejected_vetter_row_does_not_hide_later_publisher_yank_p1() {
    let _env = ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let directory = install_fixture(tmp.path(), false);
    // A publisher's independent authority must survive corrupt vetter metadata.
    std::fs::write(
        directory.join("vetting.json"),
        b"invalid promotion metadata",
    )
    .unwrap();
    let world = world(&tmp.path().join("world"));
    let observer = Arc::new(maos_bin::yank_policy::YankRuntime::new(
        world.cap.clone(),
        world.tl.clone(),
        world.scheduler.clone(),
        world.applier.clone(),
        YankPolicy::Warn,
        "http://127.0.0.1:20101/mcp".into(),
    ));
    let poller = maos_registry::yank::YankPoller::new(
        Arc::new(FixtureYanks(vec![
            signed_entry(1, YankOrigin::Vetter),
            signed_entry(2, YankOrigin::Publisher),
        ])),
        observer,
        "http://127.0.0.1:20101/mcp".into(),
    );
    assert_eq!(poller.poll_once().unwrap(), 1);
    let rows = entries(&world.tl);
    assert_eq!(
        rows.iter()
            .filter(|row| row["outcome"] == "warned" && row["origin"] == "publisher")
            .count(),
        1
    );
    assert_eq!(poller.last_seen_ns(), 2);
    std::env::remove_var("MAOS_AUDIT_KEY");
}

/// 20-1b security F-2 — a registry parks one invalid row ahead of the verified cursor.
/// Every poll redelivers it; it is journaled once per (sequence, reason), never per poll,
/// while the verified cursor stays untouched.
#[test]
fn a_parked_invalid_row_is_journaled_once_across_polls_f2() {
    let _env = ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", tmp.path());
    let world = world(&tmp.path().join("world"));
    let observer = Arc::new(maos_bin::yank_policy::YankRuntime::new(
        world.cap.clone(),
        world.tl.clone(),
        world.scheduler.clone(),
        world.applier.clone(),
        YankPolicy::Warn,
        "http://127.0.0.1:20101/mcp".into(),
    ));
    let mut parked = YankEntry::new(
        "not-installed".into(),
        "1.0.0".into(),
        0,
        "untrusted".into(),
    );
    parked.sequence = 1;
    let mut second = YankEntry::new("other-name".into(), "1.0.0".into(), 0, "untrusted".into());
    second.sequence = 2;
    let poller = maos_registry::yank::YankPoller::new(
        Arc::new(FixtureYanks(vec![parked.clone(), second])),
        observer.clone(),
        "http://127.0.0.1:20101/mcp".into(),
    );
    for _ in 0..6 {
        assert_eq!(poller.poll_once().unwrap(), 0);
    }
    assert_eq!(poller.last_seen_ns(), 0, "the verified cursor never moves");
    let rows = entries(&world.tl);
    let count = |sequence: u64| {
        rows.iter()
            .filter(|row| row["sequence"] == sequence && row.get("reason").is_some())
            .count()
    };
    assert_eq!(
        count(1),
        1,
        "parked row journaled once across 6 polls: {rows:?}"
    );
    assert_eq!(
        count(2),
        1,
        "a second parked row is journaled once too: {rows:?}"
    );
    // A changed reason for the same sequence is new information and is journaled.
    maos_registry::yank::YankObserver::on_rejected(&*observer, &parked, "a different reason")
        .unwrap();
    assert_eq!(count(1), 1, "unchanged rows stay deduplicated");
    assert_eq!(
        entries(&world.tl)
            .iter()
            .filter(|r| r["sequence"] == 1)
            .count(),
        2
    );
    std::env::remove_var("MAOS_AUDIT_KEY");
}
#[test]
fn uninstalled_unsigned_row_never_advances_authenticated_cursor_f1() {
    let _env = ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", tmp.path());
    let world = world(&tmp.path().join("world"));
    let observer = Arc::new(maos_bin::yank_policy::YankRuntime::new(
        world.cap.clone(),
        world.tl.clone(),
        world.scheduler.clone(),
        world.applier.clone(),
        YankPolicy::Warn,
        "http://127.0.0.1:20101/mcp".into(),
    ));
    let mut entry = YankEntry::new(
        "not-installed".into(),
        "1.0.0".into(),
        0,
        "untrusted".into(),
    );
    entry.sequence = 1;
    let poller = maos_registry::yank::YankPoller::new(
        Arc::new(FixtureYanks(vec![entry])),
        observer,
        "http://127.0.0.1:20101/mcp".into(),
    );
    assert_eq!(
        poller.poll_once().unwrap(),
        0,
        "unknown authority must be rejected, not acknowledged"
    );
    assert_eq!(poller.last_seen_ns(), 0);
}

#[test]
fn corrupt_install_does_not_hide_another_spirits_expiry_p1() {
    let _env = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        install_fixture(tmp.path(), true);
        let root = tmp.path().join(".local/share/maos/registry");
        std::fs::create_dir_all(root.join("installed/corrupt/1.0.0-test")).unwrap();
        std::fs::write(
            root.join("installed/corrupt/1.0.0-test/signed_package.json"),
            b"not-json",
        )
        .unwrap();
        std::fs::write(
            root.join("installed-index.json"),
            br#"{"corrupt":"corrupt/1.0.0-test","running":"running/1.0.0-test"}"#,
        )
        .unwrap();
        let world = world(&tmp.path().join("world"));
        let mut bad = manifest();
        bad.class.as_mut().unwrap().name = "corrupt".into();
        let bad_pid = world
            .scheduler
            .load("corrupt", bad, Spirit, 1)
            .await
            .unwrap();
        world.scheduler.start(bad_pid).await.unwrap();
        let pid = world
            .scheduler
            .load("running", manifest(), Spirit, 1)
            .await
            .unwrap();
        world.scheduler.start(pid).await.unwrap();
        let observer = maos_bin::yank_policy::YankRuntime::new(
            world.cap.clone(),
            world.tl.clone(),
            world.scheduler.clone(),
            world.applier.clone(),
            YankPolicy::Warn,
            "http://127.0.0.1:20101/mcp".into(),
        );
        let _result = observer.reevaluate_at(1000);
        assert!(
            entries(&world.tl)
                .iter()
                .any(|row| row["cause"] == "expiry-lapse"),
            "one corrupt install must not abort the per-Spirit scan"
        );
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}

#[test]
fn poller_runtime_clock_and_cursor_restart_reach_expiry_p6() {
    let _env = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        install_fixture(tmp.path(), true);
        std::env::set_var("MAOS_REGISTRY_YANK_CURSOR_PATH", tmp.path().join("cursor.json"));
        let world = world(&tmp.path().join("world"));
        let pid = world.scheduler.load("running", manifest(), Spirit, 1).await.unwrap();
        world.scheduler.start(pid).await.unwrap();
        let source = Arc::new(FixtureYanks(vec![signed_entry(1, YankOrigin::Publisher)]));
        for boot in 1..=2 {
            let observer = Arc::new(maos_bin::yank_policy::YankRuntime::new(
                world.cap.clone(),
                world.tl.clone(), world.scheduler.clone(), world.applier.clone(),
                YankPolicy::Warn, "http://127.0.0.1:20101/mcp".into()).with_clock(|| Ok(1000)));
            let poller = maos_registry::yank::YankPoller::new(source.clone(), observer,
                "http://127.0.0.1:20101/mcp".into());
            poller.load_cursor().unwrap();
            assert_eq!(poller.poll_once().unwrap(), if boot == 1 { 1 } else { 0 });
            let rows = entries(&world.tl);
            assert_eq!(rows.iter().filter(|row| row["cause"] == "expiry-lapse" && row["disposition"] == "refuse-at-next-load").count(), boot,
                "each reconstructed runtime must perform local expiry independently of the remote cursor");
            assert_eq!(rows.iter().filter(|row| row["authorization"].is_array()).count(), 1,
                "the persisted cursor must not replay remote propagation");
        }
    });
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    std::env::remove_var("MAOS_AUDIT_KEY");
}

#[test]
fn no_registry_local_timer_discovers_promotion_installed_after_start_p11() {
    let _env = ENV.lock().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap();
    runtime.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());
        let world = world(&tmp.path().join("world"));
        let pid = world
            .scheduler
            .load("running", manifest(), Spirit, 1)
            .await
            .unwrap();
        world.scheduler.start(pid).await.unwrap();
        let observer = Arc::new(
            maos_bin::yank_policy::YankRuntime::new(
                world.cap.clone(),
                world.tl.clone(),
                world.scheduler.clone(),
                world.applier.clone(),
                YankPolicy::Warn,
                String::new(),
            )
            .with_clock(|| Ok(1000)),
        );
        let mut task = maos_bin::yank_policy::PollTask::start_local(
            observer,
            std::time::Duration::from_secs(30),
        );
        tokio::task::yield_now().await;
        assert!(
            entries(&world.tl).is_empty(),
            "the first timer tick must not block boot"
        );
        install_fixture(tmp.path(), true);
        tokio::time::advance(std::time::Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
        assert_eq!(
            entries(&world.tl)
                .iter()
                .filter(|row| row["cause"] == "expiry-lapse"
                    && row["disposition"] == "refuse-at-next-load")
                .count(),
            1
        );
        task.shutdown().await;
    });
    std::env::remove_var("MAOS_AUDIT_KEY");
}
