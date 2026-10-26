#![cfg(all(feature = "network", feature = "wasm-host", target_os = "linux"))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionLaunch};
use maos_bin::supervision::{unload_all_loaded, WorkerSupervisor};
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::PolicyTable,
    cap_quota::{CapQuotaTracker, Charge, QuotaClock, DEFAULT_ISSUANCE_QUOTA},
    CapabilityRegistryAdapter,
};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};

const NONCE: u64 = 0x17_03_c;

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

/// Prebuilt transport fixtures (echo-spirit, routed-relay, native twins) and the
/// actual contained release runner, resolved once from the fixture manifest;
/// every launch site runs exactly these paths (Story 20-12 review), and the
/// operator CLI (`maosctl audit query`, read by the FR4 gates) is validated.
struct Fixtures {
    echo: PathBuf,
    relay: PathBuf,
    native_twin: PathBuf,
    hostile_twin: PathBuf,
    runner: PathBuf,
}

static FIXTURES: LazyLock<Fixtures> = LazyLock::new(|| {
    fixtures::profile_binary("maosctl");
    Fixtures {
        echo: fixtures::fixture("echo_spirit.wasm"),
        relay: fixtures::fixture("routed_relay.wasm"),
        native_twin: fixtures::fixture("equiv-native-twin"),
        hostile_twin: fixtures::fixture("hostile-twin"),
        runner: fixtures::fixture("maos-wasm-runner"),
    }
});

#[derive(Debug, Default)]
struct ManualQuotaClock(AtomicU64);

impl QuotaClock for ManualQuotaClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    quota: CapQuotaTracker,
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
        Self::new_with_seams(dir, maos_bin::supervision::SupervisorSeams::default())
    }

    fn new_with_seams(dir: &Path, seams: maos_bin::supervision::SupervisorSeams) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let quota = CapQuotaTracker::with_clock(
            DEFAULT_ISSUANCE_QUOTA,
            Arc::new(ManualQuotaClock::default()),
        );
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            quota.clone(),
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
        let root = maos_bin::supervision::RootSupervision::arm_with_seams(
            Arc::clone(&scheduler),
            crash,
            Arc::clone(&tl),
            Arc::clone(&journal),
            Arc::clone(&halt),
            Arc::clone(&iac),
            metrics,
            Arc::new(maos_director_surface::notification::NotificationDispatcher::default()),
            NONCE,
            seams,
        );
        let supervisor = Arc::clone(root.supervisor());
        let audit_writer = cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl));
        Self {
            tl,
            capability,
            quota,
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

    fn exhaust_issuance(&self, pid: u32) -> Vec<Charge> {
        let mut charges = Vec::new();
        while let Ok(charge) = self.quota.charge(pid) {
            charges.push(charge);
        }
        assert!(!charges.is_empty(), "the admitted pid starts with capacity");
        charges
    }

    fn admission(&self) -> AdmissionDeps<'_> {
        AdmissionDeps {
            scheduler: &self.scheduler,
            security: &self.security,
            transparency_log: &self.tl,
            journal: &self.journal,
            memory: &self.memory,
            pid_by_spirit_id: &self.lookup,
            // Story 17-4 — the WASM form never registers with a hook pump.
            hooks: None,
        }
    }

    fn session(&self) -> SessionDeps {
        SessionDeps {
            supervisor: Arc::clone(&self.supervisor),
            capability: Arc::clone(&self.capability),
            security: Arc::clone(&self.security),
            iac: Arc::clone(&self.iac),
            journal: Arc::clone(&self.tl),
            log_recall: Arc::new(maos_iac::adapter::log_recall::LogRecallAdapter::new(
                Arc::clone(&self.tl),
            )),
            lifecycle_journal: Arc::clone(&self.journal),
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
            quota: _,
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

async fn admit_t2(
    world: &World,
    spirit_id: &str,
    resources: &str,
) -> (
    Arc<maos_bin::supervision::BindingCore>,
    maos_bin::admission::Admitted,
) {
    let text = manifest_with_sends(spirit_id, "wasm-component", false, None)
        .replace("[resources]\nmemory_max_mb = 64\n", resources);
    let gated = gate_manifest(&text, &|_| false).unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(spirit_id, &gated.bundle())
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
    (core, admitted)
}

fn echo_launch(spirit_id: &str) -> SessionLaunch {
    let artifact = FIXTURES.echo.clone();
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
        host_id: None,
        role: None,
    };
    SessionLaunch {
        plan: maos_host::SpiritLaunchPlan {
            program: FIXTURES.runner.to_string_lossy().into_owned(),
            argv: vec![
                "--component".into(),
                artifact.to_string_lossy().into_owned(),
                "--fuel".into(),
                "10000000".into(),
            ],
            env: Vec::new(),
            wire: maos_host::WireShape::ContentLengthCbor,
        },
        component_artifact: Some(artifact),
        turn_budget: Duration::from_secs(30),
        once: true,
        context_window: Default::default(),
        initial_frame: Some(IacFrame {
            frame_id: ulid::Ulid::new().to_bytes(),
            timestamp_ns: 1,
            logical_clock: 1,
            from: address("operator"),
            to: smallvec::smallvec![address(spirit_id)],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            auto_marker: FrameOrigin::HumanAuthored,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "exercise the real T2 echo component".into(),
                scope: Vec::new(),
                success_criteria: "complete one echo turn".into(),
                posture_preferences: Default::default(),
                prior_distillate_ref: None,
            }),
            consent_envelope: None,
            intent_lineage: Default::default(),
        }),
    }
}

