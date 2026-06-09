//! Story 8.14a — J0 surface integration smoke.
//!
//! Exercises: `maos init`, `maos shell`, `maos audit query`.

use std::process::Command;

fn workspace_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{d}/../.."))
        .unwrap_or_else(|_| ".".into())
}

fn isolated_home(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("maos-8-14a-{label}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn init_creates_dot_maos_and_is_idempotent() {
    let home = isolated_home("init");
    let bin = env!("CARGO_BIN_EXE_maos");

    // First init.
    let out = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(out.status.success(), "init exit 0: {}", String::from_utf8_lossy(&out.stderr));
    assert!(home.join("config.toml").exists());

    // Second init — idempotent.
    let out2 = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init again");
    assert!(out2.status.success());
    let stdout = String::from_utf8_lossy(&out2.stdout);
    assert!(stdout.contains("already initialized"), "second init should say already initialized: {stdout}");

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn shell_hello_spirit_say_hi_and_audit_query() {
    let home = isolated_home("shell");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output();

    // Shell turn: @hello-spirit say hi
    let mut child = Command::new(bin)
        .arg("shell")
        .env("MAOS_HOME", &home)
        .env("NO_COLOR", "1")
        .current_dir(workspace_root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn maos shell");

    {
        let stdin = child.stdin.as_mut().unwrap();
        use std::io::Write;
        writeln!(stdin, "@hello-spirit say hi").unwrap();
    }

    let out = child.wait_with_output().expect("shell failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Should contain the honest disclosure shape.
    assert!(
        stdout.contains("MAOS reference Spirit") || stderr.contains("MAOS reference Spirit"),
        "shell should render hello-spirit response. stdout={stdout} stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn audit_query_plain_no_ansi() {
    let home = isolated_home("audit");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init so audit DB path exists under MAOS_HOME.
    let _ = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output();

    // audit query --plain should emit zero ANSI bytes even when empty.
    let out = Command::new(bin)
        .args(["audit", "query", "--plain"])
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos audit query");

    // Empty DB is OK for this smoke; just assert no ANSI escape bytes.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.as_bytes().contains(&0x1b), "--plain must emit zero ANSI bytes");

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn shell_plain_no_ansi() {
    let home = isolated_home("shell-ansi");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output();

    // Shell with --plain should emit zero ANSI bytes.
    let mut child = Command::new(bin)
        .args(["shell", "--plain"])
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn maos shell --plain");

    {
        let stdin = child.stdin.as_mut().unwrap();
        use std::io::Write;
        writeln!(stdin, "@hello-spirit say hi").unwrap();
    }

    let out = child.wait_with_output().expect("shell failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.as_bytes().contains(&0x1b), "--plain shell must emit zero ANSI bytes, got: {stdout}");

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn audit_query_returns_shell_turn_rows() {
    let home = isolated_home("audit-shell");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = Command::new(bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output();

    // Shell turn: @hello-spirit say hi
    let mut child = Command::new(bin)
        .arg("shell")
        .env("MAOS_HOME", &home)
        .env("NO_COLOR", "1")
        .current_dir(workspace_root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn maos shell");

    {
        let stdin = child.stdin.as_mut().unwrap();
        use std::io::Write;
        writeln!(stdin, "@hello-spirit say hi").unwrap();
    }

    let out = child.wait_with_output().expect("shell failed");
    assert!(out.status.success(), "shell should succeed: {}", String::from_utf8_lossy(&out.stderr));

    // Now query the audit log.
    let audit_out = Command::new(bin)
        .args(["audit", "query", "--format", "plain"])
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to run maos audit query");

    let audit_stdout = String::from_utf8_lossy(&audit_out.stdout);
    // The audit log should contain at least one row from the shell turn.
    // With record_invocation writing CapAuditEvent::Invocation, the plain format
    // should emit at least one line.
    assert!(!audit_stdout.trim().is_empty(), "audit query should return rows after shell interaction, got: {audit_stdout}");

    let _ = std::fs::remove_dir_all(&home);
}
