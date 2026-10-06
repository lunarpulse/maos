#![cfg(all(feature = "network", feature = "wasm-host", target_os = "linux"))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionLaunch};
use maos_bin::supervision::{unload_all_loaded, WorkerSupervisor};
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};

const NONCE: u64 = 0x17_03_c;

fn target_root() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

static FIXTURES: LazyLock<()> = LazyLock::new(|| {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (manifest, wasm) in [
        ("crates/maos-wasm-host/guests/echo-spirit/Cargo.toml", true),
        ("crates/maos-wasm-host/guests/routed-relay/Cargo.toml", true),
        (
            "crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml",
            false,
        ),
    ] {
        let mut command = std::process::Command::new("cargo");
        command
            .args([
                "build",
                "--offline",
                "--locked",
                "--release",
                "--manifest-path",
            ])
            .arg(workspace.join(manifest))
            .arg("--target-dir")
            .arg(target_root());
        if wasm {
            command.args(["--target", "wasm32-wasip2"]);
        }
        let output = command.output().expect("build real transport fixture");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = std::process::Command::new("cargo")
        .args([
            "build",
            "--offline",
            "--locked",
            "--release",
            "-p",
            "maos-wasm-host",
            "--bin",
            "maos-wasm-runner",
        ])
        .arg("--target-dir")
        .arg(target_root())
        .current_dir(&workspace)
        .output()
        .expect("build the actual contained runner");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The operator's audit surface (`maosctl audit query`), read by the FR4 gates.
    let output = std::process::Command::new("cargo")
        .args([
            "build",
            "--offline",
            "--locked",
            "-p",
            "maos-cli",
            "--bin",
            "maosctl",
        ])
        .arg("--target-dir")
        .arg(target_root())
        .current_dir(workspace)
        .output()
        .expect("build the actual operator CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
});

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    policy: Arc<PolicyTable>,
    security: Arc<SecurityManagerAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    iac: Arc<IacBusAdapter>,
    halt: Arc<maos_kernel_core::halt::HaltRegistry>,
    supervisor: Arc<WorkerSupervisor>,
    root: maos_bin::supervision::RootSupervision,
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    audit_writer: tokio::task::JoinHandle<()>,
}

impl World {
    fn new(dir: &Path) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ));
        let database = dir.join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&database).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&database)
                    .unwrap(),
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
            Arc::clone(&halt),
            Arc::clone(&iac),
            metrics,
            Arc::new(maos_director_surface::notification::NotificationDispatcher::default()),
            NONCE,
        );
        let supervisor = Arc::clone(root.supervisor());
        let audit_writer = cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl));
        Self {
            tl,
            capability,
            policy,
            security,
            memory,
            journal,
            scheduler,
            iac,
            halt,
            supervisor,
            root,
            lookup: Arc::new(RwLock::new(BTreeMap::new())),
            audit_writer,
        }
    }

    fn admission(&self) -> AdmissionDeps<'_> {
        AdmissionDeps {
            scheduler: &self.scheduler,
            security: &self.security,
            transparency_log: &self.tl,
            journal: &self.journal,
            memory: &self.memory,
            pid_by_spirit_id: &self.lookup,
        }
    }

    fn session(&self) -> SessionDeps {
        SessionDeps {
            supervisor: Arc::clone(&self.supervisor),
            capability: Arc::clone(&self.capability),
            security: Arc::clone(&self.security),
            iac: Arc::clone(&self.iac),
            journal: Arc::clone(&self.tl),
            pid_by_spirit_id: Arc::clone(&self.lookup),
            enterprise: None,
            pdp: None,
        }
    }

    fn governed_session(&self, cedar_policy: &str) -> SessionDeps {
        let mut session = self.session();
        session.pdp = Some(
            maos_bin::enterprise_pdp_runtime::EnterprisePdpRuntime::new(
                Arc::new(maos_pdp::CedarPolicyAdapter::with_policy(cedar_policy).unwrap()),
                Arc::clone(&self.policy),
                Duration::from_secs(30),
                Duration::from_secs(300),
            )
            .unwrap(),
        );
        session
    }

    async fn drain(self) -> Arc<TransparencyLogAdapter> {
        let Self {
            tl,
            capability,
            policy: _,
            security,
            memory,
            journal,
            scheduler,
            iac,
            halt,
            supervisor,
            lookup,
            audit_writer,
            root,
        } = self;
        root.stop_and_join().await;
        drop((
            capability, security, memory, journal, scheduler, iac, halt, supervisor, lookup,
        ));
        tokio::time::timeout(Duration::from_secs(5), audit_writer)
            .await
            .expect("real audit writer drains after all senders close")
            .unwrap();
        tl
    }
}

/// A schema-5 session manifest granting `iac.send = ["spirit:peer"]` (so the
/// guest can deliver addressed frames) and, when `grant`, the delegable MCP
/// scope.
fn manifest(id: &str, form: &str, grant: bool) -> String {
    manifest_with_sends(id, form, grant, Some(&["spirit:peer"]))
}

