#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 17-9 AC5.5 / FW-06 A: real CLI admission and kernel-backed binding,
//! with no workload launch after quota refusal. Admission's host-side bridge
//! probe is deliberately allowed; it is not a supervised workload launch.
//! The shared T3 prerequisite names local skips and fails closed in CI.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use maos_bin::supervision::{
    BindingCore, BindingPhase, WorkerBinding, WorkerSupervision, WorkerSupervisionError,
    WorkerSupervisor, WorkerTask,
};
use maos_bin::worker_spawn::{
    issue_enterprise_governed_capability, resolve_cli_binary, run_cli_wrapper_manifest,
    GovernedMintError, RunArgs,
};
use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable},
    cap_quota::{CapQuotaTracker, IssuanceQuota},
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::security::sandbox::t3::{
    child::SandboxedContainerChild, image_lock::VerifiedImageAttestation,
};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const NONCE: u64 = 0x17_09_D;
const PREPARED_PID: u32 = 1;
const CHILD: &str = "MAOS_17_9_CLI_QUOTA_CHILD";
const REMOTE_MARKER: &str = "CLI-QUOTA-17-9 remote exhaustion refused before workload";
const LOCAL_MARKER: &str = "CLI-QUOTA-17-9 local exhaustion refused before workload";
const D1_MARKER: &str = "CLI-QUOTA-17-9 absent grant D1 completed without token";

#[derive(Debug, Default)]
struct ManualClock(AtomicU64);

impl maos_kernel_core::capability::cap_quota::QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn tracker() -> CapQuotaTracker {
    CapQuotaTracker::with_clock(
        IssuanceQuota {
            budget_per_window: 1,
            window: Duration::from_secs(3600),
        },
        Arc::new(ManualClock::default()),
    )
}

fn registry(policy: Arc<PolicyTable>, quota: CapQuotaTracker) -> CapabilityRegistryAdapter {
    CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        Ed25519SigningKey::new([0x79; 32]),
        NONCE,
        policy,
        cap_audit::channel().0,
        quota,
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
    )
}

fn spawn_scope(program: &str) -> Scope {
    Scope::CliSubprocessSpawn {
        cli_binary_path: program.to_owned(),
        argv_prefix_hash: maos_kernel_core::lifecycle::cli_wrapper::argv_prefix_hash(&[
            "--maos-worker".to_owned(),
        ]),
        output_shape_version: "1.0.0".to_owned(),
    }
}

fn grant(policy: &PolicyTable, scope: Scope) {
    policy.inner().rcu(|old| {
        let mut next = (**old).clone();
        next.manifest_scopes.insert(
            PREPARED_PID,
            ManifestCapabilityScope {
                scopes: vec![scope.clone()],
                declared_tier: SandboxTier::T3,
                trust_tier: TrustTier::Verified,
            },
        );
        Arc::new(next)
    });
}

/// Always runnable, including off-substrate: policy denial must still win over
/// an already-spent quota. This is the actual mint result D1 handles with None,
/// not a fabricated Mediation error or a grant accidentally left in the fixture.
#[test]
fn absent_grant_is_policy_denied_even_with_spent_quota() {
    let policy = Arc::new(PolicyTable::new());
    let quota = tracker();
    let capability = registry(Arc::clone(&policy), quota.clone());
    let _reserved = quota.charge(PREPARED_PID).expect("reserve the only slot");
    let result = issue_enterprise_governed_capability(
        &capability,
        None,
        None,
        PREPARED_PID,
        spawn_scope("/not-launched/worker-cli-fixture"),
        300,
        [0; 32],
        IntentClass::Standard,
    );
    assert!(
        matches!(result, Err(GovernedMintError::KernelPolicyDenied)),
        "D1's absent grant must remain KernelPolicyDenied, not quota exhaustion: {result:?}"
    );
    assert!(policy.inner().load().manifest_scopes.is_empty());
    println!("CLI-QUOTA-17-9 absent grant policy denial precedes quota");
}

