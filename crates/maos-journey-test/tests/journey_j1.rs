#![forbid(unsafe_code)]

//! J1 — Founder-loop journey (Grade A: real `maos run` topology manifest).
//!
//! The founder-class Spirits now load through the production composition root.
//! The topology path drives Orchestrator/Architect/Reviewer under one scheduler;
//! the standalone check pins the removed 8.12 FounderLoopClass short-circuit.
//!
//! Story 19-4 (AC4.2) — the resume-continuity oracle is the founder loop
//! itself: the three-section epic under strict replay judges section 2
//! ambiguous, the kernel halts the Orchestrator, the operator answers over the
//! door (`maosctl halt resolve … --kind provided-context`), the held section
//! resumes with the answer, the epic finishes, and the drain's FR17 digest
//! cites real rows — the pre-halt frame among its RESOLVED halts. Every
//! bounded wait goes through the harness (`wait_until`, `run_bounded`), per
//! the JB-7 / H4 no-wallclock guard.

use std::{collections::BTreeSet, process::Command};

use maos_journey_test::{run_bounded, wait_until, AuditDb, JourneyWorld};

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

#[path = "support/binaries.rs"]
mod binaries;

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

fn production_command() -> Command {
    let binary = maos_bin();
    let profile = std::path::Path::new(&binary)
        .parent()
        .expect("maos binary profile directory");
    let mut paths = vec![profile.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let mut command = Command::new(binary);
    command.env_clear();
    add_t3_environment(&mut command);
    command.env("PATH", std::env::join_paths(paths).expect("valid PATH"));
    command
}

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        return bin.to_string();
    }
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target"));
    let target_dir = if target_dir.is_absolute() {
        target_dir
    } else {
        workspace_root().join(target_dir)
    };
    let debug_bin = target_dir.join("debug/maos");
    if debug_bin.exists() {
        return debug_bin.to_string_lossy().into_owned();
    }
    panic!("maos binary not found — run `cargo build -p maos-bin` first");
}

