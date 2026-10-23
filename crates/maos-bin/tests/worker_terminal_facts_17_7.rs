#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 17-7's terminal-facts regression suite. Runtime-backed cases are
//! deliberately capability-gated: a declared CI runner must have its signed
//! rootless T3 substrate, while local development clearly names unavailable
//! runtime scenes. Stub-runtime cases always run and exercise CID teardown.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use maos_domain::sandbox::ContainerRuntimeKind;
use maos_kernel_core::security::sandbox::t3::child::{
    SandboxedContainerChild, KILL_BUDGET, REMOVE_BUDGET, TEARDOWN_BUDGET,
};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const CID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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

struct TerminalRun {
    status: std::process::ExitStatus,
    rows: Vec<(u32, String, serde_json::Value)>,
    stdout: String,
    stderr: String,
}

fn real_worker_run(mode: Option<&str>, oom: bool) -> TerminalRun {
    use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};

    let dir = scratch("real-worker");
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
    let mut paths = vec![root.join("target/debug"), root.join("target/debug/deps")];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let output = Command::new("timeout")
        .arg("120s")
        .arg(env!("CARGO_BIN_EXE_maos"))
        .arg("run")
        .arg(&manifest)
        .arg("--once")
        .current_dir(&root)
        .env("HOME", t3_substrate::isolated_runtime_home(&home))
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", &home)
        .env("MAOS_TEST_BOOT_NONCE", 5895u64.to_string())
        .env("PATH", std::env::join_paths(paths).expect("valid PATH"))
        .envs(std::env::vars().filter(|(key, _)| {
            key.starts_with("MAOS_T3_")
                || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
        }))
        .output()
        .expect("run real T3 worker root");
    let database = home.join("audit/transparency.sqlite");
    let log =
        TransparencyLogAdapter::open(&database, 0x17_07).expect("open worker transparency log");
    let rows = log
        .query_frames(FrameFilter::default())
        .expect("query worker transparency log")
        .into_iter()
        .map(|row| {
            let payload =
                serde_json::from_slice(&row.payload_redacted).unwrap_or(serde_json::Value::Null);
            (row.spirit_pid, row.intent, payload)
        })
        .collect();
    let result = TerminalRun {
        status: output.status,
        rows,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    std::fs::remove_dir_all(&dir).expect("remove real-scene home");
    result
}

