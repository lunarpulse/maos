//! Story 16-3 / AC4 — NFR-Rel-11's planned half at EVERY `maos run` root,
//! against REAL binaries.
//!
//! # What this file exists to prove
//!
//! At Story 16-3's baseline (§6, measured at `af96c907`):
//!
//! * a serving butler root with a raised `belief_variance` halt exited 0 in
//!   ~22 ms with **no** `lifecycle.unload` row and **no** receipt — the halt
//!   stayed pending forever;
//! * `maosctl halt list --spirit butler` printed **`0 halts shown`** while the
//!   halt row existed, because the halt was raised at pid 0 (D-16-3-N's defect)
//!   and the `--spirit` filter is BY PID — a listed halt is the proof the pid
//!   binding landed;
//! * butler `--once` printed `audit writer drain timed out after 5s` on every
//!   run and wrote a synthetic `term-butler-…` receipt instead of the halt's
//!   own id;
//! * the standalone `[cli_wrapper]`, the interrupted topology `--once` and the
//!   `cohort-a2a-daemon` had NO unload/teardown at all.
//!
//! Every vector here drives the built `maos` binary (a scratch
//! `HOME`/`MAOS_HOME`/`XDG_DATA_HOME` per root, D-16-1-Q), signals it, and
//! reads the state back from the two stores that own it: the process'
//! stderr/stdout and its Transparency Log SQLite — read READ-ONLY, never the
//! write-capable adapter, so these reads cannot hold the lock the live
//! daemon's `busy_timeout` is racing (16-1 Trap 9).
//!
//! # The signal handshake
//!
//! A test that sends SIGTERM before `maos: root supervision armed` appears on
//! stderr is racing the LISTENER's registration (tokio signal streams miss
//! signals delivered before they were created), not testing the teardown.
//! Every signalling vector waits for [`ARMED_MARKER`] first.
//!
//! # The "child gone" oracle (story Trap 22)
//!
//! Find the Worker's runtime client and payload through its published
//! container ID, then use `inspect` by CID to resolve the historical live
//! payload PID. Both client and payload must disappear, and the CID must be
//! removed. No payload environment or payload-parent relationship is used as
//! an ownership oracle.
//!
//! # Vector 6
//!
//! 16-1's idle-SIGTERM budget test
//! (`one_daemon_one_door_16_1.rs::ac1_sigterm_exits_promptly_and_drains_the_audit_channel`)
//! is vector 6; it runs unchanged in ITS file — this suite does not restate
//! an assertion another file owns.

#![forbid(unsafe_code)]

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

/// The stderr handshake (D-16-3-M): the listener's SIGTERM/SIGINT streams are
/// created synchronously at the site BEFORE this line is printed, so a signal
/// sent after reading it is captured by the listener instead of racing the
/// registration.
const ARMED_MARKER: &str = "maos: root supervision armed";

const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";

/// Bounded waits for one-time startup events (binary boot, admission probe,
/// fixture spawn). These are HANG guards, not assertions: a real regression
/// fails the assertion the wait feeds, never the wait itself.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const OUTPUT_ROW_TIMEOUT: Duration = Duration::from_secs(30);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

/// `maosctl` is a `[[bin]]` of a SIBLING package, so `CARGO_BIN_EXE_maosctl`
/// is not defined for this test target. Resolve it beside the `maos` binary
/// cargo built for us — the same profile directory.
fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — build it with the same profile as `maos` \
         (e.g. `cargo build -p maos-cli --bin maosctl`)",
        path.display()
    );
    path
}

/// The fixture binary is resolved as a sibling or parent of the RUNNING exe
/// (`worker_spawn.rs`), so no PATH help is strictly required; prepending the
/// running exe's parent keeps a `--release` suite run working even if a
/// helper regresses to `$PATH` lookup.
fn child_process_path() -> std::ffi::OsString {
    let mut paths = vec![maos().parent().expect("maos has a parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(&paths).expect("valid PATH entries")
}

// ─────────────────────────────────────────────────────────────────────────────
// Transparency Log oracle — read-only, never the write-capable adapter
// ─────────────────────────────────────────────────────────────────────────────

/// One Transparency Log row, reduced to what these vectors join on.
#[derive(Debug, Clone)]
struct TlRow {
    spirit_pid: i64,
    intent: String,
    payload: String,
}

/// Every row of one Transparency Log, in write order.
///
/// READ-ONLY on purpose (16-1 Trap 9). While a root is LIVE the SQLite writer
/// may hold the database for an insert; a read that loses that race returns an
/// EMPTY vec and the caller's polling loop simply tries again — a final
/// assertion is always evaluated after the root has exited, when no lock can
/// still be held.
fn tl_rows(db: &Path) -> Vec<TlRow> {
    if !db.exists() {
        return Vec::new();
    }
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let Ok(mut statement) = conn.prepare(
        "SELECT spirit_pid, intent, payload_redacted FROM transparency_log ORDER BY rowid",
    ) else {
        return Vec::new();
    };
    let rows = statement.query_map([], |row| {
        let payload: Vec<u8> = row.get(2)?;
        Ok(TlRow {
            spirit_pid: row.get(0)?,
            intent: row.get(1)?,
            payload: String::from_utf8_lossy(&payload).into_owned(),
        })
    });
    let Ok(rows) = rows else {
        return Vec::new();
    };
    rows.filter_map(|row| row.ok()).collect()
}

fn rows_with_intent<'a>(rows: &'a [TlRow], intent: &str) -> Vec<&'a TlRow> {
    rows.iter().filter(|row| row.intent == intent).collect()
}

/// The `halt_id` a raised-halt or receipt payload carries, when it parses.
fn payload_halt_id(payload: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()?
        .get("halt_id")?
        .as_str()
        .map(str::to_owned)
}

/// The pid a `lifecycle.load` row bound to one spirit id (D-16-3-N's
/// host-side binding writes `{"spirit_id": …, "spirit_pid": …}` here).
fn load_pid(rows: &[TlRow], spirit_id: &str) -> Option<i64> {
    rows_with_intent(rows, "lifecycle.load")
        .into_iter()
        .filter_map(|row| {
            let value = serde_json::from_str::<serde_json::Value>(&row.payload).ok()?;
            let id = value.get("spirit_id")?.as_str()?;
            (id == spirit_id).then_some(row.spirit_pid)
        })
        .next()
}

fn expect_load_pid(rows: &[TlRow], spirit_id: &str) -> i64 {
    load_pid(rows, spirit_id).unwrap_or_else(|| {
        let loaded: Vec<String> = rows_with_intent(rows, "lifecycle.load")
            .iter()
            .filter_map(|row| {
                serde_json::from_str::<serde_json::Value>(&row.payload)
                    .ok()?
                    .get("spirit_id")?
                    .as_str()
                    .map(str::to_owned)
            })
            .collect();
        panic!("no `lifecycle.load` row for spirit {spirit_id:?}; loaded spirits were {loaded:?}")
    })
}

/// The ids of every RECEIPT (non-empty payload) at one pid, for one
/// termination kind. `terminate_spirit` writes an empty-payload marker row
/// before each receipt; only the receipt rows carry a `halt_id`.
fn receipt_ids(rows: &[TlRow], intent: &str, pid: i64) -> Vec<String> {
    rows_with_intent(rows, intent)
        .iter()
        .filter(|row| row.spirit_pid == pid && !row.payload.is_empty())
        .filter_map(|row| payload_halt_id(&row.payload))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// /proc oracles (the "child gone" oracle — see the module docs)
// ─────────────────────────────────────────────────────────────────────────────

fn proc_cmdline_args(pid: u32) -> Option<Vec<String>> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    Some(
        bytes
            .split(|byte| *byte == 0)
            .filter(|arg| !arg.is_empty())
            .map(|arg| String::from_utf8_lossy(arg).into_owned())
            .collect(),
    )
}

/// `(state, ppid)` from `/proc/<pid>/stat`. The comm field may contain spaces
/// and parentheses, so parsing starts after the LAST `)`.
fn proc_stat(pid: u32) -> Option<(char, u32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    let fields = stat[close + 1..].split_whitespace().collect::<Vec<_>>();
    let state = fields.first()?.chars().next()?;
    let ppid = fields.get(1)?.parse().ok()?;
    Some((state, ppid))
}

/// T3 has a runtime-client PID and a payload host PID. The published CID is
/// the container identity; neither PID nor a requested name authorizes cleanup.
#[derive(Clone, Debug)]
struct FixtureContainer {
    runtime_client_pid: u32,
    payload_host_pid: u32,
    container_id: String,
    runtime_client_start_time: String,
    payload_start_time: String,
}