/// `sends`: the `[capabilities.required.iac].send` grant, or none at all.
fn manifest_with_sends(id: &str, form: &str, grant: bool, sends: Option<&[&str]>) -> String {
    let artifact = if form == "wasm-component" {
        "artifact = \"echo_spirit.wasm\"\n"
    } else {
        ""
    };
    let mut caps = String::new();
    if grant {
        caps.push_str("\n[capabilities.required]\nprovider.complete = [\"anthropic.claude-3-haiku-20240307\"]\n[[capabilities.required.mcp.servers]]\nname = \"scope-tools\"\nallowed_tools = [\"search\"]\n");
    }
    if let Some(sends) = sends {
        let list = sends
            .iter()
            .map(|send| format!("\"{send}\""))
            .collect::<Vec<_>>()
            .join(", ");
        caps.push_str(&format!("\n[capabilities.required.iac]\nsend = [{list}]\n"));
    }
    format!("[class]\nname = \"{id}\"\ndescription = \"Live scoped assignment fixture\"\nversion = \"0.1.0\"\nabi = \"1.0\"\nmanifest_schema_version = 5\nmin_substrate_version = \"0.1.0-alpha.1\"\nforms = [\"{form}\"]\n{artifact}trust_tier = \"local\"\n[posture]\ndefault = \"assistive\"\nallowed_max = \"assistive\"\n[output_shape]\nrequired_fields = [\"introduction\"]\n[budget]\ncontext_window_size = 4096\ntime_cap_seconds = 30\n[resources]\nmemory_max_mb = 64\n[sandbox]\ntier = \"T2\"\n{caps}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eight_real_admitted_forms_use_trusted_sender_policy_not_echoed_claims() {
    LazyLock::force(&FIXTURES);
    for wasm in [false, true] {
        for granted in [false, true] {
            for forged in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let world = World::new(dir.path());
                let peer = gate_manifest(&manifest("butler", "subprocess", !granted), &|name| {
                    name == "butler"
                })
                .unwrap();
                let (peer_core, peer_obj) = world
                    .supervisor
                    .prepare_spawned("butler", &peer.bundle())
                    .unwrap();
                let peer_admitted = load_admit_start(
                    &world.admission(),
                    &peer,
                    peer_obj,
                    NONCE,
                    StartPolicy::LeaveLoaded,
                    &mut |pid| peer_core.bind_scheduler_pid(pid),
                )
                .await
                .unwrap();
                let id = if wasm { "scope-wasm" } else { "worker" };
                let form = if wasm { "wasm-component" } else { "subprocess" };
                let gated =
                    gate_manifest(&manifest(id, form, granted), &|name| name == "worker").unwrap();
                let (core, obj) = world
                    .supervisor
                    .prepare_spawned(id, &gated.bundle())
                    .unwrap();
                let admitted = load_admit_start(
                    &world.admission(),
                    &gated,
                    obj,
                    NONCE,
                    StartPolicy::StartNow,
                    &mut |pid| core.bind_scheduler_pid(pid),
                )
                .await
                .unwrap();
                let pid = admitted.pid;
                assert_ne!(pid, 0);
                assert_ne!(pid, peer_admitted.pid);
                let mut receiver = world
                    .iac
                    .register_spirit_typed(&SpiritId::from("scope-receiver"))
                    .unwrap();
                let address = |name: &str| FrameAddress {
                    spirit_id: SpiritId::from(name),
                    host_id: None,
                    role: None,
                };
                let mut claimed = address(if forged { "butler" } else { id });
                if forged {
                    claimed.role = Some(SpiritRole::Director);
                }
                // A child that republishes this seed impersonates its claimed author.
                // The other claimed author was genuinely admitted with the opposite grant.
                let seed = IacFrame {
                    frame_id: [0x17; 16],
                    timestamp_ns: 1,
                    logical_clock: 1,
                    from: claimed,
                    to: smallvec::smallvec![address(id), address("scope-receiver")],
                    kind: FrameKind::TaskAssign,
                    intent: if forged {
                        IntentClass::HighPrivilege
                    } else {
                        IntentClass::Standard
                    },
                    auto_marker: if forged {
                        FrameOrigin::HumanAuthored
                    } else {
                        FrameOrigin::SpiritAuto
                    },
                    payload: FramePayload::TaskAssign(TaskAssignPayload {
                        goal: "delegate search".into(),
                        scope: vec![Scope::McpCall {
                            server: "scope-tools".into(),
                            tool: "search".into(),
                        }],
                        success_criteria: "scope verdict".into(),
                        posture_preferences: Default::default(),
                        prior_distillate_ref: None,
                    }),
                    // A forged consent claim the guest echoes back: it must stay
                    // a recorded claim, never reach a recipient.
                    consent_envelope: forged.then(|| maos_domain::frame::ConsentEnvelope {
                        consent_id: [0x66; 16],
                        granter: address("butler"),
                        timestamp_ns: 1,
                        intent_class: Some(maos_domain::invariants::i8::A2AIntent::new(
                            "forged:consent",
                        )),
                        valid_until_ns: None,
                    }),
                    intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![
                        maos_domain::invariants::i8::A2AIntent::new(
                            "scope-verdict:delegate-search",
                        ),
                    ]),
                };
                let artifact = target_root().join("wasm32-wasip2/release/echo_spirit.wasm");
                let plan = maos_host::SpiritLaunchPlan {
                    program: target_root()
                        .join(if wasm {
                            "release/maos-wasm-runner"
                        } else {
                            "release/equiv-native-twin"
                        })
                        .to_string_lossy()
                        .into_owned(),
                    argv: if wasm {
                        vec![
                            "--component".into(),
                            artifact.to_string_lossy().into_owned(),
                            "--fuel".into(),
                            "10000000".into(),
                        ]
                    } else {
                        Vec::new()
                    },
                    env: Vec::new(),
                    wire: maos_host::WireShape::ContentLengthCbor,
                };
                let report = run_admitted_session(
                    world.governed_session("permit(principal, action, resource);"),
                    admitted,
                    core,
                    SessionLaunch {
                        plan,
                        component_artifact: wasm.then_some(artifact),
                        turn_budget: Duration::from_secs(30),
                        once: true,
                        initial_frame: Some(seed),
                    },
                )
                .await
                .unwrap();
                assert_eq!(report.turns, 1);
                assert_eq!(report.delivered_frames, usize::from(granted));
                assert_eq!(report.denied_frames, usize::from(!granted));
                let rows = world
                    .tl
                    .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                        spirit_pid: Some(pid),
                        ..Default::default()
                    })
                    .unwrap();
                let authorization = rows
                    .iter()
                    .find(|row| row.intent == "spirit.frame.authorization")
                    .expect("real authorization receipt at the admitted PID");
                let authorization: serde_json::Value =
                    serde_json::from_slice(&authorization.payload_redacted).unwrap();
                assert_eq!(authorization["spirit_pid"], pid);
                assert_eq!(authorization["trusted_sender"], id);
                assert_eq!(authorization["allowed"], granted);
                // The send itself is mediated in every case (both forms hold
                // `iac.send`); only the delegated scope verdict varies.
                assert_eq!(authorization["send"]["kernel_verdict"], "allow");
                assert_eq!(
                    authorization["decisions"][0]["kernel_verdict"],
                    if granted { "allow" } else { "deny" }
                );
                assert_eq!(
                    authorization["decisions"][0]["scope"]["McpCall"]["server"],
                    "scope-tools"
                );
                assert_eq!(
                    authorization["claims"]["from"]["spirit_id"],
                    if forged { "butler" } else { id }
                );
                // The guest's echoed claims are recorded verbatim, forged or not.
                let (claimed_intent, claimed_origin) = if forged {
                    (IntentClass::HighPrivilege, FrameOrigin::HumanAuthored)
                } else {
                    (IntentClass::Standard, FrameOrigin::SpiritAuto)
                };
                assert_eq!(
                    authorization["claims"]["intent"],
                    serde_json::to_value(claimed_intent).unwrap()
                );
                assert_eq!(
                    authorization["claims"]["auto_marker"],
                    serde_json::to_value(claimed_origin).unwrap()
                );
                assert_eq!(
                    authorization["claims"]["consent_envelope"].is_null(),
                    !forged
                );
                // Only an authorized frame is journaled as delivered, at the
                // trusted PID and under the trusted sender.
                assert_eq!(
                    rows.iter().any(|row| {
                        row.kind == maos_kernel_core::iac::transparency_log::FrameKind::TaskAssign
                            && row.from_spirit_id == id
                            && row.capability_token.is_some()
                    }),
                    granted,
                    "delivered guest frame row is attributed to admitted pid {pid} with its iac.send token"
                );
                let terminal = rows
                    .iter()
                    .find(|row| row.intent == "cli.subprocess.exit")
                    .unwrap();
                let terminal: serde_json::Value =
                    serde_json::from_slice(&terminal.payload_redacted).unwrap();
                assert_eq!(terminal["is_crash"], false);
                assert!(rows.iter().any(|row| row.intent == "lifecycle.unload"));
                let (_, original) = receiver
                    .try_recv()
                    .unwrap()
                    .expect("actual operator seed delivery");
                assert_eq!(
                    original.intent,
                    if forged {
                        IntentClass::HighPrivilege
                    } else {
                        IntentClass::Standard
                    }
                );
                let emitted = receiver.try_recv().unwrap();
                if granted {
                    let (_, frame) = emitted.expect("authorized guest assignment reaches receiver");
                    assert_eq!(frame.from.spirit_id.as_str(), id);
                    assert_eq!(frame.from.role, None);
                    assert_eq!(frame.intent, IntentClass::Standard);
                    assert_eq!(frame.auto_marker, FrameOrigin::SpiritAuto);
                    assert_eq!(
                        frame.consent_envelope, None,
                        "guest consent claims never deliver"
                    );
                    assert_eq!(
                        frame.intent_lineage.as_slice()[0].as_str(),
                        "scope-verdict:delegate-search",
                        "the turn's inbound lineage, stamped by the kernel"
                    );
                    assert_eq!(
                        frame.logical_clock, 2,
                        "kernel Lamport stamp after inbound clock 1"
                    );
                } else {
                    assert!(
                        emitted.is_none(),
                        "denied assignment has no guest mailbox effect"
                    );
                }
                assert!(
                    world.scheduler.resolve_pid(id).is_none(),
                    "session unload removes its only SCB"
                );
                let cleanup =
                    unload_all_loaded(world.scheduler.as_ref(), world.halt.as_ref()).await;
                assert!(!cleanup.had_failures());
                drop(peer_core);
                world.drain().await;
                println!("SCOPED-VERDICT form={form} grant={granted} forged={forged} trusted_pid={pid} child_pid={} allowed={granted}", report.child_pid);
            }
        }
    }
}

