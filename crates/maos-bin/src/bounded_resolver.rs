//! Story 23-2 — the production [`ResolveHost`] (D-23-0b-06 a).
//!
//! One mechanism for every caller that resolves a name the policy must vet
//! (the KMS endpoint check at boot, OIDC discovery, and the adapters that
//! follow): a single worker thread per resolver, a bounded job queue, a
//! per-call deadline and a cap on the number of answers.
//!
//! - **No thread per call.** `getaddrinfo` cannot be interrupted. A caller
//!   that times out leaves the worker busy, and later calls are not given a
//!   new thread: they wait in a queue of [`QUEUE_DEPTH`] jobs or fail at once
//!   when it is full. A job whose deadline passed while it waited is skipped,
//!   never run late.
//! - **One deadline.** The deadline is one absolute instant fixed when the
//!   call starts. Queueing, the lookup and the receipt of its answer all
//!   happen before it; an answer that arrives after it (a lookup that was
//!   already running when it passed) is discarded, never returned.
//! - **Answers are capped.** More than [`MAX_ANSWERS`] distinct addresses is
//!   a failure, not a truncation: `PolicedIo` vets every answer and the bound
//!   connection tries every vetted address, so an unbounded list is unbounded
//!   work. Duplicates collapse; order is kept. The OS adapter reads at most
//!   [`MAX_RAW_ANSWERS`] raw entries; an answer that is longer than that is a
//!   failure too, because the entries past the budget were never looked at
//!   (see [`distinct_answers`]).
//! - **Fail closed.** A lookup error, an empty answer, a full queue, a dead
//!   worker and a missed deadline are all [`ResolveFailed`].
//!
//! Limit: while the OS resolver hangs, every call through the same resolver
//! fails (the process-wide [`system`] resolver is shared on purpose, so a hung
//! resolver is one stuck thread, not one per request).

use std::net::{IpAddr, ToSocketAddrs};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::{Duration, Instant};

use maos_domain::outbound::{ResolveFailed, ResolveHost};

/// Most distinct addresses one lookup may return.
pub const MAX_ANSWERS: usize = 16;
/// Most raw answer entries the OS adapter reads before it gives up (duplicates included).
pub const MAX_RAW_ANSWERS: usize = MAX_ANSWERS * 4;
/// Jobs that may wait behind the one running lookup.
pub const QUEUE_DEPTH: usize = 4;
/// Deadline of one lookup, queue wait included.
pub const RESOLVE_DEADLINE: Duration = Duration::from_secs(2);

type Lookup = dyn Fn(&str, u16) -> std::io::Result<Vec<IpAddr>> + Send + Sync;
type Now = dyn Fn() -> Instant + Send + Sync;
type Answer = Result<Vec<IpAddr>, ResolveFailed>;

struct Job {
    host: String,
    port: u16,
    expires: Instant,
    reply: SyncSender<Answer>,
}

/// A bounded, deadline-driven resolver. Cloning shares the worker.
#[derive(Clone)]
pub struct BoundedResolver {
    /// `None` when the worker thread could not start: every call fails.
    jobs: Option<SyncSender<Job>>,
    deadline: Duration,
    now: Arc<Now>,
}

/// The distinct addresses of a raw answer, within the scan budget.
///
/// Reads at most [`MAX_RAW_ANSWERS`] entries. Reaching the end of the answer inside the budget
/// returns the distinct addresses (order kept). More than [`MAX_ANSWERS`] distinct ones stop the
/// scan early and are returned for the caller's cap to refuse. Entries remaining after the
/// budget was spent are an error: truncating there would accept an answer whose unread tail
/// could hold more addresses than the cap allows.
///
/// # Errors
/// `InvalidData` when the budget ran out before the end of the answer.
pub fn distinct_answers(mut raw: impl Iterator<Item = IpAddr>) -> std::io::Result<Vec<IpAddr>> {
    let mut seen = Vec::new();
    for _ in 0..MAX_RAW_ANSWERS {
        let Some(address) = raw.next() else {
            return Ok(seen);
        };
        if !seen.contains(&address) {
            seen.push(address);
        }
        if seen.len() > MAX_ANSWERS {
            return Ok(seen);
        }
    }
    if raw.next().is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "resolver answer exceeds the scan budget",
        ));
    }
    Ok(seen)
}

impl BoundedResolver {
    /// A resolver over the operating system's name service.
    #[must_use]
    pub fn os(deadline: Duration) -> Self {
        Self::with_lookup(
            Arc::new(|host: &str, port: u16| {
                distinct_answers((host, port).to_socket_addrs()?.map(|address| address.ip()))
            }),
            deadline,
        )
    }

    /// A resolver over an injected lookup (tests, and the OS lookup above).
    #[must_use]
    pub fn with_lookup(lookup: Arc<Lookup>, deadline: Duration) -> Self {
        Self::with_lookup_and_clock(lookup, deadline, Arc::new(Instant::now))
    }

    /// As [`Self::with_lookup`], reading time from `now` (tests move it without sleeping).
    #[must_use]
    pub fn with_lookup_and_clock(lookup: Arc<Lookup>, deadline: Duration, now: Arc<Now>) -> Self {
        let (jobs, queue) = sync_channel::<Job>(QUEUE_DEPTH);
        let worker_now = Arc::clone(&now);
        let started = thread::Builder::new()
            .name("maos-resolver".to_owned())
            .spawn(move || serve(&queue, lookup.as_ref(), worker_now.as_ref()))
            .is_ok();
        Self {
            jobs: started.then_some(jobs),
            deadline,
            now,
        }
    }
}

fn serve(queue: &Receiver<Job>, lookup: &Lookup, now: &Now) {
    while let Ok(job) = queue.recv() {
        if now() >= job.expires {
            continue;
        }
        let answer = lookup(&job.host, job.port)
            .map_err(|_| ResolveFailed)
            .and_then(|addresses| {
                let mut distinct: Vec<IpAddr> = Vec::new();
                for address in addresses {
                    if !distinct.contains(&address) {
                        distinct.push(address);
                    }
                }
                if distinct.is_empty() || distinct.len() > MAX_ANSWERS {
                    Err(ResolveFailed)
                } else {
                    Ok(distinct)
                }
            });
        let _ = job.reply.send(answer);
    }
}

impl ResolveHost for BoundedResolver {
    fn resolve(&self, host: &str, port: u16) -> Answer {
        let jobs = self.jobs.as_ref().ok_or(ResolveFailed)?;
        // One absolute deadline for queueing, the lookup and the receipt of its answer.
        let expires = (self.now)() + self.deadline;
        let (reply, answer) = sync_channel(1);
        jobs.try_send(Job {
            host: host.to_owned(),
            port,
            expires,
            reply,
        })
        .map_err(|_| ResolveFailed)?;
        let outcome = answer
            .recv_timeout(expires.saturating_duration_since((self.now)()))
            .map_err(|_| ResolveFailed)?;
        // A lookup that was already running when the deadline passed answers late: discard it.
        if (self.now)() >= expires {
            return Err(ResolveFailed);
        }
        outcome
    }
}

/// The process-wide OS resolver: one worker for the whole process.
#[must_use]
pub fn system() -> BoundedResolver {
    static SYSTEM: LazyLock<BoundedResolver> =
        LazyLock::new(|| BoundedResolver::os(RESOLVE_DEADLINE));
    SYSTEM.clone()
}
