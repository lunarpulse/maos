#![forbid(unsafe_code)]

//! Story 19-1 (AC1, AC2.1, AC4) — `maos run … --epic <file>` end to end
//! through the production binary, each run on a fresh data home (the FR21
//! 60 s proxy is per data home).
//!
//! * The three-Worker run, the pending-halt plant and the crashed-Worker
//!   runs need the signed T3 substrate (`require_t3`: a named local skip,
//!   never a green skip on a declared-capable runner).
//! * Every refusal fires before any Worker is spawned, so those vectors run
//!   everywhere: they read stderr and the Transparency Log, never a skip.
//! * Story 19-6 (D-19-6-09 A) — a crashed section's Worker reaches the
//!   Orchestrator as a kernel-sent orphan, and the loop applies the declared
//!   disposition: one vector per `nack` / `reassign-to-replica` (once, then
//!   escalation on a second crash) / `escalate-to-operator`, plus a crashed
//!   `reassign-to-replica` section 1, which has no prior distillate for a
//!   re-dispatch to carry and so escalates.
//!
//! Observables are the root's stdout events and the TL rows it wrote — never
//! state this test set itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
const EPIC: &str = "tests/fixtures/j1/epic.md";
/// `FrameKind::TaskAssign` / `FrameKind::TaskComplete` /
/// `FrameKind::EpistemicHalt` / `FrameKind::CapabilityInvocation` /
/// `FrameKind::Distillate` (maos-iac TL codes).
const KIND_TASK_ASSIGN: i64 = 0;
const KIND_TASK_COMPLETE: i64 = 1;
const KIND_EPISTEMIC_HALT: i64 = 3;
const KIND_CAPABILITY_INVOCATION: i64 = 7;
const KIND_DISTILLATE: i64 = 11;
/// FR12 / NFR-Rel-1: the orphan reaches its originator within 5 s.
const DELIVERY_FLOOR_NS: i64 = 5_000_000_000;

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

