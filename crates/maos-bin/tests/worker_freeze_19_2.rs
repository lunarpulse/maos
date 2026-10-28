#![forbid(unsafe_code)]

//! Story 19-2 AC7.2/AC7.4 + AC8 leg 3 (D-19-2-14 A, D-19-2-16 A, D-19-2-19
//! A(2)) — FR51(a)(d) for a T3 Worker, through the production `maos` root
//! and `maosctl`, on the signed T3 substrate (`require_t3`: a named local
//! skip, never a green skip on a declared-capable runner).
//!
//! * 20 door pauses of a standalone `hang-chatty` Worker (one heartbeat line
//!   per second): each `maosctl pause worker` answers `frozen: true,
//!   fenced: true`, and no heartbeat is journaled after the freeze is
//!   acknowledged (a line the payload wrote just before the freeze may land
//!   within `ACK_SLACK`). Latency = POST → the later of the acknowledgement
//!   and the last heartbeat journaled after the POST; P99 ≤ 2 s. Every
//!   resume thaws the SAME container (state kept, FR51(b)), and a root
//!   SIGTERM still tears down a frozen Worker.
//! * 20 revokes of a Worker's `CliSubprocessSpawn` token (one root each, in
//!   `LANES` parallel lanes): `maosctl revoke-token` answers
//!   `worker_stopped: "worker"` and the container is gone within
//!   `TEARDOWN_BUDGET` (25 s). Latency = POST → container terminal.
//!
//! Measured figures are printed for the Dev Agent Record.
//!
//! Proven red (story §Proven-red controls): a freeze hook made a no-op
//! (`BindingCore::set_frozen` skipping `terminal.set_frozen`) keeps the
//! heartbeats flowing after the acknowledgement → the pause vector fails.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const PAUSES: usize = 20;
const REVOKES: usize = 20;
const LANES: usize = 4;
const HEARTBEAT: &str = "worker: fixture heartbeat";
/// FR51(a) — P99 bound for a pause (`functional-requirements.md:55`).
const P99_BOUND: Duration = Duration::from_secs(2);
/// `t3/child.rs` `TEARDOWN_BUDGET` — the revoke stop's confirmation bound.
const TEARDOWN_BUDGET: Duration = Duration::from_secs(25);
/// How long a frozen Worker must stay silent (> one heartbeat interval).
const QUIET: Duration = Duration::from_millis(1500);
/// A heartbeat written just before the freeze may be journaled just after
/// the acknowledgement (pipe read + TL insert); never a full interval.
const ACK_SLACK_NS: i64 = 250_000_000;
const STARTUP_BOUND: Duration = Duration::from_secs(90);
const EXIT_BOUND: Duration = Duration::from_secs(40);
/// `(timestamp_ns, spirit_pid, intent, capability_token column, payload)`.
type Row = (i64, i64, String, Option<Vec<u8>>, serde_json::Value);

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos() as i64
}

