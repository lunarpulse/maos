#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]
//! Story 17.3d: real release runner, real T2, real admission/session dispatch.
use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionError, SessionLaunch};
use maos_bin::supervision::WorkerSupervisor;
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::transparency_log::{
    FrameFilter, FrameKind as JournalKind, TransparencyLogEntry,
};
use maos_kernel_core::capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, LazyLock, RwLock,
};
use std::time::Duration;
const NONCE: u64 = 0x17_03_d;
const ID: &str = "recall-probe";
#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
/// Relay a re-executed child test's output without letting its own libtest
/// `test ... ok` / `test result:` lines read as this suite's (CI counts them).
fn relay_child(out: &std::process::Output) {
    for stream in [&out.stdout, &out.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            println!("child| {line}");
        }
    }
}
fn artifact() -> PathBuf {
    FIXTURES.1.clone()
}
// Recall's larger import/link surface needs more address space than echo.
// The manifest declares 1 GiB for the undelegated lane's RLIMIT_AS fallback;
// this remains a real T2 child, with Landlock and seccomp unchanged.
/// Prebuilt release `maos-wasm-runner` and `recall_probe.wasm` (manifest).
static FIXTURES: LazyLock<(PathBuf, PathBuf)> = LazyLock::new(|| {
    match std::env::var("MAOS_17_2_CGROUP_MODE").as_deref() {
        Ok("delegated" | "undelegated") => {}
        other => panic!("MAOS_17_2_CGROUP_MODE must be delegated or undelegated: {other:?}"),
    }
    (
        fixtures::fixture("maos-wasm-runner"),
        fixtures::fixture("recall_probe.wasm"),
    )
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
    _precharge: Option<maos_kernel_core::capability::cap_quota::Charge>,
}

impl World {
    fn new(dir: &Path) -> Self {
        Self::with_quota(dir, false)
    }

    fn with_quota(dir: &Path, exhausted: bool) -> Self {
        use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
        let quota = if exhausted {
            CapQuotaTracker::with_clock(
                IssuanceQuota {
                    budget_per_window: 1,
                    window: Duration::from_secs(3600),
                },
                Arc::new(QuotaClock::default()),
            )
        } else {
            CapQuotaTracker::new()
        };
        Self::with_tracker(dir, quota, exhausted)
    }

    fn with_tracker(
        dir: &Path,
        quota: maos_kernel_core::capability::cap_quota::CapQuotaTracker,
        exhausted: bool,
    ) -> Self {
        let seams = maos_bin::supervision::SupervisorSeams::default();
        // A fresh child process admits pid 1. Retain its sole reservation for
        // the World's lifetime so the next issuance refuses before rollover.
        let precharge = exhausted.then(|| quota.charge(1).unwrap());
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        // Story 17-8: recipient scope uses the same trusted map as admission/session.
        let lookup = Arc::new(RwLock::new(BTreeMap::<String, u32>::new()));
        tl.install_recipient_resolver(Arc::clone(&lookup) as _)
            .expect("install the sole admission/session recipient resolver");
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
            quota,
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
            audit_writer,
            _precharge: precharge,
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
            _precharge: _,
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

fn manifest(grant: bool) -> String {
    format!(
        r#"[class]
name = "{ID}"
description = "real recall integration consumer"
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
{}"#,
        if grant {
            "[capabilities.required.log]\nrecall = true\nfetch = true\n"
        } else {
            ""
        }
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

fn input(script: &str) -> IacFrame {
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
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
            success_criteria: "report exact recall results".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

fn launch(frame: IacFrame, once: bool) -> SessionLaunch {
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
        once,
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
        .filter(|r| r.intent.starts_with("log.") || r.intent.starts_with("recall.channel."))
        .collect()
}
fn assert_row(row: &TransparencyLogEntry, pid: u32, kind: i64, intent: &str, token: bool) {
    assert_eq!(
        (
            row.spirit_pid,
            row.boot_nonce,
            row.kind as i64,
            row.intent.as_str()
        ),
        (pid, NONCE, kind, intent)
    );
    assert_eq!(
        row.capability_token.is_some(),
        token,
        "{intent}: actual minted-token column"
    );
    if let Some(token) = row.capability_token {
        assert_ne!(&token[..16], &[0; 16]);
        assert_eq!(&token[16..], &[0; 16]);
    }
}
fn data(frame: &IacFrame) -> &str {
    let FramePayload::TelemetryEvent(body) = &frame.payload else {
        panic!("probe emits TelemetryEvent")
    };
    assert_eq!(body.event_type, "recall.probe");
    &body.data
}
fn claim_reports(world: &World, pid: u32) -> Vec<String> {
    rows(world, pid)
        .into_iter()
        .filter(|r| r.intent == "spirit.frame.authorization")
        .map(|r| {
            let value: serde_json::Value = serde_json::from_slice(&r.payload_redacted).unwrap();
            let frame: IacFrame = serde_json::from_value(value["claims"].clone()).unwrap();
            data(&frame).to_owned()
        })
        .collect()
}

fn t2_result<T>(result: Result<T, SessionError>) -> Option<T> {
    match result {
        Err(SessionError::Bridge(error)) if error.to_lowercase().contains("permission denied") => {
            if !ci::skip_unless(&format!("real T2 permitted by this host ({error})"), false) {
                panic!("recall_17_3d: real T2 unavailable on runner declared capable (rule 11(b)): {error}");
            }
            None
        }
        other => Some(other.unwrap_or_else(|e| panic!("real admitted T2 session failed: {e:?}"))),
    }
}

async fn once(world: &World, granted: bool, script: &str) -> Option<(u32, String)> {
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(world, &manifest(granted)).await;
    let pid = admitted.pid;
    let report = t2_result(
        run_admitted_session(world.session(), admitted, core, launch(input(script), true)).await,
    )?;
    assert_eq!(report.turns, 1);
    assert_ne!(report.child_pid, std::process::id());
    let reports = claim_reports(world, pid);
    assert_eq!(reports.len(), 1);
    Some((pid, reports[0].clone()))
}

// Story 17-8: received seeds now succeed without a refusal receipt.
fn assert_r1_r2(world: &World, pid: u32) {
    let ops = operations(world, pid);
    assert_eq!(ops.len(), 2);
    assert_row(&ops[0], pid, 7, "log.recall", true);
    assert_row(&ops[1], pid, 7, "log.fetch", true);
    let all = rows(world, pid);
    assert!(
        all.iter().all(
            |r| !(r.intent.starts_with("log.") && r.intent.contains(".refused:"))
                && !r.intent.starts_with("recall.channel.closed:")
        ),
        "successful R1→R2 has no refusal or channel-close receipt"
    );
    let positions = [ops[0].frame_id, ops[1].frame_id]
        .map(|id| all.iter().position(|r| r.frame_id == id).unwrap());
    let authorization = all
        .iter()
        .position(|r| r.intent == "spirit.frame.authorization")
        .unwrap();
    assert!(
        positions[0] < positions[1] && positions[1] < authorization,
        "R1→R2 precede guest output authorization, allowing interleaved identity rows"
    );
}

fn assert_r1_r2_r3(world: &World, pid: u32) {
    let ops = operations(world, pid);
    assert_eq!(ops.len(), 3);
    assert_row(&ops[0], pid, 7, "log.recall", true);
    assert_row(&ops[1], pid, 7, "log.fetch", true);
    assert_row(&ops[2], pid, 28, "log.fetch.refused:scope-violation", false);
    let all = rows(world, pid);
    let r3 = all
        .iter()
        .position(|r| r.frame_id == ops[2].frame_id)
        .unwrap();
    let authorization = all
        .iter()
        .position(|r| r.intent == "spirit.frame.authorization")
        .unwrap();
    assert!(
        r3 < authorization,
        "R1→R2→R3 precede guest output authorization"
    );
}

fn fr4(dir: &Path, pid: u32) -> String {
    let entries = maos_audit::query(
        &dir.join("transparency.sqlite"),
        maos_audit::AuditFilter {
            spirit_pid: Some(pid),
            boot_nonce: Some(NONCE),
            ..Default::default()
        },
    )
    .unwrap();
    let mut plain = vec![];
    maos_audit::to_fr4_plain(entries, &mut plain)
        .expect("real audit --spirit plain formatter succeeds (CLI exit 0)");
    String::from_utf8(plain).unwrap()
}

fn t2_available() -> bool {
    // Story 20-12 AC3 — off CI an undeclared 17-2 lane is one `SKIP:` line; a
    // declared-CI host runs on and `FIXTURES` fails closed on the unset lane.
    if ci::skip_unless(
        "MAOS_17_2_CGROUP_MODE=delegated|undelegated (scripts/run_cgroup_ceiling_17_2.py)",
        matches!(
            std::env::var("MAOS_17_2_CGROUP_MODE").as_deref(),
            Ok("delegated" | "undelegated")
        ),
    ) {
        return false;
    }
    static AVAILABLE: LazyLock<bool> = LazyLock::new(|| {
        use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec, SpawnError};
        let spec = SandboxSpec {
            tier: maos_domain::invariants::i9::SandboxTier::T2,
            resolved_caps: Default::default(),
            declared_scopes: vec![],
            spirit_id: "recall-t2-availability".into(),
            output_shape_predicate: None,
        };
        match spawn_sandboxed(&spec, &mut std::process::Command::new("/bin/true")) {
            Ok(mut child) => {
                assert!(child.wait().unwrap().success());
                true
            }
            Err(SpawnError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                assert!(
                    !ci::declared_ci(),
                    "real T2 unavailable on runner declared capable (rule 11(b)): {error}"
                );
                false
            }
            Err(error) => panic!("T2 availability probe failed: {error:?}"),
        }
    });
    !ci::skip_unless("real T2 permitted by this host (rule 11(b))", *AVAILABLE)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn seed_fetch_succeeds_r1_r2_precede_authorization_without_receipt_and_fr4_succeeds() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    // Story 17-8: compare live output, since audit claims redact hex ids/payloads.
    let mut telemetry = world.iac.mailbox().subscribe_telemetry();
    if let Some((pid, _)) = once(&world, true, "recall\nfetch self").await {
        let emitted = next_report(&world, pid, &mut telemetry).await;
        // Story 17-8: the seed is a received participant row, not emitter-only.
        let seed = world
            .tl
            .query_frames(FrameFilter {
                spirit_pid: Some(0),
                kind: Some(JournalKind::TaskAssign),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(seed.len(), 1);
        let seed = &seed[0];
        assert_eq!(data(&emitted), format!(
            "recall ok 1 cursor=none\nentry {} {} TaskAssign {} true\nfetch ok {} {} TaskAssign {} {} human-authored\n",
            hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes()),
            hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes()), hex::encode(&seed.payload_redacted)
        ));
        assert_r1_r2(&world, pid);
        let plain = fr4(dir.path(), pid);
        println!("FR4-SEED-BEGIN\n{plain}FR4-SEED-END");
        assert!(!plain.contains(".refused:"), "{plain}");
        assert!(!plain.contains("recall.channel.closed:"), "{plain}");
        for intent in ["log.recall", "log.fetch"] {
            assert!(plain.contains(intent), "{plain}");
        }
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_page_has_one_token_bearing_call_and_no_receipt() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    // Story 17-8: deliberately exclude the received TaskAssign seed.
    if let Some((pid, report)) = once(&world, true, "recall kind=TelemetryEvent").await {
        assert_eq!(report, "recall ok 0 cursor=none\n");
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 1);
        assert_row(&ops[0], pid, 7, "log.recall", true);
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ungranted_manifest_denies_with_receipt_only() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    if let Some((pid, report)) = once(&world, false, "recall").await {
        assert_eq!(report, "recall err capability-denied\n");
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 1);
        assert_row(
            &ops[0],
            pid,
            28,
            "log.recall.refused:capability-denied",
            false,
        );
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operator_deny_overrides_manifest_and_has_receipt_only() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let mut policy = (*world.policy.inner().load_full()).clone();
    policy
        .operator_policy
        .per_capability_deny
        .insert("log.recall".into());
    world.policy.update(policy);
    if let Some((pid, report)) = once(&world, true, "recall").await {
        assert_eq!(report, "recall err capability-denied\n");
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 1);
        assert_row(
            &ops[0],
            pid,
            28,
            "log.recall.refused:capability-denied",
            false,
        );
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exhausted_quota_has_receipt_without_tokenless_fallback_call() {
    if !t2_available() {
        return;
    }
    // Scheduler pid allocation is process-global. Each operation gets its own
    // fresh process, scheduler, registry, reservation and journal.
    const CHILD: &str = "MAOS_17_3D_QUOTA_CHILD";
    let Ok(op) = std::env::var(CHILD) else {
        for op in ["recall", "fetch"] {
            let out = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "exhausted_quota_has_receipt_without_tokenless_fallback_call",
                    "--nocapture",
                ])
                .env(CHILD, op)
                .output()
                .unwrap();
            relay_child(&out);
            assert!(out.status.success(), "{op} exhaustion child");
        }
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let world = World::with_quota(dir.path(), true);
    let calls = Arc::new(AtomicUsize::new(0));
    let mut deps = world.session();
    deps.log_recall = Arc::new(
        maos_iac::adapter::log_recall::LogRecallAdapter::new(Arc::clone(&world.tl))
            .with_consent_clock({
                let calls = Arc::clone(&calls);
                Arc::new(move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    maos_domain::frame::consent_wall_now_ns()
                })
            }),
    );
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    assert_eq!(pid, 1, "pre-charge the actual admitted pid");
    let script = if op == "fetch" {
        "fetch self"
    } else {
        "recall"
    };
    if t2_result(run_admitted_session(deps, admitted, core, launch(input(script), true)).await)
        .is_some()
    {
        assert_eq!(
            claim_reports(&world, pid),
            vec![format!("{op} err capability-denied\n")],
            "guest-visible denial contains no pid, budget or window"
        );
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 1);
        assert_row(
            &ops[0],
            pid,
            28,
            &format!("log.{op}.refused:capability-denied"),
            false,
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "exhaustion never invokes the adapter"
        );
    }
    drop(_collector);
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_boot_same_pid_is_not_recalled_and_fetch_is_scope_violation() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    // Story 17-8: live output preserves recalled ids that audit claims redact.
    let mut telemetry = world.iac.mailbox().subscribe_telemetry();
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    let old =
        TransparencyLogAdapter::open(&dir.path().join("transparency.sqlite"), NONCE - 1).unwrap();
    old.insert_frame_event_with_id(
        Some([0x33; 16]),
        JournalKind::TaskAssign,
        pid,
        ID,
        "",
        None,
        "Standard",
        b"old incarnation",
        FrameOrigin::SpiritAuto,
    );
    drop(old);
    let script = format!("recall\nfetch {}", hex::encode([0x33; 16]));
    let seed = input(&script);
    let seed_id = seed.frame_id;
    if t2_result(run_admitted_session(world.session(), admitted, core, launch(seed, true)).await)
        .is_some()
    {
        // Story 17-8: current-boot received seed is visible; old incarnation is not.
        let seed = world.tl.query_frame_by_id(seed_id).unwrap().unwrap();
        let emitted = next_report(&world, pid, &mut telemetry).await;
        assert_eq!(claim_reports(&world, pid).len(), 1);
        assert_eq!(
            data(&emitted),
            format!("recall ok 1 cursor=none\nentry {} {} TaskAssign {} true\nfetch err scope-violation\n",
                hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes()))
        );
        assert_r1_r2_r3(&world, pid);
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn oversized_fetch_reply_is_typed_and_next_call_same_turn_still_works() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    world.tl.insert_frame_event_with_id(
        Some([0x44; 16]),
        JournalKind::TaskAssign,
        pid,
        ID,
        "",
        None,
        "Standard",
        &vec![b'x'; maos_wasm_host::recall_wire::REPLY_BYTE_BOUND as usize + 1],
        FrameOrigin::SpiritAuto,
    );
    let script = format!("fetch {}\nrecall limit=0", hex::encode([0x44; 16]));
    if t2_result(
        run_admitted_session(
            world.session(),
            admitted,
            core,
            launch(input(&script), true),
        )
        .await,
    )
    .is_some()
    {
        assert_eq!(
            claim_reports(&world, pid),
            ["fetch err reply-too-large\nrecall ok 0 cursor=none\n"]
        );
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 3);
        assert_row(&ops[0], pid, 7, "log.fetch", true);
        assert_row(&ops[1], pid, 28, "log.fetch.refused:reply-too-large", false);
        assert_row(&ops[2], pid, 7, "log.recall", true);
    }
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sixty_fifth_recall_closes_channel_once_with_typed_protocol_end() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    let result = run_admitted_session(
        world.session(),
        admitted,
        core,
        launch(input("recalls 65"), true),
    )
    .await;
    assert!(
        matches!(
            result,
            Err(SessionError::Protocol("recall budget exceeded"))
        ),
        "{result:?}"
    );
    let ops = operations(&world, pid);
    assert_eq!(ops.len(), 65);
    for row in &ops[..64] {
        assert_row(row, pid, 7, "log.recall", true);
    }
    assert_row(
        &ops[64],
        pid,
        28,
        "recall.channel.closed:protocol-violation",
        false,
    );
    world.drain().await;
}

// A failing consumer assertion must not leave an idle spawn_blocking session
// alive forever while Tokio waits for blocking tasks during runtime teardown.
struct StopSession(Arc<WorkerSupervisor>);
impl Drop for StopSession {
    fn drop(&mut self) {
        self.0.stop_workers();
    }
}

async fn next_report(
    world: &World,
    pid: u32,
    handle: &mut tokio::sync::broadcast::Receiver<IacFrame>,
) -> IacFrame {
    let result = tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            if let Ok(frame) = handle.try_recv() {
                return frame;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    match result {
        Ok(frame) => frame,
        Err(error) => {
            world.supervisor.stop_workers();
            let journal = rows(world, pid)
                .into_iter()
                .map(|r| {
                    (
                        r.intent,
                        String::from_utf8_lossy(&r.payload_redacted).into_owned(),
                    )
                })
                .collect::<Vec<_>>();
            panic!("real runner report deadline: {error}; journal={journal:?}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn own_frame_second_turn_recalls_kernel_minted_id_kind_intent_and_fetches_stored_payload() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let mut collector = world.iac.mailbox().subscribe_telemetry();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    let first_input = input("");
    let guest_id = first_input.frame_id;
    let session = tokio::spawn(run_admitted_session(
        world.session(),
        admitted,
        core,
        launch(first_input, false),
    ));
    let stop = StopSession(world.supervisor.clone());
    let first = next_report(&world, pid, &mut collector).await;
    assert_eq!(data(&first), "");
    assert_ne!(
        first.frame_id, guest_id,
        "session, not guest, mints the stored id"
    );
    let stored = world.tl.query_frame_by_id(first.frame_id).unwrap().unwrap();
    assert_eq!(stored.spirit_pid, pid);
    assert_eq!(stored.boot_nonce, NONCE);
    assert_eq!(stored.kind as i64, 4);
    assert_eq!(stored.intent, "standard");
    // Story 17-8: exclude both received TaskAssign inputs; recall only own output.
    let second_input = input("recall kind=TelemetryEvent\nfetch last");
    world
        .iac
        .deliver_typed(second_input, 0, None)
        .await
        .unwrap();
    let second = next_report(&world, pid, &mut collector).await;
    let expected = format!("recall ok 1 cursor=none\nentry {} {} TelemetryEvent {} true\nfetch ok {} {} TelemetryEvent {} {} spirit-auto\n",
        hex::encode(first.frame_id), stored.timestamp_ns, hex::encode(stored.intent.as_bytes()),
        hex::encode(first.frame_id), stored.timestamp_ns, hex::encode(stored.intent.as_bytes()), hex::encode(&stored.payload_redacted));
    assert_eq!(data(&second), expected);
    drop(stop);
    session.await.unwrap().unwrap_err();
    let ops = operations(&world, pid);
    assert_eq!(ops.len(), 2);
    assert_row(&ops[0], pid, 7, "log.recall", true);
    assert_row(&ops[1], pid, 7, "log.fetch", true);
    drop(collector);
    world.drain().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn own_receipt_and_authorization_ids_are_never_recalled_and_fetch_is_scope_violation() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let mut collector = world.iac.mailbox().subscribe_telemetry();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    // Story 17-8: fetch-self now succeeds; bootstrap with a known kind-7 row.
    let bootstrap_id = [0x58; 16];
    world.tl.insert_frame_event_with_id(
        Some(bootstrap_id),
        JournalKind::CapabilityInvocation,
        pid,
        ID,
        "",
        None,
        "receipt-isolation.bootstrap",
        b"non-disclosable bootstrap",
        FrameOrigin::Kernel,
    );
    let seed = input(&format!("fetch {}", hex::encode(bootstrap_id)));
    let seed_id = seed.frame_id;
    let session = tokio::spawn(run_admitted_session(
        world.session(),
        admitted,
        core,
        launch(seed, false),
    ));
    let stop = StopSession(world.supervisor.clone());
    let first = next_report(&world, pid, &mut collector).await;
    assert_eq!(data(&first), "fetch err scope-violation\n");
    let all = rows(&world, pid);
    let receipt = all
        .iter()
        .find(|r| r.intent == "log.fetch.refused:scope-violation")
        .unwrap();
    let authorization = all
        .iter()
        .find(|r| r.intent == "spirit.frame.authorization")
        .unwrap();
    let script = format!(
        "recall\nfetch {}\nfetch {}",
        hex::encode(receipt.frame_id),
        hex::encode(authorization.frame_id)
    );
    let command = input(&script);
    let command_id = command.frame_id;
    world.iac.deliver_typed(command, 0, None).await.unwrap();
    let second = next_report(&world, pid, &mut collector).await;
    // Story 17-8: unfiltered recall includes both received commands and own output,
    // but still excludes the bootstrap, refusal receipt and authorization.
    let mut expected_rows = [
        (
            world.tl.query_frame_by_id(seed_id).unwrap().unwrap(),
            "TaskAssign",
        ),
        (
            world.tl.query_frame_by_id(first.frame_id).unwrap().unwrap(),
            "TelemetryEvent",
        ),
        (
            world.tl.query_frame_by_id(command_id).unwrap().unwrap(),
            "TaskAssign",
        ),
    ];
    expected_rows.sort_by_key(|(row, _)| (row.timestamp_ns, row.frame_id));
    let entries = expected_rows
        .iter()
        .map(|(row, kind)| {
            format!(
                "entry {} {} {kind} {} true\n",
                hex::encode(row.frame_id),
                row.timestamp_ns,
                hex::encode(row.intent.as_bytes())
            )
        })
        .collect::<String>();
    assert_eq!(
        data(&second),
        format!(
        "recall ok 3 cursor=none\n{entries}fetch err scope-violation\nfetch err scope-violation\n"
    )
    );
    drop(stop);
    session.await.unwrap().unwrap_err();
    let ops = operations(&world, pid);
    assert_eq!(ops.len(), 7);
    for (row, intent, kind, token) in [
        (&ops[0], "log.fetch", 7, true),
        (&ops[1], "log.fetch.refused:scope-violation", 28, false),
        (&ops[2], "log.recall", 7, true),
        (&ops[3], "log.fetch", 7, true),
        (&ops[4], "log.fetch.refused:scope-violation", 28, false),
        (&ops[5], "log.fetch", 7, true),
        (&ops[6], "log.fetch.refused:scope-violation", 28, false),
    ] {
        assert_row(row, pid, kind, intent, token);
    }
    drop(collector);
    world.drain().await;
}

#[path = "../../maos-sso/tests/fixtures.rs"]
mod sso_fixtures;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn configured_sso_preserves_r1_r2_order_and_never_recalls_identity_asserted() {
    if !t2_available() {
        return;
    }
    // Isolate the process environment rather than racing other tests' issuance.
    const CHILD: &str = "MAOS_17_3D_SSO_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "configured_sso_preserves_r1_r2_order_and_never_recalls_identity_asserted",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("MAOS_SSO_ASSERTION", sso_fixtures::TOKEN_GOOD_RS256)
            .output()
            .unwrap();
        relay_child(&out);
        assert!(out.status.success());
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let config = maos_bin::enterprise_identity::EnterpriseConfig::empty().with_sso_available();
    assert!(config.sso_configured());
    let verifier = maos_sso::OidcVerifier::from_static_jwks(
        sso_fixtures::JWKS_KEY_A,
        &[maos_sso::OidcAlgorithm::Rs256],
        &[sso_fixtures::ISS_GOOD],
        sso_fixtures::AUD_EXPECTED,
    )
    .unwrap();
    let enterprise = Arc::new(
        maos_bin::enterprise_identity::EnterpriseRuntime::from_ports(
            config,
            Some(Arc::new(verifier)),
            None,
            None,
            None,
            Some(dir.path().join("transparency.sqlite")),
            None,
            NONCE,
        ),
    );
    let mut deps = world.session();
    deps.enterprise = Some(enterprise);
    // Story 17-8: live output preserves recalled ids/payloads that claims redact.
    let mut telemetry = world.iac.mailbox().subscribe_telemetry();
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    let seed = input("recall\nfetch self");
    let seed_id = seed.frame_id;
    if t2_result(run_admitted_session(deps, admitted, core, launch(seed, true)).await).is_some() {
        // Story 17-8: only the received seed is recalled, never identity.asserted.
        let seed = world.tl.query_frame_by_id(seed_id).unwrap().unwrap();
        let emitted = next_report(&world, pid, &mut telemetry).await;
        assert_eq!(claim_reports(&world, pid).len(), 1);
        assert_eq!(data(&emitted), format!(
            "recall ok 1 cursor=none\nentry {} {} TaskAssign {} true\nfetch ok {} {} TaskAssign {} {} human-authored\n",
            hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes()),
            hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes()), hex::encode(&seed.payload_redacted)
        ));
        assert_r1_r2(&world, pid);
        let all = rows(&world, pid);
        let r1 = all.iter().position(|r| r.intent == "log.recall").unwrap();
        let r2 = all.iter().position(|r| r.intent == "log.fetch").unwrap();
        assert!(
            all[r1 + 1..r2]
                .iter()
                .any(|r| r.intent == "identity.asserted"),
            "identity.asserted interleaves R1→R2 without entering the recall page"
        );
        let identity = maos_audit::query(
            &dir.path().join("transparency.sqlite"),
            maos_audit::AuditFilter {
                spirit_pid: Some(pid),
                kind: Some("identity.asserted".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            identity.len(),
            3,
            "recall, fetch and emitted-frame send each assert identity"
        );
        for row in identity {
            assert_eq!(row.boot_nonce, NONCE);
            assert_eq!(row.kind, "identity.asserted");
        }
    }
    world.drain().await;
}

async fn hostile_stub(body: Vec<u8>, between_exports: bool) {
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    // Launch-plan seam is the production T2 bridge; this WASM-free child speaks
    // ADR-032 directly, so malformed records reach the actual session dispatcher.
    let octal = body
        .iter()
        .map(|byte| format!("\\{byte:03o}"))
        .collect::<String>();
    let script = format!("printf 'Content-Length: 0\\r\\n\\r\\n'\n{}\nprintf 'Content-Length: {}\\r\\n\\r\\n{octal}'\nwhile IFS= read -r line; do :; done\n",
        if between_exports { ":" } else { "IFS= read -r line" }, body.len());
    let mut plan = launch(input(""), true);
    plan.plan.program = "/bin/sh".into();
    plan.plan.argv = vec!["-c".into(), script];
    plan.component_artifact = None;
    if between_exports {
        plan.initial_frame = None;
        plan.once = false;
    }
    let result = run_admitted_session(world.session(), admitted, core, plan).await;
    assert!(
        matches!(result, Err(SessionError::Protocol(_))),
        "{result:?}"
    );
    let ops = operations(&world, pid);
    assert_eq!(ops.len(), 1);
    assert_row(
        &ops[0],
        pid,
        28,
        "recall.channel.closed:protocol-violation",
        false,
    );
    world.drain().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tagged_record_between_exports_closes_once_with_typed_protocol_end() {
    if !t2_available() {
        return;
    }
    hostile_stub(
        maos_wasm_host::recall_wire::encode_request(
            &maos_wasm_host::recall_wire::RecallRequest::Fetch {
                frame_id: maos_wasm_host::recall_wire::WireBytes(vec![0; 16]),
            },
        )
        .unwrap(),
        true,
    )
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unknown_tag_closes_once_with_typed_protocol_end() {
    if !t2_available() {
        return;
    }
    hostile_stub(vec![0xd9, 0xea, 0x62, 0xf6], false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn junk_after_tag_closes_once_with_typed_protocol_end() {
    if !t2_available() {
        return;
    }
    let mut body = maos_wasm_host::recall_wire::encode_request(
        &maos_wasm_host::recall_wire::RecallRequest::Fetch {
            frame_id: maos_wasm_host::recall_wire::WireBytes(vec![0; 16]),
        },
    )
    .unwrap();
    body.push(0xf6);
    hostile_stub(body, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn checked_in_researcher_admits_both_scopes_and_native_port_rows_pass_fr4() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::new(dir.path());
    let text = include_str!("../../../spirits/researcher/manifest.toml");
    let gated = gate_manifest(text, &|name| name == "researcher").unwrap();
    let (core, admitted) = admit(&world, text).await;
    let pid = admitted.pid;
    let policy = world.policy.inner().load_full();
    let scopes = &policy.manifest_scopes[&pid].scopes;
    assert!(scopes.contains(&Scope::LogRecall));
    assert!(scopes.contains(&Scope::LogFetch));
    drop(policy);
    let recall = maos_bin::recall_mediation::MediatedLogRecall::new(
        Arc::new(maos_iac::adapter::log_recall::LogRecallAdapter::new(
            world.tl.clone(),
        )),
        world.capability.clone(),
        world.security.clone(),
        world.tl.clone(),
        None,
        None,
        "researcher".into(),
    );
    assert!(recall
        .recall(pid, Default::default())
        .unwrap()
        .entries
        .is_empty());
    assert!(fr4(dir.path(), pid).contains("log.recall"));
    let ops = operations(&world, pid);
    assert_eq!(ops.len(), 1);
    assert_row(&ops[0], pid, 7, "log.recall", true);
    drop((recall, admitted, core, gated));
    assert!(!maos_bin::supervision::unload_all_loaded(
        world.scheduler.as_ref(),
        world.halt.as_ref(),
        None
    )
    .await
    .had_failures());
    world.drain().await;
}

#[derive(Debug, Default)]
struct QuotaClock(AtomicU64);

impl maos_kernel_core::capability::cap_quota::QuotaClock for QuotaClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn kernel_quota_rows_and_mcp_refusals_never_cross_the_guest_recall_boundary() {
    use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(QuotaClock::default());
    let quota = CapQuotaTracker::with_clock(
        IssuanceQuota {
            budget_per_window: 8,
            window: Duration::from_secs(1),
        },
        Arc::clone(&clock) as _,
    );
    let world = World::with_tracker(dir.path(), quota.clone(), false);
    let mut telemetry = world.iac.mailbox().subscribe_telemetry();
    let _collector = world
        .iac
        .register_spirit_typed(&SpiritId::from("collector"))
        .unwrap();
    let (core, admitted) = admit(&world, &manifest(true)).await;
    let pid = admitted.pid;
    let reservations: Vec<_> = (0..8).map(|_| quota.charge(pid).unwrap()).collect();
    assert!(matches!(
        world.capability.issue_with_mediation(pid, Scope::LogRecall, 60, [0; 32], IntentClass::Standard),
        Err(maos_domain::ports::capability::CapError::IssuanceQuotaExhausted { spirit_id }) if spirit_id == pid
    ));
    assert!(matches!(
        maos_bin::worker_spawn::issue_governed_mcp_capability(
            &world.capability,
            None,
            None,
            &world.tl,
            pid,
            ID,
            "denied-server",
            "denied-tool",
            [0; 32],
        ),
        Err(maos_bin::worker_spawn::GovernedMintError::KernelPolicyDenied)
    ));
    // The persisted audit row is the synchronization barrier, not elapsed time.
    let private = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let private: Vec<_> = rows(&world, pid)
                .into_iter()
                .filter(|row| {
                    row.intent == "cap.quota:exhausted"
                        || row.intent == "mcp.call.refused:capability-denied"
                })
                .collect();
            if private.len() == 2 {
                break private;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("real quota audit writer persists before the guest turn");
    for row in &private {
        assert_eq!(row.capability_token, None);
        assert_eq!(row.origin, FrameOrigin::Kernel);
    }
    drop(reservations);
    clock.0.store(1_000_000_000, Ordering::SeqCst);
    let script = format!(
        "recall\nfetch {}\nfetch {}",
        hex::encode(private[0].frame_id),
        hex::encode(private[1].frame_id)
    );
    let seed = input(&script);
    let seed_id = seed.frame_id;
    if t2_result(run_admitted_session(world.session(), admitted, core, launch(seed, true)).await)
        .is_some()
    {
        let seed = world.tl.query_frame_by_id(seed_id).unwrap().unwrap();
        let emitted = next_report(&world, pid, &mut telemetry).await;
        assert_eq!(
            data(&emitted),
            format!("recall ok 1 cursor=none\nentry {} {} TaskAssign {} true\nfetch err scope-violation\nfetch err scope-violation\n",
                hex::encode(seed.frame_id), seed.timestamp_ns, hex::encode(seed.intent.as_bytes())),
            "neither operator-only metadata nor either payload reaches the guest"
        );
        let ops = operations(&world, pid);
        assert_eq!(ops.len(), 5);
        assert_row(&ops[0], pid, 7, "log.recall", true);
        for pair in ops[1..].chunks_exact(2) {
            assert_row(&pair[0], pid, 7, "log.fetch", true);
            assert_row(
                &pair[1],
                pid,
                28,
                "log.fetch.refused:scope-violation",
                false,
            );
        }
    }
    drop(_collector);
    world.drain().await;
}