fn open_tl(home: &DataHome) -> Option<rusqlite::Connection> {
    let db = home.0.join("audit").join("transparency.sqlite");
    if !db.exists() {
        return None;
    }
    Some(
        rusqlite::Connection::open_with_flags(
            db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .expect("open transparency log"),
    )
}

/// `(frame_id hex, spirit_pid, payload JSON)` of every TL row of `kind`, in
/// journal order. A run refused before the log was created has none.
fn tl_rows(home: &DataHome, kind: i64) -> Vec<(String, i64, serde_json::Value)> {
    journal_rows(home, kind)
        .into_iter()
        .map(|row| (row.frame_id, row.spirit_pid, row.payload))
        .collect()
}

/// One TL row with every column a 19-6 vector joins on.
#[derive(Debug)]
struct JournalRow {
    frame_id: String,
    timestamp_ns: i64,
    spirit_pid: i64,
    from: String,
    intent: String,
    payload: serde_json::Value,
}

/// Every TL row of `kind`, in journal order.
fn journal_rows(home: &DataHome, kind: i64) -> Vec<JournalRow> {
    let Some(connection) = open_tl(home) else {
        return Vec::new();
    };
    let mut statement = connection
        .prepare(
            "SELECT frame_id, timestamp_ns, spirit_pid, from_spirit_id, intent, payload_redacted \
             FROM transparency_log WHERE kind = ?1 ORDER BY timestamp_ns, rowid",
        )
        .expect("prepare TL query");
    statement
        .query_map([kind], |row| {
            let frame_id: Vec<u8> = row.get(0)?;
            let payload: Vec<u8> = row.get(5)?;
            Ok(JournalRow {
                frame_id: hex::encode(frame_id),
                timestamp_ns: row.get(1)?,
                spirit_pid: row.get(2)?,
                from: row.get(3)?,
                intent: row.get(4)?,
                payload: serde_json::from_slice(&payload).unwrap_or(serde_json::Value::Null),
            })
        })
        .expect("query TL rows")
        .collect::<Result<_, _>>()
        .expect("read TL rows")
}

/// `(recipient_pid, recipient_spirit_id)` recorded for one frame.
fn recipients(home: &DataHome, frame_id_hex: &str) -> Vec<(i64, String)> {
    let connection = open_tl(home).expect("the run created its transparency log");
    let frame_id = hex::decode(frame_id_hex).expect("hex frame id");
    let mut statement = connection
        .prepare(
            "SELECT recipient_pid, recipient_spirit_id FROM transparency_log_recipients \
             WHERE frame_id = ?1 ORDER BY recipient_pid",
        )
        .expect("prepare recipients query");
    statement
        .query_map([frame_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query recipients")
        .collect::<Result<_, _>>()
        .expect("read recipients")
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

/// The admitted pid of the topology's `orchestrator` entry, from its
/// `spirit_loaded` admission event.
fn orchestrator_pid(events: &[serde_json::Value]) -> u64 {
    let loaded: Vec<u64> = named(events, "spirit_loaded")
        .into_iter()
        .filter(|event| event["spirit_id"].as_str() == Some("orchestrator"))
        .map(|event| event["pid"].as_u64().expect("spirit_loaded pid"))
        .collect();
    assert_eq!(loaded.len(), 1, "one Orchestrator admission: {events:?}");
    assert_ne!(loaded[0], 0, "the Orchestrator is admitted at a real pid");
    loaded[0]
}

/// Accept#9 — every `delegation_completed` leaves the Orchestrator at a
/// safe point.
fn assert_completions_at_safe_point(events: &[serde_json::Value]) {
    for event in named(events, "delegation_completed") {
        assert_eq!(
            event["orchestrator_safe_point"].as_bool(),
            Some(true),
            "a completed delegation leaves the Orchestrator at a safe point: {event}"
        );
    }
}

/// A T3 `worker-cli-fixture` manifest — the known-admitting shape — with an
/// optional fixture mode and an optional declared `[on_crash]` action.
fn worker_manifest(home: &DataHome, mode: Option<&str>, on_crash: Option<&str>) -> PathBuf {
    let path = home.0.join(format!(
        "worker-{}-{}.toml",
        mode.unwrap_or("default"),
        on_crash.unwrap_or("nack")
    ));
    let argv = match mode {
        Some(mode) => format!("[\"--maos-worker\", \"--maos-fixture-mode={mode}\"]"),
        None => "[\"--maos-worker\"]".to_string(),
    };
    let on_crash = on_crash
        .map(|action| format!("\n[on_crash]\naction = \"{action}\"\n"))
        .unwrap_or_default();
    std::fs::write(
        &path,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = {argv}\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n{on_crash}"
        ),
    )
    .expect("write Worker manifest");
    path
}

/// The epic topology with `manifest` as section `section`'s entry (D-19-1-03
/// A: section k ↔ host-bearing entry k); the other host-bearing entries keep
/// the default Worker.
fn topology_with_section_entry(home: &DataHome, section: usize, manifest: &Path) -> PathBuf {
    let root = workspace_root();
    let hosts: String = (1..=3)
        .map(|k| {
            let path = if k == section {
                manifest.display().to_string()
            } else {
                format!("{root}/spirits/worker/manifest.toml")
            };
            format!(
                "\n[[topology.spirits]]\nmanifest = \"{path}\"\n\
                 host = \"developer-remote-host\"\n"
            )
        })
        .collect();
    let topology = home.0.join(format!("epic-section-{section}-entry.toml"));
    std::fs::write(
        &topology,
        format!(
            "[topology]\nname = \"epic-section-{section}-entry-19-6\"\n\n\
             [[topology.spirits]]\nmanifest = \"{root}/spirits/orchestrator/manifest.toml\"\n\n\
             [[topology.spirits]]\nmanifest = \"{root}/spirits/architect/manifest.toml\"\n\n\
             [[topology.spirits]]\nmanifest = \"{root}/spirits/reviewer/manifest.toml\"\n{hosts}"
        ),
    )
    .expect("write topology");
    topology
}

/// `maos run <topology> --epic EPIC --once`, colour off so the notification
/// channel's stderr line is plain text.
fn run_epic(home: &DataHome, topology: &Path, env: &[(&str, &str)]) -> Output {
    let mut env = env.to_vec();
    env.push(("NO_COLOR", "1"));
    run_maos(
        home,
        &[topology.to_str().expect("utf-8 path"), "--epic", EPIC],
        &env,
    )
}

/// One crashed delegation attempt, as the kernel and the delivery journaled it.
struct CrashedAttempt {
    task_id: String,
    /// The kernel's classification, from its `task.orphaned` row.
    cause: String,
    worker_pid: i64,
    delivered_ns: i64,
}

/// The kernel's task id of the delegation whose `task.assign` frame is
/// `assign_frame_hex` (the id every crash row is joined on).
fn delegation_task_id(assign_frame_hex: &str) -> String {
    let frame_id: [u8; 16] = hex::decode(assign_frame_hex)
        .expect("hex frame id")
        .try_into()
        .expect("a 16-byte frame id");
    maos_bin::supervision::WorkerTask::Delegation { frame_id }
        .record_ids("worker")
        .0
}

/// AC1.2 / AC2.2 / AC2.4 — one crashed attempt joined end to end on its
/// delegation's `task.assign` frame id: the kernel's `task.orphaned` row at
/// the Worker's pid (cause, originator) and its pid-0 disposition row
/// (`disposition_intent`), both unchanged 16-3 rows; then exactly ONE
/// kernel-sent `TaskComplete{"orphaned"}` at pid 0 whose id is derived from
/// the delegation's, with ONE recipient row naming the admitted Orchestrator,
/// journaled ≤ 5 s after the Worker's death as the kernel journaled it (the
/// earliest of its `cli.subprocess.exit` and `task.orphaned` rows; the
/// kill-stamped floor is the 16-3 corpus's, AC5). `cause: None` ⇒ any
/// kernel-classified `fault.*` (a root-side kill's cause is the kernel's call).
fn assert_orphan_reached_orchestrator(
    home: &DataHome,
    assign_frame_hex: &str,
    orchestrator: u64,
    cause: Option<&str>,
    disposition_intent: &str,
) -> CrashedAttempt {
    let frame_id: [u8; 16] = hex::decode(assign_frame_hex)
        .expect("hex frame id")
        .try_into()
        .expect("a 16-byte frame id");
    let task_id = delegation_task_id(assign_frame_hex);
    let complete = journal_rows(home, KIND_TASK_COMPLETE);
    let of_task = |intent: &str| -> Vec<&JournalRow> {
        complete
            .iter()
            .filter(|row| row.intent == intent && row.payload["task_id"] == task_id.as_str())
            .collect()
    };

    let orphaned = of_task("task.orphaned");
    assert_eq!(
        orphaned.len(),
        1,
        "one task.orphaned for {task_id}: {complete:?}"
    );
    let worker_pid = orphaned[0].spirit_pid;
    assert_ne!(worker_pid, 0, "task.orphaned sits at the Worker's own pid");
    let observed_cause = orphaned[0].payload["cause"]
        .as_str()
        .expect("task.orphaned names a cause")
        .to_string();
    match cause {
        Some(cause) => assert_eq!(observed_cause, cause),
        None => assert!(observed_cause.starts_with("fault."), "{observed_cause}"),
    }
    assert_eq!(orphaned[0].payload["originator_spirit_id"], "orchestrator");
    let disposition = of_task(disposition_intent);
    assert_eq!(
        disposition.len(),
        1,
        "one {disposition_intent} for {task_id}: {complete:?}"
    );
    assert_eq!(disposition[0].spirit_pid, 0, "dispositions sit at pid 0");

    let orphan_id = hex::encode(maos_bin::delegation::orphan_frame_id(frame_id));
    let delivered: Vec<&JournalRow> = complete
        .iter()
        .filter(|row| row.frame_id == orphan_id)
        .collect();
    assert_eq!(
        delivered.len(),
        1,
        "exactly one kernel-sent orphan for {task_id}: {complete:?}"
    );
    let delivered = delivered[0];
    assert_eq!(delivered.from, "__kernel", "the kernel is the author");
    assert_eq!(delivered.spirit_pid, 0);
    assert_eq!(delivered.payload["TaskComplete"]["result"], "orphaned");
    assert_eq!(
        recipients(home, &delivered.frame_id),
        vec![(orchestrator as i64, "orchestrator".to_string())],
        "the orphan is recorded as received by the admitted Orchestrator"
    );
    let died_ns = journal_rows(home, KIND_CAPABILITY_INVOCATION)
        .iter()
        .chain(orphaned.iter().copied())
        .filter(|row| {
            row.spirit_pid == worker_pid
                && (row.intent == "cli.subprocess.exit" || row.intent == "task.orphaned")
        })
        .map(|row| row.timestamp_ns)
        .min()
        .expect("the kernel journaled the Worker's death");
    assert!(
        delivered.timestamp_ns - died_ns <= DELIVERY_FLOOR_NS,
        "the orphan is delivered ≤ 5 s after the Worker's death ({} ns)",
        delivered.timestamp_ns - died_ns
    );
    CrashedAttempt {
        task_id,
        cause: observed_cause,
        worker_pid,
        delivered_ns: delivered.timestamp_ns,
    }
}

/// AC1.2 — one `delegation_orphaned` per crashed attempt of section
/// `section`, in order, each naming the kernel's cause and the APPLIED
/// disposition and leaving the Orchestrator at a safe point (its in-flight
/// task released).
fn assert_orphaned_events(
    events: &[serde_json::Value],
    section: u64,
    attempts: &[(&CrashedAttempt, &str)],
) {
    let orphaned = named(events, "delegation_orphaned");
    assert_eq!(orphaned.len(), attempts.len(), "{events:?}");
    for (event, (attempt, disposition)) in orphaned.iter().zip(attempts) {
        assert_eq!(event["section"].as_u64(), Some(section), "{event}");
        assert_eq!(event["task_id"].as_str(), Some(attempt.task_id.as_str()));
        assert_eq!(
            event["cause"].as_str(),
            Some(attempt.cause.as_str()),
            "{event}"
        );
        assert_eq!(event["disposition"].as_str(), Some(*disposition), "{event}");
        assert_eq!(
            event["orchestrator_safe_point"].as_bool(),
            Some(true),
            "the orphan releases the Orchestrator's in-flight delegation: {event}"
        );
    }
}

/// AC1.2 / AC3.3 — the typed stop names section, cause and disposition.
fn assert_stopped(output: &Output, section: u64, cause: &str, disposition: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a crashed section fails the epic run; stderr:\n{stderr}"
    );
    let stop = format!(
        "maos run: section {section} Worker crashed ({cause}); disposition {disposition}; \
         dispatch stopped"
    );
    assert!(
        stderr.contains(&stop),
        "expected `{stop}`; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("did not complete"),
        "a crash is no longer the generic non-completion; stderr:\n{stderr}"
    );
}

/// AC3.3 (D-19-6-06 A) — one `AnomalyFlagged` from the crash detector about
/// the crashed Worker of section `section`, on the root dispatcher's
/// terminal channel.
fn assert_anomaly_flagged(output: &Output, section: u64, worker_pid: i64) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let subject = format!("pid:{worker_pid}");
    let named_section = format!("section {section}:");
    let flagged: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("Anomaly kernel.crash-detector"))
        .collect();
    assert_eq!(flagged.len(), 1, "one AnomalyFlagged; stderr:\n{stderr}");
    assert!(
        flagged[0].contains(&subject) && flagged[0].contains(&named_section),
        "the notification names the Worker and the section: {}",
        flagged[0]
    );
}

