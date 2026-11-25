//! Story 22-4 — the directed-task registry, in-process: its bounds, idempotency, the halt and
//! orphan correlations, the unload and pause behaviour, and log-before-deliver. The Spirit's
//! turn and the kernel are a scripted host and runner here; the real halt registry stands in
//! for D-22-4-09's correlation (`halt_for_task` is the code `KernelTaskHost` calls).
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use maos_bin::directed_task::{
    ask_replay_key, halt_for_task, AcceptError, DirectedTasks, RunFailure, SpiritRunState,
    TaskHalt, TaskHost, TaskLimits, TaskRecord, TaskRunner, TaskState,
};

const WAIT: Duration = Duration::from_secs(20);

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let until = Instant::now() + WAIT;
    while !done() {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The scripted kernel: Spirit states, a Transparency-Log stand-in and an event trace.
#[derive(Default)]
struct FakeHost {
    states: Mutex<HashMap<String, SpiritRunState>>,
    /// `accepted:<id>`, `run:<id>`, `terminal:<id>:<state>` in the order they happened.
    events: Arc<Mutex<Vec<String>>>,
    accepted: Mutex<HashMap<String, TaskRecord>>,
    terminals: Mutex<HashMap<String, TaskRecord>>,
    /// The reply rows: task id -> journaled answer.
    replies: Mutex<HashMap<String, String>>,
    /// key hash -> task id, as the door's completion rows would answer after a restart.
    journaled_keys: Mutex<HashMap<String, String>>,
    /// Every task ends with a `task.orphaned` row.
    orphan_all: Mutex<bool>,
    fail_accept_row: Mutex<bool>,
    halts: Mutex<Option<(Arc<maos_kernel_core::halt::HaltRegistry>, u32)>>,
    /// Pretend the halt clock starts at zero (the plant of `a_halt_before_the_task_started_is_not`).
    ignore_since: Mutex<bool>,
}

impl FakeHost {
    fn state(&self, spirit: &str, state: SpiritRunState) {
        self.states.lock().unwrap().insert(spirit.into(), state);
    }

    fn trace(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn count(&self, prefix: &str) -> usize {
        self.trace()
            .iter()
            .filter(|e| e.starts_with(prefix))
            .count()
    }
}

impl TaskHost for FakeHost {
    fn spirit_state(&self, spirit_id: &str) -> SpiritRunState {
        self.states
            .lock()
            .unwrap()
            .get(spirit_id)
            .copied()
            .unwrap_or(SpiritRunState::Running)
    }

    fn now_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn write_accepted(&self, record: &TaskRecord) -> Result<(), String> {
        if *self.fail_accept_row.lock().unwrap() {
            return Err("the log refused the row".into());
        }
        self.events
            .lock()
            .unwrap()
            .push(format!("accepted:{}", record.task_id));
        self.accepted
            .lock()
            .unwrap()
            .insert(record.task_id.clone(), record.clone());
        Ok(())
    }

    fn write_terminal(&self, record: &TaskRecord) -> Result<(), String> {
        self.events.lock().unwrap().push(format!(
            "terminal:{}:{}",
            record.task_id,
            record.state.as_str()
        ));
        self.terminals
            .lock()
            .unwrap()
            .insert(record.task_id.clone(), record.clone());
        Ok(())
    }

    fn reply_result(&self, task_id: &str) -> Option<(String, u64)> {
        let result = self.replies.lock().unwrap().get(task_id).cloned()?;
        Some((result, self.now_ns()))
    }

    fn orphan_row(&self, _task_id: &str) -> bool {
        *self.orphan_all.lock().unwrap()
    }

    fn halt_since(&self, _spirit_id: &str, since_ns: u64) -> Option<TaskHalt> {
        let (registry, pid) = self.halts.lock().unwrap().clone()?;
        let since = if *self.ignore_since.lock().unwrap() {
            0
        } else {
            since_ns
        };
        halt_for_task(&registry, pid, since)
    }

    fn find_key(&self, key_hash: &str) -> Option<String> {
        self.journaled_keys.lock().unwrap().get(key_hash).cloned()
    }

    /// The audit trail's reading of a task: the same precedence the real host applies.
    fn read_back(&self, task_id: &str) -> Option<TaskRecord> {
        let mut record = self.accepted.lock().unwrap().get(task_id)?.clone();
        if let Some(terminal) = self.terminals.lock().unwrap().get(task_id) {
            return Some(terminal.clone());
        }
        if let Some(result) = self.replies.lock().unwrap().get(task_id) {
            record.state = TaskState::Completed;
            record.result = Some(result.clone());
        } else {
            record.state = TaskState::Interrupted;
        }
        Some(record)
    }
}

/// A Spirit that holds every turn until the gate opens.
#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
    started: AtomicUsize,
}

impl Gate {
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

struct GatedRunner {
    gate: Arc<Gate>,
    events: Arc<Mutex<Vec<String>>>,
    host: Arc<FakeHost>,
}

impl TaskRunner for GatedRunner {
    fn run(&self, task_id: [u8; 16], text: &str) -> Result<String, RunFailure> {
        let id = ulid::Ulid::from_bytes(task_id).to_string();
        self.events.lock().unwrap().push(format!("run:{id}"));
        self.gate.started.fetch_add(1, Ordering::SeqCst);
        let mut open = self.gate.open.lock().unwrap();
        let until = Instant::now() + WAIT;
        while !*open {
            let left = until.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "the gate was never released");
            open = self.gate.changed.wait_timeout(open, left).unwrap().0;
        }
        // The turn's reply row, journaled as the real session would before the turn ends.
        self.host
            .replies
            .lock()
            .unwrap()
            .insert(id, format!("answer to {text}"));
        Ok(format!("live answer to {text}"))
    }
}

/// A Spirit whose turn ends at once with `outcome`.
struct ScriptedRunner {
    host: Arc<FakeHost>,
    outcome: Result<String, RunFailure>,
    /// Runs inside the turn, before it ends.
    during: Option<Box<dyn Fn() + Send + Sync>>,
}

impl TaskRunner for ScriptedRunner {
    fn run(&self, task_id: [u8; 16], _text: &str) -> Result<String, RunFailure> {
        let id = ulid::Ulid::from_bytes(task_id).to_string();
        self.host.events.lock().unwrap().push(format!("run:{id}"));
        if let Some(during) = &self.during {
            during();
        }
        if let Ok(answer) = &self.outcome {
            // The reply row the real session journals before the turn ends.
            self.host.replies.lock().unwrap().insert(id, answer.clone());
        }
        self.outcome.clone()
    }
}

fn limits() -> TaskLimits {
    TaskLimits {
        orphan_grace: Duration::ZERO,
        ..TaskLimits::default()
    }
}

fn world() -> (Arc<FakeHost>, Arc<DirectedTasks>) {
    let host = Arc::new(FakeHost::default());
    let tasks = DirectedTasks::new(host.clone(), limits());
    (host, tasks)
}

fn gated(host: &Arc<FakeHost>, tasks: &Arc<DirectedTasks>, spirit: &str) -> Arc<Gate> {
    let gate = Arc::new(Gate::default());
    tasks.register(
        spirit,
        Arc::new(GatedRunner {
            gate: Arc::clone(&gate),
            events: Arc::clone(&host.events),
            host: Arc::clone(host),
        }),
    );
    gate
}

fn scripted(
    host: &Arc<FakeHost>,
    tasks: &Arc<DirectedTasks>,
    spirit: &str,
    outcome: Result<String, RunFailure>,
    during: Option<Box<dyn Fn() + Send + Sync>>,
) {
    tasks.register(
        spirit,
        Arc::new(ScriptedRunner {
            host: Arc::clone(host),
            outcome,
            during,
        }),
    );
}

fn finished(tasks: &Arc<DirectedTasks>, id: &str) -> TaskRecord {
    wait_until(&format!("task {id} to end"), || {
        tasks
            .get(id)
            .is_some_and(|record| !matches!(record.state, TaskState::Queued | TaskState::Running))
    });
    tasks.get(id).unwrap()
}

#[test]
fn queue_refuses_past_its_bound_and_never_blocks() {
    let (host, tasks) = world();
    let gate = gated(&host, &tasks, "probe");

    let first = tasks.accept("probe", "task 0", None).unwrap();
    wait_until("the first task to run", || {
        gate.started.load(Ordering::SeqCst) == 1
    });
    for n in 1..=32 {
        tasks
            .accept("probe", &format!("task {n}"), None)
            .unwrap_or_else(|error| panic!("queued task {n} refused: {error:?}"));
    }
    assert_eq!(host.count("accepted:"), 33, "one running plus 32 queued");

    // The 33rd queued ask is refused while the Spirit is stuck mid-turn: the door handler
    // never waits on the Spirit, the mailbox or the pump.
    let before = Instant::now();
    let refused = tasks.accept("probe", "one too many", None);
    assert!(
        before.elapsed() < Duration::from_secs(2),
        "the refusal waited {:?}",
        before.elapsed()
    );
    assert_eq!(
        refused.unwrap_err(),
        AcceptError::QueueFull {
            scope: "spirit",
            limit: 32
        }
    );
    // Nothing was minted, logged or delivered for it.
    assert_eq!(host.count("accepted:"), 33);
    assert_eq!(tasks.counts().queued, 32);
    assert_eq!(tasks.counts().running, 1);
    assert_eq!(gate.started.load(Ordering::SeqCst), 1, "one task at a time");

    // A refusal frees nothing and loses nothing: release the Spirit and all 33 deliver, once, in
    // accept order.
    gate.release();
    wait_until("all 33 tasks to end", || {
        tasks.counts().queued == 0 && tasks.counts().running == 0
    });
    assert_eq!(gate.started.load(Ordering::SeqCst), 33);
    let accepted: Vec<String> = host
        .trace()
        .into_iter()
        .filter_map(|event| event.strip_prefix("accepted:").map(str::to_owned))
        .collect();
    let ran: Vec<String> = host
        .trace()
        .into_iter()
        .filter_map(|event| event.strip_prefix("run:").map(str::to_owned))
        .collect();
    assert_eq!(ran, accepted, "FIFO, each delivered exactly once");
    assert_eq!(accepted[0], first.task_id);
}

#[test]
fn the_host_wide_bound_refuses_across_spirits() {
    let host = Arc::new(FakeHost::default());
    let tasks = DirectedTasks::new(
        host.clone(),
        TaskLimits {
            host: 40,
            ..limits()
        },
    );
    let a = gated(&host, &tasks, "alpha");
    let b = gated(&host, &tasks, "beta");
    tasks.accept("alpha", "a0", None).unwrap();
    tasks.accept("beta", "b0", None).unwrap();
    wait_until("both to run", || {
        a.started.load(Ordering::SeqCst) == 1 && b.started.load(Ordering::SeqCst) == 1
    });
    for n in 0..32 {
        tasks.accept("alpha", &format!("a{n}"), None).unwrap();
    }
    for n in 0..6 {
        tasks.accept("beta", &format!("b{n}"), None).unwrap();
    }
    // 33 + 7 = 40 queued and running; beta's own queue has room, the host does not.
    assert_eq!(
        tasks.accept("beta", "over", None).unwrap_err(),
        AcceptError::QueueFull {
            scope: "host",
            limit: 40
        }
    );
    a.release();
    b.release();
}

#[test]
fn a_repeated_key_returns_the_same_task() {
    let (host, tasks) = world();
    let gate = gated(&host, &tasks, "probe");
    scripted(&host, &tasks, "other", Ok("ok".into()), None);

    let first = tasks.accept("probe", "do it", Some("retry-1")).unwrap();
    let second = tasks.accept("probe", "do it", Some("retry-1")).unwrap();
    assert!(!first.replayed);
    assert!(second.replayed, "the repeat is marked");
    assert_eq!(first.task_id, second.task_id, "the same task id");
    wait_until("the task to run", || {
        gate.started.load(Ordering::SeqCst) == 1
    });
    assert_eq!(
        host.count("accepted:"),
        1,
        "the repeat logged and queued nothing"
    );

    // A key is scoped to its Spirit and to its value; no key is always a new task.
    let other_key = tasks.accept("probe", "do it", Some("retry-2")).unwrap();
    let other_spirit = tasks.accept("other", "do it", Some("retry-1")).unwrap();
    let keyless_a = tasks.accept("probe", "do it", None).unwrap();
    let keyless_b = tasks.accept("probe", "do it", None).unwrap();
    let ids: HashSet<&str> = [
        first.task_id.as_str(),
        other_key.task_id.as_str(),
        other_spirit.task_id.as_str(),
        keyless_a.task_id.as_str(),
        keyless_b.task_id.as_str(),
    ]
    .into();
    assert_eq!(ids.len(), 5, "five distinct tasks");

    // A key an earlier boot accepted is found in the audit trail: the same id, nothing queued.
    let earlier = ulid::Ulid::new().to_string();
    host.accepted.lock().unwrap().insert(
        earlier.clone(),
        TaskRecord {
            task_id: earlier.clone(),
            spirit_id: "probe".into(),
            origin: "door".into(),
            actor: "director".into(),
            state: TaskState::Queued,
            result: None,
            halt: None,
            reason: None,
            accepted_ns: 1,
            ended_ns: None,
        },
    );
    host.journaled_keys
        .lock()
        .unwrap()
        .insert(ask_replay_key("probe", "from-last-boot"), earlier.clone());
    let before = host.count("accepted:");
    let replay = tasks
        .accept("probe", "do it", Some("from-last-boot"))
        .unwrap();
    assert_eq!(replay.task_id, earlier);
    assert!(replay.replayed);
    assert_eq!(
        replay.state,
        TaskState::Interrupted,
        "an earlier boot's unfinished task reads interrupted and is not re-run"
    );
    assert_eq!(host.count("accepted:"), before);
    gate.release();
}

#[test]
fn a_halt_on_the_target_during_the_task_is_reported() {
    let (host, tasks) = world();
    let journal_dir = tempfile::tempdir().unwrap();
    let log = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(0xCAFE));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&journal_dir.path().join("j.ndjson"))
            .unwrap(),
    );
    let registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    const PID: u32 = 4242;
    *host.halts.lock().unwrap() = Some((Arc::clone(&registry), PID));
    let (during_registry, during_log, during_journal) = (
        Arc::clone(&registry),
        Arc::clone(&log),
        Arc::clone(&journal),
    );
    scripted(
        &host,
        &tasks,
        "probe",
        // The Spirit answers, but the kernel recorded a halt on it while the task ran.
        Ok("an answer the halt overtook".into()),
        Some(Box::new(move || {
            let payload = maos_domain::frame::EpistemicHaltPayload::new(
                "halt-during".into(),
                "claim.security".into(),
                0.91,
                Some(0.8),
                "pol-1".into(),
                "frame:abc".into(),
            )
            .unwrap();
            maos_kernel_core::halt::invoke_halt(
                &during_log,
                &during_journal,
                &during_registry,
                payload,
                PID,
                "probe",
                1,
            )
            .unwrap();
        })),
    );
    let accepted = tasks.accept("probe", "risky", None).unwrap();
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Halted, "{record:?}");
    assert_eq!(
        record.halt,
        Some(TaskHalt {
            halt_id: "halt-during".into(),
            state: "PendingResolution".into()
        })
    );
    assert_eq!(record.result, None, "a halted task carries no answer");
    assert_eq!(host.count("terminal:"), 1, "its end is journaled");

    drop(journal_dir);
}