/// What one hostile native session left behind.
struct HostileRun {
    result: Result<maos_bin::spirit_session::SessionReport, maos_bin::spirit_session::SessionError>,
    pid: u32,
    rows: Vec<maos_kernel_core::iac::transparency_log::TransparencyLogEntry>,
    /// Frames the delegated destination received, the operator seed first.
    received: Vec<IacFrame>,
    /// The operator's FR4 view: actual `maosctl audit query --spirit worker`.
    audit: std::process::Output,
}

impl HostileRun {
    fn authorizations(&self) -> Vec<serde_json::Value> {
        self.rows
            .iter()
            .filter(|row| row.intent == "spirit.frame.authorization")
            .map(|row| serde_json::from_slice(&row.payload_redacted).unwrap())
            .collect()
    }

    fn report(&self) -> &maos_bin::spirit_session::SessionReport {
        self.result
            .as_ref()
            .expect("hostile session completes its turn")
    }
}

/// Run `hostile-twin --mode <mode>` as an admitted, T2-sandboxed native
/// session that receives one operator seed addressed to itself and to
/// `scope-receiver`, with the default `iac.send = ["spirit:peer"]` grant.
async fn run_hostile_twin(mode: &str, turn_budget: Duration) -> HostileRun {
    run_hostile_twin_with(mode, turn_budget, Some(&["spirit:peer"])).await
}

async fn run_hostile_twin_with(
    mode: &str,
    turn_budget: Duration,
    sends: Option<&[&str]>,
) -> HostileRun {
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let id = "worker";
    let gated = gate_manifest(
        &manifest_with_sends(id, "subprocess", true, sends),
        &|name| name == id,
    )
    .unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(id, &gated.bundle())
        .unwrap();
    let admitted = load_admit_start(
        &world.admission(),
        &gated,
        obj,
        NONCE,
        StartPolicy::StartNow,
        &mut |pid| core.bind_scheduler_pid(pid),
    )
    .await
    .unwrap();
    let pid = admitted.pid;
    let mut receiver = world
        .iac
        .register_spirit_typed(&SpiritId::from("scope-receiver"))
        .unwrap();
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
        host_id: None,
        role: None,
    };
    let seed = IacFrame {
        frame_id: [0x55; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: address("operator"),
        to: smallvec::smallvec![address(id), address("scope-receiver")],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        auto_marker: FrameOrigin::HumanAuthored,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("hostile {mode}"),
            scope: Vec::new(),
            success_criteria: "kernel-governed outcome".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        consent_envelope: None,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![
            maos_domain::invariants::i8::A2AIntent::new("operator:hostile"),
        ]),
    };
    let plan = maos_host::SpiritLaunchPlan {
        program: target_root()
            .join("release/hostile-twin")
            .to_string_lossy()
            .into_owned(),
        argv: vec!["--mode".into(), mode.into()],
        env: Vec::new(),
        wire: maos_host::WireShape::ContentLengthCbor,
    };
    let result = tokio::time::timeout(
        turn_budget * 4 + Duration::from_secs(20),
        run_admitted_session(
            world.session(),
            admitted,
            core,
            SessionLaunch {
                plan,
                component_artifact: None,
                turn_budget,
                once: true,
                initial_frame: Some(seed),
            },
        ),
    )
    .await
    .expect("a hostile guest can never wedge its session");
    let mut received = Vec::new();
    while let Some((_, frame)) = receiver.try_recv().unwrap() {
        received.push(frame);
    }
    let rows = world
        .tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap();
    assert!(
        world.scheduler.resolve_pid(id).is_none(),
        "every outcome unloads the SCB"
    );
    drop(receiver);
    world.drain().await;
    let audit = operator_audit_view(dir.path(), id);
    HostileRun {
        result,
        pid,
        rows,
        received,
        audit,
    }
}

