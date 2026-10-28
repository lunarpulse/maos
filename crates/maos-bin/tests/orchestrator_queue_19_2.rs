#![forbid(unsafe_code)]

//! Story 19-2 (AC1–AC4, AC6, AC7.3; D-19-2-10 A) — the FR20 enqueue door and
//! the safe-point drain, end to end through the production `maos` and
//! `maosctl` binaries, each root on a fresh data home with its own door
//! (`maos init`, D-16-1-Q).
//!
//! * `--epic` vectors (T3, `require_t3`: a named local skip, never a green
//!   skip on a declared-capable runner) hold section 1 in flight with the
//!   debug-only seam (`MAOS_TEST_ONLY_HOLD_SECTION`, D-19-2-04 A), act on the
//!   door while the delegation is in flight, then release it:
//!   - queue → ONE instruction per safe point (D-19-2-02 B), appended to the
//!     NEXT section's goal only; the rest stays queued (D-19-2-03 A);
//!   - recall (FR51(c)) → the recalled instruction reaches no Worker; the
//!     intent row precedes the per-instruction rows (D-19-2-06 A);
//!   - pause of the Orchestrator → the loop holds before section 2's frame
//!     and continues after resume (D-19-2-17 A).
//! * Butler vectors (no T3): a director's pick on a SERVING Butler root —
//!   409 `principal_unbound` with nothing recorded when no principal was
//!   bound at boot; with `MAOS_BUTLER_PRINCIPAL` the declared action runs and
//!   the decision is attested at the Butler's pid AFTER the door's intent
//!   row; a scene with a different env principal refuses the boot.
//!
//! Observables are the roots' stdout events, `maosctl` output and the rows
//! the roots wrote (Transparency Log + Approval Decision Log), never state
//! this test set itself.
//!
//! Proven red (story §Proven-red controls): draining at the in-flight hold
//! (bypassing `drain_next`) reads `0/` during the hold and names section 1;
//! a recall handler that skips `recall_all_pending` lets the instruction
//! reach section 2's goal; a pick handler that answers a canned 200 leaves no
//! `butler:acceptance` row.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use maos_bin::orchestrator_root::HOLD_SECTION_ENV;

const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
const EPIC: &str = "tests/fixtures/j1/epic.md";
const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const SCENE_FIXTURE: &str = "tests/fixtures/j-butler/acceptance-21d.jsonl";
const SCENE_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const PRINCIPAL_ENV: &str = "MAOS_BUTLER_PRINCIPAL";
const PRINCIPAL: &str = "director-ada";
/// `FrameKind::TaskAssign` / `FrameKind::EpistemicHalt` (maos-iac TL codes).
const KIND_TASK_ASSIGN: i64 = 0;
const KIND_EPISTEMIC_HALT: i64 = 3;
/// Every wait is bounded; these are ceilings, not sleeps.
const EVENT_BOUND: Duration = Duration::from_secs(90);
const RUN_BOUND: Duration = Duration::from_secs(180);
/// How long a paused Orchestrator must hold section 2 before it is resumed.
const PAUSED_WINDOW: Duration = Duration::from_millis(1500);

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
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

fn add_t3_environment(command: &mut Command) {
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
}

