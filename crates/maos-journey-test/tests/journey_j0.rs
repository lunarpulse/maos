#![forbid(unsafe_code)]

//! J0 — Evaluator surface journey (Grade A: production entry surface).
//!
//! J0 = `maos init` → shell banner → structured intro → ambiguity halt.
//! At v0.3-β the shell is stdin-line-based; the PTY drives it with write.

use maos_journey_test::{AuditDb, JourneyWorld, Pty};

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

#[test]
fn j0_init_creates_config() {
    let home = tempfile::TempDir::new().unwrap();
    let output = std::process::Command::new(maos_bin())
        .args(["init"])
        .env("MAOS_HOME", home.path())
        .env("XDG_DATA_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn maos init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "maos init should exit 0; stderr:\n{stderr}"
    );
    assert!(
        home.path().join("config.toml").exists(),
        "maos init should create config.toml"
    );
    assert!(
        stdout.contains("initialized"),
        "maos init should confirm initialization; stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn j0_shell_banner_via_pty() {
    let audit = AuditDb::temp();
    let cassette = workspace_root()
        .join("crates/maos-journey-test/cassettes/j0/shell-intro.json");
    let world = JourneyWorld::builder()
        .audit(audit)
        .cassette(cassette.to_str().unwrap())
        .build();

    let cmd = format!("{}", maos_bin());
    let pty = Pty::spawn(&cmd, &world);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let screen = pty.screen();
        if screen.contains("maos shell") || screen.contains("hello-spirit") {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("j0: shell banner did not appear within 5s");
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let screen = pty.screen();
    assert!(!screen.text().is_empty(), "PTY screen should have content");

    drop(pty);
}
