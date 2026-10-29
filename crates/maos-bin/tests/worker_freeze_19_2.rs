#![forbid(unsafe_code)]

//! Story 19-2 AC7.2/AC7.4 + AC8 leg 3 (D-19-2-14 A, D-19-2-16 A, D-19-2-19
//! A(2)) — FR51(a)(d) for a T3 Worker, through the production `maos` root
//! for pause and an explicitly token-bearing in-process root for revoke.
//! Both use the real operator HTTP door and `maosctl` on the signed T3
//! substrate (`require_t3`: a named local skip, never a green CI skip).
//!
//! * 20 door pauses of a standalone `hang-chatty` Worker (one heartbeat line
//!   per second): each `maosctl pause worker` answers `frozen: true,
//!   fenced: true`, and no heartbeat is journaled after the freeze is
//!   acknowledged (a line the payload wrote just before the freeze may land
//!   within `ACK_SLACK_NS`) for the whole quiet window, which ends
//!   `QUIET_MARGIN_NS` after the first line an unfrozen payload would have
//!   journaled past that slack (≈ 1.4 s after the cycle's heartbeat, never a
//!   fixed sleep). Latency = POST → the later of
//!   the acknowledgement and the last heartbeat journaled after the POST;
//!   all 20 ≤ 2 s. At n=20 the nearest-rank P99 IS the max, so this is
//!   AC7.4's "P99 ≤ 2 s over 20 pauses" read strictly; the P99 / P99.9
//!   claim (n=1000) is `fr51_pause_fence_19_2.rs`'s. Every resume thaws the
//!   SAME container (state kept, FR51(b)), and a root SIGTERM still tears
//!   down a frozen Worker.
//! * 20 revokes of a Worker's `CliSubprocessSpawn` token — 20 because AC8.3
//!   and D-19-2-19 A(2) fix the count; one test root each, because the revoke
//!   stops its only Worker — in `LANES` parallel lanes. Each isolated child
//!   prepares the exact spawn scope at the actual bound pid, then runs the
//!   production T3 spawn/supervision path. Unlike an ordinary standalone root
//!   (which may use tokenless AC5 host-grant authority), it really mints a
//!   grant. `maosctl revoke-token` answers `worker_stopped: "worker"` and the
//!   container is gone within `TEARDOWN_BUDGET` (25 s), all 20 (again the max,
//!   not a P99). Latency = POST → container terminal.
//!
//! Measured figures are printed for the Dev Agent Record.
//!
//! Wall: nextest's default profile kills a test at 120 s and
//! `[[profile.default.overrides]]` may not set a slow-timeout
//! (`.config/nextest.toml`). The revoke lanes share a 55 s wall deadline,
//! including child startup and cleanup; no extra runtime-boot sleep is added.
//! Hosted timing for the corrected revoke root still requires a rerun.
//!
//! Planted red (story §Proven-red controls; T3-only): a freeze hook made a
//! no-op (`BindingCore::set_frozen` skipping `terminal.set_frozen`) keeps
//! the heartbeats flowing after the acknowledgement → the pause vector
//! fails. The plant is described here; its run is recorded in the story's
//! `19-2-fr20-enqueue-door-and-safe-point-evidence/plants.md`.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;
#[path = "support/worker_revoke_root.rs"]
mod worker_revoke_root;

