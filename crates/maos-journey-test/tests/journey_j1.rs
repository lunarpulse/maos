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