/// A cleared environment over one data home (HOME = MAOS_HOME = XDG).
fn base(binary: &Path, home: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env_clear()
        .env("PATH", path_with_target_debug())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .current_dir(workspace_root());
    command
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

fn data_home(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-19-2-{tag}-{nanos}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create data home");
    path
}

/// One `maos run` root with a door, its stdout captured line by line.
struct Root {
    home: PathBuf,
    child: Option<Child>,
    stdout: Arc<Mutex<Vec<String>>>,
    stderr: Arc<Mutex<String>>,
}

impl Root {
    fn boot(tag: &str, t3: bool, args: &[&str], env: &[(&str, &str)]) -> Self {
        let home = data_home(tag);
        if t3 {
            t3_substrate::isolated_runtime_home(&home);
        }
        let init = base(&maos(), &home)
            .arg("init")
            .output()
            .expect("run maos init");
        assert!(
            init.status.success(),
            "maos init must exit 0 on a fresh home: {}",
            String::from_utf8_lossy(&init.stderr)
        );
        let mut command = base(&maos(), &home);
        if t3 {
            add_t3_environment(&mut command);
        }
        command
            .arg("run")
            .args(args)
            .env("NO_COLOR", "1")
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("spawn maos run");
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
        Self {
            home,
            child: Some(child),
            stdout,
            stderr,
        }
    }

    fn stderr(&self) -> String {
        self.stderr.lock().expect("stderr capture").clone()
    }

    fn events(&self) -> Vec<serde_json::Value> {
        self.stdout
            .lock()
            .expect("stdout capture")
            .iter()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    fn named(&self, name: &str) -> Vec<serde_json::Value> {
        self.events()
            .into_iter()
            .filter(|event| event["event"].as_str() == Some(name))
            .collect()
    }

    fn wait_for(&self, what: &str, mut ready: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + EVENT_BOUND;
        while Instant::now() < deadline {
            if ready(self) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "{what} not seen within {EVENT_BOUND:?}; stdout:\n{}\nstderr:\n{}",
            self.stdout.lock().expect("stdout capture").join("\n"),
            self.stderr()
        );
    }

    fn wait_event(&self, name: &str, section: u64) {
        self.wait_for(&format!("{name} for section {section}"), |root| {
            root.named(name)
                .iter()
                .any(|event| event["section"].as_u64() == Some(section))
        });
    }

    /// `maosctl` against this root's door (`control.json` under MAOS_HOME).
    fn ctl(&self, args: &[&str]) -> Output {
        base(&fixtures::profile_binary("maosctl"), &self.home)
            .args(args)
            .output()
            .expect("run maosctl")
    }

    /// `maosctl …` that must exit 0 and print one JSON body.
    fn ctl_json(&self, args: &[&str]) -> serde_json::Value {
        let out = self.ctl(args);
        assert!(
            out.status.success(),
            "maosctl {args:?} must exit 0; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap_or_else(|error| {
            panic!(
                "maosctl {args:?} prints a JSON body ({error}): {}",
                String::from_utf8_lossy(&out.stdout)
            )
        })
    }

    fn status_line(&self) -> String {
        let out = self.ctl(&["orchestrator", "status", "--spirit", "orchestrator"]);
        assert!(
            out.status.success(),
            "status: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn release(&self, section: u64) {
        std::fs::write(self.home.join(format!("release-section-{section}")), b"")
            .expect("write the hold release file");
    }

    fn wait_exit(&mut self) -> ExitStatus {
        let child = self.child.as_mut().expect("root still owned");
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().expect("poll the root") {
                // Let the capture threads drain the closed pipes.
                std::thread::sleep(Duration::from_millis(100));
                return status;
            }
            if started.elapsed() >= RUN_BOUND {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "the root did not exit within {RUN_BOUND:?}; stderr:\n{}",
                    self.stderr()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn lifecycle_state(&self, spirit_id: &str) -> Option<String> {
        let out = self.ctl(&["spirit", "inspect", spirit_id]);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("lifecycle_state: "))
            .map(str::to_owned)
    }

    fn connection(&self) -> Option<rusqlite::Connection> {
        let db = self.home.join("audit").join("transparency.sqlite");
        db.exists().then(|| {
            rusqlite::Connection::open_with_flags(
                db,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .expect("open the transparency log read-only")
        })
    }

    /// Every TL row, in journal order.
    fn tl(&self) -> Vec<TlRow> {
        let Some(connection) = self.connection() else {
            return Vec::new();
        };
        let mut statement = connection
            .prepare(
                "SELECT frame_id, timestamp_ns, spirit_pid, kind, intent, payload_redacted \
                 FROM transparency_log ORDER BY rowid",
            )
            .expect("prepare TL read");
        statement
            .query_map([], |row| {
                let frame_id: Vec<u8> = row.get(0)?;
                let payload: Option<Vec<u8>> = row.get(5)?;
                Ok(TlRow {
                    frame_id: hex::encode(frame_id),
                    timestamp_ns: row.get(1)?,
                    spirit_pid: row.get(2)?,
                    kind: row.get(3)?,
                    intent: row.get(4)?,
                    payload: payload
                        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .expect("query TL rows")
            .collect::<Result<_, _>>()
            .expect("read TL rows")
    }

    /// Every Approval Decision Log row, in insertion order.
    fn approvals(&self) -> Vec<Approval> {
        let Some(connection) = self.connection() else {
            return Vec::new();
        };
        let mut statement = connection
            .prepare(
                "SELECT timestamp_ns, actor, target, capability, intent, reasoning \
                 FROM approval_decision_log ORDER BY decision_id",
            )
            .expect("prepare approval read");
        statement
            .query_map([], |row| {
                Ok(Approval {
                    timestamp_ns: row.get(0)?,
                    actor: row.get(1)?,
                    target: row.get(2)?,
                    capability: row.get(3)?,
                    intent: row.get(4)?,
                    reasoning: row.get(5)?,
                })
            })
            .expect("query approval rows")
            .collect::<Result<_, _>>()
            .expect("read approval rows")
    }

    fn approvals_for(&self, capability: &str) -> Vec<Approval> {
        self.approvals()
            .into_iter()
            .filter(|row| row.capability == capability)
            .collect()
    }

    fn terminate(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(30) {
            if let Ok(Some(_)) = child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        self.terminate();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

#[derive(Debug)]
struct TlRow {
    frame_id: String,
    timestamp_ns: i64,
    spirit_pid: i64,
    kind: i64,
    intent: String,
    payload: serde_json::Value,
}

#[derive(Debug)]
struct Approval {
    timestamp_ns: i64,
    actor: String,
    target: String,
    capability: String,
    intent: String,
    reasoning: Option<String>,
}

fn sections() -> Vec<orchestrator::EpicSection> {
    let text = std::fs::read_to_string(Path::new(workspace_root()).join(EPIC)).expect("read epic");
    orchestrator::split_epic(&text, 3).expect("fixture epic splits")
}

/// Section `text` with ONE drained instruction folded in (D-19-2-02 B).
fn folded(text: &str, instruction: &str) -> String {
    format!(
        "{text}{}{instruction}",
        orchestrator::DIRECTOR_INSTRUCTION_DELIMITER
    )
}

fn position(events: &[serde_json::Value], name: &str, nth: usize) -> usize {
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| event["event"].as_str() == Some(name))
        .nth(nth)
        .map(|(at, _)| at)
        .unwrap_or_else(|| panic!("no {name} #{nth} in {events:?}"))
}

fn task_assigns(root: &Root) -> Vec<TlRow> {
    root.tl()
        .into_iter()
        .filter(|row| row.kind == KIND_TASK_ASSIGN)
        .collect()
}

fn goal(row: &TlRow) -> &str {
    row.payload["TaskAssign"]["goal"]
        .as_str()
        .unwrap_or_else(|| panic!("a task.assign row carries its goal: {row:?}"))
}

/// The `cli.subprocess.output` lines a Worker printed at `pid`.
fn worker_lines(root: &Root, pid: u64) -> Vec<String> {
    root.tl()
        .into_iter()
        .filter(|row| row.intent == "cli.subprocess.output" && row.spirit_pid as u64 == pid)
        .filter_map(|row| row.payload["line"].as_str().map(str::to_owned))
        .collect()
}

fn completion_rows(root: &Root, verb: &str) -> Vec<TlRow> {
    let prefix = format!("operator.{verb}.");
    root.tl()
        .into_iter()
        .filter(|row| row.intent.starts_with(&prefix) && row.intent.ends_with(":completed"))
        .collect()
}

fn epic_root(tag: &str) -> Root {
    Root::boot(
        tag,
        true,
        &[EPIC_TOPOLOGY, "--epic", EPIC, "--once"],
        &[(HOLD_SECTION_ENV, "1")],
    )
}

/// AC1 + AC2 + D-19-2-02 B / 03 A — three instructions queued while section
/// 1 is in flight: none is consumed in flight; section 2's goal carries the
/// first, section 3's the second, each journaled before its frame; the third
/// stays queued and the drain event says so.
#[test]
fn queued_mid_delegation_one_instruction_reaches_each_next_section_and_the_rest_stays_queued() {
    if !t3_substrate::require_t3("19-2 safe-point drain") {
        return;
    }
    let mut root = epic_root("drain");
    root.wait_event("dispatch_held_test_only", 1);

    let texts = [
        "19-2 director instruction A: prefer the staging database",
        "19-2 director instruction B: keep the changelog terse",
        "19-2 director instruction C: never dispatched",
    ];
    let mut ids = Vec::new();
    for (n, text) in texts.iter().enumerate() {
        let body = root.ctl_json(&["orchestrator", "queue", "--spirit", "orchestrator", text]);
        assert_eq!(body["spirit_id"], "orchestrator", "{body}");
        assert_eq!(body["pending"].as_u64(), Some(n as u64 + 1), "{body}");
        ids.push(body["instruction_id"].as_u64().expect("instruction_id"));
    }
    // FR20: nothing is taken while section 1's delegation is in flight.
    let status = root.status_line();
    assert!(
        status.contains(": 3/"),
        "three pending during the hold: {status}"
    );
    assert!(root.named("orchestrator_instruction_dispatched").is_empty());

    root.release(1);
    let exit = root.wait_exit();
    assert!(
        exit.success(),
        "the epic run exits 0; stderr:\n{}",
        root.stderr()
    );
    let events = root.events();
    let sections = sections();

    let dispatched = root.named("orchestrator_instruction_dispatched");
    assert_eq!(
        dispatched.len(),
        2,
        "one instruction per safe point: {dispatched:?}"
    );
    for (k, event) in dispatched.iter().enumerate() {
        assert_eq!(
            event["instruction_id"].as_u64(),
            Some(ids[k]),
            "FIFO: {event}"
        );
        assert_eq!(event["section"].as_u64(), Some(k as u64 + 2), "{event}");
        // Taken after section k+1's completion, before section k+2's frame.
        let at = position(&events, "orchestrator_instruction_dispatched", k);
        assert!(
            position(&events, "delegation_completed", k) < at,
            "after the completion"
        );
        assert!(
            at < position(&events, "delegation_routed", k + 1),
            "before the next frame"
        );
    }

    let routed = root.named("delegation_routed");
    let expected = [
        sections[0].text.clone(),
        folded(&sections[1].text, texts[0]),
        folded(&sections[2].text, texts[1]),
    ];
    assert_eq!(routed.len(), 3);
    for (event, want) in routed.iter().zip(&expected) {
        assert_eq!(event["goal"].as_str(), Some(want.as_str()), "routed goal");
    }

    let assigns = task_assigns(&root);
    assert_eq!(assigns.len(), 3, "three task.assign rows: {assigns:?}");
    for (row, want) in assigns.iter().zip(&expected) {
        assert_eq!(goal(row), want, "the journaled goal is the routed goal");
    }
    assert!(
        assigns.iter().all(|row| !goal(row).contains(texts[2])),
        "the third instruction reached no Worker"
    );

    let rows = root.approvals_for("orchestrator.dispatch");
    assert_eq!(
        rows.len(),
        2,
        "one safe-point row per drained instruction: {rows:?}"
    );
    for (k, row) in rows.iter().enumerate() {
        let frame = dispatched[k]["frame_id"].as_str().expect("frame_id");
        assert_eq!(
            frame,
            assigns[k + 1].frame_id,
            "the event names section {}'s frame",
            k + 2
        );
        assert_eq!(
            (row.actor.as_str(), row.target.as_str()),
            ("orchestrator", "orchestrator")
        );
        assert_eq!(row.intent, "safe-point");
        assert_eq!(
            row.reasoning.as_deref(),
            Some(format!("id={} section={} frame={frame}", ids[k], k + 2).as_str())
        );
        assert!(
            row.timestamp_ns <= assigns[k + 1].timestamp_ns,
            "the consumed row is journaled before its frame is delivered (CR-2)"
        );
    }

    // The Worker of section 2 received the instruction (its own echo).
    let loaded = root.named("cli_wrapper_loaded");
    assert_eq!(loaded.len(), 3);
    let worker_two = loaded[1]["spirit_pid"].as_u64().expect("spirit_pid");
    assert!(
        worker_lines(&root, worker_two)
            .iter()
            .any(|line| line == texts[0]),
        "Worker 2 echoes the drained instruction"
    );
    let worker_one = loaded[0]["spirit_pid"].as_u64().expect("spirit_pid");
    assert!(worker_lines(&root, worker_one)
        .iter()
        .all(|line| !line.contains(texts[0])));

    let drain = root
        .named("drain")
        .into_iter()
        .find(|event| event["topology"] == true)
        .expect("the topology drain event");
    assert_eq!(
        drain["orchestrator_instructions_pending"].as_u64(),
        Some(1),
        "{drain}"
    );
    for event in root.named("delegation_completed") {
        assert_eq!(
            event["orchestrator_safe_point"].as_bool(),
            Some(true),
            "{event}"
        );
    }
}

/// AC3 (FR51(c), D-19-2-05/06 A) — an instruction queued and then recalled
/// during the section-1 hold never reaches a Worker; the intent row precedes
/// the per-instruction row; an empty recall is a 200 with `recalled: []`.
#[test]
fn a_recalled_instruction_never_reaches_a_worker_and_is_journaled_before_removal() {
    if !t3_substrate::require_t3("19-2 recall") {
        return;
    }
    let mut root = epic_root("recall");
    root.wait_event("dispatch_held_test_only", 1);

    let text = "19-2 director instruction R: recalled before any safe point";
    let queued = root.ctl_json(&["orchestrator", "queue", "--spirit", "orchestrator", text]);
    let id = queued["instruction_id"].as_u64().expect("instruction_id");

    let recalled = root.ctl_json(&["orchestrator", "recall", "--spirit", "orchestrator"]);
    assert_eq!(
        recalled,
        serde_json::json!({
            "spirit_id": "orchestrator",
            "recalled": [{"instruction_id": id, "text": text}],
            "pending": 0,
        })
    );
    let status = root.status_line();
    assert!(
        status.contains(": 0/"),
        "nothing pending after the recall: {status}"
    );
    let empty = root.ctl_json(&["orchestrator", "recall", "--spirit", "orchestrator"]);
    assert_eq!(empty["recalled"], serde_json::json!([]), "{empty}");

    let unknown = root.ctl(&["orchestrator", "recall", "--spirit", "no-such-spirit"]);
    assert_eq!(unknown.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        stderr.contains("spirit_not_loaded") && stderr.contains("HTTP 404"),
        "{stderr}"
    );

    root.release(1);
    let exit = root.wait_exit();
    assert!(
        exit.success(),
        "the epic run exits 0; stderr:\n{}",
        root.stderr()
    );

    assert!(
        root.named("orchestrator_instruction_dispatched").is_empty(),
        "a recalled instruction is never dispatched"
    );
    let sections = sections();
    let routed = root.named("delegation_routed");
    assert_eq!(routed.len(), 3);
    for (event, section) in routed.iter().zip(&sections) {
        assert_eq!(
            event["goal"].as_str(),
            Some(section.text.as_str()),
            "byte-identical goals"
        );
    }
    assert!(task_assigns(&root)
        .iter()
        .all(|row| !goal(row).contains(text)));
    assert!(
        root.tl()
            .iter()
            .all(|row| row.intent != "cli.subprocess.output"
                || !row.payload["line"]
                    .as_str()
                    .unwrap_or_default()
                    .contains(text)),
        "no Worker printed the recalled text"
    );

    let rows = root.approvals_for("orchestrator.recall");
    let shape: Vec<(&str, &str, Option<&str>)> = rows
        .iter()
        .map(|row| {
            (
                row.actor.as_str(),
                row.intent.as_str(),
                row.reasoning.as_deref(),
            )
        })
        .collect();
    let removed = format!("id={id}: {text}");
    assert_eq!(
        shape,
        vec![
            ("director", "recall-requested", Some("pending=1")),
            ("director", "recall", Some(removed.as_str())),
            ("director", "recall-requested", Some("pending=0")),
        ],
        "intent row before the drain, then one row per recalled instruction (D-19-2-06 A)"
    );
    assert!(rows.iter().all(|row| row.target == "orchestrator"));
    let completions = completion_rows(&root, "orchestrator-recall");
    assert_eq!(
        completions.len(),
        2,
        "two completed recalls: {completions:?}"
    );
    assert!(
        rows[0].timestamp_ns <= completions[0].timestamp_ns,
        "intent before completion"
    );

    let drain = root
        .named("drain")
        .into_iter()
        .find(|event| event["topology"] == true)
        .expect("the topology drain event");
    assert_eq!(drain["orchestrator_instructions_pending"].as_u64(), Some(0));
}

/// AC7.3 (D-19-2-17 A) — while the Orchestrator is Paused the loop holds
/// before section 2's frame (`dispatch_paused` once, no `task.assign`), and
/// continues after resume; the pause body reads the fence back.
#[test]
fn a_paused_orchestrator_holds_dispatch_until_resume() {
    if !t3_substrate::require_t3("19-2 paused Orchestrator holds dispatch") {
        return;
    }
    let mut root = epic_root("paused");
    root.wait_event("dispatch_held_test_only", 1);

    let paused = root.ctl_json(&["pause", "orchestrator"]);
    assert_eq!(paused["lifecycle_state"], "Paused", "{paused}");
    assert_eq!(
        paused["fenced"], true,
        "the capability fence is read back: {paused}"
    );
    assert!(
        paused.get("frozen").is_none(),
        "no Worker hook acted: {paused}"
    );

    root.release(1);
    root.wait_event("dispatch_paused", 2);
    std::thread::sleep(PAUSED_WINDOW);
    assert_eq!(
        root.named("delegation_routed").len(),
        1,
        "no frame while Paused"
    );
    assert_eq!(task_assigns(&root).len(), 1, "no task.assign while Paused");

    let resumed = root.ctl_json(&["resume", "orchestrator"]);
    assert_eq!(resumed["lifecycle_state"], "Running", "{resumed}");
    assert_eq!(resumed["fenced"], false, "{resumed}");

    let exit = root.wait_exit();
    assert!(
        exit.success(),
        "the epic completes after resume; stderr:\n{}",
        root.stderr()
    );
    assert_eq!(root.named("dispatch_paused").len(), 1, "printed once");
    assert_eq!(root.named("delegation_routed").len(), 3);
    assert_eq!(task_assigns(&root).len(), 3);
}

fn butler_root(tag: &str, env: &[(&str, &str)]) -> Root {
    let mut env = env.to_vec();
    env.push(("MAOS_IDLE_FAST", "1"));
    let root = Root::boot(tag, false, &[BUTLER_MANIFEST], &env);
    root.wait_for("butler Running", |root| {
        root.lifecycle_state("butler").as_deref() == Some("Running")
    });
    root
}

fn pick(root: &Root, n: &str) -> Output {
    root.ctl(&["spirit", "pick", "butler", n, "--work-mode", "focus"])
}

fn acceptance_rows(root: &Root) -> Vec<TlRow> {
    root.tl()
        .into_iter()
        .filter(|row| row.intent == butler::ACCEPTANCE_INTENT)
        .collect()
}

/// AC4 / D-19-2-08 A — a serving Butler booted with no principal answers a
/// pick 409 `principal_unbound` and records nothing; an unknown Spirit is 404.
#[test]
fn a_pick_without_a_boot_principal_is_refused_and_records_nothing() {
    let root = butler_root("pick-unbound", &[]);
    let out = pick(&root, "1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("principal_unbound") && stderr.contains("HTTP 409"),
        "{stderr}"
    );
    assert!(out.stdout.is_empty());

    let unknown = root.ctl(&["spirit", "pick", "nobody", "1", "--work-mode", "focus"]);
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert_eq!(unknown.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("spirit_not_loaded") && stderr.contains("HTTP 404"),
        "{stderr}"
    );

    assert!(
        root.approvals_for("butler.pick").is_empty(),
        "nothing journaled"
    );
    assert!(acceptance_rows(&root).is_empty(), "no decision attested");
}

/// AC4 (D-19-2-07/08 A; deferred-work :1195) — with `MAOS_BUTLER_PRINCIPAL`
/// a director's pick reaches the RUNNING Butler: the declared action runs,
/// the decision is attested at the Butler's pid after the door's intent row,
/// and the notification is consumed (a second pick is 409).
#[test]
fn a_pick_on_a_principal_bound_serving_butler_runs_the_declared_action() {
    let root = butler_root("pick-bound", &[(PRINCIPAL_ENV, PRINCIPAL)]);
    // The pass that stores the notification raises the belief-variance halt
    // after it (`idle_pass`): its row is the "notification pending" signal.
    root.wait_for("the Butler pass's EpistemicHalt row", |root| {
        root.tl().iter().any(|row| row.kind == KIND_EPISTEMIC_HALT)
    });
    let butler_pid = root
        .tl()
        .into_iter()
        .find(|row| row.kind == KIND_EPISTEMIC_HALT)
        .map(|row| row.spirit_pid)
        .expect("the halt row");

    assert!(
        butler::DETERMINISTIC_OPTIONS[1].starts_with("reminder:"),
        "option 2 of the deterministic notification is the reminder"
    );
    let body = root.ctl_json(&["spirit", "pick", "butler", "2", "--work-mode", "focus"]);
    assert_eq!(body["spirit_id"], "butler", "{body}");
    assert_eq!(body["action"], "reminder", "{body}");
    assert_eq!(body["accepted"], true, "{body}");
    assert_eq!(body["effect"], "stub (live send v0.4)", "{body}");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Slack"),
        "{body}"
    );

    let attested = acceptance_rows(&root);
    assert_eq!(attested.len(), 1, "one attested decision: {attested:?}");
    assert_eq!(
        attested[0].spirit_pid, butler_pid,
        "attested at the Butler's pid"
    );

    let intents = root.approvals_for("butler.pick");
    assert_eq!(intents.len(), 1, "{intents:?}");
    assert_eq!(
        (intents[0].actor.as_str(), intents[0].target.as_str()),
        ("director", "butler")
    );
    assert_eq!(intents[0].intent, "pick-requested");
    assert_eq!(
        intents[0].reasoning.as_deref(),
        Some(format!("pick=2 work_mode=focus principal={PRINCIPAL}").as_str())
    );
    assert!(
        intents[0].timestamp_ns <= attested[0].timestamp_ns,
        "the door's intent row precedes the Butler's record (I2)"
    );
    assert_eq!(completion_rows(&root, "spirit-pick").len(), 1);

    let again = pick(&root, "2");
    let stderr = String::from_utf8_lossy(&again.stderr);
    assert_eq!(again.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("no_pending_notification") && stderr.contains("HTTP 409"),
        "the pick consumed the notification: {stderr}"
    );
    assert_eq!(
        acceptance_rows(&root).len(),
        1,
        "a refused pick attests nothing"
    );
}

/// D-19-2-08 A — a scene and a different `MAOS_BUTLER_PRINCIPAL` refuse the
/// boot, typed, before any pass.
#[test]
fn a_scene_with_a_different_env_principal_refuses_the_boot() {
    let home = data_home("principal-conflict");
    let out = base(&maos(), &home)
        .args([
            "run",
            BUTLER_MANIFEST,
            "--clock",
            "compressed:21d",
            "--acceptance",
        ])
        .arg(Path::new(workspace_root()).join(SCENE_FIXTURE))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", SCENE_CASSETTE)
        .env(PRINCIPAL_ENV, "director-grace")
        .output()
        .expect("spawn maos run");
    let _ = std::fs::remove_dir_all(&home);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "the boot is refused: {stderr}");
    assert!(
        stderr.contains(&format!(
            "MAOS_BUTLER_PRINCIPAL `director-grace` differs from the scene's principal `{PRINCIPAL}`"
        )),
        "{stderr}"
    );
    let events: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(
        events
            .iter()
            .all(|event| event["event"] != "acceptance_day"),
        "no scene day ran: {events:?}"
    );
}
