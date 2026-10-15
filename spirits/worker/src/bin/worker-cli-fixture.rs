#![forbid(unsafe_code)]

//! `worker-cli-fixture` — the real in-crate fixture-CLI the Worker
//! CliWrapperSpirit wraps (Story 8.4, Decision B).
//!
//! Behavior:
//! - With `--maos-bridge-probe` anywhere in argv: print the JSON output-shape
//!   envelope as the FIRST stdout line and exit 0. This is what the kernel's
//!   `probe_and_verify_shape` reads to assert the declared `output_shape_version`.
//! - `--maos-fixture-mode=resource-probe`: bounded real thread and descriptor
//!   exhaustion plus a bounded CPU burst, held briefly for OCI inspection.
//! - With `--maos-fixture-mode=<mode>`: run the named Story-16-3 crash/hang
//!   scene (ruling D-16-3-L) so the supervision corpora can drive real
//!   subprocess deaths; unknown mode ⇒ exit 2. Modes: `hang`, `line-then-hang`,
//!   `hang-chatty`, `hang-with-grandchild`, `sigkill-self`, `exit-nonzero`.
//! - Otherwise: echo the deterministic canned output (one line each) and exit 0.
//!   This is the fixture-replayed "work product" captured to the Transparency
//!   Log as `FrameKind::CliSubprocessOutput=21` rows.
//!
//! The reported shape version defaults to [`worker::OUTPUT_SHAPE_VERSION`] but
//! can be overridden via `MAOS_FIXTURE_SHAPE_VERSION` so tests can drive a
//! shape MISMATCH against a stable binary (no write-then-exec race).
//!
//! Hermetic by construction: no network, no filesystem writes, deterministic
//! stdout. Self-terminating T3 scenes exit immediately; native execution is
//! unchanged.

use std::io::Write as _;
use std::time::Duration;

/// Story 16-3 (ruling D-16-3-L): the fixture-mode flag, carried in the
/// manifest's `argv_prefix` (hashed into the cap token). Parsed AFTER the
/// [`worker::PROBE_FLAG`] check so admission's 2s probe still answers and
/// exits — a mode flag must NOT change the probe path or its envelope bytes.
const MODE_FLAG_PREFIX: &str = "--maos-fixture-mode=";

/// The hang modes sleep in this loop step; the harness SIGKILLs the process.
const HANG_POLL: Duration = Duration::from_secs(3600);

/// Exit code for an unrecognized `--maos-fixture-mode=<v>` value (D-16-3-L).
const UNKNOWN_MODE_EXIT: i32 = 2;

/// Exit code when the `kill` binary cannot be run at all in `sigkill-self` —
/// distinct from every other exit so AC1's `signaled(sig=9)` expectation reds
/// loudly instead of passing on a non-signal exit.
const KILL_SPAWN_FAILURE_EXIT: i32 = 97;