fn lifecycle_entries(path: &Path) -> Vec<maos_domain::invariants::i10::LifecycleEntry> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str::<maos_domain::invariants::i10::JournalEntry>(line).unwrap()
        })
        .filter_map(|entry| match entry {
            maos_domain::invariants::i10::JournalEntry::Lifecycle(entry) => Some(entry),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CgroupTestMode {
    Delegated,
    Undelegated,
}

fn cgroup_test_mode() -> CgroupTestMode {
    match std::env::var("MAOS_17_2_CGROUP_MODE")
        .unwrap_or_else(|error| panic!("MAOS_17_2_CGROUP_MODE is required: {error}"))
        .as_str()
    {
        "delegated" => CgroupTestMode::Delegated,
        "undelegated" => CgroupTestMode::Undelegated,
        other => panic!("MAOS_17_2_CGROUP_MODE must be delegated or undelegated, got {other:?}"),
    }
}

fn process_is_live(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| stat.rsplit_once(')').map(|(_, fields)| fields.to_owned()))
        .and_then(|fields| fields.split_whitespace().next().map(str::to_owned))
        .is_some_and(|state| state != "Z" && state != "X")
}

fn proc_soft_limit(pid: u32, label: &str) -> String {
    let limits = std::fs::read_to_string(format!("/proc/{pid}/limits"))
        .expect("read the live T2 runner's actual limits");
    limits
        .lines()
        .find(|line| line.starts_with(label))
        .unwrap_or_else(|| panic!("missing {label} in child limits"))
        .split_whitespace()
        .nth(3)
        .unwrap_or_else(|| panic!("missing soft limit for {label}"))
        .to_owned()
}

fn child_cgroup_leaf(pid: u32) -> PathBuf {
    let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .expect("read the live T2 runner's actual cgroup membership");
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("the T2 runner has unified cgroup membership");
    let leaf = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    assert!(leaf.is_dir(), "runner cgroup leaf is absent: {leaf:?}");
    let members = std::fs::read_to_string(leaf.join("cgroup.procs"))
        .expect("read actual cgroup leaf membership");
    assert!(
        members.lines().any(|member| member == pid.to_string()),
        "runner {pid} is not present in its reported cgroup leaf"
    );
    leaf
}

fn t2_turn_complete(world: &World, spirit_pid: u32) -> bool {
    let has_guest_frame = world
        .tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            spirit_pid: Some(spirit_pid),
            ..Default::default()
        })
        .unwrap()
        .iter()
        .any(|row| row.intent == "spirit.frame.authorization");
    if !has_guest_frame {
        return false;
    }
    let scbs = world.scheduler.scbs();
    let scbs = scbs.read().unwrap_or_else(|error| error.into_inner());
    let Some(scb) = scbs.get(&spirit_pid) else {
        return false;
    };
    let empty = match scb.task_assignments_in_flight.lock() {
        Ok(ledger) => ledger.is_empty(),
        Err(error) => error.into_inner().is_empty(),
    };
    empty
}

async fn wait_for_live_t2_turn(
    world: &World,
    spirit_pid: u32,
    turn_budget: Duration,
) -> Result<u32, String> {
    // Match the existing hostile-session envelope: startup, seed delivery and
    // turn execution retain their declared budgets even under a CPU ceiling.
    let deadline = std::time::Instant::now() + turn_budget * 4 + Duration::from_secs(20);
    loop {
        if let Some(binding) = world
            .supervisor
            .bindings()
            .into_iter()
            .find(|binding| binding.spirit_pid == spirit_pid)
        {
            if let Some(child_pid) = binding.child_pid {
                if binding.phase == maos_bin::supervision::BindingPhase::Running
                    && binding.child.is_some()
                    && process_is_live(child_pid)
                    && t2_turn_complete(world, spirit_pid)
                {
                    return Ok(child_pid);
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "T2 runner {spirit_pid} did not remain live through Ready and one completed turn"
            ));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
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
                        recall: maos_domain::frame::ConsentRecall::Full,
                    }),
                    intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![
                        maos_domain::invariants::i8::A2AIntent::new(
                            "scope-verdict:delegate-search",
                        ),
                    ]),
                };
                let artifact = FIXTURES.echo.clone();
                let plan = maos_host::SpiritLaunchPlan {
                    program: if wasm {
                        &FIXTURES.runner
                    } else {
                        &FIXTURES.native_twin
                    }
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
                        context_window: Default::default(),
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
                    unload_all_loaded(world.scheduler.as_ref(), world.halt.as_ref(), None).await;
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
    run_hostile_twin_with_quota(mode, turn_budget, sends, false).await
}