/// Parse stdout lines as JSON, collecting successfully parsed values.
fn parse_json_lines(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn j1_founder_loop_topology_run_once() {
    if !t3_substrate::require_t3("J1 founder-loop journey delegated Worker") {
        return;
    }
    let home = tempfile::TempDir::new().unwrap();
    let mut command = production_command();
    let output = command
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(home.path()))
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        // 15-6 §A6 review P1: cassette-free child; never inherit a
        // job-level MAOS_INFERENCE_MODE (nightly rerecord leg).
        .env_remove("MAOS_INFERENCE_MODE")
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn founder-loop topology");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "founder-loop topology should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // Structured JSON assertions — no string scraping.
    let events = parse_json_lines(&stdout);
    assert!(
        !events.is_empty(),
        "topology stdout must contain JSON events; raw stdout:\n{stdout}"
    );

    // Collect spirit_ids from spirit_loaded events.
    let loaded_ids: BTreeSet<&str> = events
        .iter()
        .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .filter_map(|e| e.get("spirit_id").and_then(|v| v.as_str()))
        .collect();
    let expected: BTreeSet<&str> = ["orchestrator", "architect", "reviewer"]
        .iter()
        .copied()
        .collect();
    assert_eq!(
        loaded_ids, expected,
        "topology must load exactly {{orchestrator, architect, reviewer}}; got {loaded_ids:?}"
    );

    // Assert a drain event with topology: true exists.
    let has_topology_drain = events.iter().any(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("drain")
            && e.get("topology").and_then(|v| v.as_bool()) == Some(true)
    });
    assert!(
        has_topology_drain,
        "topology run should terminate through the drain-complete seam; events:\n{events:?}"
    );

    // ── j1-crosshost-1a AC1.7 — the leg this journey exists to prove ──────────
    //
    // Before this story `journey_j1` asserted NOTHING about the Worker: it proved
    // three class Spirits loaded and the run drained. The delegated Worker could
    // have been skipped entirely and this test stayed green. These assertions are
    // the positive control.
    let event = |name: &str| {
        events
            .iter()
            .find(|e| e.get("event").and_then(|v| v.as_str()) == Some(name))
            .unwrap_or_else(|| panic!("missing `{name}` event; events:\n{events:?}"))
    };

    // (1) A real `task.assign` was routed over the loopback A2A layer, carrying the
    //     ADR-012 effect-authority intent — not a job title, not `task.assign`
    //     (which is not even a legal consent intent).
    let routed = event("delegation_routed");
    assert_eq!(
        routed.get("intent").and_then(|v| v.as_str()),
        Some("development-task:write-workspace"),
        "the delegation must carry the namespaced effect-authority intent"
    );
    assert_eq!(
        routed.get("to_host").and_then(|v| v.as_str()),
        Some("developer-remote-host")
    );
    assert_eq!(
        routed.get("recipient").and_then(|v| v.as_str()),
        Some("developer-remote")
    );
    let delegated_goal = routed
        .get("goal")
        .and_then(|v| v.as_str())
        .expect("the routed frame must carry a goal drained from its TaskAssign payload");
    assert!(
        !delegated_goal.is_empty(),
        "an empty goal would mean the consumer drained nothing"
    );

    // (2) The Worker was admitted through the FRAME, not an env var. `frame_borne`
    //     is false if the topology entry lost its `host` key, which is exactly how
    //     this leg would silently regress to a local load.
    let admit = event("topology_worker_admit");
    assert_eq!(
        admit.get("frame_borne").and_then(|v| v.as_bool()),
        Some(true),
        "the Worker must be admitted from a routed frame, never from an environment variable"
    );

    // (3) The Worker actually RAN: a real child process, clean exit.
    let loaded = event("cli_wrapper_loaded");
    let child_pid = loaded
        .get("child_pid")
        .and_then(|v| v.as_u64())
        .expect("a real subprocess reports its pid");
    assert_ne!(
        child_pid,
        std::process::id() as u64,
        "the Worker must be a separate process, not the harness"
    );
    let exit = event("cli_wrapper_exit");
    assert_eq!(exit.get("is_crash").and_then(|v| v.as_bool()), Some(false));

    // (4) And COMPLETED, by the adapter's oracle over captured output.
    let completion = event("worker_completion");
    assert_eq!(
        completion.get("completed").and_then(|v| v.as_bool()),
        Some(true),
        "the delegated Worker must complete; events:\n{events:?}"
    );

    // (5) The completion was journaled as a real frame and the Orchestrator
    //     received it, closing the FR20 safe point. `orchestrator_frames_drained`
    //     of 0 would mean the completion frame was emitted into a void.
    let completed = event("delegation_completed");
    assert_eq!(
        completed.get("result").and_then(|v| v.as_str()),
        Some("completed")
    );
    assert!(
        completed
            .get("orchestrator_frames_drained")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "the TaskComplete frame must reach the Orchestrator's handle; events:\n{events:?}"
    );
    assert_eq!(
        completed
            .get("orchestrator_safe_point")
            .and_then(|v| v.as_bool()),
        Some(true),
        "the in-flight delegation must be closed out (FR20 safe point re-opened)"
    );
}

#[test]
fn j1_founder_class_standalone_load_succeeds() {
    let home = tempfile::TempDir::new().unwrap();
    let mut command = production_command();
    let output = command
        .args(["run", "spirits/orchestrator/manifest.toml", "--once"])
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .env_remove("MAOS_INFERENCE_MODE")
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn maos run for founder-class");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "maos run of a founder-class spirit should load; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    let events = parse_json_lines(&stdout);

    // Verify spirit_loaded event for orchestrator.
    let loaded = events.iter().any(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded")
            && e.get("spirit_id").and_then(|v| v.as_str()) == Some("orchestrator")
    });
    assert!(
        loaded,
        "standalone founder-class load should emit spirit_loaded for orchestrator; events:\n{events:?}"
    );

    // Verify drain event.
    let has_drain = events
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("drain"));
    assert!(
        has_drain,
        "standalone founder-class load should emit a drain event; events:\n{events:?}"
    );
}

