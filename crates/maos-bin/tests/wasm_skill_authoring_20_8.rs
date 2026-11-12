#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#[path = "wasm_skill_authoring_20_8/signed.rs"]
mod signed;

#[test]
fn default_build_checks_the_versioned_skill_import() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../wit/spirit.wit");
    let source = std::fs::read_to_string(path).unwrap();
    let mut resolve = wit_parser::Resolve::new();
    let package = resolve.push_source(path, &source).unwrap();
    assert_eq!(
        resolve.packages[package]
            .name
            .version
            .as_ref()
            .unwrap()
            .to_string(),
        "2.4.0"
    );
    let world = resolve
        .worlds
        .iter()
        .find(|(_, world)| world.name == "spirit")
        .unwrap()
        .1;
    let skills = world
        .imports
        .values()
        .find_map(|item| match item {
            wit_parser::WorldItem::Interface { id, .. }
                if resolve.interfaces[*id].name.as_deref() == Some("skills") =>
            {
                Some(&resolve.interfaces[*id])
            }
            _ => None,
        })
        .expect("spirit world must import skills");
    assert_eq!(
        skills
            .functions
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["author", "propose"]
    );
}
#[cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

#[cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
mod live {

    use super::fixtures;
    use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
    use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionLaunch};
    use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
    use maos_domain::invariants::{i1::IntentClass, i3::FrameOrigin};
    use maos_iac::adapter::FrameFilter;
    use maos_kernel_core::{
        capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter},
        iac::{IacBusAdapter, TransparencyLogAdapter},
        memory::MemoryManagerAdapter,
        scheduler::SpiritSchedulerAdapter,
        security::SecurityManagerAdapter,
    };
    use maos_skill::{LocalFsSkillQueueStore, SkillAdmissionState, SkillQueueStore};
    use maos_spirit_abi::identity::{FrameKind, SpiritId};
    use std::{
        collections::BTreeMap,
        path::Path,
        sync::{Arc, RwLock},
        time::Duration,
    };

    const ID: &str = "recall-probe";
    const NONCE: u64 = 0x20_08;

    fn manifest(granted: bool) -> String {
        format!(
            r#"[class]
name = "recall-probe"
description = "WASM skill witness"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "recall_probe.wasm"
trust_tier = "local"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["introduction"]
[budget]
context_window_size = 4096
time_cap_seconds = 30
[resources]
fd_max = 64
[sandbox]
tier = "T2"
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.skill]
author_self = {granted}
"#
        )
    }

    fn seed(script: String) -> IacFrame {
        let address = |id| FrameAddress {
            spirit_id: SpiritId::from(id),
            host_id: None,
            role: None,
        };
        IacFrame {
            frame_id: ulid::Ulid::new().to_bytes(),
            timestamp_ns: 1,
            logical_clock: 1,
            from: address("operator"),
            to: smallvec::smallvec![address(ID)],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            auto_marker: FrameOrigin::HumanAuthored,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: format!("to collector\n{script}"),
                scope: vec![],
                success_criteria: "report".into(),
                posture_preferences: Default::default(),
                prior_distillate_ref: None,
            }),
            consent_envelope: None,
            intent_lineage: Default::default(),
        }
    }

    async fn run(
        dir: &Path,
        granted: bool,
        script: String,
    ) -> (
        Vec<maos_skill::QueueEntry>,
        Vec<maos_iac::adapter::TransparencyLogEntry>,
    ) {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(TransparencyLogAdapter::open(&dir.join("tl.sqlite"), NONCE).unwrap());
        maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&dir.join("tl.sqlite"))
            .unwrap();
        let lookup = Arc::new(RwLock::new(BTreeMap::new()));
        tl.install_recipient_resolver(Arc::clone(&lookup) as _)
            .unwrap();
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(
            Arc::clone(&policy),
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
        ));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([8; 32]),
            NONCE,
            policy,
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ));
        let db = dir.join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db).unwrap(),
            ),
            Arc::clone(&tl),
        ));
        let iac = Arc::new(IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
            Arc::clone(&tl),
        ));
        let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            Arc::clone(&memory),
            Arc::clone(&iac),
            Arc::clone(&halt),
            Arc::clone(&metrics),
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson")).unwrap(),
        );
        let crash = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&tl),
            Arc::clone(&halt),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&metrics),
            Arc::clone(&journal),
        ));
        Arc::get_mut(&mut scheduler)
            .unwrap()
            .set_crash_detector(Arc::clone(&crash));
        let root = maos_bin::supervision::RootSupervision::arm(
            Arc::clone(&scheduler),
            crash,
            Arc::clone(&tl),
            Arc::clone(&journal),
            Arc::clone(&halt),
            Arc::clone(&iac),
            Arc::clone(&metrics),
            Arc::new(maos_director_surface::notification::NotificationDispatcher::default()),
            NONCE,
        );
        let supervisor = Arc::clone(root.supervisor());
        let writer = cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl));
        let store = Arc::new(LocalFsSkillQueueStore::at_path(dir.join("queue.json")));
        let telemetry = Arc::new(
            maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator::new(
                metrics,
                Arc::clone(&halt),
                Arc::clone(&tl),
            ),
        );
        let binder = maos_bin::skill_authoring::binder(
            &capability,
            &security,
            &tl,
            &telemetry,
            &scheduler.scbs(),
            &store,
            None,
            None,
        );
        let gated = gate_manifest(&manifest(granted), &|_| false).unwrap();
        let scopes = maos_manifest::capabilities_required_to_scopes(&gated.caps_required);
        let (core, obj) = supervisor.prepare_spawned(ID, &gated.bundle()).unwrap();
        let _collector = iac
            .register_spirit_typed(&SpiritId::from("collector"))
            .unwrap();
        let admitted = load_admit_start(
            &AdmissionDeps {
                scheduler: &scheduler,
                security: &security,
                transparency_log: &tl,
                journal: &journal,
                memory: &memory,
                pid_by_spirit_id: &lookup,
                hooks: None,
                providers: None,
                iac: Some(&iac),
            },
            &gated,
            obj,
            NONCE,
            StartPolicy::StartNow,
            &mut |pid| core.bind_scheduler_pid(pid),
        )
        .await
        .unwrap();
        let runner = fixtures::fixture("maos-wasm-runner");
        let component = fixtures::fixture("recall_probe.wasm");
        let launch = SessionLaunch {
            plan: maos_host::SpiritLaunchPlan {
                program: runner.to_string_lossy().into_owned(),
                argv: vec![
                    "--component".into(),
                    component.to_string_lossy().into_owned(),
                    "--fuel".into(),
                    "100000000".into(),
                ],
                env: vec![],
                wire: maos_host::WireShape::ContentLengthCbor,
            },
            component_artifact: Some(component),
            turn_budget: Duration::from_secs(30),
            context_window: Default::default(),
            once: true,
            initial_frame: Some(seed(script)),
            scopes,
            origin: std::path::PathBuf::new(),
        };
        let deps = SessionDeps {
            supervisor: Arc::clone(&supervisor),
            capability: Arc::clone(&capability),
            security: Arc::clone(&security),
            iac: Arc::clone(&iac),
            journal: Arc::clone(&tl),
            log_recall: Arc::new(maos_iac::adapter::log_recall::LogRecallAdapter::new(
                Arc::clone(&tl),
            )),
            lifecycle_journal: Arc::clone(&journal),
            pid_by_spirit_id: Arc::clone(&lookup),
            enterprise: None,
            pdp: None,
            skill: Some(binder),
        };
        run_admitted_session(deps, admitted, core, launch)
            .await
            .expect("real T2 skill session");
        let entries = store.load().unwrap();
        let rows = tl.query_frames(FrameFilter::default()).unwrap();
        root.stop_and_join().await;
        drop((
            supervisor, scheduler, capability, security, iac, memory, journal, halt, telemetry,
            store, lookup, _collector,
        ));
        tokio::time::timeout(Duration::from_secs(5), writer)
            .await
            .unwrap()
            .unwrap();
        (entries, rows)
    }
    fn guest_report(rows: &[maos_iac::adapter::transparency_log::TransparencyLogEntry]) -> String {
        let row = rows
            .iter()
            .find(|row| row.intent == "spirit.frame.authorization")
            .expect("guest report authorization");
        let value: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
        let frame: IacFrame = serde_json::from_value(value["claims"].clone()).unwrap();
        let FramePayload::TelemetryEvent(body) = frame.payload else {
            panic!("probe must report through telemetry")
        };
        body.data
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wasm_skill_authoring_enforces_root_identity_and_capability() {
        let document = "---\nid='recall-probe.example'\nversion='1.0.0'\nname='Witness'\ndescription='test'\nrequired_capabilities=['skill.author.self']\n---\nDo the work.\n";
        let script = format!(
            "skill-author {}\nskill-propose foreign.skill 1.0.0 {}",
            hex::encode(document),
            hex::encode("change")
        );
        let allowed = tempfile::tempdir().unwrap();
        let (entries, rows) = run(allowed.path(), true, script.clone()).await;
        assert_eq!(
            entries.len(),
            1,
            "journal intents: {:?}",
            rows.iter()
                .map(|row| (&row.intent, String::from_utf8_lossy(&row.payload_redacted)))
                .collect::<Vec<_>>()
        );
        assert_eq!(entries[0].state, SkillAdmissionState::Pending);
        assert_eq!(entries[0].author.as_ref().unwrap().class, ID);
        assert!(rows
            .iter()
            .any(|row| row.intent == "skill.author.refused:target-not-owned"));
        let report = guest_report(&rows);
        assert!(report.contains("skill ok "), "{report}");
        assert!(report.contains("skill err target-not-owned\n"), "{report}");
        let denied = tempfile::tempdir().unwrap();
        let (entries, rows) = run(denied.path(), false, script).await;
        assert!(entries.is_empty());
        assert!(rows
            .iter()
            .any(|row| row.intent == "skill.author.refused:capability-denied"));
        let report = guest_report(&rows);
        assert!(report.contains("skill err capability-denied\n"), "{report}");
    }
}