async fn run_hostile_twin_with_quota(
    mode: &str,
    turn_budget: Duration,
    sends: Option<&[&str]>,
    exhaust_issuance: bool,
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
    // Reserve all remaining slots after admission, before the first guest turn.
    // Keep the charges alive until this session has finished.
    let _charges = exhaust_issuance.then(|| world.exhaust_issuance(pid));
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
        program: FIXTURES.hostile_twin.to_string_lossy().into_owned(),
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
                context_window: Default::default(),
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
    std::process::Command::new(fixtures::profile_binary("maosctl"))
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
    // A granted send still needs capacity in this admitted pid's issuance window.
    let exhausted = run_hostile_twin_with_quota(
        "provenance",
        Duration::from_secs(30),
        Some(&["spirit:peer"]),
        true,
    )
    .await;
    assert_eq!(
        exhausted.report().turns,
        1,
        "a refused send completes its turn"
    );
    for (run, verdict, refusal) in [
        (&ungranted, Some("deny"), None),
        (&broadcast_only, None, Some("iac_peer_class_not_granted")),
        (&exhausted, Some("exhausted"), None),
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
    println!("HOSTILE-IAC-SEND ungranted=deny broadcast_only=iac_peer_class_not_granted exhausted=exhausted");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wasm_delivery_is_refused_when_its_issuance_window_is_exhausted() {
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let id = "scope-wasm";
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
    let pid = admitted.pid;
    let _charges = world.exhaust_issuance(pid);
    let mut receiver = world
        .iac
        .register_spirit_typed(&SpiritId::from("scope-receiver"))
        .unwrap();
    let mut launch = echo_launch(id);
    let seed = launch.initial_frame.as_mut().unwrap();
    seed.to.push(FrameAddress {
        spirit_id: SpiritId::from("scope-receiver"),
        host_id: None,
        role: None,
    });
    let seed_id = seed.frame_id;
    let report = run_admitted_session(world.session(), admitted, core, launch)
        .await
        .expect("the exhausted send does not terminate the WASM session");
    assert_eq!(report.turns, 1, "a refused send completes its turn");
    assert_eq!((report.delivered_frames, report.denied_frames), (0, 1));
    let authorizations = world
        .tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            spirit_pid: Some(pid),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .filter(|row| row.intent == "spirit.frame.authorization")
        .map(|row| serde_json::from_slice::<serde_json::Value>(&row.payload_redacted).unwrap())
        .collect::<Vec<_>>();
    let [authorization] = authorizations.as_slice() else {
        panic!("one WASM send authorization: {authorizations:?}")
    };
    assert_eq!(authorization["allowed"], false);
    assert_eq!(authorization["send"]["kernel_verdict"], "exhausted");
    let (_, seed) = receiver
        .try_recv()
        .unwrap()
        .expect("the operator seed arrives");
    assert_eq!(seed.frame_id, seed_id);
    assert_eq!(seed.from.spirit_id.as_str(), "operator");
    assert!(
        receiver.try_recv().unwrap().is_none(),
        "only the operator seed arrives"
    );
    assert!(
        world.scheduler.resolve_pid(id).is_none(),
        "the completed session unloads its SCB"
    );
    drop(receiver);
    world.drain().await;
    let audit = operator_audit_view(dir.path(), id);
    assert!(
        audit.status.success(),
        "a refused WASM send leaves the FR4 view green: {}{}",
        String::from_utf8_lossy(&audit.stdout),
        String::from_utf8_lossy(&audit.stderr)
    );
    println!("WASM-IAC-SEND exhausted=exhausted denied_frames=1 delivered_frames=0 turns=1 seed_only=true fr4_view=ok");
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
        program: FIXTURES.runner.to_string_lossy().into_owned(),
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
            context_window: Default::default(),
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
    let fault_runner = fixtures::fixture("maos-wasm-runner-fault-inject");
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
        let artifact = FIXTURES.echo.clone();
        let mut argv = vec![
            "--component".into(),
            artifact.to_string_lossy().into_owned(),
        ];
        if forbidden {
            argv.push("--test-forbidden-syscall-after-ready".into());
        }
        let plan = maos_host::SpiritLaunchPlan {
            program: fault_runner.to_string_lossy().into_owned(),
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
                context_window: Default::default(),
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
        let anomalies = maos_escape_detector::Detector::detect(
            &dir.path().join("transparency.sqlite"),
            &[declaration],
        )
        .unwrap()
        .len();
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
    let release = FIXTURES.runner.clone();
    let artifact = FIXTURES.echo.clone();
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
    let artifact = FIXTURES.relay.clone();
    let plan = maos_host::SpiritLaunchPlan {
        program: FIXTURES.runner.to_string_lossy().into_owned(),
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
            context_window: Default::default(),
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
            &maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgeExit {
                cause: maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause::Exited {
                    code: 0,
                },
                child_pid: 0,
                container: None,
            },
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
            let artifact = FIXTURES.echo.clone();
            let plan = maos_host::SpiritLaunchPlan {
                program: if wasm {
                    &FIXTURES.runner
                } else {
                    &FIXTURES.native_twin
                }
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
                    context_window: Default::default(),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resource_downgrade_memory_only_is_persisted_for_each_fast_t2_restart() {
    use maos_domain::invariants::i10::{LifecycleEvent, ResourceCeilingDowngradedPayload};
    use maos_domain::invariants::i9::SandboxTier;

    let mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let spirit_id = "memory-only-restart";
    for expected_rows in 1..=2 {
        let (core, admitted) = admit_t2(
            &world,
            spirit_id,
            "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
        )
        .await;
        let spirit_pid = admitted.pid;
        assert_eq!(admitted.sandbox.tier, SandboxTier::T2);
        assert_eq!(admitted.sandbox.resolved_caps.cpu_max_pct, None);
        assert_eq!(admitted.sandbox.resolved_caps.memory_max_mb, Some(64));
        assert_eq!(admitted.sandbox.resolved_caps.pids_max, None);
        assert_eq!(admitted.sandbox.resolved_caps.fd_max, Some(64));
        assert_eq!(
            world.journal.last_event(spirit_id),
            Some(LifecycleEvent::Load),
            "each admission's Load remains the raw latest event until its fallback is recorded"
        );

        let report = run_admitted_session(world.session(), admitted, core, echo_launch(spirit_id))
            .await
            .expect("the actual release runner completes an admitted T2 echo turn");
        assert_eq!(report.turns, 1);
        assert_ne!(report.child_pid, std::process::id());
        let binding = world
            .supervisor
            .bindings()
            .into_iter()
            .find(|binding| binding.spirit_pid == spirit_pid)
            .expect("the production T2 session binding remains observable");
        assert_eq!(binding.child_pid, Some(report.child_pid));
        assert_eq!(
            binding.phase,
            maos_bin::supervision::BindingPhase::ExitedClean,
            "the short-lived runner reached its terminal phase"
        );
        assert!(
            world
                .tl
                .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                    spirit_pid: Some(spirit_pid),
                    ..Default::default()
                })
                .unwrap()
                .iter()
                .any(|row| row.intent == "cli.subprocess.exit"),
            "the actual child exit was finalized"
        );

        let entries = lifecycle_entries(&dir.path().join("lifecycle.ndjson"));
        let rows = entries
            .iter()
            .filter(|entry| {
                entry.spirit_id == spirit_id
                    && entry.lifecycle_event == LifecycleEvent::ResourceCeilingDowngraded
            })
            .collect::<Vec<_>>();
        let expected_count = if mode == CgroupTestMode::Undelegated {
            expected_rows
        } else {
            0
        };
        assert_eq!(
            rows.len(),
            expected_count,
            "successful fallback is recorded once per spawn; a delegated cgroup is not a downgrade"
        );
        match mode {
            CgroupTestMode::Undelegated => {
                let row = rows.last().unwrap();
                assert_eq!(row.effective_sandbox_tier, Some(SandboxTier::T2));
                let payload: ResourceCeilingDowngradedPayload =
                    serde_json::from_slice(row.payload.as_deref().unwrap()).unwrap();
                assert!(
                    payload.missing_mechanism.contains("cgroup v2 memory.max"),
                    "the row names the unavailable native mechanism: {}",
                    payload.missing_mechanism
                );
                assert_eq!(payload.memory_max_mb, Some(64));
                assert_eq!(payload.fd_max, Some(64));
                assert!(payload.rlimit_as);
                assert!(payload.rlimit_nofile);
                assert_eq!(
                    world.journal.last_event(spirit_id),
                    Some(LifecycleEvent::ResourceCeilingDowngraded),
                    "last_event continues to expose the raw latest lifecycle event"
                );
            }
            CgroupTestMode::Delegated => {
                assert_eq!(
                    world.journal.last_event(spirit_id),
                    Some(LifecycleEvent::Load)
                );
            }
        }
    }

    let journal_path = dir.path().join("lifecycle.ndjson");
    drop(world.drain().await);
    let sequence = lifecycle_entries(&journal_path);
    let persisted_last = sequence
        .iter()
        .rev()
        .find(|entry| entry.spirit_id == spirit_id)
        .map(|entry| entry.lifecycle_event);
    let reopened = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    assert_eq!(reopened.last_event(spirit_id), persisted_last);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn memory_only_t2_live_child_has_exact_rlimits_and_mode_truthful_journal() {
    use maos_domain::invariants::i10::{LifecycleEvent, ResourceCeilingDowngradedPayload};
    use maos_domain::invariants::i9::SandboxTier;

    let mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let journal_path = dir.path().join("lifecycle.ndjson");
    let world = World::new(dir.path());
    let spirit_id = "live-memory-only-t2";
    let (core, admitted) = admit_t2(
        &world,
        spirit_id,
        "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
    )
    .await;
    let spirit_pid = admitted.pid;
    assert_eq!(admitted.sandbox.tier, SandboxTier::T2);
    assert_eq!(admitted.sandbox.resolved_caps.memory_max_mb, Some(64));
    assert_eq!(admitted.sandbox.resolved_caps.fd_max, Some(64));
    let mut launch = echo_launch(spirit_id);
    launch.once = false;
    let turn_budget = launch.turn_budget;
    let session = tokio::spawn(run_admitted_session(
        world.session(),
        admitted,
        core,
        launch,
    ));
    let child_pid = wait_for_live_t2_turn(&world, spirit_pid, turn_budget)
        .await
        .expect("real T2 release runner reaches Ready and completes a turn while staying live");
    assert_ne!(child_pid, std::process::id());
    assert_eq!(proc_soft_limit(child_pid, "Max address space"), "67108864");
    assert_eq!(proc_soft_limit(child_pid, "Max open files"), "64");
    let binding = world
        .supervisor
        .bindings()
        .into_iter()
        .find(|binding| binding.spirit_pid == spirit_pid)
        .expect("the actual admitted Spirit has a supervisor binding");
    assert_eq!(binding.child_pid, Some(child_pid));
    let entries = lifecycle_entries(&journal_path);
    let downgrade_rows = entries
        .iter()
        .filter(|entry| {
            entry.spirit_id == spirit_id
                && entry.lifecycle_event == LifecycleEvent::ResourceCeilingDowngraded
        })
        .collect::<Vec<_>>();
    match mode {
        CgroupTestMode::Undelegated => {
            assert_eq!(downgrade_rows.len(), 1);
            let row = downgrade_rows[0];
            assert_eq!(row.effective_sandbox_tier, Some(SandboxTier::T2));
            let payload: ResourceCeilingDowngradedPayload =
                serde_json::from_slice(row.payload.as_deref().unwrap()).unwrap();
            assert!(payload.missing_mechanism.contains("cgroup v2 memory.max"));
            assert_eq!(payload.memory_max_mb, Some(64));
            assert_eq!(payload.fd_max, Some(64));
            assert!(payload.rlimit_as);
            assert!(payload.rlimit_nofile);
            assert_eq!(
                world.journal.last_event(spirit_id),
                Some(LifecycleEvent::ResourceCeilingDowngraded)
            );
        }
        CgroupTestMode::Delegated => {
            assert!(downgrade_rows.is_empty());
            let leaf = child_cgroup_leaf(child_pid);
            assert_eq!(
                std::fs::read_to_string(leaf.join("memory.max"))
                    .expect("read the actual delegated memory ceiling")
                    .trim(),
                "67108864"
            );
            assert_eq!(
                world.journal.last_event(spirit_id),
                Some(LifecycleEvent::Load)
            );
        }
    }
    world.supervisor.stop_workers();
    let result = tokio::time::timeout(Duration::from_secs(10), session)
        .await
        .expect("live memory-only T2 session stops within its cleanup bound")
        .unwrap();
    assert_eq!(
        world
            .supervisor
            .bindings()
            .into_iter()
            .find(|binding| binding.spirit_pid == spirit_pid)
            .unwrap()
            .phase,
        maos_bin::supervision::BindingPhase::PlannedStop,
        "planned teardown must not become a crash; terminal result: {result:?}"
    );
    assert!(
        !Path::new(&format!("/proc/{child_pid}/stat")).exists(),
        "the stopped live runner was reaped"
    );
    world.drain().await;
    let persisted = lifecycle_entries(&journal_path)
        .into_iter()
        .filter(|entry| entry.spirit_id == spirit_id)
        .map(|entry| entry.lifecycle_event)
        .collect::<Vec<_>>();
    let expected_last = if mode == CgroupTestMode::Undelegated {
        LifecycleEvent::ResourceCeilingDowngraded
    } else {
        LifecycleEvent::Load
    };
    assert_eq!(persisted.last(), Some(&expected_last));
    let reopened = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    assert_eq!(reopened.last_event(spirit_id), Some(expected_last));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resource_downgrade_no_resource_or_fd_only_t2_spawn_emits_no_row() {
    use maos_domain::invariants::i10::LifecycleEvent;

    let _mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    for (spirit_id, resources, memory_max_mb, fd_max) in [
        ("no-resource-cap", "[resources]\n", None, None),
        ("fd-only-cap", "[resources]\nfd_max = 64\n", None, Some(64)),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let world = World::new(dir.path());
        let (core, admitted) = admit_t2(&world, spirit_id, resources).await;
        assert_eq!(admitted.sandbox.resolved_caps.memory_max_mb, memory_max_mb);
        assert_eq!(admitted.sandbox.resolved_caps.fd_max, fd_max);
        assert_eq!(
            run_admitted_session(world.session(), admitted, core, echo_launch(spirit_id))
                .await
                .expect("the actual T2 runner completes without a cgroup memory fallback")
                .turns,
            1
        );
        let entries = lifecycle_entries(&dir.path().join("lifecycle.ndjson"));
        assert!(
            entries.iter().all(|entry| {
                entry.spirit_id != spirit_id
                    || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
            }),
            "no-resource and FD-only requests have no memory-ceiling downgrade row"
        );
        assert_eq!(
            world.journal.last_event(spirit_id),
            Some(LifecycleEvent::Load),
            "the existing last-event index retains Load when no downgrade was recorded"
        );
        world.drain().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resource_downgrade_strict_cpu_pid_and_failed_t2_spawns_emit_no_row() {
    use maos_domain::invariants::i10::LifecycleEvent;

    let mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    for (spirit_id, resources) in [
        ("strict-cpu-cap", "[resources]\ncpu_max_pct = 10\n"),
        ("strict-pid-cap", "[resources]\npids_max = 16\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let world = World::new(dir.path());
        let (core, admitted) = admit_t2(&world, spirit_id, resources).await;
        let spirit_pid = admitted.pid;
        let result =
            run_admitted_session(world.session(), admitted, core, echo_launch(spirit_id)).await;
        let child_pid = match (mode, result) {
            (
                CgroupTestMode::Undelegated,
                Err(maos_bin::spirit_session::SessionError::Bridge(error)),
            ) => {
                assert!(
                    error.to_ascii_lowercase().contains("cgroup"),
                    "strict-cap refusal identifies the missing cgroup mechanism: {error}"
                );
                None
            }
            (CgroupTestMode::Undelegated, other) => {
                panic!("undelegated strict CPU/PID request must refuse before guest execution: {other:?}")
            }
            (CgroupTestMode::Delegated, Ok(report)) => {
                assert_eq!(report.turns, 1);
                Some(report.child_pid)
            }
            (CgroupTestMode::Delegated, Err(other)) => {
                panic!("delegated strict CPU/PID request should run under its cgroup: {other}")
            }
        };
        let binding = world
            .supervisor
            .bindings()
            .into_iter()
            .find(|binding| binding.spirit_pid == spirit_pid)
            .expect("strict-cap session binding remains observable");
        assert_eq!(binding.child_pid, child_pid);
        if mode == CgroupTestMode::Undelegated {
            assert_eq!(
                binding.phase,
                maos_bin::supervision::BindingPhase::PlannedStop
            );
            assert!(
                world
                    .tl
                    .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                        spirit_pid: Some(spirit_pid),
                        ..Default::default()
                    })
                    .unwrap()
                    .iter()
                    .all(|row| row.intent != "spirit.frame.authorization"
                        && row.intent != "cli.subprocess.exit"),
                "a strict-cap refusal has no guest output or child exit row"
            );
        }
        let entries = lifecycle_entries(&dir.path().join("lifecycle.ndjson"));
        assert!(entries.iter().all(|entry| {
            entry.spirit_id != spirit_id
                || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
        }));
        assert_eq!(
            world.journal.last_event(spirit_id),
            Some(LifecycleEvent::Load)
        );
        world.drain().await;
    }

    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let spirit_id = "failed-memory-spawn";
    let (core, admitted) = admit_t2(
        &world,
        spirit_id,
        "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
    )
    .await;
    let mut launch = echo_launch(spirit_id);
    launch.plan.program = dir
        .path()
        .join("does-not-exist/maos-wasm-runner")
        .to_string_lossy()
        .into_owned();
    assert!(matches!(
        run_admitted_session(world.session(), admitted, core, launch).await,
        Err(maos_bin::spirit_session::SessionError::Bridge(_))
    ));
    let entries = lifecycle_entries(&dir.path().join("lifecycle.ndjson"));
    assert!(entries.iter().all(|entry| {
        entry.spirit_id != spirit_id
            || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
    }));
    assert_eq!(
        world.journal.last_event(spirit_id),
        Some(LifecycleEvent::Load)
    );
    world.drain().await;
}

async fn run_append_failure_child() {
    use maos_domain::invariants::i10::LifecycleEvent;

    assert_eq!(cgroup_test_mode(), CgroupTestMode::Undelegated);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let spirit_id = "downgrade-append-failure";
    let (core, admitted) = admit_t2(
        &world,
        spirit_id,
        "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
    )
    .await;
    let spirit_pid = admitted.pid;
    let journal_path = dir.path().join("lifecycle.ndjson");
    // A process-local cap also covers SQLite. Pad only the lifecycle journal
    // with valid empty lines so its append fails without starving live cleanup.
    {
        use std::io::Write;
        let mut padding_file = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal_path)
            .unwrap();
        let padding = [b'\n'; 4096];
        let mut remaining =
            (1024 * 1024_u64).saturating_sub(padding_file.metadata().unwrap().len());
        while remaining != 0 {
            let amount = remaining.min(padding.len() as u64) as usize;
            padding_file.write_all(&padding[..amount]).unwrap();
            remaining -= amount as u64;
        }
    }
    let journal_size = std::fs::metadata(&journal_path)
        .expect("stat the open regular lifecycle journal")
        .len();
    assert!(journal_path.is_file());
    assert!(
        journal_size > 0,
        "admission wrote its real Load row before the cap"
    );

    let fsize = rustix::process::getrlimit(rustix::process::Resource::Fsize);
    rustix::process::setrlimit(
        rustix::process::Resource::Fsize,
        rustix::process::Rlimit {
            current: Some(journal_size),
            maximum: fsize.maximum,
        },
    )
    .expect("lower only this isolated child's soft RLIMIT_FSIZE to the journal EOF");
    let result =
        run_admitted_session(world.session(), admitted, core, echo_launch(spirit_id)).await;
    rustix::process::setrlimit(rustix::process::Resource::Fsize, fsize)
        .expect("restore the isolated process's original RLIMIT_FSIZE after JoinError");
    match result {
        Err(maos_bin::spirit_session::SessionError::Supervision(error)) => {
            assert!(
                error.contains("Journal append failed"),
                "the Lifecycle Journal's fail-loud append panic propagates: {error}"
            );
        }
        other => panic!("append failure must propagate as the blocking session panic: {other:?}"),
    }
    assert_eq!(
        world.supervisor.strategy_name(),
        "pidfd",
        "this Linux test uses the real pidfd observer"
    );
    assert_eq!(
        world.supervisor.counters().observer_unavailable(),
        0,
        "watch installed a live observer before the append panic"
    );
    let binding = world
        .supervisor
        .bindings()
        .into_iter()
        .find(|binding| binding.spirit_pid == spirit_pid)
        .expect("the failed session's live owner was registered");
    let child_pid = binding
        .child_pid
        .expect("watch recorded the actual child pid before journaling");
    assert_ne!(child_pid, std::process::id());
    assert_ne!(binding.phase, maos_bin::supervision::BindingPhase::Running);
    assert!(
        !Path::new(&format!("/proc/{child_pid}/stat")).exists(),
        "LiveWorker panic cleanup stopped and reaped its real child"
    );
    assert_eq!(
        world.journal.last_event(spirit_id),
        Some(LifecycleEvent::Load),
        "a failed append leaves the raw latest event at Load"
    );
    let entries = lifecycle_entries(&journal_path);
    assert!(entries.iter().all(|entry| {
        entry.spirit_id != spirit_id
            || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
    }));
    drop(world.drain().await);
    let reopened = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    assert_eq!(reopened.last_event(spirit_id), Some(LifecycleEvent::Load));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resource_downgrade_append_panic_follows_live_child_observer_installation() {
    use maos_domain::invariants::i10::LifecycleEvent;

    let mode = cgroup_test_mode();
    if std::env::var_os("MAOS_17_2_APPEND_FAILURE_CHILD").is_some() {
        run_append_failure_child().await;
    } else {
        // The parent builds real fixtures before exec; the isolated fault process
        // must not contend with another test's Cargo build lock before its deadline.
        LazyLock::force(&FIXTURES);
        match mode {
            CgroupTestMode::Undelegated => {
                let executable =
                    std::env::current_exe().expect("locate this integration-test binary");
                let output = std::process::Command::new("timeout")
                    .args(["--signal=KILL", "60", "/bin/bash"])
                    .args([
                        "-c",
                        "trap '' XFSZ; exec \"$@\"",
                        "maos-17-2-append-failure",
                    ])
                    .arg(executable)
                    .args([
                        "--exact",
                        "resource_downgrade_append_panic_follows_live_child_observer_installation",
                        "--nocapture",
                    ])
                    .env("MAOS_17_2_CGROUP_MODE", "undelegated")
                    .env("MAOS_17_2_APPEND_FAILURE_CHILD", "1")
                    .output()
                    .expect("start isolated RLIMIT_FSIZE append-failure test process");
                let status = output.status;
                assert!(
                    output.status.success(),
                    "isolated append-failure process failed ({status:?}):\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            CgroupTestMode::Delegated => {
                let dir = tempfile::tempdir().unwrap();
                let world = World::new(dir.path());
                let spirit_id = "downgrade-append-failure";
                let (core, admitted) = admit_t2(
                    &world,
                    spirit_id,
                    "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
                )
                .await;
                let spirit_pid = admitted.pid;
                let mut launch = echo_launch(spirit_id);
                launch.once = false;
                let turn_budget = launch.turn_budget;
                let session = tokio::spawn(run_admitted_session(
                    world.session(),
                    admitted,
                    core,
                    launch,
                ));
                let child_pid = wait_for_live_t2_turn(&world, spirit_pid, turn_budget)
                    .await
                    .expect("delegated session completes a turn with its child still live");
                let leaf = child_cgroup_leaf(child_pid);
                assert_eq!(
                    std::fs::read_to_string(leaf.join("memory.max"))
                        .expect("read actual delegated memory.max")
                        .trim(),
                    "67108864"
                );
                let entries = lifecycle_entries(&dir.path().join("lifecycle.ndjson"));
                assert!(entries.iter().all(|entry| {
                    entry.spirit_id != spirit_id
                        || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
                }));
                assert_eq!(
                    world.journal.last_event(spirit_id),
                    Some(LifecycleEvent::Load)
                );
                world.supervisor.stop_workers();
                let result = tokio::time::timeout(Duration::from_secs(10), session)
                    .await
                    .expect("delegated child cleanup completes")
                    .unwrap();
                assert_eq!(
                    world
                        .supervisor
                        .bindings()
                        .into_iter()
                        .find(|binding| binding.spirit_pid == spirit_pid)
                        .unwrap()
                        .phase,
                    maos_bin::supervision::BindingPhase::PlannedStop,
                    "planned delegated teardown; terminal result: {result:?}"
                );
                assert!(!leaf.exists());
                world.drain().await;
            }
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn admitted_t2_echo_enforces_exact_live_caps_in_delegated_mode() {
    use maos_domain::invariants::i10::LifecycleEvent;

    let mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let spirit_id = "delegated-live-t2-caps";
    let resources =
        "[resources]\ncpu_max_pct = 10\nmemory_max_mb = 64\npids_max = 12\nfd_max = 64\n";
    let (core, admitted) = admit_t2(&world, spirit_id, resources).await;
    let spirit_pid = admitted.pid;
    assert_eq!(admitted.sandbox.resolved_caps.cpu_max_pct, Some(10));
    assert_eq!(admitted.sandbox.resolved_caps.memory_max_mb, Some(64));
    assert_eq!(admitted.sandbox.resolved_caps.pids_max, Some(12));
    assert_eq!(admitted.sandbox.resolved_caps.fd_max, Some(64));

    match mode {
        CgroupTestMode::Undelegated => {
            let result =
                run_admitted_session(world.session(), admitted, core, echo_launch(spirit_id)).await;
            match result {
                Err(maos_bin::spirit_session::SessionError::Bridge(error)) => assert!(
                    error.to_ascii_lowercase().contains("cgroup"),
                    "strict CPU/PID native refusal identifies cgroup setup: {error}"
                ),
                other => {
                    panic!("undelegated CPU/PID caps must refuse before guest execution: {other:?}")
                }
            }
            let binding = world
                .supervisor
                .bindings()
                .into_iter()
                .find(|binding| binding.spirit_pid == spirit_pid)
                .expect("refused T2 binding remains observable");
            assert_eq!(binding.child_pid, None);
            assert!(lifecycle_entries(&dir.path().join("lifecycle.ndjson"))
                .iter()
                .all(|entry| {
                    entry.spirit_id != spirit_id
                        || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
                }));
            assert_eq!(
                world.journal.last_event(spirit_id),
                Some(LifecycleEvent::Load)
            );
            world.drain().await;
        }
        CgroupTestMode::Delegated => {
            let mut launch = echo_launch(spirit_id);
            launch.once = false;
            let turn_budget = launch.turn_budget;
            let session = tokio::spawn(run_admitted_session(
                world.session(),
                admitted,
                core,
                launch,
            ));
            let child_pid = match wait_for_live_t2_turn(&world, spirit_pid, turn_budget).await {
                Ok(child_pid) => child_pid,
                Err(error) => {
                    world.supervisor.stop_workers();
                    let terminal = tokio::time::timeout(Duration::from_secs(10), session).await;
                    panic!("{error}; bounded session cleanup result: {terminal:?}");
                }
            };
            assert_ne!(child_pid, std::process::id());
            assert_eq!(proc_soft_limit(child_pid, "Max address space"), "67108864");
            assert_eq!(proc_soft_limit(child_pid, "Max open files"), "64");
            let leaf = child_cgroup_leaf(child_pid);
            assert_eq!(
                std::fs::read_to_string(leaf.join("cpu.max"))
                    .expect("read exact delegated T2 CPU ceiling")
                    .trim(),
                "10000 100000"
            );
            assert_eq!(
                std::fs::read_to_string(leaf.join("memory.max"))
                    .expect("read exact delegated T2 memory ceiling")
                    .trim(),
                "67108864"
            );
            assert_eq!(
                std::fs::read_to_string(leaf.join("pids.max"))
                    .expect("read exact delegated T2 PID ceiling")
                    .trim(),
                "12"
            );
            assert!(
                lifecycle_entries(&dir.path().join("lifecycle.ndjson"))
                    .iter()
                    .all(|entry| {
                        entry.spirit_id != spirit_id
                            || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
                    }),
                "a real delegated memory.max has no downgrade row before the completed turn"
            );
            world.supervisor.stop_workers();
            let result = tokio::time::timeout(Duration::from_secs(10), session)
                .await
                .expect("live delegated session stops within its cleanup bound")
                .unwrap();
            assert_eq!(
                world
                    .supervisor
                    .bindings()
                    .into_iter()
                    .find(|binding| binding.spirit_pid == spirit_pid)
                    .unwrap()
                    .phase,
                maos_bin::supervision::BindingPhase::PlannedStop,
                "planned limited-runner teardown; terminal result: {result:?}"
            );
            assert!(
                !leaf.exists(),
                "session teardown removes its owned cgroup leaf"
            );
            world.drain().await;
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fast_t2_crash_then_restart_orders_one_downgrade_after_crash() {
    use maos_domain::invariants::i10::LifecycleEvent;

    let mode = cgroup_test_mode();
    LazyLock::force(&FIXTURES);
    let dir = tempfile::tempdir().unwrap();
    let journal_path = dir.path().join("lifecycle.ndjson");
    let (watched_tx, watched_rx) = std::sync::mpsc::channel();
    let (after_watch_release_tx, after_watch_release_rx) = std::sync::mpsc::channel();
    let first_watch = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let callback_release = std::sync::Mutex::new(after_watch_release_rx);
    let callback_first_watch = Arc::clone(&first_watch);
    let seams = maos_bin::supervision::SupervisorSeams {
        after_watch: Some(Arc::new(move |child_pid| {
            if callback_first_watch.swap(false, std::sync::atomic::Ordering::SeqCst) {
                watched_tx
                    .send(child_pid)
                    .expect("publish the actual watched child PID to the test");
                callback_release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(20))
                    .expect("release the first watcher within the bounded ordering window");
            }
        })),
        ..Default::default()
    };
    let world = World::new_with_seams(dir.path(), seams);
    // Dropping this sender during unwinding releases the callback before World teardown.
    let after_watch_release = after_watch_release_tx;
    let spirit_id = "fast-crash-downgrade-restart";
    let (first_core, first_admitted) = admit_t2(
        &world,
        spirit_id,
        "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
    )
    .await;
    let old_spirit_pid = first_admitted.pid;
    let mut first_launch = echo_launch(spirit_id);
    first_launch.once = false;
    let first_session = tokio::spawn(run_admitted_session(
        world.session(),
        first_admitted,
        first_core,
        first_launch,
    ));
    let old_child_pid = watched_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the actual observer is installed before the T2 owner continues");
    let old_binding = world
        .supervisor
        .bindings()
        .into_iter()
        .find(|binding| binding.spirit_pid == old_spirit_pid)
        .expect("old admitted T2 has a supervisor binding");
    assert_eq!(old_binding.child_pid, Some(old_child_pid));
    let old_child = old_binding
        .child
        .as_ref()
        .expect("observer has installed the real child")
        .clone();
    rustix::process::pidfd_send_signal(old_child.pidfd(), rustix::process::Signal::KILL)
        .expect("explicitly crash the real runner before startup publication");
    let crash_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let crashed = lifecycle_entries(&journal_path).iter().any(|entry| {
            entry.spirit_id == spirit_id && entry.lifecycle_event == LifecycleEvent::Crash
        });
        let scb_removed = {
            let scbs = world.scheduler.scbs();
            let removed = !scbs
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .contains_key(&old_spirit_pid);
            removed
        };
        if crashed && scb_removed {
            break;
        }
        assert!(
            std::time::Instant::now() < crash_deadline,
            "real observer must persist Crash and unload/remove the SCB before T2 continues"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!process_is_live(old_child_pid));
    assert!(
        lifecycle_entries(&journal_path).iter().all(|entry| {
            entry.spirit_id != spirit_id
                || entry.lifecycle_event != LifecycleEvent::ResourceCeilingDowngraded
        }),
        "producer remains behind the after-watch gate until crash handling completes"
    );
    after_watch_release
        .send(())
        .expect("release the first registered watcher after real crash handling");
    let first_result = tokio::time::timeout(Duration::from_secs(10), first_session)
        .await
        .expect("crashed fast runner is observed and finalized")
        .unwrap();
    assert!(matches!(
        first_result,
        Err(maos_bin::spirit_session::SessionError::Child(_))
    ));
    assert!(!process_is_live(old_child_pid));
    assert!(
        !world
            .scheduler
            .scbs()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(&old_spirit_pid),
        "crash handling unloads and removes the old SCB"
    );

    let (second_core, second_admitted) = admit_t2(
        &world,
        spirit_id,
        "[resources]\nmemory_max_mb = 64\nfd_max = 64\n",
    )
    .await;
    assert_ne!(second_admitted.pid, old_spirit_pid);
    let second_spirit_pid = second_admitted.pid;
    let mut second_launch = echo_launch(spirit_id);
    second_launch.once = true;
    if let Some(frame) = second_launch.initial_frame.as_mut() {
        frame.frame_id = [0x18; 16];
    }
    let report = run_admitted_session(world.session(), second_admitted, second_core, second_launch)
        .await
        .expect("same Spirit can restart after fast crash and perform a real turn");
    assert_eq!(report.turns, 1);
    let new_binding = world
        .supervisor
        .bindings()
        .into_iter()
        .find(|binding| binding.spirit_pid == second_spirit_pid)
        .expect("restarted T2 has its own supervisor binding");
    assert_eq!(new_binding.child_pid, Some(report.child_pid));
    assert_ne!(report.child_pid, std::process::id());
    assert!(
        !world
            .scheduler
            .scbs()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(&second_spirit_pid),
        "once-session unloads its restarted SCB after the completed turn"
    );

    let entries = lifecycle_entries(&journal_path);
    let sequence = entries
        .iter()
        .filter(|entry| entry.spirit_id == spirit_id)
        .map(|entry| entry.lifecycle_event)
        .collect::<Vec<_>>();
    assert_eq!(
        sequence
            .iter()
            .filter(|event| **event == LifecycleEvent::Load)
            .count(),
        2
    );
    assert_eq!(
        sequence
            .iter()
            .filter(|event| **event == LifecycleEvent::Crash)
            .count(),
        1,
        "the old fast child has exactly one crash lifecycle event"
    );
    let crash_index = sequence
        .iter()
        .position(|event| *event == LifecycleEvent::Crash)
        .expect("the first child's explicit crash is persisted");
    assert_eq!(sequence[crash_index - 1], LifecycleEvent::Load);
    match mode {
        CgroupTestMode::Undelegated => {
            assert_eq!(
                sequence
                    .iter()
                    .filter(|event| **event == LifecycleEvent::ResourceCeilingDowngraded)
                    .count(),
                2,
                "each undelegated admitted session emits its own one-shot row"
            );
            assert_eq!(
                sequence[crash_index + 1],
                LifecycleEvent::ResourceCeilingDowngraded,
                "the first owner consumes its downgrade only after crash unload"
            );
            assert_eq!(
                sequence.last(),
                Some(&LifecycleEvent::ResourceCeilingDowngraded)
            );
        }
        CgroupTestMode::Delegated => {
            assert!(sequence
                .iter()
                .all(|event| *event != LifecycleEvent::ResourceCeilingDowngraded));
            assert_eq!(sequence.last(), Some(&LifecycleEvent::Load));
        }
    }
    assert!(crash_index < sequence.len() - 1);
    let expected_last = *sequence.last().expect("restart sequence is non-empty");
    world.drain().await;
    let reopened_entries = lifecycle_entries(&journal_path)
        .into_iter()
        .filter(|entry| entry.spirit_id == spirit_id)
        .map(|entry| entry.lifecycle_event)
        .collect::<Vec<_>>();
    assert_eq!(reopened_entries, sequence);
    let reopened = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    assert_eq!(reopened.last_event(spirit_id), Some(expected_last));
}
