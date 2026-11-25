//! Story 22-8 — the native directed leg, in-process: the real `DirectedTasks`, the real Mira,
//! a fake Inference Port, the real `HaltRegistry` (through the kernel's own `invoke_halt`), a
//! real Transparency Log and a real pid-bound memory. A keyless boot of the default cast
//! (Nash with a declared provider, the mode unset) is the one process witness.
#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

use maos_bin::directed_task::{
    halt_for_task, AcceptError, DirectedTasks, SpiritRunState, TaskHalt, TaskHost, TaskLimits,
    TaskLog, TaskRecord, TaskState,
};
use maos_bin::native_task::{MiraRunner, PidBoundDiagnosisMemory, PidSource};
use maos_domain::halt::HaltReceipt;
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::EpistemicScalarPort;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PID: u32 = 7;
const WAIT: Duration = Duration::from_secs(20);
const KNOWN: &str =
    r#"{"subject":"payments-api","metric":"error_rate","observed":0.42,"baseline":0.02}"#;
const SEVERE_UNKNOWN: &str =
    r#"{"subject":"ledger-sync","metric":"queue_skew","observed":9.0,"baseline":1.0}"#;

/// The kernel's own halt path behind the scalar port: what `ButlerOrchestratorAdapter` reaches
/// through the orchestrator, minus the policy evaluation (Mira's handler decides the boundary).
struct HaltingScalar {
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    registry: Arc<HaltRegistry>,
}

impl EpistemicScalarPort for HaltingScalar {
    fn write_scalar(
        &self,
        _pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        let payload = maos_domain::frame::EpistemicHaltPayload::new(
            ulid::Ulid::new().to_string(),
            tag.to_owned(),
            value as f32,
            Some(0.5),
            "mira.diagnostic_confidence".to_owned(),
            derived_from.to_owned(),
        )
        .map_err(|e| ScalarPortError::Backend(e.to_string()))?;
        invoke_halt(
            &self.tl,
            &self.journal,
            &self.registry,
            payload,
            PID,
            spirit_id,
            1,
        )
        .map(Some)
        .map_err(|e| ScalarPortError::Backend(e.to_string()))
    }
}

struct FakeInference {
    reply: String,
    calls: Mutex<u32>,
}

impl InferencePort for FakeInference {
    fn complete(&self, _request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        *self.calls.lock().unwrap() += 1;
        Ok(InferenceResponse {
            text: self.reply.clone(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "fake".into(),
                endpoint_url: "fake://".into(),
                model_id: None,
            },
        })
    }
}

/// The registry's kernel seam over a real Transparency Log: every row a real `TaskLog` writes,
/// a Spirit that is always Running, and the real halt correlation.
struct LogHost {
    log: TaskLog,
    registry: Arc<HaltRegistry>,
}

impl TaskHost for LogHost {
    fn spirit_state(&self, _spirit_id: &str) -> SpiritRunState {
        SpiritRunState::Running
    }

    fn now_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn write_accepted(&self, record: &TaskRecord) -> Result<(), String> {
        self.log.write_accepted(record, PID);
        Ok(())
    }

    fn write_terminal(&self, record: &TaskRecord) -> Result<(), String> {
        self.log.write_terminal(record, PID);
        Ok(())
    }

    fn reply_result(&self, task_id: &str) -> Option<(String, u64)> {
        self.log.reply_result(task_id)
    }

    fn orphan_row(&self, task_id: &str) -> bool {
        self.log.orphan_row(task_id)
    }

    fn halt_since(&self, _spirit_id: &str, since_ns: u64) -> Option<TaskHalt> {
        halt_for_task(&self.registry, PID, since_ns)
    }

    fn find_key(&self, key_hash: &str) -> Option<String> {
        self.log.find_key(key_hash)
    }

    fn read_back(&self, task_id: &str) -> Option<TaskRecord> {
        self.log.read_back(task_id)
    }
}

struct Kernel {
    _dir: tempfile::TempDir,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    registry: Arc<HaltRegistry>,
    memory: Arc<MemoryManagerAdapter>,
}