fn path_with_target_debug() -> std::ffi::OsString {
    let profile = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .and_then(Path::parent)
        .expect("test profile directory")
        .to_path_buf();
    let mut paths = vec![profile.clone(), profile.join("deps")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("test PATH entries are valid")
}

fn base(binary: &Path, home: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env_clear()
        .env("PATH", path_with_target_debug())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .current_dir(workspace_root());
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
    command
}

/// One standalone `hang-chatty` Worker root (`maos run <worker> --once`).
struct WorkerRoot {
    home: PathBuf,
    child: Option<Child>,
    stdout: Arc<Mutex<Vec<String>>>,
    stderr: Arc<Mutex<String>>,
}

impl WorkerRoot {
    fn boot(tag: &str) -> Self {
        let nanos = now_ns();
        let home = std::env::temp_dir().join(format!(
            "maos-19-2-freeze-{tag}-{nanos}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&home).expect("create data home");
        t3_substrate::isolated_runtime_home(&home);
        let maos = PathBuf::from(env!("CARGO_BIN_EXE_maos"));
        let init = base(&maos, &home).arg("init").output().expect("maos init");
        assert!(
            init.status.success(),
            "maos init: {}",
            String::from_utf8_lossy(&init.stderr)
        );
        let manifest = home.join("worker-hang-chatty.toml");
        std::fs::write(
            &manifest,
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang-chatty\"]\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n",
        )
        .expect("write the Worker manifest");
        let mut child = base(&maos, &home)
            .arg("run")
            .arg(&manifest)
            .arg("--once")
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run");
        let stdout = Arc::new(Mutex::new(Vec::new()));
        let pipe = child.stdout.take().expect("stdout piped");
        let sink = Arc::clone(&stdout);
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                let Ok(line) = line else { break };
                sink.lock().expect("stdout capture").push(line);
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let mut pipe = child.stderr.take().expect("stderr piped");
        let sink = Arc::clone(&stderr);
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                sink.lock()
                    .expect("stderr capture")
                    .push_str(&String::from_utf8_lossy(&chunk[..n]));
            }
        });
        let root = Self {
            home,
            child: Some(child),
            stdout,
            stderr,
        };
        root.wait_for("the first heartbeat", STARTUP_BOUND, |root| {
            !root.heartbeats().is_empty()
        });
        root
    }

    fn stderr(&self) -> String {
        self.stderr.lock().expect("stderr capture").clone()
    }

    fn wait_for(&self, what: &str, bound: Duration, mut ready: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + bound;
        while Instant::now() < deadline {
            if ready(self) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "{what} not seen within {bound:?}; stderr:\n{}",
            self.stderr()
        );
    }

    fn container_id(&self) -> String {
        self.stdout
            .lock()
            .expect("stdout capture")
            .iter()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|event| event["event"] == "cli_wrapper_loaded")
            .and_then(|event| event["container_id"].as_str().map(str::to_owned))
            .expect("cli_wrapper_loaded publishes the CID")
    }

    fn ctl(&self, args: &[&str]) -> Output {
        base(&fixtures::profile_binary("maosctl"), &self.home)
            .args(args)
            .output()
            .expect("run maosctl")
    }

    fn ctl_json(&self, args: &[&str]) -> serde_json::Value {
        let out = self.ctl(args);
        assert!(
            out.status.success(),
            "maosctl {args:?} must exit 0; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("maosctl prints a JSON body")
    }

    fn rows(&self, sql: &str) -> Vec<Row> {
        let db = self.home.join("audit").join("transparency.sqlite");
        if !db.exists() {
            return Vec::new();
        }
        let connection = rusqlite::Connection::open_with_flags(
            db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("open the transparency log read-only");
        let mut statement = connection.prepare(sql).expect("prepare TL read");
        statement
            .query_map([], |row| {
                let payload: Option<Vec<u8>> = row.get(4)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    payload
                        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                        .unwrap_or(serde_json::Value::Null),
                ))
            })
            .expect("query TL rows")
            .collect::<Result<_, _>>()
            .expect("read TL rows")
    }

    /// `(timestamp_ns, child_pid, spirit_pid)` of every journaled heartbeat.
    fn heartbeats(&self) -> Vec<(i64, i64, i64)> {
        self.rows(
            "SELECT timestamp_ns, spirit_pid, intent, capability_token, payload_redacted \
             FROM transparency_log WHERE intent = 'cli.subprocess.output' ORDER BY timestamp_ns, rowid",
        )
        .into_iter()
        .filter(|(_, _, _, _, payload)| payload["line"] == HEARTBEAT)
        .map(|(ts, pid, _, _, payload)| (ts, payload["child_pid"].as_i64().unwrap_or(-1), pid))
        .collect()
    }

    /// The Worker's `CliSubprocessSpawn` grant, as `maosctl revoke-token` takes it.
    fn spawn_token_hex(&self, worker_pid: i64) -> String {
        self.rows(
            "SELECT timestamp_ns, spirit_pid, intent, capability_token, payload_redacted \
             FROM transparency_log WHERE intent LIKE 'cap.issue%' ORDER BY rowid",
        )
        .into_iter()
        .find(|(_, pid, _, _, payload)| {
            *pid == worker_pid && payload.get("CliSubprocessSpawn").is_some()
        })
        .and_then(|(_, _, _, column, _)| column.filter(|column| column.len() >= 16))
        .map(|column| hex::encode(&column[..16]))
        .expect("the Worker's spawn grant is journaled at its pid")
    }

    fn wait_new_heartbeat(&self) {
        let seen = self.heartbeats().len();
        self.wait_for("a fresh heartbeat", Duration::from_secs(10), |root| {
            root.heartbeats().len() > seen
        });
    }

    fn wait_exit(&mut self) -> ExitStatus {
        let child = self.child.as_mut().expect("root still owned");
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().expect("poll the root") {
                return status;
            }
            if started.elapsed() >= EXIT_BOUND {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "the root did not exit within {EXIT_BOUND:?}; stderr:\n{}",
                    self.stderr()
                );
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn sigterm(&self) {
        let pid = self.child.as_ref().expect("root still owned").id();
        let sent = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .expect("run kill(1)");
        assert!(sent.success(), "kill -TERM {pid}");
    }
}