/// Run each scheduler-backed case in a fresh process: allocation is global,
/// so PREPARED_PID is the actual first Worker, independent of test ordering.
/// Require the child's marker AND exactly one executed test (no green zero-test
/// process). Pass markers are emitted only after all case assertions succeed.
fn in_fresh_child(name: &str, marker: &str) -> bool {
    // `require_t3` prints the one `SKIP:` line when the substrate is absent.
    if !t3_substrate::require_t3(name) {
        return false;
    }
    if let Some(child) = std::env::var_os(CHILD) {
        assert_eq!(child, name, "fresh child must execute its selected case");
        return true;
    }
    let dir = tempfile::tempdir().expect("isolated child home");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("create child home");
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .env("HOME", t3_substrate::isolated_runtime_home(&home))
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", &home)
        .env_remove("MAOS_LIVE_AGENT")
        .output()
        .expect("run isolated CLI quota case");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Relay prefixed: the child's own `test result:` summary must never be
    // counted as a second suite by the CI log checker.
    for line in stdout.lines() {
        println!("[child {name}] {line}");
    }
    for line in stderr.lines() {
        eprintln!("[child {name}] {line}");
    }
    assert!(output.status.success(), "fresh child failed: {stderr}");
    assert!(
        stdout.contains("test result: ok. 1 passed;"),
        "fresh child must execute exactly one test: {stdout}"
    );
    assert_eq!(
        stdout.matches(marker).count(),
        1,
        "missing case proof marker"
    );
    false
}

struct World {
    log: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    policy: Arc<PolicyTable>,
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    supervision: ObservedSupervisor,
}

impl World {
    fn new(dir: &Path, quota: CapQuotaTracker) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
        let policy = Arc::new(PolicyTable::new());
        let capability = Arc::new(registry(Arc::clone(&policy), quota));
        let memory_db = dir.join("memory.sqlite");
        let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                    .expect("open shared memory"),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .expect("open principal index"),
            ),
            Arc::clone(&log),
        ));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::mailbox::Mailbox::new(Arc::clone(
                &metrics,
            ))),
            Arc::clone(&log),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
            Arc::clone(&log),
            Arc::clone(&capability),
            memory,
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
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson"))
                .expect("open lifecycle journal"),
        );
        let crash_detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&log),
            Arc::clone(&halt_registry),
            Arc::clone(&capability),
            Arc::clone(&iac),
            metrics,
            Arc::clone(&journal),
        ));
        Arc::get_mut(&mut scheduler)
            .expect("scheduler has no clones before detector installation")
            .set_crash_detector(Arc::clone(&crash_detector));
        let inner = WorkerSupervisor::new(
            Arc::clone(&scheduler),
            crash_detector,
            Arc::clone(&log),
            journal,
            halt_registry,
            iac,
            tokio::runtime::Handle::current(),
            NONCE,
            tokio_util::sync::CancellationToken::new(),
        );
        Self {
            log,
            capability,
            policy,
            scheduler,
            supervision: ObservedSupervisor {
                inner,
                launch_entries: AtomicUsize::new(0),
                token_binds: AtomicUsize::new(0),
            },
        }
    }

    fn run(&self, remote: bool) -> Result<maos_bin::worker_cli::WorkerCompletion, String> {
        let manifest: toml::Value = toml::from_str(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\"]\noutput_shape_version = \"1.0.0\"\n\
             skill_bundle = [\"maos-bridge\"]\nrecovery_policy = \"respawn_fresh\"\n\
             [cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\n\
             control_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\
             [sandbox]\ntier = \"T3\"\n[author]\nname = \"MAOS Project\"\n",
        )
        .expect("valid T3 manifest");
        tokio::task::block_in_place(|| {
            run_cli_wrapper_manifest(
                &manifest,
                &RunArgs {
                    manifest_path: "cli-quota-17-9.toml".into(),
                    live: false,
                    once: true,
                },
                Arc::clone(&self.log),
                Arc::clone(&self.capability),
                Arc::clone(&self.policy),
                None,
                None,
                None,
                remote.then_some("quota refusal must not launch this delegated task"),
                remote,
                &self.supervision,
                if remote {
                    WorkerTask::Delegation {
                        frame_id: [0x79; 16],
                    }
                } else {
                    WorkerTask::Standalone
                },
                None,
            )
            .map_err(|error| error.to_string())
        })
    }
}

