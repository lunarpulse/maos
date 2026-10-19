//! In-process supervisor scenes over stub-runtime CIDs (run locally and in CI):
//! one root deadline for N concurrent teardowns (AC4.4 / PF-06), late
//! attachment after a stop (PF-07), and the startup report journaled through
//! the shared lifecycle journal (AC2.4).

use super::stub_scenes::{calls, image, podman, state};
use super::{scratch, CID};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use maos_bin::supervision::{
    ObserverGate, SupervisorSeams, WorkerSupervision, WorkerSupervisor, WorkerTask,
};
use maos_bin::worker_cli::{
    FixtureCli, WorkerCli, WorkerCompletion, WorkerNonCompletion, FIXTURE_COMPLETION_MARKER,
};
use maos_domain::sandbox::{ObservedPid, SandboxInspectReport};
use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
    argv_prefix_hash, bridge_container, Backpressure, BridgeExit, BridgeSpawnSpec,
};
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::security::sandbox::t3::child::{
    SandboxedContainerChild, KILL_BUDGET, SNAPSHOT_BUDGET,
};
use maos_kernel_core::security::sandbox::SandboxSpec;
use maos_manifest::{CliWrapperControlChannel, CliWrapperStdioShape};
use std::sync::Arc;

const NONCE: u64 = 0x17_07_5c;

pub(super) struct World {
    pub(super) journal_path: std::path::PathBuf,
    pub(super) supervisor: Arc<WorkerSupervisor>,
    pub(super) tl: Arc<TransparencyLogAdapter>,
}

pub(super) fn world(dir: &Path) -> World {
    world_with(dir, SupervisorSeams::default())
}

fn world_with(dir: &Path, seams: SupervisorSeams) -> World {
    world_with_log(
        dir,
        seams,
        Arc::new(TransparencyLogAdapter::open_in_memory(NONCE)),
    )
}

fn world_with_log(dir: &Path, seams: SupervisorSeams, tl: Arc<TransparencyLogAdapter>) -> World {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let policy = Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new());
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            policy,
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let memory_db = dir.join("memory.sqlite");
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            dir.join("private"),
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db).unwrap()),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db).unwrap(),
        ),
        Arc::clone(&tl),
    ));
    let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        memory,
        Arc::clone(&iac),
        Arc::clone(&halt),
        Arc::clone(&telemetry),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal_path = dir.join("lifecycle.ndjson");
    let journal = Arc::new(maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap());
    let crash = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
        scheduler.scbs(),
        Arc::clone(&tl),
        Arc::clone(&halt),
        capability,
        Arc::clone(&iac),
        telemetry,
        Arc::clone(&journal),
    ));
    Arc::get_mut(&mut scheduler)
        .expect("scheduler Arc strong_count == 1")
        .set_crash_detector(Arc::clone(&crash));
    let supervisor = Arc::new(
        WorkerSupervisor::new(
            scheduler,
            crash,
            Arc::clone(&tl),
            journal,
            halt,
            iac,
            tokio::runtime::Handle::current(),
            NONCE,
            tokio_util::sync::CancellationToken::new(),
        )
        .with_seams(seams),
    );
    World {
        journal_path,
        supervisor,
        tl,
    }
}