fn inspect_payload_pid(container_id: &str) -> Result<u32, String> {
    let runtime = std::env::var("MAOS_T3_RUNTIME")
        .map_err(|error| format!("MAOS_T3_RUNTIME unavailable during inspect: {error}"))?;
    let output = Command::new("timeout")
        .arg("8s")
        .arg(runtime)
        .args(["inspect", "--format", "{{.State.Pid}}", container_id])
        .output()
        .map_err(|error| format!("run OCI inspect for CID {container_id}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "OCI inspect CID {container_id} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|error| format!("container {container_id} reported invalid host PID: {error}"))
}

fn fixture_children_of(stdout: &str) -> Vec<FixtureContainer> {
    let Some((runtime_client_pid, container_id)) = stdout.lines().find_map(|line| {
        let event: serde_json::Value = serde_json::from_str(line).ok()?;
        if event["event"] != "cli_wrapper_loaded" {
            return None;
        }
        Some((
            u32::try_from(event["child_pid"].as_u64()?).ok()?,
            event["container_id"].as_str()?.to_owned(),
        ))
    }) else {
        return Vec::new();
    };
    let Ok(payload_host_pid) = inspect_payload_pid(&container_id) else {
        return Vec::new();
    };
    if payload_host_pid <= 1 {
        return Vec::new();
    }
    let Ok(cgroup) = std::fs::read_to_string(format!("/proc/{payload_host_pid}/cgroup")) else {
        return Vec::new();
    };
    if !maos_domain::sandbox::cgroup_names_container(&cgroup, &container_id) {
        return Vec::new();
    }
    let (Some(runtime_client_start_time), Some(payload_start_time)) = (
        proc_start_time(runtime_client_pid),
        proc_start_time(payload_host_pid),
    ) else {
        return Vec::new();
    };
    vec![FixtureContainer {
        runtime_client_pid,
        payload_host_pid,
        container_id,
        runtime_client_start_time,
        payload_start_time,
    }]
}

fn wait_for_fixture_child(root: &RunRoot, mode: &str, timeout: Duration) -> FixtureContainer {
    wait_for_fixture_child_with(|| root.stdout(), mode, timeout, String::new)
}

/// Wait for `cli_wrapper_loaded`, whose published CID is the identity oracle.
fn wait_for_fixture_child_with(
    output: impl Fn() -> String,
    mode: &str,
    timeout: Duration,
    context: impl Fn() -> String,
) -> FixtureContainer {
    let deadline = Instant::now() + timeout;
    loop {
        let mut children = fixture_children_of(&output());
        if let Some(child) = children.pop() {
            return child;
        }
        assert!(
            Instant::now() < deadline,
            "no published {mode} OCI Worker appeared within {timeout:?}; root output:\n{}",
            context()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_pid_gone(pid: u32, start_time: &str, timeout: Duration, what: &str) {
    let deadline = Instant::now() + timeout;
    loop {
        if proc_start_time(pid).as_deref() != Some(start_time) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{what} (pid {pid}) is still in /proc after {timeout:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_container_stopped(container_id: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let runtime = std::env::var("MAOS_T3_RUNTIME").expect("T3 runtime configured");
        let exists = Command::new("timeout")
            .arg("8s")
            .arg(runtime)
            .args(["container", "exists", container_id])
            .output()
            .unwrap_or_else(|error| panic!("check owned CID {container_id} existence: {error}"));
        if exists.status.code() == Some(1) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "OCI container {container_id} was not removed within {timeout:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_worker_stopped(worker: &FixtureContainer, timeout: Duration, what: &str) {
    wait_pid_gone(
        worker.payload_host_pid,
        &worker.payload_start_time,
        timeout,
        what,
    );
    wait_pid_gone(
        worker.runtime_client_pid,
        &worker.runtime_client_start_time,
        timeout,
        "the OCI runtime client for the Worker",
    );
    wait_container_stopped(&worker.container_id, timeout);
}

/// Linux `/proc/<pid>/stat` start time, used to reject reuse after CID lookup.
fn proc_start_time(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    stat[close + 1..]
        .split_whitespace()
        .nth(19)
        .map(str::to_owned)
}

struct GrandchildGuard {
    pid: u32,
    args: Vec<String>,
    start_time: String,
}

impl GrandchildGuard {
    fn still_ours(&self) -> bool {
        proc_cmdline_args(self.pid).as_deref() == Some(self.args.as_slice())
            && proc_start_time(self.pid).as_deref() == Some(self.start_time.as_str())
    }

    fn kill_now(&self) {
        if self.still_ours() {
            let _ = Command::new("kill")
                .args(["-KILL", &self.pid.to_string()])
                .status();
        }
    }
}

impl Drop for GrandchildGuard {
    fn drop(&mut self) {
        self.kill_now();
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.still_ours() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

/// The fixture grandchild is `<payload> --maos-fixture-mode=hang`. Its argv[0]
/// is the binary path natively and the T3 mount `/maos/spirit` in OCI, so the
/// stable identity is its exact mode argument under the inspected payload.
fn wait_for_host_grandchild(parent_host_pid: u32, timeout: Duration) -> u32 {
    let deadline = Instant::now() + timeout;
    loop {
        let found = std::fs::read_dir("/proc")
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
            .find(|pid| {
                proc_stat(*pid).map(|(_, ppid)| ppid) == Some(parent_host_pid)
                    && proc_cmdline_args(*pid).is_some_and(|args| {
                        args.len() == 2 && args[1] == "--maos-fixture-mode=hang"
                    })
            });
        if let Some(pid) = found {
            return pid;
        }
        assert!(
            Instant::now() < deadline,
            "no host-namespace fixture grandchild of payload pid {parent_host_pid} appeared"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn grandchild_guard(pid: u32) -> GrandchildGuard {
    GrandchildGuard {
        pid,
        args: proc_cmdline_args(pid).unwrap_or_else(|| panic!("grandchild {pid} has no argv")),
        start_time: proc_start_time(pid)
            .unwrap_or_else(|| panic!("grandchild {pid} has no /proc start time")),
    }
}

/// The grandchild pid from the fixture's FIRST stdout line
/// (`worker-fixture: grandchild pid <n>`), read back out of the Transparency
/// Log row the stdio bridge journaled for it — proof the line really flowed
/// through the bridge, not just that some process exists.
fn wait_for_grandchild_pid(db: &Path, timeout: Duration) -> u32 {
    let deadline = Instant::now() + timeout;
    loop {
        for row in tl_rows(db) {
            let Some(tail) = row.payload.split("grandchild pid ").nth(1) else {
                continue;
            };
            let digits: String = tail
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if let Ok(pid) = digits.parse::<u32>() {
                return pid;
            }
        }
        assert!(
            Instant::now() < deadline,
            "no `grandchild pid` output row landed in {} within {timeout:?}",
            db.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The run-root harness — one fresh root per vector (16-1's shape)
// ─────────────────────────────────────────────────────────────────────────────

/// Own the root from `spawn`, before any Worker or grandchild discovery can
/// panic. Native descendants inherit its private process group; T3 cleanup
/// uses only CIDs published by this root, never a guessed name or payload PID.
struct OwnedRoot {
    child: Child,
    /// The root was reaped by a wait below, so its pid — and with it the group
    /// id — may since have been recycled.
    reaped: bool,
    stdout_text: Arc<Mutex<String>>,
    stdout_reader: Option<std::thread::JoinHandle<()>>,
    runtime: Option<std::ffi::OsString>,
    environment: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)>,
}

impl OwnedRoot {
    fn spawn(command: &mut Command) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let environment: Vec<_> = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(std::ffi::OsStr::to_os_string)))
            .collect();
        let runtime = environment
            .iter()
            .find(|(key, _)| key == "MAOS_T3_RUNTIME")
            .and_then(|(_, value)| value.clone())
            .or_else(|| std::env::var_os("MAOS_T3_RUNTIME"));
        Self {
            child: command.spawn().expect("spawn owned root"),
            reaped: false,
            stdout_text: Arc::new(Mutex::new(String::new())),
            stdout_reader: None,
            runtime,
            environment,
        }
    }

    fn capture_stdout(&mut self) -> Arc<Mutex<String>> {
        let stdout = self.child.stdout.take().expect("root stdout piped");
        let capture = Arc::clone(&self.stdout_text);
        self.stdout_reader = Some(std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let mut sink = capture.lock().expect("stdout capture lock poisoned");
                sink.push_str(&line);
                sink.push('\n');
            }
        }));
        Arc::clone(&self.stdout_text)
    }

    /// Shadows `Child::try_wait` so a reap is recorded for the group cleanup.
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        let status = self.child.try_wait()?;
        self.reaped |= status.is_some();
        Ok(status)
    }

    /// Shadows `Child::wait`; see [`Self::try_wait`].
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }
}

impl std::ops::Deref for OwnedRoot {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.child
    }
}

impl std::ops::DerefMut for OwnedRoot {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

impl Drop for OwnedRoot {
    fn drop(&mut self) {
        // Signal the group only while its id provably still belongs to it:
        // an unreaped root (even a zombie) holds its pid, and a reaped root's
        // group stays reserved while any member lives. A reaped root whose pid
        // now names a live process means the id was recycled — never signal it.
        #[cfg(unix)]
        if let Some(group) = rustix::process::Pid::from_raw(self.child.id() as i32) {
            let recycled = self.reaped && Path::new(&format!("/proc/{}", self.child.id())).exists();
            if !recycled {
                let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.stdout_reader.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !reader.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if reader.is_finished() {
                let _ = reader.join();
            }
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        let stdout = self
            .stdout_text
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for line in stdout.lines() {
            let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if event["event"] != "cli_wrapper_loaded" {
                continue;
            }
            let Some(cid) = event["container_id"].as_str() else {
                continue;
            };
            if cid.len() != 64 || !cid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                eprintln!("owned-root cleanup refused invalid published CID {cid:?}");
                continue;
            }
            let mut cleanup = Command::new("timeout");
            cleanup.arg("8s").arg(runtime).args(["rm", "-f", "--", cid]);
            for (key, value) in &self.environment {
                if let Some(value) = value {
                    cleanup.env(key, value);
                } else {
                    cleanup.env_remove(key);
                }
            }
            match cleanup.output() {
                Ok(output) if output.status.success() => {}
                result => eprintln!("owned-root cleanup of published CID {cid}: {result:?}"),
            }
        }
    }
}

/// This is a real fixture process, not a mock of the ownership oracle. Only
/// after the planted unwind do we discover its grandchild and measure it.
#[cfg(target_os = "linux")]
#[test]
fn harness_owns_grandchildren_before_discovery_even_when_startup_panics() {
    let fixture = maos().with_file_name("worker-cli-fixture");
    assert!(
        fixture.is_file(),
        "build the real fixture with cargo build -p worker --bin worker-cli-fixture"
    );
    let captured = Arc::new(Mutex::new(None));
    let result = std::panic::catch_unwind({
        let captured = Arc::clone(&captured);
        move || {
            let mut command = Command::new(fixture);
            command
                .arg("--maos-fixture-mode=hang-with-grandchild")
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let mut root = OwnedRoot::spawn(&mut command);
            let stdout = root.capture_stdout();
            *captured.lock().expect("capture slot") = Some(Arc::clone(&stdout));
            let deadline = Instant::now() + Duration::from_secs(10);
            while !stdout
                .lock()
                .expect("fixture stdout")
                .contains("grandchild pid ")
            {
                assert!(
                    Instant::now() < deadline,
                    "fixture did not spawn its grandchild"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("planted panic after spawn, before grandchild discovery");
        }
    });
    let panic = result.expect_err("the planted startup panic must execute");
    assert_eq!(
        panic.downcast_ref::<&str>().copied(),
        Some("planted panic after spawn, before grandchild discovery")
    );
    let stdout = captured
        .lock()
        .expect("capture slot")
        .take()
        .expect("fixture ran");
    let text = stdout.lock().expect("fixture stdout").clone();
    let grandchild: u32 = text
        .split_once("grandchild pid ")
        .expect("real fixture announcement")
        .1
        .trim()
        .parse()
        .expect("announced grandchild pid");
    let deadline = Instant::now() + Duration::from_secs(5);
    let live = loop {
        let live = proc_cmdline_args(grandchild)
            .is_some_and(|args| args.iter().any(|arg| arg == "--maos-fixture-mode=hang"));
        if !live || Instant::now() >= deadline {
            break live;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let ps = Command::new("ps")
        .args(["-eo", "pid,ppid,etimes,args"])
        .output()
        .expect("measure fixture processes after unwind");
    println!("planted-panic grandchild pid={grandchild}; live after unwind={live}");
    for line in String::from_utf8_lossy(&ps.stdout).lines() {
        if line.contains("worker-cli-fixture") {
            println!("{line}");
        }
    }
    if live {
        let status = Command::new("kill")
            .args(["-KILL", &grandchild.to_string()])
            .status()
            .expect("clean up planted leak");
        println!("cleaned planted leak pid={grandchild}: {status}");
    }
    assert!(
        !live,
        "pre-discovery panic leaked fixture grandchild {grandchild}"
    );
}

struct RunRoot {
    scratch: PathBuf,
    home: PathBuf,
    db: PathBuf,
    child: Option<OwnedRoot>,
    stdout_text: Arc<Mutex<String>>,
    stderr_text: Arc<Mutex<String>>,
    stderr_lines: mpsc::Receiver<String>,
}

impl RunRoot {
    /// `maos run <manifest> [--once]` on a scratch home. Every root gets its
    /// own `HOME`/`MAOS_HOME`/`XDG_DATA_HOME` (D-16-1-Q): 16-1's decoy door
    /// otherwise holds this runner's real `control.json` endpoint.
    fn spawn(label: &str, manifest: &Path, once: bool, extra_env: &[(&str, &str)]) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "maos-root-shutdown-16-3-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let home = scratch.join("home");
        std::fs::create_dir_all(&home).expect("create scratch home");

        let init = Command::new(maos())
            .arg("init")
            .env("HOME", &scratch)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maos init");
        assert!(
            init.status.success(),
            "maos init must exit 0 on a fresh home: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let mut command = Command::new(maos());
        command.env_clear().arg("run").arg(manifest);
        if once {
            command.arg("--once");
        }
        command
            .env("HOME", t3_substrate::isolated_runtime_home(&scratch))
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("PATH", child_process_path())
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in [
            "MAOS_T3_RUNTIME",
            "MAOS_T3_IMAGE_LOCK_PATH",
            "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let mut child = OwnedRoot::spawn(&mut command);
        let stdout_text = child.capture_stdout();
        let stderr = child.stderr.take().expect("stderr piped");
        let stderr_text = Arc::new(Mutex::new(String::new()));
        let (tx, rx) = mpsc::channel::<String>();
        {
            let capture = Arc::clone(&stderr_text);
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    let mut sink = capture.lock().expect("stderr capture lock poisoned");
                    sink.push_str(&line);
                    sink.push('\n');
                    let _ = tx.send(line);
                }
            });
        }

        Self {
            db: home.join("audit").join("transparency.sqlite"),
            home,
            scratch,
            child: Some(child),
            stdout_text,
            stderr_text,
            stderr_lines: rx,
        }
    }

    fn butler_serving(label: &str, extra_env: &[(&str, &str)]) -> Self {
        let mut env = vec![
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE),
        ];
        env.extend_from_slice(extra_env);
        Self::spawn(label, &workspace_root().join(BUTLER_MANIFEST), false, &env)
    }

    fn butler_once(label: &str, extra_env: &[(&str, &str)]) -> Self {
        let mut env = vec![
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE),
        ];
        env.extend_from_slice(extra_env);
        Self::spawn(label, &workspace_root().join(BUTLER_MANIFEST), true, &env)
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("root still running").id()
    }

    fn stderr(&self) -> String {
        self.stderr_text
            .lock()
            .expect("stderr capture lock poisoned")
            .clone()
    }

    fn stdout(&self) -> String {
        self.stdout_text
            .lock()
            .expect("stdout capture lock poisoned")
            .clone()
    }

    /// Block until stderr names `needle` — the ARMED handshake and the
    /// readiness lines live here. A signal sent BEFORE this returns is racing
    /// the listener's registration, not testing the teardown.
    fn wait_for_stderr(&self, needle: &str, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let mut seen = String::new();
        while Instant::now() < deadline {
            loop {
                match self.stderr_lines.try_recv() {
                    Ok(line) => {
                        seen.push_str(&line);
                        seen.push('\n');
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            if seen.contains(needle) || self.stderr().contains(needle) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "stderr never printed {needle:?} within {timeout:?}; captured:\n{}",
            self.stderr()
        );
    }

    fn sigterm(&self) {
        let pid = self.pid();
        let sent = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .expect("run kill(1) — this crate is #![forbid(unsafe_code)]");
        assert!(sent.success(), "kill -TERM {pid} failed");
    }

    /// Reap the root or kill it and fail loudly — a root that never exits is
    /// the regression several vectors exist to catch, so the timeout message
    /// carries the whole stderr.
    fn wait_exit(&mut self, bound: Duration) -> (std::process::ExitStatus, Duration) {
        // Keep spawn ownership until all shutdown assertions have measured the
        // production teardown; dropping it here would clean up the regression.
        let child = self.child.as_mut().expect("root still owned");
        let started = Instant::now();
        loop {
            match child.try_wait().expect("poll root status") {
                Some(status) => return (status, started.elapsed()),
                None if started.elapsed() >= bound => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "root did not exit within {bound:?}; stderr:\n{}",
                        self.stderr()
                    );
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }

    fn tl(&self) -> Vec<TlRow> {
        tl_rows(&self.db)
    }

    /// `maosctl` against THIS root's stores (offline readers resolve
    /// everything from `MAOS_HOME`; no daemon needed).
    fn ctl(&self, args: &[&str]) -> std::process::Output {
        Command::new(maosctl())
            .args(args)
            .env("HOME", &self.scratch)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl")
    }
}

impl Drop for RunRoot {
    fn drop(&mut self) {
        drop(self.child.take());
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Worker-manifest / topology-manifest fixtures
// ─────────────────────────────────────────────────────────────────────────────

/// The only known-admitting `[cli_wrapper]` manifest shape (the built-in host
/// grant names `worker-cli-fixture` at T3 exactly), with the Story-16-3
/// fixture-mode flag carried in `argv_prefix` — hashed into the cap token, so
/// the mode a root runs is part of what was admitted.
fn write_worker_manifest(dir: &Path, mode: &str) -> PathBuf {
    let path = dir.join(format!("worker-{mode}.toml"));
    std::fs::write(
        &path,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode={mode}\"]\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n"
        ),
    )
    .expect("write worker manifest");
    path
}

/// A `[topology]` manifest over ABSOLUTE child-manifest paths (a scratch
/// directory cannot borrow the workspace's `../orchestrator` relative form).
fn write_topology_manifest(dir: &Path, name: &str, child_manifests: &[PathBuf]) -> PathBuf {
    let mut body = format!("[topology]\nname = \"{name}\"\n");
    for manifest in child_manifests {
        body.push_str(&format!(
            "\n[[topology.spirits]]\nmanifest = \"{}\"\n",
            manifest.display()
        ));
    }
    let path = dir.join(format!("{name}.toml"));
    std::fs::write(&path, body).expect("write topology manifest");
    path
}

fn class_manifest(class: &str) -> PathBuf {
    workspace_root().join(format!("spirits/{class}/manifest.toml"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 1 — serving butler: the halt is LISTED at butler's pid, and SIGTERM
// receipts it with its own id
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (1) — D-16-3-N + D-16-3-M at the serving root.
///
/// Refuses three HEAD defects at once: (a) `halt list --spirit butler`
/// printing `0 halts shown` while the row existed (the halt used to be raised
/// at pid 0; the filter is BY PID, so a listed halt IS at butler's real pid);
/// (b) SIGTERM leaving the halt pending with no unload and no receipt; (c) a
/// synthetic `term-…` receipt standing in for the halt's own id.
#[test]
fn ac4_v1_serving_butler_receipts_the_raised_halt_on_sigterm() {
    if !t3_substrate::require_t3("root shutdown serving Worker") {
        return;
    }
    let mut root = RunRoot::butler_serving("v1", &[("MAOS_IDLE_FAST", "1")]);

    // The handshake first: arming installs the listener's streams; everything
    // this vector signals afterwards is captured by the listener.
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    // Wait for the halt itself. Deliberately PID-AGNOSTIC: which pid the row
    // sits at is the D-16-3-N assertion below, not a gate — if this poll
    // filtered by butler's pid, a reverted pid binding would fail the vector
    // HERE (on row presence) and the receipt half would never be exercised.
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    let raised_before: Vec<String> = loop {
        let raised: Vec<String> = rows_with_intent(&root.tl(), "belief_variance")
            .iter()
            .filter_map(|row| payload_halt_id(&row.payload))
            .collect();
        if !raised.is_empty() {
            break raised;
        }
        assert!(
            Instant::now() < deadline,
            "butler never raised belief_variance within {STARTUP_TIMEOUT:?}; stderr:\n{}",
            root.stderr()
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let butler_pid = expect_load_pid(&root.tl(), "butler");

    // A pre-signal listing, recorded for the run log: `--spirit` filters by
    // resolved pid, so a listed halt IS at butler's real pid. (The HARD
    // pid-binding assertions run after the receipt half — see below.)
    let listed = root.ctl(&["halt", "list", "--spirit", "butler"]);
    assert!(
        listed.status.success(),
        "halt list must exit 0: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    println!(
        "v1: halt list before SIGTERM -> {}",
        String::from_utf8_lossy(&listed.stdout).trim()
    );

    // Now the teardown under test.
    root.sigterm();
    let (status, elapsed) = root.wait_exit(Duration::from_secs(3));
    let stderr = root.stderr();
    assert!(
        status.success(),
        "a serving root's SIGTERM keeps exit 0, got {status:?}: {stderr}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "16-1's budget: the root must exit within 3 s of SIGTERM, took {elapsed:?}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "every audit_tx owner is dropped before the writer await: {stderr}"
    );

    // THE RECEIPT HALF — every raised halt got a PlannedUnload receipt
    // carrying ITS OWN id, read after exit so no in-flight write can race the
    // comparison. With the D-16-3-N pid binding reverted this is the
    // assertion that must red, and on the `term-` id: the halt is raised at
    // pid 0, `drain_for_spirit(butler)` misses it, and `terminate_spirit`
    // writes the synthetic `term-butler-…` form in its place.
    let rows = root.tl();
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == butler_pid),
        "butler (pid {butler_pid}) must have a lifecycle.unload row; rows:\n{:?}",
        rows_with_intent(&rows, "lifecycle.unload")
    );
    let receipts = receipt_ids(&rows, "planned_unload", butler_pid);
    assert!(
        !receipts.is_empty(),
        "no PlannedUnload receipt at butler's pid {butler_pid}"
    );
    let raised_after: Vec<String> = rows_with_intent(&rows, "belief_variance")
        .iter()
        .filter_map(|row| payload_halt_id(&row.payload))
        .collect();
    assert!(
        !raised_after.is_empty(),
        "the raised halt rows must survive the run — a receipt without its halt proves nothing"
    );
    let mut sorted_receipts = receipts.clone();
    let mut sorted_raised = raised_after.clone();
    sorted_receipts.sort();
    sorted_raised.sort();
    assert_eq!(
        sorted_receipts, sorted_raised,
        "every raised halt needs a receipt with its own id and nothing else; \
         raised before SIGTERM was {raised_before:?}"
    );
    assert!(
        receipts.iter().all(|id| !id.starts_with("term-")),
        "a synthetic term- receipt must never stand in for a raised halt: \
         receipts {receipts:?} for raised {raised_after:?}"
    );

    // THE PID-BINDING HALF (D-16-3-N): the raised row sits at BUTLER's pid —
    // never pid 0 — and the offline `halt list --spirit butler` therefore
    // lists it. At HEAD this printed `0 halts shown` while the row existed.
    assert!(
        !rows
            .iter()
            .any(|row| row.intent == "belief_variance" && row.spirit_pid == 0),
        "a belief_variance row at pid 0 means the D-16-3-N pid binding regressed"
    );
    let listed_after = root.ctl(&["halt", "list", "--spirit", "butler"]);
    let listed_stdout = String::from_utf8_lossy(&listed_after.stdout).into_owned();
    let listed_raised: Vec<String> = listed_stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .unwrap_or_else(|error| panic!("halt list line {line:?} is not JSON: {error}"))
        })
        .filter(|value| value.get("record").and_then(|record| record.as_str()) == Some("raised"))
        .map(|value| {
            value
                .get("halt_id")
                .and_then(|id| id.as_str())
                .expect("a raised row always lists its halt_id")
                .to_string()
        })
        .collect();
    assert!(
        listed_raised.contains(&raised_after[0]),
        "`halt list --spirit butler` must list the raised halt (a listed halt \
         IS at butler's pid — the `0 halts shown` defect D-16-3-N closes); \
         stdout:\n{listed_stdout}"
    );
    assert!(
        stderr.contains("closed by planned unload"),
        "the caller renders `halt <id> closed by planned unload (no resolution)`: {stderr}"
    );
    println!(
        "v1: SIGTERM-to-exit {elapsed:?}; halt ids raised={raised_after:?} receipted={receipts:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 2 — butler `--once`: same receipt, the drain completes, the revoke
// row lands
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (2) — the single-class `--once` teardown.
///
/// Refuses: the ~5.06 s `drain timed out` HEAD measured on every butler
/// `--once` (twenty-odd `audit_tx` owners left alive), the missing unload, and
/// the synthetic `term-` receipt.
#[test]
fn ac4_v2_butler_once_receipts_the_raised_halt_and_drains() {
    let started = Instant::now();
    if !t3_substrate::require_t3("root shutdown one-shot Worker") {
        return;
    }
    let mut root = RunRoot::butler_once("v2", &[("MAOS_IDLE_FAST", "1")]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    // `--once` runs its one pass and exits by itself; no signal is involved.
    let (status, elapsed) = root.wait_exit(Duration::from_secs(45));
    let stderr = root.stderr();
    let stdout = root.stdout();
    assert!(
        status.success(),
        "butler --once must exit 0, got {status:?}; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "the pass completed — the writer await must not time out (HEAD: every \
         butler --once printed `drain timed out after 5s`): {stderr}"
    );
    assert!(
        stderr.contains("--once complete — exiting cleanly"),
        "the clean-completion line must be the run's last word: {stderr}"
    );

    // The halt the pass raised, as the process itself reported it — the
    // stdout JSON carries the Debug form `HaltId("…")`, normalised here.
    let halt_event = stdout
        .lines()
        .find_map(|line| {
            let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
            (value.get("event").and_then(|event| event.as_str()) == Some("halt")).then_some(value)
        })
        .unwrap_or_else(|| panic!("no `halt` event on stdout; stdout:\n{stdout}"));
    let debug_id = halt_event
        .get("halt_id")
        .and_then(|id| id.as_str())
        .expect("the halt event names its halt_id")
        .to_string();
    let reported_pid = halt_event
        .get("spirit_pid")
        .and_then(|pid| pid.as_i64())
        .expect("the halt event names its spirit_pid");
    let raised_id = debug_id
        .strip_prefix("HaltId(\"")
        .and_then(|rest| rest.strip_suffix("\")"))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            panic!("the stdout halt id must be the Debug form HaltId(\"…\"), got {debug_id:?}")
        });
    assert!(
        !raised_id.starts_with("term-"),
        "the pass reported the synthetic id — the halt it raised was lost: {raised_id}"
    );

    let rows = root.tl();
    let butler_pid = expect_load_pid(&rows, "butler");
    assert_eq!(
        reported_pid, butler_pid,
        "the pass's halt must be bound at butler's loaded pid (D-16-3-N)"
    );
    let receipts = receipt_ids(&rows, "planned_unload", butler_pid);
    assert!(
        receipts.iter().any(|id| *id == raised_id),
        "the PlannedUnload receipt must carry the raised halt's id \
         {raised_id:?}; receipts at pid {butler_pid} were {receipts:?}"
    );
    assert!(
        receipts.iter().all(|id| !id.starts_with("term-")),
        "no synthetic receipt may stand beside the halt's own: {receipts:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == butler_pid),
        "the unload must have written butler's lifecycle.unload row"
    );
    // The drain invariant's second half: the unload's `SpiritUnload` revoke
    // reached the log through the audit channel — proof the writer was
    // awaited with every owner dropped.
    assert!(
        rows.iter()
            .any(|row| row.intent == "cap.revoke" && row.spirit_pid == butler_pid),
        "the unload's SpiritUnload revoke row is missing at butler's pid {butler_pid}"
    );
    println!(
        "v2: butler --once total {elapsed:?} (test {} s), no `drain timed out`; \
         raised halt {raised_id:?} == receipted at pid {butler_pid}",
        started.elapsed().as_secs_f32()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 3 — topology `--once` unloads EVERY loaded spirit
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (3) — the topology `--once` teardown.
///
/// Refuses the pre-story shape where the topology tail returned `Ok` without
/// unloading anything: after a clean `--once` every spirit the topology
/// loaded has its own `lifecycle.unload` row.
#[test]
fn ac4_v3_topology_once_unloads_every_loaded_spirit() {
    if !t3_substrate::require_t3("root shutdown topology Worker") {
        return;
    }
    let scratch = scratch_dir("v3");
    let topology = write_topology_manifest(
        &scratch,
        "v3-unload-topology",
        &[class_manifest("architect"), class_manifest("reviewer")],
    );
    let mut root = RunRoot::spawn("v3", &topology, true, &[("MAOS_OLLAMA_URL", "skip")]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);
    let (status, elapsed) = root.wait_exit(Duration::from_secs(90));
    let stderr = root.stderr();
    assert!(
        status.success(),
        "a clean topology --once must exit 0, got {status:?}: {stderr}"
    );
    assert!(
        stderr.contains("topology --once complete — exiting cleanly"),
        "the topology tail must reach its clean-completion line: {stderr}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "the topology drain must complete: {stderr}"
    );

    let rows = root.tl();
    let mut unloaded = 0;
    for spirit in ["architect", "reviewer"] {
        let pid = expect_load_pid(&rows, spirit);
        assert!(
            rows.iter()
                .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == pid),
            "{spirit} (pid {pid}) was loaded but never unloaded; unload rows:\n{:?}",
            rows_with_intent(&rows, "lifecycle.unload")
        );
        unloaded += 1;
    }
    assert_eq!(unloaded, 2, "every loaded spirit is checked");
    println!("v3: topology --once {elapsed:?}, both class spirits unloaded");
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 3b — SIGTERM mid-topology: what loaded is unloaded, the worker's
// task is nacked, nothing later loads, exit 1
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (3b) — a topology `--once` broken off mid-load.
///
/// Refuses: an interrupted topology exiting 0 (the 2a AC1.5 false-success
/// shape), a hung Worker's task left in flight (no `task.nacked`), spirits
/// that already loaded left undrained, and entries BEYOND the signal still
/// being admitted.
#[test]
fn ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker() {
    if !t3_substrate::require_t3("root shutdown interrupted topology Worker") {
        return;
    }
    let scratch = scratch_dir("v3b");
    let worker = write_worker_manifest(&scratch, "hang");
    let topology = write_topology_manifest(
        &scratch,
        "v3b-interrupted-topology",
        &[
            class_manifest("architect"),
            worker,
            class_manifest("reviewer"),
        ],
    );
    let mut root = RunRoot::spawn("v3b", &topology, true, &[("MAOS_OLLAMA_URL", "skip")]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    // The root publishes the container CID in cli_wrapper_loaded; inspect it
    // by CID and validate the payload's cgroup identity before signalling.
    let worker = wait_for_fixture_child(&root, "hang", STARTUP_TIMEOUT);

    root.sigterm();
    let (status, elapsed) = root.wait_exit(Duration::from_secs(45));
    let stderr = root.stderr();
    let exit_code = status
        .code()
        .expect("a broken-off --once exits with a code, not a signal");
    assert_eq!(
        exit_code, 1,
        "an interrupted topology --once must exit 1 (an exit 0 there is the \
         false-success shape): stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("interrupted by signal before the --once pass completed"),
        "the interrupted line must replace `complete — exiting cleanly`: {stderr}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "the teardown still drains: {stderr}"
    );
    wait_worker_stopped(
        &worker,
        Duration::from_secs(5),
        "the interrupted topology Worker",
    );

    let rows = root.tl();
    // The Worker: unloaded, receipted, and its in-flight task nacked by id.
    let worker_pid = expect_load_pid(&rows, "worker");
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid),
        "the interrupted Worker's SCB must be unloaded; unload rows:\n{:?}",
        rows_with_intent(&rows, "lifecycle.unload")
    );
    let worker_receipts = receipt_ids(&rows, "planned_unload", worker_pid);
    assert!(
        !worker_receipts.is_empty(),
        "the Worker's PlannedUnload receipt is missing at pid {worker_pid}"
    );
    let nack_rows = rows_with_intent(&rows, "task.nacked");
    let nack = nack_rows
        .iter()
        .find(|row| row.payload.contains("\"task_id\":\"topology:1\""))
        .unwrap_or_else(|| {
            panic!(
                "the hang Worker is topology entry 1, so its task id is `topology:1`; \
                 task.nacked rows:\n{nack_rows:?}"
            )
        });
    assert!(
        nack.payload
            .contains("\"originator_spirit_id\":\"topology\""),
        "the nack names its originator: {}",
        nack.payload
    );
    // Every previously loaded spirit is unloaded too.
    let architect_pid = expect_load_pid(&rows, "architect");
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == architect_pid),
        "architect (entry 0) loaded before the signal and must be unloaded"
    );
    // And NO later entry was admitted.
    assert!(
        load_pid(&rows, "reviewer").is_none(),
        "the reviewer entry sits AFTER the signalled Worker — it must never load"
    );
    println!("v3b: SIGTERM mid-worker, exit 1 in {elapsed:?}, worker nacked and unloaded");
}

// ─────────────────────────────────────────────────────────────────────────────
// Vectors 4 / 4b — the standalone `[cli_wrapper]` root under SIGTERM
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (4) — SIGTERM during a standalone `hang` Worker.
///
/// Refuses: an orphaned Worker process outliving the root, a root that hangs
/// past its own teardown, the missing unload/receipt, and a drain timeout.
#[test]
fn ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan() {
    if !t3_substrate::require_t3("root shutdown standalone Worker") {
        return;
    }
    let scratch = scratch_dir("v4");
    let worker = write_worker_manifest(&scratch, "hang");
    let mut root = RunRoot::spawn("v4", &worker, true, &[]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    let worker = wait_for_fixture_child(&root, "hang", STARTUP_TIMEOUT);

    root.sigterm();
    let (status, elapsed) = root.wait_exit(Duration::from_secs(25));
    let stderr = root.stderr();
    assert!(
        !status.success(),
        "a standalone run broken off by SIGTERM must exit non-zero (the pass \
         never completed), got {status:?}: {stderr}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "the standalone drain must complete: {stderr}"
    );
    // The child was SIGKILLed by the supervisor and reaped by its observer —
    // the /proc oracle (see module docs): gone means gone.
    wait_worker_stopped(&worker, Duration::from_secs(5), "the standalone Worker");

    let rows = root.tl();
    let worker_pid = expect_load_pid(&rows, "worker");
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid),
        "the standalone Worker's SCB must be unloaded by the root's teardown"
    );
    let receipts = receipt_ids(&rows, "planned_unload", worker_pid);
    assert!(
        !receipts.is_empty(),
        "a PlannedUnload receipt must exist for the standalone Worker at pid {worker_pid}"
    );
    println!(
        "v4: SIGTERM during standalone hang worker, exit {:?} in {elapsed:?}",
        status.code()
    );
}

/// AC3 — an operator-door unload while the Worker is live must signal the
/// child, dispose its task once, unload its SCB, and receipt the unload.
#[test]
fn ac3_operator_door_unload_stops_a_live_worker_once() {
    if !t3_substrate::require_t3("root unload live Worker") {
        return;
    }
    let scratch = scratch_dir("ac3-door-unload");
    let worker = write_worker_manifest(&scratch, "hang");
    let mut root = RunRoot::spawn("ac3-door-unload", &worker, true, &[]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    let worker = wait_for_fixture_child(&root, "hang", STARTUP_TIMEOUT);
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    let worker_pid = loop {
        if let Some(pid) = load_pid(&root.tl(), "worker") {
            break pid;
        }
        assert!(
            Instant::now() < deadline,
            "the live Worker never produced its lifecycle.load row"
        );
        std::thread::sleep(Duration::from_millis(20));
    };

    let unload = root.ctl(&["unload", "worker"]);
    assert!(
        unload.status.success(),
        "maosctl unload worker failed: {}",
        String::from_utf8_lossy(&unload.stderr)
    );

    let (_status, elapsed) = root.wait_exit(Duration::from_secs(25));
    wait_worker_stopped(
        &worker,
        Duration::from_secs(5),
        "the operator-unloaded Worker",
    );

    let rows = root.tl();
    let unloads = rows
        .iter()
        .filter(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid)
        .count();
    assert_eq!(unloads, 1, "the Worker's SCB must unload exactly once");
    let receipts = receipt_ids(&rows, "planned_unload", worker_pid);
    assert_eq!(
        receipts.len(),
        1,
        "the operator unload must produce exactly one PlannedUnload receipt"
    );
    assert_eq!(
        rows_with_intent(&rows, "task.nacked").len(),
        1,
        "the live Worker's task must be disposed exactly once"
    );
    println!("AC3 door unload: live Worker stopped and receipted in {elapsed:?}");
}

/// AC4 (4b) — root shutdown stops the complete T3 Worker container, including
/// its descendant, instead of leaving a payload process behind.
#[test]
fn ac4_v4b_grandchild_is_stopped_with_its_worker_container() {
    if !t3_substrate::require_t3("root shutdown descendant Worker") {
        return;
    }
    let scratch = scratch_dir("v4b");
    let worker_manifest = write_worker_manifest(&scratch, "hang-with-grandchild");
    let mut root = RunRoot::spawn("v4b", &worker_manifest, true, &[]);
    root.wait_for_stderr(ARMED_MARKER, STARTUP_TIMEOUT);

    let worker = wait_for_fixture_child(&root, "hang-with-grandchild", STARTUP_TIMEOUT);
    let _announced_guest_pid = wait_for_grandchild_pid(&root.db, OUTPUT_ROW_TIMEOUT);
    let grandchild = wait_for_host_grandchild(worker.payload_host_pid, OUTPUT_ROW_TIMEOUT);
    let guard = grandchild_guard(grandchild);

    root.sigterm();
    let (status, elapsed) = root.wait_exit(Duration::from_secs(25));
    assert!(
        !status.success(),
        "the interrupted --once pass must not report success: {}",
        root.stderr()
    );
    wait_worker_stopped(
        &worker,
        Duration::from_secs(5),
        "the descendant-holding Worker",
    );
    assert!(
        !guard.still_ours(),
        "stopping the Worker's CID-owned T3 container must remove its grandchild"
    );
    let rows = root.tl();
    let worker_pid = expect_load_pid(&rows, "worker");
    assert!(
        rows.iter()
            .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid),
        "root shutdown must unload the Worker SCB"
    );
    assert_eq!(
        receipt_ids(&rows, "planned_unload", worker_pid).len(),
        1,
        "root shutdown must persist one PlannedUnload receipt"
    );
    println!("v4b: stopped Worker container and descendant during root teardown in {elapsed:?}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 8 — the supervision did not leak into the one-shot arms
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 (8) — `MAOS_ONE_SHOT=acp-server` has NO signal listener.
///
/// One-shot arms dispatch BEFORE the run block arms `RootSupervision`, so no
/// SIGTERM stream exists there and the default disposition holds: the process
/// dies BY SIGNAL 15, promptly. (Recorded at HEAD-equivalent first, per the
/// story: the one-shot arms never armed a listener, so signal-death is the
/// pre-existing behaviour this vector pins against leakage.)
///
/// Refuses: a leaked `RootSupervision` arming inside the one-shot — tokio
/// never restores the default disposition, so a leaked listener would turn
/// signal-death into a swallowed signal and a root that will not die.
#[cfg(unix)]
#[test]
fn ac4_v8_one_shot_acp_server_has_no_listener_and_dies_by_signal_15() {
    use std::io::Write as _;

    let scratch = scratch_dir("v8");
    let home = scratch.join("home");
    std::fs::create_dir_all(&home).expect("create scratch home");
    let mut child = Command::new(maos())
        .env("MAOS_ONE_SHOT", "acp-server")
        .env("MAOS_OLLAMA_URL", "skip")
        .env("HOME", &home)
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn maos acp-server");
    let stdout = child.stdout.take().expect("stdout piped");
    let stdout_text = Arc::new(Mutex::new(String::new()));
    {
        let capture = Arc::clone(&stdout_text);
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let mut sink = capture.lock().expect("stdout capture lock poisoned");
                sink.push_str(&line);
                sink.push('\n');
            }
        });
    }

    // Hold stdin open and drive a real session so a leaked registration would
    // already be in place by the time the signal lands.
    let session_start = concat!(
        r#"{"kind":"session_start","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"#,
        r#""editor_id":"jetbrains","editor_version":"2024.3"}"#,
    );
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(format!("{session_start}\n").as_bytes())
        .expect("send session_start");
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while !stdout_text
        .lock()
        .expect("stdout capture lock poisoned")
        .contains("session_ready")
    {
        assert!(
            Instant::now() < deadline,
            "the acp server never answered session_ready; stdout:\n{}",
            stdout_text.lock().expect("stdout capture lock poisoned")
        );
        if let Some(status) = child.try_wait().expect("poll acp server") {
            panic!("the acp server exited before session_ready ({status})");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let pid = child.id();
    let sent = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("run kill(1)");
    assert!(sent.success(), "kill -TERM {pid} failed");

    // Death BY SIGNAL 15 within 2 s is the contract.
    use std::os::unix::process::ExitStatusExt as _;
    let started = Instant::now();
    let status = loop {
        match child.try_wait().expect("poll acp server") {
            Some(status) => break status,
            None if started.elapsed() >= Duration::from_secs(2) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "the one-shot must die by signal 15 within 2 s — a leaked \
                     listener would swallow the signal instead"
                );
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    assert_eq!(
        status.signal(),
        Some(15),
        "the one-shot must die BY SIGNAL 15 (no listener, default disposition), \
         got {status:?}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "maos-root-shutdown-16-3-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

// ─────────────────────────────────────────────────────────────────────────────
// Vectors 5 / 5b / 5c — the cohort-a2a-daemon root (two-host TLS fixture,
// helpers reused from `two_host_delegation_2b.rs`)
//
// Gated on the `network` feature exactly like the fixture they reuse: the
// daemon arm, the optional TLS crates and `maos-cohort` all exist only in a
// network build (maos-bin's DEFAULT), so an air-gap build still compiles this
// file — with these three vectors absent, and nothing else.
// ─────────────────────────────────────────────────────────────────────────────
//
// ⚠ THESE THREE VECTORS REQUIRE A **DEBUG** PROFILE. Do not "fix" the CI job by
// folding them back into the `--release` run; they were RED from the day this
// file was created and the Epic-16 retrospective (2026-09-21) measured why.
//
// `NONCE_A`/`NONCE_B` below are pre-paired into each host's `[tcp] peer_pins`
// row because `PinnedFingerprint.boot_nonce` is REQUIRED with no serde default
// (`maos-a2a-tcp/src/config.rs:31-36`) and `verify_pinned_sync` returns
// `NotPinned` rather than TOFU-pinning on first contact
// (`maos-a2a-core/src/tofu.rs:457-466`) — the config IS the pre-pairing
// mechanism, so the harness cannot omit the nonce. The daemon honours
// `MAOS_TEST_BOOT_NONCE` only under `cfg!(debug_assertions)`
// (`crates/maos-bin/src/main.rs:1948`), which is deliberate and stays: a shipped
// release binary that accepts an env-supplied boot nonce has no NFR-Rel-6
// restart detection. Under `--release` both hosts mint random nonces
// (`main.rs:1956-1959`), the receiver fires `CODE_SPIRIT_RESTART_DETECTED`
// (`maos-a2a-core/src/router.rs:1372-1380`) and the sender reports
// `CrossHostPinMismatch`. Measured: release 9/11 with these three red; debug
// 3/3 green. `.github/workflows/discipline.yml` runs the suite in both profiles
// and asserts 8 + 3 == 11 so the split cannot silently drop a vector.
//
// NOT covered here, and deliberately so: the designed `re-pin` recovery. Since
// Story 19-7 `TofuPinStore::await_repin_consent` has a production caller — the
// `cohort-a2a-daemon` door behind `maosctl a2a repin approve` — and its
// two-process vector lives in `crates/maos-bin/tests/f4_pairing.rs`.

#[cfg(feature = "network")]
mod cohort {
    use super::*;

    use ed25519_dalek::SigningKey;
    use maos_a2a_core::PeerCertFingerprint;
    use maos_bin::delegation;
    use orchestrator::DELEGATION_CONSENT_INTENT;

    const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
    const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
    const NONCE_A: u64 = 0x16_3_A;
    const NONCE_B: u64 = 0x16_3_B;

    struct Fixture {
        dir: PathBuf,
        manifest: PathBuf,
        authority: SigningKey,
        a_cert: PathBuf,
        a_key: PathBuf,
        a_fingerprint: String,
        b_cert: PathBuf,
        b_key: PathBuf,
        b_fingerprint: String,
        worker_manifest: PathBuf,
        host_a_log: PathBuf,
        host_b_log: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Spawn ownership stays armed during startup and through the assertions.
    struct RunningDaemon(OwnedRoot);

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[0x16; 32])
    }

    fn authority_key_hex(key: &SigningKey) -> String {
        hex::encode(key.verifying_key().to_bytes())
    }

    fn mint_pems(dir: &Path) -> (PathBuf, PathBuf, String) {
        std::fs::create_dir_all(dir).expect("create identity directory");
        let key = rcgen::KeyPair::generate().expect("rcgen keypair");
        let params =
            rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("rcgen parameters");
        let cert = params
            .self_signed(&key)
            .expect("rcgen self-signed certificate");
        let fingerprint = PeerCertFingerprint::from_cert_der(cert.der().as_ref()).to_string();
        let cert_path = dir.join("own.cert.pem");
        let key_path = dir.join("own.key.pem");
        std::fs::write(&cert_path, cert.pem()).expect("write certificate pem");
        std::fs::write(&key_path, key.serialize_pem()).expect("write private key pem");
        (cert_path, key_path, fingerprint)
    }

    fn signed_manifest(key: &SigningKey, a_fingerprint: &str, b_fingerprint: &str) -> String {
        use maos_cohort::{
            CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ManifestSignature,
            TeamEntry, COHORT_SCHEMA_V4, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
        };
        use maos_domain::ports::registry::SpiritId;
        use maos_domain::region::Region;
        use maos_domain::team::TeamId;

        toml::to_string(
            &CohortManifest {
                schema_version: COHORT_SCHEMA_V4,
                cohort_id: "ac4-cohort-16-3".to_string(),
                version: 1,
                authority: CohortAuthority {
                    threshold: 1,
                    keys: vec![authority_key_hex(key)],
                },
                members: vec![
                    CohortMember {
                        host_id: "host-a".to_string(),
                        fingerprint: a_fingerprint.to_string(),
                        roles: vec!["worker".to_string()],
                        team: Some(TeamId::new("team-a").expect("valid team id")),
                    },
                    CohortMember {
                        host_id: "host-b".to_string(),
                        fingerprint: b_fingerprint.to_string(),
                        roles: vec!["worker".to_string()],
                        team: Some(TeamId::new("team-b").expect("valid team id")),
                    },
                ],
                consent: ConsentMatrix::default(),
                reserved_intents: vec![
                    RESERVED_INTENT_REISSUE.to_string(),
                    RESERVED_INTENT_HALT_RECEIPT.to_string(),
                ],
                t_stale_secs: 120,
                teams: Some(vec![
                    TeamEntry {
                        team_id: TeamId::new("team-a").expect("valid team id"),
                        region: Region::canonicalize("region-a").expect("valid region"),
                        datname: "maos_team_a".to_string(),
                        members: vec![SpiritId::from("spirit-a")],
                    },
                    TeamEntry {
                        team_id: TeamId::new("team-b").expect("valid team id"),
                        region: Region::canonicalize("region-b").expect("valid region"),
                        datname: "maos_team_b".to_string(),
                        members: vec![SpiritId::from("spirit-b")],
                    },
                ]),
                signature: ManifestSignature { sig: String::new() },
                cross_team_consent: Vec::new(),
            }
            .signed_with(key),
        )
        .expect("serialize signed cohort manifest")
    }

    fn fixture(tag: &str, mode: &str) -> Fixture {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("maos-ac4-cohort-{tag}-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create fixture directory");
        let (a_cert, a_key, a_fingerprint) = mint_pems(&dir.join("host-a"));
        let (b_cert, b_key, b_fingerprint) = mint_pems(&dir.join("host-b"));
        let authority = signing_key();
        let manifest = dir.join("cohort.toml");
        std::fs::write(
            &manifest,
            signed_manifest(&authority, &a_fingerprint, &b_fingerprint),
        )
        .expect("write cohort manifest");
        let worker_manifest = super::write_worker_manifest(&dir, mode);
        Fixture {
            host_a_log: dir.join("host-a.transparency.sqlite"),
            host_b_log: dir.join("host-b.transparency.sqlite"),
            dir,
            manifest,
            authority,
            a_cert,
            a_key,
            a_fingerprint,
            b_cert,
            b_key,
            b_fingerprint,
            worker_manifest,
        }
    }

    fn fingerprint_toml(fingerprint: &str) -> String {
        let hex = fingerprint
            .strip_prefix("sha256:")
            .expect("minted fingerprint has sha256 prefix");
        format!("{{ algo = 'sha256', hex = '{hex}' }}")
    }

    fn write_daemon_config(
        fixture: &Fixture,
        host: &str,
        cert: &Path,
        private_key: &Path,
        peer_id: &str,
        peer_fingerprint: &str,
        peer_nonce: u64,
        peer_endpoint: &str,
        include_worker_manifest: bool,
    ) -> PathBuf {
        let worker_manifest = include_worker_manifest
            .then(|| {
                format!(
                    "worker_manifest = '{}'\n",
                    fixture.worker_manifest.display()
                )
            })
            .unwrap_or_default();
        let peer_fingerprint = fingerprint_toml(peer_fingerprint);
        let config = format!(
            "manifest_path = '{manifest}'\nauthority_keys = ['{authority}']\n\
             local_host = '{host}'\ncontrol_spirit = 'orchestrator'\n{worker_manifest}\n\
             [[peers]]\npeer_id = '{peer_id}'\nendpoint = '{peer_endpoint}'\n\
             cert_fingerprint = {peer_fingerprint}\nsend_allowlist = ['{intent}']\n\
             accept_allowlist = ['{intent}']\n\n[tcp]\nlisten_addr = '127.0.0.1:0'\n\
             own_cert_chain = '{cert}'\nown_private_key = '{private_key}'\n\
             peer_pins = [{{ peer_id = '{peer_id}', fingerprint = {peer_fingerprint}, boot_nonce = {peer_nonce} }}]\n\n\
             [digest_summary]\nframes = 0\nhalts = 0\nconflicts = 0\n",
            manifest = fixture.manifest.display(),
            authority = authority_key_hex(&fixture.authority),
            intent = DELEGATION_CONSENT_INTENT,
            cert = cert.display(),
            private_key = private_key.display(),
        );
        let path = fixture.dir.join(format!("{host}.daemon.toml"));
        std::fs::write(&path, config).expect("write daemon config");
        path
    }

    fn host_a_config(fixture: &Fixture, port_b: u16) -> PathBuf {
        write_daemon_config(
            fixture,
            "host-a",
            &fixture.a_cert,
            &fixture.a_key,
            delegation::TO_HOST,
            &fixture.b_fingerprint,
            NONCE_B,
            &format!("tls://127.0.0.1:{port_b}"),
            false,
        )
    }

    fn host_b_config(fixture: &Fixture) -> PathBuf {
        write_daemon_config(
            fixture,
            "host-b",
            &fixture.b_cert,
            &fixture.b_key,
            delegation::FROM_HOST,
            &fixture.a_fingerprint,
            NONCE_A,
            "tls://127.0.0.1:1",
            true,
        )
    }

    fn daemon_command(config: &Path, audit_db: &Path, boot_nonce: u64) -> Command {
        let mut command = Command::new(maos());
        command
            .current_dir(workspace_root())
            .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
            .env("MAOS_COHORT_DAEMON_CONFIG", config)
            .env("MAOS_AUDIT_DB", audit_db)
            .env("MAOS_OLLAMA_URL", "skip")
            .env("MAOS_TEST_BOOT_NONCE", boot_nonce.to_string())
            // Both roots share one log (MAOS_AUDIT_DB) by design, so only
            // "HOME" moves — no developer control.json endpoint for either.
            .env(
                "HOME",
                t3_substrate::isolated_runtime_home(doorless_home::doorless_home()),
            )
            .env("PATH", child_process_path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn run_host_a(config: &Path, audit_db: &Path) -> std::process::Output {
        Command::new(maos())
            .args([
                "run",
                "spirits/topologies/j1-founder-loop-crosshost.toml",
                "--once",
            ])
            .current_dir(workspace_root())
            .env("MAOS_COHORT_DAEMON_CONFIG", config)
            .env("MAOS_AUDIT_DB", audit_db)
            .env("MAOS_OLLAMA_URL", "skip")
            .env("MAOS_TEST_BOOT_NONCE", NONCE_A.to_string())
            .env(
                "HOME",
                t3_substrate::isolated_runtime_home(doorless_home::doorless_home()),
            )
            // Required on every cross-host arm: the goal is operator-supplied
            // and the fixture asserts the MECHANISM, not the goal's content.
            .env(
                "MAOS_DELEGATED_GOAL",
                "hermetic AC4 mechanism probe — the teardown is the assertion, not this goal",
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn host A")
            .wait_with_output()
            .expect("wait for host A")
    }

    /// Spawn the daemon and scrape stderr until the listening line, keeping
    /// both pipes draining afterwards (2b's shape).
    fn boot_until_listening(
        mut command: Command,
    ) -> (RunningDaemon, u16, Arc<Mutex<String>>, Arc<Mutex<String>>) {
        let mut child = OwnedRoot::spawn(&mut command);
        let stdout_text = child.capture_stdout();
        let stderr = child.stderr.take().expect("daemon stderr piped");
        let stderr_text = Arc::new(Mutex::new(String::new()));
        {
            let capture = Arc::clone(&stderr_text);
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    let mut sink = capture.lock().expect("stderr capture lock poisoned");
                    sink.push_str(&line);
                    sink.push('\n');
                }
            });
        }
        let deadline = Instant::now() + LISTEN_TIMEOUT;
        loop {
            let seen = stderr_text
                .lock()
                .expect("stderr capture lock poisoned")
                .clone();
            // Parse the LISTENING LINE only. `seen` is the whole accumulated
            // stderr, so without `.lines().next()` the port is read after the
            // LAST `:` of everything that followed — e.g. the next line's
            // "connect 127.0.0.1:1: Connection refused (os error 111)" — and
            // the parse fails for good once that line lands before this 100 ms
            // poll. Measured on CI 6af9423a: the daemon had printed
            // "listening on 127.0.0.1:37595" and the test still timed out at
            // 90 s. The sibling harnesses parse per line and never had this.
            if let Some(port) = seen
                .split_once(LISTENING_MARKER)
                .and_then(|(_, rest)| rest.lines().next())
                .and_then(|address| address.trim().rsplit(':').next())
                .and_then(|port| port.parse::<u16>().ok())
            {
                break (RunningDaemon(child), port, stdout_text, stderr_text);
            }
            if let Some(status) = child.try_wait().expect("poll daemon") {
                panic!("daemon exited before listening ({status}); stderr:\n{seen}");
            }
            assert!(
                Instant::now() < deadline,
                "daemon never printed the listening line within {LISTEN_TIMEOUT:?}; \
                 stderr:\n{seen}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn daemon_exit(
        daemon: &mut RunningDaemon,
        bound: Duration,
        stderr_text: &Arc<Mutex<String>>,
    ) -> (std::process::ExitStatus, Duration) {
        let started = Instant::now();
        loop {
            match daemon.0.try_wait().expect("poll daemon") {
                Some(status) => return (status, started.elapsed()),
                None if started.elapsed() >= bound => {
                    let _ = daemon.0.kill();
                    let _ = daemon.0.wait();
                    panic!(
                        "the daemon did not exit within {bound:?}; stderr:\n{}",
                        stderr_text.lock().expect("stderr capture lock poisoned")
                    );
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }

    /// AC4 (5) — SIGTERM with a live host-B `hang` Worker.
    ///
    /// Refuses: a cohort daemon that never tears its Workers down (child
    /// alive after exit), unload/receipt rows missing from the daemon's OWN
    /// log, and — because this story adds the daemon's writer await — a
    /// `drain timed out` or a missing `SpiritUnload` revoke row.
    #[test]
    fn ac4_v5_cohort_sigterm_with_live_worker_unloads_and_drains() {
        if !super::t3_substrate::require_t3("cohort root shutdown Worker") {
            return;
        }
        let fixture = fixture("v5", "hang");
        let b_config = host_b_config(&fixture);
        let (mut daemon, _port_b, b_stdout, b_stderr) =
            boot_until_listening(daemon_command(&b_config, &fixture.host_b_log, NONCE_B));
        assert!(
            b_stderr.lock().expect("stderr lock").contains(ARMED_MARKER),
            "the daemon's site-2 listener must be armed before any Worker runs"
        );

        let daemon_pid = daemon.0.id();
        let output = run_host_a(&host_a_config(&fixture, _port_b), &fixture.host_a_log);
        let host_a_stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "host A failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            host_a_stdout.contains("topology_worker_delegated_offhost")
                && host_a_stdout.contains("\"local_worker_spawned\":false"),
            "the hang Worker must be host B's, not a local spawn: {host_a_stdout}"
        );

        // Read the published CID, then inspect it and verify its cgroup.
        let worker_child = wait_for_fixture_child_with(
            || b_stdout.lock().expect("stdout lock").clone(),
            "hang",
            LISTEN_TIMEOUT,
            || b_stderr.lock().expect("stderr lock").clone(),
        );

        daemon_sigterm(daemon_pid);
        let (status, elapsed) = daemon_exit(&mut daemon, Duration::from_secs(30), &b_stderr);
        let stderr = b_stderr.lock().expect("stderr lock").clone();
        assert!(
            status.code().is_some(),
            "the daemon must exit THROUGH ITS TEARDOWN, not by signal: {stderr}"
        );
        wait_worker_stopped(
            &worker_child,
            Duration::from_secs(5),
            "host B's hang Worker",
        );

        let rows = tl_rows(&fixture.host_b_log);
        let worker_pid = expect_load_pid(&rows, "worker");
        assert!(
            rows.iter()
                .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid),
            "host B must have unloaded the Worker's SCB synchronously; unload rows:\n{:?}",
            rows_with_intent(&rows, "lifecycle.unload")
        );
        let receipts = receipt_ids(&rows, "planned_unload", worker_pid);
        assert!(
            !receipts.is_empty(),
            "a PlannedUnload receipt must exist at the Worker's pid {worker_pid}"
        );
        assert!(
            rows.iter()
                .any(|row| row.intent == "cap.revoke" && row.spirit_pid == worker_pid),
            "the SpiritUnload revoke row must reach the daemon's log — the writer \
             await this story added must have completed"
        );
        assert!(
            !stderr.contains("drain timed out"),
            "the daemon's audit writer must drain: {stderr}"
        );
        println!(
            "v5: cohort daemon SIGTERM with a live host-B worker, exit {:?} in {elapsed:?}",
            status.code()
        );
    }

    fn daemon_sigterm(pid: u32) {
        let sent = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .expect("run kill(1)");
        assert!(sent.success(), "kill -TERM {pid} failed");
    }

    /// AC4 (5b) — SIGTERM during daemon startup.
    ///
    /// The site-2 listener installs its streams BEFORE the daemon's own
    /// `select!` exists, so a signal in that window must drive the daemon
    /// through its teardown — an exit WITH A CODE, never death by signal 15,
    /// and never the daemon's own `received SIGTERM` line (that line would
    /// mean the signal landed too late and the run proves nothing; it is
    /// retried, not counted).
    ///
    /// PROVEN RED by removing the `root_shutdown` arm from the daemon's
    /// `select!`: without it the daemon serves forever and the run times out.
    #[test]
    fn ac4_v5b_sigterm_during_daemon_startup_exits_through_its_teardown() {
        const MAX_ATTEMPTS: usize = 5;
        if !super::t3_substrate::require_t3("cohort root startup shutdown Worker") {
            return;
        }
        let mut counted = 0usize;
        for attempt in 1..=MAX_ATTEMPTS {
            let fixture = fixture("v5b", "hang");
            let b_config = host_b_config(&fixture);
            let mut command = daemon_command(&b_config, &fixture.host_b_log, NONCE_B);
            let mut child = command.spawn().expect("spawn cohort daemon");
            let pid = child.id();
            let stderr = child.stderr.take().expect("daemon stderr piped");
            let stdout = child.stdout.take().expect("daemon stdout piped");
            let stderr_text = Arc::new(Mutex::new(String::new()));
            {
                let capture = Arc::clone(&stderr_text);
                std::thread::spawn(move || {
                    for line in std::io::BufReader::new(stdout).lines() {
                        let Ok(line) = line else { break };
                        let mut sink = capture.lock().expect("stdout capture lock poisoned");
                        sink.push_str(&line);
                        sink.push('\n');
                    }
                });
            }
            // Signal the instant the listener announces itself — lowest
            // latency straight out of the reader loop.
            let signalled = Arc::new(Mutex::new(false));
            {
                let capture = Arc::clone(&stderr_text);
                let signalled = Arc::clone(&signalled);
                std::thread::spawn(move || {
                    for line in std::io::BufReader::new(stderr).lines() {
                        let Ok(line) = line else { break };
                        let mut seen = capture.lock().expect("stderr capture lock poisoned");
                        seen.push_str(&line);
                        seen.push('\n');
                        let armed = seen.contains(ARMED_MARKER);
                        drop(seen);
                        if armed {
                            let mut done = signalled.lock().expect("signal flag poisoned");
                            if !*done {
                                *done = true;
                                drop(done);
                                let _ = Command::new("kill")
                                    .args(["-TERM", &pid.to_string()])
                                    .status();
                            }
                        }
                    }
                });
            }

            let started = Instant::now();
            let status = loop {
                match child.try_wait().expect("poll daemon") {
                    Some(status) => break status,
                    None => {
                        assert!(
                            started.elapsed() < Duration::from_secs(45),
                            "attempt {attempt}: the daemon served forever after a \
                             startup SIGTERM (the removed-arm shape); stderr:\n{}",
                            stderr_text.lock().expect("stderr capture lock poisoned")
                        );
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            };
            let stderr = stderr_text
                .lock()
                .expect("stderr capture lock poisoned")
                .clone();
            let late = stderr.contains("maos: cohort-a2a-daemon received SIGTERM");
            if late {
                // The signal landed after the daemon's own streams existed —
                // this run proves nothing about the startup window; retry.
                println!("v5b attempt {attempt}: signal landed late (daemon's own stream took it); retrying");
                continue;
            }
            counted += 1;
            assert!(
                *signalled.lock().expect("signal flag poisoned"),
                "attempt {attempt}: the listener never armed — nothing was signalled; \
                 stderr:\n{stderr}"
            );
            assert!(
                status.code().is_some(),
                "attempt {attempt}: the daemon must exit THROUGH ITS TEARDOWN (a \
                 code), not by signal 15; status {status:?}; stderr:\n{stderr}"
            );
            println!(
                "v5b attempt {attempt}: counted run — daemon exited with code {:?} \
                 through its teardown",
                status.code()
            );
            break;
        }
        assert!(
            counted > 0,
            "no run counted in {MAX_ATTEMPTS} attempts — every SIGTERM landed \
             after the daemon's own streams existed, so the startup window was \
             never exercised"
        );
    }

    /// AC4 (5c) — the cohort daemon stops the CID-owned T3 Worker container,
    /// including a descendant that held its stdio pipe open.
    #[test]
    fn ac4_v5c_cohort_shutdown_stops_worker_container_descendants() {
        if !super::t3_substrate::require_t3("cohort root descendant Worker") {
            return;
        }
        let fixture = fixture("v5c", "hang-with-grandchild");
        let b_config = host_b_config(&fixture);
        let (mut daemon, _port_b, b_stdout, b_stderr) =
            boot_until_listening(daemon_command(&b_config, &fixture.host_b_log, NONCE_B));

        let daemon_pid = daemon.0.id();
        let output = run_host_a(&host_a_config(&fixture, _port_b), &fixture.host_a_log);
        assert!(
            output.status.success(),
            "host A failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let worker = wait_for_fixture_child_with(
            || b_stdout.lock().expect("stdout lock").clone(),
            "hang-with-grandchild",
            LISTEN_TIMEOUT,
            || b_stderr.lock().expect("stderr lock").clone(),
        );
        let _announced_guest_pid = wait_for_grandchild_pid(&fixture.host_b_log, OUTPUT_ROW_TIMEOUT);
        let grandchild = wait_for_host_grandchild(worker.payload_host_pid, OUTPUT_ROW_TIMEOUT);
        let guard = grandchild_guard(grandchild);

        daemon_sigterm(daemon_pid);
        let (status, elapsed) = daemon_exit(&mut daemon, Duration::from_secs(12), &b_stderr);
        let stderr = b_stderr.lock().expect("stderr lock").clone();
        // T0's exit-1 grace path was the descendant residual: the grandchild
        // held the stdio pipe past the grace. Stopping the CID-owned container
        // removes it, so the daemon takes the same teardown as v5.
        assert!(
            status.code().is_some(),
            "the daemon must exit THROUGH ITS TEARDOWN, not by signal: {stderr}"
        );
        assert!(
            elapsed < Duration::from_secs(12),
            "the root teardown budget is 5 s grace + 2 s slack; took {elapsed:?}"
        );
        wait_worker_stopped(
            &worker,
            Duration::from_secs(5),
            "host B's descendant-holding Worker",
        );
        assert!(
            !guard.still_ours(),
            "stopping the CID-owned host-B Worker container must remove its grandchild"
        );
        let rows = tl_rows(&fixture.host_b_log);
        let worker_pid = expect_load_pid(&rows, "worker");
        assert!(
            rows.iter()
                .any(|row| row.intent == "lifecycle.unload" && row.spirit_pid == worker_pid),
            "host B must unload the Worker SCB"
        );
        assert_eq!(
            receipt_ids(&rows, "planned_unload", worker_pid).len(),
            1,
            "host B must persist one PlannedUnload receipt"
        );
    }
}
