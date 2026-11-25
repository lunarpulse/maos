//! Story 22-4 — directed task ingress: an authenticated operator hands a named, running Spirit
//! one task and reads its result by task id.
//!
//! The door's `ask` verb ([`DirectedTasks::accept`]) enqueues into a bounded per-Spirit FIFO and
//! returns a task id at once. One pump thread per directable Spirit delivers one task at a time
//! through the Spirit's [`TaskRunner`], so the door handler never waits on the mailbox channel.
//! The task id is the ULID of the `TaskAssign` frame, so the Transparency Log row, the SCB
//! ledger (`turn:<ulid>`) and the kernel's `task.orphaned` row share it.
//!
//! Durable record (D-22-4-08 A): the accept row `task.<ulid>:accepted`, the `TaskAssign` row
//! (`frame_id` = task id), the reply row (correlation `maos.task-reply/<hex>`), the kernel's
//! `task.orphaned` row, and one terminal row `task.<ulid>:<state>` for every end that gave no
//! answer. [`TaskHost::read_back`] rebuilds a finished task from them, so a restart changes
//! nothing about what `GET /v1/tasks/{id}` returns; a task with an accept row and nothing else
//! reads `interrupted` and is never delivered again.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use maos_control::Assertion;

/// `origin` of a task handed over the operator door (23-5 adds its own origin).
pub const ORIGIN_DOOR: &str = "door";
/// `actor` of a door task until 24-1 replaces it with the operator principal.
pub const ACTOR_DIRECTOR: &str = "director";

/// The bounds of D-22-4-06 A, in one place.
#[derive(Debug, Clone, Copy)]
pub struct TaskLimits {
    /// Queued tasks per Spirit, beside the one running (below the mailbox's MPSC-64).
    pub per_spirit: usize,
    /// Queued and running tasks across the Host.
    pub host: usize,
    /// Finished records kept in memory (oldest first out; the Transparency Log has them all).
    pub finished: usize,
    /// How long a session end waits for the kernel's `task.orphaned` row before it is read
    /// as a plain `session_ended`.
    pub orphan_grace: Duration,
}

impl Default for TaskLimits {
    fn default() -> Self {
        Self {
            per_spirit: 32,
            host: 512,
            finished: 1024,
            orphan_grace: Duration::from_secs(2),
        }
    }
}

/// Where a task is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Halted,
    Failed,
    Orphaned,
    Interrupted,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Halted => "halted",
            Self::Failed => "failed",
            Self::Orphaned => "orphaned",
            Self::Interrupted => "interrupted",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "halted" => Self::Halted,
            "failed" => Self::Failed,
            "orphaned" => Self::Orphaned,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }

    fn is_finished(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}

/// The kernel halt recorded on the target while a task ran (D-22-4-09 A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskHalt {
    pub halt_id: String,
    pub state: String,
}

/// One directed task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub task_id: String,
    pub spirit_id: String,
    pub origin: String,
    pub actor: String,
    pub state: TaskState,
    pub result: Option<String>,
    pub halt: Option<TaskHalt>,
    pub reason: Option<String>,
    pub accepted_ns: u64,
    pub ended_ns: Option<u64>,
}

impl TaskRecord {
    pub fn to_row(&self) -> maos_control::TaskStatusRow {
        maos_control::TaskStatusRow {
            task_id: self.task_id.clone(),
            spirit_id: self.spirit_id.clone(),
            origin: self.origin.clone(),
            state: self.state.as_str().to_owned(),
            result: self.result.clone(),
            halt: self.halt.as_ref().map(|halt| maos_control::TaskHaltRow {
                halt_id: halt.halt_id.clone(),
                state: halt.state.clone(),
            }),
            reason: self.reason.clone(),
            accepted_ns: self.accepted_ns,
            ended_ns: self.ended_ns,
        }
    }
}

/// Why a turn gave no answer: a typed reason (`deadline_exceeded`, `session_ended`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFailure {
    pub reason: &'static str,
    pub detail: String,
}

/// One Spirit's turn-and-answer contract. `run` blocks until the Spirit's answer, or the typed
/// reason it gave none; it is only ever called from that Spirit's pump, one task at a time.
/// `task_id` is the host-minted `TaskAssign` frame id. `assertion` is the asserting principal's
/// own assertion when the task carried one (Story 23-2, D-23-2-04 A): the runner makes it the
/// assertion of every capability mint for its Spirit's pid for exactly the duration of the turn
/// ([`crate::assertion_supply::enter`]), and never delivers it to the Spirit.
pub trait TaskRunner: Send + Sync {
    fn run(
        &self,
        task_id: [u8; 16],
        text: &str,
        assertion: Option<&Assertion>,
    ) -> Result<String, RunFailure>;
}