/// A transparent observer, NOT a stub supervisor: binding, lifecycle, teardown,
/// token binding, OCI launch and exit handling all delegate to the real adapter.
/// The state-directory boundary is entered before any workload OCI operation;
/// counting it detects even a launch that fails before output/exit journaling.
/// Admission probes precede binding and never enter this boundary.
struct ObservedSupervisor {
    inner: WorkerSupervisor,
    launch_entries: AtomicUsize,
    token_binds: AtomicUsize,
}

impl WorkerSupervision for ObservedSupervisor {
    fn create_t3_state_dir(&self) -> Result<PathBuf, String> {
        self.launch_entries.fetch_add(1, Ordering::SeqCst);
        self.inner.create_t3_state_dir()
    }

    fn payload_pidfd(&self) -> bool {
        self.inner.payload_pidfd()
    }

    fn bind(
        &self,
        task: &WorkerTask,
        bundle: SpiritManifestBundle,
    ) -> Result<WorkerBinding, WorkerSupervisionError> {
        self.inner.bind(task, bundle)
    }

    fn watch(&self, core: &Arc<BindingCore>, child_pid: u32) {
        self.inner.watch(core, child_pid);
    }

    fn commit_container_spawn(
        &self,
        core: &Arc<BindingCore>,
        child: &SandboxedContainerChild,
        image: &VerifiedImageAttestation,
    ) -> Result<(), WorkerSupervisionError> {
        self.inner.commit_container_spawn(core, child, image)
    }

    fn bind_token(&self, core: &Arc<BindingCore>, token_id: TokenId, ttl_deadline_ns: u64) {
        self.token_binds.fetch_add(1, Ordering::SeqCst);
        self.inner.bind_token(core, token_id, ttl_deadline_ns);
    }

    fn is_stopping(&self) -> bool {
        self.inner.is_stopping()
    }

    fn after_mint(&self) {
        self.inner.after_mint();
    }

    fn error_after_watch(&self) -> Result<(), String> {
        self.inner.error_after_watch()
    }
}

