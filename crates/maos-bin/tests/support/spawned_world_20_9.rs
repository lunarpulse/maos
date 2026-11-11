//! Story 20-9 — a real root in-process: scheduler, capability registry, bus, supervisor, the
//! 17-4 callers, the hot-swap coordinator and the real successor factory, driving the real
//! `maos-wasm-runner` children of the lifecycle-probe guest.
#![allow(dead_code)]

use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::hook_pump::{ConsolidationCadence, ConsolidationWatchdog, HookPump};
use maos_bin::operator_door::SuccessorManifestSlot;
use maos_bin::spawned_lifecycle::{spawned_successor, HostedSessions, SwapHandles};
use maos_bin::spirit_session::{
    manifest_launch, run_admitted_session, SessionDeps, SessionError, SessionReport,
};
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::{i1::IntentClass, i3::FrameOrigin};
use maos_iac::adapter::FrameFilter;
use maos_kernel_core::capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter};
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::iac::transparency_log::FrameKind as TlKind;
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::{UpgradeError, UpgradeOrchestrator, UpgradePolicy};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::{IdleWatchdog, ScheduleWatchdog, SpiritSchedulerAdapter};
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[path = "../../../../tests/harness/fixtures.rs"]
pub mod fixtures;

pub const NONCE: u64 = 0x2009;
pub const ID: &str = "lifecycle-probe";

/// The environment is process-global: every vector of a binary shares these windows.
pub fn fast_clocks() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("MAOS_IDLE_FAST", "1");
        std::env::set_var("MAOS_SCHEDULE_FAST", "1");
        std::env::set_var("MAOS_AUTO_REVERT_FAST", "1");
        // The FR11 archive (`$XDG_DATA_HOME`/`$HOME`, not MAOS_HOME) and the journals: keep the
        // user's untouched.
        let home = tempfile::tempdir().unwrap().keep();
        std::env::set_var("MAOS_ARCHIVE_DIR", home.join("spirit-archives"));
        std::env::set_var("MAOS_HOME", home);
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    });
}