#[test]
fn a_halt_before_the_task_started_is_not() {
    let (host, tasks) = world();
    let journal_dir = tempfile::tempdir().unwrap();
    let log = maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(0xCAFE);
    let journal =
        maos_kernel_core::journal::JournalAdapter::open(&journal_dir.path().join("j.ndjson"))
            .unwrap();
    let registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    const PID: u32 = 4243;
    // The halt is on the target's pid, recorded before the task is even accepted.
    let payload = maos_domain::frame::EpistemicHaltPayload::new(
        "halt-before".into(),
        "claim.security".into(),
        0.91,
        Some(0.8),
        "pol-1".into(),
        "frame:abc".into(),
    )
    .unwrap();
    maos_kernel_core::halt::invoke_halt(&log, &journal, &registry, payload, PID, "probe", 1)
        .unwrap();
    std::thread::sleep(Duration::from_millis(5));
    *host.halts.lock().unwrap() = Some((registry, PID));
    scripted(&host, &tasks, "probe", Ok("fine".into()), None);

    let accepted = tasks.accept("probe", "calm", None).unwrap();
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(
        record.state,
        TaskState::Completed,
        "an earlier halt is not the task's: {record:?}"
    );
    assert_eq!(record.halt, None);
    assert_eq!(
        host.count("terminal:"),
        0,
        "a completed task needs no terminal row"
    );
    drop(journal_dir);
}

