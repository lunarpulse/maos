#![forbid(unsafe_code)]

//! Story 19-1 (AC1, AC2.1, AC4) — `maos run … --epic <file>` end to end
//! through the production binary, each run on a fresh data home (the FR21
//! 60 s proxy is per data home).
//!
//! * The three-Worker run and the pending-halt plant need the signed T3
//!   substrate (`require_t3`: a named local skip, never a green skip on a
//!   declared-capable runner).
//! * Every refusal fires before any Worker is spawned, so those vectors run
//!   everywhere: they read stderr and the Transparency Log, never a skip.
//!
//! Observables are the root's stdout events and the TL rows it wrote — never
//! state this test set itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
const EPIC: &str = "tests/fixtures/j1/epic.md";
/// `FrameKind::TaskAssign` / `FrameKind::Distillate` (maos-iac TL codes).
const KIND_TASK_ASSIGN: i64 = 0;
const KIND_DISTILLATE: i64 = 11;

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

/// A fresh, self-removing data home per run.
struct DataHome(PathBuf);

impl Drop for DataHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn data_home(tag: &str) -> DataHome {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-19-1-{tag}-{nanos}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create data home");
    DataHome(path)
}

/// `maos run <args> --once` from the workspace root in a clean environment.
fn run_maos(home: &DataHome, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command.env_clear();
    add_t3_environment(&mut command);
    command
        .arg("run")
        .args(args)
        .arg("--once")
        .env("PATH", path_with_target_debug())
        .env("HOME", t3_substrate::isolated_runtime_home(&home.0))
        .env("XDG_DATA_HOME", &home.0)
        .env("MAOS_HOME", &home.0);
    for (key, value) in env {
        command.env(key, value);
    }
    command
        .current_dir(workspace_root())
        .output()
        .expect("execute maos")
}

fn events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn named<'a>(events: &'a [serde_json::Value], name: &str) -> Vec<&'a serde_json::Value> {
    events
        .iter()
        .filter(|event| event["event"].as_str() == Some(name))
        .collect()
}

fn sections(path: &str, entries: usize) -> Vec<orchestrator::EpicSection> {
    let text = std::fs::read_to_string(Path::new(workspace_root()).join(path)).expect("read epic");
    orchestrator::split_epic(&text, entries).expect("fixture epic splits")
}

/// `(frame_id hex, spirit_pid, payload JSON)` of every TL row of `kind`, in
/// journal order. A run refused before the log was created has none.
fn tl_rows(home: &DataHome, kind: i64) -> Vec<(String, i64, serde_json::Value)> {
    let db = home.0.join("audit").join("transparency.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let connection = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open transparency log");
    let mut statement = connection
        .prepare(
            "SELECT frame_id, spirit_pid, payload_redacted FROM transparency_log \
             WHERE kind = ?1 ORDER BY timestamp_ns, rowid",
        )
        .expect("prepare TL query");
    statement
        .query_map([kind], |row| {
            let frame_id: Vec<u8> = row.get(0)?;
            let payload: Vec<u8> = row.get(2)?;
            Ok((
                hex::encode(frame_id),
                row.get::<_, i64>(1)?,
                serde_json::from_slice(&payload).unwrap_or(serde_json::Value::Null),
            ))
        })
        .expect("query TL rows")
        .collect::<Result<_, _>>()
        .expect("read TL rows")
}

/// The `prior_distillate_ref` of a journaled `task.assign` payload.
fn prior_ref(payload: &serde_json::Value) -> &serde_json::Value {
    &payload["TaskAssign"]["prior_distillate_ref"]
}

fn hex_of_byte_array(value: &serde_json::Value) -> String {
    let bytes: Vec<u8> = value
        .as_array()
        .expect("a byte array")
        .iter()
        .map(|b| b.as_u64().expect("a byte") as u8)
        .collect();
    hex::encode(bytes)
}