fn bind(world: &World) -> maos_bin::supervision::WorkerBinding {
    tokio::task::block_in_place(|| {
        world
            .supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    })
}
fn bridge_container_for(
    child: SandboxedContainerChild,
    spirit_id: &str,
) -> maos_kernel_core::lifecycle::cli_wrapper::runtime::SpawnedBridge {
    let argv_prefix = vec!["stub-worker".to_owned()];
    let mut sandbox = SandboxSpec::new_for_test(maos_domain::invariants::i9::SandboxTier::T3);
    sandbox.spirit_id = spirit_id.to_owned();
    bridge_container(
        BridgeSpawnSpec {
            program: "stub-worker".to_owned(),
            sandbox,
            expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
            argv_prefix,
            task_args: Vec::new(),
            from_spirit_id: spirit_id.to_owned(),
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: Some("SIGTERM".to_owned()),
            channel_capacity: 8,
            backpressure: Backpressure::Block,
            env: Vec::new(),
            t3: None,
        },
        child,
        image(),
    )
    .expect("bridge the owned stub T3 child")
}
pub(super) fn finalize_owned_child(
    world: &World,
    child: SandboxedContainerChild,
) -> (u32, BridgeExit) {
    tokio::task::block_in_place(|| {
        let binding = bind(world);
        let spirit_id = binding.core().spirit_id().to_owned();
        let mut bridge = bridge_container_for(child, &spirit_id);
        binding
            .core()
            .attach_container(bridge.container_terminal().expect("T3 bridge terminal"));
        world.supervisor.watch(binding.core(), bridge.child_pid());
        let pid = binding.core().spirit_pid();
        bridge.pump_to_journal(&world.tl, pid, "kernel", "runtime-worker", &[]);
        let exit = bridge.wait_and_finalize(&world.tl, pid, None, |_| {});
        tokio::task::block_in_place(|| {
            let _ = binding.finish(&exit);
        });
        (pid, exit)
    })
}

fn finalize_stub_bridge(world: &World, root: &Path, label: &str, mode: &str) -> BridgeExit {
    tokio::task::block_in_place(|| {
        let spawn_dir = root.join(label);
        std::fs::create_dir(&spawn_dir).expect("spawn scratch directory");
        let state_dir = state(&spawn_dir);
        let child =
            super::stub_scenes::launch(&spawn_dir, mode, &state_dir, &[]).expect("stub T3 launch");
        let binding = bind(world);
        let spirit_id = binding.core().spirit_id().to_owned();
        let mut bridge = bridge_container_for(child, &spirit_id);
        binding
            .core()
            .attach_container(bridge.container_terminal().expect("T3 bridge terminal"));
        world.supervisor.watch(binding.core(), bridge.child_pid());
        let pid = binding.core().spirit_pid();
        bridge.pump_to_journal(&world.tl, pid, "kernel", "stub-worker", &[]);
        let exit = bridge.wait_and_finalize(&world.tl, pid, None, |_| {});
        tokio::task::block_in_place(|| {
            let _ = binding.finish(&exit);
        });
        exit
    })
}

fn adopt(dir: &Path, mode: &str) -> SandboxedContainerChild {
    let runtime = podman(dir, mode);
    SandboxedContainerChild::adopt(runtime, CID.to_owned(), &state(dir))
}

fn count(dir: &Path, verb: &str) -> usize {
    calls(dir)
        .lines()
        .filter(|line| line.starts_with(&format!("argv=<{verb}>")))
        .count()
}

/// PF-06: three bindings whose CID kills hang are latched and torn down
/// concurrently under ONE root deadline, not three sequential windows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deadlines_root_stop_tears_down_n_cids_under_one_deadline() {
    let root = scratch("root-deadline");
    let world = world(&root);
    let dirs: Vec<_> = (0..3).map(|n| root.join(format!("cid-{n}"))).collect();
    let mut owned = Vec::new();
    let mut bindings = Vec::new();
    for dir in &dirs {
        std::fs::create_dir(dir).unwrap();
        let binding = bind(&world);
        let child = adopt(dir, "kill-hang");
        binding
            .core()
            .attach_container(Arc::clone(child.terminal()));
        owned.push(child);
        bindings.push(binding);
    }
    let began = Instant::now();
    tokio::task::block_in_place(|| world.supervisor.stop_workers());
    let elapsed = began.elapsed();
    let first: Vec<_> = dirs
        .iter()
        .map(|dir| (count(dir, "kill"), count(dir, "rm")))
        .collect();
    let verdicts: Vec<_> = owned
        .iter()
        .map(|child| child.terminal().teardown(Instant::now()))
        .collect();
    // Drop the bindings before any assertion: an unwinding drop inside the
    // async test body would block_on in the runtime and abort the harness.
    tokio::task::block_in_place(|| {
        drop(bindings);
        drop(owned);
    });
    assert!(
        elapsed < KILL_BUDGET * 2,
        "three hung kills took {elapsed:?}: teardown is serial, not one root deadline"
    );
    assert!(
        elapsed >= KILL_BUDGET - Duration::from_millis(500),
        "kills were bounded, not skipped"
    );
    assert_eq!(
        first,
        vec![(1, 1); 3],
        "every CID gets exactly one kill and one removal"
    );
    assert_eq!(verdicts, vec![Ok(()); 3], "verdicts are final");
    for dir in &dirs {
        assert_eq!(
            count(dir, "rm"),
            1,
            "binding and guard drops never retry the verdict"
        );
    }
}

