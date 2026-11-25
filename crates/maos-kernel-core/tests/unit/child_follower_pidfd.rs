//! S3 regression (RCA `s3-pidfd-unknown-rca-2026-11-26`): a follower of
//! `ContainerTerminal::facts_by` that expires at the snapshot deadline (or a
//! capped follower that gives up) must keep the verified payload pidfd fact
//! instead of freezing `payload_exit = unknown`. Compiled into the lib test
//! binary through `#[path]` (Linux only) so it can reach the crate-private
//! `arm`.
//!
//! The branches are FORCED, not raced: the owner is held by a test gate
//! (`hold_owner_before_publish`, the one `cfg(test)` hook in `facts_by`)
//! after its observation and before it publishes, for as long as the test
//! needs the follower to run. The stub `inspect` fails fast, so no test depends
//! on a stalled runtime, a sleep or a wall-clock window; the only hang guards
//! are 10 s upper bounds that fail loudly.

use super::*;
use maos_domain::sandbox::{ContainerRuntimeKind, PayloadExitSource, TerminalUnavailable};
use std::cell::RefCell;
use std::os::fd::{AsFd, OwnedFd};
use std::process::{Child, Command, Stdio};
use std::sync::PoisonError;

#[path = "../../../../tests/harness/ci.rs"]
mod ci;

const HANG_GUARD: Duration = Duration::from_secs(10);

/// Two-phase gate: the owner announces it reached the hold, then waits for release.
struct Gate {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}

impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new((false, false)),
            changed: Condvar::new(),
        })
    }

    /// Owner side. A missed release is a LOUD failure: the owner thread
    /// panics (outside the lock, so it poisons nothing; a panic in a spawned
    /// thread cannot abort other tests, and the test's `join().unwrap()`
    /// reports it).
    fn hold(&self) {
        let released = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.0 = true;
            self.changed.notify_all();
            let (state, _) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(60), |state| !state.1)
                .unwrap_or_else(PoisonError::into_inner);
            state.1
        };
        assert!(
            released,
            "hang guard (60 s): the test never released the owner"
        );
    }

    fn wait_reached(&self) {
        // Copy the flag and drop the lock BEFORE asserting: a panic under the
        // lock would poison it and abort the process from `ReleaseOnDrop`.
        let reached = {
            let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let (state, _) = self
                .changed
                .wait_timeout_while(state, HANG_GUARD, |state| !state.0)
                .unwrap_or_else(PoisonError::into_inner);
            state.0
        };
        assert!(
            reached,
            "hang guard (10 s): the owner never reached the publish hold"
        );
    }

    /// Recovers from poisoning so the owner is always woken.
    fn release(&self) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).1 = true;
        self.changed.notify_all();
    }
}

