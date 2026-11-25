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
//! - **Answers are capped.** More than [`MAX_ANSWERS`] distinct addresses is
//!   a failure, not a truncation: `PolicedIo` vets every answer and the bound
//!   connection tries every vetted address, so an unbounded list is unbounded
//!   work. Duplicates collapse; order is kept.
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
/// Jobs that may wait behind the one running lookup.
pub const QUEUE_DEPTH: usize = 4;
/// Deadline of one lookup, queue wait included.
pub const RESOLVE_DEADLINE: Duration = Duration::from_secs(2);

type Lookup = dyn Fn(&str, u16) -> std::io::Result<Vec<IpAddr>> + Send + Sync;
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
}

impl BoundedResolver {
    /// A resolver over the operating system's name service.
    #[must_use]
    pub fn os(deadline: Duration) -> Self {
        Self::with_lookup(
            Arc::new(|host: &str, port: u16| {
                let mut seen = Vec::new();
                // `take(MAX_ANSWERS + 1)` bounds the work done before the cap
                // check; the extra one is what makes an oversize answer visible.
                for address in (host, port).to_socket_addrs()?.take(MAX_ANSWERS * 4 + 1) {
                    if !seen.contains(&address.ip()) {
                        seen.push(address.ip());
                    }
                    if seen.len() > MAX_ANSWERS {
                        break;
                    }
                }
                Ok(seen)
            }),
            deadline,
        )
    }

    /// A resolver over an injected lookup (tests, and the OS lookup above).
    #[must_use]
    pub fn with_lookup(lookup: Arc<Lookup>, deadline: Duration) -> Self {
        let (jobs, queue) = sync_channel::<Job>(QUEUE_DEPTH);
        let started = thread::Builder::new()
            .name("maos-resolver".to_owned())
            .spawn(move || serve(&queue, lookup.as_ref()))
            .is_ok();
        Self {
            jobs: started.then_some(jobs),
            deadline,
        }
    }
}

fn serve(queue: &Receiver<Job>, lookup: &Lookup) {
    while let Ok(job) = queue.recv() {
        if Instant::now() >= job.expires {
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
        let (reply, answer) = sync_channel(1);
        jobs.try_send(Job {
            host: host.to_owned(),
            port,
            expires: Instant::now() + self.deadline,
            reply,
        })
        .map_err(|_| ResolveFailed)?;
        answer
            .recv_timeout(self.deadline)
            .map_err(|_| ResolveFailed)?
    }
}

/// The process-wide OS resolver: one worker for the whole process.
#[must_use]
pub fn system() -> BoundedResolver {
    static SYSTEM: LazyLock<BoundedResolver> =
        LazyLock::new(|| BoundedResolver::os(RESOLVE_DEADLINE));
    SYSTEM.clone()
}
