#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 16-3 — **AC2's hang corpus** (ruling D-16-3-K, second half), the
//! NFR-Rel-2 evidence: a silent Worker is `task.stalled` within 60 s of its
//! spawn, at the DEFAULT 30 000 ms progress threshold, while a Worker that is
//! merely slow is never stalled — and every live Worker in the corpus still
//! reaches the crash path when it is SIGKILLed.
//!
//! What each half of this corpus refuses:
//!
//! * **The 50 `hang` Workers refuse "the watchdog exists, therefore hangs are
//!   seen."** At `af96c907` the ProgressWatchdog ran only in the serving tail,
//!   after every Worker-bearing root had returned, so nothing ever emitted
//!   `task.stalled`. Here `RootSupervision::arm` spawns the watchdog AND the
//!   progress stamper BEFORE any Worker is bound, and 50 silent Workers must
//!   stall within 60 s of their own spawn (NFR-Rel-2's floor, ≥48/50 to leave
//!   scheduler headroom on a loaded CI box).
//! * **The 5 `line-then-hang` Workers refuse an INCLUSIVE progress cursor
//!   (D-16-3-G).** Each prints exactly one line and then goes silent forever.
//!   The stamper reads new output rows with the keyset cursor
//!   `(timestamp_ns, frame_id) >` so every row is returned exactly once; a
//!   `since_ns` high-water is inclusive and would re-return the newest row
//!   every tick, re-stamping the Worker that printed LAST forever. This corpus
//!   proves the 5 stalled rows EXIST; the mutation RED for the inclusive
//!   cursor lives elsewhere (see the report in the story: this corpus's chatty
//!   Workers print the newest row every tick, so an inclusive cursor would
//!   still stall every silent Worker here and the red would not show).
//! * **The 5 `hang-chatty` Workers refuse "any quiet period means hung."** A
//!   chatty Worker prints one line per second, forever — it is slow, not
//!   hung — and false-stalling it is the defect this half exists to catch:
//!   0/5 chatty Workers may stall across a window that lasts until the last
//!   silent Worker stalls (or 60 s) and never less than 35 s.
//! * **All 60 refuse "stall detection replaces crash handling."** After the
//!   stall asserts, the corpus SIGKILLs every Worker through its binding's
//!   pidfd (`ObservedChild::pidfd()` + `pidfd_send_signal`, the ruled harness
//!   path) and requires a `lifecycle.crash` row at each Worker's own pid.
//!
//! Housekeeping the ruling pins: counting is by DISTINCT WORKER, never by row
//! (a stalled Worker refires a fresh `task.stalled` row after 2× threshold,
//! `progress_watchdog.rs`), and the Transparency Log is read by ONE poller
//! thread holding ONE keyset cursor over all new rows — never per-pid queries,
//! because the log has one mutex shared with the writes being measured.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use maos_bin::supervision::exit_observation::ObservedChild;
use maos_bin::supervision::{RootSupervision, WorkerTask};
use maos_bin::worker_spawn::{run_cli_wrapper_manifest, RunArgs};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

/// Distinct from every other 16-3 world's nonce so an in-memory log can never
/// be confused with a sibling file's.
const NONCE: u64 = 0x16_05;

/// The corpus composition (ruling D-16-3-K): 50 silent hangs, 5 one-line-then
/// hangs (the exclusive cursor's proof), 5 per-second chatterers (the
/// slow-is-not-hung refusal). Indices are the `TopologyEntry` task indices,
/// and the `task.stalled` payload carries `first_in_flight_task_id =
/// "topology:<index>"`, so a stall row names exactly which corpus arm it
/// belongs to — no pid bookkeeping is needed to classify one.
const HANG_WORKERS: usize = 50;
const LINE_WORKERS: usize = 5;
const CHATTY_WORKERS: usize = 5;
const TOTAL_WORKERS: usize = HANG_WORKERS + LINE_WORKERS + CHATTY_WORKERS;

/// How the poller polls. The log has ONE mutex (`transparency_log.rs`),
/// shared with the stamper, the watchdog and every crash write this corpus is
/// measuring — so there is exactly one query loop, never a waiter per Worker.
const POLL_TICK: Duration = Duration::from_millis(50);