#[test]
fn the_journaled_answer_wins_and_a_completed_task_reads_the_same_after_eviction() {
    let (host, tasks) = world();
    let gate = gated(&host, &tasks, "probe");
    gate.release();
    let accepted = tasks.accept("probe", "hello", None).unwrap();
    let record = finished(&tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Completed);
    // The runner returned "live answer to hello"; the reply row said otherwise, and the row is
    // what `task show` returns, live or after a restart.
    assert_eq!(record.result.as_deref(), Some("answer to hello"));
    assert_eq!(
        host.read_back(&accepted.task_id).unwrap().result,
        record.result,
        "the audit trail reads the same bytes"
    );
    assert_eq!(record.origin, "door");
    assert!(record.ended_ns.unwrap() >= record.accepted_ns);
}

#[test]
fn every_task_is_journaled_before_it_is_delivered() {
    let (host, tasks) = world();
    scripted(&host, &tasks, "probe", Ok("ok".into()), None);
    for n in 0..20 {
        tasks.accept("probe", &format!("t{n}"), None).unwrap();
    }
    wait_until("the tasks to run", || host.count("run:") == 20);
    let trace = host.trace();
    for event in trace.iter().filter(|e| e.starts_with("run:")) {
        let id = event.strip_prefix("run:").unwrap();
        let accepted = trace.iter().position(|e| e == &format!("accepted:{id}"));
        let ran = trace.iter().position(|e| e == event);
        assert!(
            accepted.is_some() && accepted < ran,
            "{id} was delivered before its accept row: {trace:?}"
        );
    }
}

