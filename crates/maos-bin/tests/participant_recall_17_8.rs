#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]
//! Story 17-8 AC1/AC4: real admitted native Researcher and real T2 recall-probe.
//! Every withheld window checks the complete caller journal and emitted reports,
//! including hex-encoded payloads, as well as the native port's result.
use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::recall_mediation::MediatedLogRecall;
use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionLaunch};
use maos_bin::supervision::WorkerSupervisor;
use maos_domain::frame::{
    ConsentEnvelope, ConsentRecall, FrameAddress, FramePayload, IacFrame, RuptureReason,
    TaskAssignPayload,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::log_recall::{
    EmitterScopeReason, FrameKindLabel, LogRecallError, LogRecallFilter, RecallScope,
};
use maos_domain::ports::{
    CrossWallRecallConsentDecision, CrossWallRecallConsentError, CrossWallRecallConsentPort,
    LogRecallPort,
};
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::mailbox::{ConsentGate, Mailbox, SpiritMailboxHandle};
use maos_iac::adapter::transparency_log::{
    FrameFilter, FrameKind as JournalKind, TransparencyLogEntry,
};
use maos_iac::adapter::DrrScheduler;
use maos_kernel_core::capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

const NONCE: u64 = 0x17_08;
const ID: &str = "recall-probe";
const INTENT: &str = "diagnosis-handoff:read-only-evidence";

#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

/// Story 20-12 AC3 — the WASM form runs only under the 17-2 cgroup runner,
/// which declares its lane. Off CI an unset lane is one `SKIP:` line; on a
/// declared-CI host the test runs on and `FIXTURES` fails closed.
fn cgroup_lane_skip() -> bool {
    ci::skip_unless(
        "MAOS_17_2_CGROUP_MODE=delegated|undelegated (scripts/run_cgroup_ceiling_17_2.py)",
        matches!(
            std::env::var("MAOS_17_2_CGROUP_MODE").as_deref(),
            Ok("delegated" | "undelegated")
        ),
    )
}

fn artifact() -> PathBuf {
    FIXTURES.1.clone()
}
fn relay_child(out: &std::process::Output) {
    for stream in [&out.stdout, &out.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            println!("child| {line}");
        }
    }
}
/// Prebuilt release `maos-wasm-runner` and `recall_probe.wasm` (manifest).
static FIXTURES: LazyLock<(PathBuf, PathBuf)> = LazyLock::new(|| {
    assert!(matches!(
        std::env::var("MAOS_17_2_CGROUP_MODE").as_deref(),
        Ok("delegated" | "undelegated")
    ));
    (
        fixtures::fixture("maos-wasm-runner"),
        fixtures::fixture("recall_probe.wasm"),
    )
});

#[derive(Default)]
struct CountingGate {
    calls: AtomicUsize,
    reject_caller: AtomicBool,
}
impl ConsentGate for CountingGate {
    fn evaluate(&self, _: &IacFrame, to: &FrameAddress) -> Result<(), RuptureReason> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(to.spirit_id.as_str(), ID | "researcher")
            && self.reject_caller.load(Ordering::SeqCst)
        {
            Err(RuptureReason::IntentAllowlistMismatch)
        } else {
            Ok(())
        }
    }
}

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
    recall: Arc<LogRecallAdapter>,
    clock: Arc<AtomicU64>,
    gate: Arc<CountingGate>,
    handles: Vec<SpiritMailboxHandle>,
    audit_writer: tokio::task::JoinHandle<()>,
}
impl World {
    fn new(dir: &Path, drr: bool) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        let lookup = Arc::new(RwLock::new(BTreeMap::from([
            ("a".into(), 10_001),
            ("b".into(), 10_002),
            ("other".into(), 10_003),
        ])));
        tl.install_recipient_resolver(Arc::clone(&lookup) as _)
            .unwrap();
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
        let gate = Arc::new(CountingGate::default());
        let mailbox = Arc::new(
            Mailbox::new(Arc::clone(&metrics))
                .with_consent_gate(Arc::clone(&gate) as _)
                .with_transparency_log(Arc::clone(&tl)),
        );
        let handles = ["a", "b", "other"]
            .map(|id| mailbox.register_spirit(id).unwrap())
            .into();
        let mut bus = IacBusAdapter::new(mailbox, Arc::clone(&tl));
        if drr {
            let (warnings, _) = tokio::sync::mpsc::unbounded_channel();
            bus = bus.with_drr_scheduler(DrrScheduler::new(Arc::clone(&tl), warnings));
        }
        let iac = Arc::new(bus);
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
            maos_bin::supervision::SupervisorSeams::default(),
        );
        let supervisor = Arc::clone(root.supervisor());
        let audit_writer = cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl));
        let clock = Arc::new(AtomicU64::new(1_000));
        let now = Arc::clone(&clock);
        let recall = Arc::new(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_consent_clock(Arc::new(move || now.load(Ordering::SeqCst))),
        );
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
            lookup,
            recall,
            clock,
            gate,
            handles,
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
            log_recall: Arc::clone(&self.recall),
            lifecycle_journal: Arc::clone(&self.journal),
            pid_by_spirit_id: Arc::clone(&self.lookup),
            enterprise: None,
            pdp: None,
        }
    }
    fn native(&self) -> MediatedLogRecall {
        MediatedLogRecall::new(
            Arc::clone(&self.recall),
            Arc::clone(&self.capability),
            Arc::clone(&self.security),
            Arc::clone(&self.tl),
            None,
            None,
            "researcher".into(),
        )
    }
    async fn drain(self) {
        let Self {
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
            lookup,
            recall,
            clock: _,
            gate: _,
            handles,
            audit_writer,
        } = self;
        root.stop_and_join().await;
        drop((
            capability, policy, security, memory, journal, scheduler, iac, halt, supervisor,
            lookup, recall, handles,
        ));
        tokio::time::timeout(Duration::from_secs(5), audit_writer)
            .await
            .expect("real audit writer drains")
            .unwrap();
        drop(tl);
    }
}

