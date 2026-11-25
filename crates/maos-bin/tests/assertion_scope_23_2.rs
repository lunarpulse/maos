//! Story 23-2 AC3 — the per-request assertion, in-process: the accept-time refusal, the per-pid
//! turn scope (D-23-2-04 A), the environment fallback (D-23-2-05 A), replay without re-binding,
//! and confidentiality of the carried credential. The real daemon's end-to-end witness is
//! `directed_task_22_4.rs`.
//!
//! Every test uses its own pids: the scope registry is process-wide, like the seam it proves.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use maos_bin::assertion_supply::{self, ScopeBusy};
use maos_bin::directed_task::{
    AcceptError, AssertionCheck, DirectedTasks, RunFailure, SpiritRunState, TaskHalt, TaskHost,
    TaskLimits, TaskRecord, TaskRunner, TaskState,
};
use maos_control::Assertion;

const CANARY: &str = "canary-head.canary-claims.canary-signature";
const OTHER: &str = "other-head.other-claims.other-signature";

fn assertion(text: &str) -> Assertion {
    Assertion::parse(text).expect("a compact JWT")
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        std::thread::yield_now();
    }
}

/// The kernel stand-in: it keeps every row it is asked to write, as text, so a test can search the
/// "log" for a credential.
#[derive(Default)]
struct Journal {
    rows: Mutex<Vec<String>>,
    replies: Mutex<HashMap<String, String>>,
}

impl TaskHost for Journal {
    fn spirit_state(&self, _spirit_id: &str) -> SpiritRunState {
        SpiritRunState::Running
    }
    fn now_ns(&self) -> u64 {
        1
    }
    fn write_accepted(&self, record: &TaskRecord) -> Result<(), String> {
        self.rows
            .lock()
            .unwrap()
            .push(format!("accepted {record:?} {:?}", record.to_row()));
        Ok(())
    }
    fn write_terminal(&self, record: &TaskRecord) -> Result<(), String> {
        self.rows
            .lock()
            .unwrap()
            .push(format!("terminal {record:?} {:?}", record.to_row()));
        Ok(())
    }
    fn reply_result(&self, task_id: &str) -> Option<(String, u64)> {
        Some((self.replies.lock().unwrap().get(task_id)?.clone(), 2))
    }
    fn orphan_row(&self, _task_id: &str) -> bool {
        false
    }
    fn halt_since(&self, _spirit_id: &str, _since_ns: u64) -> Option<TaskHalt> {
        None
    }
    fn find_key(&self, _key_hash: &str) -> Option<String> {
        None
    }
    fn read_back(&self, _task_id: &str) -> Option<TaskRecord> {
        None
    }
}

/// A runner that does what the production runners do: enter the turn scope for its Spirit's pid,
/// then "mint" (resolve the assertion the seam supplies for that pid and for a neighbour's).
struct Turn {
    pid: u32,
    neighbour: u32,
    journal: Arc<Journal>,
    /// (assertion handed to the runner, assertion the seam supplies for `pid`, for `neighbour`).
    seen: Mutex<Vec<(Option<String>, Option<String>, Option<String>)>>,
    /// Panic inside the turn, after entering the scope.
    panics: bool,
}

impl TaskRunner for Turn {
    fn run(
        &self,
        task_id: [u8; 16],
        _text: &str,
        carried: Option<&Assertion>,
    ) -> Result<String, RunFailure> {
        let _scope =
            assertion_supply::enter(self.pid, carried).map_err(|ScopeBusy| RunFailure {
                reason: "dispatch_refused",
                detail: "scope busy".into(),
            })?;
        self.seen.lock().unwrap().push((
            carried.map(|a| a.expose().to_owned()),
            assertion_supply::supply(self.pid),
            assertion_supply::supply(self.neighbour),
        ));
        assert!(!self.panics, "the turn panics inside its scope");
        let id = ulid::Ulid::from_bytes(task_id).to_string();
        self.journal
            .replies
            .lock()
            .unwrap()
            .insert(id, "answer".into());
        Ok("answer".into())
    }
}

