#![forbid(unsafe_code)]

//! Bounded, std-only native workload for the delegated cgroup ceiling smoke.

use std::fs::File;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--fd-marker") => {
            let marker = std::env::args().nth(2).expect("FD probe marker path");
            let limits = std::fs::read_to_string("/proc/self/limits").expect("actual guest limits");
            std::fs::write(marker, limits).expect("record guest execution and actual limits");
            return;
        }
        Some("--descendant-leaf") => {
            hold_for(Duration::from_secs(25));
            return;
        }
        Some("--descendant-root") => {
            descendant_root();
            return;
        }
        _ => {}
    }
    let membership = std::fs::read_to_string("/proc/self/cgroup").expect("read own membership");
    let unified = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("own cgroup-v2 membership");
    writeln!(io::stdout().lock(), "MEMBERSHIP 0::{unified}").expect("write own membership");
    let mut threads = Vec::new();
    let mut saw_task_exhaustion = false;
    for _ in 0..64 {
        match thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| hold_for(Duration::from_secs(8)))
        {
            Ok(thread) => threads.push(thread),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                saw_task_exhaustion = true;
                break;
            }
            Err(error) => panic!("unexpected native task creation error: {error}"),
        }
    }
    assert!(saw_task_exhaustion, "pids.max did not refuse a bounded thread attempt");

    let mut descriptors = Vec::new();
    let mut saw_fd_exhaustion = false;
    for _ in 0..128 {
        match File::open("/dev/null") {
            Ok(file) => descriptors.push(file),
            Err(error) if error.raw_os_error() == Some(24) => {
                saw_fd_exhaustion = true;
                break;
            }
            Err(error) => panic!("unexpected file-descriptor error: {error}"),
        }
    }
    assert!(saw_fd_exhaustion, "RLIMIT_NOFILE did not refuse a bounded open attempt");

    let mut oversized = Vec::<u8>::new();
    assert!(
        oversized.try_reserve_exact(96 * 1024 * 1024).is_err(),
        "the bounded allocation must hit the 64-MiB native memory ceiling"
    );

    let until = Instant::now() + Duration::from_secs(2);
    let mut work = 0u64;
    while Instant::now() < until {
        work = work.wrapping_add(1);
        std::hint::black_box(work);
    }
    writeln!(io::stdout().lock(), "READY tasks={} work={work}", threads.len())
        .expect("write fixture readiness");
    for thread in threads {
        thread.join().expect("bounded worker thread");
    }
    hold_for(Duration::from_secs(1));
    drop(descriptors);
}

fn descendant_root() {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--descendant-leaf")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn bounded descendant with its own pipes");
    writeln!(io::stdout().lock(), "DESCENDANT {}", child.id()).unwrap();
    child.wait().expect("reap bounded descendant on natural exit");
}

fn hold_for(duration: Duration) {
    let deadline = Instant::now() + duration;
    // Timed parking may wake spuriously; do not release a task before its bound.
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        thread::park_timeout(remaining);
    }
}

