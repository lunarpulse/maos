#![forbid(unsafe_code)]

//! JB-3 (P0) — self-tuning epistemic halt fires through the production daemon.
//!
//! Integration-level subprocess test: spawns `maos run butler --once` and
//! asserts the halt event, its render-string (shared constant oracle), and
//! stderr visibility. No PTY, no JourneyWorld — pure subprocess.

use std::process::Command;

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// Unique on-disk state root so parallel subprocesses don't contend on the
/// shared `~/.local/share/maos` SQLite audit DB / journal (Story 8.11 lesson).
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
    let path = std::env::temp_dir().join(format!("maos-jb3-{tag}-{nanos}"));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

fn maos_cmd() -> Command {
    let cmd = if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        Command::new(bin)
    } else {
        let debug_bin = std::path::Path::new(workspace_root()).join("target/debug/maos");
        if debug_bin.exists() {
            Command::new(debug_bin)
        } else {
            let mut c = Command::new("cargo");
            c.args(["run", "-p", "maos-bin", "--"]);
            c
        }
    };
    cmd
}

#[test]
fn jb3_self_tunes_via_belief_variance_halt() {
    let home = isolated_data_home("halt");
    let output = maos_cmd()
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // (a) Process exits 0.
    assert!(
        output.status.success(),
        "maos run butler --once must exit 0; stderr:\n{stderr}"
    );

    // (b-c) Parse stdout JSON lines; find the halt event and verify its render
    // field matches the shared constant oracle (FORB 3 fix — compile-error on
    // drift between production and harness).
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    let halt = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("halt"))
        .expect("stdout must contain a halt event");

    let expected_render = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert_eq!(
        halt.get("render").and_then(|v| v.as_str()),
        Some(expected_render.as_str()),
        "halt render-string must equal the shared production constant"
    );

    // (d) AC5 — halt visible to director on stderr.
    assert!(
        stderr.contains(&expected_render),
        "REGRESSION: production halt render-string {expected_render:?} must appear on stderr \
         (AC5 — halt visible to director). stderr was:\n{stderr}"
    );
}