/// A carried assertion was refused. Carries nothing: the door answers a typed
/// `sso_assertion_invalid` that never echoes the assertion or the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssertionRefused;

/// Story 23-2 (D-23-2-04 A) — verifies a carried assertion at accept.
pub trait AssertionCheck: Send + Sync {
    /// # Errors
    /// [`AssertionRefused`] for every refusal.
    fn check(&self, assertion: &Assertion) -> Result<(), AssertionRefused>;
}

/// What the pump needs to know about a Spirit before it delivers the next task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiritRunState {
    Running,
    /// Loaded or Paused: the queue holds until the Spirit runs.
    NotRunning(&'static str),
    Quarantined,
    Unloaded,
}

/// The registry's seam to the kernel: Spirit state, the Transparency Log and halts.
pub trait TaskHost: Send + Sync {
    fn spirit_state(&self, spirit_id: &str) -> SpiritRunState;
    /// Wall-clock nanoseconds since the UNIX epoch (the clock halts are stamped with).
    fn now_ns(&self) -> u64;
    /// Journal the accept row, before the task can be delivered (I2).
    fn write_accepted(&self, record: &TaskRecord) -> Result<(), String>;
    /// Journal the terminal row of a task that gave no answer.
    fn write_terminal(&self, record: &TaskRecord) -> Result<(), String>;
    /// The journaled (redacted) answer of `task_id` and the timestamp of its reply row, if the
    /// row exists.
    fn reply_result(&self, task_id: &str) -> Option<(String, u64)>;
    /// Whether the kernel journaled `task.orphaned` for `turn:<task_id>`.
    fn orphan_row(&self, task_id: &str) -> bool;
    /// A halt the kernel recorded on `spirit_id` at or after `since_ns`.
    fn halt_since(&self, spirit_id: &str, since_ns: u64) -> Option<TaskHalt>;
    /// The task an earlier boot accepted under this idempotency key hash.
    fn find_key(&self, key_hash: &str) -> Option<String>;
    /// A finished (or interrupted) task rebuilt from the audit trail.
    fn read_back(&self, task_id: &str) -> Option<TaskRecord>;
}

/// The door-side idempotency key: `operator.ask.replay:<sha256(spirit, key)>`. The door writes
/// it as the completion row's correlation (as the forget replay does).
pub fn ask_replay_key(spirit_id: &str, key: &str) -> String {
    use sha2::{Digest as _, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"maos.operator.ask.v1");
    hash.update((spirit_id.len() as u64).to_le_bytes());
    hash.update(spirit_id.as_bytes());
    hash.update((key.len() as u64).to_le_bytes());
    hash.update(key.as_bytes());
    format!("operator.ask.replay:{:x}", hash.finalize())
}

/// A task handed to the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub task_id: String,
    /// The key matched an earlier task: nothing was enqueued.
    pub replayed: bool,
    pub state: TaskState,
}

/// Why `accept` refused. None of these minted a task id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptError {
    /// No pump serves this Spirit: it is not a directable form.
    NotDirectable,
    /// `scope` (`spirit` or `host`) holds `limit` queued tasks already.
    QueueFull { scope: &'static str, limit: usize },
    /// The Host is stopping; nothing more is accepted.
    ShuttingDown,
    /// The accept row could not be journaled; nothing was enqueued.
    Log(String),
    /// A carried assertion failed verification (or this Host has nothing to verify it with);
    /// nothing was queued. Carries nothing.
    AssertionInvalid,
}

struct Live {
    record: TaskRecord,
    /// The request text, held until the task starts.
    text: Option<String>,
    /// The asserting principal's assertion (Story 23-2): in memory only, never in `record`, a
    /// row or `to_row`; dropped when the task starts, ends or is drained.
    assertion: Option<Assertion>,
    key_hash: Option<String>,
}

struct SpiritSlot {
    runner: Arc<dyn TaskRunner>,
    queue: VecDeque<String>,
    /// Reserved but not yet journaled: counts toward the bound, is not yet deliverable.
    pending: usize,
    running: Option<String>,
}

#[derive(Default)]
struct Inner {
    spirits: HashMap<String, SpiritSlot>,
    tasks: HashMap<String, Live>,
    finished: VecDeque<String>,
    keys: HashMap<String, String>,
    shutdown: bool,
}