impl Drop for WorkerRoot {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// Whether the OCI runtime still knows the CID (`container exists` exits 1
/// once it is gone).
fn container_exists(container_id: &str) -> bool {
    let runtime = std::env::var("MAOS_T3_RUNTIME").expect("T3 runtime configured");
    let out = Command::new("timeout")
        .arg("8s")
        .arg(runtime)
        .args(["container", "exists", container_id])
        .output()
        .expect("run the OCI runtime");
    out.status.code() != Some(1)
}

fn wait_container_gone(container_id: &str, bound: Duration) -> Duration {
    let started = Instant::now();
    while container_exists(container_id) {
        assert!(
            started.elapsed() < bound,
            "container {container_id} still exists after {bound:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    started.elapsed()
}

fn p99(mut sample: Vec<Duration>) -> (Duration, Duration) {
    sample.sort_unstable();
    let rank = (sample.len() * 99).div_ceil(100).max(1);
    (sample[rank - 1], sample[sample.len() - 1])
}

// AC7.2 / AC7.4 — 20 door pauses freeze the Worker within the FR51(a) bound;
// resume thaws the same container; a frozen Worker is still torn down.
#[test]
fn twenty_door_pauses_freeze_a_chatty_worker_within_p99_two_seconds() {
    if !t3_substrate::require_t3("19-2 Worker freeze") {
        return;
    }
    let mut root = WorkerRoot::boot("pause");
    let container = root.container_id();

    let mut latencies = Vec::with_capacity(PAUSES);
    for cycle in 0..PAUSES {
        root.wait_new_heartbeat();
        let posted = now_ns();
        let paused = root.ctl_json(&["pause", "worker"]);
        let acked = now_ns();
        assert_eq!(
            paused["lifecycle_state"], "Paused",
            "cycle {cycle}: {paused}"
        );
        assert_eq!(
            paused["frozen"], true,
            "cycle {cycle}: the container froze: {paused}"
        );
        assert_eq!(
            paused["fenced"], true,
            "cycle {cycle}: the pid is fenced: {paused}"
        );

        std::thread::sleep(QUIET);
        let after: Vec<i64> = root
            .heartbeats()
            .into_iter()
            .map(|(ts, _, _)| ts)
            .filter(|ts| *ts > posted)
            .collect();
        let late: Vec<&i64> = after
            .iter()
            .filter(|ts| **ts > acked + ACK_SLACK_NS)
            .collect();
        assert!(
            late.is_empty(),
            "cycle {cycle}: {} heartbeat(s) journaled after the freeze was acknowledged — \
             the Worker was not frozen",
            late.len()
        );
        let last = after.last().copied().unwrap_or(acked).max(acked);
        latencies.push(Duration::from_nanos(
            u64::try_from(last - posted).unwrap_or(0),
        ));

        let resumed = root.ctl_json(&["resume", "worker"]);
        assert_eq!(
            resumed["lifecycle_state"], "Running",
            "cycle {cycle}: {resumed}"
        );
        assert_eq!(resumed["frozen"], false, "cycle {cycle}: thawed: {resumed}");
        assert_eq!(resumed["fenced"], false, "cycle {cycle}: {resumed}");
    }
    root.wait_new_heartbeat();

    // FR51(b): one Worker process across every freeze — thawed, never respawned.
    let children: std::collections::BTreeSet<i64> = root
        .heartbeats()
        .into_iter()
        .map(|(_, child, _)| child)
        .collect();
    assert_eq!(
        children.len(),
        1,
        "one child across 20 freezes: {children:?}"
    );

    let (p99, max) = p99(latencies);
    eprintln!("AC7 T3 Worker pause -> silent: n={PAUSES} p99={p99:?} max={max:?}");
    assert!(
        p99 <= P99_BOUND,
        "Worker pause P99 {p99:?} exceeds {P99_BOUND:?}"
    );

    // Root teardown still kills a frozen container.
    let paused = root.ctl_json(&["pause", "worker"]);
    assert_eq!(paused["frozen"], true, "{paused}");
    root.sigterm();
    let _ = root.wait_exit();
    wait_container_gone(&container, TEARDOWN_BUDGET);
}

/// One revoke run: boot, revoke the spawn grant, measure POST → terminal.
fn revoke_once(tag: &str) -> Duration {
    let mut root = WorkerRoot::boot(tag);
    let container = root.container_id();
    let worker_pid = root.heartbeats()[0].2;
    let token = root.spawn_token_hex(worker_pid);

    let posted = Instant::now();
    let body = root.ctl_json(&["revoke-token", &token]);
    assert_eq!(body["revoked"], true, "{tag}: {body}");
    assert_eq!(
        body["worker_stopped"], "worker",
        "{tag}: the binding was stopped: {body}"
    );
    let remaining = TEARDOWN_BUDGET.saturating_sub(posted.elapsed());
    wait_container_gone(&container, remaining);
    let latency = posted.elapsed();
    let _ = root.wait_exit();
    latency
}

// AC8 leg 3 — revoking a Worker's spawn token stops it within the teardown
// budget, 20 times.
#[test]
fn twenty_spawn_token_revokes_stop_the_worker_within_the_teardown_budget() {
    if !t3_substrate::require_t3("19-2 Worker revoke-stop") {
        return;
    }
    let lanes: Vec<_> = (0..LANES)
        .map(|lane| {
            std::thread::spawn(move || {
                (0..REVOKES / LANES)
                    .map(|run| revoke_once(&format!("revoke-{lane}-{run}")))
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let latencies: Vec<Duration> = lanes
        .into_iter()
        .flat_map(|lane| lane.join().expect("a revoke lane panicked"))
        .collect();
    assert_eq!(latencies.len(), REVOKES);
    let (p99, max) = p99(latencies);
    eprintln!("AC8 T3 revoke -> container terminal: n={REVOKES} p99={p99:?} max={max:?}");
    assert!(
        max <= TEARDOWN_BUDGET,
        "a revoke stop exceeded {TEARDOWN_BUDGET:?}"
    );
}