/// PF-07: a stop requested before the container is attached is executed on
/// attachment, exactly once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn late_attachment_after_stop_tears_down_once() {
    let dir = scratch("late-attach");
    let world = world(&dir);
    let binding = bind(&world);
    tokio::task::block_in_place(|| world.supervisor.stop_workers());
    let child = adopt(&dir, "normal");
    tokio::task::block_in_place(|| {
        binding
            .core()
            .attach_container(Arc::clone(child.terminal()))
    });
    let at_attach = count(&dir, "rm");
    let confirmed = child.terminal().termination_confirmed();
    tokio::task::block_in_place(|| {
        world.supervisor.stop_binding(binding.spirit_pid());
        drop(binding);
        drop(child);
    });
    assert_eq!(
        at_attach, 1,
        "attachment after the latch tears down at once"
    );
    assert!(confirmed);
    assert_eq!(
        count(&dir, "rm"),
        1,
        "later stops and drops join the one verdict"
    );
}

/// AC2.4: the startup report is journaled through the shared lifecycle
/// journal with its payload, readable after reopening the file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sandbox_applied_report_survives_journal_reopen() {
    let dir = scratch("journal-reopen");
    let world = world(&dir);
    let binding = bind(&world);
    let child = adopt(&dir, "normal");
    world
        .supervisor
        .commit_container_spawn(binding.core(), &child, &image())
        .expect("commit the T3 startup report");
    let reports: Vec<SandboxInspectReport> = std::fs::read_to_string(&world.journal_path)
        .expect("reopen lifecycle journal")
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|entry| {
            let lifecycle = entry.get("Lifecycle").unwrap_or(&entry);
            (lifecycle["lifecycle_event"] == "SandboxApplied").then(|| lifecycle["payload"].clone())
        })
        .map(|payload| {
            let bytes: Vec<u8> = serde_json::from_value(payload).expect("payload bytes kept");
            serde_json::from_slice(&bytes).expect("payload is the inspect report")
        })
        .collect();
    let spirit_id = binding.spirit_id().to_owned();
    tokio::task::block_in_place(|| {
        drop(binding);
        drop(child);
    });
    assert_eq!(
        reports.len(),
        1,
        "exactly one SandboxApplied entry with its payload"
    );
    assert_eq!(reports[0].container_id, CID);
    assert_eq!(reports[0].spirit_id, spirit_id);
    assert!(matches!(reports[0].payload_pid, ObservedPid::Historical(_)));
}
fn persisted_fixture_completion(
    world: &World,
    pid: u32,
    may_complete: bool,
    exit: &BridgeExit,
) -> WorkerCompletion {
    let rows = world
        .tl
        .query_frames(FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::CliSubprocessOutput),
            spirit_pid: Some(pid),
            ..FrameFilter::default()
        })
        .expect("query bridge-persisted CLI output");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for row in rows {
        let payload: serde_json::Value =
            serde_json::from_slice(&row.payload_redacted).expect("CLI output payload");
        let line = payload["line"].as_str().unwrap_or_default().to_owned();
        match payload["stream"].as_str() {
            Some("stdout") => stdout.push(line),
            Some("stderr") => stderr.push(line),
            _ => {}
        }
    }
    assert!(
        stdout
            .iter()
            .any(|line| line.contains(FIXTURE_COMPLETION_MARKER)),
        "completion marker must come from persisted bridge output"
    );
    let worker_exit = maos_bin::worker_spawn::worker_exit_from_finalized_bridge(may_complete, exit);
    FixtureCli.parse_completion(&stdout, &stderr, worker_exit)
}

