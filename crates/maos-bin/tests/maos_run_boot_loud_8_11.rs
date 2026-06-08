#![forbid(unsafe_code)]

//! Story 8.11 · AC6 (FORK D, boot-loud) — the negative-boot test IS the done-bar.
//!
//! Because Story 8.1's review marked a fix applied that wasn't, "boot-loud is
//! implemented" is not an acceptable claim — only a failing test is. This pair
//! is the gate:
//!
//! - (a) a **halt-posture** Spirit (Butler, `allowed_max = autonomous-with-halt`)
//!   loaded with its EpistemicScalarPort STRIPPED must **fatally fail boot**
//!   (specific `Err`, not a panic); the serving loop is never reached. Goes RED
//!   the moment anyone re-adds a `None`/store-only escape.
//! - (b) a **deterministic-posture** Spirit (Researcher, `assistive`) with no
//!   port must boot **SUCCESSFULLY** — the guard that the posture-keyed predicate
//!   does not over-fire and decapitate Researcher (written to pass, NOT panic).

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

/// (a) Halt-posture Spirit without a port → FATAL boot, serving loop never entered.
#[test]
fn halt_posture_spirit_without_port_fatally_fails_boot() {
    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        // The test seam that simulates "forgot to wire the port" — the 8.1 footgun.
        .env("MAOS_TEST_ONLY_STRIP_SCALAR_PORT", "1")
        .env("XDG_DATA_HOME", isolated_data_home("bootloud-a").path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        !output.status.success(),
        "a halt-posture Spirit with no EpistemicScalarPort MUST fail boot (the 8.1 \
         None-footgun is fail-closed); instead it exited 0"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("FATAL boot") && stderr.contains("None-footgun"),
        "boot failure must be the explicit fail-closed error, not an incidental crash; got:\n{stderr}"
    );
    // The serving loop must NEVER be reached: no spirit_loaded / on_idle events.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("on_idle_fired"),
        "on_idle must NOT fire when boot fails closed"
    );
}

/// (b) Deterministic-posture Spirit without a port → boots clean (the over-fire guard).
#[test]
fn deterministic_posture_spirit_without_port_boots_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", isolated_data_home("bootloud-b").path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "Researcher (assistive, no port) MUST boot clean — the posture-keyed boot-loud \
         predicate must not decapitate a deterministic Spirit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let loaded = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .expect("Researcher must load");
    assert_eq!(
        loaded.get("boot_loud_port").and_then(|v| v.as_bool()),
        Some(false),
        "Researcher must boot WITHOUT requiring an EpistemicScalarPort"
    );
}