const PAUSES: usize = 20;
/// AC8.3 / D-19-2-19 A(2): 20 runs, not a tunable.
const REVOKES: usize = 20;
/// 4 sequential boot→revoke runs per lane.
const LANES: usize = 5;
const REVOKE_CHILD: &str = "MAOS_19_2_REVOKE_CHILD";
const REVOKE_SAMPLE: &str = "AC8_SAMPLE_NS=";
const _: () = assert!(REVOKES % LANES == 0, "every lane runs the same count");
const HEARTBEAT: &str = "worker: fixture heartbeat";
/// FR51(a) — the pause bound (`functional-requirements.md:55`); at n=20 it
/// is asserted on the max.
const PAUSE_BOUND: Duration = Duration::from_secs(2);
/// `t3/child.rs` `TEARDOWN_BUDGET` — the revoke stop's confirmation bound.
const TEARDOWN_BUDGET: Duration = Duration::from_secs(25);
/// The `hang-chatty` payload's line period (`worker-cli-fixture.rs`: one
/// `println!` then `sleep(1 s)`).
const BEAT_NS: i64 = 1_000_000_000;
/// How long past the first line an unfrozen payload would journal the quiet
/// window still waits (pipe read + TL insert jitter on a loaded runner).
const QUIET_MARGIN_NS: i64 = 400_000_000;
/// A heartbeat written just before the freeze may be journaled just after
/// the acknowledgement (pipe read + TL insert); never a full interval.
const ACK_SLACK_NS: i64 = 250_000_000;
/// Ceilings, not sleeps: each stays under nextest's 120 s kill so a stuck
/// boot or exit reports its own stderr.
const STARTUP_BOUND: Duration = Duration::from_secs(45);
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

    /// The actual pid's spawn grant; the audit writer is asynchronous even
    /// though minting and binding precede spawn. Never search another pid or
    /// accept another token kind to make an absent grant appear.
    fn spawn_token_hex(&self, worker_pid: i64) -> String {
        let mut token = None;
        self.wait_for(
            "the Worker's spawn grant journaled at its pid",
            Duration::from_secs(5),
            |root| {
                token = root.rows(
                    "SELECT timestamp_ns, spirit_pid, intent, capability_token, payload_redacted \
                     FROM transparency_log WHERE intent LIKE 'cap.issue%' ORDER BY rowid",
                )
                .into_iter()
                .find(|(_, pid, _, _, payload)| {
                    *pid == worker_pid && payload.get("CliSubprocessSpawn").is_some()
                })
                .and_then(|(_, _, _, column, _)| column.filter(|column| column.len() >= 16))
                .map(|column| hex::encode(&column[..16]));
                token.is_some()
            },
        );
        token.expect("the Worker's spawn grant is journaled at its pid")
    }

    /// Waits for a heartbeat journaled after the call; returns the newest
    /// heartbeat's timestamp.
    fn wait_new_heartbeat(&self) -> i64 {
        let seen = self.heartbeats().len();
        let mut newest = None;
        self.wait_for("a fresh heartbeat", Duration::from_secs(10), |root| {
            newest = root
                .heartbeats()
                .get(seen..)
                .and_then(|fresh| fresh.last())
                .map(|(ts, _, _)| *ts);
            newest.is_some()
        });
        newest
            .unwrap_or_else(|| unreachable!("wait_for returns only once a fresh heartbeat is seen"))
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

/// `(median, max)`. At n=20 the nearest-rank P99 is the max, so the vectors
/// assert the max and never call it a P99.
fn median_max(mut sample: Vec<Duration>) -> (Duration, Duration) {
    sample.sort_unstable();
    (sample[sample.len() / 2], sample[sample.len() - 1])
}

fn sleep_until_ns(at: i64) {
    if let Ok(rest) = u64::try_from(at - now_ns()) {
        std::thread::sleep(Duration::from_nanos(rest));
    }
}

// AC7.2 / AC7.4 — 20 door pauses freeze the Worker within the FR51(a) bound;
// resume thaws the same container; a frozen Worker is still torn down.
// Expected wall ≈ 40–60 s: one boot (≈ 5–15 s) + 20 cycles of ≈ 1.6–2.1 s
// (pause ctl, the quiet window ≈ 1.4 s past the cycle's heartbeat, resume
// ctl; a thawed payload's expired sleep heartbeats at once) + teardown.
#[test]
fn twenty_door_pauses_freeze_a_chatty_worker_each_within_two_seconds() {
    if !t3_substrate::require_t3("19-2 Worker freeze") {
        return;
    }
    let mut root = WorkerRoot::boot("pause");
    let container = root.container_id();

    let mut latencies = Vec::with_capacity(PAUSES);
    for cycle in 0..PAUSES {
        let beat = root.wait_new_heartbeat();
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

        // The shortest window that proves the freeze: unfrozen, the payload
        // journals a line every `BEAT_NS` after `beat`; wait out the first
        // such line due after `acked + ACK_SLACK_NS`, plus the margin.
        let beats_due = (acked + ACK_SLACK_NS - beat).div_euclid(BEAT_NS) + 1;
        sleep_until_ns(beat + beats_due * BEAT_NS + QUIET_MARGIN_NS);
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

    let (median, max) = median_max(latencies);
    eprintln!("AC7 T3 Worker pause -> silent: n={PAUSES} median={median:?} max={max:?}");
    assert!(
        max <= PAUSE_BOUND,
        "all {PAUSES} ≤ 2 s (P99 at n={PAUSES} = max): the slowest Worker pause took {max:?}"
    );

    // Root teardown still kills a frozen container.
    let paused = root.ctl_json(&["pause", "worker"]);
    assert_eq!(paused["frozen"], true, "{paused}");
    root.sigterm();
    let _ = root.wait_exit();
    wait_container_gone(&container, TEARDOWN_BUDGET);
}

/// Isolate HOME/workspaces and pid allocation without mutating process-wide
/// environment in the five lanes. The child composes real kernel adapters and
/// the real door, not a second production-binary policy convention.
fn revoke_once(tag: &str, deadline: Instant) -> Duration {
    let home = tempfile::tempdir().expect("revoke child home");
    t3_substrate::isolated_runtime_home(home.path());
    let remaining = deadline.saturating_duration_since(Instant::now());
    assert!(!remaining.is_zero(), "{tag}: revoke lanes exceeded 55 s");
    let output = base(Path::new("timeout"), home.path())
        .arg(format!("{}s", remaining.as_secs_f64()))
        .arg(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "twenty_spawn_token_revokes_stop_the_worker_within_the_teardown_budget",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(REVOKE_CHILD, tag)
        .output()
        .expect("run token-bearing revoke child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{tag}: revoke child failed (55 s lane deadline): {}; stdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("test result: ok. 1 passed;"),
        "{tag}: {stdout}"
    );
    let samples: Vec<_> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(REVOKE_SAMPLE))
        .collect();
    assert_eq!(
        samples.len(),
        1,
        "{tag}: exactly one measured revoke: {stdout}"
    );
    Duration::from_nanos(samples[0].parse().expect("revoke sample nanoseconds"))
}

#[test]
fn a_prepared_spawn_grant_is_journaled_at_the_bound_worker_pid() {
    worker_revoke_root::assert_mint_journal();
}

// AC8 leg 3 — 20 real spawn grants, real door revokes and T3 terminal facts.
// Five lanes of four isolated, in-process roots; all share a 55 s wall bound.
#[test]
fn twenty_spawn_token_revokes_stop_the_worker_within_the_teardown_budget() {
    if !t3_substrate::require_t3("19-2 Worker revoke-stop") {
        return;
    }
    if let Ok(tag) = std::env::var(REVOKE_CHILD) {
        let home = PathBuf::from(std::env::var_os("MAOS_HOME").expect("isolated child home"));
        let latency = worker_revoke_root::revoke_once(home, &tag);
        println!("{REVOKE_SAMPLE}{}", latency.as_nanos());
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(55);
    let lanes: Vec<_> = (0..LANES)
        .map(|lane| {
            std::thread::spawn(move || {
                (0..REVOKES / LANES)
                    .map(|run| revoke_once(&format!("revoke-{lane}-{run}"), deadline))
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let latencies: Vec<Duration> = lanes
        .into_iter()
        .flat_map(|lane| lane.join().expect("a revoke lane panicked"))
        .collect();
    assert_eq!(latencies.len(), REVOKES);
    let (median, max) = median_max(latencies);
    eprintln!("AC8 T3 revoke -> container terminal: n={REVOKES} median={median:?} max={max:?}");
    assert!(
        max <= TEARDOWN_BUDGET,
        "all {REVOKES} ≤ 25 s (P99 at n={REVOKES} = max): the slowest revoke stop took {max:?}"
    );
}
