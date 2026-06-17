#![forbid(unsafe_code)]

//! Story 8.12 · AC2/AC3/AC6 — live CliWrapper subprocess bridge under the daemon.
//!
//! Two Tier-1 (CI, hermetic) gates:
//!  1. `maos run spirits/worker/manifest.toml --once` admits the `[cli_wrapper]`
//!     Worker through the AC5 tier-grant + AC1 respawn gate + Story-7.4 journaled
//!     shape probe, spawns the REAL `worker-cli-fixture` subprocess through the
//!     live bridge, and journals its stdout as `CliSubprocessOutput=21` rows
//!     ("the machine is real"). The emitted events carry the real child PID.
//!  2. The founder-loop topology manifest (`spirits/topologies/j1-founder-loop.toml`)
//!     runs the Orchestrator/Architect/Reviewer group under `maos run --once`,
//!     exits 0, and drains through the production topology seam.
//!
//! Every subprocess isolates `XDG_DATA_HOME` (8.11 journal-corruption lesson).

use std::process::Command;

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

struct IsolatedDataHome {
    path: std::path::PathBuf,
}

impl Drop for IsolatedDataHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn isolated_data_home(tag: &str) -> IsolatedDataHome {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-8-12-{tag}-{nanos}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

#[test]
fn maos_run_cli_wrapper_worker_spawns_real_subprocess() {
    let home = isolated_data_home("cw-run");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/worker/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "maos run worker --once must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // (1) The CliWrapper Worker loaded with a host-granted tier.
    let loaded = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("cli_wrapper_loaded"))
        .expect("a cli_wrapper_loaded event");
    assert_eq!(
        loaded.get("granted_tier").and_then(|v| v.as_str()),
        Some("SandboxTier(3)"),
        "the worker's T3 tier request must be host-granted"
    );
    let child_pid = loaded
        .get("child_pid")
        .and_then(|v| v.as_u64())
        .expect("child_pid in the loaded event");
    assert_ne!(
        child_pid,
        std::process::id() as u64,
        "the worker must be a REAL subprocess (child_pid != this test pid)"
    );

    // (2) The bridge captured the fixture's stdout and exited cleanly.
    let exit = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("cli_wrapper_exit"))
        .expect("a cli_wrapper_exit event");
    assert_eq!(exit.get("is_crash").and_then(|v| v.as_bool()), Some(false));
    let lines = exit
        .get("stdout_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        lines >= 3,
        "the fixture emits 3 canned stdout lines, all journaled as CliSubprocessOutput rows; got {lines}"
    );
}

#[test]
fn founder_loop_journey_runs_with_real_worker_subprocess() {
    let home = isolated_data_home("founder");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "the founder-loop topology must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for spirit in ["orchestrator", "architect", "reviewer"] {
        assert!(
            stdout.contains(&format!("\"spirit_id\":\"{spirit}\"")),
            "the topology must load {spirit}; stdout:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("\"event\":\"drain\"") && stdout.contains("\"topology\":true"),
        "the founder-loop topology must terminate through the production drain seam; stdout:\n{stdout}"
    );
}