/// Occupancy, for the tests and the doctor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskCounts {
    pub queued: usize,
    pub running: usize,
    pub finished_in_memory: usize,
}

/// The Host's directed tasks: per-Spirit bounded FIFOs, their pumps and the task records.
pub struct DirectedTasks {
    host: Arc<dyn TaskHost>,
    limits: TaskLimits,
    inner: Mutex<Inner>,
    wake: Condvar,
    threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
    assertion_check: Mutex<Option<Arc<dyn AssertionCheck>>>,
}

/// How long an idle or held pump sleeps between checks of its queue and its Spirit.
const PUMP_TICK: Duration = Duration::from_millis(250);

impl DirectedTasks {
    pub fn new(host: Arc<dyn TaskHost>, limits: TaskLimits) -> Arc<Self> {
        Arc::new(Self {
            host,
            limits,
            inner: Mutex::new(Inner::default()),
            wake: Condvar::new(),
            threads: Mutex::new(Vec::new()),
            assertion_check: Mutex::new(None),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// Install the verifier a carried assertion is checked with at accept. Until it is set,
    /// every task that carries an assertion is refused (`AssertionInvalid`): nothing can verify it.
    pub fn set_assertion_check(&self, check: Arc<dyn AssertionCheck>) {
        *self
            .assertion_check
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(check);
    }

    /// Make `spirit_id` directable: start its pump over `runner`. A second registration (a
    /// swap that kept the port) replaces the runner and keeps the one pump.
    pub fn register(self: &Arc<Self>, spirit_id: &str, runner: Arc<dyn TaskRunner>) {
        let mut inner = self.lock();
        if let Some(slot) = inner.spirits.get_mut(spirit_id) {
            slot.runner = runner;
            return;
        }
        inner.spirits.insert(
            spirit_id.to_owned(),
            SpiritSlot {
                runner,
                queue: VecDeque::new(),
                pending: 0,
                running: None,
            },
        );
        drop(inner);
        let registry = Arc::clone(self);
        let spirit = spirit_id.to_owned();
        let thread = std::thread::Builder::new()
            .name(format!("maos-task-pump-{spirit_id}"))
            .spawn(move || registry.pump(&spirit))
            .expect("task pump thread");
        self.threads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(thread);
    }

    /// Whether `spirit_id` has a pump (is an installed component under a door root).
    pub fn is_directable(&self, spirit_id: &str) -> bool {
        self.lock().spirits.contains_key(spirit_id)
    }

    pub fn counts(&self) -> TaskCounts {
        let inner = self.lock();
        TaskCounts {
            queued: inner
                .spirits
                .values()
                .map(|slot| slot.queue.len() + slot.pending)
                .sum(),
            running: inner
                .spirits
                .values()
                .filter(|slot| slot.running.is_some())
                .count(),
            finished_in_memory: inner.finished.len(),
        }
    }

    /// The Host is stopping: no task is accepted or delivered from here, and a turn the stop
    /// cuts short is left unjournaled. Call it before the Spirits are unloaded.
    pub fn quiesce(&self) {
        self.lock().shutdown = true;
        self.wake.notify_all();
    }

    /// [`Self::quiesce`], and let go of every pump's runner. Queued and running tasks stay
    /// unjournaled, so they read `interrupted` from the audit trail and are never delivered again.
    pub fn shutdown(&self) {
        let mut inner = self.lock();
        inner.shutdown = true;
        inner.spirits.clear();
        // No task will start again: drop every assertion still held in memory.
        for live in inner.tasks.values_mut() {
            live.assertion = None;
        }
        drop(inner);
        self.wake.notify_all();
    }

    /// [`Self::shutdown`], then wait for every pump thread to leave, so the Arcs they hold
    /// (and through them the audit writer's senders) are released before the root drains. A
    /// pump inside a turn leaves when that turn ends, so stop the sessions first.
    pub async fn stop(self: Arc<Self>) {
        self.shutdown();
        let _ = tokio::task::spawn_blocking(move || {
            let threads = std::mem::take(
                &mut *self
                    .threads
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()),
            );
            for thread in threads {
                let _ = thread.join();
            }
        })
        .await;
    }

    /// Accept one task: refuse past the bounds, replay a repeated key, journal the accept row,
    /// then queue it. Never waits on the Spirit.
    ///
    /// A carried `assertion` (Story 23-2) is verified first, before anything is reserved or
    /// journaled: one that fails (or finds no verifier installed) is `AssertionInvalid` and
    /// queues nothing. A replay under a known key returns the first task and never re-binds it
    /// to a different assertion; the replay's own assertion is still verified, so an invalid one
    /// is refused. Verification may wait on the identity provider's refresh, so call this off
    /// the async workers.
    pub fn accept(
        &self,
        spirit_id: &str,
        text: &str,
        key: Option<&str>,
        assertion: Option<&Assertion>,
    ) -> Result<Accepted, AcceptError> {
        if let Some(assertion) = assertion {
            let check = self
                .assertion_check
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if !check.is_some_and(|check| check.check(assertion).is_ok()) {
                return Err(AcceptError::AssertionInvalid);
            }
        }
        let key_hash = key.map(|key| ask_replay_key(spirit_id, key));
        // An earlier boot's task under this key: the audit trail, then the live registry.
        let journaled = key_hash
            .as_deref()
            .and_then(|hash| self.host.find_key(hash));
        let (task_id, record) = {
            let mut inner = self.lock();
            if let Some(hash) = &key_hash {
                let known = inner.keys.get(hash).cloned().or(journaled.clone());
                if let Some(task_id) = known {
                    let state = inner.tasks.get(&task_id).map(|live| live.record.state);
                    drop(inner);
                    let state = state
                        .or_else(|| self.host.read_back(&task_id).map(|r| r.state))
                        .unwrap_or(TaskState::Interrupted);
                    return Ok(Accepted {
                        task_id,
                        replayed: true,
                        state,
                    });
                }
            }
            let host_active: usize = inner
                .spirits
                .values()
                .map(|slot| slot.queue.len() + slot.pending + usize::from(slot.running.is_some()))
                .sum();
            if inner.shutdown {
                return Err(AcceptError::ShuttingDown);
            }
            let Some(slot) = inner.spirits.get_mut(spirit_id) else {
                return Err(AcceptError::NotDirectable);
            };
            if slot.queue.len() + slot.pending >= self.limits.per_spirit {
                return Err(AcceptError::QueueFull {
                    scope: "spirit",
                    limit: self.limits.per_spirit,
                });
            }
            if host_active >= self.limits.host {
                return Err(AcceptError::QueueFull {
                    scope: "host",
                    limit: self.limits.host,
                });
            }
            slot.pending += 1;
            let task_id = ulid::Ulid::new().to_string();
            let accepted_ns = self.host.now_ns();
            let record = TaskRecord {
                task_id: task_id.clone(),
                spirit_id: spirit_id.to_owned(),
                origin: ORIGIN_DOOR.into(),
                actor: ACTOR_DIRECTOR.into(),
                state: TaskState::Queued,
                result: None,
                halt: None,
                reason: None,
                accepted_ns,
                ended_ns: None,
            };
            if let Some(hash) = &key_hash {
                inner.keys.insert(hash.clone(), task_id.clone());
            }
            inner.tasks.insert(
                task_id.clone(),
                Live {
                    record: record.clone(),
                    text: Some(text.to_owned()),
                    assertion: assertion.cloned(),
                    key_hash: key_hash.clone(),
                },
            );
            (task_id, record)
        };
        // Log before deliver (I2): the accept row precedes anything the pump can do with it.
        if let Err(error) = self.host.write_accepted(&record) {
            let mut inner = self.lock();
            inner.tasks.remove(&task_id);
            if let Some(hash) = &key_hash {
                inner.keys.remove(hash);
            }
            if let Some(slot) = inner.spirits.get_mut(spirit_id) {
                slot.pending = slot.pending.saturating_sub(1);
            }
            return Err(AcceptError::Log(error));
        }
        let mut inner = self.lock();
        if let Some(slot) = inner.spirits.get_mut(spirit_id) {
            slot.pending = slot.pending.saturating_sub(1);
            slot.queue.push_back(task_id.clone());
        }
        drop(inner);
        self.wake.notify_all();
        Ok(Accepted {
            task_id,
            replayed: false,
            state: TaskState::Queued,
        })
    }

    /// One task: the live record, else the audit trail. `None` is an id this Host never
    /// accepted (or whose rows were rotated away, B7).
    pub fn get(&self, task_id: &str) -> Option<TaskRecord> {
        if let Some(live) = self.lock().tasks.get(task_id) {
            return Some(live.record.clone());
        }
        self.host.read_back(task_id)
    }

    /// The Spirit left (unload, uninstall): its queued tasks end `failed/spirit_unloaded`. The
    /// running one ends when its turn does.
    pub fn spirit_unloaded(&self, spirit_id: &str) {
        let drained: Vec<String> = {
            let mut inner = self.lock();
            match inner.spirits.get_mut(spirit_id) {
                Some(slot) => slot.queue.drain(..).collect(),
                None => Vec::new(),
            }
        };
        for task_id in drained {
            self.end_unstarted(spirit_id, &task_id, "spirit_unloaded");
        }
    }

    fn pump(self: Arc<Self>, spirit_id: &str) {
        loop {
            let (task_id, runner) = {
                let mut inner = self.lock();
                loop {
                    if inner.shutdown {
                        return;
                    }
                    let Some(slot) = inner.spirits.get(spirit_id) else {
                        return;
                    };
                    if let Some(front) = slot.queue.front() {
                        break (front.clone(), Arc::clone(&slot.runner));
                    }
                    inner = self
                        .wake
                        .wait_timeout(inner, PUMP_TICK)
                        .unwrap_or_else(|error| error.into_inner())
                        .0;
                }
            };
            // Only a running Spirit is handed work; a paused one holds its queue.
            match self.host.spirit_state(spirit_id) {
                SpiritRunState::Running => {}
                SpiritRunState::Unloaded => {
                    self.pop_front_if(spirit_id, &task_id);
                    self.end_unstarted(spirit_id, &task_id, "spirit_unloaded");
                    continue;
                }
                SpiritRunState::Quarantined => {
                    self.pop_front_if(spirit_id, &task_id);
                    self.end_unstarted(spirit_id, &task_id, "spirit_quarantined");
                    continue;
                }
                SpiritRunState::NotRunning(_) => {
                    let inner = self.lock();
                    let _ = self.wake.wait_timeout(inner, PUMP_TICK);
                    continue;
                }
            }
            let started_ns = self.host.now_ns();
            let (text, assertion) = {
                let mut inner = self.lock();
                let Some(slot) = inner.spirits.get_mut(spirit_id) else {
                    return;
                };
                if slot.queue.front() != Some(&task_id) {
                    // `spirit_unloaded` drained it between the peek and here.
                    continue;
                }
                slot.queue.pop_front();
                slot.running = Some(task_id.clone());
                let Some(live) = inner.tasks.get_mut(&task_id) else {
                    continue;
                };
                live.record.state = TaskState::Running;
                (live.text.take().unwrap_or_default(), live.assertion.take())
            };
            let Ok(ulid) = ulid::Ulid::from_string(&task_id) else {
                self.finish(
                    spirit_id,
                    &task_id,
                    started_ns,
                    Err(RunFailure {
                        reason: "dispatch_refused",
                        detail: "task id is not a ULID".into(),
                    }),
                );
                continue;
            };
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runner.run(ulid.to_bytes(), &text, assertion.as_ref())
            }))
            .unwrap_or_else(|_| {
                Err(RunFailure {
                    reason: "dispatch_refused",
                    detail: "the Spirit's turn runner panicked".into(),
                })
            });
            self.finish(spirit_id, &task_id, started_ns, outcome);
        }
    }

    fn pop_front_if(&self, spirit_id: &str, task_id: &str) {
        if let Some(slot) = self.lock().spirits.get_mut(spirit_id) {
            if slot.queue.front().map(String::as_str) == Some(task_id) {
                slot.queue.pop_front();
            }
        }
    }

    /// End a task that never started with a typed reason.
    fn end_unstarted(&self, spirit_id: &str, task_id: &str, reason: &'static str) {
        let Some(mut record) = self.lock().tasks.get(task_id).map(|l| l.record.clone()) else {
            return;
        };
        record.state = TaskState::Failed;
        record.reason = Some(reason.into());
        record.ended_ns = Some(self.host.now_ns());
        self.publish(spirit_id, record);
    }

    /// Classify a finished turn, journal its terminal row, then publish the final record.
    fn finish(
        &self,
        spirit_id: &str,
        task_id: &str,
        started_ns: u64,
        outcome: Result<String, RunFailure>,
    ) {
        let Some(mut record) = self.lock().tasks.get(task_id).map(|l| l.record.clone()) else {
            return;
        };
        // A halt the kernel recorded on the target while the task ran wins over any answer.
        let halt = self.host.halt_since(spirit_id, started_ns);
        match (halt, outcome) {
            (Some(halt), _) => {
                record.state = TaskState::Halted;
                record.halt = Some(halt);
            }
            (None, Ok(answer)) => {
                record.state = TaskState::Completed;
                // As journaled, so a live read and a read after a restart are the same bytes:
                // the answer and the end time are the reply row's.
                match self.host.reply_result(task_id) {
                    Some((result, ended_ns)) => {
                        record.result = Some(result);
                        record.ended_ns = Some(ended_ns);
                    }
                    None => record.result = Some(answer),
                }
            }
            (None, Err(failure)) => {
                record.state = TaskState::Failed;
                record.reason = Some(failure.reason.into());
                eprintln!(
                    "maos: task {task_id} for '{spirit_id}' gave no answer: {}: {}",
                    failure.reason, failure.detail
                );
                if failure.reason == "session_ended" {
                    if self.orphan_row_within_grace(task_id) {
                        record.state = TaskState::Orphaned;
                    } else {
                        match self.host.spirit_state(spirit_id) {
                            SpiritRunState::Unloaded => {
                                record.reason = Some("spirit_unloaded".into())
                            }
                            SpiritRunState::Quarantined => {
                                record.reason = Some("spirit_quarantined".into());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if record.ended_ns.is_none() {
            record.ended_ns = Some(self.host.now_ns());
        }
        self.publish(spirit_id, record);
    }

    fn orphan_row_within_grace(&self, task_id: &str) -> bool {
        let until = std::time::Instant::now() + self.limits.orphan_grace;
        loop {
            if self.host.orphan_row(task_id) {
                return true;
            }
            if std::time::Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Journal the terminal row (unless the Spirit answered), then make the end visible.
    fn publish(&self, spirit_id: &str, mut record: TaskRecord) {
        // A task the stop cut short is not journaled as a failure: with no terminal row it reads
        // `interrupted` from the audit trail, as after a SIGKILL.
        let stopping = self.lock().shutdown;
        if stopping && record.state != TaskState::Completed {
            record.state = TaskState::Interrupted;
            record.reason = None;
        } else if record.state != TaskState::Completed {
            if let Err(error) = self.host.write_terminal(&record) {
                eprintln!(
                    "maos: task {} ended {} but its terminal row was not written: {error}",
                    record.task_id,
                    record.state.as_str()
                );
            }
        }
        let mut inner = self.lock();
        if let Some(slot) = inner.spirits.get_mut(spirit_id) {
            if slot.running.as_deref() == Some(record.task_id.as_str()) {
                slot.running = None;
            }
        }
        let task_id = record.task_id.clone();
        if let Some(live) = inner.tasks.get_mut(&task_id) {
            live.record = record;
            live.text = None;
            live.assertion = None;
        }
        inner.finished.push_back(task_id);
        while inner.finished.len() > self.limits.finished {
            let Some(oldest) = inner.finished.pop_front() else {
                break;
            };
            if let Some(live) = inner.tasks.remove(&oldest) {
                if let Some(hash) = live.key_hash {
                    inner.keys.remove(&hash);
                }
            }
        }
        drop(inner);
        self.wake.notify_all();
    }
}

/// D-22-4-09 A — the halt the kernel recorded on `spirit_pid` at or after `since_ns`, newest
/// first. One running task per Spirit makes the correlation unambiguous.
pub fn halt_for_task(
    registry: &maos_kernel_core::halt::HaltRegistry,
    spirit_pid: u32,
    since_ns: u64,
) -> Option<TaskHalt> {
    let mut halts = registry.halt_metadata_for_spirit(spirit_pid, since_ns);
    halts.sort_by_key(|(_, meta)| std::cmp::Reverse(meta.fired_ns));
    let (halt_id, _) = halts.into_iter().next()?;
    let state = registry
        .lookup_state(&halt_id)
        .map_or_else(|| "Unknown".to_owned(), |state| format!("{state:?}"));
    Some(TaskHalt {
        halt_id: halt_id.as_str().to_owned(),
        state,
    })
}

/// The Transparency-Log side of the registry (D-22-4-08 A): every row a task leaves, and the
/// read-back that rebuilds a finished task from them.
pub struct TaskLog {
    log: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
}

impl TaskLog {
    pub fn new(log: Arc<maos_kernel_core::iac::TransparencyLogAdapter>) -> Self {
        Self { log }
    }

    fn correlation(task_id: &str) -> String {
        format!("maos.task/{task_id}")
    }

    fn write_row(&self, record: &TaskRecord, pid: u32, outcome: &str, payload: serde_json::Value) {
        let _ = self.log.insert_frame_event_with_correlation(
            maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent,
            pid,
            None,
            &Self::correlation(&record.task_id),
            &format!("task.{}:{outcome}", record.task_id),
            payload.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
    }

    /// The accept row `task.<id>:accepted`.
    pub fn write_accepted(&self, record: &TaskRecord, pid: u32) {
        self.write_row(
            record,
            pid,
            "accepted",
            serde_json::json!({
                "task_id": record.task_id,
                "spirit_id": record.spirit_id,
                "origin": record.origin,
                "actor": record.actor,
                "accepted_ns": record.accepted_ns,
            }),
        );
    }

    /// The terminal row `task.<id>:<state>` of a task that gave no answer.
    pub fn write_terminal(&self, record: &TaskRecord, pid: u32) {
        self.write_row(
            record,
            pid,
            record.state.as_str(),
            serde_json::json!({
                "task_id": record.task_id,
                "spirit_id": record.spirit_id,
                "state": record.state.as_str(),
                "reason": record.reason,
                "halt": record.halt.as_ref().map(|halt| {
                    serde_json::json!({ "halt_id": halt.halt_id, "state": halt.state })
                }),
                "ended_ns": record.ended_ns,
            }),
        );
    }

    fn rows(
        &self,
        filter: maos_kernel_core::iac::transparency_log::FrameFilter,
    ) -> Vec<maos_kernel_core::iac::transparency_log::TransparencyLogEntry> {
        self.log.query_frames(filter).unwrap_or_default()
    }

    /// The reply row of `task_id`: its journaled answer and the row's timestamp.
    fn reply_row(&self, task_id: &str) -> Option<(String, u64)> {
        use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
        let id = ulid::Ulid::from_string(task_id).ok()?;
        let row = self
            .rows(FrameFilter {
                correlation_id: Some(maos_iac::adapter::mailbox::reply_correlation(
                    maos_iac::adapter::mailbox::ReplyLabel::Task,
                    id.to_bytes(),
                )),
                kind: Some(FrameKind::TaskComplete),
                descending: true,
                limit: Some(1),
                ..Default::default()
            })
            .pop()?;
        let value: serde_json::Value = serde_json::from_slice(&row.payload_redacted).ok()?;
        let result = value["TaskComplete"]["result"]
            .as_str()
            .or_else(|| value["result"].as_str())?;
        Some((result.to_owned(), row.timestamp_ns))
    }

    /// The journaled (redacted) answer of `task_id` and its row's timestamp.
    pub fn reply_result(&self, task_id: &str) -> Option<(String, u64)> {
        self.reply_row(task_id)
    }

    /// Whether the kernel journaled `task.orphaned` for `turn:<task_id>` (the crash detector's
    /// drain of the SCB ledger).
    pub fn orphan_row(&self, task_id: &str) -> bool {
        use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
        let turn = format!("turn:{task_id}");
        self.rows(FrameFilter {
            intent: Some("task.orphaned".into()),
            kind: Some(FrameKind::TaskComplete),
            ..Default::default()
        })
        .iter()
        .any(|row| {
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                .is_ok_and(|payload| payload["task_id"] == turn)
        })
    }

    /// The task a door completion row recorded under `key_hash` ([`ask_replay_key`]).
    pub fn find_key(&self, key_hash: &str) -> Option<String> {
        use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
        let row = self
            .rows(FrameFilter {
                correlation_id: Some(key_hash.to_owned()),
                kind: Some(FrameKind::TelemetryEvent),
                descending: true,
                limit: Some(1),
                ..Default::default()
            })
            .pop()?;
        if row.origin != maos_domain::invariants::i3::FrameOrigin::Kernel
            || !row.intent.starts_with("operator.ask.")
        {
            return None;
        }
        let receipt: serde_json::Value = serde_json::from_slice(&row.payload_redacted).ok()?;
        receipt["task_id"].as_str().map(str::to_owned)
    }

    /// A task rebuilt from its rows: the terminal row, else the reply row, else the kernel's
    /// orphan row, else `interrupted` (accepted, never finished, never re-run).
    pub fn read_back(&self, task_id: &str) -> Option<TaskRecord> {
        use maos_kernel_core::iac::transparency_log::FrameFilter;
        let rows = self.rows(FrameFilter {
            correlation_id: Some(Self::correlation(task_id)),
            order_by_insertion: true,
            ..Default::default()
        });
        let accepted_intent = format!("task.{task_id}:accepted");
        let accepted = rows.iter().find(|row| row.intent == accepted_intent)?;
        let accepted: serde_json::Value =
            serde_json::from_slice(&accepted.payload_redacted).ok()?;
        let mut record = TaskRecord {
            task_id: task_id.to_owned(),
            spirit_id: accepted["spirit_id"].as_str()?.to_owned(),
            origin: accepted["origin"]
                .as_str()
                .unwrap_or(ORIGIN_DOOR)
                .to_owned(),
            actor: accepted["actor"]
                .as_str()
                .unwrap_or(ACTOR_DIRECTOR)
                .to_owned(),
            state: TaskState::Interrupted,
            result: None,
            halt: None,
            reason: None,
            accepted_ns: accepted["accepted_ns"].as_u64().unwrap_or_default(),
            ended_ns: None,
        };
        let terminal_prefix = format!("task.{task_id}:");
        let terminal = rows
            .iter()
            .find(|row| row.intent.starts_with(&terminal_prefix) && row.intent != accepted_intent);
        if let Some(row) = terminal {
            let value: serde_json::Value = serde_json::from_slice(&row.payload_redacted).ok()?;
            record.state = value["state"]
                .as_str()
                .and_then(TaskState::parse)
                .filter(|state| state.is_finished())
                .unwrap_or(TaskState::Interrupted);
            record.reason = value["reason"].as_str().map(str::to_owned);
            record.halt = value["halt"].as_object().map(|halt| TaskHalt {
                halt_id: halt["halt_id"].as_str().unwrap_or_default().to_owned(),
                state: halt["state"].as_str().unwrap_or_default().to_owned(),
            });
            record.ended_ns = value["ended_ns"].as_u64();
        } else if let Some((result, ended_ns)) = self.reply_row(task_id) {
            record.state = TaskState::Completed;
            record.result = Some(result);
            record.ended_ns = Some(ended_ns);
        } else if self.orphan_row(task_id) {
            record.state = TaskState::Orphaned;
            record.reason = Some("session_ended".into());
        }
        Some(record)
    }
}

/// The registry's host over the daemon's real kernel objects.
pub struct KernelTaskHost {
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    capability: Arc<maos_kernel_core::api::CapabilityRegistryAdapter>,
    halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    log: TaskLog,
}

impl KernelTaskHost {
    pub fn new(
        scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
        capability: Arc<maos_kernel_core::api::CapabilityRegistryAdapter>,
        halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
        log: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    ) -> Arc<Self> {
        Arc::new(Self {
            scheduler,
            capability,
            halt_registry,
            log: TaskLog::new(log),
        })
    }

    fn pid(&self, spirit_id: &str) -> u32 {
        self.scheduler.resolve_pid(spirit_id).unwrap_or(0)
    }
}

impl TaskHost for KernelTaskHost {
    fn spirit_state(&self, spirit_id: &str) -> SpiritRunState {
        use maos_kernel_core::scheduler::control_block::ScbLifecycleState as Scb;
        let Some(pid) = self.scheduler.resolve_pid(spirit_id) else {
            return SpiritRunState::Unloaded;
        };
        let state = {
            let scbs = self.scheduler.scbs();
            let guard = scbs.read().unwrap_or_else(|error| error.into_inner());
            guard.get(&pid).map(|scb| scb.current_state())
        };
        match state {
            None | Some(Scb::Unloaded) => SpiritRunState::Unloaded,
            _ if self.capability.is_quarantined(pid) => SpiritRunState::Quarantined,
            Some(Scb::Running) => SpiritRunState::Running,
            Some(Scb::Paused) => SpiritRunState::NotRunning("Paused"),
            Some(Scb::Loaded) => SpiritRunState::NotRunning("Loaded"),
        }
    }

    fn now_ns(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn write_accepted(&self, record: &TaskRecord) -> Result<(), String> {
        self.log.write_accepted(record, self.pid(&record.spirit_id));
        Ok(())
    }

    fn write_terminal(&self, record: &TaskRecord) -> Result<(), String> {
        self.log.write_terminal(record, self.pid(&record.spirit_id));
        Ok(())
    }

    fn reply_result(&self, task_id: &str) -> Option<(String, u64)> {
        self.log.reply_result(task_id)
    }

    fn orphan_row(&self, task_id: &str) -> bool {
        self.log.orphan_row(task_id)
    }

    fn halt_since(&self, spirit_id: &str, since_ns: u64) -> Option<TaskHalt> {
        let pid = self.scheduler.resolve_pid(spirit_id)?;
        halt_for_task(&self.halt_registry, pid, since_ns)
    }

    fn find_key(&self, key_hash: &str) -> Option<String> {
        self.log.find_key(key_hash)
    }

    fn read_back(&self, task_id: &str) -> Option<TaskRecord> {
        self.log.read_back(task_id)
    }
}