impl Kernel {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("audit.sqlite");
        let tl =
            Arc::new(TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, 1).unwrap());
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(dir.path().join("memory"), 4 * 1024)),
            Arc::new(SharedMemoryStore::open(&db).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
            Arc::clone(&tl),
        ));
        let journal = Arc::new(JournalAdapter::open(&dir.path().join("lifecycle.ndjson")).unwrap());
        Self {
            _dir: dir,
            tl,
            journal,
            registry: Arc::new(HaltRegistry::new()),
            memory,
        }
    }

    fn pid_of() -> PidSource {
        Arc::new(|| Some(PID))
    }

    fn mira_runner(&self, inference: Option<Arc<dyn InferencePort + Send + Sync>>) -> MiraRunner {
        let mira = mira::Mira::default()
            .with_id("mira")
            .with_scalar_port(Arc::new(HaltingScalar {
                tl: Arc::clone(&self.tl),
                journal: Arc::clone(&self.journal),
                registry: Arc::clone(&self.registry),
            }));
        MiraRunner::new(
            mira,
            inference,
            PidBoundDiagnosisMemory::new(Arc::downgrade(&self.memory), Self::pid_of()),
            None,
            Arc::clone(&self.tl),
            Self::pid_of(),
        )
    }

    /// A fresh registry over the same log: what a restarted Host reads.
    fn tasks(&self) -> Arc<DirectedTasks> {
        DirectedTasks::new(
            Arc::new(LogHost {
                log: TaskLog::new(Arc::clone(&self.tl)),
                registry: Arc::clone(&self.registry),
            }),
            TaskLimits::default(),
        )
    }
}

fn finished(tasks: &DirectedTasks, id: &str) -> TaskRecord {
    let until = Instant::now() + WAIT;
    loop {
        if let Some(record) = tasks.get(id) {
            if !matches!(record.state, TaskState::Queued | TaskState::Running) {
                return record;
            }
        }
        assert!(Instant::now() < until, "task {id} never ended");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn mira_answers_a_directed_task_and_its_reply_row_is_journaled() {
    let kernel = Kernel::new();
    let fake = Arc::new(FakeInference {
        reply: KNOWN.into(),
        calls: Mutex::new(0),
    });
    let tasks = kernel.tasks();
    tasks.register("mira", Arc::new(kernel.mira_runner(Some(fake.clone()))));

    // Free text goes through the fake provider once; the answer carries the deterministic
    // diagnosis, the memory count and the (unrouted) advisory verdict.
    let accepted = tasks
        .accept(
            "mira",
            "payments-api is throwing errors since the deploy",
            None,
            None,
        )
        .expect("mira is directable");
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Completed, "{record:?}");
    let answer: serde_json::Value =
        serde_json::from_str(record.result.as_deref().unwrap()).unwrap();
    assert_eq!(answer["subject"], "payments-api");
    assert_eq!(answer["advisory"], "not_routed");
    assert_eq!(answer["prior_diagnoses"], 0);
    for field in ["finding", "severity", "confidence"] {
        assert!(answer.get(field).is_some(), "{field} in {answer}");
    }
    assert_eq!(*fake.calls.lock().unwrap(), 1);

    // The reply row is in the log before `completed`, and a registry that never ran the task
    // (a restarted Host) reads the same bytes back from it.
    let row = TaskLog::new(Arc::clone(&kernel.tl))
        .reply_result(&accepted.task_id)
        .expect("the reply row precedes the answer");
    assert_eq!(Some(row.0.as_str()), record.result.as_deref());
    let restarted = kernel.tasks();
    let again = restarted.get(&accepted.task_id).expect("read back");
    assert_eq!(again.state, TaskState::Completed);
    assert_eq!(again.result, record.result);

    // The second diagnosis of the subject counts the first (her own pid-bound memory).
    let second = tasks.accept("mira", KNOWN, None, None).unwrap();
    let second = finished(&tasks, &second.task_id);
    let answer: serde_json::Value =
        serde_json::from_str(second.result.as_deref().unwrap()).unwrap();
    assert_eq!(answer["prior_diagnoses"], 1);
    assert_eq!(
        *fake.calls.lock().unwrap(),
        1,
        "a JSON signal asks no provider"
    );
}

#[test]
fn a_low_confidence_task_ends_halted() {
    let kernel = Kernel::new();
    let tasks = kernel.tasks();
    tasks.register("mira", Arc::new(kernel.mira_runner(None)));
    let accepted = tasks.accept("mira", SEVERE_UNKNOWN, None, None).unwrap();
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Halted, "{record:?}");
    let halt = record.halt.expect("the task names its halt");
    assert_eq!(halt.state, "PendingResolution");
    assert!(
        kernel
            .registry
            .pending_halt_ids()
            .iter()
            .any(|id| id.as_str() == halt.halt_id),
        "the halt is pending in the real registry"
    );
    // A restarted registry reads the terminal row, not the answer.
    let again = kernel.tasks().get(&accepted.task_id).unwrap();
    assert_eq!(again.state, TaskState::Halted);
}

