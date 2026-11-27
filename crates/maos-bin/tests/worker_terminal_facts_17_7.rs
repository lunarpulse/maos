#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 17-7's terminal-facts regression suite. Runtime-backed cases are
//! deliberately capability-gated: a declared CI runner must have its signed
//! rootless T3 substrate, while local development clearly names unavailable
//! runtime scenes. Stub-runtime cases always run and exercise CID teardown.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead as _, Read, Write as _};
use std::os::unix::{
    fs::FileExt as _,
    process::{CommandExt as _, ExitStatusExt as _},
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::process::{self, Pid, Signal, WaitId, WaitIdOptions};

use maos_domain::sandbox::{parse_container_id, ContainerRuntimeKind};
use maos_kernel_core::security::sandbox::t3::child::{
    SandboxedContainerChild, KILL_BUDGET, REMOVE_BUDGET, TEARDOWN_BUDGET,
};
use maos_kernel_core::security::sandbox::t3::spawn::read_receipt;

#[path = "../../../tests/harness/layout.rs"]
mod layout;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const CID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn init_clock() {
    // All in-process Worlds share the process-global base, including under libtest.
    static CLOCK: std::sync::Once = std::sync::Once::new();
    CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
}

/// `require_t3` prints the one `SKIP:` line when the substrate is absent.
fn scene(name: &str) -> bool {
    t3_substrate::require_t3(name)
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "maos-terminal-facts-17-7-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir(&path).expect("create unique test directory");
    path
}

const CAPTURE_LIMIT: usize = 64 * 1024;

// File-backed stderr (and ordinary stdout) remains readable even when an OCI
// descendant outlives its root. No EOF wait, reader join, or raw output dump.
fn capture_file() -> (File, Stdio) {
    let file = tempfile::tempfile().expect("create private subprocess capture");
    let writer = file.try_clone().expect("clone subprocess capture writer");
    (file, Stdio::from(writer))
}

fn capture_text(file: &File) -> String {
    let len = file.metadata().map_or(0, |metadata| metadata.len());
    let mut bytes = vec![0; len.min(CAPTURE_LIMIT as u64) as usize];
    let n = file.read_at(&mut bytes, 0).unwrap_or_default();
    bytes.truncate(n);
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

// Emit only fixed labels, never arbitrary error text, environment, or output.
fn diagnostic_tags(text: &str) -> String {
    const TAGS: &[(&str, &str)] = &[
        ("create client failed", "create-client-failed"),
        ("t3 creation ownership unknown", "create-ownership-unknown"),
        ("t3 launch refused at create", "create-refused"),
        ("t3 launch refused at init", "init-refused"),
        ("t3 launch refused at start", "start-refused"),
        ("init failed", "init-failed"),
        ("start client failed", "start-client-failed"),
        ("sandbox", "sandbox"),
        ("scope", "scope"),
        ("permission denied", "permission-denied"),
        ("operation not permitted", "operation-not-permitted"),
        ("no such file", "missing-file"),
        ("command not found", "missing-command"),
        ("workspace", "workspace"),
        ("runtime refused", "runtime-refused"),
        ("timed out", "timed-out"),
        ("deadline", "deadline"),
        ("exited (", "client-exit"),
        ("residue ", "residue"),
        ("worker unload failed", "unload-failed"),
        ("panicked at", "panic"),
    ];
    let lower = text.to_ascii_lowercase();
    let tags: Vec<_> = TAGS
        .iter()
        .filter_map(|&(needle, tag)| lower.contains(needle).then_some(tag))
        .collect();
    format!("{} bytes sampled, tags={tags:?}", text.len())
}

fn custody_roots() -> impl Iterator<Item = PathBuf> {
    let fallback =
        std::env::temp_dir().join(format!("maos-worker-state-{}", process::geteuid().as_raw()));
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(|dir| PathBuf::from(dir).join("maos-worker-state"));
    std::iter::once(fallback).chain(runtime)
}

fn custody_process_dirs() -> impl Iterator<Item = PathBuf> {
    custody_roots()
        .filter_map(|root| std::fs::read_dir(root).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
}

fn custody_paths(pid: u32, deadline: Instant) -> io::Result<HashSet<PathBuf>> {
    let prefix = format!("process-{pid}-");
    let mut paths = HashSet::new();
    for root in custody_roots() {
        check_cleanup_time(deadline)?;
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries {
            check_cleanup_time(deadline)?;
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix))
            {
                paths.insert(entry.path());
            }
        }
    }
    Ok(paths)
}

fn check_cleanup_time(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Cleanup {
    status: Option<ExitStatus>,
    removed: usize,
    failed: usize,
    not_attempted: usize,
    custody_unreadable: bool,
    custody_incomplete: bool,
    reaper_started: bool,
}

const CID_CLEANUP_BUDGET: Duration = Duration::from_secs(8);
const REAP_BUDGET: Duration = Duration::from_secs(5);

struct SceneChild {
    child: Option<Child>,
    reaped: Option<ExitStatus>,
    custody_before: Option<HashSet<PathBuf>>,
    cid: Option<String>,
}

impl SceneChild {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        Ok(Self {
            child: Some(command.process_group(0).spawn()?),
            reaped: None,
            custody_before: None,
            cid: None,
        })
    }

    fn child(&mut self) -> &mut Child {
        self.child.as_mut().expect("scene child ownership")
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("scene child ownership").id()
    }

    fn worker_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new("/bin/sh");
        // The PID is reserved while the wrapper waits and stays the same
        // across exec. No Worker can create custody before the snapshot.
        command
            .args([
                "-c",
                "IFS= read -r release || exit 125; exec \"$@\"",
                "worker-scene",
            ])
            .arg(program);
        command
    }

    fn spawn_worker(command: &mut Command) -> io::Result<Self> {
        let mut scene = Self::spawn(command.stdin(Stdio::piped()))?;
        scene.custody_before = Some(custody_paths(scene.pid(), Instant::now() + REAP_BUDGET)?);
        scene
            .child()
            .stdin
            .take()
            .expect("Worker launch barrier")
            .write_all(b"\n")?;
        Ok(scene)
    }

    fn poll(&self) -> io::Result<Option<ExitStatus>> {
        if self.reaped.is_some() {
            return Ok(self.reaped);
        }
        let pid = Pid::from_raw(self.pid() as i32).expect("valid child PID");
        let flags = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
        let result = loop {
            match process::waitid(WaitId::Pid(pid), flags) {
                Err(rustix::io::Errno::INTR) => continue,
                result => break result?,
            }
        };
        Ok(result.map(|status| {
            if let Some(code) = status.exit_status() {
                ExitStatus::from_raw(code << 8)
            } else {
                let signal = status.terminating_signal().expect("waitid exit event");
                ExitStatus::from_raw(signal | if status.dumped() { 0x80 } else { 0 })
            }
        }))
    }

    fn wait_until(&self, deadline: Instant) -> io::Result<Option<ExitStatus>> {
        loop {
            let status = self.poll()?;
            let left = deadline.saturating_duration_since(Instant::now());
            if status.is_some() || left.is_zero() {
                return Ok(status);
            }
            std::thread::sleep(left.min(Duration::from_millis(20)));
        }
    }

    fn reap(&mut self) -> io::Result<Option<ExitStatus>> {
        self.reaped = self.child().try_wait()?;
        Ok(self.reaped)
    }

    fn signal_group(&self) {
        // An unreaped direct child reserves its PID even after exit. If the
        // wait reports ECHILD, we no longer own the identity; never signal it.
        if self.reaped.is_none() && self.poll().is_ok() {
            let pid = Pid::from_raw(self.pid() as i32).expect("valid process group ID");
            let _ = process::kill_process_group(pid, Signal::KILL);
        }
    }

    fn finish_cleanup(&mut self) -> (Option<ExitStatus>, bool) {
        let mut child = self.child.take().expect("scene child ownership");
        if let Ok(Some(status)) = child.try_wait() {
            return (Some(status), false);
        }
        let reaper_started = std::thread::Builder::new()
            .spawn(move || child.wait())
            .is_ok();
        (None, reaper_started)
    }

    fn cleanup_failure(&mut self) -> Cleanup {
        let reap_by = Instant::now() + REAP_BUDGET;
        let deadline = reap_by + CID_CLEANUP_BUDGET;
        self.signal_group();
        let observed = self.wait_until(reap_by);
        let mut result = Cleanup::default();
        let mut cids: HashSet<String> = self.cid.take().into_iter().collect();
        if let Some(before) = &self.custody_before {
            // Read owned receipts before reaping or yielding PID ownership.
            match (self.reaped, observed) {
                (None, Ok(_)) => match custody_paths(self.pid(), deadline) {
                    Ok(paths) => {
                        'paths: for path in paths.difference(before) {
                            if check_cleanup_time(deadline).is_err() {
                                result.custody_incomplete = true;
                                break;
                            }
                            let entries = std::fs::symlink_metadata(path).and_then(|metadata| {
                                if !metadata.is_dir() {
                                    return Err(io::ErrorKind::InvalidData.into());
                                }
                                std::fs::read_dir(path)
                            });
                            let Ok(entries) = entries else {
                                result.custody_unreadable = true;
                                continue;
                            };
                            for entry in entries {
                                if check_cleanup_time(deadline).is_err() {
                                    result.custody_incomplete = true;
                                    break 'paths;
                                }
                                let cid = entry.and_then(custody_cid);
                                match cid {
                                    Ok(Some(cid)) => {
                                        cids.insert(cid);
                                    }
                                    Ok(None) => {}
                                    Err(_) => result.custody_unreadable = true,
                                }
                            }
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::TimedOut => {
                        result.custody_incomplete = true;
                    }
                    Err(_) => result.custody_unreadable = true,
                },
                _ => result.custody_unreadable = true,
            }
        }
        (result.status, result.reaper_started) = self.finish_cleanup();
        for cid in cids {
            match remove_scene_cid_by(&cid, deadline) {
                Some(true) => result.removed += 1,
                Some(false) => result.failed += 1,
                None => result.not_attempted += 1,
            }
        }
        result
    }
}

impl Drop for SceneChild {
    fn drop(&mut self) {
        if self.child.is_some() && self.reaped.is_none() {
            let _ = self.cleanup_failure();
        }
    }
}

fn custody_cid(entry: std::fs::DirEntry) -> io::Result<Option<String>> {
    if !entry.file_type()?.is_dir() {
        return Ok(None);
    }
    let path = entry.path().join("cid");
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    read_receipt(&path, 65)
        .and_then(|bytes| parse_container_id(&bytes))
        .map(Some)
        .ok_or_else(|| io::ErrorKind::InvalidData.into())
}

fn valid_cid(cid: &str) -> bool {
    cid.len() == 64
        && cid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

struct CidReceipt {
    cid: String,
    client_pid: Option<u32>,
}

fn cid_receipt(line: &str) -> Option<CidReceipt> {
    let event: serde_json::Value = serde_json::from_str(line).ok()?;
    let cid = event["container_id"].as_str()?;
    (event["event"] == "cli_wrapper_loaded" && valid_cid(cid)).then(|| CidReceipt {
        cid: cid.to_owned(),
        client_pid: event["child_pid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok()),
    })
}

fn cid_channel(
    stdout: impl Read + Send + 'static,
) -> (
    std::sync::mpsc::Receiver<CidReceipt>,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let eof = std::sync::Arc::new(AtomicBool::new(false));
    let drained = eof.clone();
    std::thread::spawn(move || {
        let mut reader = io::BufReader::new(stdout);
        for line in (&mut reader).take(CAPTURE_LIMIT as u64).lines() {
            let Ok(line) = line else { continue };
            if let Some(receipt) = cid_receipt(&line) {
                let _ = tx.send(receipt);
                break;
            }
        }
        drop(tx);
        // Drain without keeping any more output; the caller never joins.
        drained.store(
            io::copy(&mut reader, &mut io::sink()).is_ok(),
            Ordering::Relaxed,
        );
    });
    (rx, eof)
}

struct Captured {
    status: Option<ExitStatus>,
    stdout: File,
    stderr: String,
}

impl Captured {
    fn stdout_text(&self) -> String {
        capture_text(&self.stdout)
    }
}

fn run_captured(command: &mut Command, budget: Duration) -> io::Result<Captured> {
    let (stdout, out) = capture_file();
    let (stderr, err) = capture_file();
    let mut child = SceneChild::spawn(command.stdout(out).stderr(err))?;
    let status = child.wait_until(Instant::now() + budget)?;
    if status.is_some() {
        child.reap()?;
    }
    Ok(Captured {
        status,
        stdout,
        stderr: capture_text(&stderr),
    })
}

fn remove_scene_cid_by(cid: &str, deadline: Instant) -> Option<bool> {
    debug_assert!(valid_cid(cid));
    let runtime = std::env::var_os("MAOS_T3_RUNTIME")?;
    if Instant::now() >= deadline {
        return None;
    }
    let Ok(mut child) = SceneChild::spawn(
        Command::new(runtime)
            .args(["rm", "--force", "--volumes", "--time=0", cid])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    ) else {
        return Some(false);
    };
    let stop_by = deadline - Duration::from_secs(1);
    match child.wait_until(stop_by) {
        Ok(Some(status)) => Some(child.reap().is_ok() && status.success()),
        _ => {
            child.signal_group();
            // Use only the remainder of the shared deadline for reaping.
            let _ = child.wait_until(deadline);
            let _ = child.finish_cleanup();
            Some(false)
        }
    }
}

fn remove_scene_cid(cid: &str) {
    if valid_cid(cid) {
        let _ = remove_scene_cid_by(cid, Instant::now() + CID_CLEANUP_BUDGET);
    }
}

struct TerminalRun {
    status: std::process::ExitStatus,
    rows: Vec<(u32, String, serde_json::Value)>,
    stdout: String,
    stderr: String,
}

fn real_worker_run(mode: Option<&str>, oom: bool) -> TerminalRun {
    use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};

    let scratch = tempfile::tempdir().expect("real Worker scratch");
    let dir = scratch.path();
    let home = dir.join("home");
    std::fs::create_dir_all(&home).expect("create isolated worker home");
    let args = mode.map_or_else(
        || "[\"--maos-worker\"]".to_owned(),
        |mode| format!("[\"--maos-worker\", \"--maos-fixture-mode={mode}\"]"),
    );
    let resources = if oom {
        "\n[resources]\nmemory_max_mb = 64\n"
    } else {
        ""
    };
    let manifest = dir.join("worker.toml");
    std::fs::write(
        &manifest,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = {args}\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n{resources}"
        ),
    )
    .expect("write T3 worker manifest");
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let paths = layout::path_with_profile();
    let (stdout, out) = capture_file();
    let (stderr, err) = capture_file();
    let mut child = SceneChild::spawn_worker(
        SceneChild::worker_command(env!("CARGO_BIN_EXE_maos"))
            .arg("run")
            .arg(&manifest)
            .arg("--once")
            .current_dir(&root)
            .env("HOME", t3_substrate::isolated_runtime_home(&home))
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", &home)
            .env("MAOS_TEST_BOOT_NONCE", 5895u64.to_string())
            .env("PATH", paths)
            .envs(std::env::vars().filter(|(key, _)| {
                key.starts_with("MAOS_T3_")
                    || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
            }))
            .stdout(out)
            .stderr(err),
    )
    .expect("spawn real T3 worker root");
    let status = child
        .wait_until(Instant::now() + Duration::from_secs(120))
        .expect("poll real Worker root");
    let stdout = capture_text(&stdout);
    let stderr = capture_text(&stderr);
    child.cid = stdout.lines().find_map(cid_receipt).map(|r| r.cid);
    let Some(status) = status else {
        let cleanup = child.cleanup_failure();
        panic!(
            "real Worker phase=root-exit timeout; cleanup={cleanup:?}; stderr: {}",
            diagnostic_tags(&stderr)
        );
    };
    let why = || format!("root status={status}; stderr: {}", diagnostic_tags(&stderr));
    let database = home.join("audit/transparency.sqlite");
    let log = TransparencyLogAdapter::open(&database, 0x17_07)
        .unwrap_or_else(|_| panic!("open worker transparency log; {}", why()));
    let rows: Vec<_> = log
        .query_frames(FrameFilter::default())
        .unwrap_or_else(|_| panic!("query worker transparency log; {}", why()))
        .into_iter()
        .map(|row| {
            let payload =
                serde_json::from_slice(&row.payload_redacted).unwrap_or(serde_json::Value::Null);
            (row.spirit_pid, row.intent, payload)
        })
        .collect();
    if rows
        .iter()
        .any(|(_, intent, _)| intent == "cli.subprocess.exit")
    {
        child.reap().expect("reap completed Worker root");
    } else {
        let cleanup = child.cleanup_failure();
        eprintln!("real Worker phase=missing-exit-row; cleanup={cleanup:?}");
    }
    TerminalRun {
        status,
        rows,
        stdout,
        stderr,
    }
}

fn exit_row(run: &TerminalRun) -> (u32, &serde_json::Value) {
    let exits: Vec<_> = run
        .rows
        .iter()
        .filter(|(_, intent, _)| intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(
        exits.len(),
        1,
        "one Worker exit row: root status={}; stderr: {}; audit rows={} nack={} unload={}",
        run.status,
        diagnostic_tags(&run.stderr),
        run.rows.len(),
        run.rows
            .iter()
            .any(|(_, intent, _)| intent == "task.nacked"),
        run.rows
            .iter()
            .any(|(_, intent, _)| intent == "lifecycle.unload")
    );
    let exit = &exits[0].2;
    let outer: std::collections::BTreeSet<&str> = exit
        .as_object()
        .expect("exit payload object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        outer,
        [
            "event",
            "cli_child_pid",
            "exit_cause",
            "is_crash",
            "container"
        ]
        .into_iter()
        .collect()
    );
    let container: std::collections::BTreeSet<&str> = exit["container"]
        .as_object()
        .expect("T3 container object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        container,
        [
            "runtime_exit_code",
            "oom_marker",
            "payload_exit",
            "payload_exit_source",
            "removed",
            "residue",
        ]
        .into_iter()
        .collect()
    );
    (exits[0].0, exit)
}
fn proc_state(pid: u32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let state = stat.rsplit_once(") ")?.1.chars().next()?;
    "RSDZTtXxKWPI".contains(state).then_some(state)
}

#[test]
fn failed_scene_stops_owned_group_after_leader_exits() {
    let (stdout, out) = capture_file();
    let mut child = SceneChild::spawn_worker(
        SceneChild::worker_command("/bin/sh")
            .args(["-c", "/bin/sleep 30 & printf '%s\\n' \"$!\"; exit 7"])
            .stdout(out),
    )
    .expect("spawn scene with a live group member");
    let exit = child
        .wait_until(Instant::now() + Duration::from_secs(5))
        .expect("observe scene exit");
    assert_eq!(exit.and_then(|status| status.code()), Some(7));
    assert_eq!(proc_state(child.pid()), Some('Z'), "keep the leader PID");
    let member: u32 = capture_text(&stdout).trim().parse().expect("group member");
    let is_live = || proc_state(member).is_some_and(|state| !matches!(state, 'Z' | 'X' | 'x'));
    assert!(is_live(), "group member must outlive the leader");
    let cleanup = child.cleanup_failure();
    assert_eq!(cleanup.status, exit, "reap only after owned cleanup");
    let deadline = Instant::now() + Duration::from_secs(3);
    while is_live() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!is_live(), "cleanup must stop the surviving group member");
}

fn proc_wait_channel(pid: u32) -> Option<String> {
    let channel = std::fs::read_to_string(format!("/proc/{pid}/wchan")).ok()?;
    let channel = channel.trim();
    (channel.len() <= 64
        && !channel.is_empty()
        && channel
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.'))
    .then(|| channel.to_owned())
}

fn partial_s3_audit(database: &Path) -> String {
    // Do not create/migrate the database or wait behind its live writer.
    // Query counts only; neither NDJSON nor audit payloads are diagnostic text.
    let sample = || -> rusqlite::Result<_> {
        let db = rusqlite::Connection::open_with_flags(
            database,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        db.busy_timeout(Duration::ZERO)?;
        db.query_row(
            "SELECT count(*), count(CASE WHEN intent='cli.subprocess.output' THEN 1 END), \
             count(CASE WHEN intent='cli.subprocess.exit' THEN 1 END), \
             count(CASE WHEN intent='task.orphaned' THEN 1 END), \
             count(CASE WHEN intent='lifecycle.unload' THEN 1 END) \
             FROM (SELECT intent FROM transparency_log LIMIT 256)",
            [],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, u32>(4)?,
                ))
            },
        )
    };
    match sample() {
        Ok((rows, output, exit, orphan, unload)) => {
            format!("audit sample rows={rows} output={output} exit={exit} orphan={orphan} unload={unload}")
        }
        Err(error) => format!(
            "audit sample unavailable (SQLite code {:?})",
            error.sqlite_error_code()
        ),
    }
}

