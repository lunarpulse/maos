//! S3 regression (RCA `s3-pidfd-unknown-rca-2026-11-26`): a follower of
//! `ContainerTerminal::facts_by` that expires at the snapshot deadline (or a
//! capped follower that gives up) must keep the verified payload pidfd fact
//! instead of freezing `payload_exit = unknown`. Compiled into the lib test
//! binary through `#[path]` so it can reach the crate-private `arm`.
//!
//! No sleeps are an oracle and no assertion reads a clock: the stub runtime's
//! `inspect` hangs, the gate is the stub's own call log, and the assertion is
//! on the frozen facts only. With the fix both orderings (follower or owner
//! publishes first) yield `Signaled(9)`; without it the follower publishes
//! `Unknown` whenever it wins, which the owner's `DRAIN_GRACE` after the
//! deadline makes the usual outcome.

use super::*;
use maos_domain::sandbox::{ContainerRuntimeKind, PayloadExitSource};
use std::io::{BufRead, BufReader};
use std::os::fd::AsFd;
use std::process::{Command, Stdio};

#[path = "../../../../tests/harness/ci.rs"]
mod ci;

/// Kernel releases below 6.15 have no `PIDFD_INFO_EXIT`. Returns `false`
/// (after printing one `SKIP:` line) only on an undeclared-CI host; a
/// declared-CI host with an old kernel panics rather than pass vacuously.
fn require_pidfd_exit(test: &str) -> bool {
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_default()
        .trim()
        .to_owned();
    let mut parts = release
        .split('.')
        .filter_map(|part| part.split('-').next()?.parse::<u32>().ok());
    let supported = matches!(
        (parts.next(), parts.next()),
        (Some(major), Some(minor)) if major > 6 || (major == 6 && minor >= 15)
    );
    if supported {
        eprintln!("{test}: RUN (kernel {release} supports PIDFD_INFO_EXIT)");
        return true;
    }
    if ci::skip_unless(
        &format!("{test}: Linux >= 6.15 PIDFD_INFO_EXIT (kernel {release} predates it)"),
        false,
    ) {
        return false;
    }
    panic!("{test}: PIDFD_INFO_EXIT requires Linux >= 6.15; kernel is {release}");
}

/// A stub runtime whose `inspect` logs and then hangs; `container exists`
/// reports absence (so teardown's removal check passes); the rest exit 0.
fn hanging_inspect_runtime(dir: &Path) -> (ContainerRuntime, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let executable = dir.join("stub-runtime");
    let log = dir.join("runtime.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'\ncase \"$1\" in inspect) exec sleep 30 ;; container) exit 1 ;; esac\n",
        log.display()
    );
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    (
        ContainerRuntime {
            kind: ContainerRuntimeKind::Podman,
            path: executable,
            version: "stub".into(),
        },
        log,
    )
}

/// A payload that is not our child (an owner shell reaps it), SIGKILLed after
/// its pidfd is open. Returns the owner and the armed-ready pidfd; the exit
/// data is final once the owner acknowledges the reap.
fn killed_external_payload() -> (std::process::Child, u32, std::os::fd::OwnedFd) {
    let mut owner = Command::new("sh")
        .arg("-c")
        .arg("sleep 30 & p=$!; echo $p; wait $p; echo reaped")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(owner.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let pid: u32 = line.trim().parse().unwrap();
    let pidfd = crate::security::sandbox::linux::pidfd_open(pid).unwrap();
    assert!(
        Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .unwrap()
            .success(),
        "SIGKILL the external payload"
    );
    let mut ack = String::new();
    stdout.read_line(&mut ack).unwrap();
    assert_eq!(ack.trim(), "reaped", "the owner shell reaps, then says so");
    assert!(
        crate::security::sandbox::linux::readable(pidfd.as_fd(), Duration::from_secs(10)),
        "hang guard (10 s): the killed payload's pidfd never became readable"
    );
    (owner, pid, pidfd)
}

/// One armed terminal over a hanging `inspect`, plus the wait for the stub to
/// have entered `inspect` (the owner is then inside `observe`). Field order is
/// drop order: the child's teardown still needs the stub, so the tempdir last.
struct Armed {
    child: SandboxedContainerChild,
    log: PathBuf,
    _temp: tempfile::TempDir,
}

fn armed_terminal(pid: u32, pidfd: std::os::fd::OwnedFd) -> Armed {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, log) = hanging_inspect_runtime(temp.path());
    let state_dir = temp.path().join("state");
    std::fs::create_dir(&state_dir).unwrap();
    let mut child = SandboxedContainerChild::adopt(runtime, "ab".repeat(32), &state_dir);
    child.arm(pid, Some(pidfd));
    Armed {
        _temp: temp,
        child,
        log,
    }
}