/// `extra` is appended verbatim (more sections); `hooks` empty means every hook is allowed.
pub fn manifest(name: &str, version: &str, artifact: &str, hooks: &[&str], extra: &str) -> String {
    let hooks = hooks
        .iter()
        .map(|hook| format!("\"{hook}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"[class]
name = "{name}"
description = "lifecycle witness"
version = "{version}"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "{artifact}"
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
[scheduling]
priority_weight = 100
idle_window_ms = 2000
[lifecycle]
enabled_hooks = [{hooks}]
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
{extra}"#
    )
}

pub const ALL_HOOKS: [&str; 9] = [
    "on_idle",
    "on_consolidate",
    "on_telemetry_event",
    "on_schedule",
    "on_pause",
    "on_resume",
    "on_swap_in",
    "snapshot",
    "migrate",
];

struct Host(PathBuf);

impl maos_host::SpiritHostPort for Host {
    fn resolve_launch(
        &self,
        request: &maos_host::SpiritLaunchRequest,
    ) -> Result<maos_host::SpiritLaunchPlan, maos_host::SpiritHostError> {
        Ok(maos_host::SpiritLaunchPlan {
            program: self.0.to_string_lossy().into_owned(),
            argv: vec![
                "--component".into(),
                request.artifact.clone(),
                "--fuel".into(),
                "100000000000".into(),
            ],
            env: vec![],
            wire: maos_host::WireShape::ContentLengthCbor,
        })
    }
    fn supported_forms(&self) -> &[maos_host::SpiritForm] {
        &[maos_host::SpiritForm::WasmComponent]
    }
}

pub struct Running {
    pub pid: u32,
    pub session: tokio::task::JoinHandle<Result<SessionReport, SessionError>>,
}

pub struct World {
    pub dir: tempfile::TempDir,
    pub tl: Arc<TransparencyLogAdapter>,
    pub capability: Arc<CapabilityRegistryAdapter>,
    pub security: Arc<SecurityManagerAdapter>,
    pub iac: Arc<IacBusAdapter>,
    pub memory: Arc<MemoryManagerAdapter>,
    pub journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    pub halt: Arc<maos_kernel_core::halt::HaltRegistry>,
    pub scheduler: Arc<SpiritSchedulerAdapter>,
    pub supervisor: Arc<maos_bin::supervision::WorkerSupervisor>,
    pub lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    pub telemetry: Arc<TelemetryStreamAdapter>,
    pub pump: Arc<HookPump>,
    pub handles: Arc<SwapHandles>,
    pub orchestrator: Arc<UpgradeOrchestrator>,
    pub staged: SuccessorManifestSlot,
    pub collector: maos_iac::adapter::mailbox::SpiritMailboxHandle,
    pub cancel: CancellationToken,
    seen: usize,
    root: Option<maos_bin::supervision::RootSupervision>,
    consolidation: Option<ConsolidationWatchdog>,
}

impl World {
    pub async fn new() -> Self {
        fast_clocks();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let tl = Arc::new(TransparencyLogAdapter::open(&path.join("tl.sqlite"), NONCE).unwrap());
        maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&path.join("tl.sqlite"))
            .unwrap();
        let lookup = Arc::new(RwLock::new(BTreeMap::new()));
        tl.install_recipient_resolver(Arc::clone(&lookup) as _)
            .unwrap();
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let telemetry = Arc::new(TelemetryStreamAdapter::default());
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([9; 32]),
            NONCE,
            policy,
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::clone(&telemetry),
        ));
        let db = path.join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                path.join("private"),
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
            maos_kernel_core::journal::JournalAdapter::open(&path.join("lifecycle.ndjson"))
                .unwrap(),
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
        // The audit writer is the journal of `cap.revoke`; it ends with the process.
        std::mem::forget(cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl)));
        let pump = Arc::new(HookPump::new(&scheduler, &telemetry, Arc::clone(&tl)));
        let cancel = CancellationToken::new();
        // The real callers, as `main.rs` spawns them.
        Arc::new(IdleWatchdog::new(
            scheduler.scbs(),
            scheduler.dispatcher_arc(),
        ))
        .spawn(cancel.child_token());
        Arc::new(
            ScheduleWatchdog::new(
                scheduler.scbs(),
                scheduler.dispatcher_arc(),
                Arc::clone(&tl),
            )
            .with_capability(Arc::clone(&capability)),
        )
        .spawn(cancel.child_token());
        let consolidation = ConsolidationWatchdog::spawn(&pump, ConsolidationCadence::from_env());
        let session: Arc<
            dyn Fn(Arc<maos_bin::supervision::WorkerSupervisor>) -> SessionDeps + Send + Sync,
        > = {
            let (capability, security, iac) = (
                Arc::clone(&capability),
                Arc::clone(&security),
                Arc::clone(&iac),
            );
            let (tl, journal, lookup) =
                (Arc::clone(&tl), Arc::clone(&journal), Arc::clone(&lookup));
            Arc::new(move |supervisor| SessionDeps {
                supervisor,
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
                skill: None,
            })
        };
        let handles = Arc::new(SwapHandles {
            supervisor: Arc::downgrade(&supervisor),
            host: Arc::new(Host(fixtures::fixture("maos-wasm-runner"))),
            session,
            hosted: Arc::new(HostedSessions::default()),
        });
        let staged = SuccessorManifestSlot::default();
        let coordinator = Arc::new(HotSwapCoordinator::new(
            scheduler.scbs(),
            Arc::clone(&journal),
            Arc::clone(&tl),
            Arc::clone(&halt),
            Arc::clone(&capability),
            Arc::clone(&iac),
            scheduler.dispatcher_arc(),
            Arc::clone(&metrics),
            path.join("archive"),
        ));
        let factory_handles = Arc::clone(&handles);
        let (factory_tl, factory_staged) = (Arc::clone(&tl), staged.clone());
        let orchestrator = Arc::new(UpgradeOrchestrator::new(
            Arc::clone(&scheduler),
            coordinator,
            Arc::clone(&tl),
            Arc::clone(&journal),
            metrics,
            Arc::new(
                move |bundle: &maos_kernel_core::scheduler::SpiritManifestBundle| {
                    spawned_successor(
                        &factory_handles,
                        &factory_tl,
                        bundle,
                        factory_staged.peek(),
                        factory_staged.peek_origin(),
                    )
                    .map_err(|reason| UpgradeError::SuccessorFactory { reason })
                },
            ),
        ));
        let collector = iac
            .register_spirit_typed(&SpiritId::from("collector"))
            .unwrap();
        for (name, bytes) in [
            (
                "lifecycle_probe.wasm",
                std::fs::read(fixtures::fixture("lifecycle_probe.wasm")).unwrap(),
            ),
            (
                "echo_2_3.wasm",
                std::fs::read(frozen("echo_spirit_2_3_0.wasm")).unwrap(),
            ),
        ] {
            std::fs::write(path.join(name), bytes).unwrap();
        }
        Self {
            dir,
            tl,
            capability,
            security,
            iac,
            memory,
            journal,
            halt,
            scheduler,
            supervisor,
            lookup,
            telemetry,
            pump,
            handles,
            orchestrator,
            staged,
            collector,
            cancel,
            seen: 0,
            root: Some(root),
            consolidation: Some(consolidation),
        }
    }

    pub fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `maos run`'s standalone arm for a component: gate, prepare, admit, start, run.
    pub async fn start(&self, manifest_path: &Path) -> Running {
        let text = std::fs::read_to_string(manifest_path).unwrap();
        let gated = gate_manifest(&text, &|_| false).unwrap();
        let launch = manifest_launch(&*self.handles.host, &gated, manifest_path, false).unwrap();
        let (core, obj) = self
            .supervisor
            .prepare_spawned(&gated.spirit_id, &gated.bundle())
            .unwrap();
        let bound = Arc::clone(&core);
        let admitted = load_admit_start(
            &AdmissionDeps {
                providers: None,
                iac: Some(&self.iac),
                scheduler: &self.scheduler,
                security: &self.security,
                transparency_log: &self.tl,
                journal: &self.journal,
                memory: &self.memory,
                pid_by_spirit_id: &self.lookup,
                hooks: Some(&self.pump),
            },
            &gated,
            obj,
            NONCE,
            StartPolicy::StartNow,
            &mut |pid| bound.bind_scheduler_pid(pid),
        )
        .await
        .unwrap();
        let pid = admitted.pid;
        let deps = (self.handles.session)(Arc::clone(&self.supervisor));
        let session = tokio::spawn(run_admitted_session(deps, admitted, core, launch));
        let deadline = Instant::now() + Duration::from_secs(30);
        let target = loop {
            if let Some(target) = self.supervisor.spawned_target(&gated.spirit_id) {
                if target
                    .lifecycle()
                    .wait_supported(Duration::from_millis(50))
                    .is_some()
                {
                    break target;
                }
            }
            assert!(
                Instant::now() < deadline && !session.is_finished(),
                "session never became ready"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        drop(target);
        Running { pid, session }
    }

    /// Deliver one scripted frame to `spirit` and return the probe's report (its first line
    /// onward), read off the `collector` mailbox.
    pub async fn turn(&mut self, spirit: &str, script: &str) -> String {
        self.send(spirit, script).await;
        match self.report(Duration::from_secs(15)).await {
            Some(report) => report,
            None => {
                let rows = self.tl.query_frames(Default::default()).unwrap();
                let tail: Vec<String> = rows
                    .iter()
                    .filter(|row| !row.intent.starts_with("cap.") && row.intent != "log.recall")
                    .rev()
                    .take(12)
                    .map(|row| {
                        format!(
                            "{:?} {} {}",
                            row.kind,
                            row.intent,
                            String::from_utf8_lossy(&row.payload_redacted)
                                .chars()
                                .take(160)
                                .collect::<String>()
                        )
                    })
                    .collect();
                panic!(
                    "no report within 15 s for `{spirit}`; last TL rows:\n{}",
                    tail.join("\n")
                );
            }
        }
    }

    /// Mark `spirit` quiescent from now: the IdleWatchdog fires once per quiet period, and the
    /// kernel does not stamp inbound time for a mailbox delivery in this composition.
    pub fn quiet(&self, spirit: &str) {
        let pid = self.scheduler.resolve_pid(spirit).expect("a loaded spirit");
        let scb = self
            .scheduler
            .scbs()
            .read()
            .unwrap()
            .get(&pid)
            .cloned()
            .expect("its SCB");
        scb.last_inbound_frame_ns.store(
            maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Deliver without waiting for the report.
    pub async fn send(&self, spirit: &str, script: &str) {
        let address = |id: &str| FrameAddress {
            spirit_id: SpiritId::from(id),
            host_id: None,
            role: None,
        };
        let frame = IacFrame {
            frame_id: ulid::Ulid::new().to_bytes(),
            timestamp_ns: 1,
            logical_clock: 1,
            from: address("operator"),
            to: smallvec::smallvec![address(spirit)],
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
        };
        self.iac.deliver_typed(frame, 0, None).await.unwrap();
    }

    /// The next unread probe report. Telemetry frames are broadcast, so the witness is the
    /// session's own authorization row, which carries the guest's frame verbatim.
    pub async fn report(&mut self, within: Duration) -> Option<String> {
        let deadline = Instant::now() + within;
        while Instant::now() < deadline {
            let reports: Vec<String> = self
                .rows(TlKind::GovernanceEvent, "spirit.frame.authorization")
                .iter()
                .filter_map(|row| {
                    let value: serde_json::Value =
                        serde_json::from_slice(&row.payload_redacted).ok()?;
                    let frame: IacFrame = serde_json::from_value(value["claims"].clone()).ok()?;
                    match frame.payload {
                        FramePayload::TelemetryEvent(body)
                            if body.event_type == "lifecycle.probe" =>
                        {
                            Some(body.data)
                        }
                        _ => None,
                    }
                })
                .collect();
            if let Some(next) = reports.get(self.seen) {
                self.seen += 1;
                return Some(next.clone());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        None
    }

    /// Stage `path` as the successor and run the real orchestrator.
    pub async fn upgrade(&self, path: &Path, policy: UpgradePolicy) -> Result<(), String> {
        self.staged.stage(path.to_path_buf());
        let result = self.orchestrator.upgrade(ID, path, policy).await;
        self.staged.take();
        result.map(|_| ()).map_err(|error| error.to_string())
    }

    pub fn rows(
        &self,
        kind: TlKind,
        intent_prefix: &str,
    ) -> Vec<maos_iac::adapter::TransparencyLogEntry> {
        self.tl
            .query_frames(FrameFilter {
                kind: Some(kind),
                order_by_insertion: true,
                ..Default::default()
            })
            .unwrap()
            .into_iter()
            .filter(|row| row.intent.starts_with(intent_prefix))
            .collect()
    }

    /// `name=N` of a report line.
    pub fn count(report: &str, name: &str) -> u64 {
        report
            .split_whitespace()
            .find_map(|word| word.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("no `{name}=` in:\n{report}"))
    }

    pub async fn finish(mut self) {
        self.cancel.cancel();
        if let Some(consolidation) = self.consolidation.take() {
            consolidation.shutdown().await;
        }
        self.pump.shutdown().await;
        if let Some(root) = self.root.take() {
            root.stop_and_join().await;
        }
    }
}

pub fn frozen(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/wasm/frozen")
        .join(name)
}

impl Drop for World {
    /// A failed vector must not leave sessions (blocking threads) or runner children behind:
    /// latching the stop makes every session end, which kills its child.
    fn drop(&mut self) {
        self.cancel.cancel();
        self.supervisor.stop_workers();
    }
}