/// Story 19-4 (D-19-4-04 A / D-19-4-09 A) — the epic topology and its
/// fixture: three `### Story` sections, section 2 under-specified.
const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
const FOUNDER_LOOP_EPIC: &str = "tests/fixtures/j1/founder-loop-epic.md";
/// s1 clear, s2 ambiguous (> 0.5), s3 clear, in judgement order.
const FOUNDER_LOOP_CASSETTE: &str = "crates/maos-journey-test/cassettes/j1/founder-loop.json";
const FOUNDER_LOOP_CLEAR_TWIN: &str =
    "crates/maos-journey-test/cassettes/j1/founder-loop.clear-twin.json";
/// The operator's answer to the section-2 halt.
const J1_CLARIFICATION: &str =
    "faster = the nightly import of 1e6 rows finishes in under 10 minutes";

fn maosctl_bin() -> String {
    binaries::resolve("maosctl", option_env!("CARGO_BIN_EXE_maosctl"))
}

fn ctl(world: &JourneyWorld, args: &[&str]) -> maos_journey_test::BoundedRun {
    run_bounded(world, &maosctl_bin(), args, &[], 60, "maosctl")
}

/// Drain one child pipe to EOF on its own thread, so a child writing past the
/// pipe buffer never blocks while the journey waits on the door.
fn read_to_end(
    pipe: Option<impl std::io::Read + Send + 'static>,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = std::io::Read::read_to_string(&mut pipe, &mut text);
        }
        text
    })
}

fn named<'a>(events: &'a [serde_json::Value], name: &str) -> Vec<&'a serde_json::Value> {
    events
        .iter()
        .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some(name))
        .collect()
}