async fn exhaustion_case(remote: bool) {
    let dir = tempfile::tempdir().expect("isolated kernel world");
    let quota = tracker();
    let world = World::new(dir.path(), quota.clone());
    let program = resolve_cli_binary("worker-cli-fixture")
        .expect("build worker-cli-fixture into CARGO_TARGET_DIR before running this suite");
    grant(&world.policy, spawn_scope(&program));
    let _reserved = quota.charge(PREPARED_PID).expect("exhaust prepared Worker");
    let result = world.run(remote);
    let bindings = world.supervision.inner.bindings();
    assert_eq!(bindings.len(), 1, "refusal must occur AFTER real binding");
    let binding = &bindings[0];
    assert_eq!(
        binding.spirit_pid, PREPARED_PID,
        "exhaust the actual bound pid"
    );
    assert!(
        matches!(&result, Err(error) if error.contains("cli_wrapper spawn REFUSED")
            && error.contains("kernel capability issuance quota exhausted")),
        "quota exhaustion must refuse before workload launch on remote={remote}: {result:?}"
    );
    assert_eq!(binding.phase, BindingPhase::PlannedStop);
    assert_eq!(
        binding.child_pid, None,
        "admission probe is not the workload"
    );
    assert_eq!(
        world.supervision.launch_entries.load(Ordering::SeqCst),
        0,
        "no workload launch boundary after quota refusal; admission probe is allowed"
    );
    assert_eq!(world.supervision.token_binds.load(Ordering::SeqCst), 0);
    assert!(
        world.scheduler.resolve_pid(&binding.spirit_id).is_none(),
        "guard unloads bound SCB"
    );
    let rows = world
        .log
        .query_frames(FrameFilter::default())
        .expect("read durable refusal evidence");
    assert_eq!(
        rows.iter()
            .filter(|row| row.spirit_pid == PREPARED_PID && row.intent == "lifecycle.load")
            .count(),
        1,
        "real admission and bind succeeded"
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.spirit_pid == PREPARED_PID && row.intent == "lifecycle.unload")
            .count(),
        1,
        "post-bind guard unloads exactly once"
    );
    // Termination writes an empty marker row followed by the serialized
    // receipt, with the same intent. Assert the receipt, not two intent rows.
    let receipts: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.spirit_pid == PREPARED_PID
                && row.kind == FrameKind::EpistemicHalt
                && row.intent == "planned_unload"
                && !row.payload_redacted.is_empty()
        })
        .map(|row| {
            serde_json::from_slice::<maos_domain::halt::HaltReceipt>(&row.payload_redacted)
                .expect("PlannedUnload receipt decodes")
        })
        .collect();
    assert_eq!(
        receipts.len(),
        1,
        "guard persists one PlannedUnload receipt"
    );
    assert_eq!(receipts[0].spirit_pid, PREPARED_PID);
    assert_eq!(receipts[0].boot_nonce, NONCE);
    assert_eq!(
        receipts[0].terminal_state,
        Some(maos_domain::halt::HaltState::Terminated)
    );
    assert_eq!(
        receipts[0].resolution_kind.as_deref(),
        Some("planned_unload")
    );
    // NACKs are written at pid 0 for the task's originator, not at the dead
    // Worker's pid. Join the disposition to this binding by its task record.
    let nacks: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == "task.nacked")
        .collect();
    assert_eq!(nacks.len(), 1, "guard applies the stop disposition");
    let nack: serde_json::Value =
        serde_json::from_slice(&nacks[0].payload_redacted).expect("task NACK payload");
    let task = if remote {
        WorkerTask::Delegation {
            frame_id: [0x79; 16],
        }
    } else {
        WorkerTask::Standalone
    };
    let (task_id, originator) = task.record_ids(&binding.spirit_id);
    assert_eq!(nack["task_id"], task_id);
    assert_eq!(nack["originator_spirit_id"], originator);
    assert!(nack["capability_token"].is_null());
    assert!(
        !rows.iter().any(|row| row.intent == "cli.subprocess.exit"),
        "no workload exit row after refusal"
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.kind == FrameKind::CliSubprocessOutput),
        "pre-mint admission probe must not be confused with workload output"
    );
    println!("{}", if remote { REMOTE_MARKER } else { LOCAL_MARKER });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn remote_exhaustion_refuses_before_workload() {
    if in_fresh_child("remote_exhaustion_refuses_before_workload", REMOTE_MARKER) {
        exhaustion_case(true).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_exhaustion_refuses_before_workload() {
    if in_fresh_child("local_exhaustion_refuses_before_workload", LOCAL_MARKER) {
        exhaustion_case(false).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn absent_grant_keeps_d1_tokenless_proceed_posture() {
    if !in_fresh_child("absent_grant_keeps_d1_tokenless_proceed_posture", D1_MARKER) {
        return;
    }
    let dir = tempfile::tempdir().expect("isolated D1 kernel world");
    let quota = tracker();
    let world = World::new(dir.path(), quota.clone());
    // Exhaustion must not turn an absent grant into a quota refusal: policy
    // denies first, and D1 still proceeds under the existing host grant.
    let _reserved = quota.charge(PREPARED_PID).expect("exhaust D1 control pid");
    assert!(world.policy.inner().load().manifest_scopes.is_empty());
    let completion = world
        .run(true)
        .expect("D1 absent grant proceeds on the remote path");
    assert!(
        completion.is_completed(),
        "D1 workload actually completes: {completion:?}"
    );
    let bindings = world.supervision.inner.bindings();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].spirit_pid, PREPARED_PID);
    assert_eq!(bindings[0].phase, BindingPhase::ExitedClean);
    assert!(
        bindings[0].child_pid.is_some(),
        "real workload was supervised"
    );
    assert_eq!(world.supervision.launch_entries.load(Ordering::SeqCst), 1);
    assert_eq!(
        world.supervision.token_binds.load(Ordering::SeqCst),
        0,
        "D1 proceeds with None, not a token"
    );
    let rows = world
        .log
        .query_frames(FrameFilter::default())
        .expect("read D1 workload evidence");
    let exits: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(
        exits.len(),
        1,
        "one completed workload, excluding admission probe"
    );
    assert_eq!(exits[0].spirit_pid, PREPARED_PID);
    assert!(exits[0].capability_token.is_none(), "D1 exit is tokenless");
    assert!(
        rows.iter().any(|row| row.spirit_pid == PREPARED_PID
            && row.kind == FrameKind::CliSubprocessOutput),
        "real workload output exists"
    );
    println!("{D1_MARKER}");
}