#[test]
fn a_refused_accept_row_queues_nothing_and_frees_its_key() {
    let (host, tasks) = world();
    scripted(&host, &tasks, "probe", Ok("ok".into()), None);
    *host.fail_accept_row.lock().unwrap() = true;
    assert_eq!(
        tasks.accept("probe", "x", Some("k")).unwrap_err(),
        AcceptError::Log("the log refused the row".into())
    );
    assert_eq!(tasks.counts().queued, 0);
    *host.fail_accept_row.lock().unwrap() = false;
    let retried = tasks.accept("probe", "x", Some("k")).unwrap();
    assert!(!retried.replayed, "the failed accept left no key behind");
    assert_eq!(
        finished(&tasks, &retried.task_id).state,
        TaskState::Completed
    );
    assert_eq!(host.count("run:"), 1);
}

#[test]
fn an_unregistered_spirit_is_not_directable_and_mints_nothing() {
    let (host, tasks) = world();
    assert!(!tasks.is_directable("nash"));
    assert_eq!(
        tasks.accept("nash", "x", None).unwrap_err(),
        AcceptError::NotDirectable
    );
    assert_eq!(host.count("accepted:"), 0);
}

#[test]
fn unloading_a_spirit_ends_its_queued_tasks_failed_and_not_the_running_one() {
    let (host, tasks) = world();
    let gate = gated(&host, &tasks, "probe");
    let running = tasks.accept("probe", "running", None).unwrap();
    wait_until("it to run", || gate.started.load(Ordering::SeqCst) == 1);
    let queued: Vec<String> = (0..3)
        .map(|n| {
            tasks
                .accept("probe", &format!("q{n}"), None)
                .unwrap()
                .task_id
        })
        .collect();

    tasks.spirit_unloaded("probe");
    for id in &queued {
        let record = tasks.get(id).unwrap();
        assert_eq!(record.state, TaskState::Failed, "{record:?}");
        assert_eq!(record.reason.as_deref(), Some("spirit_unloaded"));
        assert!(host.terminals.lock().unwrap().contains_key(id));
    }
    assert_eq!(
        tasks.get(&running.task_id).unwrap().state,
        TaskState::Running
    );
    gate.release();
    assert_eq!(
        finished(&tasks, &running.task_id).state,
        TaskState::Completed
    );
    assert_eq!(
        gate.started.load(Ordering::SeqCst),
        1,
        "the queued three never ran"
    );
}