fn manifest() -> String {
    format!(
        r#"[class]
name = "{ID}"
description = "participant recall integration consumer"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 5
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
memory_max_mb = 1024
fd_max = 64
[sandbox]
tier = "T2"
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
fetch = true
"#
    )
}
async fn admit(
    world: &World,
    text: &str,
) -> (
    Arc<maos_bin::supervision::BindingCore>,
    maos_bin::admission::Admitted,
) {
    let gated = gate_manifest(text, &|name| name == "researcher").unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(&gated.spirit_id, &gated.bundle())
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
fn addr(id: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    }
}
fn envelope(granter: &str, policy: ConsentRecall) -> ConsentEnvelope {
    ConsentEnvelope {
        consent_id: ulid::Ulid::new().to_bytes(),
        granter: addr(granter),
        timestamp_ns: 1,
        intent_class: Some(A2AIntent::new(INTENT)),
        valid_until_ns: Some(2_000),
        recall: policy,
    }
}
fn input(to: &[FrameAddress], script: &str, sentinel: &str) -> IacFrame {
    IacFrame {
        frame_id: ulid::Ulid::new().to_bytes(),
        timestamp_ns: 1,
        logical_clock: 1,
        from: addr("operator"),
        to: to.iter().cloned().collect(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        auto_marker: FrameOrigin::HumanAuthored,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("to collector\n{script}"),
            scope: vec![],
            success_criteria: sentinel.into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}
fn launch(frame: IacFrame) -> SessionLaunch {
    LazyLock::force(&FIXTURES);
    SessionLaunch {
        plan: maos_host::SpiritLaunchPlan {
            program: FIXTURES.0.to_string_lossy().into_owned(),
            argv: vec![
                "--component".into(),
                artifact().to_string_lossy().into_owned(),
                "--fuel".into(),
                "100000000".into(),
            ],
            env: vec![],
            wire: maos_host::WireShape::ContentLengthCbor,
        },
        component_artifact: Some(artifact()),
        turn_budget: Duration::from_secs(30),
        once: false,
        context_window: Default::default(),
        initial_frame: Some(frame),
    }
}
fn rows(world: &World, pid: u32) -> Vec<TransparencyLogEntry> {
    world
        .tl
        .query_frames(FrameFilter {
            spirit_pid: Some(pid),
            order_by_insertion: true,
            ..Default::default()
        })
        .unwrap()
}
fn operations(world: &World, pid: u32) -> Vec<TransparencyLogEntry> {
    rows(world, pid)
        .into_iter()
        .filter(|r| r.intent.starts_with("log."))
        .collect()
}
fn data(frame: &IacFrame) -> &str {
    let FramePayload::TelemetryEvent(body) = &frame.payload else {
        panic!("probe report is telemetry")
    };
    assert_eq!(body.event_type, "recall.probe");
    &body.data
}
struct StopSession(Arc<WorkerSupervisor>);
impl Drop for StopSession {
    fn drop(&mut self) {
        self.0.stop_workers();
    }
}
async fn next_report(
    world: &World,
    pid: u32,
    rx: &mut tokio::sync::broadcast::Receiver<IacFrame>,
) -> IacFrame {
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            match rx.try_recv() {
                Ok(frame) if frame.from.spirit_id.as_str() == ID => return frame,
                Ok(_) | Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
                Err(error) => panic!("report receiver: {error}"),
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|error| {
        world.supervisor.stop_workers();
        panic!(
            "real T2 report deadline: {error}; caller rows={:?}",
            rows(world, pid)
        );
    })
}

#[derive(Clone, Copy, Debug)]
enum Form {
    Native,
    Wasm,
}
impl Form {
    fn id(self) -> &'static str {
        match self {
            Self::Native => "researcher",
            Self::Wasm => ID,
        }
    }
}
struct Caller {
    form: Form,
    pid: u32,
    native: Option<MediatedLogRecall>,
    admitted: Option<maos_bin::admission::Admitted>,
    core: Option<Arc<maos_bin::supervision::BindingCore>>,
    session: Option<
        tokio::task::JoinHandle<
            Result<maos_bin::spirit_session::SessionReport, maos_bin::spirit_session::SessionError>,
        >,
    >,
    stop: Option<StopSession>,
    reports: tokio::sync::broadcast::Receiver<IacFrame>,
    first: Option<IacFrame>,
}
impl Caller {
    async fn start(world: &mut World, form: Form) -> Self {
        let mut reports = world.iac.mailbox().subscribe_telemetry();
        let text = match form {
            Form::Native => include_str!("../../../spirits/researcher/manifest.toml").to_owned(),
            Form::Wasm => manifest(),
        };
        let (core, admitted) = admit(world, &text).await;
        let pid = admitted.pid;
        let policy = world.policy.inner().load_full();
        assert!(policy.manifest_scopes[&pid]
            .scopes
            .contains(&Scope::LogRecall));
        assert!(policy.manifest_scopes[&pid]
            .scopes
            .contains(&Scope::LogFetch));
        drop(policy);
        assert_eq!(world.lookup.read().unwrap().get(form.id()), Some(&pid));
        let (native, admitted, core, session, stop, first) = match form {
            Form::Native => {
                world.handles.push(
                    world
                        .iac
                        .register_spirit_typed(&SpiritId::from("researcher"))
                        .unwrap(),
                );
                (
                    Some(world.native()),
                    Some(admitted),
                    Some(core),
                    None,
                    None,
                    None,
                )
            }
            Form::Wasm => {
                let frame = input(&[addr(ID)], "", "bootstrap");
                let session = tokio::spawn(run_admitted_session(
                    world.session(),
                    admitted,
                    core,
                    launch(frame),
                ));
                let stop = StopSession(Arc::clone(&world.supervisor));
                let first = next_report(world, pid, &mut reports).await;
                assert_eq!(data(&first), "");
                (None, None, None, Some(session), Some(stop), Some(first))
            }
        };
        Self {
            form,
            pid,
            native,
            admitted,
            core,
            session,
            stop,
            reports,
            first,
        }
    }
    async fn deliver(
        &mut self,
        world: &World,
        frame: IacFrame,
        reaches_caller: bool,
    ) -> TransparencyLogEntry {
        let id = frame.frame_id;
        // CR-5: native/fixture peer bus sends are root-owned. An admitted
        // WASM sender is attributed to its scheduler pid, never a frame claim.
        let sender_pid =
            if matches!(self.form, Form::Wasm) && frame.from.spirit_id.as_str() == self.form.id() {
                self.pid
            } else {
                0
            };
        let caller_deliveries = frame
            .to
            .iter()
            .filter(|to| to.spirit_id.as_str() == self.form.id())
            .count();
        world
            .iac
            .deliver_typed(frame, sender_pid, None)
            .await
            .unwrap();
        if reaches_caller && matches!(self.form, Form::Wasm) {
            for _ in 0..caller_deliveries {
                let report = next_report(world, self.pid, &mut self.reports).await;
                assert_eq!(
                    data(&report),
                    "",
                    "data delivery is a report barrier, not a recall command"
                );
            }
        }
        world.tl.query_frame_by_id(id).unwrap().unwrap()
    }
    async fn finish(mut self, world: &World) {
        drop(self.native.take());
        drop(self.stop.take());
        if let Some(session) = self.session.take() {
            assert!(tokio::time::timeout(Duration::from_secs(10), session)
                .await
                .expect("bounded real session teardown")
                .unwrap()
                .is_err());
        }
        drop((self.admitted.take(), self.core.take()));
        assert!(!maos_bin::supervision::unload_all_loaded(
            world.scheduler.as_ref(),
            world.halt.as_ref(),
            None
        )
        .await
        .had_failures());
    }
}

#[derive(Clone, Copy, Debug)]
enum Outcome {
    Payload,
    HeaderOnly,
    Withheld,
    ScopeViolation,
}
fn assert_absent(bytes: &[u8], sentinel: &str) {
    for needle in [sentinel.to_owned(), hex::encode(sentinel)] {
        assert!(
            !bytes
                .windows(needle.len())
                .any(|part| part == needle.as_bytes()),
            "unauthorized sentinel (plain or hex) escaped: {sentinel}"
        );
    }
}
fn assert_operations(
    world: &World,
    pid: u32,
    start: usize,
    expected: Outcome,
    target: &TransparencyLogEntry,
) {
    let all = operations(world, pid);
    let ops = &all[start..];
    let code = match expected {
        Outcome::Payload => None,
        Outcome::ScopeViolation => Some("scope-violation"),
        _ => Some("consent-withheld"),
    };
    assert_eq!(
        ops.len(),
        if code.is_some() { 3 } else { 2 },
        "exact operation/receipt count"
    );
    let mut tokens = Vec::new();
    for (row, intent) in ops[..2].iter().zip(["log.recall", "log.fetch"]) {
        assert_eq!(
            (
                row.spirit_pid,
                row.boot_nonce,
                row.kind as u8,
                row.intent.as_str()
            ),
            (pid, NONCE, FrameKind::CapabilityInvocation as u8, intent)
        );
        let token = row
            .capability_token
            .expect("minted capability must be attributed");
        assert_ne!(&token[..16], &[0; 16]);
        assert_eq!(
            &token[16..],
            &[0; 16],
            "16-byte token id followed by sixteen zero bytes"
        );
        tokens.push(token);
    }
    assert_ne!(
        tokens[0], tokens[1],
        "recall and fetch mint independent tokens"
    );
    if let Some(code) = code {
        let receipt = &ops[2];
        assert_eq!(
            (
                receipt.spirit_pid,
                receipt.boot_nonce,
                receipt.kind as u8,
                receipt.intent.as_str()
            ),
            (pid, NONCE, 28, format!("log.fetch.refused:{code}").as_str())
        );
        assert_eq!(receipt.capability_token, None);
        let payload: serde_json::Value = serde_json::from_slice(&receipt.payload_redacted).unwrap();
        assert_eq!(payload["op"], "fetch");
        assert_eq!(payload["code"], code);
        assert_eq!(payload["boot_nonce"], NONCE);
        assert_eq!(
            payload["frame_id_hex"],
            researcher::encode_frame_id_hex(&target.frame_id)
        );
        if code == "consent-withheld" {
            assert!(payload.get("owner_pid").is_none());
        } else {
            assert_eq!(payload["owner_pid"], target.spirit_pid);
        }
    }
}
impl Caller {
    async fn check(
        &mut self,
        world: &World,
        target: &TransparencyLogEntry,
        outcome: Outcome,
        sentinel: &str,
    ) {
        let start = operations(world, self.pid).len();
        let row_start = rows(world, self.pid).len();
        let kind: FrameKindLabel = serde_json::from_str(&format!("\"{:?}\"", target.kind)).unwrap();
        let label = format!("{kind:?}");
        let listed = matches!(outcome, Outcome::Payload | Outcome::HeaderOnly);
        let available = matches!(outcome, Outcome::Payload);
        match self.form {
            Form::Native => {
                let port: &dyn LogRecallPort = self.native.as_ref().unwrap();
                let page = port
                    .recall(
                        self.pid,
                        LogRecallFilter {
                            kind: Some(kind),
                            since_ns: Some(target.timestamp_ns),
                            until_ns: Some(target.timestamp_ns),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                assert_eq!(page.scope, RecallScope::Participant);
                assert_eq!(page.next_cursor, None);
                let expected = if listed {
                    vec![(
                        target.frame_id,
                        target.timestamp_ns,
                        target.intent.clone(),
                        available,
                    )]
                } else {
                    vec![]
                };
                assert_eq!(
                    page.entries
                        .iter()
                        .map(|e| (
                            e.frame_id,
                            e.timestamp_ns,
                            e.intent.clone(),
                            e.payload_available
                        ))
                        .collect::<Vec<_>>(),
                    expected
                );
                let result = port.fetch(self.pid, target.frame_id);
                if !available {
                    assert_absent(&serde_json::to_vec(&page).unwrap(), sentinel);
                    assert_absent(format!("{result:?}").as_bytes(), sentinel);
                }
                match outcome {
                    Outcome::Payload => {
                        let fetched = result.unwrap();
                        assert_eq!(fetched.frame_id, target.frame_id);
                        assert_eq!(fetched.timestamp_ns, target.timestamp_ns);
                        assert_eq!(fetched.intent, target.intent);
                        assert_eq!(fetched.payload_redacted, target.payload_redacted);
                    }
                    Outcome::ScopeViolation => assert_eq!(
                        result,
                        Err(LogRecallError::ScopeViolation {
                            frame_id: target.frame_id,
                            requested_pid: self.pid,
                            owner_pid: target.spirit_pid
                        })
                    ),
                    _ => assert_eq!(
                        result,
                        Err(LogRecallError::ConsentWithheld {
                            frame_id: target.frame_id
                        })
                    ),
                }
            }
            Form::Wasm => {
                let script = format!(
                    "recall kind={label} since={} until={}\nfetch {}",
                    target.timestamp_ns,
                    target.timestamp_ns,
                    hex::encode(target.frame_id)
                );
                world
                    .iac
                    .deliver_typed(input(&[addr(ID)], &script, "accepted command"), 0, None)
                    .await
                    .unwrap();
                let report = next_report(world, self.pid, &mut self.reports).await;
                let mut expected = format!("recall ok {} cursor=none\n", usize::from(listed));
                if listed {
                    expected.push_str(&format!(
                        "entry {} {} {label} {} {available}\n",
                        hex::encode(target.frame_id),
                        target.timestamp_ns,
                        hex::encode(target.intent.as_bytes())
                    ));
                }
                match outcome {
                    Outcome::Payload => {
                        let origin = match target.origin {
                            FrameOrigin::HumanAuthored => "human-authored",
                            FrameOrigin::SpiritAuto => "spirit-auto",
                            FrameOrigin::SpiritDraftedHumanApproved => {
                                "spirit-drafted-human-approved"
                            }
                            FrameOrigin::Kernel => "kernel",
                        };
                        expected.push_str(&format!(
                            "fetch ok {} {} {label} {} {} {origin}\n",
                            hex::encode(target.frame_id),
                            target.timestamp_ns,
                            hex::encode(target.intent.as_bytes()),
                            hex::encode(&target.payload_redacted)
                        ));
                    }
                    Outcome::ScopeViolation => expected.push_str("fetch err scope-violation\n"),
                    _ => expected.push_str("fetch err consent-withheld\n"),
                }
                assert_eq!(data(&report), expected);
                if !available {
                    assert_absent(&serde_json::to_vec(&report).unwrap(), sentinel);
                }
            }
        }
        assert_operations(world, self.pid, start, outcome, target);
        if !available {
            for row in &rows(world, self.pid)[row_start..] {
                assert_absent(&row.payload_redacted, sentinel);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sender_owned_frames_ignore_received_consent_in_native_and_wasm() {
    if cgroup_lane_skip() {
        return;
    }
    for form in [Form::Native, Form::Wasm] {
        let dir = tempfile::tempdir().unwrap();
        let mut world = World::new(dir.path(), false);
        // V1 uses a real guest emission. Session dispatch deliberately strips
        // guest-supplied consent; V13 installs its sender envelope explicitly.
        let mut restricted = envelope(form.id(), ConsentRecall::HeaderOnly);
        restricted.valid_until_ns = Some(999);
        let mut caller = Caller::start(&mut world, form).await;
        let first = match form {
            Form::Native => {
                // Exactly the kernel MCP journal shape: sender-only, no routed to[] side rows.
                world.tl.insert_frame_event_with_sender(
                    JournalKind::McpInvocation,
                    caller.pid,
                    "researcher",
                    "",
                    None,
                    "mcp.invoke",
                    b"native-own-MCP-payload",
                    FrameOrigin::SpiritAuto,
                );
                rows(&world, caller.pid)
                    .into_iter()
                    .find(|r| r.kind == JournalKind::McpInvocation)
                    .unwrap()
            }
            Form::Wasm => world
                .tl
                .query_frame_by_id(caller.first.as_ref().unwrap().frame_id)
                .unwrap()
                .unwrap(),
        };
        caller
            .check(&world, &first, Outcome::Payload, "unused-authorized")
            .await;
        let mut own = input(&[], "", "sender-restricted-payload");
        own.from = addr(form.id());
        own.consent_envelope = Some(restricted);
        let own_row = match form {
            Form::Native => {
                // V13 retains the sender-only MCP journal shape. Only its
                // persisted envelope differs; native bus sends stay pid 0.
                own.auto_marker = FrameOrigin::SpiritAuto;
                world.tl.insert_routed_frame(
                    &own,
                    JournalKind::McpInvocation,
                    caller.pid,
                    None,
                    "mcp.invoke",
                    b"sender-restricted-payload",
                    &[],
                );
                world.tl.query_frame_by_id(own.frame_id).unwrap().unwrap()
            }
            Form::Wasm => {
                // Same sender-attributed TL shape as a probe send, plus the
                // envelope that production rightly forbids a guest to forge.
                own.to = smallvec::smallvec![addr("a")];
                caller.deliver(&world, own, false).await
            }
        };
        let conn = rusqlite::Connection::open(dir.path().join("transparency.sqlite")).unwrap();
        let policy: u32 = conn
            .query_row(
                "SELECT recall_policy FROM transparency_log_consent WHERE frame_id=?1",
                [&own_row.frame_id[..]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(policy, 1, "V13 really has a persisted HeaderOnly envelope");
        caller
            .check(&world, &own_row, Outcome::Payload, "unused-authorized")
            .await;
        caller.finish(&world).await;
        world.drain().await;
    }
}

async fn addressed_vectors(drr: bool) {
    for form in [Form::Native, Form::Wasm] {
        let dir = tempfile::tempdir().unwrap();
        let mut world = World::new(dir.path(), drr);
        let mut caller = Caller::start(&mut world, form).await;
        for vector in if drr {
            vec![2, 4]
        } else {
            vec![2, 3, 30, 4, 11, 14]
        } {
            let mut role_c = addr(form.id());
            role_c.role = Some(SpiritRole::Worker);
            let mut role_other = addr("other");
            role_other.role = Some(SpiritRole::Worker);
            let to = match vector {
                2 | 14 => vec![addr(form.id())],
                3 => vec![role_c.clone(), role_c],
                30 => vec![role_other],
                4 => vec![addr("a"), addr(form.id()), addr("b")],
                11 => vec![addr("other")],
                _ => unreachable!(),
            };
            let sentinel = format!("withheld-V{vector}-{form:?}-drr{drr}-secret");
            let frame = input(&to, "", &sentinel);
            let reaches = matches!(vector, 2 | 3 | 4);
            let before = world.gate.calls.load(Ordering::SeqCst);
            let withheld_start = rows(&world, caller.pid).len();
            world
                .gate
                .reject_caller
                .store(vector == 14, Ordering::SeqCst);
            let target = caller.deliver(&world, frame, reaches).await;
            world.gate.reject_caller.store(false, Ordering::SeqCst);
            assert_eq!(
                world.gate.calls.load(Ordering::SeqCst) - before,
                to.len(),
                "one gate evaluation per address"
            );
            let conn = rusqlite::Connection::open(dir.path().join("transparency.sqlite")).unwrap();
            let recipient_count: u32 = conn.query_row("SELECT count(*) FROM transparency_log_recipients WHERE frame_id=?1 AND recipient_pid=?2", rusqlite::params![&target.frame_id[..], caller.pid], |r| r.get(0)).unwrap();
            assert_eq!(
                recipient_count,
                u32::from(reaches),
                "id-based participation, deduped"
            );
            if vector == 11 {
                let count: u32 = conn.query_row("SELECT count(*) FROM transparency_log_recipients WHERE frame_id=?1 AND recipient_spirit_id='other'", [&target.frame_id[..]], |r| r.get(0)).unwrap();
                assert_eq!(count, 1, "V11 must have a recorded different recipient");
            }
            caller
                .check(
                    &world,
                    &target,
                    if reaches {
                        Outcome::Payload
                    } else {
                        Outcome::ScopeViolation
                    },
                    &sentinel,
                )
                .await;
            if !reaches {
                for row in &rows(&world, caller.pid)[withheld_start..] {
                    assert_absent(&row.payload_redacted, &sentinel);
                }
            }
        }
        caller.finish(&world).await;
        world.drain().await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn direct_recipient_ids_roles_dedup_multiaddress_and_gate_rejection_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    addressed_vectors(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn drr_addressed_and_nonfirst_recipients_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    addressed_vectors(true).await;
}

async fn consent_transition(rupture: bool) {
    for form in [Form::Native, Form::Wasm] {
        let dir = tempfile::tempdir().unwrap();
        let mut world = World::new(dir.path(), false);
        let mut caller = Caller::start(&mut world, form).await;
        let sentinel = format!("consent-transition-{rupture}-{form:?}-secret");
        let mut frame = input(&[addr(form.id())], "", &sentinel);
        frame.from = addr("a");
        frame.consent_envelope = Some(envelope("a", ConsentRecall::Full));
        let target = caller.deliver(&world, frame, true).await;
        caller
            .check(&world, &target, Outcome::Payload, &sentinel)
            .await;
        // The authorized report above is the barrier. Only subsequent rows/results
        // belong to the withheld window for THIS SAME stored frame id.
        if rupture {
            world.gate.reject_caller.store(true, Ordering::SeqCst);
            let mut later = input(&[addr(form.id())], "", "rupture-trigger-not-the-secret");
            later.from = addr("a");
            later.consent_envelope = Some(envelope("a", ConsentRecall::Full));
            let before = world.gate.calls.load(Ordering::SeqCst);
            caller.deliver(&world, later, false).await;
            world.gate.reject_caller.store(false, Ordering::SeqCst);
            assert_eq!(world.gate.calls.load(Ordering::SeqCst) - before, 1);
            let conn = rusqlite::Connection::open(dir.path().join("transparency.sqlite")).unwrap();
            let count: u32 = conn.query_row("SELECT count(*) FROM transparency_log_consent_ruptures WHERE recipient_pid=?1 AND granter_spirit_id='a' AND intent_class=?2", rusqlite::params![caller.pid, INTENT], |r| r.get(0)).unwrap();
            assert_eq!(
                count, 1,
                "real mailbox quarantine produces the grant-scoped rupture"
            );
        } else {
            world.clock.store(2_000, Ordering::SeqCst);
            caller
                .check(&world, &target, Outcome::Payload, &sentinel)
                .await;
            world.clock.store(2_001, Ordering::SeqCst);
        }
        caller
            .check(&world, &target, Outcome::Withheld, &sentinel)
            .await;
        caller.finish(&world).await;
        world.drain().await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn valid_consent_then_same_frame_expires_between_report_barriers_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    consent_transition(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn valid_consent_then_real_gate_rupture_withholds_same_frame_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    consent_transition(true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn header_only_including_oversized_and_withheld_envelopes_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    for form in [Form::Native, Form::Wasm] {
        let dir = tempfile::tempdir().unwrap();
        let mut world = World::new(dir.path(), false);
        let mut caller = Caller::start(&mut world, form).await;
        for (policy, large, outcome) in [
            (ConsentRecall::HeaderOnly, false, Outcome::HeaderOnly),
            (ConsentRecall::HeaderOnly, true, Outcome::HeaderOnly),
            (ConsentRecall::Withheld, false, Outcome::Withheld),
        ] {
            let sentinel = format!("policy-{policy:?}-large{large}-{form:?}-secret");
            let body = if large {
                format!(
                    "{sentinel}{}",
                    // JSON journals escape each control byte to six bytes;
                    // CBOR delivery remains below its own frame cap.
                    "\u{1f}".repeat(maos_wasm_host::recall_wire::REPLY_BYTE_BOUND as usize / 6 + 1)
                )
            } else {
                sentinel.clone()
            };
            let mut frame = input(&[addr(form.id())], "", &body);
            frame.from = addr("a");
            frame.consent_envelope = Some(envelope("a", policy));
            let withheld_start = rows(&world, caller.pid).len();
            let target = caller.deliver(&world, frame, true).await;
            if large {
                assert!(
                    target.payload_redacted.len()
                        > maos_wasm_host::recall_wire::REPLY_BYTE_BOUND as usize
                );
            }
            caller.check(&world, &target, outcome, &sentinel).await;
            for row in &rows(&world, caller.pid)[withheld_start..] {
                assert_absent(&row.payload_redacted, &sentinel);
            }
        }
        caller.finish(&world).await;
        world.drain().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn earlier_boot_same_pid_recipient_is_out_of_scope_both_forms() {
    if cgroup_lane_skip() {
        return;
    }
    const CHILD: &str = "MAOS_17_8_PRIOR_BOOT_FORM";
    let form = match std::env::var(CHILD).as_deref() {
        Ok("native") => Form::Native,
        Ok("wasm") => Form::Wasm,
        _ => {
            for form in ["native", "wasm"] {
                let out = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "earlier_boot_same_pid_recipient_is_out_of_scope_both_forms",
                        "--nocapture",
                    ])
                    .env(CHILD, form)
                    .output()
                    .unwrap();
                relay_child(&out);
                assert!(out.status.success(), "prior-boot {form} child");
            }
            return;
        }
    };
    let dir = tempfile::tempdir().unwrap();
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let sentinel = format!("earlier-boot-{form:?}-secret");
    let path = dir.path().join("transparency.sqlite");
    let old = Arc::new(TransparencyLogAdapter::open(&path, NONCE - 1).unwrap());
    let lookup = Arc::new(RwLock::new(BTreeMap::from([(form.id().to_owned(), 1u32)])));
    old.install_recipient_resolver(Arc::clone(&lookup) as _)
        .unwrap();
    let mailbox = Arc::new(Mailbox::new(Arc::new(
        maos_iac::adapter::IacRtMetrics::new(),
    )));
    let handle = mailbox.register_spirit(form.id()).unwrap();
    let bus = IacBusAdapter::new(mailbox, Arc::clone(&old));
    let frame = input(&[addr(form.id())], "", &sentinel);
    let id = frame.frame_id;
    bus.deliver_typed(frame, 0, None).await.unwrap();
    drop((bus, handle, old));
    // Reopen the SAME artifact under a new boot, then admit the same numeric pid.
    let mut world = World::new(dir.path(), false);
    let mut caller = Caller::start(&mut world, form).await;
    assert_eq!(
        caller.pid, 1,
        "isolated child pins the reincarnated numeric pid"
    );
    let target = world.tl.query_frame_by_id(id).unwrap().unwrap();
    assert_eq!(target.boot_nonce, NONCE - 1);
    caller
        .check(&world, &target, Outcome::ScopeViolation, &sentinel)
        .await;
    caller.finish(&world).await;
    world.drain().await;
}

struct GrantedConsent;
impl CrossWallRecallConsentPort for GrantedConsent {
    fn decide(
        &self,
        _: &maos_domain::team::TeamId,
        _: &str,
    ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
        Ok(CrossWallRecallConsentDecision::Granted)
    }
}
struct RestoreHome(Option<std::ffi::OsString>);
impl Drop for RestoreHome {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("MAOS_HOME", value),
            None => std::env::remove_var("MAOS_HOME"),
        }
    }
}
#[path = "../../../tests/harness/legacy_tl_schema.rs"]
mod legacy_tl_schema;
/// V15/V16 (AC6.2, AC6.3) — `team-b` is a current artifact whose side tables
/// are populated (a recipient row even names the caller on the unrelated
/// emitter's frame); `team-c` is built from the verbatim pre-17-8 DDL, never
/// by dropping tables. Both page identically: emitter-scoped metadata with the
/// cross-wall marker, zero payload, and the side rows are never consulted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_wall_remote_artifact_is_identically_emitter_scoped_with_or_without_side_tables() {
    let _restore = RestoreHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("MAOS_HOME", home.path());
    let dir = tempfile::tempdir().unwrap();
    let mut world = World::new(dir.path(), false);
    let caller = Caller::start(&mut world, Form::Native).await;
    let remotes = [
        ("team-b", "maos_team_b", true),
        ("team-c", "maos_team_c", false),
    ]
    .map(|(team, datname, current)| {
        let remote = maos_domain::team::TeamId::new(team).unwrap();
        let path =
            maos_audit::transparency_log_path_for_tenant_mode(true, Some(remote.as_str())).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if current {
            drop(TransparencyLogAdapter::open(&path, NONCE - 1).unwrap());
        }
        let conn = rusqlite::Connection::open(&path).unwrap();
        if !current {
            conn.execute_batch(legacy_tl_schema::LEGACY_SCHEMA_SQL)
                .unwrap();
        }
        for (id, pid, marker) in [
            (1u8, caller.pid, "remote-emitter-metadata"),
            (2, caller.pid + 1, "remote-unrelated-emitter"),
        ] {
            conn.execute(
                "INSERT INTO transparency_log
                     (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                      boot_nonce, capability_token, kind, intent, correlation_id,
                      payload_redacted, origin)
                     VALUES (?1, ?2, ?3, '', '', ?4, NULL, 0, ?5, NULL, ?6, 0)",
                rusqlite::params![
                    vec![id; 16],
                    i64::from(id),
                    pid,
                    (NONCE - 1) as i64,
                    marker,
                    marker.as_bytes()
                ],
            )
            .unwrap();
        }
        if current {
            conn.execute(
                "INSERT INTO transparency_log_recipients VALUES (?1, ?2, 'researcher', NULL)",
                rusqlite::params![vec![2u8; 16], caller.pid],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO transparency_log_consent
                     VALUES (?1, ?2, 'sender', NULL, NULL, ?3, 'remote', NULL, 2)",
                rusqlite::params![vec![1u8; 16], vec![7u8; 16], i64::MIN + 1],
            )
            .unwrap();
        }
        // Freeze the offline artifact in rollback-journal mode (as the
        // readers suite does) so the read-only opener never needs a WAL.
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .unwrap();
        drop(conn);
        maos_audit::write_tenant_binding(&path, &remote, Some(datname)).unwrap();
        remote
    });
    let adapter = LogRecallAdapter::new(Arc::clone(&world.tl))
        .with_cross_wall_consent(Arc::new(GrantedConsent))
        .with_cross_wall_read(Arc::new(
            maos_bin::cross_wall_log_read::CrossWallLogReadAdapter::new(true),
        ));
    let [with_tables, without_tables] = remotes.each_ref().map(|remote| {
        adapter
            .recall_cross_wall(caller.pid, remote, LogRecallFilter::default())
            .unwrap()
    });
    assert_eq!(with_tables, without_tables);
    assert_eq!(
        with_tables.scope,
        RecallScope::Emitter {
            reason: EmitterScopeReason::CrossWall
        }
    );
    assert_eq!(with_tables.entries.len(), 1);
    assert_eq!(with_tables.entries[0].frame_id, [1; 16]);
    assert_eq!(with_tables.entries[0].peer_spirit_pid, caller.pid);
    assert_eq!(with_tables.entries[0].intent, "remote-emitter-metadata");
    assert!(with_tables.entries[0].payload_available);
    assert_eq!(with_tables.next_cursor, None);
    let ops = operations(&world, caller.pid);
    assert_eq!(
        ops.len(),
        4,
        "pre-movement and truthful outcome rows per cross-wall page"
    );
    for (row, (team, outcome)) in ops.iter().zip([
        ("team-b", "disclosing"),
        ("team-b", "disclosed"),
        ("team-c", "disclosing"),
        ("team-c", "disclosed"),
    ]) {
        assert_eq!(row.spirit_pid, caller.pid);
        assert_eq!(row.boot_nonce, NONCE);
        assert_eq!(row.intent, "log.recall.cross-wall");
        assert_eq!(row.kind, JournalKind::CapabilityInvocation);
        assert_eq!(row.capability_token, None);
        let payload: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "remote_team": team, "outcome": outcome, "consent_grant": "log:recall",
                "crossed_team_boundary": outcome == "disclosed", "refusal": null,
            })
        );
    }
    drop(adapter);
    caller.finish(&world).await;
    world.drain().await;
}
