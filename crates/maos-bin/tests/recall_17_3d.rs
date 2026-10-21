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
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;
const NONCE: u64 = 0x17_03_d;
const ID: &str = "recall-probe";
/// The cargo target directory this test binary was built into
/// (`<target>/debug/deps/recall_17_3d-*`), wherever `CARGO_TARGET_DIR` points.
fn target_root() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.ancestors().nth(3).unwrap().to_owned()
}
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
    target_root().join("recall-probe-guest/wasm32-wasip2/release/recall_probe.wasm")
}
// Recall's larger import/link surface needs more address space than echo.
// The manifest declares 1 GiB for the undelegated lane's RLIMIT_AS fallback;
// this remains a real T2 child, with Landlock and seccomp unchanged.
static FIXTURES: LazyLock<()> = LazyLock::new(|| {
    match std::env::var("MAOS_17_2_CGROUP_MODE").as_deref() {
        Ok("delegated" | "undelegated") => {}
        other => panic!("MAOS_17_2_CGROUP_MODE must be delegated or undelegated: {other:?}"),
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for guest in [true, false] {
        let mut cmd = std::process::Command::new("cargo");
        cmd.current_dir(&workspace)
            .env("CARGO_TARGET_DIR", target_root())
            .args(["build", "--locked", "--release"]);
        if guest {
            cmd.args([
                "--manifest-path",
                "crates/maos-wasm-host/guests/recall-probe/Cargo.toml",
                "--target",
                "wasm32-wasip2",
                "--target-dir",
            ])
            .arg(target_root().join("recall-probe-guest"));
        } else {
            cmd.args(["-p", "maos-wasm-host", "--bin", "maos-wasm-runner"]);
        }
        let out = cmd.output().expect("build real recall fixture");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
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
        Self::with_quota(dir, false)
    }

    fn with_quota(dir: &Path, exhausted: bool) -> Self {
        let seams = maos_bin::supervision::SupervisorSeams::default();
        let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
        if exhausted {
            // Admission allocates pid 1 in a new scheduler. Charge below the
            // exclusive exhaustion threshold; the next issuance must refuse.
            quota.check_and_increment(1, 999_999, 1_000_000).unwrap();
        }
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
            program: target_root()
                .join("release/maos-wasm-runner")
                .to_string_lossy()
                .into_owned(),
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
            let reason = format!("SKIP recall_17_3d: real T2 unavailable: {error}");
            assert!(
                std::env::var_os("CI").is_none() && std::env::var_os("GITHUB_ACTIONS").is_none(),
                "{reason}"
            );
            eprintln!("{reason}");
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
                    std::env::var_os("CI").is_none()
                        && std::env::var_os("GITHUB_ACTIONS").is_none(),
                    "real T2 unavailable on runner declared capable (rule 11(b)): {error}"
                );
                false
            }
            Err(error) => panic!("T2 availability probe failed: {error:?}"),
        }
    });
    if !*AVAILABLE {
        eprintln!("SKIP recall_17_3d: host refuses real T2 (rule 11(b))");
    }
    *AVAILABLE
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
    // Scheduler pid allocation is process-global. A fresh test process makes
    // the pre-construction quota charge target the first actual admitted pid.
    const CHILD: &str = "MAOS_17_3D_QUOTA_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "exhausted_quota_has_receipt_without_tokenless_fallback_call",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        relay_child(&out);
        assert!(out.status.success());
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let world = World::with_quota(dir.path(), true);
    if let Some((pid, report)) = once(&world, true, "recall").await {
        assert_eq!(pid, 1, "pre-charge the actual admitted pid");
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