/// Accepts exactly the assertions in `valid`.
struct Verifier {
    valid: Vec<&'static str>,
}

impl AssertionCheck for Verifier {
    fn check(&self, assertion: &Assertion) -> Result<(), ()> {
        self.valid
            .iter()
            .any(|valid| *valid == assertion.expose())
            .then_some(())
            .ok_or(())
    }
}

struct World {
    journal: Arc<Journal>,
    tasks: Arc<DirectedTasks>,
    turn: Arc<Turn>,
}

fn world(pid: u32, valid: Option<Vec<&'static str>>, panics: bool) -> World {
    let journal = Arc::new(Journal::default());
    let tasks = DirectedTasks::new(journal.clone(), TaskLimits::default());
    if let Some(valid) = valid {
        tasks.set_assertion_check(Arc::new(Verifier { valid }));
    }
    let turn = Arc::new(Turn {
        pid,
        neighbour: pid + 1,
        journal: Arc::clone(&journal),
        seen: Mutex::new(Vec::new()),
        panics,
    });
    tasks.register("probe", turn.clone());
    World {
        journal,
        tasks,
        turn,
    }
}

fn finished(tasks: &Arc<DirectedTasks>, id: &str) -> TaskRecord {
    wait_until("the task to end", || {
        tasks
            .get(id)
            .is_some_and(|record| !matches!(record.state, TaskState::Queued | TaskState::Running))
    });
    tasks.get(id).unwrap()
}

/// The scope belongs to one pid, ends with its guard, is never replaced while held, and a panic
/// unwinding through the turn releases it.
#[test]
fn a_scope_serves_its_own_pid_only_and_ends_on_every_exit() {
    let (pid, other) = (910_001, 910_002);
    let carried = assertion(CANARY);
    assert_eq!(assertion_supply::enter(pid, None).unwrap().is_none(), true);
    assert_eq!(assertion_supply::supply(pid).as_deref(), None);

    let scope = assertion_supply::enter(pid, Some(&carried))
        .unwrap()
        .unwrap();
    assert!(assertion_supply::is_scoped(pid));
    assert_eq!(assertion_supply::supply(pid).as_deref(), Some(CANARY));
    assert_ne!(assertion_supply::supply(other).as_deref(), Some(CANARY));
    // Two turns on one pid would mix principals: refused, and the first scope is intact.
    let rival = assertion(OTHER);
    assert_eq!(
        assertion_supply::enter(pid, Some(&rival)).unwrap_err(),
        ScopeBusy
    );
    assert_eq!(assertion_supply::supply(pid).as_deref(), Some(CANARY));
    drop(scope);
    assert!(!assertion_supply::is_scoped(pid));
    assert_ne!(assertion_supply::supply(pid).as_deref(), Some(CANARY));

    // A panic through the turn releases the scope too.
    let unwound = std::panic::catch_unwind(|| {
        let _scope = assertion_supply::enter(910_003, Some(&assertion(CANARY))).unwrap();
        panic!("the turn dies inside its scope");
    });
    assert!(unwound.is_err());
    assert!(!assertion_supply::is_scoped(910_003));
}

/// An assertion that does not verify, or that nothing can verify, is refused at accept and
/// queues nothing: no row, no task, no turn.
#[test]
fn an_invalid_assertion_is_refused_before_anything_is_queued() {
    for (valid, label) in [
        (Some(vec![OTHER]), "wrong assertion"),
        (None, "no verifier"),
    ] {
        let world = world(910_010, valid, false);
        let refused = world
            .tasks
            .accept("probe", "do it", None, Some(&assertion(CANARY)));
        assert_eq!(refused, Err(AcceptError::AssertionInvalid), "{label}");
        assert!(world.journal.rows.lock().unwrap().is_empty(), "{label}");
        let counts = world.tasks.counts();
        assert_eq!(
            (counts.queued, counts.running, counts.finished_in_memory),
            (0, 0, 0)
        );
        assert!(world.turn.seen.lock().unwrap().is_empty(), "{label}");
    }
}

