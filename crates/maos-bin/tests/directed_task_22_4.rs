//! Story 22-4 — directed task ingress against a real `maos serve` Host. A signed, installed
//! `recall_probe.wasm` component (it answers the shell request shape with `recall-probe ok`)
//! stands in the cast beside a native class; `maosctl ask` hands it tasks through the door and
//! `maosctl task show` reads them back, across SIGTERM and SIGKILL restarts. The queue bound,
//! the halt/orphan classification and the read-back rebuilds are proven in-process in
//! `task_registry_22_4.rs` (the 20-9 runner fault fixture is not wired into a serve cast).
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;

use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use support::{publish_artifact, root, sibling, InProcessServer, Scratch};

const PROBE: &str = "recall_probe.wasm";
const ID: &str = "probe-d";
const WAIT: Duration = Duration::from_secs(90);

fn manifest(id: &str) -> String {
    format!(
        r#"[class]
name = "{id}"
description = "directed task witness"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "dist/{PROBE}"
trust_tier = "public-untrusted"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["introduction"]
[budget]
context_window_size = 4096
time_cap_seconds = 30
[resources]
fd_max = 64
[sandbox]
tier = "T2"
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
fetch = true
"#
    )
}

fn text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A real `maos serve` Host over one installed probe, optionally beside a native class.
struct Host {
    scratch: Scratch,
    _server: InProcessServer,
    config: PathBuf,
    child: Option<Child>,
    /// The cast holds the native class `nash` beside the probe.
    native: bool,
}

