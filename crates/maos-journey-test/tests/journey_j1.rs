#![forbid(unsafe_code)]

//! J1 — Founder-loop journey (Grade A: real `maos run` topology manifest).
//!
//! The founder-class Spirits now load through the production composition root.
//! The topology path drives Orchestrator/Architect/Reviewer under one scheduler;
//! the standalone check pins the removed 8.12 FounderLoopClass short-circuit.

use std::collections::BTreeSet;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        return bin.to_string();
    }
    let debug_bin = workspace_root().join("target/debug/maos");
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
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
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
        completed.get("orchestrator_safe_point").and_then(|v| v.as_bool()),
        Some(true),
        "the in-flight delegation must be closed out (FR20 safe point re-opened)"
    );
}

#[test]
fn j1_founder_class_standalone_load_succeeds() {
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/orchestrator/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
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

#[test]
fn j1_resume_continuity_ref_identity_oracle() {
    // Run 1: first topology run in a fresh temp home.
    let home1 = tempfile::TempDir::new().unwrap();
    let output1 = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("XDG_DATA_HOME", home1.path())
        .env("MAOS_HOME", home1.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn founder-loop topology (run 1)");

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    assert!(
        output1.status.success(),
        "run 1 should exit 0; stderr:\n{stderr1}\nstdout:\n{stdout1}"
    );

    let events1 = parse_json_lines(&stdout1);
    let has_drain1 = events1
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("drain"));
    assert!(has_drain1, "run 1 must produce a drain event");

    // Run 2: second topology run in a DIFFERENT temp home (cold-start negative control).
    let home2 = tempfile::TempDir::new().unwrap();
    let output2 = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("XDG_DATA_HOME", home2.path())
        .env("MAOS_HOME", home2.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn founder-loop topology (run 2)");

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        output2.status.success(),
        "run 2 should exit 0; stderr:\n{stderr2}\nstdout:\n{stdout2}"
    );

    let events2 = parse_json_lines(&stdout2);
    let has_drain2 = events2
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("drain"));
    assert!(has_drain2, "run 2 must produce a drain event");

    // Negative control: the two cold-start runs must produce non-identical
    // event streams, proving output is not hard-coded / compiled-in.
    // Use the `on_idle_fired` events' `outcome` field which contains wall-clock
    // nanoseconds — these MUST differ across separate process invocations.
    let extract_idle_outcomes = |events: &[serde_json::Value]| -> BTreeSet<String> {
        events
            .iter()
            .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some("on_idle_fired"))
            .filter_map(|e| e.get("outcome").and_then(|v| v.as_str()).map(String::from))
            .collect()
    };

    let outcomes1 = extract_idle_outcomes(&events1);
    let outcomes2 = extract_idle_outcomes(&events2);

    assert!(
        !outcomes1.is_empty(),
        "run 1 must emit on_idle_fired events; events:\n{events1:?}"
    );
    assert!(
        !outcomes2.is_empty(),
        "run 2 must emit on_idle_fired events; events:\n{events2:?}"
    );
    // Wall-clock nanosecond durations differ across invocations — if the two
    // sets are identical, the output is static/hard-coded (the old tautology bug).
    assert_ne!(
        outcomes1, outcomes2,
        "cold-start runs must produce distinct on_idle_fired outcomes, \
         proving output is not static.\nrun 1: {outcomes1:?}\nrun 2: {outcomes2:?}"
    );

    // D4 full halt/resume oracle deferred: the production resume seam (halt → persist
    // TrajectoryRef → resume → assert post_resume_digest.cited_refs.contains(pre_halt_ref))
    // is not yet implemented. When it lands, this test should capture the pre-halt
    // TrajectoryRef from run 1's stdout, then resume into the same MAOS_HOME and assert
    // the post-resume digest cites the original ref. For now we verify the topology
    // loads, drains, and produces distinct runtime-dependent output across cold starts.
}
