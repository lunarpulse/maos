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
//! - (b) a Spirit whose declared Halt rules are all OUTSIDE the halt-port set
//!   with no port must boot **SUCCESSFULLY** — the guard that the
//!   posture-keyed predicate does not over-fire on any declared halt (written
//!   to pass, NOT panic). Story 19-4 (D-19-4-03 B) admitted the
//!   Orchestrator's `dispatch_ambiguity` into the set, so no shipped manifest
//!   is outside it any more: the guard boots a TEST-ONLY manifest
//!   (`tests/fixtures/boot_loud_unlisted_halt_manifest.toml`) whose one Halt
//!   tag no transport lists, and the shipped Orchestrator now boots WITH the
//!   port requirement.

use std::process::Command;

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;

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
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        // The test seam that simulates "forgot to wire the port" — the 8.1 footgun.
        .env("MAOS_TEST_ONLY_STRIP_SCALAR_PORT", "1")
        // Story 16-1 / D-16-1-Q: empty scratch "HOME" so this root never reads
        // the developer's $HOME/.maos/control.json (EndpointInUse collision).
        .env("HOME", doorless_home::doorless_home())
        .env(
            "XDG_DATA_HOME",
            isolated_data_home("bootloud-a").path.clone(),
        )
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

/// The test-only over-fire subject (Story 19-4, D-19-4-03 B): the Orchestrator
/// class with ONE Halt rule whose tag is outside the halt-port set.
const UNLISTED_HALT_MANIFEST: &str =
    "crates/maos-bin/tests/fixtures/boot_loud_unlisted_halt_manifest.toml";

/// `maos run <manifest> --once` on a doorless home; the `spirit_loaded` event's
/// `boot_loud_port`, after asserting a clean boot.
fn boot_loud_port_of(manifest: &str, tag: &str) -> Option<bool> {
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", manifest, "--once"])
        .env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", isolated_data_home(tag).path.clone())
        .env_remove("MAOS_INFERENCE_MODE")
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "{manifest} MUST boot clean — the posture-keyed boot-loud predicate must not \
         over-fire; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let loaded = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .unwrap_or_else(|| panic!("{manifest} must load; stdout:\n{stdout}"));
    loaded.get("boot_loud_port").and_then(|v| v.as_bool())
}

/// (b) A Spirit with Halt rules outside the halt-port set, no port → boots clean
/// (the over-fire guard). Story 18-3 (D-18-3-09 A) moved the Researcher's tags
/// into the set; Story 19-4 (D-19-4-03 B) moved the Orchestrator's
/// `dispatch_ambiguity` in, so the guard boots the test-only manifest whose one
/// Halt tag is unlisted.
#[test]
fn deterministic_posture_spirit_without_port_boots_clean() {
    assert_eq!(
        boot_loud_port_of(UNLISTED_HALT_MANIFEST, "bootloud-b"),
        Some(false),
        "a Spirit whose only Halt rule is outside the halt-port set must boot WITHOUT \
         requiring an EpistemicScalarPort"
    );
}

/// Story 19-4 (AC2.4, D-19-4-03 B) — the flip itself, at process level: the
/// shipped Orchestrator's `dispatch_ambiguity` Halt rule now has a writer, so its
/// boot requires (and is bound to) the scalar port.
#[test]
fn the_shipped_orchestrator_boots_with_the_halt_port_required() {
    assert_eq!(
        boot_loud_port_of("spirits/orchestrator/manifest.toml", "bootloud-c"),
        Some(true),
        "the Orchestrator's `dispatch_ambiguity` Halt rule is in the halt-port set \
         (D-19-4-03 B): its boot requires the EpistemicScalarPort"
    );
}