fn terminal_feed_scene(
    gated_observer: bool,
    mode: &str,
) -> (
    World,
    tempfile::TempDir,
    Arc<maos_kernel_core::security::sandbox::t3::child::ContainerTerminal>,
    BridgeExit,
    u32,
    bool,
) {
    tokio::task::block_in_place(|| {
        let dir = tempfile::tempdir().expect("scene temp dir");
        let gate = gated_observer.then(|| Arc::new(ObserverGate::default()));
        let world = world_with(
            dir.path(),
            SupervisorSeams {
                observer_gate: gate.clone(),
                ..Default::default()
            },
        );
        let mut binding = bind(&world);
        let cancel = tokio_util::sync::CancellationToken::new();
        let (egress, egress_task) = maos_bin::egress::EgressHandle::start(
            dir.path().join("egress-home"),
            Arc::clone(&world.tl),
            cancel.clone(),
        )
        .expect("start the binding's actual proxy owner");
        let lease = egress
            .spawn(
                binding.spirit_pid(),
                binding.spirit_id(),
                &[],
                &[],
                maos_egress::Credential::None,
                Duration::from_secs(60),
            )
            .expect("attach a real lease before observing unavailable facts");
        let lease_socket = lease.socket_path().to_owned();
        binding.attach_egress(lease);
        let child = super::stub_scenes::launch(dir.path(), mode, &state(dir.path()), &[])
            .expect("stub T3 launch");
        let spirit_id = binding.core().spirit_id().to_owned();
        let mut bridge = bridge_container_for(child, &spirit_id);
        let terminal = bridge
            .container_terminal()
            .expect("bridge owns its T3 terminal");
        binding.core().attach_container(Arc::clone(&terminal));
        let pid = binding.core().spirit_pid();
        world.supervisor.watch(binding.core(), bridge.child_pid());
        if gated_observer {
            let deadline = Instant::now() + Duration::from_secs(5);
            while world
                .supervisor
                .bindings()
                .iter()
                .find(|view| view.spirit_pid == pid)
                .is_none_or(|view| view.child_pid.is_none())
            {
                assert!(
                    Instant::now() < deadline,
                    "observer did not store client pid"
                );
                std::thread::yield_now();
            }
        } else {
            let deadline = Instant::now() + SNAPSHOT_BUDGET + Duration::from_secs(5);
            while binding.core().phase() == maos_bin::supervision::BindingPhase::Running {
                assert!(
                    Instant::now() < deadline,
                    "observer did not classify client exit before the terminal snapshot deadline"
                );
                std::thread::yield_now();
            }
        }
        bridge.pump_to_journal(&world.tl, pid, "kernel", "stub-worker", &[]);
        let exit = bridge.wait_and_finalize(&world.tl, pid, None, |_| {});
        if let Some(gate) = gate {
            gate.open();
            let deadline = Instant::now() + Duration::from_secs(5);
            while world.supervisor.counters().observer_echild() == 0 {
                assert!(Instant::now() < deadline, "observer did not observe ECHILD");
                std::thread::yield_now();
            }
        }
        let (facts, verdict) = exit
            .container
            .as_ref()
            .expect("bridge finalizer persists T3 facts");
        assert!(
            facts.state.is_err(),
            "running or failed inspect must remain unavailable terminal evidence"
        );
        assert_eq!(*verdict, Ok(()), "bridge removes the owned test CID");
        let may_complete = tokio::task::block_in_place(|| {
            let result = binding.finish(&exit);
            assert!(
                !lease_socket.exists(),
                "confirmed cleanup releases the lease despite immutable unavailable facts"
            );
            let mut replacement = egress
                .spawn(
                    pid,
                    &spirit_id,
                    &[],
                    &[],
                    maos_egress::Credential::None,
                    Duration::from_secs(60),
                )
                .expect("confirmed binding finalization must release the owner's lease");
            replacement.finish(true).unwrap();
            cancel.cancel();
            tokio::runtime::Handle::current()
                .block_on(egress_task)
                .unwrap()
                .unwrap();
            result
        });
        (world, dir, terminal, exit, pid, may_complete)
    })
}