/// AC1.1 + AC4.1 — three sections, three Workers, in order; each Worker
/// distilled at its OWN pid; frames 2 and 3 reference the previous
/// section's distillate, and the FR21 gate admits them on the real rows.
#[test]
fn epic_dispatches_each_section_to_its_worker_with_distillate_fed_followups() {
    if !t3_substrate::require_t3("epic three-Worker dispatch") {
        return;
    }
    let home = data_home("three");
    let output = run_maos(&home, &[EPIC_TOPOLOGY, "--epic", EPIC], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the epic run must exit 0; stderr:\n{stderr}"
    );
    let events = events(&output);
    let sections = sections(EPIC, 3);

    // Per section, in this order, before the next section starts.
    let order: Vec<&str> = events
        .iter()
        .filter_map(|event| event["event"].as_str())
        .filter(|name| {
            [
                "delegation_routed",
                "topology_worker_admit",
                "cli_wrapper_loaded",
                "cli_wrapper_exit",
                "distillate_written",
                "delegation_completed",
            ]
            .contains(name)
        })
        .collect();
    let one = [
        "delegation_routed",
        "topology_worker_admit",
        "cli_wrapper_loaded",
        "cli_wrapper_exit",
        "distillate_written",
        "delegation_completed",
    ];
    assert_eq!(order, one.repeat(3), "per-section event order");

    let routed = named(&events, "delegation_routed");
    for (event, section) in routed.iter().zip(&sections) {
        assert_eq!(event["transport"], "loopback-rehearsal");
        assert_eq!(
            event["goal"].as_str(),
            Some(section.text.as_str()),
            "section {}'s text is its Worker's goal byte for byte",
            section.ordinal
        );
    }

    let loaded = named(&events, "cli_wrapper_loaded");
    let child_pids: std::collections::BTreeSet<u64> = loaded
        .iter()
        .map(|event| event["child_pid"].as_u64().expect("child_pid"))
        .collect();
    assert_eq!(child_pids.len(), 3, "three distinct Worker processes");
    let worker_pids: Vec<u64> = loaded
        .iter()
        .map(|event| event["spirit_pid"].as_u64().expect("spirit_pid"))
        .collect();

    let written = named(&events, "distillate_written");
    let distillates = tl_rows(&home, KIND_DISTILLATE);
    for (k, event) in written.iter().enumerate() {
        assert_eq!(event["section"].as_u64(), Some(k as u64 + 1));
        assert_eq!(
            event["producer_pid"].as_u64(),
            Some(worker_pids[k]),
            "section {}'s distillate is written at ITS Worker's pid (I11)",
            k + 1
        );
        let digest = event["digest_frame_id"].as_str().expect("digest_frame_id");
        let row = distillates
            .iter()
            .find(|(id, _, _)| id == digest)
            .unwrap_or_else(|| panic!("a Distillate TL row {digest}"));
        assert_eq!(
            row.1 as u64, worker_pids[k],
            "the row's pid is the Worker's"
        );
    }

    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    assert_eq!(assigns.len(), 3, "three task.assign rows: {assigns:?}");
    assert!(
        prior_ref(&assigns[0].2).is_null(),
        "the first dispatch has no predecessor"
    );
    for k in 1..3 {
        let prior = prior_ref(&assigns[k].2);
        assert_eq!(
            hex_of_byte_array(&prior["digest_frame_id"]),
            written[k - 1]["digest_frame_id"]
                .as_str()
                .expect("digest id"),
            "frame {} carries section {}'s distillate (FR21)",
            k + 1,
            k
        );
        assert!(prior["distillation_depth"].as_u64() >= Some(1));
    }
    assert!(named(&events, "dispatch_halted").is_empty());
}