/// NFR-Rel-2's floor: a hang is stalled within 60 s of spawn. The silent
/// Workers stall at threshold (30 s) + one stamper tick + one watchdog tick,
/// so 60 s leaves honest headroom without making the corpus slow.
const HANG_WINDOW: Duration = Duration::from_secs(60);

/// The chatty Workers must survive the WHOLE observation window, which never
/// closes earlier than this. A window shorter than the refire-adjacent window
/// the watchdog actually uses could miss a late false stall.
const CHATTY_MIN_WINDOW: Duration = Duration::from_secs(35);

/// The Workers are bound well before the first stall can possibly fire
/// (threshold is 30 s), so a 30 s spawn budget is generous even with 60
/// concurrent admission pipelines.
const SPAWN_BUDGET: Duration = Duration::from_secs(30);

/// A SIGKILLed Worker reaches `handle_crash` in well under a second; 10 s
/// absorbs scheduler noise on a loaded box for all 60 at once.
const CRASH_WINDOW: Duration = Duration::from_secs(10);

/// The fixture binary is a workspace package, not a test fixture: build it
/// once, lazily, exactly as the sibling 16-3 suites do.
static WORKER_FIXTURE_BUILT: LazyLock<()> = LazyLock::new(|| {
    // `resolve_cli_binary` searches beside the running test executable, so
    // the fixture must use the same debug/release profile as this corpus.
    let mut command = std::process::Command::new("cargo");
    command
        .args(["build", "-q", "-p", "worker", "--bin", "worker-cli-fixture"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let output = command
        .output()
        .expect("run cargo build for worker fixture");
    assert!(
        output.status.success(),
        "build worker-cli-fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
});

// ────────────────────────────────────────────────────────────────────────────
// The in-process kernel world: a REAL scheduler, REAL crash detector, REAL
// Transparency Log — and `RootSupervision::arm`, which is what puts the
// ProgressWatchdog and the progress stamper in the loop before any Worker
// exists. Without `arm`, nothing in this world can ever emit `task.stalled`.
// ────────────────────────────────────────────────────────────────────────────

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    crash_detector: Arc<maos_kernel_core::supervision::CrashDetector>,
    iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
    telemetry: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
    notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
}

impl World {
    fn new(dir: &Path) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
        let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let capability = Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x16; 32]),
                NONCE,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
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
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                    .expect("open shared memory"),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .expect("open principal namespace index"),
            ),
            Arc::clone(&tl),
        ));
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
            Arc::clone(&tl),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            memory,
            Arc::clone(&iac),
            Arc::clone(&halt_registry),
            Arc::clone(&telemetry),
            None,
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
            Arc::clone(&tl),
            Arc::clone(&halt_registry),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&telemetry),
            journal,
        ));
        // Before any clone of the scheduler Arc: `set_crash_detector` needs the
        // strong count at 1 (`Arc::get_mut`).
        Arc::get_mut(&mut scheduler)
            .expect("scheduler Arc strong_count == 1")
            .set_crash_detector(Arc::clone(&crash_detector));
        Self {
            tl,
            capability,
            scheduler,
            halt_registry,
            crash_detector,
            iac,
            telemetry,
            notification_dispatcher: Arc::new(
                maos_director_surface::notification::NotificationDispatcher::default(),
            ),
        }
    }
}

fn scratch_dir(tag: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("maos-hang-corpus-16-3-{tag}-{nonce}"));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The only known-admitting `[cli_wrapper]` shape: the built-in host grant
/// names `worker-cli-fixture` exactly, so no machine-local `MAOS_HOST_GRANTS`
/// file can change the outcome. Deliberately NO `[supervision]` section — the
/// corpus runs at the DEFAULT 30 000 ms progress threshold, which is the
/// number NFR-Rel-2's ">30s" names.
fn write_manifest(dir: &Path, mode: &str) -> PathBuf {
    let path = dir.join(format!("worker-{mode}.toml"));
    std::fs::write(
        &path,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode={mode}\"]\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n"
        ),
    )
    .expect("write worker manifest");
    path
}