/// Exit code when the `hang-with-grandchild` grandchild cannot be spawned at
/// all — failing loud beats silently sleeping without a grandchild.
const GRANDCHILD_SPAWN_FAILURE_EXIT: i32 = 98;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let version = std::env::var("MAOS_FIXTURE_SHAPE_VERSION")
        .unwrap_or_else(|_| worker::OUTPUT_SHAPE_VERSION.to_string());

    if args.iter().any(|a| a == worker::PROBE_FLAG) {
        // First stdout line MUST be the probe envelope.
        println!("{}", worker::probe_envelope(&version));
        return;
    }

    // Story 16-3 crash/hang scenes — parsed AFTER the probe check (D-16-3-L),
    // so `--maos-bridge-probe` combined with a mode flag still emits the
    // envelope and exits instead of hanging the admission probe.
    if let Some(mode) = args.iter().find_map(|a| a.strip_prefix(MODE_FLAG_PREFIX)) {
        match mode {
            "resource-probe" => resource_probe(),
            "hang" => sleep_forever(),
            "line-then-hang" => {
                println!("{}", worker::CANNED_OUTPUT_LINES[0]);
                flush_stdout();
                sleep_forever();
            }
            "hang-chatty" => loop {
                println!("worker: fixture heartbeat");
                flush_stdout();
                std::thread::sleep(Duration::from_secs(1));
            },
            "hang-with-grandchild" => hang_with_grandchild(),
            "sigkill-self" => sigkill_self(),
            // The sanctioned non-crash failure: a clean `fault.non-zero-exit`
            // with `exit_code: 3`.
            "exit-nonzero" => std::process::exit(3),
            "exit-137" => {
                flush_stdout();
                let _ = std::io::stderr().flush();
                std::process::exit(137)
            }
            "oom" => oom(),
            unknown => {
                eprintln!(
                    "worker-fixture: unknown fixture mode `{unknown}` (expected resource-probe, \
                     hang, line-then-hang, hang-chatty, hang-with-grandchild, sigkill-self, \
                     exit-nonzero, exit-137 or oom)"
                );
                std::process::exit(UNKNOWN_MODE_EXIT);
            }
        }
    }

    // Non-probe: the FIRST line acknowledges the routed task. When the bridge
    // routes a task (a trailing non-flag argv after the `--maos-worker` prefix),
    // echo it — this is what makes task routing PROVABLE hermetically (the task
    // text lands in the Transparency Log). With no task arg the line falls back
    // to the canned acknowledgement, so the SHA-pinned 3-line shape and the
    // `CANNED_OUTPUT_LINES` constant are preserved unchanged. The terminal line
    // stays `worker: task complete` (the completion oracle's marker).
    let routed_task = args.iter().skip(1).find(|a| !a.starts_with("--"));
    match routed_task {
        Some(task) => println!("worker: received task assignment: {task}"),
        None => println!("{}", worker::CANNED_OUTPUT_LINES[0]),
    }
    println!("{}", worker::CANNED_OUTPUT_LINES[1]);
    println!("{}", worker::CANNED_OUTPUT_LINES[2]);
    flush_stdout();
}

fn flush_stdout() {
    let _ = std::io::stdout().flush();
}

/// OOM positive control: emit cgroup limits first, then retain touched pages
/// until the runtime kills the payload or the fixed 1 GiB cap is reached.
fn oom() -> ! {
    use std::io::Write;
    let memory_max =
        std::fs::read_to_string("/sys/fs/cgroup/memory.max").unwrap_or_else(|_| "unknown".into());
    let swap_max = std::fs::read_to_string("/sys/fs/cgroup/memory.swap.max")
        .unwrap_or_else(|_| "unknown".into());
    eprintln!("{}", memory_max.trim());
    eprintln!("{}", swap_max.trim());
    let _ = std::io::stderr().flush();
    let mut pages = Vec::<Box<[u8; 1024 * 1024]>>::new();
    for total in (1..=1024).map(|n| n * 1024 * 1024usize) {
        let mut page = Box::new([0u8; 1024 * 1024]);
        for byte in page.iter_mut().step_by(4096) {
            *byte = 1;
        }
        pages.push(page);
        eprintln!("{total}");
        let _ = std::io::stderr().flush();
    }
    std::hint::black_box(pages);
    std::process::exit(0)
}

/// Sleeps effectively forever, in a loop — the harness SIGKILLs the process.
fn sleep_forever() -> ! {
    loop {
        std::thread::sleep(HANG_POLL);
    }
}

/// `hang-with-grandchild`: spawn a child copy of THIS binary in `hang` mode
/// with inherited stdout — the grandchild holds the parent's stdout pipe,
/// which is the point of the mode (`exec` would leave no grandchild to kill).
/// The grandchild pid is the parent's FIRST stdout line.
fn hang_with_grandchild() -> ! {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("worker-fixture: could not resolve current_exe: {e}");
            std::process::exit(GRANDCHILD_SPAWN_FAILURE_EXIT);
        }
    };
    let grandchild = match std::process::Command::new(exe)
        .arg(format!("{MODE_FLAG_PREFIX}hang"))
        .stdout(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(grandchild) => grandchild,
        Err(e) => {
            eprintln!("worker-fixture: could not spawn grandchild: {e}");
            std::process::exit(GRANDCHILD_SPAWN_FAILURE_EXIT);
        }
    };
    println!("worker-fixture: grandchild pid {}", grandchild.id());
    flush_stdout();
    // No wait(): the parent sleeps forever beside the grandchild and is
    // SIGKILLed by the harness while the grandchild keeps holding the
    // inherited stdout pipe.
    sleep_forever()
}