/// Section 1 completed and was distilled before section 2 crashed; nothing
/// after it was distilled, and section 3 was never assigned.
fn assert_only_section_one_distilled(home: &DataHome, events: &[serde_json::Value]) {
    let written = named(events, "distillate_written");
    assert_eq!(written.len(), 1, "only section 1 is distilled: {events:?}");
    assert_eq!(written[0]["section"].as_u64(), Some(1));
    let distillates = tl_rows(home, KIND_DISTILLATE);
    assert_eq!(distillates.len(), 1, "one Distillate row: {distillates:?}");
    assert_eq!(
        Some(distillates[0].0.as_str()),
        written[0]["digest_frame_id"].as_str(),
        "section 1's distillate row exists"
    );
    assert_eq!(
        Some(distillates[0].1 as u64),
        written[0]["producer_pid"].as_u64(),
        "section 1's distillate sits at its Worker's pid"
    );
    assert_eq!(named(events, "delegation_completed").len(), 1);
    assert_completions_at_safe_point(events);
    assert!(named(events, "dispatch_halted").is_empty());
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
    assert_eq!(
        worker_pids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "three distinct Worker SCB pids: {worker_pids:?}"
    );

    let written = named(&events, "distillate_written");
    let distillates = tl_rows(&home, KIND_DISTILLATE);
    assert_eq!(written.len(), 3, "three distillate_written events");
    assert_eq!(
        distillates.len(),
        3,
        "three Distillate TL rows: {distillates:?}"
    );
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
    assert_eq!(named(&events, "delegation_completed").len(), 3);
    assert_completions_at_safe_point(&events);
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
    let halt_ids: Vec<&str> = halted[0]["halt_ids"]
        .as_array()
        .map(|ids| ids.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default();
    assert_eq!(halt_ids.len(), 1, "the event names the one pending halt");
    // TI-1 — the halt sits on the Orchestrator's ADMITTED pid, and every
    // kernel EpistemicHalt row for it is journaled at that same pid: the
    // raise row, plus — because the run unloads the Orchestrator with the
    // halt still pending — the unload's closure, which journals a
    // `HaltReceipt` row with `terminal_state: "Terminated"` (D-19-1-09 A).
    let orchestrator = orchestrator_pid(&events);
    assert_eq!(
        halted[0]["orchestrator_pid"].as_u64(),
        Some(orchestrator),
        "dispatch_halted names the admitted Orchestrator pid"
    );
    let halt_rows: Vec<_> = tl_rows(&home, KIND_EPISTEMIC_HALT)
        .into_iter()
        .filter(|(_, _, payload)| payload["halt_id"].as_str() == Some(halt_ids[0]))
        .collect();
    for (frame_id, spirit_pid, payload) in &halt_rows {
        assert_eq!(
            *spirit_pid as u64, orchestrator,
            "EpistemicHalt row {frame_id} for the halt is at the Orchestrator's pid: {payload}"
        );
    }
    let raise_rows = halt_rows
        .iter()
        .filter(|(_, _, payload)| payload["terminal_state"].is_null())
        .count();
    assert_eq!(
        raise_rows, 1,
        "exactly one EpistemicHalt raise row (no terminal_state) for the halt: {halt_rows:?}"
    );
    let closure_rows = halt_rows
        .iter()
        .filter(|(_, _, payload)| payload["terminal_state"] == "Terminated")
        .count();
    assert!(
        closure_rows <= 1,
        "at most one unload-closure (Terminated) receipt row for the halt: {halt_rows:?}"
    );
    assert_eq!(
        raise_rows + closure_rows,
        halt_rows.len(),
        "no EpistemicHalt row for the halt carries a terminal state other than \
         Terminated (a halted dispatch is never resumed or overridden): {halt_rows:?}"
    );
    assert_eq!(named(&events, "delegation_routed").len(), 1);
    assert_eq!(named(&events, "cli_wrapper_loaded").len(), 1);
    assert_eq!(
        tl_rows(&home, KIND_TASK_ASSIGN).len(),
        1,
        "no task.assign after the halt"
    );
    assert_completions_at_safe_point(&events);
}

/// TI-3, cut over by 19-6 AC1 (default `nack`) — the second Worker exits
/// non-zero (`--maos-fixture-mode=exit-nonzero`; `sigkill-self` cannot kill a
/// T3 payload, which is its PID namespace's init): the crash reaches the
/// Orchestrator as ONE kernel-sent orphan, `delegation_orphaned` names it,
/// section 1's distillate stands, section 3 is never assigned, and the run
/// fails typed naming section 2, the cause and `nack`.
#[test]
fn a_crashed_second_worker_reaches_the_orchestrator_and_nacks_the_epic() {
    if !t3_substrate::require_t3("epic crashed second Worker (nack)") {
        return;
    }
    let home = data_home("crash-nack");
    let second = worker_manifest(&home, Some("exit-nonzero"), None);
    let output = run_epic(&home, &topology_with_section_entry(&home, 2, &second), &[]);
    let events = events(&output);
    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    assert_eq!(
        assigns.len(),
        2,
        "sections 1 and 2 were assigned and section 3 never was: {assigns:?}"
    );
    let crashed = assert_orphan_reached_orchestrator(
        &home,
        &assigns[1].0,
        orchestrator_pid(&events),
        Some("fault.non-zero-exit"),
        "task.nacked",
    );
    assert_orphaned_events(&events, 2, &[(&crashed, "nack")]);
    assert_stopped(&output, 2, "fault.non-zero-exit", "nack");
    assert_only_section_one_distilled(&home, &events);
}

/// AC3.2 (D-19-6-04 A) — section 2's entry declares `reassign-to-replica`
/// and its FIRST Worker is killed while still running (the debug-only
/// `MAOS_TEST_ONLY_CRASH_SECTION_ONCE` root hook; the entry's fixture mode
/// `delay-then-complete` keeps it running for 2 s, then completes). The
/// kill is proven first (TI-3: a starved plant would let the fixture
/// complete, and every count below would red without saying why). The
/// orphan reaches the Orchestrator, the kernel reassigns (`task.reassigned`
/// naming the originator, rule 11), and section 2 is re-dispatched ONCE — a
/// new `task.assign` with the same goal and the same `prior_distillate_ref`,
/// after the delivery — to a fresh Worker that completes; the epic goes on
/// to section 3 and exits 0 with three distillates.
#[test]
fn a_section_crashed_once_is_redispatched_to_a_fresh_worker_and_the_epic_completes() {
    if !t3_substrate::require_t3("epic section crashed once, re-dispatched") {
        return;
    }
    let home = data_home("crash-once");
    let second = worker_manifest(
        &home,
        Some("delay-then-complete"),
        Some("reassign-to-replica"),
    );
    let output = run_epic(
        &home,
        &topology_with_section_entry(&home, 2, &second),
        &[("MAOS_TEST_ONLY_CRASH_SECTION_ONCE", "2")],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "a reassigned section that completes completes the epic; stderr:\n{stderr}"
    );
    let events = events(&output);
    let orchestrator = orchestrator_pid(&events);

    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    // TI-3 (TestInfra19x6) — the plant really killed section 2's FIRST
    // Worker before any count is read: that attempt's `task.assign` has a
    // kernel `task.orphaned` row with a `fault.*` cause, and the loop printed
    // `delegation_orphaned`.
    let first_attempt = assigns
        .get(1)
        .map(|(frame_id, _, _)| delegation_task_id(frame_id))
        .unwrap_or_else(|| panic!("section 2 was never assigned: {assigns:?}"));
    let complete = journal_rows(&home, KIND_TASK_COMPLETE);
    let killed = complete.iter().any(|row| {
        row.intent == "task.orphaned"
            && row.payload["task_id"] == first_attempt.as_str()
            && row.payload["cause"]
                .as_str()
                .is_some_and(|cause| cause.starts_with("fault."))
    });
    assert!(
        killed && !named(&events, "delegation_orphaned").is_empty(),
        "the MAOS_TEST_ONLY_CRASH_SECTION_ONCE plant failed to kill section 2's first Worker \
         ({first_attempt}) before it completed: no fault.* task.orphaned row or no \
         delegation_orphaned event; TaskComplete rows: {complete:?}; events: {events:?}"
    );
    assert_eq!(
        assigns.len(),
        4,
        "section 1, section 2 twice, section 3: {assigns:?}"
    );
    let goal = |k: usize| assigns[k].2["TaskAssign"]["goal"].clone();
    assert_eq!(goal(1), goal(2), "the re-dispatch carries section 2's goal");
    assert_ne!(goal(1), goal(3));
    assert_eq!(
        prior_ref(&assigns[1].2),
        prior_ref(&assigns[2].2),
        "the re-dispatch carries the same prior_distillate_ref (FR21)"
    );
    assert_ne!(assigns[1].0, assigns[2].0, "a new task.assign frame");

    let crashed = assert_orphan_reached_orchestrator(
        &home,
        &assigns[1].0,
        orchestrator,
        None,
        "task.reassigned",
    );
    let reassigned: Vec<&JournalRow> = complete
        .iter()
        .filter(|row| row.intent == "task.reassigned")
        .collect();
    assert_eq!(reassigned.len(), 1, "{complete:?}");
    assert_eq!(
        reassigned[0].payload["replica_spirit_id"], "orchestrator",
        "rule 11: the row names the originator, which re-dispatches"
    );
    assert!(
        complete.iter().all(|row| row.intent != "task.escalated"),
        "one crash, one reassignment, no escalation"
    );
    assert_eq!(
        complete.iter().filter(|row| row.from == "__kernel").count(),
        1,
        "one kernel-sent orphan for one crash"
    );
    let assign_rows = journal_rows(&home, KIND_TASK_ASSIGN);
    assert!(
        assign_rows[2].timestamp_ns > crashed.delivered_ns,
        "the re-dispatch follows the orphan's delivery to the Orchestrator"
    );
    assert_orphaned_events(&events, 2, &[(&crashed, "reassign-to-replica")]);

    // The fresh Worker completes section 2, and the epic goes on.
    let written = named(&events, "distillate_written");
    let ordinals: Vec<Option<u64>> = written
        .iter()
        .map(|event| event["section"].as_u64())
        .collect();
    assert_eq!(ordinals, vec![Some(1), Some(2), Some(3)], "{events:?}");
    assert_eq!(tl_rows(&home, KIND_DISTILLATE).len(), 3);
    let producer = |k: usize| written[k]["producer_pid"].as_u64().expect("producer_pid") as i64;
    assert_ne!(
        producer(1),
        crashed.worker_pid,
        "section 2 is distilled at the FRESH Worker's pid, not the crashed one's"
    );
    assert_eq!(
        hex_of_byte_array(&prior_ref(&assigns[3].2)["digest_frame_id"]),
        written[1]["digest_frame_id"].as_str().expect("digest id"),
        "section 3 is fed the re-run's distillate"
    );
    assert_eq!(named(&events, "delegation_routed").len(), 4);
    assert_eq!(named(&events, "delegation_completed").len(), 3);
    assert_completions_at_safe_point(&events);
    assert!(named(&events, "dispatch_halted").is_empty());
    assert!(
        !stderr.contains("dispatch stopped") && !stderr.contains("Anomaly kernel.crash-detector"),
        "a reassigned crash neither stops nor escalates; stderr:\n{stderr}"
    );
}

/// AC3.2 + AC3.3 (D-19-6-04 A) — an entry declaring `reassign-to-replica`
/// whose Worker crashes on BOTH attempts (`exit-nonzero`): the first crash
/// is reassigned and section 2 re-dispatched once to a fresh Worker; the
/// second finds the budget spent and escalates (`task.escalated`,
/// `AnomalyFlagged`, typed stop). No third `task.assign` for section 2, and
/// section 3 is never assigned.
#[test]
fn a_reassigned_section_that_crashes_again_escalates_and_stops() {
    if !t3_substrate::require_t3("epic reassigned section crashes twice") {
        return;
    }
    let home = data_home("crash-twice");
    let second = worker_manifest(&home, Some("exit-nonzero"), Some("reassign-to-replica"));
    let output = run_epic(&home, &topology_with_section_entry(&home, 2, &second), &[]);
    let events = events(&output);
    let orchestrator = orchestrator_pid(&events);
    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    assert_eq!(
        assigns.len(),
        3,
        "section 1, section 2, section 2 once more — and nothing after: {assigns:?}"
    );
    let goal = |k: usize| assigns[k].2["TaskAssign"]["goal"].clone();
    assert_eq!(goal(1), goal(2), "the re-dispatch carries section 2's goal");
    assert_eq!(
        prior_ref(&assigns[1].2),
        prior_ref(&assigns[2].2),
        "the re-dispatch carries the same prior_distillate_ref (FR21)"
    );
    assert_ne!(assigns[1].0, assigns[2].0, "a new task.assign frame");

    let first = assert_orphan_reached_orchestrator(
        &home,
        &assigns[1].0,
        orchestrator,
        Some("fault.non-zero-exit"),
        "task.reassigned",
    );
    let second = assert_orphan_reached_orchestrator(
        &home,
        &assigns[2].0,
        orchestrator,
        Some("fault.non-zero-exit"),
        "task.escalated",
    );
    assert_ne!(
        first.worker_pid, second.worker_pid,
        "the re-dispatch ran on a fresh Worker"
    );
    let complete = journal_rows(&home, KIND_TASK_COMPLETE);
    assert_eq!(
        complete
            .iter()
            .filter(|row| row.intent == "task.reassigned")
            .count(),
        1,
        "budget one: {complete:?}"
    );
    let assign_rows = journal_rows(&home, KIND_TASK_ASSIGN);
    assert!(
        assign_rows[2].timestamp_ns > first.delivered_ns,
        "the re-dispatch follows the orphan's delivery to the Orchestrator"
    );
    assert_orphaned_events(
        &events,
        2,
        &[
            (&first, "reassign-to-replica"),
            (&second, "escalate-to-operator"),
        ],
    );
    assert_stopped(&output, 2, "fault.non-zero-exit", "escalate-to-operator");
    assert_anomaly_flagged(&output, 2, second.worker_pid);
    assert_only_section_one_distilled(&home, &events);
}

/// AC3.3 (D-19-6-06 A) — an entry declaring `escalate-to-operator`: the
/// kernel escalates, one `AnomalyFlagged` reaches the root's dispatcher, and
/// dispatch stops typed naming `escalate-to-operator`; no re-dispatch.
#[test]
fn an_escalating_section_flags_the_operator_and_stops() {
    if !t3_substrate::require_t3("epic escalating section") {
        return;
    }
    let home = data_home("crash-escalate");
    let second = worker_manifest(&home, Some("exit-nonzero"), Some("escalate-to-operator"));
    let output = run_epic(&home, &topology_with_section_entry(&home, 2, &second), &[]);
    let events = events(&output);
    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    assert_eq!(
        assigns.len(),
        2,
        "no re-dispatch, no section 3: {assigns:?}"
    );
    let crashed = assert_orphan_reached_orchestrator(
        &home,
        &assigns[1].0,
        orchestrator_pid(&events),
        Some("fault.non-zero-exit"),
        "task.escalated",
    );
    assert_orphaned_events(&events, 2, &[(&crashed, "escalate-to-operator")]);
    assert_stopped(&output, 2, "fault.non-zero-exit", "escalate-to-operator");
    assert_anomaly_flagged(&output, 2, crashed.worker_pid);
    assert_only_section_one_distilled(&home, &events);
}

/// Edge19x6 / Blind19x6 / Accept19x6 P1 (D-19-6-04 A) — section 1's entry
/// declares `reassign-to-replica` and its Worker crashes (`exit-nonzero`).
/// With no prior distillate a re-dispatch could never pass the FR21 gate, so
/// the root leaves the resolver unarmed: the kernel escalates
/// (`task.escalated`, never a `task.reassigned` claiming a re-dispatch that
/// cannot happen), one `AnomalyFlagged` names section 1, and dispatch stops
/// typed naming section 1 — exactly one `task.assign`, nothing distilled.
#[test]
fn a_reassigning_first_section_that_crashes_escalates_without_a_prior_distillate() {
    if !t3_substrate::require_t3("epic reassigning first section crashes") {
        return;
    }
    let home = data_home("crash-first");
    let first = worker_manifest(&home, Some("exit-nonzero"), Some("reassign-to-replica"));
    let output = run_epic(&home, &topology_with_section_entry(&home, 1, &first), &[]);
    let events = events(&output);
    let assigns = tl_rows(&home, KIND_TASK_ASSIGN);
    assert_eq!(
        assigns.len(),
        1,
        "section 1 once — no re-dispatch, nothing after: {assigns:?}"
    );
    assert!(
        prior_ref(&assigns[0].2).is_null(),
        "section 1 carries no prior distillate"
    );
    let crashed = assert_orphan_reached_orchestrator(
        &home,
        &assigns[0].0,
        orchestrator_pid(&events),
        Some("fault.non-zero-exit"),
        "task.escalated",
    );
    let complete = journal_rows(&home, KIND_TASK_COMPLETE);
    assert!(
        complete.iter().all(|row| row.intent != "task.reassigned"),
        "an unarmed resolver never claims a re-dispatch: {complete:?}"
    );
    assert_orphaned_events(&events, 1, &[(&crashed, "escalate-to-operator")]);
    assert_stopped(
        &output,
        1,
        "fault.non-zero-exit",
        "escalate-to-operator (reassign-to-replica needs a prior distillate)",
    );
    assert_anomaly_flagged(&output, 1, crashed.worker_pid);
    assert!(
        named(&events, "distillate_written").is_empty(),
        "{events:?}"
    );
    assert!(tl_rows(&home, KIND_DISTILLATE).is_empty());
    assert_eq!(named(&events, "delegation_routed").len(), 1);
    assert!(named(&events, "delegation_completed").is_empty());
    assert!(named(&events, "dispatch_halted").is_empty());
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