fn open_live_payload(cid: &str) -> rustix::fd::OwnedFd {
    let runtime = std::env::var("MAOS_T3_RUNTIME").expect("runtime provisioned");
    let output = run_captured(
        Command::new(runtime).args(["inspect", "--format", "{{.State.Pid}}", cid]),
        Duration::from_secs(8),
    )
    .expect("inspect published CID");
    assert!(
        output.status.is_some_and(|status| status.success()),
        "inspect CID failed: status={:?}; stderr: {}",
        output.status,
        diagnostic_tags(&output.stderr)
    );
    let pid: u32 = output
        .stdout_text()
        .trim()
        .parse()
        .expect("inspect returned host payload PID");
    assert!(pid > 1);
    let pidfd = process::pidfd_open(
        Pid::from_raw(i32::try_from(pid).expect("Linux PID range")).expect("payload PID"),
        process::PidfdFlags::empty(),
    )
    .expect("hold the payload's process identity");
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).expect("payload cgroup");
    assert!(maos_domain::sandbox::cgroup_names_container(&cgroup, cid));
    pidfd
}

fn kill_payload_worker() -> (TerminalRun, Duration) {
    use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};

    let scratch = tempfile::tempdir().expect("S3 scene scratch");
    let dir = scratch.path();
    let home = dir.join("home");
    std::fs::create_dir_all(&home).expect("create root home");
    let manifest = dir.join("worker.toml");
    std::fs::write(
        &manifest,
        "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
         argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=line-then-hang\"]\n\
         output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
         recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
         stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
         shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\
         \n[author]\nname = \"MAOS Project\"\n",
    )
    .expect("write hanging worker manifest");
    let root_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let paths = layout::path_with_profile();
    let (stderr, err) = capture_file();
    let mut child = SceneChild::spawn_worker(
        SceneChild::worker_command(env!("CARGO_BIN_EXE_maos"))
            .arg("run")
            .arg(&manifest)
            .arg("--once")
            .current_dir(&root_dir)
            .env("HOME", t3_substrate::isolated_runtime_home(&home))
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", &home)
            .env("MAOS_OLLAMA_URL", "skip")
            .env("MAOS_TEST_BOOT_NONCE", "5895")
            .env("PATH", paths)
            .envs(std::env::vars().filter(|(key, _)| key.starts_with("MAOS_T3_")))
            .stdout(Stdio::piped())
            .stderr(err),
    )
    .expect("spawn root for external SIGKILL scene");
    let (receipt_rx, stdout_eof) = cid_channel(child.child().stdout.take().expect("root stdout"));
    let database = home.join("audit/transparency.sqlite");
    let receipt = match receipt_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(receipt) => receipt,
        Err(error) => {
            let status = child.poll().expect("poll S3 launch");
            let stderr = diagnostic_tags(&capture_text(&stderr));
            let audit = partial_s3_audit(&database);
            let cleanup = child.cleanup_failure();
            panic!(
                "S3 phase=await-cid; receipt={error:?}; root status={status:?}; \
                 cleanup={cleanup:?}; stderr: {stderr}; {audit}"
            );
        }
    };
    let payload = open_live_payload(child.cid.insert(receipt.cid));
    let started = Instant::now();
    process::pidfd_send_signal(&payload, Signal::KILL).expect("kill CID-verified payload");
    let root_deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if Instant::now() >= root_deadline {
            let elapsed = started.elapsed();
            let root_state = proc_state(child.pid());
            let client_state = receipt.client_pid.and_then(proc_state);
            let root_wait = proc_wait_channel(child.pid());
            let client_wait = receipt.client_pid.and_then(proc_wait_channel);
            let stdout_eof = stdout_eof.load(std::sync::atomic::Ordering::Relaxed);
            let stderr = diagnostic_tags(&capture_text(&stderr));
            let audit = partial_s3_audit(&database);
            let cleanup = child.cleanup_failure();
            panic!(
                "S3 phase=await-root-exit after payload SIGKILL; elapsed={elapsed:?}; \
                 cid receipt=true; stdout EOF={stdout_eof}; root state={root_state:?} wait={root_wait:?}; \
                 client pid={:?} state={client_state:?} wait={client_wait:?}; \
                 cleanup={cleanup:?}; {audit}; stderr: {stderr}",
                receipt.client_pid
            );
        }
        if let Some(status) = child.poll().expect("poll crashed root") {
            break status;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let stderr = capture_text(&stderr);
    let log = TransparencyLogAdapter::open(&database, 0x17_07).expect("open S3 transparency log");
    let rows: Vec<_> = log
        .query_frames(FrameFilter::default())
        .expect("query S3 transparency log")
        .into_iter()
        .map(|row| {
            (
                row.spirit_pid,
                row.intent,
                serde_json::from_slice(&row.payload_redacted).unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();
    if rows
        .iter()
        .any(|(_, intent, _)| intent == "cli.subprocess.exit")
    {
        child.reap().expect("reap S3 Worker root");
    } else {
        let cleanup = child.cleanup_failure();
        eprintln!("S3 phase=missing-exit-row; cleanup={cleanup:?}");
    }
    (
        TerminalRun {
            status,
            rows,
            stdout: String::new(),
            stderr,
        },
        elapsed,
    )
}

// Scene groups. Each file is a child module, so it reaches this harness's
// private helpers through `super::`.
#[path = "worker_terminal_facts_17_7/root_crash.rs"]
mod root_crash;
#[path = "worker_terminal_facts_17_7/runtime_scenes.rs"]
mod runtime_scenes;
#[path = "worker_terminal_facts_17_7/stub_scenes.rs"]
mod stub_scenes;
#[path = "worker_terminal_facts_17_7/unload_scenes.rs"]
mod unload_scenes;

#[path = "worker_terminal_facts_17_7/startup_scenes.rs"]
mod startup_scenes;
#[path = "worker_terminal_facts_17_7/supervisor_scenes.rs"]
mod supervisor_scenes;

#[test]
fn s1_immediate_exit_terminal_facts() {
    if !scene("S1 immediate exit") {
        return;
    }
    let run = real_worker_run(None, false);
    assert!(run.status.success(), "S1 must complete: {}", run.stdout);
    let (pid, exit) = exit_row(&run);
    assert_eq!(exit["exit_cause"], "exited(code=0)");
    assert_eq!(exit["is_crash"], false);
    assert_eq!(exit["container"]["runtime_exit_code"], 0);
    assert_eq!(exit["container"]["oom_marker"], false);
    assert_eq!(exit["container"]["payload_exit"], "exited(code=0)");
    assert_eq!(exit["container"]["payload_exit_source"], "pidfd");
    assert_eq!(exit["container"]["removed"], true);
    assert_eq!(exit["container"]["residue"], serde_json::Value::Null);
    assert!(!run.rows.iter().any(|(row_pid, intent, _)| {
        *row_pid == pid
            && matches!(
                intent.as_str(),
                "task.orphaned" | "lifecycle.crash" | "unplanned_crash"
            )
    }));
    assert!(run.rows.iter().any(|(row_pid, intent, payload)| {
        *row_pid == pid && intent == "worker.completion-verdict" && payload["completed"] == true
    }));
}

#[test]
fn s2_intentional_exit_137_is_not_a_signal() {
    if !scene("S2 exit-137") {
        return;
    }
    let run = real_worker_run(Some("exit-137"), false);
    assert!(!run.status.success(), "S2 intentional exit(137) is a crash");
    let (pid, exit) = exit_row(&run);
    assert_eq!(exit["exit_cause"], "exited(code=137)");
    assert_eq!(exit["is_crash"], true);
    assert_eq!(exit["container"]["runtime_exit_code"], 137);
    assert_eq!(exit["container"]["oom_marker"], false);
    assert_eq!(exit["container"]["payload_exit"], "exited(code=137)");
    assert_eq!(exit["container"]["payload_exit_source"], "pidfd");
    assert_eq!(exit["container"]["removed"], true);
    assert_eq!(exit["container"]["residue"], serde_json::Value::Null);
    let orphaned = run
        .rows
        .iter()
        .find(|(row_pid, intent, _)| *row_pid == pid && intent == "task.orphaned")
        .expect("S2 task.orphaned row");
    assert_eq!(orphaned.2["cause"], "fault.non-zero-exit");
    assert_eq!(orphaned.2["exit_code"], 137);
    assert_eq!(orphaned.2["exit_signal"], serde_json::Value::Null);
    let crash_receipts: Vec<_> = run
        .rows
        .iter()
        .filter(|(row_pid, intent, payload)| {
            *row_pid == pid && intent == "unplanned_crash" && !payload.is_null()
        })
        .collect();
    assert_eq!(crash_receipts.len(), 1, "{crash_receipts:?}");
    assert!(run.stdout.contains("not_completed:process_crash"));
    assert!(run.rows.iter().any(|(row_pid, intent, payload)| {
        *row_pid == pid && intent == "worker.completion-verdict" && payload["completed"] == false
    }));
}

/// Story 20-7 AC7.3: keep the kernel release and typed terminal facts first
/// (PIDFD_INFO_EXIT requires Linux >= 6.15). Do not replay raw rows or stderr.
fn s3_diagnostics(run: &TerminalRun, elapsed: Duration, version: &str) -> String {
    use maos_domain::sandbox::ContainerExitRecord;

    let exit = run
        .rows
        .iter()
        .find(|(_, intent, _)| intent == "cli.subprocess.exit")
        .and_then(|(_, _, row)| {
            serde_json::from_value::<ContainerExitRecord>(row["container"].clone()).ok()
        });
    let facts = exit.map_or_else(
        || "missing or malformed".to_owned(),
        |exit| {
            let payload = exit.payload_exit.as_str();
            let numeric = |prefix| {
                payload
                    .strip_prefix(prefix)?
                    .strip_suffix(')')?
                    .parse::<u8>()
                    .ok()
            };
            let payload = if let Some(code) = numeric("exited(code=") {
                format!("exited(code={code})")
            } else if let Some(signal) = numeric("signaled(sig=") {
                format!("signaled(sig={signal})")
            } else if payload == "unknown" {
                "unknown".to_owned()
            } else {
                "malformed".to_owned()
            };
            format!(
                "payload={payload} source={:?} code={:?} oom={:?} removed={} residue={:?}",
                exit.payload_exit_source,
                exit.runtime_exit_code,
                exit.oom_marker,
                exit.removed,
                exit.residue
            )
        },
    );
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    let kernel = kernel.trim();
    format!(
        "kernel {kernel}; exit facts [{facts}]; elapsed {elapsed:?}; runtime {version}; \
         root status={}; stderr: {}",
        run.status,
        diagnostic_tags(&run.stderr)
    )
}

#[test]
fn s3_external_sigkill_requires_pidfd_fact() {
    if !scene("S3 external SIGKILL") {
        return;
    }
    let (run, elapsed) = kill_payload_worker();
    let runtime = std::env::var("MAOS_T3_RUNTIME").expect("runtime provisioned");
    let version = run_captured(
        Command::new(&runtime).arg("--version"),
        Duration::from_secs(5),
    )
    .ok()
    .filter(|output| output.status.is_some_and(|status| status.success()))
    .and_then(|output| {
        let stdout = output.stdout_text();
        let (prefix, version) = stdout.split_once(" version ")?;
        if !matches!(prefix, "podman" | "Docker") {
            return None;
        }
        let version = version
            .split_ascii_whitespace()
            .next()?
            .trim_end_matches(',');
        (version.len() <= 32 && version.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
            .then(|| format!("{runtime} {version}"))
    })
    .unwrap_or_else(|| format!("{runtime} version unavailable"));
    let why = s3_diagnostics(&run, elapsed, &version);
    assert!(
        !run.status.success(),
        "external SIGKILL must fail the Worker run: {why}"
    );
    // Facts first, timing last (20-7 D-20-7-01 A). The pidfd and the cleanup
    // verdict hold whether or not the runtime CLI answered its snapshot.
    let (pid, exit) = exit_row(&run);
    assert_eq!(exit["is_crash"], true, "{why}");
    assert_eq!(
        exit["container"]["payload_exit"], "signaled(sig=9)",
        "{why}"
    );
    assert_eq!(exit["container"]["payload_exit_source"], "pidfd", "{why}");
    assert_eq!(exit["container"]["removed"], true, "{why}");
    assert_eq!(
        exit["container"]["residue"],
        serde_json::Value::Null,
        "{why}"
    );
    let orphaned = run
        .rows
        .iter()
        .find(|(row_pid, intent, _)| *row_pid == pid && intent == "task.orphaned")
        .expect("S3 task.orphaned row");
    let crash_receipts: Vec<_> = run
        .rows
        .iter()
        .filter(|(row_pid, intent, payload)| {
            *row_pid == pid && intent == "unplanned_crash" && !payload.is_null()
        })
        .collect();
    assert_eq!(crash_receipts.len(), 1, "{why}");
    // The runtime's own facts exist only when podman answered its snapshot. A
    // stalled runtime is the typed unavailable form (null code and marker) and
    // classifies `fault.truncated` by the 17-7 rule that unavailable state is
    // always truncated (`container_terminal_facts_17_7.rs`).
    if exit["container"]["runtime_exit_code"].is_null() {
        println!("S3: runtime snapshot unavailable (typed form); {why}");
        assert!(exit["container"]["oom_marker"].is_null(), "{why}");
        assert_eq!(orphaned.2["cause"], "fault.truncated", "{why}");
    } else {
        assert_eq!(exit["exit_cause"], "exited(code=137)", "{why}");
        assert_eq!(exit["container"]["runtime_exit_code"], 137, "{why}");
        assert_eq!(exit["container"]["oom_marker"], false, "{why}");
        assert_eq!(orphaned.2["cause"], "fault.signaled", "{why}");
        assert_eq!(orphaned.2["exit_signal"], 9, "{why}");
        assert_eq!(orphaned.2["exit_code"], serde_json::Value::Null, "{why}");
    }
    // The bound times kill -> root exit: FR12 detection plus the whole T3
    // teardown plus root shutdown. The teardown's own documented budget is the
    // snapshot budget (podman 5.x holds the container lock ~8 s after exit; a
    // stalled podman 4.x client ran the snapshot to its deadline on hosted
    // runs 38807569588…41281999213) plus 2 s for the kill, the removal and the
    // exit. FR12's 2 s / 5 s floors stay asserted by
    // `worker_crash_corpus_16_3` (>= 99/100).
    let bound =
        maos_kernel_core::security::sandbox::t3::child::SNAPSHOT_BUDGET + Duration::from_secs(2);
    assert!(
        elapsed <= bound,
        "orphan detection took {elapsed:?} (bound {bound:?}): {why}"
    );
}

#[test]
fn s4_oom_marker_is_not_payload_evidence_alone() {
    if !scene("S4 memory.max OOM") {
        return;
    }
    let marker = runtime_scenes::runtime_reports_oom_marker(64);
    let run = real_worker_run(Some("oom"), true);
    assert!(
        !run.status.success(),
        "S4 OOM must be classified as a crash: root status={}; stderr: {}",
        run.status,
        diagnostic_tags(&run.stderr)
    );
    let (pid, exit) = exit_row(&run);
    // "First persisted stderr line" is the Worker's first journaled stderr
    // row, never the root's own stderr.
    let mut stderr: Vec<_> = run
        .rows
        .iter()
        .filter(|(row_pid, intent, row)| {
            *row_pid == pid && intent == "cli.subprocess.output" && row["stream"] == "stderr"
        })
        .map(|(_, _, row)| (row["line_no"].as_u64(), row["line"].as_str().unwrap_or("")))
        .collect();
    stderr.sort_by_key(|(line_no, _)| *line_no);
    assert!(
        stderr.first().is_some_and(|(_, line)| *line == "67108864"),
        "the first persisted stderr line must be the fixture's 64 MiB marker"
    );
    assert!(
        !stderr.iter().any(|(_, line)| *line == "1073741824"),
        "the OOM fixture must die before its fixed 1 GiB cap"
    );
    assert_eq!(exit["is_crash"], true);
    assert_eq!(exit["container"]["runtime_exit_code"], 137);
    assert_eq!(
        exit["container"]["oom_marker"], marker,
        "MAOS records the runtime's own marker"
    );
    assert_eq!(exit["container"]["payload_exit"], "signaled(sig=9)");
    assert_eq!(exit["container"]["payload_exit_source"], "pidfd");
    assert_eq!(exit["container"]["removed"], true);
    assert_eq!(exit["container"]["residue"], serde_json::Value::Null);
    let orphaned = run
        .rows
        .iter()
        .find(|(row_pid, intent, _)| *row_pid == pid && intent == "task.orphaned")
        .expect("S4 task.orphaned row");
    // With the marker the SIGKILL is an OOM; without it the same pidfd
    // SIGKILL is the S3 row — the marker is never invented.
    let cause = if marker {
        "fault.oom_killed"
    } else {
        "fault.signaled"
    };
    assert_eq!(orphaned.2["cause"], cause);
    assert_eq!(orphaned.2["exit_signal"], 9);
    assert_eq!(orphaned.2["exit_code"], serde_json::Value::Null);
    assert!(run.rows.iter().any(|(row_pid, intent, payload)| {
        *row_pid == pid && intent == "worker.completion-verdict" && payload["completed"] == false
    }));
}