/// AC4.2 / AC5 / AC3 — the J1 resume continuity, end to end in ONE boot.
///
/// `maos init` gives the home its door; `maos run <epic topology> --epic …
/// --once` replays the founder-loop cassette strictly (`MAOS_REPLAY_STRICT=1`
/// set on the CHILD — the env is cleared) with a hold far longer than the
/// waits below, so only the door's resolution can release section 2. The
/// pre-halt ref is the halt's Transparency-Log frame id that `dispatch_judged`
/// names; the post-resume digest must cite it in `resolved_halts`.
#[test]
fn j1_resume_continuity_ref_identity_oracle() {
    if !t3_substrate::require_t3("J1 resume-continuity topology Workers") {
        return;
    }
    let world = JourneyWorld::builder().audit(AuditDb::temp()).build();
    let init = run_bounded(&world, &maos_bin(), &["init"], &[], 60, "maos init");
    assert_eq!(init.code, Some(0), "maos init; stderr:\n{}", init.stderr);
    let home = std::path::PathBuf::from(&world.env()["MAOS_HOME"]);
    assert!(
        home.join("control.json").is_file(),
        "the door's control.json"
    );

    let root = workspace_root();
    let mut command = production_command();
    command
        .args(["run", EPIC_TOPOLOGY, "--epic", FOUNDER_LOOP_EPIC, "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home))
        .env("XDG_DATA_HOME", &world.env()["XDG_DATA_HOME"])
        .env("MAOS_HOME", &home)
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", root.join(FOUNDER_LOOP_CASSETTE))
        .env("MAOS_EPIC_HALT_HOLD_SECS", "900")
        .env("NO_COLOR", "1")
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn the founder-loop epic");
    let stdout_reader = read_to_end(child.stdout.take());
    let stderr_reader = read_to_end(child.stderr.take());

    // The halt reaches the door once section 1's Worker has run; an early
    // exit ends the wait and fails below with the run's stderr.
    let mut exited_early = false;
    let mut halt_id = String::new();
    let raised = wait_until(100, "the section-2 halt reaches the door", || {
        if matches!(child.try_wait(), Ok(Some(_))) {
            exited_early = true;
            return true;
        }
        let list = ctl(&world, &["halt", "list", "--spirit", "orchestrator"]);
        let ids: Vec<String> = list
            .stdout
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|row| row["record"] == "raised")
            .filter_map(|row| row["halt_id"].as_str().map(String::from))
            .collect();
        match ids.as_slice() {
            [only] => {
                halt_id = only.clone();
                true
            }
            _ => false,
        }
    });
    if !raised || exited_early {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "no halt reached the door (exited early: {exited_early}); stderr:\n{}",
            stderr_reader.join().unwrap_or_default()
        );
    }
    let resolve = ctl(
        &world,
        &[
            "halt",
            "resolve",
            &halt_id,
            "--spirit",
            "orchestrator",
            "--kind",
            "provided-context",
            "--text",
            J1_CLARIFICATION,
        ],
    );
    assert_eq!(
        resolve.code,
        Some(0),
        "door resolve exit 0; stdout:\n{}\nstderr:\n{}",
        resolve.stdout,
        resolve.stderr
    );
    let mut status = None;
    let finished = wait_until(100, "the resumed epic drains", || {
        status = child.try_wait().ok().flatten();
        status.is_some()
    });
    if !finished {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    assert!(
        finished && status.is_some_and(|s| s.success()),
        "the resumed epic must drain and exit 0 (finished: {finished}); stderr:\n{stderr}"
    );
    let events = parse_json_lines(&stdout);

    // The judgement halted section 2 through the kernel and named the pre-halt
    // ref; the door's halt is that halt.
    let judged = named(&events, "dispatch_judged");
    assert_eq!(
        judged
            .iter()
            .map(|e| e["section"].as_u64())
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)],
        "each section judged once, in order: {events:?}"
    );
    assert_eq!(judged[1]["tag"], "dispatch_ambiguity");
    assert_eq!(judged[1]["halt_id"].as_str(), Some(halt_id.as_str()));
    assert!(judged[0]["halt_id"].is_null() && judged[2]["halt_id"].is_null());
    let pre_halt_ref = judged[1]["halt_frame_id"]
        .as_str()
        .expect("the halted judgement names the halt's TL frame")
        .to_owned();
    let halted = named(&events, "dispatch_halted");
    assert_eq!(halted.len(), 1, "dispatch_halted once: {events:?}");
    let resumed = named(&events, "dispatch_resumed");
    assert_eq!(resumed.len(), 1, "dispatch_resumed once: {events:?}");
    assert_eq!(resumed[0]["section"], 2);
    assert_eq!(resumed[0]["halt_ids"], serde_json::json!([halt_id]));
    assert_eq!(resumed[0]["provided_context"].as_bool(), Some(true));

    // The whole epic ran: three frames, section 2's carrying the answer.
    let goals: Vec<&str> = named(&events, "delegation_routed")
        .iter()
        .filter_map(|e| e["goal"].as_str())
        .collect();
    assert_eq!(goals.len(), 3, "{events:?}");
    assert_eq!(
        goals
            .iter()
            .map(|g| g.contains(J1_CLARIFICATION))
            .collect::<Vec<_>>(),
        vec![false, true, false],
        "only section 2's goal carries the operator's answer: {goals:?}"
    );
    let completed = named(&events, "delegation_completed");
    assert_eq!(completed.len(), 3, "{events:?}");
    assert!(completed.iter().all(|e| e["result"] == "completed"));

    // AC3 — every digest ref resolves to a real audit row of the right kind,
    // and the pre-halt ref is cited as RESOLVED, never open.
    let audit = ctl(
        &world,
        &["audit", "query", "--range", "1d", "--format", "ndjson"],
    );
    assert_eq!(
        audit.code,
        Some(0),
        "audit query; stderr:\n{}",
        audit.stderr
    );
    let kind_of: std::collections::BTreeMap<String, String> = audit
        .stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|row| {
            Some((
                row["frame_id"].as_str()?.to_owned(),
                row["kind"].as_str()?.to_owned(),
            ))
        })
        .collect();
    let digests = named(&events, "morning_digest");
    assert_eq!(
        digests.len(),
        1,
        "one digest at the --epic drain: {events:?}"
    );
    let digest = &digests[0]["digest"];
    let refs = |field: &str| -> Vec<String> {
        digest[field]
            .as_array()
            .unwrap_or_else(|| panic!("digest.{field}[]: {digest}"))
            .iter()
            .filter_map(|e| e["source_log_ref"].as_str().map(String::from))
            .collect()
    };
    let completions = refs("completed");
    assert!(completions.len() >= 3, "≥ 3 cited completions: {digest}");
    for r in &completions {
        assert!(
            matches!(
                kind_of.get(r).map(String::as_str),
                Some("task.complete" | "cli.subprocess.output")
            ),
            "completion ref {r} names a real completion row: {digest}"
        );
    }
    let open: BTreeSet<String> = refs("open_halts").into_iter().collect();
    let resolved: BTreeSet<String> = refs("resolved_halts").into_iter().collect();
    for r in open.iter().chain(&resolved) {
        assert_eq!(
            kind_of.get(r).map(String::as_str),
            Some("epistemic.halt"),
            "halt ref {r} names a real halt row: {digest}"
        );
    }
    assert!(
        resolved.contains(&pre_halt_ref),
        "the post-resume digest cites the pre-halt ref in resolved_halts: {digest}"
    );
    assert!(
        !open.contains(&pre_halt_ref),
        "a resolved halt is never reported open: {digest}"
    );
}