fn run_args(manifest: &Path) -> RunArgs {
    RunArgs {
        manifest_path: manifest.display().to_string(),
        live: false,
        once: true,
    }
}

/// Drive one Worker to its verdict on a blocking thread — the exact shape the
/// daemon's Worker-bearing roots use, so the corpus measures the production
/// path and nothing else.
#[allow(clippy::too_many_arguments)]
async fn run_worker_owned(
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    supervisor: Arc<maos_bin::supervision::WorkerSupervisor>,
    manifest: PathBuf,
    task: WorkerTask,
) -> Result<String, String> {
    let manifest_root: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).expect("read manifest"))
            .expect("parse manifest");
    let run = run_args(&manifest);
    tokio::task::spawn_blocking(move || {
        run_cli_wrapper_manifest(
            &manifest_root,
            &run,
            tl,
            capability,
            None,
            None,
            None,
            None,
            false,
            supervisor.as_ref(),
            task,
        )
        .map(|completion| completion.label().to_string())
        .map_err(|error| error.to_string())
    })
    .await
    .expect("worker blocking thread joined")
}

// ────────────────────────────────────────────────────────────────────────────
// The ONE poller. One keyset-cursor `query_frames` per tick over ALL new rows
// — the exclusive `(timestamp_ns, frame_id) >` cursor returns each row exactly
// once, so the poller's cost per tick is "what was journaled since last tick",
// never a per-pid sweep against the log's single mutex.
// ────────────────────────────────────────────────────────────────────────────

/// What the poller has seen, keyed by DISTINCT Worker.
#[derive(Default, Clone)]
struct Observed {
    /// `spirit_pid -> (task index, first stall receipt, total stall rows)`.
    /// Rows after the first are the watchdog's 2×-threshold refire and are
    /// deliberately NOT workers — the counts stay, but only for the report.
    stalls: HashMap<u32, (usize, Instant, u32)>,
    /// `spirit_pid -> first `lifecycle.crash` receipt`.
    crashes: HashMap<u32, Instant>,
    /// Poller-side query failures: never a panic (the poller must not take the
    /// test down from a transient read), never silent (the count is asserted).
    query_errors: u32,
}