/// AC4.2 / AC4.3(b) (D-19-1-09 A) — a pending halt on the Orchestrator's pid
/// before section 2 stops dispatch: one `task.assign`, `dispatch_halted`
/// naming sections 2–3, and the run drains normally.
#[test]
fn a_pending_halt_before_section_two_stops_dispatch() {
    if !t3_substrate::require_t3("epic pending-halt stop") {
        return;
    }
    let home = data_home("halt");
    let output = run_maos(
        &home,
        &[EPIC_TOPOLOGY, "--epic", EPIC],
        &[("MAOS_TEST_ONLY_HALT_BEFORE_SECTION", "2")],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "a halted dispatch drains normally (exit 0, D-19-1-09 A); stderr:\n{stderr}"
    );
    let events = events(&output);
    let halted = named(&events, "dispatch_halted");
    assert_eq!(halted.len(), 1, "one dispatch_halted event: {events:?}");
    assert_eq!(
        halted[0]["undispatched_sections"],
        serde_json::json!([2, 3])
    );
    assert_eq!(halted[0]["dispatched_sections"].as_u64(), Some(1));
    assert!(
        halted[0]["halt_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty()),
        "the event names the pending halt"
    );
    assert_eq!(named(&events, "delegation_routed").len(), 1);
    assert_eq!(named(&events, "cli_wrapper_loaded").len(), 1);
    assert_eq!(
        tl_rows(&home, KIND_TASK_ASSIGN).len(),
        1,
        "no task.assign after the halt"
    );
}

/// AC1.6 — the proven-red split: the fixture with its second heading
/// replaced by `### Notes` is refused typed, naming 2 sections vs 3 entries,
/// with zero `task.assign` rows and a non-zero exit.
#[test]
fn a_heading_less_section_is_refused_before_any_emit() {
    let home = data_home("heading");
    let text = std::fs::read_to_string(Path::new(workspace_root()).join(EPIC)).expect("read epic");
    let planted = text.replacen(
        "### Story 2: Draft the design note",
        "### Notes: Draft the design note",
        1,
    );
    assert_ne!(planted, text, "the plant must change the second heading");
    let epic = home.0.join("epic-planted.md");
    std::fs::write(&epic, planted).expect("write planted epic");
    let output = run_maos(
        &home,
        &[EPIC_TOPOLOGY, "--epic", epic.to_str().expect("utf-8 path")],
        &[],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "the planted epic must fail the run"
    );
    assert!(
        stderr.contains("is headed `### Notes: Draft the design note`, not `### Story`")
            && stderr.contains("2 `### Story` sections vs 3 host-bearing entries"),
        "the refusal names the heading and both counts; stderr:\n{stderr}"
    );
    assert!(named(&events(&output), "delegation_routed").is_empty());
    assert!(tl_rows(&home, KIND_TASK_ASSIGN).is_empty());
}

/// AC1.4 (D-19-1-03 A) — a section count that differs from the host-bearing
/// entry count is refused before any emit.
#[test]
fn a_section_count_mismatch_is_refused_before_any_emit() {
    let home = data_home("count");
    let output = run_maos(
        &home,
        &[
            EPIC_TOPOLOGY,
            "--epic",
            "tests/fixtures/j1/epic-one-section.md",
        ],
        &[],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("1 `### Story` sections vs 3 host-bearing entries"),
        "stderr:\n{stderr}"
    );
    assert!(tl_rows(&home, KIND_TASK_ASSIGN).is_empty());
}

/// AC2.1 (D-19-1-02 A) — `MAOS_DELEGATED_GOAL` on the loopback arm is
/// refused typed, naming `--epic`, before any frame.
#[test]
fn a_delegated_goal_on_loopback_is_refused_naming_epic() {
    let home = data_home("env-goal");
    let output = run_maos(
        &home,
        &["spirits/topologies/j1-founder-loop.toml"],
        &[("MAOS_DELEGATED_GOAL", "write a haiku to ./h.txt")],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("MAOS_DELEGATED_GOAL") && stderr.contains("--epic"),
        "stderr:\n{stderr}"
    );
    assert!(named(&events(&output), "delegation_routed").is_empty());
    assert!(tl_rows(&home, KIND_TASK_ASSIGN).is_empty());
}

/// AC1.2 — `--epic` on a manifest with no host-bearing entry (a single-Spirit
/// manifest) is refused typed.
#[test]
fn epic_on_a_single_spirit_manifest_is_refused() {
    let home = data_home("single");
    let output = run_maos(
        &home,
        &["spirits/orchestrator/manifest.toml", "--epic", EPIC],
        &[],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("--epic needs a topology with host-bearing Worker entries"),
        "stderr:\n{stderr}"
    );
}

/// D-19-1-04 A — the loaded Orchestrator emits every frame, so a host-bearing
/// entry with no `orchestrator` entry before it is refused typed.
#[test]
fn a_host_bearing_entry_before_the_orchestrator_is_refused() {
    let home = data_home("order");
    let topology = home.0.join("worker-first.toml");
    std::fs::write(
        &topology,
        format!(
            "[topology]\nname = \"worker-first-19-1\"\n\n\
             [[topology.spirits]]\nmanifest = \"{root}/spirits/worker/manifest.toml\"\n\
             host = \"developer-remote-host\"\n\n\
             [[topology.spirits]]\nmanifest = \"{root}/spirits/orchestrator/manifest.toml\"\n",
            root = workspace_root()
        ),
    )
    .expect("write topology");
    let output = run_maos(&home, &[topology.to_str().expect("utf-8 path")], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("no `orchestrator` Orchestrator entry precedes it"),
        "stderr:\n{stderr}"
    );
    assert!(tl_rows(&home, KIND_TASK_ASSIGN).is_empty());
}