/// AC2.6 — the same journey and strict replay contract with the clear twin:
/// section 2 is judged below the threshold, so nothing halts or resumes.
#[test]
fn j1_clear_judgement_twin_completes_without_halt_or_resume() {
    if !t3_substrate::require_t3("J1 clear-twin topology Workers") {
        return;
    }
    let world = JourneyWorld::builder().audit(AuditDb::temp()).build();
    let init = run_bounded(&world, &maos_bin(), &["init"], &[], 60, "maos init");
    assert_eq!(init.code, Some(0), "maos init; stderr:\n{}", init.stderr);
    let home = std::path::PathBuf::from(&world.env()["MAOS_HOME"]);
    assert!(
        home.join("control.json").is_file(),
        "the door's control.json"
    );
    let root = workspace_root();
    let mut command = production_command();
    command
        .args(["run", EPIC_TOPOLOGY, "--epic", FOUNDER_LOOP_EPIC, "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home))
        .env("XDG_DATA_HOME", &world.env()["XDG_DATA_HOME"])
        .env("MAOS_HOME", &home)
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", root.join(FOUNDER_LOOP_CLEAR_TWIN))
        .env("MAOS_EPIC_HALT_HOLD_SECS", "0")
        .env("NO_COLOR", "1")
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn the clear-twin epic");
    let stdout_reader = read_to_end(child.stdout.take());
    let stderr_reader = read_to_end(child.stderr.take());
    let mut status = None;
    let finished = wait_until(100, "the clear-twin epic drains", || {
        status = child.try_wait().ok().flatten();
        status.is_some()
    });
    if !finished {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    assert!(
        finished && status.is_some_and(|s| s.success()),
        "the clear twin must drain and exit 0 (finished: {finished}); stderr:\n{stderr}"
    );
    let events = parse_json_lines(&stdout);
    let judged = named(&events, "dispatch_judged");
    assert_eq!(
        judged
            .iter()
            .map(|e| e["section"].as_u64())
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)],
        "each new section judged exactly once: {events:?}"
    );
    assert!(judged.iter().all(|e| e["halt_id"].is_null()), "{events:?}");
    assert!(named(&events, "dispatch_halted").is_empty(), "{events:?}");
    assert!(named(&events, "dispatch_resumed").is_empty(), "{events:?}");
    assert_eq!(named(&events, "delegation_routed").len(), 3, "{events:?}");
    let completed = named(&events, "delegation_completed");
    assert_eq!(completed.len(), 3, "{events:?}");
    assert!(
        completed.iter().all(|e| e["result"] == "completed"),
        "{events:?}"
    );
    let digests = named(&events, "morning_digest");
    assert_eq!(digests.len(), 1, "{events:?}");
    for field in ["open_halts", "resolved_halts"] {
        assert!(
            digests[0]["digest"][field]
                .as_array()
                .is_some_and(Vec::is_empty),
            "clear twin has no halt citations in {field}: {digests:?}"
        );
    }
}