/// Actual `maosctl audit query --spirit <id>` (the FR4 per-Spirit view) over a
/// drained World's Transparency Log, placed where the operator surface reads it.
fn operator_audit_view(dir: &Path, id: &str) -> std::process::Output {
    let audit_dir = dir.join("home/audit");
    std::fs::create_dir_all(&audit_dir).unwrap();
    for suffix in ["", "-wal", "-shm"] {
        let source = dir.join(format!("transparency.sqlite{suffix}"));
        if source.exists() {
            std::fs::copy(
                &source,
                audit_dir.join(format!("transparency.sqlite{suffix}")),
            )
            .unwrap();
        }
    }
    std::process::Command::new(target_root().join("debug/maosctl"))
        .args(["audit", "query", "--spirit", id, "--format", "plain"])
        .env("MAOS_HOME", dir.join("home"))
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forged_guest_provenance_is_restamped_by_the_kernel() {
    let run = run_hostile_twin("provenance", Duration::from_secs(30)).await;
    assert_eq!(run.report().delivered_frames, 1);
    let [seed, emitted] = run.received.as_slice() else {
        panic!("seed plus one guest frame: {:?}", run.received)
    };
    assert_eq!(seed.from.spirit_id.as_str(), "operator");
    assert_eq!(emitted.from.spirit_id.as_str(), "worker");
    assert_eq!(emitted.consent_envelope, None);
    assert_eq!(
        emitted
            .intent_lineage
            .as_slice()
            .iter()
            .map(|intent| intent.as_str())
            .collect::<Vec<_>>(),
        ["operator:hostile"],
        "lineage comes from the turn's inbound frame, never from the guest"
    );
    assert_eq!(
        emitted.logical_clock, 2,
        "kernel Lamport stamp, not the guest's 999"
    );
    let [authorization] = run.authorizations().try_into().unwrap();
    assert_eq!(authorization["allowed"], true);
    assert_eq!(authorization["claims"]["logical_clock"], 999);
    assert_eq!(
        authorization["claims"]["intent_lineage"][0],
        "forged:lineage"
    );
    assert!(!authorization["claims"]["consent_envelope"].is_null());
    // FR4 / AC1: the delivery was mediated by its own kernel `iac.send` token,
    // and the operator's per-Spirit audit view accepts every row at the pid.
    assert_eq!(authorization["send"]["kernel_verdict"], "allow");
    assert_eq!(
        authorization["send"]["scope"]["IacSend"]["peer_class"],
        "spirit:peer"
    );
    let delivered = run
        .rows
        .iter()
        .find(|row| row.kind == maos_kernel_core::iac::transparency_log::FrameKind::TaskAssign)
        .expect("the guest's delivered TaskAssign row at its trusted pid");
    assert!(
        delivered.capability_token.is_some(),
        "delivery row carries its token"
    );
    let audit = String::from_utf8_lossy(&run.audit.stdout);
    assert!(
        run.audit.status.success(),
        "maosctl audit query --spirit worker: {}{audit}",
        String::from_utf8_lossy(&run.audit.stderr)
    );
    for intent in [
        "spirit.frame.authorization",
        "standard",
        "cli.subprocess.exit",
    ] {
        assert!(
            audit.contains(intent),
            "FR4 view lists `{intent}`:\n{audit}"
        );
    }
    println!(
        "HOSTILE-PROVENANCE trusted_pid={} restamped=lineage,consent,clock fr4_view=ok",
        run.pid
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_delivery_needs_its_iac_send_grant() {
    // No `iac.send` grant at all: the kernel itself denies the send.
    let ungranted = run_hostile_twin_with("provenance", Duration::from_secs(30), None).await;
    // Only broadcast granted: an addressed frame's peer class is not covered.
    let broadcast_only =
        run_hostile_twin_with("provenance", Duration::from_secs(30), Some(&["broadcast"])).await;
    for (run, verdict, refusal) in [
        (&ungranted, Some("deny"), None),
        (&broadcast_only, None, Some("iac_peer_class_not_granted")),
    ] {
        let report = run.report();
        assert_eq!((report.delivered_frames, report.denied_frames), (0, 1));
        let [authorization] = run.authorizations().try_into().unwrap();
        assert_eq!(authorization["allowed"], false);
        assert_eq!(authorization["send"]["kernel_verdict"].as_str(), verdict);
        if let Some(refusal) = refusal {
            assert_eq!(authorization["send"]["refusal"], refusal);
        }
        assert_eq!(run.received.len(), 1, "only the operator seed arrives");
        assert!(
            run.audit.status.success(),
            "a refused send leaves the FR4 view green"
        );
    }
    println!("HOSTILE-IAC-SEND ungranted=deny broadcast_only=iac_peer_class_not_granted");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn kernel_only_cross_host_oversized_and_foreign_retract_frames_are_refused() {
    let run = run_hostile_twin("refusals", Duration::from_secs(30)).await;
    let report = run.report();
    assert_eq!((report.delivered_frames, report.denied_frames), (1, 4));
    let authorizations = run.authorizations();
    let mut refusals = authorizations
        .iter()
        .filter(|record| record["allowed"] == false)
        .map(|record| record["refusal"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    refusals.sort();
    assert_eq!(refusals.len(), 4, "{authorizations:?}");
    assert_eq!(refusals[0], "cross_host_destination");
    assert_eq!(refusals[1], "kernel_reserved_kind");
    assert!(refusals[2].starts_with("retract_"), "{refusals:?}");
    assert_eq!(refusals[3], "scope_limit");
    assert!(
        authorizations
            .iter()
            .all(|record| record["decisions"].as_array().unwrap().is_empty()),
        "frame-level refusals mint nothing"
    );
    assert_eq!(
        run.received.len(),
        2,
        "only the seed and the one admissible frame arrive: {:?}",
        run.received
    );
    println!(
        "HOSTILE-REFUSALS trusted_pid={} refused={refusals:?}",
        run.pid
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn guest_floods_end_in_typed_failures_not_a_wedged_session() {
    use maos_bin::spirit_session::{SessionError, MAX_FRAMES_PER_TURN};
    let flood = run_hostile_twin("flood", Duration::from_secs(30)).await;
    assert!(
        matches!(
            flood.result,
            Err(SessionError::Protocol("frame budget exceeded"))
        ),
        "{:?}",
        flood.result
    );
    assert_eq!(
        flood.authorizations().len(),
        MAX_FRAMES_PER_TURN,
        "frames past the budget are never authorized"
    );
    // Self-addressed ConsentRequests overfill the guest's own mailbox, which
    // only this session drains: delivery must give up at the turn deadline.
    let started = std::time::Instant::now();
    let wedge = run_hostile_twin("self-flood", Duration::from_secs(3)).await;
    assert!(
        matches!(wedge.result, Err(SessionError::Deadline("delivery"))),
        "{:?}",
        wedge.result
    );
    assert!(started.elapsed() < Duration::from_secs(30));
    println!("HOSTILE-FLOODS frame_budget={MAX_FRAMES_PER_TURN} self_flood=deadline");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn guest_diagnostics_are_bounded_per_session() {
    use maos_bin::spirit_session::MAX_DIAGNOSTIC_ROWS;
    let run = run_hostile_twin("noisy", Duration::from_secs(30)).await;
    let report = run.report();
    assert_eq!(report.delivered_frames, 1);
    assert!(report.dropped_diagnostics >= 1);
    let events = run
        .rows
        .iter()
        .filter(|row| row.intent == "spirit.diagnostic")
        .map(|row| {
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted).unwrap()["event"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        events
            .iter()
            .filter(|event| *event == "spirit_diagnostic")
            .count(),
        MAX_DIAGNOSTIC_ROWS
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| *event == "spirit_diagnostic_dropped")
            .count(),
        1,
        "one marker when dropping starts"
    );
    println!(
        "HOSTILE-DIAGNOSTICS journaled={MAX_DIAGNOSTIC_ROWS} dropped={}",
        report.dropped_diagnostics
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn runner_refusal_exits_before_ready_are_typed_and_not_crashes() {
    use maos_bin::spirit_session::SessionError;
    use maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause;
    for (code, expected) in [
        (2, "incompatible_world"),
        (3, "invalid_component"),
        (5, "unrepresentable_frame"),
    ] {
        let run = run_hostile_twin(
            &format!("exit-before-ready-{code}"),
            Duration::from_secs(30),
        )
        .await;
        let error = run.result.as_ref().unwrap_err();
        assert!(
            matches!(
                error,
                SessionError::IncompatibleWorld
                    | SessionError::InvalidComponent
                    | SessionError::UnrepresentableFrame
            ) && error.to_string().starts_with(expected),
            "exit {code} before Ready: {error:?}"
        );
        assert!(
            !run.rows.iter().any(|row| row.intent == "lifecycle.crash"),
            "a typed launch refusal is not a crash"
        );
    }
    let generic = run_hostile_twin("exit-before-ready-1", Duration::from_secs(30)).await;
    assert!(
        matches!(
            generic.result,
            Err(SessionError::Child(ExitCause::Exited { code: 1 }))
        ),
        "{:?}",
        generic.result
    );
    let after_ready = run_hostile_twin("exit-after-ready-3", Duration::from_secs(30)).await;
    assert!(
        matches!(
            after_ready.result,
            Err(SessionError::Child(ExitCause::Exited { code: 3 }))
        ),
        "the same code after Ready is a crash: {:?}",
        after_ready.result
    );
    println!("HOSTILE-EXITS typed_before_ready=2,3,5 crash_after_ready=3");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_once_session_that_never_completes_its_turn_is_not_success() {
    use maos_bin::spirit_session::SessionError;
    let run = run_hostile_twin("ready-then-exit", Duration::from_secs(30)).await;
    assert!(
        matches!(
            run.result,
            Err(SessionError::UnexpectedEof
                | SessionError::Protocol("once session ended before its turn"))
        ),
        "{:?}",
        run.result
    );
    println!("HOSTILE-ONCE turns=0 result=error");
}

/// `aliases`: `count` canonical strings alias one 4 MiB guest range; otherwise
/// `count` zero-filled frames of pure canonical metadata.
fn hostile_component(dir: &Path, aliases: bool, count: usize) -> PathBuf {
    let wit = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wit/spirit.wit");
    let mut resolve = wit_parser::Resolve::new();
    resolve
        .push_source(
            wit.to_str().unwrap(),
            &std::fs::read_to_string(&wit).unwrap(),
        )
        .unwrap();
    let (frame_id, frame) = resolve
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("iac-frame"))
        .unwrap();
    let wit_parser::TypeDefKind::Record(record) = &frame.kind else {
        panic!("iac-frame must be a record")
    };
    let mut sizes = wit_parser::SizeAlign::default();
    sizes.fill(&resolve);
    let stride = sizes.size(&wit_parser::Type::Id(frame_id)).size_wasm32();
    let offsets = sizes.field_offsets(record.fields.iter().map(|field| &field.ty));
    let from_offset = offsets[record
        .fields
        .iter()
        .position(|field| field.name == "frame-from")
        .unwrap()]
    .0
    .size_wasm32();
    let mut setup = String::new();
    if aliases {
        // Four independent canonical strings alias one 4 MiB guest range.
        setup.push_str("i32.const 1048576 i32.const 97 i32.const 4194304 memory.fill\n");
        for index in 0..count {
            let pointer = 65536 + index * stride + from_offset;
            setup.push_str(&format!(
                "i32.const {pointer} i32.const 1048576 i32.store\n"
            ));
            setup.push_str(&format!(
                "i32.const {} i32.const 4194304 i32.store\n",
                pointer + 4
            ));
        }
    }
    let bytes = if aliases {
        6 * 1024 * 1024
    } else {
        65536 + count * stride
    };
    let pages = bytes.div_ceil(65536);
    let wat = format!(
        r#"(module
      (memory (export "memory") {pages})
      (global $heap (mut i32) (i32.const 4096))
      (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $ptr i32)
        global.get $heap local.get 2 i32.const 1 i32.sub i32.add
        local.get 2 i32.const 1 i32.sub i32.const -1 i32.xor i32.and local.tee $ptr
        local.get 3 i32.add global.set $heap local.get $ptr)
      (func (export "on-start") (result i32) i32.const 32)
      (func (export "cabi_post_on-start") (param i32))
      (func (export "on-shutdown"))
      (func (export "handle-frame") (param i32) (result i32)
        {setup}
        i32.const 20 i32.const 65536 i32.store
        i32.const 24 i32.const {count} i32.store
        i32.const 16)
      (func (export "cabi_post_handle-frame") (param i32))
    )"#
    );
    let core = dir.join("hostile.wat");
    let embedded = dir.join("embedded.wasm");
    let component = dir.join("hostile.wasm");
    std::fs::write(&core, wat).unwrap();
    for args in [
        vec![
            "component",
            "embed",
            wit.to_str().unwrap(),
            core.to_str().unwrap(),
            "--world",
            "spirit",
            "--output",
            embedded.to_str().unwrap(),
        ],
        vec![
            "component",
            "new",
            embedded.to_str().unwrap(),
            "--output",
            component.to_str().unwrap(),
        ],
    ] {
        let output = std::process::Command::new("wasm-tools")
            .args(args)
            .output()
            .expect("run actual component assembler");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("HOSTILE-FIXTURE aliases={aliases} canonical_stride={stride} count={count} guest_pages={pages}");
    component
}

/// The hostcall-fuel trap text Wasmtime 46 raises while lifting guest data.
const HOSTCALL_FUEL_EXHAUSTED: &str = "fuel allocated for hostcalls has been exhausted";

/// Runs the hostile component through a real T2 session. `exhausts` selects the
/// over-budget assertions; otherwise this is the under-budget control, which
/// must clear the canonical call the over-budget case dies in.
async fn assert_hostile_lifting(aliases: bool, count: usize, exhausts: bool) {
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let artifact = hostile_component(dir.path(), aliases, count);
    let world = World::new(dir.path());
    let id = "hostile-lifting";
    // The approved runtime contract (768 MiB process cap), not a smaller cap
    // that could kill the runner before hostcall fuel decides.
    let text =
        manifest(id, "wasm-component", false).replace("memory_max_mb = 64", "memory_max_mb = 768");
    let gated = gate_manifest(&text, &|_| false).unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(id, &gated.bundle())
        .unwrap();
    let admitted = load_admit_start(
        &world.admission(),
        &gated,
        obj,
        NONCE,
        StartPolicy::StartNow,
        &mut |pid| core.bind_scheduler_pid(pid),
    )
    .await
    .unwrap();
    let pid = admitted.pid;
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
        host_id: None,
        role: None,
    };
    let seed = IacFrame {
        frame_id: [0x17; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: address("operator"),
        to: smallvec::smallvec![address(id)],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        auto_marker: FrameOrigin::HumanAuthored,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "bounded lifting probe".into(),
            scope: Vec::new(),
            success_criteria: "controlled pre-serialization failure".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        consent_envelope: None,
        intent_lineage: Default::default(),
    };
    let plan = maos_host::SpiritLaunchPlan {
        program: target_root()
            .join("release/maos-wasm-runner")
            .to_string_lossy()
            .into_owned(),
        argv: vec![
            "--component".into(),
            artifact.to_string_lossy().into_owned(),
            "--fuel".into(),
            "10000000".into(),
        ],
        env: Vec::new(),
        wire: maos_host::WireShape::ContentLengthCbor,
    };
    let result = run_admitted_session(
        world.session(),
        admitted,
        core,
        SessionLaunch {
            plan,
            component_artifact: Some(artifact),
            turn_budget: Duration::from_secs(30),
            once: true,
            initial_frame: Some(seed),
        },
    )
    .await;
    let rows = world
        .tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap();
    let diagnostics = rows
        .iter()
        .filter(|row| row.intent == "spirit.diagnostic")
        .filter_map(|row| {
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted).unwrap()["line"]
                .as_str()
                .map(str::to_owned)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let stage = diagnostics
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| event["event"] == "guest_export_failed");
    if !exhausts {
        assert!(
            stage.is_none() && !diagnostics.contains(HOSTCALL_FUEL_EXHAUSTED),
            "an under-budget output must clear the canonical call: {diagnostics}"
        );
        assert!(world.scheduler.resolve_pid(id).is_none());
        world.drain().await;
        println!("HOSTILE-LIFTING-CONTROL aliases={aliases} count={count} canonical_call=passed");
        return;
    }
    assert!(matches!(result, Err(maos_bin::spirit_session::SessionError::Child(
            maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause::Exited { code: 1 }
        ))), "controlled canonical-call error, not OOM, fuel exhaustion, or successful serialization: {result:?}");
    let stage = stage.expect("real runner export-stage diagnostic");
    assert_eq!(stage["export"], "handle-frame");
    assert_eq!(stage["stage"], "canonical-call");
    assert!(
        diagnostics.contains(HOSTCALL_FUEL_EXHAUSTED),
        "the canonical call must die of hostcall-fuel exhaustion, not another trap: {diagnostics}"
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.intent == "spirit.frame.authorization"),
        "no frame reached domain conversion and authorization"
    );
    assert!(world.scheduler.resolve_pid(id).is_none());
    world.drain().await;
    println!(
        "HOSTILE-LIFTING aliases={aliases} count={count} trusted_pid={pid} cause=hostcall-fuel"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hostile_metadata_fails_before_serialization_under_real_t2() {
    assert_hostile_lifting(false, 65536, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hostile_aliased_strings_fail_before_serialization_under_real_t2() {
    assert_hostile_lifting(true, 4, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_aliased_string_under_the_hostcall_budget_clears_the_canonical_call() {
    assert_hostile_lifting(true, 1, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_runner_sigsys_reaches_escape_consumer_but_planned_stop_does_not() {
    LazyLock::force(&FIXTURES);
    let build = std::process::Command::new("cargo")
        .args([
            "build",
            "--offline",
            "--locked",
            "-p",
            "maos-wasm-host",
            "--features",
            "runner-fault-inject",
            "--bin",
            "maos-wasm-runner",
        ])
        .arg("--target-dir")
        .arg(target_root())
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    for forbidden in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let world = World::new(dir.path());
        let id = "runner-sigsys";
        // Debug instrumentation uses the separately approved candidate process
        // ceiling; this is not a claim that the debug binary fits in 64 MiB.
        let text = manifest(id, "wasm-component", false)
            .replace("memory_max_mb = 64", "memory_max_mb = 768");
        let gated = gate_manifest(&text, &|_| false).unwrap();
        let (core, obj) = world
            .supervisor
            .prepare_spawned(id, &gated.bundle())
            .unwrap();
        let admitted = load_admit_start(
            &world.admission(),
            &gated,
            obj,
            NONCE,
            StartPolicy::StartNow,
            &mut |pid| core.bind_scheduler_pid(pid),
        )
        .await
        .unwrap();
        let pid = admitted.pid;
        let artifact = target_root().join("wasm32-wasip2/release/echo_spirit.wasm");
        let mut argv = vec![
            "--component".into(),
            artifact.to_string_lossy().into_owned(),
        ];
        if forbidden {
            argv.push("--test-forbidden-syscall-after-ready".into());
        }
        let plan = maos_host::SpiritLaunchPlan {
            program: target_root()
                .join("debug/maos-wasm-runner")
                .to_string_lossy()
                .into_owned(),
            argv,
            env: Vec::new(),
            wire: maos_host::WireShape::ContentLengthCbor,
        };
        let session = tokio::spawn(run_admitted_session(
            world.session(),
            admitted,
            core,
            SessionLaunch {
                plan,
                component_artifact: Some(artifact),
                turn_budget: Duration::from_secs(30),
                once: false,
                initial_frame: None,
            },
        ));
        if !forbidden {
            tokio::time::timeout(Duration::from_secs(10), async {
                while !world
                    .supervisor
                    .bindings()
                    .iter()
                    .any(|binding| binding.child_pid.is_some())
                {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("real runner attached");
            world.supervisor.stop_workers();
        }
        let result = tokio::time::timeout(Duration::from_secs(30), session)
            .await
            .unwrap()
            .unwrap();
        if forbidden {
            assert!(
                matches!(
                    result,
                    Err(maos_bin::spirit_session::SessionError::Child(
                        maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause::Signaled {
                            signal: 31
                        }
                    ))
                ),
                "actual seccomp SIGSYS, never a synthetic receipt: {result:?}"
            );
        } else {
            assert_eq!(
                world.supervisor.bindings()[0].phase,
                maos_bin::supervision::BindingPhase::PlannedStop
            );
            assert!(world.supervisor.counters().signals_sent() >= 1);
        }
        let tl = world.drain().await;
        let blocks = tl
            .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                spirit_pid: Some(pid),
                kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::SandboxBlock),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(blocks.len(), usize::from(forbidden));
        let declaration = maos_escape_detector::ManifestDeclaration {
            spirit_pid: i64::from(pid),
            declared_tier: 2,
            anticipated_kill: false,
        };
        let anomalies = maos_bin::escape_detector_consumer::report_escape_anomalies(
            &dir.path().join("transparency.sqlite"),
            &[declaration],
        )
        .unwrap();
        assert_eq!(anomalies, usize::from(forbidden));
        // Epic AC4: the kind-8 SandboxBlock row shows in `maos audit query`.
        let audit = operator_audit_view(dir.path(), id);
        let view = String::from_utf8_lossy(&audit.stdout);
        assert!(
            audit.status.success(),
            "maosctl audit query --spirit {id}: {}{view}",
            String::from_utf8_lossy(&audit.stderr)
        );
        assert_eq!(view.contains("sandbox.block"), forbidden, "{view}");
        println!("RUNNER-SIGSYS forbidden={forbidden} admitted_pid={pid} actual_blocks={} consumer_anomalies={anomalies} audit_view=ok", blocks.len());
    }
    // Same component, release runner: without the flag it reaches Ready (and
    // exits cleanly on stdin EOF); with it the parser refuses the unknown flag.
    let release = target_root().join("release/maos-wasm-runner");
    let artifact = target_root().join("wasm32-wasip2/release/echo_spirit.wasm");
    let control = std::process::Command::new(&release)
        .arg("--component")
        .arg(&artifact)
        .output()
        .unwrap();
    assert_eq!(control.status.code(), Some(0));
    assert!(
        control.stdout.starts_with(b"Content-Length: 0"),
        "the release runner reaches Ready without the flag"
    );
    let refused = std::process::Command::new(&release)
        .arg("--component")
        .arg(&artifact)
        .arg("--test-forbidden-syscall-after-ready")
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty(), "no Ready before the refusal");
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("unknown argument: --test-forbidden-syscall-after-ready"),
        "the release parser does not know the debug-only flag: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn repeated_mailbox_exports_survive_idle_without_an_active_task_or_stall() {
    // Covers runner startup (compile) and each export; idle must outlast it.
    const TURN_BUDGET: Duration = Duration::from_secs(10);
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let id = "routed-relay";
    let text = manifest(id, "wasm-component", false)
        .replace("echo_spirit.wasm", "routed_relay.wasm")
        .replace("time_cap_seconds = 30", "time_cap_seconds = 10");
    let gated = gate_manifest(&text, &|_| false).unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(id, &gated.bundle())
        .unwrap();
    let admitted = load_admit_start(
        &world.admission(),
        &gated,
        obj,
        NONCE,
        StartPolicy::StartNow,
        &mut |pid| core.bind_scheduler_pid(pid),
    )
    .await
    .unwrap();
    let pid = admitted.pid;
    let scbs = world.scheduler.scbs();
    let scb = {
        let guard = match scbs.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        Arc::clone(guard.get(&pid).unwrap())
    };
    let ledger_empty = || match scb.task_assignments_in_flight.lock() {
        Ok(ledger) => ledger.is_empty(),
        Err(poisoned) => poisoned.into_inner().is_empty(),
    };
    let mut receiver = world
        .iac
        .register_spirit_typed(&SpiritId::from("scope-receiver"))
        .unwrap();
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
        host_id: None,
        role: None,
    };
    let seed = |number: u8| IacFrame {
        frame_id: [number; 16],
        timestamp_ns: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
        logical_clock: u64::from(number),
        from: address("operator"),
        to: smallvec::smallvec![address(id), address("scope-receiver")],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        auto_marker: FrameOrigin::HumanAuthored,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("mailbox turn {number}"),
            scope: Vec::new(),
            success_criteria: "one routed guest export".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        consent_envelope: None,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![
            maos_domain::invariants::i8::A2AIntent::new("operator:mailbox-turn"),
        ]),
    };
    let artifact = target_root().join("wasm32-wasip2/release/routed_relay.wasm");
    let plan = maos_host::SpiritLaunchPlan {
        program: target_root()
            .join("release/maos-wasm-runner")
            .to_string_lossy()
            .into_owned(),
        argv: vec![
            "--component".into(),
            artifact.to_string_lossy().into_owned(),
        ],
        env: Vec::new(),
        wire: maos_host::WireShape::ContentLengthCbor,
    };
    let mut session = tokio::spawn(run_admitted_session(
        world.session(),
        admitted,
        core,
        SessionLaunch {
            plan,
            component_artifact: Some(artifact),
            turn_budget: TURN_BUDGET,
            once: false,
            initial_frame: Some(seed(1)),
        },
    ));
    for number in [1, 2] {
        if number == 2 {
            world
                .iac
                .deliver_typed(seed(number), 0, None)
                .await
                .unwrap();
        }
        let reply_wait = tokio::time::timeout(TURN_BUDGET, async {
            loop {
                if let Ok(Some((_, frame))) = receiver.try_recv() {
                    if frame.from.spirit_id.as_str() == id {
                        break frame;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        let reply = tokio::select! {
            outcome = &mut session => panic!("session ended before mailbox reply {number}: {outcome:?}"),
            reply = reply_wait => reply.expect("actual guest reply through its mailbox turn"),
        };
        let FramePayload::TaskAssign(task) = reply.payload else {
            panic!("guest reply payload")
        };
        assert_eq!(task.goal, format!("mailbox turn {number}"));
        assert_eq!(reply.intent, IntentClass::Standard);
        assert_eq!(reply.auto_marker, FrameOrigin::SpiritAuto);
        tokio::time::timeout(Duration::from_secs(3), async {
            while !ledger_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("TurnComplete retires the real SCB task");
        if number == 1 {
            // Idle longer than the export budget: an idle session has no
            // active-turn deadline and no in-flight task. (The watchdog
            // baseline reset for the next turn is pinned directly by
            // `a_new_turn_after_idle_restarts_the_progress_window`.)
            tokio::time::sleep(TURN_BUDGET + Duration::from_secs(1)).await;
            assert!(
                !session.is_finished(),
                "healthy mailbox idle has no active-turn deadline"
            );
            assert!(ledger_empty());
        }
    }
    world.supervisor.stop_workers();
    tokio::time::timeout(Duration::from_secs(10), session)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(
        world.supervisor.bindings()[0].phase,
        maos_bin::supervision::BindingPhase::PlannedStop
    );
    drop(scb);
    let tl = world.drain().await;
    let rows = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap();
    let fault = rows
        .iter()
        .find(|row| row.intent == "task.stalled" || row.intent == "lifecycle.crash");
    assert!(
        fault.is_none(),
        "unexpected lifecycle fault: {:?}",
        fault.map(|row| (&row.intent, String::from_utf8_lossy(&row.payload_redacted)))
    );
    println!("REPEATED-MAILBOX admitted_pid={pid} exports=2 idle_seconds={} turn_budget_seconds={} planned_stop=true", TURN_BUDGET.as_secs() + 1, TURN_BUDGET.as_secs());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_turn_after_idle_restarts_the_progress_window() {
    use std::sync::atomic::Ordering;
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let id = "progress-window";
    let gated = gate_manifest(&manifest(id, "wasm-component", false), &|_| false).unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(id, &gated.bundle())
        .unwrap();
    let admitted = load_admit_start(
        &world.admission(),
        &gated,
        obj,
        NONCE,
        StartPolicy::StartNow,
        &mut |pid| core.bind_scheduler_pid(pid),
    )
    .await
    .unwrap();
    let scb = {
        let scbs = world.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(guard.get(&admitted.pid).unwrap())
    };
    let ledger_len = || match scb.task_assignments_in_flight.lock() {
        Ok(ledger) => ledger.len(),
        Err(poisoned) => poisoned.into_inner().len(),
    };
    let binding = world.supervisor.adopt_spawned(core).unwrap();
    // Older than the default 30 s no-progress threshold, as after a long idle.
    let idle_age = Duration::from_secs(40).as_nanos() as u64;
    for turn in 1..=2 {
        let stale =
            maos_kernel_core::capability::cap_tokens::monotonic_now_ns().saturating_sub(idle_age);
        scb.last_progress_iac_ns.store(stale, Ordering::Relaxed);
        world
            .supervisor
            .begin_turn(binding.core(), format!("turn:{turn}"))
            .unwrap();
        let age = maos_kernel_core::capability::cap_tokens::monotonic_now_ns()
            .saturating_sub(scb.last_progress_iac_ns.load(Ordering::Relaxed));
        assert!(
            age < Duration::from_secs(5).as_nanos() as u64,
            "turn {turn} starts its own no-progress window, not the idle one (age {age} ns)"
        );
        assert_eq!(ledger_len(), 1);
        world.supervisor.complete_turn(binding.core());
        assert_eq!(ledger_len(), 0);
    }
    // `finish` unloads through the scheduler's blocking executor.
    tokio::task::block_in_place(|| {
        binding.finish(
            &maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause::Exited { code: 0 },
        )
    });
    drop(scb);
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn configured_enterprise_and_mixed_scope_refusals_are_atomic_for_both_forms() {
    LazyLock::force(&FIXTURES);
    for wasm in [false, true] {
        for case in ["enterprise_denied", "policy_mixed", "unmediated"] {
            let enterprise_denied = case == "enterprise_denied";
            let dir = tempfile::tempdir().unwrap();
            let world = World::new(dir.path());
            let id = if wasm { "scope-wasm" } else { "worker" };
            let form = if wasm { "wasm-component" } else { "subprocess" };
            let gated = gate_manifest(&manifest(id, form, true), &|name| name == "worker").unwrap();
            let (core, obj) = world
                .supervisor
                .prepare_spawned(id, &gated.bundle())
                .unwrap();
            let admitted = load_admit_start(
                &world.admission(),
                &gated,
                obj,
                NONCE,
                StartPolicy::StartNow,
                &mut |pid| core.bind_scheduler_pid(pid),
            )
            .await
            .unwrap();
            let pid = admitted.pid;
            let address = |name: &str| FrameAddress {
                spirit_id: SpiritId::from(name),
                host_id: None,
                role: None,
            };
            let mut scopes = vec![Scope::McpCall {
                server: "scope-tools".into(),
                tool: "search".into(),
            }];
            match case {
                // Granted MCP first, then a kernel-mediated kind the manifest
                // never granted: the first token must be rolled back.
                "policy_mixed" => scopes.push(Scope::FsWrite {
                    subtree: "/etc".into(),
                }),
                // An ABI scope the kernel cannot mediate refuses the frame
                // before any mint.
                "unmediated" => scopes.push(Scope::SkillAuthorSelf),
                _ => {}
            }
            let seed = IacFrame {
                frame_id: [0x37; 16],
                timestamp_ns: 1,
                logical_clock: 1,
                from: address("operator"),
                to: smallvec::smallvec![address(id), address("scope-receiver")],
                kind: FrameKind::TaskAssign,
                intent: IntentClass::Standard,
                auto_marker: FrameOrigin::HumanAuthored,
                payload: FramePayload::TaskAssign(TaskAssignPayload {
                    goal: "atomic governed delegation".into(),
                    scope: scopes,
                    success_criteria: "no partially authorized guest delivery".into(),
                    posture_preferences: Default::default(),
                    prior_distillate_ref: None,
                }),
                consent_envelope: None,
                intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![
                    maos_domain::invariants::i8::A2AIntent::new("operator:atomic-delegation"),
                ]),
            };
            let mut receiver = world
                .iac
                .register_spirit_typed(&SpiritId::from("scope-receiver"))
                .unwrap();
            let artifact = target_root().join("wasm32-wasip2/release/echo_spirit.wasm");
            let plan = maos_host::SpiritLaunchPlan {
                program: target_root()
                    .join(if wasm {
                        "release/maos-wasm-runner"
                    } else {
                        "release/equiv-native-twin"
                    })
                    .to_string_lossy()
                    .into_owned(),
                argv: if wasm {
                    vec![
                        "--component".into(),
                        artifact.to_string_lossy().into_owned(),
                        "--fuel".into(),
                        "10000000".into(),
                    ]
                } else {
                    Vec::new()
                },
                env: Vec::new(),
                wire: maos_host::WireShape::ContentLengthCbor,
            };
            let policy = if enterprise_denied {
                "permit(principal, action, resource); forbid(principal, action, resource);"
            } else {
                "permit(principal, action, resource);"
            };
            let report = run_admitted_session(
                world.governed_session(policy),
                admitted,
                core,
                SessionLaunch {
                    plan,
                    component_artifact: wasm.then_some(artifact),
                    turn_budget: Duration::from_secs(30),
                    once: true,
                    initial_frame: Some(seed),
                },
            )
            .await
            .unwrap();
            assert_eq!(report.turns, 1);
            assert_eq!(report.delivered_frames, 0);
            assert_eq!(report.denied_frames, 1);
            let (_, original) = receiver
                .try_recv()
                .unwrap()
                .expect("operator seed was actually delivered");
            assert_eq!(original.from.spirit_id.as_str(), "operator");
            assert!(
                receiver.try_recv().unwrap().is_none(),
                "no guest effect from partial or enterprise-denied authority"
            );
            let tl = world.drain().await;
            let all_rows = tl.query_frames(Default::default()).unwrap();
            let rows = all_rows
                .iter()
                .filter(|row| row.spirit_pid == pid)
                .collect::<Vec<_>>();
            let authorization: serde_json::Value = serde_json::from_slice(
                &rows
                    .iter()
                    .find(|row| row.intent == "spirit.frame.authorization")
                    .unwrap()
                    .payload_redacted,
            )
            .unwrap();
            assert_eq!(authorization["allowed"], false);
            assert_eq!(authorization["trusted_sender"], id);
            let decisions = authorization["decisions"].as_array().unwrap();
            // Tokens minted for this frame's delegated MCP scope (issue rows
            // journal the scope as payload at the trusted pid).
            let issued = rows
                .iter()
                .filter(|row| {
                    row.intent.starts_with("cap.issue")
                        && String::from_utf8_lossy(&row.payload_redacted).contains("scope-tools")
                })
                .filter_map(|row| row.capability_token)
                .collect::<Vec<_>>();
            match case {
                "enterprise_denied" => {
                    // Cedar forbids the send itself, before any scope is delegated.
                    let send = &authorization["send"];
                    assert!(send["kernel_verdict"].is_null());
                    assert!(
                        send["refusal"]
                            .as_str()
                            .is_some_and(|refusal| !refusal.is_empty()),
                        "real Cedar forbid refuses before the kernel mint: {send:?}"
                    );
                    assert!(
                        decisions.is_empty(),
                        "nothing delegated after a refused send"
                    );
                    assert!(
                        issued.is_empty(),
                        "no token minted under an enterprise deny"
                    );
                }
                "policy_mixed" => {
                    assert_eq!(decisions.len(), 2);
                    assert_eq!(decisions[0]["kernel_verdict"], "allow");
                    assert_eq!(decisions[1]["kernel_verdict"], "deny");
                    // The allowed scope's token was really minted, and the
                    // refusal revoked it (operator revocations journal at pid 0
                    // with the token id; unload's bulk revoke carries no id).
                    assert_eq!(issued.len(), 1, "exactly the granted scope was minted");
                    assert!(
                        all_rows.iter().any(|row| row.intent == "cap.revoke"
                            && row.capability_token == Some(issued[0])),
                        "the partially authorized frame's token is rolled back"
                    );
                }
                _ => {
                    assert_eq!(decisions.len(), 1);
                    assert_eq!(decisions[0]["kernel_verdict"], "unmediated");
                    assert_eq!(decisions[0]["scope"], "SkillAuthorSelf");
                    assert!(issued.is_empty(), "an unmediated scope mints nothing first");
                }
            }
            let terminal: serde_json::Value = serde_json::from_slice(
                &rows
                    .iter()
                    .find(|row| row.intent == "cli.subprocess.exit")
                    .unwrap()
                    .payload_redacted,
            )
            .unwrap();
            assert_eq!(terminal["is_crash"], false);
            println!("GOVERNED-ATOMIC form={form} case={case} trusted_pid={pid} decisions={} minted={} guest_effects=0", decisions.len(), issued.len());
        }
    }
}