impl Host {
    fn new(native: bool) -> Self {
        let scratch = Scratch::new();
        let server = InProcessServer::start(&scratch);
        let artifact = sibling(PROBE);
        let key = scratch.0.join("publisher.key");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&key, "53".repeat(32)).unwrap();
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let package = scratch.0.join("package").join(ID);
        std::fs::create_dir_all(&package).unwrap();
        std::fs::copy(&artifact, package.join(PROBE)).unwrap();
        let published = package.join("manifest.toml");
        std::fs::write(&published, manifest(ID)).unwrap();
        let output = publish_artifact(
            &scratch,
            &sibling("maos-spirit"),
            &server.addr,
            &published,
            &artifact,
            &key,
        );
        assert!(output.status.success(), "publish: {}", text(&output));
        let install = scratch
            .command(&sibling("maosctl"))
            .args([
                "spirit",
                "install",
                &format!("{ID}@1.0.0"),
                "--registry-uri",
                &format!("http://{}/mcp", server.addr),
            ])
            .output()
            .unwrap();
        assert!(install.status.success(), "install: {}", text(&install));
        let _ = sibling("maos-wasm-runner");
        let init = scratch
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maos")))
            .args(["init", "--plain"])
            .output()
            .unwrap();
        assert!(init.status.success(), "init: {}", text(&init));
        let mut cast =
            format!("[host]\ninference_mode = 'replay'\n\n[[host.spirits]]\ninstalled = '{ID}'\n");
        if native {
            cast.push_str(&format!(
                "\n[[host.spirits]]\nmanifest = '{}'\n",
                root().join("spirits/nash/manifest.toml").display()
            ));
        }
        let config = scratch.0.join("host.toml");
        std::fs::write(&config, cast).unwrap();
        Self {
            scratch,
            _server: server,
            config,
            child: None,
            native,
        }
    }

    fn command(&self, program: &std::path::Path) -> Command {
        let mut command = self.scratch.command(program);
        command
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_REGISTRY_URI", "")
            .env("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "300")
            .env("MAOS_YANK_POLICY", "warn")
            .env_remove("MAOS_INFERENCE_MODE")
            .env_remove("MAOS_LOOM_POSTGRES")
            .env_remove("MAOS_LOOM_HOME_TEAM")
            .env_remove("MAOS_CROSS_TEAM_SHARE_PEER")
            .current_dir(root());
        command
    }

    fn boot(&mut self) {
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.scratch.0.join("serve.stderr"))
            .unwrap();
        let child = self
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maos")))
            .args(["serve", "--config"])
            .arg(&self.config)
            .env(
                "MAOS_REPLAY_CASSETTE",
                root().join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
            )
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .expect("spawn maos serve");
        self.child = Some(child);
        self.await_running();
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.scratch.0.join("serve.stderr")).unwrap_or_default()
    }

    fn ctl(&self, args: &[&str]) -> Output {
        self.command(&sibling("maosctl"))
            .args(args)
            .output()
            .unwrap()
    }

    fn agents(&self) -> Vec<String> {
        let out = self.ctl(&["agents"]);
        if !out.status.success() {
            return Vec::new();
        }
        // Story 22-5 widened the line to `id pid state ...`; the read-back here is id and state.
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| {
                let mut fields = line.split('\t');
                format!(
                    "{}\t{}",
                    fields.next().unwrap_or_default(),
                    fields.nth(1).unwrap_or_default()
                )
            })
            .collect()
    }

    /// The whole cast Running: the probe and, when the cast has one, `nash`. The cast is
    /// admitted in order, so a probe Running does not mean `nash` is even registered yet.
    fn await_running(&mut self) {
        let until = Instant::now() + WAIT;
        let mut want = vec![format!("{ID}\tRunning")];
        if self.native {
            want.push("nash\tRunning".to_owned());
        }
        loop {
            let agents = self.agents();
            if want.iter().all(|line| agents.contains(line)) {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                panic!("maos serve exited early: {status}\n{}", self.stderr());
            }
            assert!(
                Instant::now() < until,
                "{ID} never reached Running\n{}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn stop(&mut self, signal: &str) {
        let mut child = self.child.take().expect("a running host");
        Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .unwrap();
        let _ = child.wait();
    }

    /// The host's descendant `maos-wasm-runner` processes (the Spirit sessions).
    fn runners(&self) -> Vec<u32> {
        let root = self.child.as_ref().expect("a running host").id();
        let mut parent_of = std::collections::HashMap::new();
        let mut comm_of = std::collections::HashMap::new();
        for entry in std::fs::read_dir("/proc").unwrap().flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            // `pid (comm) state ppid ...`: the comm may hold spaces, so split after the last `)`.
            let Some(rest) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
                continue;
            };
            if let Some(ppid) = rest
                .split_whitespace()
                .nth(1)
                .and_then(|p| p.parse::<u32>().ok())
            {
                parent_of.insert(pid, ppid);
                comm_of.insert(
                    pid,
                    std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default(),
                );
            }
        }
        parent_of
            .keys()
            .copied()
            .filter(|pid| comm_of[pid].starts_with("maos-wasm-runn"))
            .filter(|pid| {
                let mut at = *pid;
                while let Some(&up) = parent_of.get(&at) {
                    if up == root {
                        return true;
                    }
                    at = up;
                }
                false
            })
            .collect()
    }

    fn signal_pids(pids: &[u32], signal: &str) {
        for pid in pids {
            let _ = Command::new("kill")
                .args([signal, &pid.to_string()])
                .status();
        }
    }

    /// `maosctl ask`, expecting acceptance; the printed task id.
    fn ask(&self, args: &[&str]) -> String {
        let mut full = vec!["ask", ID];
        full.extend_from_slice(args);
        let out = self.ctl(&full);
        assert!(
            out.status.success(),
            "ask {args:?}: {}\n{}",
            text(&out),
            self.stderr()
        );
        let id = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        assert_eq!(id.len(), 26, "a ULID task id: {id:?}");
        assert!(ulid::Ulid::from_string(&id).is_ok(), "{id:?}");
        id
    }

    fn show(&self, id: &str) -> (Option<i32>, String, serde_json::Value) {
        let out = self.ctl(&["task", "show", id]);
        let printed = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        let json = serde_json::from_str(&printed).unwrap_or(serde_json::Value::Null);
        (out.status.code(), printed, json)
    }

    /// The first `task show` of `id` whose state is not queued/running.
    fn finished(&self, id: &str) -> (String, serde_json::Value) {
        let until = Instant::now() + WAIT;
        loop {
            let (code, printed, json) = self.show(id);
            if code == Some(0) && !matches!(json["state"].as_str(), Some("queued" | "running")) {
                return (printed, json);
            }
            assert!(Instant::now() < until, "task {id} never ended: {printed}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn journal(&self) -> Connection {
        let path = self.scratch.0.join("home/.maos/audit/transparency.sqlite");
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("the audit store opens read-only");
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
    }

    fn count(&self, sql: &str, params: impl rusqlite::Params) -> i64 {
        self.journal()
            .query_row(sql, params, |row| row.get(0))
            .unwrap()
    }

    /// `TaskAssign` rows whose frame id is the task id.
    fn task_assigns(&self, id: &str) -> i64 {
        let bytes = ulid::Ulid::from_string(id).unwrap().to_bytes();
        self.count(
            "SELECT COUNT(*) FROM transparency_log WHERE frame_id = ?1 AND kind = 0",
            [&bytes[..]],
        )
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("--- maos serve stderr ---\n{}", self.stderr());
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn ask_returns_a_task_id_and_task_show_returns_the_answer() {
    let mut host = Host::new(false);
    host.boot();

    let id = host.ask(&["hello door"]);
    let (_, shown) = host.finished(&id);
    assert_eq!(shown["state"], "completed", "{shown}");
    assert_eq!(shown["result"], "recall-probe ok");
    assert_eq!(shown["task_id"], id.as_str());
    assert_eq!(shown["spirit_id"], ID);
    assert_eq!(shown["origin"], "door");
    assert!(shown["accepted_ns"].as_u64().unwrap() <= shown["ended_ns"].as_u64().unwrap());
    for absent in ["halt", "reason"] {
        assert!(
            shown.get(absent).is_none(),
            "{absent} on a completed task: {shown}"
        );
    }
    // The task id IS the TaskAssign frame id; the reply row is labelled as a door task's.
    assert_eq!(host.task_assigns(&id), 1);
    let bytes = ulid::Ulid::from_string(&id).unwrap().to_bytes();
    assert_eq!(
        host.count(
            "SELECT COUNT(*) FROM transparency_log WHERE correlation_id = ?1 AND kind = 1",
            [format!("maos.task-reply/{}", hex::encode(bytes))],
        ),
        1,
        "one reply row under maos.task-reply/<hex of the task id>"
    );

    // `--wait` prints the same object once the task ends.
    let out = host.ctl(&["ask", ID, "again", "--wait"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        text(&out),
        host.stderr()
    );
    let waited: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(waited["state"], "completed");
    assert_eq!(waited["result"], "recall-probe ok");
    assert_ne!(
        waited["task_id"],
        id.as_str(),
        "an ask without a key is a new task"
    );

    // An id the Host never accepted.
    let unknown = ulid::Ulid::new().to_string();
    let out = host.ctl(&["task", "show", &unknown]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out).contains("task_not_found"), "{}", text(&out));
    host.stop("-TERM");
}

#[test]
fn every_ask_writes_one_completion_row_naming_its_task() {
    let mut host = Host::new(false);
    host.boot();
    let id = host.ask(&["audit me"]);

    // The row is written before the 200 that carried the id: no polling between the two.
    let intent: String = host
        .journal()
        .query_row(
            "SELECT intent FROM transparency_log WHERE intent LIKE 'operator.ask.%'",
            [],
            |row| row.get(0),
        )
        .expect("exactly one ask completion row");
    let operation_id = intent
        .strip_prefix("operator.ask.")
        .and_then(|rest| rest.strip_suffix(":completed"))
        .unwrap_or_else(|| panic!("intent {intent}"))
        .to_owned();
    assert!(operation_id.starts_with("op-"), "{operation_id}");
    let payload: Vec<u8> = host
        .journal()
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE intent = ?1",
            [&intent],
            |row| row.get(0),
        )
        .unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(receipt["task_id"], id.as_str(), "{receipt}");
    assert_eq!(receipt["verb"], "ask");
    assert_eq!(receipt["spirit_id"], ID);
    assert_eq!(receipt["operation_id"], operation_id.as_str());

    let out = host.ctl(&["audit", "query", "--intent-contains", &operation_id]);
    assert!(out.status.success(), "{}", text(&out));
    let rows: Vec<&str> = std::str::from_utf8(&out.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "exactly one row finds the operation id: {rows:?}"
    );
    assert!(rows[0].contains(&id), "and it names the task: {}", rows[0]);
    // The task's own rows are findable by its id.
    assert_eq!(
        host.count(
            "SELECT COUNT(*) FROM transparency_log WHERE intent = ?1",
            [format!("task.{id}:accepted")],
        ),
        1
    );
    host.stop("-TERM");
}

#[test]
fn refusals_are_typed_and_mint_no_task() {
    let mut host = Host::new(true);
    host.boot();
    let refused = |args: &[&str], code: &str| {
        let out = host.ctl(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}: {}", text(&out));
        assert!(
            text(&out).contains(code),
            "{args:?} wants {code}: {}",
            text(&out)
        );
        assert!(
            out.stdout.is_empty(),
            "{args:?}: a refusal prints no task id"
        );
    };
    refused(&["ask", "nobody", "x"], "spirit_not_loaded");
    refused(&["ask", "nash", "x"], "spirit_not_directable");
    refused(&["ask", ID, "   "], "invalid_task");
    assert_eq!(host.ctl(&["pause", ID]).status.code(), Some(0));
    refused(&["ask", ID, "x"], "spirit_not_running");
    assert_eq!(host.ctl(&["resume", ID]).status.code(), Some(0));

    // Nothing was accepted, and every refusal is recorded in its completion row.
    assert_eq!(
        host.count(
            "SELECT COUNT(*) FROM transparency_log WHERE intent LIKE 'task.%:accepted'",
            [],
        ),
        0
    );
    for code in [
        "spirit_not_loaded",
        "spirit_not_directable",
        "invalid_task",
        "spirit_not_running",
    ] {
        assert_eq!(
            host.count(
                "SELECT COUNT(*) FROM transparency_log WHERE intent LIKE ?1",
                [format!("operator.ask.%:{code}")],
            ),
            // The unknown id has no Spirit pid; its completion row is still written.
            1,
            "{code}"
        );
    }
    // The Host still serves: a task after the refusals completes.
    let id = host.ask(&["after the refusals"]);
    assert_eq!(host.finished(&id).1["state"], "completed");
    host.stop("-TERM");
}

#[test]
fn a_key_survives_a_restart() {
    let mut host = Host::new(false);
    host.boot();
    let first = host.ask(&["once", "--key", "retry.1"]);
    let same = host.ask(&["once", "--key", "retry.1"]);
    assert_eq!(first, same, "the same key is the same task");
    let (_, shown) = host.finished(&first);
    assert_eq!(shown["state"], "completed");
    assert_eq!(
        host.task_assigns(&first),
        1,
        "one TaskAssign row for that id"
    );
    let keyless_a = host.ask(&["once"]);
    let keyless_b = host.ask(&["once"]);
    assert_ne!(keyless_a, keyless_b, "asks without a key are independent");
    assert_ne!(keyless_a, first);

    host.stop("-KILL");
    host.boot();
    let again = host.ask(&["once", "--key", "retry.1"]);
    assert_eq!(again, first, "the key outlives a SIGKILL");
    assert_eq!(host.task_assigns(&first), 1, "still one TaskAssign row");
    // The retried ask was a replay and said so in its receipt.
    let replay_rows = host.count(
        "SELECT COUNT(*) FROM transparency_log WHERE intent LIKE 'operator.ask.%:completed' \
         AND CAST(payload_redacted AS TEXT) LIKE '%\"replayed\":true%'",
        [],
    );
    assert_eq!(replay_rows, 2, "the retries before and after the restart");
    // A different Spirit-scoped key is a new task.
    assert_ne!(host.ask(&["once", "--key", "retry.2"]), first);
    host.stop("-TERM");
}

#[test]
fn task_show_reads_the_audit_trail_after_a_restart() {
    let mut host = Host::new(false);
    host.boot();
    let done = host.ask(&["finish before the stop"]);
    let (done_printed, done_json) = host.finished(&done);
    assert_eq!(done_json["state"], "completed");

    // A SIGTERM drain: the same object, byte for byte.
    host.stop("-TERM");
    host.boot();
    let (code, printed, _) = host.show(&done);
    assert_eq!(code, Some(0));
    assert_eq!(printed, done_printed, "after a SIGTERM drain");

    // A long task holds the Spirit; a second waits behind it; then SIGKILL. The turn must still
    // be running when the kill lands, and nothing about `recalls N` guarantees that: the session
    // protocol caps a turn at 64 recalls (`MAX_RECALLS_PER_TURN`), so a quick Host serves them,
    // ends the session on `recall budget exceeded`, and the task reads `orphaned` (AC7) before
    // the test's next `maosctl` spawn returns. Freeze the runner so the turn can neither finish
    // nor fail until the Host is killed.
    // The session must be ready (probe answered) before it is frozen, or the turn is never
    // delivered at all: one finished ask proves it.
    let warm = host.ask(&["warm the session"]);
    assert_eq!(host.finished(&warm).1["state"], "completed");
    let runners = host.runners();
    assert!(!runners.is_empty(), "no runner under the Host");
    Host::signal_pids(&runners, "-STOP");
    let slow = host.ask(&["recalls 300000"]);
    let until = Instant::now() + WAIT;
    while host.show(&slow).2["state"] != "running" {
        assert!(Instant::now() < until, "the slow task never ran");
        std::thread::sleep(Duration::from_millis(50));
    }
    // The audit writer is asynchronous: wait for the delivered TaskAssign row to be on disk, so
    // the kill cuts a turn whose delivery is already journaled.
    while host.task_assigns(&slow) == 0 {
        assert!(
            Instant::now() < until,
            "the slow task's TaskAssign row never landed"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let queued = host.ask(&["never delivered"]);
    assert_eq!(host.show(&queued).2["state"], "queued");
    host.stop("-KILL");
    Host::signal_pids(&runners, "-KILL");
    host.boot();
    let (_, after_kill, _) = host.show(&done);
    assert_eq!(after_kill, done_printed, "after a SIGKILL");
    for id in [&slow, &queued] {
        let (code, _, json) = host.show(id);
        assert_eq!(code, Some(0));
        assert_eq!(json["state"], "interrupted", "{id}: {json}");
        assert_eq!(json["spirit_id"], ID);
        assert!(json.get("result").is_none());
    }
    // Never re-run: the queued task has no TaskAssign row, the running one keeps its single row.
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(
        host.task_assigns(&queued),
        0,
        "a stop never delivers a queued task"
    );
    assert_eq!(
        host.task_assigns(&slow),
        1,
        "a stop never re-runs a running task"
    );
    host.stop("-TERM");
}
