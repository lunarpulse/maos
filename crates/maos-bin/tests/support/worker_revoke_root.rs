//! Explicitly token-bearing test root. Production standalone Workers may run
//! under the ratified AC5 host grant with NO kernel token; a revoke proof must
//! prepare the exact spawn scope at the actual bound pid instead.

use super::*;
use maos_bin::operator_door::{BinPrivateOps, OperatorDoor, SuccessorManifestSlot};
use maos_bin::supervision::{
    BindingCore, WorkerBinding, WorkerSupervision, WorkerSupervisionError, WorkerSupervisor,
    WorkerTask,
};
use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_kernel_core::capability::{
    cap_audit::{self, CapAuditWriter},
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable},
    CapabilityRegistryAdapter,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::security::sandbox::t3::{
    child::SandboxedContainerChild, image_lock::VerifiedImageAttestation,
};

const NONCE: u64 = 0x19_02_08;
const ARGV: &[&str] = &["--maos-worker", "--maos-fixture-mode=hang-chatty"];

/// Only policy preparation is test-specific. All lifecycle, launch custody,
/// token binding, observation and stop operations delegate to the real root.
struct GrantedSupervisor {
    inner: Arc<WorkerSupervisor>,
    policy: Arc<PolicyTable>,
    scope: Scope,
    core: Mutex<Option<Arc<BindingCore>>>,
}

impl WorkerSupervision for GrantedSupervisor {
    fn bind(
        &self,
        task: &WorkerTask,
        bundle: SpiritManifestBundle,
    ) -> Result<WorkerBinding, WorkerSupervisionError> {
        let binding = self.inner.bind(task, bundle)?;
        let pid = binding.spirit_pid();
        // Same explicit preparation as cli_wrapper_exhaustion_17_9, but use
        // the allocated pid, never an assumed pid shared by concurrent lanes.
        self.policy.inner().rcu(|old| {
            let mut next = (**old).clone();
            next.manifest_scopes.insert(
                pid,
                ManifestCapabilityScope {
                    scopes: vec![self.scope.clone()],
                    declared_tier: maos_domain::invariants::i9::SandboxTier::T3,
                    trust_tier: TrustTier::Verified,
                },
            );
            Arc::new(next)
        });
        *self.core.lock().expect("bound core") = Some(Arc::clone(binding.core()));
        Ok(binding)
    }

    fn create_t3_state_dir(&self) -> Result<PathBuf, String> {
        self.inner.create_t3_state_dir()
    }

    fn payload_pidfd(&self) -> bool {
        self.inner.payload_pidfd()
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

    fn bind_token(&self, core: &Arc<BindingCore>, token: TokenId, deadline: u64) {
        self.inner.bind_token(core, token, deadline);
    }

    fn is_stopping(&self) -> bool {
        self.inner.is_stopping()
    }
}

/// These unrelated bin-private routes must never be reached by revoke-token.
struct RevokeOnlyOps;

#[async_trait::async_trait]
impl BinPrivateOps for RevokeOnlyOps {
    async fn append_lifecycle_journal(
        &self,
        _: maos_domain::invariants::i10::LifecycleEvent,
        _: &str,
    ) -> Result<PathBuf, String> {
        unreachable!("revoke-token does not use bin-private lifecycle operations")
    }

    async fn run_uninstall_cascade(&self, _: &str) -> maos_bin::operator_door::UninstallOutcome {
        unreachable!("revoke-token does not uninstall")
    }