/// `sigkill-self`: print one line, flush, then SIGKILL this process through
/// the `kill` binary (`unsafe_code = "forbid"` rules out a raw `kill(2)` and
/// `std::process::abort()` dies by signal 6, not 9). Native only: a T3 payload
/// is its PID namespace's init, which cannot receive its own SIGKILL.
fn sigkill_self() -> ! {
    println!("{}", worker::CANNED_OUTPUT_LINES[0]);
    flush_stdout();
    match std::process::Command::new("kill")
        .args(["-KILL", &std::process::id().to_string()])
        .status()
    {
        // The SIGKILL is ours; if it landed while `kill` was still being
        // reaped, the sleep below keeps us alive to be collected either way.
        Ok(_) => sleep_forever(),
        Err(e) => {
            eprintln!("worker-fixture: could not run `kill -KILL <pid>`: {e}");
            std::process::exit(KILL_SPAWN_FAILURE_EXIT);
        }
    }
}
/// Exercise finite, actual process and descriptor limits in the T3 container.
///
/// The task count is deliberately bounded: at most 64 worker threads are
/// attempted, and all successfully-created threads are released and joined
/// before return. The 12-PID test cap therefore makes this hit EAGAIN without
/// an unbounded fork or thread bomb.
fn resource_probe() -> ! {
    use std::fs::File;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    for namespace in ["pid", "net", "user"] {
        let path = format!("/proc/self/ns/{namespace}");
        let actual = std::fs::read_link(path).expect("resource fixture namespace");
        println!("worker: namespace {namespace}={}", actual.display());
    }
    flush_stdout();
    let mut files = Vec::new();
    let fd_error = loop {
        match File::open("/dev/null") {
            Ok(file) if files.len() < 256 => files.push(file),
            Ok(_) => {
                break "fixture-bound".to_string();
            }
            Err(error) => {
                break format!("{:?}:{}", error.kind(), error.raw_os_error().unwrap_or(-1))
            }
        }
    };
    let release = Arc::new(AtomicBool::new(false));
    let mut threads = Vec::new();
    let mut task_error = None;
    for _ in 0..64 {
        let release = Arc::clone(&release);
        match std::thread::Builder::new().spawn(move || {
            while !release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(10));
            }
        }) {
            Ok(thread) => threads.push(thread),
            Err(error) => {
                task_error = Some(format!(
                    "{:?}:{}",
                    error.kind(),
                    error.raw_os_error().unwrap_or(-1)
                ));
                break;
            }
        }
    }

    println!("{}", worker::CANNED_OUTPUT_LINES[0]);
    println!(
        "worker: resource probe {{\"threads_created\":{},\"task_error\":{},\"fds_opened\":{},\"fd_error\":\"{}\"}}",
        threads.len(),
        task_error
            .as_deref()
            .map(|error| format!("\"{error}\""))
            .unwrap_or_else(|| "null".to_string()),
        files.len(),
        fd_error
    );
    println!("{}", worker::CANNED_OUTPUT_LINES[1]);
    flush_stdout();

    // Consume CPU long enough for the finite 10% quota to accrue observable
    // throttling, while keeping this workload strictly bounded.
    let until = std::time::Instant::now() + Duration::from_secs(5);
    let mut value = 1_u64;
    while std::time::Instant::now() < until {
        value = value.wrapping_mul(6364136223846793005).wrapping_add(1);
        std::hint::black_box(value);
    }
    std::thread::sleep(Duration::from_secs(2));
    drop(files);
    release.store(true, Ordering::Release);
    for thread in threads {
        let _ = thread.join();
    }
    println!("{}", worker::CANNED_OUTPUT_LINES[2]);
    flush_stdout();
    std::process::exit(0)
}