#[test]
fn the_registry_exists_without_installed_members() {
    // No installed component anywhere: one native runner is all the registry needs.
    let kernel = Kernel::new();
    let tasks = kernel.tasks();
    assert!(!tasks.is_directable("mira"));
    tasks.register("mira", Arc::new(kernel.mira_runner(None)));
    assert!(tasks.is_directable("mira"));
    let accepted = tasks.accept("mira", KNOWN, Some("idem-1"), None).unwrap();
    assert!(!accepted.replayed);
    let replayed = tasks.accept("mira", KNOWN, Some("idem-1"), None).unwrap();
    assert!(
        replayed.replayed,
        "22-4's idempotency key applies unchanged"
    );
    assert_eq!(replayed.task_id, accepted.task_id);
    finished(&tasks, &accepted.task_id);
}

#[test]
fn a_task_that_is_not_a_signal_ends_failed_with_a_typed_reason() {
    let kernel = Kernel::new();
    let tasks = kernel.tasks();
    tasks.register("mira", Arc::new(kernel.mira_runner(None)));
    let accepted = tasks
        .accept("mira", "the database is slow", None, None)
        .unwrap();
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Failed);
    assert_eq!(record.reason.as_deref(), Some("invalid_task"));
}

#[test]
fn nash_stays_not_directable() {
    let kernel = Kernel::new();
    let tasks = kernel.tasks();
    tasks.register("mira", Arc::new(kernel.mira_runner(None)));
    assert!(!tasks.is_directable("nash"));
    assert_eq!(
        tasks.accept("nash", "architect something", None, None),
        Err(AcceptError::NotDirectable)
    );
}

// ── the keyless default boot (process witness) ──────────────────────────────

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

#[test]
fn the_default_cast_boots_keyless_in_deterministic_mode() {
    let dir = std::env::temp_dir().join(format!("maos-native-22-8-{}", std::process::id()));
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let maos = PathBuf::from(env!("CARGO_BIN_EXE_maos"));
    let maosctl = maos.with_file_name("maosctl");
    let command = |program: &Path| {
        let mut command = Command::new(program);
        command
            .env("HOME", &dir)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", dir.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .env_remove("MAOS_REPLAY_CASSETTE")
            .env_remove("MAOS_ANTHROPIC_API_KEY")
            .current_dir(workspace_root());
        command
    };
    let init = command(&maos).arg("init").output().unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    // The image's default cast: Nash (a declared provider now) and Observer, no inference mode.
    let config = dir.join("host.toml");
    std::fs::write(
        &config,
        format!(
            "[host]\n\n[[host.spirits]]\nmanifest = '{}'\n\n[[host.spirits]]\nmanifest = '{}'\n",
            workspace_root()
                .join("spirits/nash/manifest.toml")
                .display(),
            workspace_root()
                .join("spirits/observer/manifest.toml")
                .display(),
        ),
    )
    .unwrap();
    let mut child = command(&maos)
        .args(["serve", "--config"])
        .arg(&config)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let mut running = false;
    while Instant::now() < until {
        let agents = command(&maosctl).arg("agents").output().unwrap();
        let listing = String::from_utf8_lossy(&agents.stdout);
        if agents.status.success()
            && ["nash", "observer"].iter().all(|id| {
                listing
                    .lines()
                    .any(|line| line.starts_with(&format!("{id}\t")) && line.contains("Running"))
            })
        {
            running = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        running,
        "the default cast must boot with no key and no mode; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