    async fn enforce_vetted_upgrade_precondition(
        &self,
        _: &str,
        _: &Path,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<(), String> {
        unreachable!("revoke-token does not upgrade")
    }

    async fn upgrade_with_plan_guard(
        &self,
        _: &str,
        _: &Path,
        _: &Path,
        _: maos_kernel_core::lifecycle::UpgradePolicy,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<serde_json::Value, maos_bin::operator_door::UpgradeFailure> {
        unreachable!("revoke-token does not upgrade")
    }

    fn admitted_manifest(&self, _: &str) -> Option<Arc<String>> {
        unreachable!("revoke-token does not upgrade")
    }

    fn record_upgraded_manifest(&self, _: &str, _: &str) {
        unreachable!("revoke-token does not upgrade")
    }

    fn migration_plan_pending(&self, _: &str) -> bool {
        unreachable!("revoke-token does not upgrade")
    }

    async fn create_migration_plan(
        &self,
        _: &str,
        _: &str,
        _: &Path,
        _: &[String],
    ) -> Result<serde_json::Value, String> {
        unreachable!("revoke-token does not plan migrations")
    }

    async fn load_spirit(
        &self,
        _: &Path,
        _: Option<Arc<String>>,
    ) -> Result<maos_bin::admission::Admitted, maos_bin::admission::AdmissionRefusal> {
        unreachable!("revoke-token does not load Spirits")
    }
}

struct RevokeRoot {
    root: WorkerRoot,
    runtime: tokio::runtime::Runtime,
    supervision: Arc<GrantedSupervisor>,
    capability: Arc<CapabilityRegistryAdapter>,
    log: Arc<TransparencyLogAdapter>,
    door: Arc<OperatorDoor>,
    server: maos_control::OperatorHttpServer,
    worker: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl RevokeRoot {
    fn new(home: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("revoke root runtime");
        let entered = runtime.enter();
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        std::fs::create_dir_all(home.join("audit")).expect("revoke audit directory");
        let log = Arc::new(
            TransparencyLogAdapter::open(&home.join("audit/transparency.sqlite"), NONCE)
                .expect("revoke transparency log"),
        );
        let policy = Arc::new(PolicyTable::new());
        let (audit_tx, audit_rx) = cap_audit::channel();
        CapAuditWriter::spawn(audit_rx, Arc::clone(&log));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x19; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
        ));
        let memory_db = home.join("memory.sqlite");
        let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                home.join("private"),
                4,
            )),
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                    .expect("shared memory"),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .expect("principal memory"),
            ),
            Arc::clone(&log),
        ));
        let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::clone(&mailbox),
            Arc::clone(&log),
        ));
        let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
            Arc::clone(&log),
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
            maos_kernel_core::journal::JournalAdapter::open(&home.join("lifecycle.ndjson"))
                .expect("revoke lifecycle journal"),
        );
        let detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&log),
            Arc::clone(&halt),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&metrics),
            Arc::clone(&journal),
        ));
        Arc::get_mut(&mut scheduler)
            .expect("install detector before scheduler clones")
            .set_crash_detector(Arc::clone(&detector));
        let supervisor = Arc::new(WorkerSupervisor::new(
            Arc::clone(&scheduler),
            detector,
            Arc::clone(&log),
            Arc::clone(&journal),
            Arc::clone(&halt),
            Arc::clone(&iac),
            runtime.handle().clone(),
            NONCE,
            tokio_util::sync::CancellationToken::new(),
        ));
        let door = Arc::new(OperatorDoor::new(
            Arc::clone(&scheduler),
            Arc::clone(&policy),
            Arc::clone(&halt),
            Arc::new(maos_kernel_core::halt::OutputMarkerRegistry::new()),
            Arc::new(maos_kernel_core::orchestrator::OrchestratorBufferRegistry::new()),
            Arc::new(maos_kernel_core::revocation::RevocationApplier::new(
                scheduler.scbs(),
                Arc::clone(&capability),
                Arc::clone(&scheduler),
                Arc::clone(&iac),
                Arc::clone(&halt),
                Arc::clone(&log),
                Arc::clone(&journal),
                Arc::clone(&metrics),
            )),
            Arc::new(maos_kernel_core::hot_swap::HotSwapCoordinator::new(
                scheduler.scbs(),
                Arc::clone(&journal),
                Arc::clone(&log),
                Arc::clone(&halt),
                Arc::clone(&capability),
                iac,
                Arc::new(maos_kernel_core::scheduler::HookDispatcher::new(
                    Arc::clone(&log),
                    Arc::clone(&metrics),
                )),
                metrics,
                home.join("hot-swap-archive"),
            )),
            Arc::clone(&capability),
            memory,
            Arc::new(
                maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
                    Arc::clone(&capability), halt,
                ),
            ),
            mailbox,
            Arc::new(maos_director_surface::notification::NotificationDispatcher::new()),
            Arc::clone(&log),
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            None,
            NONCE,
            runtime.handle().clone(),
            Arc::new(RevokeOnlyOps),
            SuccessorManifestSlot::default(),
        ));
        supervisor.install_door_hooks(&door);
        let token = maos_domain::operator_door::mint_token_hex();
        let server = maos_control::OperatorHttpServer::bind_with_commands(
            maos_control::OperatorHttpConfig {
                bind: "127.0.0.1:0".parse().expect("loopback"),
                bearer_token: token.clone(),
            },
            scheduler,
            None,
            None,
            None,
            Some(door.clone()),
        )
        .expect("real operator HTTP server");
        maos_domain::operator_door::ControlFile {
            version: maos_domain::operator_door::CONTROL_FILE_VERSION,
            endpoint: format!("tcp://{}", server.local_addr()),
            token,
        }
        .write_atomic(&home.join("control.json"))
        .expect("test root discovery");
        let scope = Scope::CliSubprocessSpawn {
            cli_binary_path: maos_bin::worker_spawn::resolve_cli_binary("worker-cli-fixture")
                .expect("Worker fixture"),
            argv_prefix_hash: maos_kernel_core::lifecycle::cli_wrapper::argv_prefix_hash(
                &ARGV.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
            ),
            output_shape_version: "1.0.0".to_owned(),
        };
        drop(entered);
        Self {
            root: WorkerRoot {
                home,
                child: None,
                stdout: Arc::new(Mutex::new(Vec::new())),
                stderr: Arc::new(Mutex::new(String::new())),
            },
            runtime,
            supervision: Arc::new(GrantedSupervisor {
                inner: supervisor,
                policy,
                scope,
                core: Mutex::new(None),
            }),
            capability,
            log,
            door,
            server,
            worker: None,
        }
    }

    fn launch(&mut self) {
        let log = Arc::clone(&self.log);
        let capability = Arc::clone(&self.capability);
        let supervision = Arc::clone(&self.supervision);
        let handle = self.runtime.handle().clone();
        self.worker = Some(std::thread::spawn(move || {
            let _entered = handle.enter();
            let manifest = toml::from_str(
                "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
                 argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang-chatty\"]\n\
                 output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
                 recovery_policy = \"respawn_fresh\"\n[cli_wrapper.posture]\n\
                 stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
                 shutdown_signal = \"SIGTERM\"\n[sandbox]\ntier = \"T3\"\n\
                 [author]\nname = \"MAOS Project\"\n",
            )
            .expect("T3 chatty Worker manifest");
            maos_bin::worker_spawn::run_cli_wrapper_manifest(
                &manifest,
                &maos_bin::worker_spawn::RunArgs {
                    manifest_path: "revoke-worker.toml".into(),
                    live: false,
                    once: true,
                    scene: None,
                    epic: None,
                },
                log,
                capability,
                Arc::clone(&supervision.policy),
                None,
                None,
                None,
                None,
                false,
                supervision.as_ref(),
                WorkerTask::Standalone,
                None,
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
        }));
        let deadline = Instant::now() + STARTUP_BOUND;
        while self.root.heartbeats().is_empty() {
            if self.worker.as_ref().expect("worker thread").is_finished() {
                let result = self.worker.take().unwrap().join();
                panic!("Worker exited before its first heartbeat: {result:?}");
            }
            assert!(
                Instant::now() < deadline,
                "Worker startup exceeded {STARTUP_BOUND:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn join_worker(&mut self) {
        let worker = self.worker.as_ref().expect("worker thread");
        let deadline = Instant::now() + EXIT_BOUND;
        while !worker.is_finished() {
            assert!(
                Instant::now() < deadline,
                "Worker exit exceeded {EXIT_BOUND:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("Worker panicked")
            .expect("Worker finalization");
    }
}

impl Drop for RevokeRoot {
    fn drop(&mut self) {
        self.supervision.inner.stop_workers();
        self.server.shutdown();
        self.runtime.block_on(self.door.drain_started_tasks());
        if self.worker.is_some() {
            self.join_worker();
        }
    }
}

pub(super) fn revoke_once(home: PathBuf, tag: &str) -> Duration {
    let mut root = RevokeRoot::new(home);
    root.launch();
    let worker_pid = root.root.heartbeats()[0].2;
    let token = root.root.spawn_token_hex(worker_pid);
    let core = root
        .supervision
        .core
        .lock()
        .expect("bound core")
        .clone()
        .expect("Worker bound");
    assert_eq!(i64::from(core.spirit_pid()), worker_pid);
    let terminal = core.container_terminal().expect("live T3 container");
    let container = terminal.container_id();

    let posted = Instant::now();
    let body = root.root.ctl_json(&["revoke-token", &token]);
    assert_eq!(body["revoked"], true, "{tag}: {body}");
    assert_eq!(
        body["worker_stopped"], "worker",
        "{tag}: the binding was stopped: {body}"
    );
    wait_container_gone(container, TEARDOWN_BUDGET.saturating_sub(posted.elapsed()));
    let latency = posted.elapsed();
    assert!(core.termination_confirmed(), "{tag}: terminal confirmation");
    root.join_worker();
    latency
}

/// Deterministic, no-substrate regression: no grant means no token or issue
/// row; after preparation the real governed mint journals at the real pid,
/// including the exact token id consumed by the revoke route.
pub(super) fn assert_mint_journal() {
    let home = tempfile::tempdir().expect("mint regression home");
    let root = RevokeRoot::new(home.path().join("root"));
    let denied = maos_bin::worker_spawn::issue_enterprise_governed_capability(
        &root.capability,
        None,
        None,
        0,
        root.supervision.scope.clone(),
        300,
        [0; 32],
        IntentClass::Standard,
    );
    assert!(matches!(
        denied,
        Err(maos_bin::worker_spawn::GovernedMintError::KernelPolicyDenied)
    ));
    let binding = root
        .supervision
        .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
        .expect("bind real Worker");
    let pid = binding.spirit_pid();
    assert_ne!(pid, 0);
    let token = maos_bin::worker_spawn::issue_enterprise_governed_capability(
        &root.capability,
        None,
        None,
        pid,
        root.supervision.scope.clone(),
        300,
        [0; 32],
        IntentClass::Standard,
    )
    .expect("prepared scope must mint");
    root.supervision
        .bind_token(binding.core(), token.token_id, u64::MAX);
    assert_eq!(
        root.root.spawn_token_hex(i64::from(pid)),
        hex::encode(token.token_id.0)
    );
    assert!(!root.root.rows("SELECT timestamp_ns, spirit_pid, intent, capability_token, payload_redacted FROM transparency_log WHERE intent LIKE 'cap.issue%'").iter().any(|row| row.1 == 0));
    let body = root
        .root
        .ctl_json(&["revoke-token", &hex::encode(token.token_id.0)]);
    assert_eq!(body["revoked"], true, "{body}");
    assert_eq!(body["worker_stopped"], "worker", "{body}");
    assert_eq!(
        binding.core().phase(),
        maos_bin::supervision::BindingPhase::PlannedStop
    );
    assert!(!root.capability.has_active_tokens_for_pid(pid));
    drop(binding);
}