/// D-23-2-04 A / D-23-2-05 A: the task's own assertion governs its turn's mints for its pid; a
/// neighbour's mints and a later task without an assertion resolve what the environment would.
#[test]
fn a_valid_assertion_governs_its_own_turn_only() {
    let pid = 910_020;
    let world = world(pid, Some(vec![CANARY]), false);
    let with = world
        .tasks
        .accept("probe", "first", None, Some(&assertion(CANARY)))
        .unwrap();
    finished(&world.tasks, &with.task_id);
    let without = world.tasks.accept("probe", "second", None, None).unwrap();
    finished(&world.tasks, &without.task_id);

    let seen = world.turn.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    let (carried, mint_for_pid, mint_for_neighbour) = &seen[0];
    assert_eq!(carried.as_deref(), Some(CANARY));
    assert_eq!(
        mint_for_pid.as_deref(),
        Some(CANARY),
        "its own pid mints under it"
    );
    assert_ne!(
        mint_for_neighbour.as_deref(),
        Some(CANARY),
        "a neighbour does not"
    );
    let (carried, mint_for_pid, _) = &seen[1];
    assert_eq!(carried, &None);
    assert_ne!(
        mint_for_pid.as_deref(),
        Some(CANARY),
        "a task without an assertion uses the environment"
    );
    assert!(
        !assertion_supply::is_scoped(pid),
        "no scope outlives its turn"
    );
}

/// A turn that panics still releases its scope, and the task ends typed.
#[test]
fn a_panicking_turn_releases_the_scope() {
    let pid = 910_030;
    let world = world(pid, Some(vec![CANARY]), true);
    let accepted = world
        .tasks
        .accept("probe", "boom", None, Some(&assertion(CANARY)))
        .unwrap();
    let record = finished(&world.tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Failed);
    assert!(!assertion_supply::is_scoped(pid));
}

/// The credential is in no record, no row and no `Debug` the registry produces.
#[test]
fn the_assertion_appears_in_no_record_row_or_debug() {
    let world = world(910_040, Some(vec![CANARY]), false);
    let accepted = world
        .tasks
        .accept("probe", "do it", Some("k-1"), Some(&assertion(CANARY)))
        .unwrap();
    let record = finished(&world.tasks, &accepted.task_id);
    assert_eq!(record.state, TaskState::Completed);
    let mut everything = format!("{accepted:?} {record:?} {:?}", record.to_row());
    for row in world.journal.rows.lock().unwrap().iter() {
        everything.push_str(row);
    }
    assert!(!everything.contains("canary"), "{everything}");
    assert!(!format!("{:?}", assertion(CANARY)).contains("canary"));
}

/// An idempotent replay with a different (valid) assertion returns the first task and does not
/// re-bind it: the Spirit's one turn ran under the first assertion only.
#[test]
fn a_replay_with_a_different_assertion_does_not_rebind_the_task() {
    let world = world(910_050, Some(vec![CANARY, OTHER]), false);
    let first = world
        .tasks
        .accept("probe", "once", Some("key-1"), Some(&assertion(CANARY)))
        .unwrap();
    let replay = world
        .tasks
        .accept("probe", "once", Some("key-1"), Some(&assertion(OTHER)))
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.task_id, first.task_id);
    finished(&world.tasks, &first.task_id);
    let seen = world.turn.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "the replay queued no second task");
    assert_eq!(seen[0].0.as_deref(), Some(CANARY));
    assert_eq!(seen[0].1.as_deref(), Some(CANARY));
    // A replay whose assertion is invalid is still refused, never answered.
    assert_eq!(
        world.tasks.accept(
            "probe",
            "once",
            Some("key-1"),
            Some(&assertion("bad-head.bad.bad"))
        ),
        Err(AcceptError::AssertionInvalid)
    );
}