#[test]
fn a_paused_spirit_holds_its_queue_and_a_quarantined_or_unloaded_one_fails_it() {
    let (host, tasks) = world();
    scripted(&host, &tasks, "probe", Ok("ok".into()), None);
    host.state("probe", SpiritRunState::NotRunning("Paused"));
    let held = tasks.accept("probe", "wait for me", None).unwrap();
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(tasks.get(&held.task_id).unwrap().state, TaskState::Queued);
    assert_eq!(
        host.count("run:"),
        0,
        "nothing is delivered to a paused Spirit"
    );
    host.state("probe", SpiritRunState::Running);
    assert_eq!(finished(&tasks, &held.task_id).state, TaskState::Completed);

    host.state("probe", SpiritRunState::Quarantined);
    let quarantined = tasks.accept("probe", "x", None).unwrap();
    let record = finished(&tasks, &quarantined.task_id);
    assert_eq!(
        (record.state, record.reason.as_deref()),
        (TaskState::Failed, Some("spirit_quarantined"))
    );
    host.state("probe", SpiritRunState::Unloaded);
    let gone = tasks.accept("probe", "x", None).unwrap();
    let record = finished(&tasks, &gone.task_id);
    assert_eq!(
        (record.state, record.reason.as_deref()),
        (TaskState::Failed, Some("spirit_unloaded"))
    );
    assert_eq!(
        host.count("run:"),
        1,
        "only the first task was ever delivered"
    );
}

