#![forbid(unsafe_code)]

//! Story 8.11 · AC1 — `maos run <manifest> --once` headless smoke.
//!
//! Proves the production run surface end-to-end: `maos run spirits/butler/
//! manifest.toml --once` loads Butler through the canonical admission path,
//! the serving loop drives its real `on_idle`, and Butler's calendar-conflict
//! scalar **fires its epistemic halt through the production daemon's own wiring**
//! (the firing Story 8.10·AC1 deliberately did NOT claim). Headless via `--once`.

use std::process::Command;

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// A unique on-disk state root so parallel `maos run` subprocesses do not
/// contend on the shared `~/.local/share/maos` SQLite audit DB / journal.
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
    let path = std::env::temp_dir().join(format!("maos-8-11-{tag}-{nanos}"));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

fn run_once(manifest: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", manifest, "--once"])
        .env("XDG_DATA_HOME", isolated_data_home("smoke").path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin")
}

#[test]
fn maos_run_butler_once_fires_epistemic_halt_through_production_wiring() {
    let output = run_once("spirits/butler/manifest.toml");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "maos run butler --once must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // (1) Butler loaded with the production boot-loud port wired.
    let loaded = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .expect("a spirit_loaded event");
    assert_eq!(
        loaded.get("spirit_id").and_then(|v| v.as_str()),
        Some("butler")
    );
    assert_eq!(
        loaded.get("boot_loud_port").and_then(|v| v.as_bool()),
        Some(true),
        "Butler must boot with the production EpistemicScalarPort wired"
    );

    // (2) on_idle fired through the dispatcher.
    assert!(
        events
            .iter()
            .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("on_idle_fired")),
        "on_idle must fire"
    );

    // (3) the epistemic halt fired through the daemon's OWN wiring, and the
    //     rendered screen-string is the SHARED constant (AC5(f) parity).
    let halt = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("halt"))
        .expect("the calendar-conflict halt must fire through production wiring");
    assert_eq!(
        halt.get("render").and_then(|v| v.as_str()),
        Some(butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE).as_str()),
        "the halt render-string must be the shared production/harness constant"
    );
}
