#![forbid(unsafe_code)]

//! Story 8.11 · AC2/AC4 — the `--live` polarity is a TRAP; pin it with a test.
//!
//! Flag ABSENT (the hermetic-CI default) MUST be the deterministic, zero-network
//! path: `maos run researcher --once` (no `--live`, no `MAOS_ANTHROPIC_API_KEY`)
//! runs the deterministic survey and exits 0 — proving the daemon does NOT
//! require (or reach) a real provider on the default path. The `--live` real-driver
//! path is gated (it needs a configured key) and is exercised in the Tier-2
//! evidence pass, not hermetic CI.

use std::process::Command;

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// A unique on-disk state root so parallel subprocesses do not contend on the
/// shared SQLite audit DB / journal.
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

/// Default (no `--live`) → deterministic survey, zero network. If the daemon
/// wrongly selected the real driver, this would fail `Unconfigured` (no key).
#[test]
fn ci_default_uses_zero_network_deterministic_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        // Explicitly ensure no real provider is configured — the default path
        // must succeed regardless (it makes no network call).
        .env_remove("MAOS_ANTHROPIC_API_KEY")
        .env_remove("MAOS_OPENAI_API_KEY")
        .env("XDG_DATA_HOME", isolated_data_home("polarity").path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "the default (no --live) researcher run must succeed with NO provider key \
         configured — proving zero network / no real-driver dependency; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let loaded = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .expect("researcher must load");
    assert_eq!(
        loaded.get("live").and_then(|v| v.as_bool()),
        Some(false),
        "the default path must NOT be live"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("deterministic survey (no --live; zero network)"),
        "the daemon must announce the deterministic (non-live) survey path"
    );
}