#[test]
fn a_turn_that_gave_no_answer_ends_with_its_typed_reason() {
    for reason in [
        "deadline_exceeded",
        "session_changed",
        "no_answer",
        "dispatch_refused",
    ] {
        let (host, tasks) = world();
        scripted(
            &host,
            &tasks,
            "probe",
            Err(RunFailure {
                reason,
                detail: "scripted".into(),
            }),
            None,
        );
        let accepted = tasks.accept("probe", "x", None).unwrap();
        let record = finished(&tasks, &accepted.task_id);
        assert_eq!(record.state, TaskState::Failed, "{reason}");
        assert_eq!(record.reason.as_deref(), Some(reason));
        assert_eq!(record.result, None);
        assert_eq!(host.count("terminal:"), 1, "{reason}: the end is journaled");
    }
}

#[test]
fn a_session_that_ended_on_a_crash_reads_orphaned_from_the_kernel_row() {
    let (host, tasks) = world();
    scripted(
        &host,
        &tasks,
        "probe",
        Err(RunFailure {
            reason: "session_ended",
            detail: "the child died".into(),
        }),
        None,
    );
    *host.orphan_all.lock().unwrap() = true;
    let crashed = tasks.accept("probe", "x", None).unwrap();
    let record = finished(&tasks, &crashed.task_id);
    assert_eq!(record.state, TaskState::Orphaned, "{record:?}");

    // Without the kernel's `task.orphaned` row the same end is a plain session end.
    *host.orphan_all.lock().unwrap() = false;
    let ended = tasks.accept("probe", "x", None).unwrap();
    let record = finished(&tasks, &ended.task_id);
    assert_eq!(
        (record.state, record.reason.as_deref()),
        (TaskState::Failed, Some("session_ended"))
    );
}