fn await_inspect_entered(log: &Path) {
    let guard = Instant::now() + Duration::from_secs(10);
    while !std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .any(|call| call == "inspect")
    {
        assert!(
            Instant::now() < guard,
            "hang guard (10 s): the owner never entered inspect"
        );
        std::thread::yield_now();
    }
}

fn assert_sigkill_pidfd_facts(facts: &ContainerTerminalFacts, who: &str) {
    assert!(
        facts.state.is_err(),
        "{who}: the hung inspect leaves typed unavailable runtime state, got {:?}",
        facts.state
    );
    assert_eq!(
        (facts.payload, facts.source),
        (PayloadExit::Signaled(9), PayloadExitSource::Pidfd),
        "{who}: the verified pidfd fact survives an unavailable snapshot"
    );
}

#[test]
fn expired_snapshot_follower_keeps_the_pidfd_fact_s3() {
    const INSTANCES: usize = 8;
    if !require_pidfd_exit("expired_snapshot_follower_keeps_the_pidfd_fact_s3") {
        return;
    }
    for instance in 0..INSTANCES {
        let (mut owner_shell, pid, pidfd) = killed_external_payload();
        let armed = armed_terminal(pid, pidfd);
        let terminal = Arc::clone(armed.child.terminal());
        // Owner: a short cap, so the snapshot deadline is the owner's cap.
        let owner = {
            let terminal = Arc::clone(&terminal);
            std::thread::spawn(move || {
                terminal.facts_by(Instant::now() + Duration::from_millis(300), false)
            })
        };
        // Gate: the owner is inside `observe`, so the snapshot deadline is set
        // and the next caller is a follower.
        await_inspect_entered(&armed.log);
        // Follower: a longer cap, uncapped by its own cap, waits to the
        // owner's deadline and expires there.
        let follower = terminal
            .facts_by(Instant::now() + Duration::from_secs(20), false)
            .expect("an uncapped follower always returns facts");
        let owner = owner
            .join()
            .unwrap()
            .expect("an uncapped owner always returns facts");
        assert!(
            Arc::ptr_eq(&owner, &follower),
            "instance {instance}: one immutable snapshot"
        );
        assert_sigkill_pidfd_facts(&follower, &format!("instance {instance} follower"));
        assert_sigkill_pidfd_facts(&owner, &format!("instance {instance} owner"));
        let _ = owner_shell.wait();
    }
}

#[test]
fn capped_follower_fallback_keeps_the_pidfd_fact_s3() {
    if !require_pidfd_exit("capped_follower_fallback_keeps_the_pidfd_fact_s3") {
        return;
    }
    let (mut owner_shell, pid, pidfd) = killed_external_payload();
    let armed = armed_terminal(pid, pidfd);
    let terminal = Arc::clone(armed.child.terminal());
    // The owner cap is generous so the shared snapshot is still in flight when
    // the capped follower asks, even on a loaded host; the test then waits for
    // the owner's deadline once.
    let owner = {
        let terminal = Arc::clone(&terminal);
        std::thread::spawn(move || {
            terminal.facts_by(Instant::now() + Duration::from_secs(3), false)
        })
    };
    await_inspect_entered(&armed.log);
    // A capped follower whose own cap is already past, with the shared
    // snapshot still in flight: `None`, the bridge's fallback path.
    assert!(
        terminal.facts_by(Instant::now(), true).is_none(),
        "a capped follower returns None before ownership publishes"
    );
    // The bridge's helper (the one `wait_and_finalize` calls) returns the same
    // local unavailable record, carrying the pidfd fact.
    assert_sigkill_pidfd_facts(
        &terminal.facts_capped(Instant::now()),
        "capped-follower fallback",
    );
    let owner = owner.join().unwrap().expect("owner facts");
    assert_sigkill_pidfd_facts(&owner, "owner");
    let _ = owner_shell.wait();
}
