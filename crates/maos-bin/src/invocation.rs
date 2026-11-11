//! One pump dispatch, as its session link sees it (Story 20-9, D-20-9-24).
//!
//! The kernel's dispatcher answers a hook with an outcome only, so the pump cannot learn from
//! it whether a spawned Spirit's session ever started the export or dropped it for contention
//! (another export held the session), and it must not charge the hook for either. The pump
//! registers each dispatch under what its link will see of the request (pid, hook kind, the
//! event's `seq` or a digest of the batch); the link attaches the call it queues, and the
//! session reports when it starts it. The registering dispatch removes the record when it ends,
//! so nothing outlives a dispatch and nothing is shared between two of them.
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

pub const TELEMETRY: u8 = 0;
pub const CONSOLIDATE: u8 = 1;

const UNSEEN: u8 = 0;
const QUEUED: u8 = 1;
const STARTED: u8 = 2;
const DROPPED: u8 = 3;

#[derive(Default)]
pub struct Invocation {
    phase: AtomicU8,
    started: OnceLock<Instant>,
}

type Key = (u32, u8, u64);

static LIVE: Mutex<Vec<(Key, Arc<Invocation>)>> = Mutex::new(Vec::new());

fn live() -> MutexGuard<'static, Vec<(Key, Arc<Invocation>)>> {
    LIVE.lock().unwrap_or_else(|error| error.into_inner())
}

impl Invocation {
    /// Link: the request is queued for its session.
    pub fn queued(&self) {
        self.phase.store(QUEUED, Ordering::Release);
    }

    /// Session: the export now runs.
    pub fn started(&self) {
        let _ = self.started.set(Instant::now());
        self.phase.store(STARTED, Ordering::Release);
    }

    /// Link: the request gave up in the queue and never ran.
    pub fn dropped(&self) {
        self.phase.store(DROPPED, Ordering::Release);
    }

    pub fn was_dropped(&self) -> bool {
        self.phase.load(Ordering::Acquire) == DROPPED
    }

    /// When the hook's own wall ends: `wall` after its export started, `wall` after `began`
    /// for a hook no link serves, and none while the request still waits in a queue (the link
    /// bounds that wait).
    pub fn deadline(&self, began: Instant, wall: Duration) -> Option<Instant> {
        match self.phase.load(Ordering::Acquire) {
            QUEUED => None,
            STARTED => self.started.get().map(|at| *at + wall),
            _ => Some(began + wall),
        }
    }
}

/// A registered dispatch, removed when it is dropped.
pub struct Registered {
    record: Arc<Invocation>,
}

impl Registered {
    /// Register a dispatch. Two live dispatches never share a key: content cannot tell the
    /// calls of identical batches apart, so a dispatch that finds its key in use (a forget's
    /// notice byte for byte the same as one still running) waits for that one to end, which
    /// its wall bounds, and runs after it.
    pub async fn open(pid: u32, kind: u8, tag: u64) -> Self {
        let record = Arc::new(Invocation {
            phase: AtomicU8::new(UNSEEN),
            started: OnceLock::new(),
        });
        loop {
            {
                let mut live = live();
                if !live.iter().any(|(key, _)| *key == (pid, kind, tag)) {
                    live.push(((pid, kind, tag), Arc::clone(&record)));
                    return Self { record };
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub fn invocation(&self) -> &Invocation {
        &self.record
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        live().retain(|(_, record)| !Arc::ptr_eq(record, &self.record));
    }
}

/// Link: the dispatch a request belongs to, if the pump registered one.
pub fn claim(pid: u32, kind: u8, tag: u64) -> Option<Arc<Invocation>> {
    live()
        .iter()
        .find(|(key, _)| *key == (pid, kind, tag))
        .map(|(_, record)| Arc::clone(record))
}

/// What identifies a consolidation batch to both sides.
pub fn digest(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