#[test]
fn the_finished_record_cache_never_exceeds_its_bound_and_evicted_tasks_read_from_the_log() {
    let (host, tasks) = world();
    scripted(&host, &tasks, "probe", Ok("ok".into()), None);
    let mut ids = Vec::new();
    for burst in 0..37 {
        for n in 0..30 {
            ids.push(
                tasks
                    .accept("probe", &format!("b{burst} t{n}"), None)
                    .unwrap()
                    .task_id,
            );
        }
        wait_until("the burst to end", || {
            let counts = tasks.counts();
            counts.queued == 0 && counts.running == 0
        });
        assert!(
            tasks.counts().finished_in_memory <= 1024,
            "the cache holds {}",
            tasks.counts().finished_in_memory
        );
    }
    assert_eq!(ids.len(), 1110);
    assert_eq!(tasks.counts().finished_in_memory, 1024);
    // The oldest left memory; the audit trail still answers for it.
    let oldest = host.read_back(&ids[0]).unwrap();
    assert_eq!(oldest.state, TaskState::Completed);
    assert_eq!(oldest.result.as_deref(), Some("ok"));
    assert_eq!(tasks.get(&ids[0]).unwrap().task_id, ids[0]);
    assert_eq!(tasks.get(&ids[1109]).unwrap().state, TaskState::Completed);
}

// ── the audit trail's read-back, over a real in-memory Transparency Log ───────

fn record(spirit: &str, state: TaskState) -> TaskRecord {
    TaskRecord {
        task_id: ulid::Ulid::new().to_string(),
        spirit_id: spirit.into(),
        origin: "door".into(),
        actor: "director".into(),
        state,
        result: None,
        halt: None,
        reason: None,
        accepted_ns: 1_000,
        ended_ns: None,
    }
}

