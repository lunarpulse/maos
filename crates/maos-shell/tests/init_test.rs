//! `maos init` tests — subprocess-isolated, parallel-safe.

use std::process::Command;

fn workspace_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{d}/../.."))
        .unwrap_or_else(|_| ".".into())
}

fn maos_bin() -> std::path::PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let workspace = std::path::Path::new(&dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let path = workspace.join("target").join(profile).join("maos");
    assert!(
        path.exists(),
        "maos binary not found at {path:?}; run `cargo build -p maos-bin` first"
    );
    path
}

fn isolated_home(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "maos-init-test-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn init_creates_config_and_dirs() {
    let home = isolated_home("create");
    let bin = maos_bin();
    let out = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out.status.success(),
        "init should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        home.join("config.toml").exists(),
        "config.toml should exist"
    );
    assert!(home.join("skills").is_dir(), "skills dir should exist");
    assert!(home.join("audit").is_dir(), "audit dir should exist");
    assert!(home.join("journal").is_dir(), "journal dir should exist");
    assert!(home.join("logs").is_dir(), "logs dir should exist");
    let cfg = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(cfg.contains("[slots]"), "config should declare slots");
    assert!(
        cfg.contains("[retention]"),
        "config should declare retention"
    );
    assert!(
        cfg.contains("default = \"persist\""),
        "retention default should be persist"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn init_is_idempotent() {
    let home = isolated_home("idempotent");
    let bin = maos_bin();
    // First init.
    let out1 = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out1.status.success(),
        "first init: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let before = std::fs::read_to_string(home.join("config.toml")).unwrap();
    // Second init — should say already initialized and NOT clobber.
    let out2 = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init again");
    assert!(out2.status.success(), "second init should succeed");
    let stdout = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout.contains("already initialized"),
        "second init should say already initialized: {stdout}"
    );
    let after = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert_eq!(before, after, "config should not be clobbered");
    let _ = std::fs::remove_dir_all(&home);
}
