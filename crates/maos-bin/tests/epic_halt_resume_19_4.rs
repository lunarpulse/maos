#![forbid(unsafe_code)]

//! Story 19-4 (AC2, AC3, AC5) — the judged ambiguity halt, the in-boot hold
//! and the door resolution of an `--epic` run, end to end through the
//! production binaries (`maos init` → `maos run … --epic … --once` with the
//! door bound → `maosctl halt resolve`), each run on a fresh data home.
//!
//! * The judged runs replay the committed founder-loop cassettes strictly
//!   (`MAOS_INFERENCE_MODE=replay`, `MAOS_REPLAY_STRICT=1`): the ambiguous one
//!   halts before section 2 through the kernel's `dispatch_ambiguity` rule, the
//!   clear-judgement twin never halts (AC2.6's proven-red twin).
//! * The hold vectors plant the halt with the 19-1 debug plant
//!   (`MAOS_TEST_ONLY_HALT_BEFORE_SECTION`) and leave the inference mode unset
//!   (no judgement, no `dispatch_judged`), so they pin the hold alone:
//!   `provided-context` resumes with the clarification, `accepted-halt` and an
//!   expired bound are the 19-1 stop (AC5.1, AC5.4).
//! * Every run spawning a Worker needs the signed T3 substrate (`require_t3`:
//!   a named local skip, never a green skip on a declared-capable runner). The
//!   malformed-bound refusal fires before any Worker and runs everywhere.
//!
//! Observables are the root's stdout events, the door's replies and the rows
//! the run wrote — never state this test set itself. Waits are bounded polls
//! on an observable (a door row, the child's exit), never a fixed sleep for
//! ordering.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
/// D-19-4-04 A — three `### Story` sections, section 2 under-specified.
const EPIC: &str = "tests/fixtures/j1/founder-loop-epic.md";
/// s1 clear, s2 ambiguous (> 0.5), s3 clear; section 2 is never re-judged.
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j1/founder-loop.json";
/// The clear-judgement twin: section 2 judged ≤ 0.5.
const CLEAR_TWIN: &str = "crates/maos-journey-test/cassettes/j1/founder-loop.clear-twin.json";
const HALT_PLANT_ENV: &str = "MAOS_TEST_ONLY_HALT_BEFORE_SECTION";
const HOLD_ENV: &str = "MAOS_EPIC_HALT_HOLD_SECS";
/// The manifest rule (`spirits/orchestrator/manifest.toml`) halts above it.
const AMBIGUITY_THRESHOLD: f64 = 0.5;
/// A hold far longer than any wait below: a run that exits inside the wait
/// was released by the resolution, never by the bound.
const LONG_HOLD_SECS: &str = "900";
/// The bound on one whole T3 epic run (three Workers) after the resolution.
const RUN_WAIT: Duration = Duration::from_secs(100);
/// The bound on the halt reaching the door (section 1's Worker runs first).
const HALT_WAIT: Duration = Duration::from_secs(100);
/// `FrameKind::TaskAssign` / `TaskComplete` / `EpistemicHalt` (TL codes).
const KIND_TASK_ASSIGN: i64 = 0;
const KIND_TASK_COMPLETE: i64 = 1;
const KIND_EPISTEMIC_HALT: i64 = 3;
const CLARIFICATION: &str = "faster = the nightly import of 1e6 rows finishes in under 10 minutes";

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
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

/// A clean-environment command over one data home (`HOME` = `MAOS_HOME`, the
/// door's `control.json` lives there), from the workspace root.
fn command_in(home: &Path, program: &Path) -> Command {
    let mut command = Command::new(program);
    command.env_clear();
    add_t3_environment(&mut command);
    command
        .env("PATH", path_with_target_debug())
        .env("HOME", t3_substrate::isolated_runtime_home(home))
        .env("XDG_DATA_HOME", home)
        .env("MAOS_HOME", home)
        .env("NO_COLOR", "1")
        .current_dir(workspace_root());
    command
}

fn maos(home: &Path) -> Command {
    command_in(home, Path::new(env!("CARGO_BIN_EXE_maos")))
}

fn maosctl(home: &Path, args: &[&str]) -> Output {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    command_in(home, &path)
        .args(args)
        .output()
        .expect("run maosctl")
}

/// `maos init` in the data home: the door's `control.json`, so the `maos run`
/// below serves `maosctl` during its `--once` topology load.
fn init_door(home: &Path) {
    let init = maos(home).arg("init").output().expect("maos init");
    assert!(
        init.status.success() && home.join("control.json").is_file(),
        "maos init must exit 0 and write control.json; stderr:\n{}",
        String::from_utf8_lossy(&init.stderr)
    );
}

fn drain(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    });
    buffer
}

/// One `maos run <topology> --epic EPIC --once` in flight, its pipes drained
/// while it runs (a full pipe never stalls the root).
struct EpicRun {
    child: std::process::Child,
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
}

impl EpicRun {
    fn spawn(home: &Path, env: &[(&str, &str)]) -> Self {
        let mut command = maos(home);
        command.args(["run", EPIC_TOPOLOGY, "--epic", EPIC, "--once"]);
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run --epic");
        let stdout = drain(child.stdout.take().expect("stdout piped"));
        let stderr = drain(child.stderr.take().expect("stderr piped"));
        Self {
            child,
            stdout,
            stderr,
        }
    }