#[test]
fn the_audit_trail_rebuilds_every_end_of_a_task() {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::FrameKind;
    let tl = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(7));
    let log = maos_bin::directed_task::TaskLog::new(Arc::clone(&tl));

    // Completed: the accept row and the reply row the session's route journals.
    let completed = record("probe", TaskState::Queued);
    log.write_accepted(&completed, 9);
    let id = ulid::Ulid::from_string(&completed.task_id).unwrap();
    let reply = serde_json::to_vec(&maos_domain::frame::FramePayload::TaskComplete(
        maos_domain::frame::TaskCompletePayload {
            result: "the journaled answer".into(),
        },
    ))
    .unwrap();
    let _ = tl.insert_frame_event_with_correlation(
        FrameKind::TaskComplete,
        9,
        None,
        &maos_iac::adapter::mailbox::reply_correlation(
            maos_iac::adapter::mailbox::ReplyLabel::Task,
            id.to_bytes(),
        ),
        "standard",
        &reply,
        FrameOrigin::SpiritAuto,
    );
    let rebuilt = log.read_back(&completed.task_id).unwrap();
    assert_eq!(rebuilt.state, TaskState::Completed);
    assert_eq!(rebuilt.result.as_deref(), Some("the journaled answer"));
    assert_eq!(rebuilt.spirit_id, "probe");
    assert_eq!(rebuilt.origin, "door");
    assert_eq!(rebuilt.accepted_ns, 1_000);
    let (journaled, row_ns) = log.reply_result(&completed.task_id).unwrap();
    assert_eq!(journaled, "the journaled answer");
    assert_eq!(
        Some(row_ns),
        rebuilt.ended_ns,
        "the end time is the reply row's"
    );
    // A shell reply row is not a task reply.
    let shell_only = record("probe", TaskState::Queued);
    log.write_accepted(&shell_only, 9);
    let shell_id = ulid::Ulid::from_string(&shell_only.task_id).unwrap();
    let _ = tl.insert_frame_event_with_correlation(
        FrameKind::TaskComplete,
        9,
        None,
        &maos_iac::adapter::mailbox::reply_correlation(
            maos_iac::adapter::mailbox::ReplyLabel::Shell,
            shell_id.to_bytes(),
        ),
        "standard",
        &reply,
        FrameOrigin::SpiritAuto,
    );
    assert_eq!(
        log.read_back(&shell_only.task_id).unwrap().state,
        TaskState::Interrupted
    );

    // Halted and failed: the terminal row carries the typed end.
    let mut halted = record("probe", TaskState::Halted);
    log.write_accepted(&halted, 9);
    halted.halt = Some(TaskHalt {
        halt_id: "halt-9".into(),
        state: "PendingResolution".into(),
    });
    halted.ended_ns = Some(2_000);
    log.write_terminal(&halted, 9);
    assert_eq!(log.read_back(&halted.task_id).unwrap(), halted);
    let mut failed = record("probe", TaskState::Failed);
    log.write_accepted(&failed, 9);
    failed.reason = Some("deadline_exceeded".into());
    failed.ended_ns = Some(3_000);
    log.write_terminal(&failed, 9);
    assert_eq!(log.read_back(&failed.task_id).unwrap(), failed);

    // Orphaned: the kernel's own `task.orphaned` row, as the crash detector writes it.
    let orphaned = record("probe", TaskState::Queued);
    log.write_accepted(&orphaned, 9);
    assert!(!log.orphan_row(&orphaned.task_id));
    let _ = tl.insert_frame_event(
        FrameKind::TaskComplete,
        9,
        None,
        "task.orphaned",
        serde_json::json!({
            "task_id": format!("turn:{}", orphaned.task_id),
            "originator_spirit_id": "@maos-shell",
            "cause": "exit",
        })
        .to_string()
        .as_bytes(),
        FrameOrigin::Kernel,
    );
    assert!(log.orphan_row(&orphaned.task_id));
    let rebuilt = log.read_back(&orphaned.task_id).unwrap();
    assert_eq!(rebuilt.state, TaskState::Orphaned);
    assert_eq!(rebuilt.reason.as_deref(), Some("session_ended"));

    // Accepted and never finished: interrupted. Never accepted: unknown.
    let stopped = record("probe", TaskState::Queued);
    log.write_accepted(&stopped, 9);
    assert_eq!(
        log.read_back(&stopped.task_id).unwrap().state,
        TaskState::Interrupted
    );
    assert!(log.read_back(&ulid::Ulid::new().to_string()).is_none());
}

#[test]
fn a_key_is_found_only_in_a_kernel_ask_completion_row() {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_kernel_core::iac::transparency_log::FrameKind;
    let tl = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(7));
    let log = maos_bin::directed_task::TaskLog::new(Arc::clone(&tl));
    let key = ask_replay_key("probe", "k1");
    let receipt = serde_json::json!({
        "operation_id": "op-1", "verb": "ask", "spirit_id": "probe",
        "outcome": "completed", "task_id": "01J9ZZZZZZZZZZZZZZZZZZZZZZ", "replayed": false,
    })
    .to_string();
    assert_eq!(log.find_key(&key), None);
    // A row under that correlation that a Spirit wrote is not the door's receipt.
    let _ = tl.insert_frame_event_with_correlation(
        FrameKind::TelemetryEvent,
        9,
        None,
        &key,
        "operator.ask.op-0:completed",
        receipt.as_bytes(),
        FrameOrigin::SpiritAuto,
    );
    assert_eq!(log.find_key(&key), None);
    let _ = tl.insert_frame_event_with_correlation(
        FrameKind::TelemetryEvent,
        9,
        None,
        &key,
        "operator.ask.op-1:completed",
        receipt.as_bytes(),
        FrameOrigin::Kernel,
    );
    assert_eq!(
        log.find_key(&key).as_deref(),
        Some("01J9ZZZZZZZZZZZZZZZZZZZZZZ")
    );
    // The key is the (spirit, key) pair.
    assert_eq!(log.find_key(&ask_replay_key("other", "k1")), None);
    assert_eq!(log.find_key(&ask_replay_key("probe", "k2")), None);
}