fn exit_row(run: &TerminalRun) -> (u32, &serde_json::Value) {
    let exits: Vec<_> = run
        .rows
        .iter()
        .filter(|(_, intent, _)| intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(exits.len(), 1, "one Worker exit row: {:?}", run.rows);
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
fn published_cid(stdout: &std::sync::Arc<std::sync::Mutex<String>>) -> Option<String> {
    stdout.lock().ok()?.lines().find_map(|line| {
        let event: serde_json::Value = serde_json::from_str(line).ok()?;
        (event["event"] == "cli_wrapper_loaded")
            .then(|| event["container_id"].as_str().map(str::to_owned))
            .flatten()
    })
}

fn inspect_live_payload(cid: &str) -> u32 {
    let runtime = std::env::var("MAOS_T3_RUNTIME").expect("runtime provisioned");
    let output = Command::new("timeout")
        .args(["8s", &runtime, "inspect", "--format", "{{.State.Pid}}", cid])
        .output()
        .expect("inspect published CID");
    assert!(output.status.success(), "inspect CID {cid} failed");
    let pid: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("inspect returned host payload PID");
    assert!(pid > 1);
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).expect("payload cgroup");
    assert!(maos_domain::sandbox::cgroup_names_container(&cgroup, cid));
    pid
}

fn kill_payload_worker() -> (TerminalRun, Duration) {
    use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
    use std::io::{BufRead as _, Read as _};

    let dir = scratch("external-sigkill");
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
    let mut paths = vec![
        root_dir.join("target/debug"),
        root_dir.join("target/debug/deps"),
    ];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .arg("run")
        .arg(&manifest)
        .arg("--once")
        .current_dir(&root_dir)
        .env("HOME", t3_substrate::isolated_runtime_home(&home))
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", &home)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_TEST_BOOT_NONCE", "5895")
        .env("PATH", std::env::join_paths(paths).expect("valid PATH"))
        .envs(std::env::vars().filter(|(key, _)| key.starts_with("MAOS_T3_")))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn root for external SIGKILL scene");
    let stdout = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let stdout_sink = std::sync::Arc::clone(&stdout);
    let stdout_pipe = child.stdout.take().expect("root stdout");
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout_pipe).lines() {
            let Ok(line) = line else { break };
            let mut capture = stdout_sink.lock().expect("stdout lock");
            capture.push_str(&line);
            capture.push('\n');
        }
    });
    let stderr_pipe = child.stderr.take().expect("root stderr");
    let stderr_thread = std::thread::spawn(move || {
        let mut text = String::new();
        let mut reader = std::io::BufReader::new(stderr_pipe);
        let _ = reader.read_to_string(&mut text);
        text
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let cid = loop {
        if let Some(cid) = published_cid(&stdout) {
            break cid;
        }
        assert!(
            Instant::now() < deadline,
            "worker did not publish cli_wrapper_loaded CID"
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    let payload_pid = inspect_live_payload(&cid);
    let started = Instant::now();
    let killed = Command::new("kill")
        .args(["-KILL", &payload_pid.to_string()])
        .status()
        .expect("kill CID-verified payload");
    assert!(killed.success());
    let root_deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll crashed root") {
            break status;
        }
        if Instant::now() >= root_deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("root did not exit within its S3 deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let stdout = stdout.lock().expect("captured stdout").clone();
    let stderr = stderr_thread.join().expect("stderr reader");
    let log = TransparencyLogAdapter::open(&home.join("audit/transparency.sqlite"), 0x17_07)
        .expect("open S3 transparency log");
    let rows = log
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
    std::fs::remove_dir_all(&dir).expect("remove S3 scene home");
    (
        TerminalRun {
            status,
            rows,
            stdout,
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

#[test]
fn s3_external_sigkill_requires_pidfd_fact() {
    if !scene("S3 external SIGKILL") {
        return;
    }
    let (run, elapsed) = kill_payload_worker();
    assert!(
        !run.status.success(),
        "external SIGKILL must fail the Worker run"
    );
    // FR12's 5 s holds on podman 4; podman 5.x keeps the container lock for
    // seconds after exit, so the snapshot may use its full budget there
    // (operator ruling 2026-10-14: SNAPSHOT_BUDGET 10 s, S3 bound 12 s).
    let version =
        maos_kernel_core::security::sandbox::t3::runtime_detect::detect_container_runtime()
            .expect("T3 runtime")
            .version;
    // `version` is the runtime's own text, e.g. "podman version 5.7.0".
    let bound = if version
        .split_whitespace()
        .last()
        .and_then(|number| number.split('.').next())
        .and_then(|major| major.parse::<u32>().ok())
        >= Some(5)
    {
        maos_kernel_core::security::sandbox::t3::child::SNAPSHOT_BUDGET + Duration::from_secs(2)
    } else {
        Duration::from_secs(5)
    };
    assert!(
        elapsed <= bound,
        "observer/orphan detection took {elapsed:?} (bound {bound:?}, runtime {version}); stderr: {}",
        run.stderr
    );
    let (pid, exit) = exit_row(&run);
    assert_eq!(exit["exit_cause"], "exited(code=137)");
    assert_eq!(exit["is_crash"], true);
    assert_eq!(exit["container"]["runtime_exit_code"], 137);
    assert_eq!(exit["container"]["oom_marker"], false);
    assert_eq!(exit["container"]["payload_exit"], "signaled(sig=9)");
    assert_eq!(exit["container"]["payload_exit_source"], "pidfd");
    assert_eq!(exit["container"]["removed"], true);
    assert_eq!(exit["container"]["residue"], serde_json::Value::Null);
    let orphaned = run
        .rows
        .iter()
        .find(|(row_pid, intent, _)| *row_pid == pid && intent == "task.orphaned")
        .expect("S3 task.orphaned row");
    assert_eq!(orphaned.2["cause"], "fault.signaled");
    assert_eq!(orphaned.2["exit_signal"], 9);
    assert_eq!(orphaned.2["exit_code"], serde_json::Value::Null);
    let crash_receipts: Vec<_> = run
        .rows
        .iter()
        .filter(|(row_pid, intent, payload)| {
            *row_pid == pid && intent == "unplanned_crash" && !payload.is_null()
        })
        .collect();
    assert_eq!(crash_receipts.len(), 1, "{crash_receipts:?}");
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
        "S4 OOM must be classified as a crash"
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
    assert_eq!(stderr.first().map(|(_, line)| *line), Some("67108864"));
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