    fn stderr(&self) -> String {
        self.stderr
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Wait (bounded) for the root to exit; a run still alive at the bound is
    /// killed and the vector fails naming `what`.
    fn finish(
        mut self,
        bound: Duration,
        what: &str,
    ) -> (ExitStatus, Vec<serde_json::Value>, String) {
        let deadline = Instant::now() + bound;
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll maos run") {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!(
                    "maos run did not exit within {bound:?} ({what}); stderr:\n{}",
                    self.stderr()
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        // Reaped: the write ends are closed, so the drainers reach EOF.
        let deadline = Instant::now() + Duration::from_secs(10);
        while Arc::strong_count(&self.stdout) > 1 || Arc::strong_count(&self.stderr) > 1 {
            assert!(
                Instant::now() < deadline,
                "the pipe drainers never reached EOF"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let stdout = self
            .stdout
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let events = stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        (status, events, self.stderr())
    }
}

impl Drop for EpicRun {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Poll the door (bounded) until the Orchestrator has exactly one raised
/// halt; returns its id. The run's death first fails naming its stderr.
fn await_raised_halt(home: &Path, run: &mut EpicRun) -> String {
    let deadline = Instant::now() + HALT_WAIT;
    loop {
        if let Some(status) = run.child.try_wait().expect("poll maos run") {
            panic!(
                "maos run exited ({status}) before its halt reached the door; stderr:\n{}",
                run.stderr()
            );
        }
        let list = maosctl(home, &["halt", "list", "--spirit", "orchestrator"]);
        if list.status.success() {
            let raised: Vec<String> = String::from_utf8_lossy(&list.stdout)
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|row| row["record"] == "raised")
                .filter_map(|row| row["halt_id"].as_str().map(str::to_owned))
                .collect();
            if !raised.is_empty() {
                assert_eq!(
                    raised.len(),
                    1,
                    "one raised halt on the Orchestrator: {raised:?}"
                );
                return raised.into_iter().next().unwrap_or_default();
            }
        }
        assert!(
            Instant::now() < deadline,
            "no halt reached the door within {HALT_WAIT:?}; stderr:\n{}",
            run.stderr()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn resolve(home: &Path, halt_id: &str, kind_args: &[&str]) {
    let mut args = vec!["halt", "resolve", halt_id, "--spirit", "orchestrator"];
    args.extend_from_slice(kind_args);
    let out = maosctl(home, &args);
    assert!(
        out.status.success(),
        "door resolve {kind_args:?} must exit 0; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn named<'a>(events: &'a [serde_json::Value], name: &str) -> Vec<&'a serde_json::Value> {
    events
        .iter()
        .filter(|event| event["event"].as_str() == Some(name))
        .collect()
}

/// Index of the first event matching `pred` (stdout order).
fn position(events: &[serde_json::Value], pred: impl Fn(&serde_json::Value) -> bool) -> usize {
    events
        .iter()
        .position(pred)
        .unwrap_or_else(|| panic!("no matching event: {events:?}"))
}

fn orchestrator_pid(events: &[serde_json::Value]) -> u64 {
    let loaded: Vec<u64> = named(events, "spirit_loaded")
        .into_iter()
        .filter(|event| event["spirit_id"].as_str() == Some("orchestrator"))
        .filter_map(|event| event["pid"].as_u64())
        .collect();
    assert_eq!(loaded.len(), 1, "one Orchestrator admission: {events:?}");
    assert_ne!(loaded[0], 0, "the Orchestrator is admitted at a real pid");
    loaded[0]
}

fn open_tl(home: &Path) -> Option<rusqlite::Connection> {
    let db = home.join("audit").join("transparency.sqlite");
    db.exists().then(|| {
        rusqlite::Connection::open_with_flags(
            db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .expect("open transparency log")
    })
}

/// `(frame_id hex, spirit_pid, intent, payload JSON)` of every TL row of
/// `kind`, in journal order. A run refused before the log existed has none.
fn tl_rows(home: &Path, kind: i64) -> Vec<(String, i64, String, serde_json::Value)> {
    let Some(connection) = open_tl(home) else {
        return Vec::new();
    };
    let mut statement = connection
        .prepare(
            "SELECT frame_id, spirit_pid, intent, payload_redacted FROM transparency_log \
             WHERE kind = ?1 ORDER BY timestamp_ns, rowid",
        )
        .expect("prepare TL query");
    statement
        .query_map([kind], |row| {
            let frame_id: Vec<u8> = row.get(0)?;
            let payload: Vec<u8> = row.get(3)?;
            Ok((
                hex::encode(frame_id),
                row.get(1)?,
                row.get(2)?,
                serde_json::from_slice(&payload).unwrap_or(serde_json::Value::Null),
            ))
        })
        .expect("query TL rows")
        .collect::<Result<_, _>>()
        .expect("read TL rows")
}

/// `(intent, reasoning)` of every `halt.resolve` approval decision.
fn halt_resolutions(home: &Path) -> Vec<(String, String)> {
    let connection = open_tl(home).expect("the run created its transparency log");
    let mut statement = connection
        .prepare(
            "SELECT intent, COALESCE(reasoning, '') FROM approval_decision_log \
             WHERE capability = 'halt.resolve' AND decision = 1 ORDER BY timestamp_ns, rowid",
        )
        .expect("prepare approval query");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query approvals")
        .collect::<Result<_, _>>()
        .expect("read approvals")
}

/// The three goals a resumed run dispatches: each section's text byte for
/// byte, section 2's followed by the root's clarification delimiter and the
/// operator's `provided-context` text (D-19-4-05 A).
fn expected_goals() -> Vec<String> {
    let text = std::fs::read_to_string(Path::new(workspace_root()).join(EPIC)).expect("read epic");
    orchestrator::split_epic(&text, 3)
        .expect("the fixture epic splits into three sections")
        .into_iter()
        .map(|section| match section.ordinal {
            2 => format!(
                "{}{}{CLARIFICATION}",
                section.text,
                maos_bin::orchestrator_root::HALT_CLARIFICATION_DELIMITER
            ),
            _ => section.text,
        })
        .collect()
}

/// The goal of every routed delegation, in dispatch order.
fn routed_goals(events: &[serde_json::Value]) -> Vec<String> {
    named(events, "delegation_routed")
        .into_iter()
        .map(|event| {
            event["goal"]
                .as_str()
                .unwrap_or_else(|| panic!("a routed frame carries its goal: {event}"))
                .to_owned()
        })
        .collect()
}

/// The whole epic was dispatched — three frames, three completions, each at
/// a safe point — and the run drained.
fn assert_three_frames_completed(events: &[serde_json::Value]) {
    assert_eq!(named(events, "delegation_routed").len(), 3, "{events:?}");
    let completed = named(events, "delegation_completed");
    assert_eq!(completed.len(), 3, "{events:?}");
    for event in completed {
        assert_eq!(event["result"], "completed", "{event}");
        assert_eq!(
            event["orchestrator_safe_point"].as_bool(),
            Some(true),
            "{event}"
        );
    }
    assert!(
        named(events, "drain")
            .iter()
            .any(|event| event["topology"].as_bool() == Some(true)),
        "the run drained through the topology seam: {events:?}"
    );
}

/// The section a run stopped on holds: one frame, `dispatch_halted` naming
/// sections 2–3 once, no resume, one `task.assign`, a normal drain.
fn assert_stopped_before_section_two(
    status: ExitStatus,
    events: &[serde_json::Value],
    stderr: &str,
    home: &Path,
) {
    assert!(
        status.success(),
        "the 19-1 stop drains normally (exit 0); stderr:\n{stderr}"
    );
    let halted = named(events, "dispatch_halted");
    assert_eq!(halted.len(), 1, "dispatch_halted printed once: {events:?}");
    assert_eq!(
        halted[0]["undispatched_sections"],
        serde_json::json!([2, 3])
    );
    assert!(named(events, "dispatch_resumed").is_empty(), "{events:?}");
    assert_eq!(named(events, "delegation_routed").len(), 1, "{events:?}");
    assert_eq!(
        tl_rows(home, KIND_TASK_ASSIGN).len(),
        1,
        "no task.assign after the stop"
    );
}

/// AC2.3 + AC5.1 + AC5.3 + AC3 — the judged epic under strict replay: section
/// 2 is judged ambiguous, the KERNEL halts the Orchestrator before frame 2
/// (one frame), the hold keeps section 2 back until the operator answers over
/// the door, `provided-context` resumes it with the clarification in its goal,
/// section 3 follows, and the drain's digest cites the halt it came through in
/// `resolved_halts` by the pre-halt frame id `dispatch_judged` named.
#[test]
fn an_ambiguous_section_halts_resolves_over_the_door_and_resumes_with_the_clarification() {
    if !t3_substrate::require_t3("epic ambiguity halt resolved over the door") {
        return;
    }
    let home = tempfile::TempDir::new().expect("data home");
    init_door(home.path());
    let mut run = EpicRun::spawn(
        home.path(),
        &[
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_STRICT", "1"),
            ("MAOS_REPLAY_CASSETTE", CASSETTE),
            (HOLD_ENV, LONG_HOLD_SECS),
        ],
    );
    let halt_id = await_raised_halt(home.path(), &mut run);
    resolve(
        home.path(),
        &halt_id,
        &["--kind", "provided-context", "--text", CLARIFICATION],
    );
    let (status, events, stderr) = run.finish(RUN_WAIT, "after the provided-context resolution");
    assert!(
        status.success(),
        "the resumed epic exits 0; stderr:\n{stderr}"
    );

    // AC2.3 — one judgement per NEW section, in order; only section 2 halts.
    let judged = named(&events, "dispatch_judged");
    assert_eq!(
        judged
            .iter()
            .map(|event| event["section"].as_u64())
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)],
        "each section judged once, section 2 never re-judged after the resolution: {events:?}"
    );
    for event in &judged {
        assert_eq!(event["tag"], "dispatch_ambiguity", "{event}");
        let value = event["value"].as_f64().expect("a judged value");
        if event["section"] == 2 {
            assert!(
                value > AMBIGUITY_THRESHOLD,
                "section 2 judged ambiguous: {event}"
            );
            assert_eq!(event["halt_id"].as_str(), Some(halt_id.as_str()), "{event}");
        } else {
            assert!(value <= AMBIGUITY_THRESHOLD, "{event}");
            assert!(
                event["halt_id"].is_null(),
                "a clear section raises no halt: {event}"
            );
        }
    }
    let halt_frame_id = judged[1]["halt_frame_id"]
        .as_str()
        .expect("the halted judgement names the halt's TL frame")
        .to_owned();

    // AC2.3 / AC1.2 — the halt lands within 2 frames (1 under D-02 A); the hold
    // prints `dispatch_halted` once and the resume follows it.
    let halted = named(&events, "dispatch_halted");
    assert_eq!(halted.len(), 1, "dispatch_halted printed once: {events:?}");
    assert_eq!(
        halted[0]["undispatched_sections"],
        serde_json::json!([2, 3])
    );
    let resumed = named(&events, "dispatch_resumed");
    assert_eq!(resumed.len(), 1, "{events:?}");
    assert_eq!(resumed[0]["section"], 2);
    assert_eq!(resumed[0]["halt_ids"], serde_json::json!([halt_id]));
    assert_eq!(resumed[0]["provided_context"].as_bool(), Some(true));
    let is = |name: &'static str| move |event: &serde_json::Value| event["event"] == name;
    let judged_2 = position(&events, |event| {
        event["event"] == "dispatch_judged" && event["section"] == 2
    });
    let halted_at = position(&events, is("dispatch_halted"));
    let resumed_at = position(&events, is("dispatch_resumed"));
    let routed_at: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event["event"] == "delegation_routed")
        .map(|(at, _)| at)
        .collect();
    assert_eq!(routed_at.len(), 3, "{events:?}");
    assert!(
        routed_at[0] < judged_2 && judged_2 < halted_at && halted_at < resumed_at,
        "frame 1, then the section-2 judgement, the halt and the resume: {events:?}"
    );
    assert!(
        resumed_at < routed_at[1],
        "no frame for section 2 before the resolution released the hold: {events:?}"
    );

    // AC5.1 — section 2's goal is its text plus the clarification; 1 and 3
    // are their text byte for byte.
    assert_eq!(routed_goals(&events), expected_goals(), "{events:?}");
    assert_three_frames_completed(&events);

    // AC2.5 (D-08 A) — the kernel journaled ONE raise row for the halt, at the
    // Orchestrator's admitted pid, under the scalar tag, with the frame id the
    // judgement named; the door journaled the resolution.
    let orchestrator = orchestrator_pid(&events);
    let raise: Vec<_> = tl_rows(home.path(), KIND_EPISTEMIC_HALT)
        .into_iter()
        .filter(|(_, _, _, payload)| {
            payload["halt_id"].as_str() == Some(halt_id.as_str())
                && payload["terminal_state"].is_null()
        })
        .collect();
    assert_eq!(raise.len(), 1, "one raise row for the halt: {raise:?}");
    let (frame_id, pid, intent, _) = &raise[0];
    assert_eq!(
        *frame_id, halt_frame_id,
        "dispatch_judged names the raise row"
    );
    assert_eq!(
        *pid as u64, orchestrator,
        "the halt sits on the admitted Orchestrator"
    );
    assert_eq!(
        intent, "dispatch_ambiguity",
        "journaled under the scalar tag (D-08 A)"
    );
    let resolutions = halt_resolutions(home.path());
    assert!(
        resolutions
            .iter()
            .any(|(intent, reasoning)| intent == "provided_context"
                && reasoning.starts_with(&format!("halt={halt_id}: provided_context: "))),
        "the door journaled the provided-context resolution: {resolutions:?}"
    );

    // AC3 + AC5.3 — the drain's digest: every completion cites a real
    // TaskComplete row (≥ 3), and the halt is RESOLVED — cited by the pre-halt
    // frame id — never open.
    let digests = named(&events, "morning_digest");
    assert_eq!(
        digests.len(),
        1,
        "an --epic drain renders one digest: {events:?}"
    );
    let digest = &digests[0]["digest"];
    let task_complete: std::collections::BTreeSet<String> =
        tl_rows(home.path(), KIND_TASK_COMPLETE)
            .into_iter()
            .map(|(frame_id, ..)| frame_id)
            .collect();
    let completed: Vec<&str> = digest["completed"]
        .as_array()
        .expect("completed[]")
        .iter()
        .filter_map(|entry| entry["source_log_ref"].as_str())
        .collect();
    assert!(completed.len() >= 3, "≥ 3 cited completions: {digest}");
    for reference in &completed {
        assert!(
            task_complete.contains(*reference),
            "completion ref {reference} resolves to a TaskComplete row: {digest}"
        );
    }
    let resolved = digest["resolved_halts"]
        .as_array()
        .expect("resolved_halts[]");
    assert!(
        resolved
            .iter()
            .any(|entry| entry["halt_id"].as_str() == Some(halt_id.as_str())
                && entry["source_log_ref"].as_str() == Some(halt_frame_id.as_str())
                && entry["resolution"] == "provided_context"),
        "the post-resume digest cites the pre-halt frame in resolved_halts: {digest}"
    );
    assert!(
        !digest["open_halts"]
            .as_array()
            .expect("open_halts[]")
            .iter()
            .any(|entry| entry["source_log_ref"].as_str() == Some(halt_frame_id.as_str())),
        "a resolved halt is never reported open: {digest}"
    );
}

/// AC2.6 — the proven-red twin: the clear-judgement cassette (section 2 ≤ 0.5)
/// raises no halt, the hold never engages, and all three frames dispatch.
#[test]
fn a_clear_judgement_twin_never_halts_and_dispatches_three_frames() {
    if !t3_substrate::require_t3("epic clear-judgement twin") {
        return;
    }
    let home = tempfile::TempDir::new().expect("data home");
    let run = EpicRun::spawn(
        home.path(),
        &[
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_STRICT", "1"),
            ("MAOS_REPLAY_CASSETTE", CLEAR_TWIN),
            (HOLD_ENV, LONG_HOLD_SECS),
        ],
    );
    let (status, events, stderr) = run.finish(RUN_WAIT, "the clear-judgement epic");
    assert!(status.success(), "stderr:\n{stderr}");
    let judged = named(&events, "dispatch_judged");
    assert_eq!(judged.len(), 3, "{events:?}");
    for event in &judged {
        assert!(
            event["value"].as_f64().expect("a judged value") <= AMBIGUITY_THRESHOLD,
            "{event}"
        );
        assert!(event["halt_id"].is_null(), "{event}");
    }
    assert!(named(&events, "dispatch_halted").is_empty(), "{events:?}");
    assert!(named(&events, "dispatch_resumed").is_empty(), "{events:?}");
    assert_three_frames_completed(&events);
    assert!(
        tl_rows(home.path(), KIND_EPISTEMIC_HALT)
            .iter()
            .all(|(_, _, intent, _)| intent != "dispatch_ambiguity"),
        "no dispatch_ambiguity halt row"
    );
}

/// AC5.1 — the hold alone (planted halt, inference unset ⇒ no judgement): a
/// `provided-context` resolution over the door releases section 2 with the
/// clarification appended to its goal, and the epic completes.
#[test]
fn a_planted_halt_resolved_with_context_resumes_section_two() {
    if !t3_substrate::require_t3("epic hold resumed with provided context") {
        return;
    }
    let home = tempfile::TempDir::new().expect("data home");
    init_door(home.path());
    let mut run = EpicRun::spawn(
        home.path(),
        &[(HALT_PLANT_ENV, "2"), (HOLD_ENV, LONG_HOLD_SECS)],
    );
    let halt_id = await_raised_halt(home.path(), &mut run);
    resolve(
        home.path(),
        &halt_id,
        &["--kind", "provided-context", "--text", CLARIFICATION],
    );
    let (status, events, stderr) = run.finish(RUN_WAIT, "after the provided-context resolution");
    assert!(status.success(), "stderr:\n{stderr}");
    assert!(
        named(&events, "dispatch_judged").is_empty(),
        "unset inference mode = no judgement, no event (ADR-064): {events:?}"
    );
    assert_eq!(named(&events, "dispatch_halted").len(), 1, "{events:?}");
    let resumed = named(&events, "dispatch_resumed");
    assert_eq!(resumed.len(), 1, "{events:?}");
    assert_eq!(resumed[0]["section"], 2);
    assert_eq!(resumed[0]["halt_ids"], serde_json::json!([halt_id]));
    assert_eq!(resumed[0]["provided_context"].as_bool(), Some(true));
    assert_eq!(routed_goals(&events), expected_goals(), "{events:?}");
    assert_three_frames_completed(&events);
}

/// AC5.1 — `accepted-halt` over the door releases the hold into the 19-1
/// stop at once (the run exits long before the 900 s bound).
#[test]
fn an_accepted_halt_releases_the_hold_into_the_19_1_stop() {
    if !t3_substrate::require_t3("epic hold released by accepted-halt") {
        return;
    }
    let home = tempfile::TempDir::new().expect("data home");
    init_door(home.path());
    let mut run = EpicRun::spawn(
        home.path(),
        &[(HALT_PLANT_ENV, "2"), (HOLD_ENV, LONG_HOLD_SECS)],
    );
    let halt_id = await_raised_halt(home.path(), &mut run);
    resolve(home.path(), &halt_id, &["--kind", "accepted-halt"]);
    let (status, events, stderr) = run.finish(RUN_WAIT, "after the accepted-halt resolution");
    assert_stopped_before_section_two(status, &events, &stderr, home.path());
}

/// AC5.4 at the root — the resolve step removed: the hold expires at its bound
/// and the run takes the 19-1 stop (`dispatch_halted`, no resume, one frame).
#[test]
fn an_unresolved_hold_expires_into_the_19_1_stop() {
    if !t3_substrate::require_t3("epic hold expiring at its bound") {
        return;
    }
    let home = tempfile::TempDir::new().expect("data home");
    init_door(home.path());
    let run = EpicRun::spawn(home.path(), &[(HALT_PLANT_ENV, "2"), (HOLD_ENV, "2")]);
    let (status, events, stderr) = run.finish(RUN_WAIT, "the expiring hold");
    assert_stopped_before_section_two(status, &events, &stderr, home.path());
}

/// AC5.2 — a malformed `MAOS_EPIC_HALT_HOLD_SECS` is refused typed before any
/// frame (never read as 0 or as "hold forever").
#[test]
fn a_malformed_hold_bound_is_refused_before_any_emit() {
    let home = tempfile::TempDir::new().expect("data home");
    let run = EpicRun::spawn(home.path(), &[(HOLD_ENV, "ten")]);
    let (status, events, stderr) = run.finish(Duration::from_secs(120), "the malformed bound");
    assert!(
        !status.success(),
        "a malformed hold bound must refuse the run; events: {events:?}"
    );
    assert!(
        stderr.contains("MAOS_EPIC_HALT_HOLD_SECS"),
        "the refusal names the variable; stderr:\n{stderr}"
    );
    assert!(named(&events, "delegation_routed").is_empty(), "{events:?}");
    assert!(
        tl_rows(home.path(), KIND_TASK_ASSIGN).is_empty(),
        "no task.assign before the refusal"
    );
}

/// Fast deterministic race vectors: real halt invocation/resolver/memory and
/// the production hold/judgement seams, without subprocesses or networking.
mod races {
    use super::*;
    use maos_bin::orchestrator_root as root;
    use maos_domain::halt::{HaltReceipt, HaltResolver, HaltState, Resolution};
    use maos_domain::ports::{EpistemicScalarPort, InferencePort};
    use maos_kernel_core::halt::{
        invoke_halt, HaltRegistry, KernelHaltResolver, OutputMarkerRegistry,
    };
    use maos_kernel_core::iac::TransparencyLogAdapter;
    use maos_kernel_core::journal::JournalAdapter;
    use maos_kernel_core::memory::{
        MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
    };
    use maos_kernel_core::scheduler::{
        control_block::{ScbLifecycleState, SpiritManifestBundle},
        SpiritSchedulerAdapter,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio_util::sync::CancellationToken;

    struct Fixture {
        tl: Arc<TransparencyLogAdapter>,
        journal: JournalAdapter,
        registry: Arc<HaltRegistry>,
        memory: Arc<MemoryManagerAdapter>,
        resolver: KernelHaltResolver,
        scheduler: SpiritSchedulerAdapter,
        _dir: tempfile::TempDir,
        tl_uri: String,
    }

    impl Fixture {
        fn new() -> Arc<Self> {
            use maos_kernel_core::capability as cap;
            cap::cap_tokens::init_monotonic_base();
            let dir = tempfile::tempdir().unwrap();
            let tl_uri = format!(
                "file:{}?mode=memory&cache=shared",
                dir.path().join("tl").display()
            );
            let tl = Arc::new(TransparencyLogAdapter::open(Path::new(&tl_uri), 194).unwrap());
            let db = dir.path().join("memory.db");
            let memory = Arc::new(MemoryManagerAdapter::new(
                Arc::new(PrivateMemoryStore::new(dir.path().join("private"), 4096)),
                Arc::new(SharedMemoryStore::open(&db).unwrap()),
                Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
                Arc::clone(&tl),
            ));
            let registry = Arc::new(HaltRegistry::new());
            let capability = Arc::new(cap::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::security::crypto::RingCryptoProvider),
                cap::cap_tokens::Ed25519SigningKey::new([0; 32]),
                194,
                Arc::new(cap::cap_policy::PolicyTable::new()),
                cap::cap_audit::channel().0,
                cap::cap_quota::CapQuotaTracker::new(),
                Arc::new(cap::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ));
            let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
            let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
            let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
                Arc::clone(&mailbox),
                Arc::clone(&tl),
            ));
            let working = Arc::new(
                cap::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
                    Arc::clone(&capability),
                    Arc::clone(&registry),
                ),
            );
            let resolver = KernelHaltResolver::new(
                Arc::clone(&registry),
                Arc::clone(&tl),
                Arc::new(OutputMarkerRegistry::new()),
                mailbox,
                194,
                Arc::clone(&memory),
                working,
            );
            let scheduler = SpiritSchedulerAdapter::new(
                Arc::clone(&tl),
                capability,
                Arc::clone(&memory),
                iac,
                Arc::clone(&registry),
                metrics,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            Arc::new(Self {
                tl,
                journal: JournalAdapter::open(&dir.path().join("journal.ndjson")).unwrap(),
                registry,
                memory,
                resolver,
                scheduler,
                _dir: dir,
                tl_uri,
            })
        }

        fn halt(&self, pid: u32, name: &str) -> HaltReceipt {
            let payload = maos_domain::frame::EpistemicHaltPayload::new(
                name.into(),
                "dispatch_ambiguity".into(),
                0.8,
                Some(0.5),
                "test-policy".into(),
                "test-judgement".into(),
            )
            .unwrap();
            invoke_halt(
                &self.tl,
                &self.journal,
                &self.registry,
                payload,
                pid,
                "orchestrator",
                194,
            )
            .unwrap()
        }

        fn context(&self, receipt: &HaltReceipt, text: &str) {
            self.resolver
                .resolve(
                    &receipt.halt_id,
                    Resolution::provided_context(text).unwrap(),
                )
                .unwrap();
            self.approve(receipt, &Resolution::provided_context(text).unwrap());
        }

        fn approve(&self, receipt: &HaltReceipt, resolution: &Resolution) {
            use maos_domain::halt::HaltJournal;
            self.tl
                .journal_halt_resolution("director", "orchestrator", &receipt.halt_id, resolution)
                .unwrap();
        }

        fn stop(&self, pid: u32, receipt: &HaltReceipt) -> serde_json::Value {
            root::judged_dispatch_stop(&self.registry, pid, Some(pid), 2, 3, Some(receipt)).unwrap()
        }

        async fn hold(
            &self,
            pid: u32,
            stop: &serde_json::Value,
            shutdown: &CancellationToken,
        ) -> Option<String> {
            root::hold_for_resolution(
                &self.tl,
                &self.registry,
                &self.memory,
                || Some(pid),
                stop,
                2,
                "section".into(),
                Duration::from_secs(1),
                shutdown,
            )
            .await
        }
    }

    thread_local! {
        static PROVIDERS: Mutex<Vec<String>> = const { Mutex::new(Vec::new()) };
    }
    struct Replay;
    impl InferencePort for Replay {
        fn complete(
            &self,
            request: maos_domain::ports::inference::InferenceRequest,
        ) -> Result<
            maos_domain::ports::inference::InferenceResponse,
            maos_domain::ports::inference::InferenceError,
        > {
            use maos_domain::ports::inference::*;
            PROVIDERS
                .with(|providers| providers.lock().unwrap().push(request.provider_id.unwrap()));
            Ok(InferenceResponse {
                text: r#"{"dispatch_ambiguity":0.31}"#.into(),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "test".into(),
                    endpoint_url: "test://".into(),
                    model_id: None,
                },
            })
        }
    }

    struct Scalar {
        fixture: std::sync::Weak<Fixture>,
        pid: u32,
        threshold: f64,
        calls: Arc<AtomicUsize>,
        name: &'static str,
    }
    impl EpistemicScalarPort for Scalar {
        fn write_scalar(
            &self,
            _: u32,
            _: &str,
            _: &str,
            value: f64,
            _: &str,
        ) -> Result<Option<HaltReceipt>, maos_domain::ports::epistemic_scalar::ScalarPortError>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let fixture = self.fixture.upgrade().ok_or_else(|| {
                maos_domain::ports::epistemic_scalar::ScalarPortError::Backend(
                    "test fixture dropped".into(),
                )
            })?;
            Ok((value > self.threshold).then(|| fixture.halt(self.pid, self.name)))
        }
    }

    fn emitter(scalar: Arc<dyn EpistemicScalarPort>) -> orchestrator::Orchestrator {
        use maos_domain::invariants::i1::{CapabilityToken, TokenId};
        let binding = orchestrator::OrchestratorInferenceBinding::per_pass(
            "test".into(),
            Arc::new(|pid, _| Ok(CapabilityToken::new(TokenId([19; 16]), pid, 0, [0; 64]))),
            Arc::new(|| Some(19)),
        );
        orchestrator::Orchestrator::new("orchestrator")
            .with_scalar_port(scalar)
            .with_deferred_inference_port(Arc::new(Replay), Arc::new(Mutex::new(Some(binding))))
    }

    fn section() -> orchestrator::EpicSection {
        orchestrator::split_epic("### Story 1: test\nMake it faster.\n", 1)
            .unwrap()
            .remove(0)
    }

    #[tokio::test(start_paused = true)]
    async fn judged_then_resolved_before_stop_consumes_that_halt() {
        let f = Fixture::new();
        let o = emitter(Arc::new(Scalar {
            fixture: Arc::downgrade(&f),
            pid: 19,
            threshold: 0.2,
            calls: Arc::new(AtomicUsize::new(0)),
            name: "judged-before-stop",
        }));
        let receipt = root::judge_section(&o, &section()).unwrap().unwrap();
        f.context(&receipt, "the answer");
        assert!(root::dispatch_stop(&f.registry, 19, Some(19), 2, 3).is_none());
        let stop = f.stop(19, &receipt);
        assert_eq!(
            stop["halt_ids"],
            serde_json::json!([receipt.halt_id.as_str()])
        );
        assert_eq!(
            f.hold(19, &stop, &CancellationToken::new()).await,
            Some(format!(
                "section{}the answer",
                root::HALT_CLARIFICATION_DELIMITER
            )),
        );
        let terminated = f.halt(19, "accepted-before-stop");
        f.resolver
            .resolve(&terminated.halt_id, Resolution::AcceptedHalt)
            .unwrap();
        assert!(f
            .hold(19, &f.stop(19, &terminated), &CancellationToken::new())
            .await
            .is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn a_halt_raised_and_resolved_between_polls_keeps_its_context() {
        let f = Fixture::new();
        let old = f.halt(19, "earlier-section");
        f.context(&old, "not this section");
        let a = f.halt(19, "hold-a");
        let stop = f.stop(19, &a);
        let shutdown = CancellationToken::new();
        let hold = f.hold(19, &stop, &shutdown);
        tokio::pin!(hold);
        assert!(futures_util::poll!(&mut hold).is_pending());
        let b = f.halt(19, "between-polls-b");
        f.context(&b, "answer B");
        let other = f.halt(20, "other-spirit");
        f.context(&other, "not our spirit");
        f.context(&a, "answer A");
        tokio::time::advance(Duration::from_millis(100)).await;
        let goal = hold.await.unwrap();
        assert!(
            goal.contains("answer A") && goal.contains("answer B"),
            "{goal}"
        );
        assert_eq!(goal.matches(root::HALT_CLARIFICATION_DELIMITER).count(), 2);
        assert!(!goal.contains("not this section") && !goal.contains("not our spirit"));
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_at_entry_and_deadline_before_empty_pending_stop() {
        let f = Fixture::new();
        let a = f.halt(19, "cancelled-at-entry");
        f.context(&a, "too late");
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        assert!(f.hold(19, &f.stop(19, &a), &shutdown).await.is_none());

        let b = f.halt(19, "deadline-empty");
        let stop = f.stop(19, &b);
        let shutdown = CancellationToken::new();
        let hold = f.hold(19, &stop, &shutdown);
        tokio::pin!(hold);
        assert!(futures_util::poll!(&mut hold).is_pending());
        tokio::time::advance(Duration::from_secs(2)).await;
        f.context(&b, "after deadline");
        assert!(f.registry.pending_halt_ids().is_empty());
        assert!(hold.await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_at_the_final_identity_boundary_cannot_resume() {
        let f = Fixture::new();
        let receipt = f.halt(19, "cancel-on-final-read");
        f.context(&receipt, "resolved before shutdown");
        let stop = f.stop(19, &receipt);
        let shutdown = CancellationToken::new();
        let reads = std::cell::Cell::new(0);
        let goal = root::hold_for_resolution(
            &f.tl,
            &f.registry,
            &f.memory,
            || {
                reads.set(reads.get() + 1);
                if reads.get() == 2 {
                    shutdown.cancel();
                }
                Some(19)
            },
            &stop,
            2,
            "section".into(),
            Duration::from_secs(1),
            &shutdown,
        )
        .await;
        assert_eq!(reads.get(), 2);
        assert!(
            goal.is_none(),
            "shutdown wins over an already terminal halt"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn resolution_while_paused_never_runs_judgement_until_resume() {
        let f = Fixture::new();
        let pid = f
            .scheduler
            .load(
                "orchestrator",
                SpiritManifestBundle::default(),
                orchestrator::Orchestrator::new("orchestrator"),
                194,
            )
            .await
            .unwrap();
        let scb = Arc::clone(f.scheduler.scbs().read().unwrap().get(&pid).unwrap());
        scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Paused)
            .unwrap();
        let halt = f.halt(pid, "resolved-during-pause");
        let calls = AtomicUsize::new(0);
        let shutdown = CancellationToken::new();
        let judgement = root::judge_after_pause(&f.scheduler, pid, 2, &shutdown, || {
            calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(
                f.registry.lookup_state(&halt.halt_id),
                Some(HaltState::Resumed)
            );
            Ok(Some(halt.clone()))
        });
        tokio::pin!(judgement);
        assert!(futures_util::poll!(&mut judgement).is_pending());
        f.context(&halt, "answered while paused");
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(futures_util::poll!(&mut judgement).is_pending());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        scb.try_transition(ScbLifecycleState::Paused, ScbLifecycleState::Running)
            .unwrap();
        tokio::time::advance(Duration::from_millis(100)).await;
        let receipt = judgement.await.unwrap().unwrap().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            f.hold(pid, &f.stop(pid, &receipt), &shutdown).await,
            Some(format!(
                "section{}answered while paused",
                root::HALT_CLARIFICATION_DELIMITER
            )),
        );
    }

    #[tokio::test(start_paused = true)]
    async fn prior_boot_markers_and_receipts_do_not_abort_current_hold() {
        use maos_domain::invariants::i3::FrameOrigin;
        use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
        let f = Fixture::new();
        let db = rusqlite::Connection::open(&f.tl_uri).unwrap();
        // The reused-home marker predates this boot at the same numeric pid.
        f.tl.insert_frame_event(
            FrameKind::EpistemicHalt,
            19,
            None,
            "terminated",
            b"",
            FrameOrigin::Kernel,
        )
        .into_inner();
        db.execute("UPDATE transparency_log SET boot_nonce = 193", [])
            .unwrap();
        let receipt = f.halt(19, "current-boot");
        let mut prior: serde_json::Value = serde_json::from_slice(
            &f.tl
                .query_frames(FrameFilter {
                    frame_id: Some(receipt.frame_id),
                    ..Default::default()
                })
                .unwrap()[0]
                .payload_redacted,
        )
        .unwrap();
        prior["halt_id"] = "prior-boot".into();
        f.tl.insert_frame_event(
            FrameKind::EpistemicHalt,
            19,
            None,
            "old-raise",
            &serde_json::to_vec(&prior).unwrap(),
            FrameOrigin::Kernel,
        )
        .into_inner();
        // Place a valid historical raise after the current one in timestamp
        // order: only a boot filter can exclude it from the hold's interval.
        db.execute("UPDATE transparency_log SET boot_nonce = 193, timestamp_ns = ?1 WHERE intent = 'old-raise'", [i64::MAX]).unwrap();
        f.tl.insert_frame_event(
            FrameKind::EpistemicHalt,
            19,
            None,
            "receipt",
            b"{}",
            FrameOrigin::Kernel,
        )
        .into_inner();
        f.context(&receipt, "current answer");
        assert!(f
            .hold(19, &f.stop(19, &receipt), &CancellationToken::new())
            .await
            .unwrap()
            .contains("current answer"));
    }

    #[tokio::test(start_paused = true)]
    async fn uncommitted_resolution_and_rollback_keep_dispatch_held() {
        let f = Fixture::new();
        let receipt = f.halt(19, "approval-required");
        let metadata = f.registry.halt_metadata_for_spirit(19, 0).pop().unwrap().1;
        let stop = f.stop(19, &receipt);
        let shutdown = CancellationToken::new();
        let hold = f.hold(19, &stop, &shutdown);
        tokio::pin!(hold);
        let answer = Resolution::provided_context("committed answer").unwrap();
        f.resolver
            .resolve(&receipt.halt_id, answer.clone())
            .unwrap();
        assert!(
            futures_util::poll!(&mut hold).is_pending(),
            "registry publication is not approval"
        );
        assert!(f
            .registry
            .rollback_resolution(&receipt.halt_id, Some(metadata)));
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(
            futures_util::poll!(&mut hold).is_pending(),
            "rollback must keep holding"
        );
        f.resolver
            .resolve(&receipt.halt_id, answer.clone())
            .unwrap();
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(futures_util::poll!(&mut hold).is_pending());
        let unrelated = f.halt(20, "unrelated-approval");
        f.context(&unrelated, "not ours");
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(
            futures_util::poll!(&mut hold).is_pending(),
            "approval must name this halt"
        );
        f.approve(&receipt, &answer);
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(hold.await.unwrap().contains("committed answer"));
        let uncommitted = f.halt(19, "approval-never-written");
        f.resolver.resolve(&uncommitted.halt_id, answer).unwrap();
        assert!(
            f.hold(19, &f.stop(19, &uncommitted), &shutdown)
                .await
                .is_none(),
            "missing approval reaches deadline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn unload_between_sections_or_during_pause_reaches_clean_stop() {
        let f = Fixture::new();
        let old = orchestrator::Orchestrator::new("orchestrator");
        let pid = f
            .scheduler
            .load(
                "orchestrator",
                SpiritManifestBundle::default(),
                old.clone(),
                194,
            )
            .await
            .unwrap();
        let scb = Arc::clone(f.scheduler.scbs().read().unwrap().get(&pid).unwrap());
        scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Paused)
            .unwrap();
        let shutdown = CancellationToken::new();
        let judge = root::judge_after_pause(&f.scheduler, pid, 2, &shutdown, || {
            root::judge_live_section(&f.scheduler, pid, &old, &section())
        });
        tokio::pin!(judge);
        assert!(futures_util::poll!(&mut judge).is_pending());
        f.scheduler.unload(pid).await.unwrap();
        tokio::time::advance(Duration::from_millis(100)).await;
        assert!(judge.await.unwrap().unwrap().is_none());
        let stop = root::dispatch_stop(
            &f.registry,
            pid,
            f.scheduler.resolve_pid("orchestrator"),
            2,
            3,
        )
        .unwrap();
        assert_eq!(stop["orchestrator_unloaded"], true);
        assert_eq!(stop["undispatched_sections"], serde_json::json!([2, 3]));
        let successor = f
            .scheduler
            .load(
                "orchestrator",
                SpiritManifestBundle::default(),
                old.clone(),
                194,
            )
            .await
            .unwrap();
        assert_ne!(pid, successor);
        assert!(
            root::judge_live_section(&f.scheduler, pid, &old, &section())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            root::dispatch_stop(&f.registry, pid, Some(successor), 2, 3).unwrap()
                ["orchestrator_unloaded"],
            true
        );
    }

    #[tokio::test(start_paused = true)]
    async fn judgement_follows_the_committed_runtime_scalar_after_swap() {
        PROVIDERS.with(|providers| providers.lock().unwrap().clear());
        let f = Fixture::new();
        let old_calls = Arc::new(AtomicUsize::new(0));
        let new_calls = Arc::new(AtomicUsize::new(0));
        let old_scalar: Arc<dyn EpistemicScalarPort> = Arc::new(Scalar {
            fixture: Arc::downgrade(&f),
            pid: 19,
            threshold: 0.5,
            calls: Arc::clone(&old_calls),
            name: "old-policy",
        });
        let old = emitter(Arc::clone(&old_scalar));
        let object = root::judgement_runtime(old.clone(), old_scalar);
        let pid = f
            .scheduler
            .load_obj("orchestrator", SpiritManifestBundle::default(), object, 194)
            .await
            .unwrap();
        assert!(
            root::judge_live_section(&f.scheduler, pid, &old, &section())
                .unwrap()
                .is_none()
        );
        let scalar: Arc<dyn EpistemicScalarPort> = Arc::new(Scalar {
            fixture: Arc::downgrade(&f),
            pid,
            threshold: 0.2,
            calls: Arc::clone(&new_calls),
            name: "new-policy",
        });
        use maos_domain::invariants::i1::{CapabilityToken, TokenId};
        let seam = butler::ButlerSeam {
            inference: Some((
                Arc::new(|| Some(Arc::new(Replay))),
                vec!["test".into(), "successor".into()],
            )),
            mint: Arc::new(|pid, _| Ok(CapabilityToken::new(TokenId([19; 16]), pid, 0, [0; 64]))),
            digest: Arc::new(butler::MorningDigestRenderer::new(
                f._dir.path().join("tl"),
                f._dir.path().join("digest-state"),
                Arc::new(|_, _| {}),
                Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
            )),
            rollover: false,
            clock: Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
            acceptance: None,
            scene_butler: Arc::default(),
            context: None,
        };
        let successor = root::construct(
            orchestrator::Orchestrator::new("orchestrator"),
            &seam,
            &["not-registered".into(), "successor".into()],
            Arc::new(move || Some(pid)),
            &|_| Ok(Arc::clone(&scalar)),
        )
        .unwrap();
        let successor = root::judgement_runtime(successor, scalar);
        // Building a candidate does not publish it; the SCB commit is the cutover.
        assert!(
            root::judge_live_section(&f.scheduler, pid, &old, &section())
                .unwrap()
                .is_none()
        );
        let scb = Arc::clone(f.scheduler.scbs().read().unwrap().get(&pid).unwrap());
        let mut runtime = scb.runtime_snapshot();
        runtime.spirit_obj = successor;
        scb.replace_runtime(runtime);
        let receipt = root::judge_live_section(&f.scheduler, pid, &old, &section())
            .unwrap()
            .unwrap();
        assert_eq!(receipt.spirit_pid, pid);
        assert_eq!(old_calls.load(Ordering::SeqCst), 2);
        assert_eq!(new_calls.load(Ordering::SeqCst), 1);
        assert_eq!(receipt.halt_id.as_str(), "new-policy");
        PROVIDERS.with(|providers| {
            assert_eq!(&*providers.lock().unwrap(), &["test", "test", "successor"])
        });
    }
}