/// Releases the owner even when an assertion unwinds.
struct ReleaseOnDrop(Arc<Gate>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

thread_local! {
    static OWNER_GATE: RefCell<Option<Arc<Gate>>> = const { RefCell::new(None) };
}

/// Called by `facts_by` on the owner path; a no-op on every thread that did
/// not install a gate.
pub(super) fn hold_owner_before_publish() {
    let gate = OWNER_GATE.with(|gate| gate.borrow().clone());
    if let Some(gate) = gate {
        gate.hold();
    }
}

type OwnerFacts = std::thread::JoinHandle<Option<Arc<ContainerTerminalFacts>>>;

fn spawn_held_owner(
    terminal: &Arc<ContainerTerminal>,
    cap: Instant,
    gate: &Arc<Gate>,
) -> OwnerFacts {
    let terminal = Arc::clone(terminal);
    let gate = Arc::clone(gate);
    std::thread::spawn(move || {
        OWNER_GATE.with(|slot| *slot.borrow_mut() = Some(gate));
        terminal.facts_by(cap, false)
    })
}

/// The kernel release, for the RUN/SKIP evidence line.
fn kernel_release() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Capability probe, not a version check: `payload_pidfd` is a killed and
/// reaped payload's pidfd, and the kernel must report its wait status
/// (`PIDFD_INFO_EXIT`). Returns `false` (after one `SKIP:` line) only on an
/// undeclared-CI host; a declared-CI host without the capability panics.
fn require_pidfd_exit(test: &str, payload_pidfd: &OwnedFd) -> bool {
    let release = kernel_release();
    let status = crate::security::sandbox::linux::pidfd_exit_status(payload_pidfd.as_fd());
    if matches!(status, Ok(Some(raw)) if PayloadExit::from_wait_status(raw) == Some(PayloadExit::Signaled(9)))
    {
        eprintln!("{test}: RUN (kernel {release}: PIDFD_INFO_EXIT probe returned signaled(9))");
        return true;
    }
    if ci::skip_unless(
        &format!(
            "{test}: PIDFD_INFO_EXIT capability probe failed ({status:?}) on kernel {release}"
        ),
        false,
    ) {
        return false;
    }
    panic!("{test}: PIDFD_INFO_EXIT capability probe failed ({status:?}) on kernel {release}");
}

/// A stub runtime: `inspect` fails fast, `container exists` reports absence
/// (so teardown's removal check passes), every other verb exits 0.
fn failing_inspect_runtime(dir: &Path) -> ContainerRuntime {
    use std::os::unix::fs::PermissionsExt;
    let executable = dir.join("stub-runtime");
    std::fs::write(
        &executable,
        "#!/bin/sh\ncase \"$1\" in inspect) exit 1 ;; container) exit 1 ;; esac\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    ContainerRuntime {
        kind: ContainerRuntimeKind::Podman,
        path: executable,
        version: "stub".into(),
    }
}

/// The payload: `cat` spawned DIRECTLY by the test (no shell; stdout/stderr
/// null) with the parent holding its stdin pipe, and kept UNREAPED until the
/// end, so its PID cannot be recycled (it stays a zombie). Dropping it kills
/// it, through the pidfd when held, else `Child::kill` (safe: the child is
/// unreaped), and reaps it, on every path including a failure before the
/// pidfd exists.
///
/// Order: `_stdin` is a FIELD, so it is closed only AFTER `drop` has run the
/// kill and the reap; closing it earlier would let `cat` exit by itself
/// (EOF) before the signal.
struct Payload {
    child: Child,
    pidfd: Option<OwnedFd>,
    _stdin: Option<std::process::ChildStdin>,
}

impl Drop for Payload {
    fn drop(&mut self) {
        match &self.pidfd {
            Some(pidfd) => {
                let _ = rustix::process::pidfd_send_signal(
                    pidfd.as_fd(),
                    rustix::process::Signal::KILL,
                );
            }
            None => {
                let _ = self.child.kill();
            }
        }
        let _ = self.child.wait();
    }
}

/// A payload with its pidfd open, SIGKILLed ONLY through that pidfd (no
/// numeric-PID signalling anywhere), then reaped. `pidfd_open` is safe: the
/// child is unreaped, so the PID cannot be reused before it. The returned
/// pidfd is the one to arm; the reaped payload's exit data stays readable
/// through it (checked by [`require_pidfd_exit`]).
fn killed_external_payload() -> (Payload, u32, OwnedFd) {
    use std::os::unix::process::ExitStatusExt;
    // `cat` blocks reading the parent-held stdin pipe, so it lives until the
    // parent kills it (no timer can end it early); EOF on a harness exit also
    // ends it.
    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let stdin = child.stdin.take();
    let mut guard = Payload {
        child,
        pidfd: None,
        _stdin: stdin,
    };
    let pidfd = crate::security::sandbox::linux::pidfd_open(pid).unwrap();
    guard.pidfd = Some(pidfd.try_clone().unwrap());
    rustix::process::pidfd_send_signal(pidfd.as_fd(), rustix::process::Signal::KILL)
        .expect("SIGKILL the payload through its pidfd");
    assert!(
        crate::security::sandbox::linux::readable(pidfd.as_fd(), HANG_GUARD),
        "hang guard (10 s): the killed payload's pidfd never became readable"
    );
    let status = guard.child.wait().expect("reap the killed payload");
    assert_eq!(status.signal(), Some(9), "the payload died of SIGKILL");
    (guard, pid, pidfd)
}

/// One terminal armed with the payload pidfd over the stub runtime. Field
/// order is drop order: the child's teardown still needs the stub, so the
/// tempdir goes last.
struct Armed {
    child: SandboxedContainerChild,
    _temp: tempfile::TempDir,
}

fn armed_terminal(pid: u32, pidfd: OwnedFd) -> Armed {
    let temp = tempfile::tempdir().unwrap();
    let runtime = failing_inspect_runtime(temp.path());
    let state_dir = temp.path().join("state");
    std::fs::create_dir(&state_dir).unwrap();
    let mut child = SandboxedContainerChild::adopt(runtime, "ab".repeat(32), &state_dir);
    child.arm(pid, Some(pidfd));
    Armed { child, _temp: temp }
}

fn assert_sigkill_pidfd_facts(facts: &ContainerTerminalFacts, who: &str) {
    assert_eq!(
        (facts.payload, facts.source),
        (PayloadExit::Signaled(9), PayloadExitSource::Pidfd),
        "{who}: the verified pidfd fact survives an unavailable snapshot"
    );
}

/// The follower expiry branch of `facts_by`: the owner is held after its
/// observation, the snapshot deadline (the owner's own cap) is already past, so
/// the follower MUST take the expiry branch and freeze `DeadlineExpired`.
#[test]
fn expired_snapshot_follower_keeps_the_pidfd_fact_s3() {
    const INSTANCES: usize = 4;
    for instance in 0..INSTANCES {
        let (_shell, pid, pidfd) = killed_external_payload();
        if !require_pidfd_exit("expired_snapshot_follower_keeps_the_pidfd_fact_s3", &pidfd) {
            return;
        }
        let armed = armed_terminal(pid, pidfd);
        let terminal = Arc::clone(armed.child.terminal());
        let gate = Gate::new();
        let _release = ReleaseOnDrop(Arc::clone(&gate));
        // An owner cap of "now" makes the snapshot deadline already expired.
        let owner = spawn_held_owner(&terminal, Instant::now(), &gate);
        gate.wait_reached();
        let follower = terminal
            .facts_by(Instant::now() + Duration::from_secs(20), false)
            .expect("an uncapped follower always returns facts");
        // The INTENDED branch: only the follower expiry branch builds
        // `DeadlineExpired`; the owner's own observation can only be a
        // runtime-derived state.
        assert!(
            matches!(follower.state, Err(TerminalUnavailable::DeadlineExpired)),
            "instance {instance}: the follower took the expiry branch, got {:?}",
            follower.state
        );
        assert_sigkill_pidfd_facts(&follower, &format!("instance {instance} follower"));
        gate.release();
        let owner = owner
            .join()
            .unwrap()
            .expect("an uncapped owner always returns facts");
        assert!(
            Arc::ptr_eq(&owner, &follower),
            "instance {instance}: the owner returns the frozen follower snapshot"
        );
    }
}

/// The bridge's capped fallback: the owner is held across BOTH the `None`
/// answer and the `facts_capped` fallback, then publishes its own snapshot.
#[test]
fn capped_follower_fallback_keeps_the_pidfd_fact_s3() {
    let (_shell, pid, pidfd) = killed_external_payload();
    if !require_pidfd_exit("capped_follower_fallback_keeps_the_pidfd_fact_s3", &pidfd) {
        return;
    }
    let armed = armed_terminal(pid, pidfd);
    let terminal = Arc::clone(armed.child.terminal());
    let gate = Gate::new();
    let _release = ReleaseOnDrop(Arc::clone(&gate));
    let owner = spawn_held_owner(&terminal, Instant::now() + Duration::from_secs(3600), &gate);
    gate.wait_reached();
    // Controlled shared deadline: `facts_by` clamped the owner's cap to the
    // 10 s SNAPSHOT_BUDGET when the owner started. Widen it while the owner is
    // held (the owner does not re-read it), so neither follower call can cross
    // the deadline however long this thread is descheduled. The rendezvous
    // guard above is a separate, loud-failure bound.
    terminal.lock().snapshot_deadline = Some(Instant::now() + Duration::from_secs(86_400));
    assert!(
        terminal.facts_by(Instant::now(), true).is_none(),
        "a capped follower returns None while the owner is in flight"
    );
    let local = terminal.facts_capped(Instant::now());
    assert!(
        matches!(local.state, Err(TerminalUnavailable::DeadlineExpired)),
        "the local fallback record is DeadlineExpired, got {:?}",
        local.state
    );
    assert_sigkill_pidfd_facts(&local, "capped-follower fallback");
    gate.release();
    let owner = owner.join().unwrap().expect("owner facts");
    assert!(
        matches!(
            owner.state,
            Err(TerminalUnavailable::InspectFailed | TerminalUnavailable::InspectTimedOut)
        ),
        "the owner published its own observation, got {:?}",
        owner.state
    );
    assert_sigkill_pidfd_facts(&owner, "owner");
    assert!(
        !Arc::ptr_eq(&local, &owner) && Arc::ptr_eq(&terminal.facts(), &owner),
        "the capped fallback is local and never frozen"
    );
}