fn clean_marker_completion_scene() -> (World, tempfile::TempDir, u32, BridgeExit, bool) {
    tokio::task::block_in_place(|| {
        let dir = tempfile::tempdir().expect("clean marker scene directory");
        let world = world(dir.path());
        let binding = bind(&world);
        let child = super::stub_scenes::launch(dir.path(), "marker-clean", &state(dir.path()), &[])
            .expect("marker-producing stub T3 launch");
        let spirit_id = binding.core().spirit_id().to_owned();
        let mut bridge = bridge_container_for(child, &spirit_id);
        binding
            .core()
            .attach_container(bridge.container_terminal().expect("bridge terminal"));
        let pid = binding.core().spirit_pid();
        bridge.pump_to_journal(&world.tl, pid, "kernel", "stub-worker", &[]);
        let exit = bridge.wait_and_finalize(&world.tl, pid, None, |_| {});
        let may_complete = tokio::task::block_in_place(|| {
            let result = binding.finish(&exit);
            result
        });
        (world, dir, pid, exit, may_complete)
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clean_exit_marker_completes_from_persisted_bridge_output() {
    let (world, _dir, pid, exit, may_complete) = clean_marker_completion_scene();
    assert!(may_complete, "clean terminal evidence permits completion");
    assert_eq!(
        persisted_fixture_completion(&world, pid, may_complete, &exit),
        WorkerCompletion::Completed {
            final_message: FIXTURE_COMPLETION_MARKER.to_owned()
        }
    );
}

fn assert_one_persisted_truncated(world: &World, pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rows = world
            .tl
            .query_frames(FrameFilter::default())
            .expect("query production-written rows");
        let orphaned: Vec<_> = rows
            .iter()
            .filter(|row| row.spirit_pid == pid && row.intent == "task.orphaned")
            .collect();
        if !orphaned.is_empty() {
            assert_eq!(orphaned.len(), 1, "exactly one persisted orphan fault");
            let payload: serde_json::Value =
                serde_json::from_slice(&orphaned[0].payload_redacted).unwrap();
            assert_eq!(payload["cause"], "fault.truncated");
            assert_eq!(payload["exit_code"], serde_json::Value::Null);
            assert_eq!(payload["exit_signal"], serde_json::Value::Null);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "missing persisted fault.truncated"
        );
        std::thread::yield_now();
    }
}

/// An unexpected client exit reaches the production pidfd observer before
/// bridge finalization; the unavailable terminal snapshot survives cleanup.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unexpected_client_observer_first_persists_one_truncated_fault() {
    let (world, _dir, terminal, exit, pid, may_complete) =
        terminal_feed_scene(false, "marker-nonterminal");
    assert_eq!(
        persisted_fixture_completion(&world, pid, may_complete, &exit),
        WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: None })
    );
    assert_one_persisted_truncated(&world, pid);
    assert!(terminal.termination_confirmed());
    assert!(Arc::ptr_eq(
        &exit.container.as_ref().unwrap().0,
        &terminal.facts()
    ));
    assert_eq!(
        exit.container.as_ref().unwrap().0.payload,
        maos_domain::sandbox::PayloadExit::Unknown
    );
    let rows = world
        .tl
        .query_frames(FrameFilter::default())
        .expect("query finalized bridge row");
    let exits: Vec<_> = rows
        .iter()
        .filter(|row| row.spirit_pid == pid && row.intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(exits.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&exits[0].payload_redacted).unwrap();
    assert_eq!(payload["container"]["removed"], true);
    assert_eq!(payload["container"]["residue"], serde_json::Value::Null);
}
/// Reaping the client while the observer is gated forces ECHILD; the real
/// binding finalizer is then the sole feed and must persist the same fault.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unexpected_client_echild_finalizer_first_persists_one_truncated_fault() {
    let (world, _dir, terminal, exit, pid, may_complete) =
        terminal_feed_scene(true, "marker-inspect-fail");
    assert_eq!(
        persisted_fixture_completion(&world, pid, may_complete, &exit),
        WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: None })
    );
    assert_one_persisted_truncated(&world, pid);
    assert!(terminal.termination_confirmed());
    assert!(Arc::ptr_eq(
        &exit.container.as_ref().unwrap().0,
        &terminal.facts()
    ));
    assert_eq!(
        exit.container.as_ref().unwrap().0.payload,
        maos_domain::sandbox::PayloadExit::Unknown
    );
    let rows = world
        .tl
        .query_frames(FrameFilter::default())
        .expect("query finalized bridge row");
    let exits: Vec<_> = rows
        .iter()
        .filter(|row| row.spirit_pid == pid && row.intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(exits.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&exits[0].payload_redacted).unwrap();
    assert_eq!(payload["container"]["removed"], true);
    assert_eq!(payload["container"]["residue"], serde_json::Value::Null);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease() {
    const CHILD: &str = "MAOS_17_7_LEASE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "supervisor_scenes::binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "lease child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("distinctive-test-bearer-not-for-logs"));
        assert!(!String::from_utf8_lossy(&output.stdout)
            .contains("distinctive-test-bearer-not-for-logs"));
        return;
    }
    use maos_egress::Credential;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        os::unix::net::UnixStream,
    };
    use tokio_util::sync::CancellationToken;

    let dir = tempfile::tempdir().expect("scene temp dir");
    let world = world(dir.path());
    let mut binding = bind(&world);
    let pid = binding.core().spirit_pid();
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback upstream");
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let upstream = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("allowed request reaches upstream");
        let mut request = [0u8; 1024];
        let _ = socket.read(&mut request).unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
    });
    let destination = maos_domain::host_grant::EgressDestination::try_from(route.clone()).unwrap();
    let cancel = CancellationToken::new();
    let egress_home = tempfile::tempdir().expect("egress home");
    let (egress, root_task) = maos_bin::egress::EgressHandle::start(
        egress_home.path().join("home"),
        Arc::clone(&world.tl),
        cancel.clone(),
    )
    .unwrap();
    let lease = egress
        .spawn(
            pid,
            binding.core().spirit_id(),
            &[destination.clone()],
            &[destination],
            Credential::None,
            Duration::from_secs(60),
        )
        .unwrap();
    let socket_path = lease.socket_path().to_owned();
    let bearer = lease.bearer().to_owned();
    binding.attach_egress(lease);

    let request = |bearer: &str| {
        let mut socket = UnixStream::connect(&socket_path).expect("worker proxy remains live");
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            socket,
            "GET /{route}/test HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {bearer}\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        response
    };
    assert!(request(&bearer).starts_with("HTTP/1.1 200"));
    upstream.join().unwrap();

    let state_dir = state(dir.path());
    let child = super::stub_scenes::launch(
        dir.path(),
        "kill-hang-rm-fail-inspect-fail",
        &state_dir,
        &[],
    )
    .expect("stub container launch");
    binding
        .core()
        .attach_container(Arc::clone(child.terminal()));
    let stopping_core = Arc::clone(binding.core());
    let stop = tokio::task::spawn_blocking(move || stopping_core.stop_session());
    let deadline = Instant::now() + Duration::from_secs(3);
    while !super::stub_scenes::calls(dir.path()).contains("argv=<kill>") {
        assert!(Instant::now() < deadline, "test runtime kill was not gated");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !child.terminal().termination_confirmed(),
        "termination remains pending while the runtime kill is gated"
    );
    let retained = egress
        .spawn(
            pid,
            binding.core().spirit_id(),
            &[],
            &[],
            Credential::None,
            Duration::from_secs(60),
        )
        .err()
        .expect("a pending CID stop cannot release its egress lease");
    assert_eq!(retained.kind(), std::io::ErrorKind::AlreadyExists);
    stop.await.unwrap();
    assert!(
        !child.terminal().termination_confirmed(),
        "the runtime refused to confirm the container stopped"
    );
    let revoked_response = request(&bearer);

    let spirit_id = binding.core().spirit_id().to_owned();
    tokio::task::block_in_place(|| drop(binding));
    let retained_after_finish = egress.spawn(
        pid,
        &spirit_id,
        &[],
        &[],
        Credential::None,
        Duration::from_secs(60),
    );
    assert_eq!(
        retained_after_finish.err().map(|error| error.kind()),
        Some(std::io::ErrorKind::AlreadyExists),
        "unconfirmed binding finish must retain the owner's lease"
    );
    assert!(
        revoked_response.starts_with("HTTP/1.1 403"),
        "binding revokes the bearer before finishing an unconfirmed stop"
    );

    cancel.cancel();
    root_task.await.unwrap().unwrap();
    assert!(
        !socket_path.exists(),
        "root cancellation joins the retained proxy and reports its unresolved lease"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bridge_finalized_exit_rows_round_trip_through_fr4_audit_query() {
    const CHILD: &str = "MAOS_17_7_FR4_CHILD";
    const DB_PATH: &str = "MAOS_17_7_FR4_DB";
    if std::env::var_os(CHILD).is_some() {
        let db = std::path::PathBuf::from(std::env::var_os(DB_PATH).unwrap());
        let entries = maos_audit::query(
            &db,
            maos_audit::AuditFilter {
                intent_contains: Some("cli.subprocess.exit".to_owned()),
                ..Default::default()
            },
        )
        .expect("read persisted bridge rows through audit query");
        let mut output = Vec::new();
        maos_audit::to_fr4_ndjson(entries, &mut output)
            .expect("NDJSON renderer accepts finalized T3 rows");
        assert!(output.is_empty(), "all T3 exit rows are non-call events");
        return;
    }

    let dir = tempfile::tempdir().expect("persistent bridge audit directory");
    let db = dir.path().join("transparency.sqlite");
    let tl = Arc::new(TransparencyLogAdapter::open(&db, NONCE).expect("persistent TL"));
    let world = world_with_log(dir.path(), SupervisorSeams::default(), Arc::clone(&tl));
    let scenarios = [
        ("successful", "normal"),
        ("runtime-fallback", "fallback-137"),
        ("unavailable", "inspect-fail"),
        ("residue", "rm-fail"),
    ];
    for (label, mode) in scenarios {
        let exit = finalize_stub_bridge(&world, dir.path(), label, mode);
        assert!(
            exit.container.is_some(),
            "bridge finalizes T3 terminal facts"
        );
    }
    drop(world);
    drop(tl);

    let entries = maos_audit::query(
        &db,
        maos_audit::AuditFilter {
            intent_contains: Some("cli.subprocess.exit".to_owned()),
            ..Default::default()
        },
    )
    .expect("reopen finalized rows through public audit query");
    assert_eq!(
        entries.len(),
        4,
        "one bridge-writer row per terminal outcome"
    );
    let containers: Vec<_> = entries
        .iter()
        .map(|entry| {
            let payload: serde_json::Value = serde_json::from_str(&entry.payload).unwrap();
            payload["container"].clone()
        })
        .collect();
    assert!(containers.iter().any(|row| {
        row["payload_exit"] == "exited(code=0)"
            && row["payload_exit_source"] == "runtime"
            && row["removed"] == true
    }));
    assert!(containers.iter().any(|row| {
        row["runtime_exit_code"] == 137
            && row["payload_exit"] == "unknown"
            && row["payload_exit_source"] == "none"
    }));
    assert!(containers.iter().any(|row| {
        row["payload_exit"] == "unknown"
            && row["payload_exit_source"] == "none"
            && row["removed"] == true
    }));
    assert!(containers
        .iter()
        .any(|row| { row["removed"] == false && row["residue"] == "remove_failed" }));

    let mut plain = Vec::new();
    maos_audit::to_fr4_plain(entries.clone(), &mut plain)
        .expect("plain renderer accepts bridge-finalized rows");
    let plain = String::from_utf8(plain).unwrap();
    assert_eq!(plain.matches("cli.subprocess.exit").count(), 4);
    let mut ndjson = Vec::new();
    maos_audit::to_fr4_ndjson(entries, &mut ndjson)
        .expect("NDJSON renderer accepts bridge-finalized rows");
    assert!(ndjson.is_empty());

    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "supervisor_scenes::bridge_finalized_exit_rows_round_trip_through_fr4_audit_query",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env(DB_PATH, &db)
        .output()
        .expect("capture NDJSON omission report");
    assert!(
        output.status.success(),
        "renderer subprocess failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("4 non-call kernel row(s)"), "{stderr}");
    assert!(stderr.contains("capability.invocation×4"), "{stderr}");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_create_and_finalized_exit_rows_never_echo_bearer() {
    const CHILD: &str = "MAOS_17_7_BEARER_BRIDGE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "supervisor_scenes::failed_create_and_finalized_exit_rows_never_echo_bearer",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .expect("capture bearer-sensitive test process");
        assert!(
            output.status.success(),
            "bearer bridge subprocess failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout)
            .contains("distinctive-test-bearer-not-for-logs"));
        assert!(!String::from_utf8_lossy(&output.stderr)
            .contains("distinctive-test-bearer-not-for-logs"));
        return;
    }

    let dir = tempfile::tempdir().expect("bearer integration directory");
    let refused_dir = dir.path().join("refused");
    std::fs::create_dir(&refused_dir).unwrap();
    let refused_state = state(&refused_dir);
    let env = vec![(
        "WORKER_SECRET".to_owned(),
        "distinctive-test-bearer-not-for-logs".to_owned(),
    )];
    let refusal =
        match super::stub_scenes::launch(&refused_dir, "create-fails-bearer", &refused_state, &env)
        {
            Ok(_) => panic!("failed create must refuse the launch"),
            Err(error) => error.to_string(),
        };
    assert!(!refusal.contains(&env[0].1));
    let calls = super::stub_scenes::calls(&refused_dir);
    assert!(calls.contains("<rm>") && calls.contains(CID));
    assert!(!calls.contains("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"));

    let db = dir.path().join("transparency.sqlite");
    let tl = Arc::new(TransparencyLogAdapter::open(&db, NONCE).expect("persistent TL"));
    let world = world_with_log(dir.path(), SupervisorSeams::default(), Arc::clone(&tl));
    let exit = finalize_stub_bridge(&world, dir.path(), "successful", "bearer-create");
    let (facts, verdict) = exit.container.as_ref().expect("bridge finalized T3 facts");
    assert!(format!("{facts:?}").contains("error_reported: true"));
    assert!(!format!("{facts:?}").contains(&env[0].1));
    assert_eq!(*verdict, Ok(()));
    drop(world);
    drop(tl);

    let entries = maos_audit::query(&db, maos_audit::AuditFilter::default())
        .expect("reopen all production-written rows");
    assert!(
        entries
            .iter()
            .all(|entry| !entry.payload.contains(&env[0].1)),
        "no transparency-log row may expose the bearer"
    );
    let exits: Vec<_> = entries
        .iter()
        .filter(|entry| entry.intent.contains("cli.subprocess.exit"))
        .collect();
    assert_eq!(exits.len(), 1);
    let payload = &exits[0].payload;
    assert!(payload.contains("\"container\""));
    assert!(!payload.contains("WORKER_SECRET"));
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fallback_client_exit_persists_nonzero_fault_through_binding_classifier() {
    let dir = tempfile::tempdir().expect("fallback feed directory");
    let world = world(dir.path());
    let exit = finalize_stub_bridge(&world, dir.path(), "fallback", "fallback-137");
    let (facts, verdict) = exit.container.as_ref().expect("bridge terminal facts");
    assert_eq!(
        facts.payload,
        maos_domain::sandbox::PayloadExit::Unknown,
        "runtime 137 is not a signal without pidfd evidence"
    );
    assert_eq!(facts.state.as_ref().unwrap().exit_code, Some(137));
    assert_eq!(*verdict, Ok(()));
    let rows = world
        .tl
        .query_frames(FrameFilter::default())
        .expect("query production classification rows");
    let faults: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == "task.orphaned")
        .collect();
    assert_eq!(faults.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&faults[0].payload_redacted).unwrap();
    assert_eq!(payload["cause"], "fault.non-zero-exit");
    assert_eq!(payload["exit_code"], 137);
    assert_eq!(payload["exit_signal"], serde_json::Value::Null);
}