fn spawn_poller(
    tl: Arc<TransparencyLogAdapter>,
    observed: Arc<Mutex<Observed>>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("hang-corpus-poller".to_string())
        .spawn(move || {
            // The keyset cursor: exclusive lower bound on (timestamp_ns,
            // frame_id). None = from the log's first row.
            let mut cursor: Option<(u64, [u8; 16])> = None;
            while !stop.load(Ordering::Relaxed) {
                let filter = FrameFilter {
                    cursor_timestamp_ns: cursor.map(|(ts, _)| ts),
                    cursor_frame_id: cursor.map(|(_, id)| id),
                    ..Default::default()
                };
                match tl.query_frames(filter) {
                    Ok(rows) => {
                        // Rows arrive in (timestamp_ns, frame_id) ASC order,
                        // so the last row's key is the new cursor position.
                        if let Some(last) = rows.last() {
                            cursor = Some((last.timestamp_ns, last.frame_id));
                        }
                        let mut seen = observed.lock().expect("observed lock poisoned");
                        for row in rows {
                            match row.kind {
                                // The watchdog's stall row is written AT the
                                // Worker's pid with the in-flight task id in
                                // the payload (progress_watchdog.rs).
                                FrameKind::TaskStalled => {
                                    let payload = String::from_utf8_lossy(&row.payload_redacted);
                                    let index = payload
                                        .split("\"first_in_flight_task_id\":\"topology:")
                                        .nth(1)
                                        .and_then(|rest| rest.split('"').next())
                                        .and_then(|digits| digits.parse::<usize>().ok());
                                    if let Some(index) = index {
                                        let entry = seen.stalls.entry(row.spirit_pid).or_insert((
                                            index,
                                            Instant::now(),
                                            0,
                                        ));
                                        entry.2 += 1;
                                    }
                                }
                                // The crash row is `lifecycle.crash` at the
                                // Worker's own pid (crash_detector.rs).
                                FrameKind::CapabilityInvocation
                                    if row.intent == "lifecycle.crash" =>
                                {
                                    seen.crashes
                                        .entry(row.spirit_pid)
                                        .or_insert_with(Instant::now);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_) => {
                        seen_error(&observed);
                    }
                }
                std::thread::sleep(POLL_TICK);
            }
        })
        .expect("spawn hang-corpus poller")
}

fn seen_error(observed: &Mutex<Observed>) {
    observed
        .lock()
        .expect("observed lock poisoned")
        .query_errors += 1;
}

/// Async wait until `predicate` holds or `budget` elapses. Returns whether
/// it held. The predicate is re-evaluated every 100 ms on the runtime's
/// timer — never a blocking sleep on a reactor thread.
async fn wait_until(budget: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + budget;
    loop {
        if predicate() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn median(sorted: &[u64]) -> u64 {
    match sorted.len() {
        0 => 0,
        1 => sorted[0],
        even if even % 2 == 0 => (sorted[even / 2 - 1] + sorted[even / 2]) / 2,
        odd => sorted[odd / 2],
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The corpus. ONE test: 60 concurrent Workers, the stall floor, the exclusive
// cursor's 5, the chatty 5's immunity, then the pidfd SIGKILL into the crash
// path for all 60.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "NFR-Rel-2 hang corpus: 60 live fixture Workers for ~75 s; run with `-- --ignored`"]
async fn sixty_workers_stall_by_silence_and_crash_by_signal() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("corpus");
    let world = World::new(&dir);
    // `arm` spawns BOTH the ProgressWatchdog and the progress stamper — the
    // pair that turns silence into `task.stalled` — before any Worker binds.
    let root = RootSupervision::arm(
        Arc::clone(&world.scheduler),
        Arc::clone(&world.crash_detector),
        Arc::clone(&world.tl),
        Arc::clone(&world.halt_registry),
        Arc::clone(&world.iac),
        Arc::clone(&world.telemetry),
        Arc::clone(&world.notification_dispatcher),
        NONCE,
    );
    let supervisor = Arc::clone(root.supervisor());
    // The kill phase reaches the children through their pidfds; on an
    // after-pump fallback root there are no pidfds and this corpus cannot run.
    assert_eq!(
        supervisor.strategy_name(),
        "pidfd",
        "the corpus kills through `ObservedChild::pidfd()`, so it requires the \
         pidfd strategy; the root fell back to {after_pump}",
        after_pump = supervisor.strategy_name(),
    );

    // One poller for the whole corpus; started before the first spawn so the
    // cursor sees every row from the log's beginning.
    let observed: Arc<Mutex<Observed>> = Arc::new(Mutex::new(Observed::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let poller = spawn_poller(
        Arc::clone(&world.tl),
        Arc::clone(&observed),
        Arc::clone(&stop),
    );

    // Three manifests, one per mode, all WITHOUT a `[supervision]` section:
    // the threshold under test is the documented default 30 000 ms.
    let hang_manifest = write_manifest(&dir, "hang");
    let line_manifest = write_manifest(&dir, "line-then-hang");
    let chatty_manifest = write_manifest(&dir, "hang-chatty");
    let manifest_for = |index: usize| -> PathBuf {
        if index < HANG_WORKERS {
            hang_manifest.clone()
        } else if index < HANG_WORKERS + LINE_WORKERS {
            line_manifest.clone()
        } else {
            chatty_manifest.clone()
        }
    };
    // Spawn all 60 CONCURRENTLY. Each `tokio::spawn` launches the run on the
    // blocking pool immediately; `spawn_instants[index]` is that Worker's own
    // t0 for the latency report.
    let spawn_instants: Vec<Instant> = (0..TOTAL_WORKERS).map(|_| Instant::now()).collect();
    let mut runs = Vec::with_capacity(TOTAL_WORKERS);
    for index in 0..TOTAL_WORKERS {
        runs.push(tokio::spawn(run_worker_owned(
            Arc::clone(&world.tl),
            Arc::clone(&world.capability),
            Arc::clone(&supervisor),
            manifest_for(index),
            WorkerTask::TopologyEntry { index },
        )));
    }

    // The denominator must be live before anything is measured: 60 bindings,
    // each with its spawned child (and thus its pidfd) in hand.
    let bound = wait_until(SPAWN_BUDGET, || {
        supervisor.bindings().len() == TOTAL_WORKERS
            && supervisor
                .bindings()
                .iter()
                .all(|view| view.child_pid.is_some())
    })
    .await;
    let bindings = supervisor.bindings();
    let bound_pids: Vec<u32> = bindings.iter().map(|view| view.spirit_pid).collect();
    assert!(
        bound,
        "only {}/{} Workers reached their spawn within {SPAWN_BUDGET:?}; \
         bound pids: {bound_pids:?}",
        bindings
            .iter()
            .filter(|view| view.child_pid.is_some())
            .count(),
        TOTAL_WORKERS,
    );
    assert_eq!(
        bindings.len(),
        TOTAL_WORKERS,
        "every spawned Worker must be bound before the stall window is measured"
    );
    let spirit_names: HashMap<u32, String> = bindings
        .iter()
        .map(|view| (view.spirit_pid, view.spirit_id.clone()))
        .collect();

    // ── The stall window ────────────────────────────────────────────────────
    // Runs until the last silent Worker has stalled, capped at HANG_WINDOW,
    // and never closes before CHATTY_MIN_WINDOW: the chatty Workers must be
    // observed clean across the whole period in which a false stall could
    // plausibly fire, not merely until the first silent Worker happens to
    // stall.
    let t0 = Instant::now();
    loop {
        let elapsed = t0.elapsed();
        let snapshot = observed.lock().expect("observed lock poisoned").clone();
        let silent_stalled: Vec<usize> = snapshot
            .stalls
            .values()
            .map(|(index, _, _)| *index)
            .filter(|index| *index < HANG_WORKERS + LINE_WORKERS)
            .collect();
        let all_silent_stalled = silent_stalled.len() == HANG_WORKERS + LINE_WORKERS;
        if (all_silent_stalled && elapsed >= CHATTY_MIN_WINDOW) || elapsed >= HANG_WINDOW {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let window = t0.elapsed();
    // Freeze the stall verdict AT the window's close: rows the watchdog fires
    // later (a late refire, a pathological late first stall) must not be able
    // to retroactively green a floor that closed red.
    let stall_snapshot: HashMap<u32, (usize, Instant, u32)> = observed
        .lock()
        .expect("observed lock poisoned")
        .stalls
        .clone();

    // Split by corpus arm, via the payload's `topology:<index>`; every tuple
    // carries the Worker's pid so reports can name the spirit.
    let hang_stalled: Vec<(u32, usize, Duration, u32)> = stall_snapshot
        .iter()
        .filter(|(_, (index, _, _))| *index < HANG_WORKERS)
        .map(|(pid, (index, at, rows))| {
            (
                *pid,
                *index,
                at.saturating_duration_since(spawn_instants[*index]),
                *rows,
            )
        })
        .collect();
    let line_stalled: Vec<(u32, usize, Duration, u32)> = stall_snapshot
        .iter()
        .filter(|(_, (index, _, _))| *index >= HANG_WORKERS && *index < HANG_WORKERS + LINE_WORKERS)
        .map(|(pid, (index, at, rows))| {
            (
                *pid,
                *index,
                at.saturating_duration_since(spawn_instants[*index]),
                *rows,
            )
        })
        .collect();
    let chatty_stalled: Vec<usize> = stall_snapshot
        .values()
        .map(|(index, _, _)| *index)
        .filter(|index| *index >= HANG_WORKERS + LINE_WORKERS)
        .collect();

    // The supervision pipeline's own view at window close — printed on every
    // run, and quoted in the floor's failure message. If the stall floor
    // fails, this names the stage that broke: an empty SCB map, a non-Running
    // state, an emptied in-flight ledger, or a fresh progress stamp each point
    // at a different link between "Worker is silent" and "watchdog emits".
    let pipeline = {
        let scbs = world.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let now_ns = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
        let mut running = 0u32;
        let mut with_ledger = 0u32;
        let mut stamp_ages_ms: Vec<u64> = Vec::new();
        for scb in guard.values() {
            if scb.current_state()
                == maos_kernel_core::scheduler::control_block::ScbLifecycleState::Running
            {
                running += 1;
            }
            let ledger_len = scb
                .task_assignments_in_flight
                .lock()
                .map(|ledger| ledger.len())
                .unwrap_or(0);
            if ledger_len > 0 {
                with_ledger += 1;
            }
            stamp_ages_ms.push(
                now_ns.saturating_sub(scb.last_progress_iac_ns.load(Ordering::Relaxed)) / 1_000_000,
            );
        }
        stamp_ages_ms.sort_unstable();
        format!(
            "{} SCBs, {} Running, {} with in-flight tasks, stamp-age ms \
             min/median/max = {}/{}/{}",
            guard.len(),
            running,
            with_ledger,
            stamp_ages_ms.first().copied().unwrap_or(0),
            median(&stamp_ages_ms),
            stamp_ages_ms.last().copied().unwrap_or(0),
        )
    };
    println!("supervision pipeline at window close: {pipeline}");

    // (1) NFR-Rel-2: ≥48/50 silent hang Workers stalled within the window.
    let mut hang_latencies: Vec<u64> = hang_stalled
        .iter()
        .map(|(_, _, ms, _)| ms.as_millis() as u64)
        .collect();
    hang_latencies.sort_unstable();
    println!(
        "hang Workers stalled: {}/{} (floor is 48) — latency-to-stall ms from \
         each Worker's own spawn: min/median/max = {}/{}/{}",
        hang_stalled.len(),
        HANG_WORKERS,
        hang_latencies.first().copied().unwrap_or(0),
        median(&hang_latencies),
        hang_latencies.last().copied().unwrap_or(0),
    );
    for (pid, index, latency, rows) in &hang_stalled {
        println!(
            "  hang topology:{index} ({}) stalled {} ms after its own spawn \
             ({} `task.stalled` row(s))",
            spirit_names
                .get(pid)
                .map(String::as_str)
                .unwrap_or("unbound pid"),
            latency.as_millis(),
            rows,
        );
    }
    assert!(
        hang_stalled.len() >= 48,
        "only {}/{} silent `hang` Workers have a `task.stalled` row within \
         {HANG_WINDOW:?} of spawn (NFR-Rel-2's floor is 48): a silent Worker \
         the supervision stack never stalls is the exact defect this corpus \
         refuses; observed latencies {hang_latencies:?} ms; supervision \
         pipeline at window close: {pipeline}",
        hang_stalled.len(),
        HANG_WORKERS,
    );

    // (2) The exclusive cursor's proof: 5/5 one-line-then-hang Workers stalled.
    println!(
        "line-then-hang Workers stalled: {}/{} — latency-to-stall ms {:?}",
        line_stalled.len(),
        LINE_WORKERS,
        line_stalled
            .iter()
            .map(|(pid, index, ms, _)| (
                spirit_names.get(pid).cloned().unwrap_or_default(),
                format!("topology:{index}"),
                ms.as_millis() as u64,
            ))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        line_stalled.len(),
        LINE_WORKERS,
        "every Worker that printed EXACTLY once and then hung must stall: \
         a stamper whose cursor re-returns old rows (inclusive `since_ns`) \
         re-stamps the Worker that printed last forever, and this assertion \
         is where that defect shows as a missing stall",
    );

    // (3) Slow is not hung: 0/5 chatty Workers stalled across a window that
    // lasted until the last silent Worker stalled (or 60 s) and ≥ 35 s.
    assert!(
        window >= CHATTY_MIN_WINDOW,
        "the chatty observation window closed after {window:?}, below the \
         {CHATTY_MIN_WINDOW:?} floor: a shorter window could miss a late \
         false stall",
    );
    println!(
        "hang-chatty Workers stalled: {}/{} over a {:?} window (threshold: \
         default 30 000 ms; every chatty Worker journaled output the whole \
         time)",
        chatty_stalled.len(),
        CHATTY_WORKERS,
        window,
    );
    assert!(
        chatty_stalled.is_empty(),
        "chatty Worker(s) {chatty_stalled:?} were `task.stalled` while printing \
         a line every second: an honest long-running agent CLI is slow, not \
         hung, and false-stalling it is the defect this half refuses",
    );
    let query_errors = observed
        .lock()
        .expect("observed lock poisoned")
        .query_errors;
    assert_eq!(
        query_errors, 0,
        "the poller's Transparency Log queries must not fail — failures could \
         hide stall or crash rows behind a read error",
    );

    // ── The kill phase ──────────────────────────────────────────────────────
    // SIGKILL every Worker through its binding's pidfd — the ruled harness
    // path (`ObservedChild::pidfd()` + `pidfd_send_signal`), safe from the
    // pid-reuse races a bare `kill(pid, SIGKILL)` carries — then require the
    // crash path: a `lifecycle.crash` row AT each Worker's own pid.
    let mut kills_sent = 0usize;
    for view in &bindings {
        let child: &ObservedChild = view
            .child
            .as_ref()
            .expect("pidfd strategy stores a child handle for every spawned Worker");
        rustix::process::pidfd_send_signal(child.pidfd(), rustix::process::Signal::KILL)
            .expect("pidfd_send_signal(SIGKILL) to the Worker's direct child");
        kills_sent += 1;
    }
    assert_eq!(
        kills_sent, TOTAL_WORKERS,
        "the denominator comes first: exactly {TOTAL_WORKERS} kills must be sent",
    );

    let crashes_landed = wait_until(CRASH_WINDOW, || {
        let seen = observed.lock().expect("observed lock poisoned");
        bound_pids.iter().all(|pid| seen.crashes.contains_key(pid))
    })
    .await;
    let crash_report = observed
        .lock()
        .expect("observed lock poisoned")
        .crashes
        .clone();
    let missing_crash: Vec<String> = bound_pids
        .iter()
        .filter(|pid| !crash_report.contains_key(pid))
        .map(|pid| {
            spirit_names
                .get(pid)
                .cloned()
                .unwrap_or_else(|| format!("pid {pid}"))
        })
        .collect();
    assert!(
        crashes_landed,
        "SIGKILLed Worker(s) {missing_crash:?} never reached the crash path: \
         {}/{} have a `lifecycle.crash` row at their own pid — a stalled \
         Worker the supervisor cannot crash-handle is the defect this phase \
         refuses",
        crash_report.len(),
        TOTAL_WORKERS,
    );
    let slowest_crash = crash_report
        .values()
        .map(|at| at.saturating_duration_since(t0))
        .max()
        .unwrap_or_default();
    println!(
        "crash path: {}/{} SIGKILLed Workers wrote `lifecycle.crash` at their \
         own pid (slowest {} ms after the stall window opened); {kills_sent} \
         kills sent",
        crash_report.len(),
        TOTAL_WORKERS,
        slowest_crash.as_millis(),
    );

    // Every run reached its verdict — nothing wedged behind the kills.
    let mut run_errors: Vec<String> = Vec::new();
    for (index, run) in runs.into_iter().enumerate() {
        match tokio::time::timeout(Duration::from_secs(20), run).await {
            Ok(joined) => match joined {
                Ok(Ok(_label)) => {}
                Ok(Err(error)) => run_errors.push(format!("topology:{index}: {error}")),
                Err(join) => run_errors.push(format!("topology:{index}: join {join}")),
            },
            Err(_) => run_errors.push(format!(
                "topology:{index}: run did not finish within 20 s of its SIGKILL"
            )),
        }
    }
    assert!(
        run_errors.is_empty(),
        "every killed Worker's run must complete with a verdict, got errors: \
         {run_errors:?}",
    );

    // Ruled teardown order: the poller first (it must not observe the
    // teardown's own unload rows), then the root's stop-and-join.
    stop.store(true, Ordering::Relaxed);
    poller.join().expect("poller thread joins");
    root.stop_and_join().await;
}
