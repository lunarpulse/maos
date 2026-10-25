//! Story 8.14a — J0 surface integration smoke.
//!
//! Exercises: `maos init`, `maos shell`, `maos audit query`.

use std::path::Path;
use std::process::Command;

fn workspace_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{d}/../.."))
        .unwrap_or_else(|_| ".".into())
}

fn isolated_home(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "maos-8-14a-{label}-{}-{}",
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

/// 18-0 CR-6 — `maos init` with `HOME` and `XDG_DATA_HOME` inside the test's
/// own home and no inherited `MAOS_CRL_PATH`, so the CRL directory `init`
/// creates never lands in the runner's real `~/.local/share/maos/crl`.
fn init(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .arg("init")
        .env("MAOS_HOME", home)
        .env("HOME", home.join("user-home"))
        .env("XDG_DATA_HOME", home.join("xdg-data"))
        .env_remove("MAOS_CRL_PATH")
        .current_dir(workspace_root());
    command
}

#[test]
fn init_creates_dot_maos_and_is_idempotent() {
    let home = isolated_home("init");

    // First init.
    let out = init(&home).output().expect("failed to execute maos init");
    assert!(
        out.status.success(),
        "init exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(home.join("config.toml").exists());

    // Second init — idempotent.
    let out2 = init(&home)
        .output()
        .expect("failed to execute maos init again");
    assert!(out2.status.success());
    let stdout = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout.contains("already initialized"),
        "second init should say already initialized: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&home);
}

/// 18-0 AC8 (E16-A5, D-18-0-12 A) — `maos init` creates the CRL directory the
/// revocation poller reads, on a fresh home and again on an initialized one
/// (before the idempotent early return), and honours `MAOS_CRL_PATH`.
#[test]
fn init_creates_the_crl_directory_and_honours_maos_crl_path() {
    let home = isolated_home("crl");
    let crl = home.join("xdg-data").join("maos").join("crl");
    for run in ["first", "second"] {
        let out = init(&home).output().expect("failed to execute maos init");
        assert!(
            out.status.success(),
            "{run} init exit 0: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(crl.is_dir(), "{run} init: {} is a directory", crl.display());
    }
    let explicit = home.join("explicit-crl");
    let out = init(&home)
        .env("MAOS_CRL_PATH", &explicit)
        .output()
        .expect("failed to execute maos init with MAOS_CRL_PATH");
    assert!(
        out.status.success(),
        "MAOS_CRL_PATH init exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        explicit.is_dir(),
        "MAOS_CRL_PATH={} is created instead",
        explicit.display()
    );

    let _ = std::fs::remove_dir_all(&home);
}

/// 18-0 review DA — an uncreatable CRL directory (`MAOS_CRL_PATH` under a
/// regular file: ENOTDIR for any uid, root included) warns on stderr and
/// `maos init` still succeeds, on a fresh home and on an initialized one.
#[test]
fn init_warns_and_succeeds_when_the_crl_directory_cannot_be_created() {
    let home = isolated_home("crl-warn");
    let blocker = home.join("not-a-dir");
    std::fs::write(&blocker, b"file").expect("write blocker file");
    let crl = blocker.join("crl");
    let warning = format!("maos: could not create CRL directory {}: ", crl.display());
    for run in ["first", "second"] {
        let out = init(&home)
            .env("MAOS_CRL_PATH", &crl)
            .output()
            .expect("failed to execute maos init");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{run} init exit 0: {stderr}");
        assert!(
            stderr.contains(&warning),
            "{run} init warns `{warning}…`: {stderr}"
        );
        assert!(home.join("config.toml").exists(), "{run} init wrote config");
    }

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn shell_hello_spirit_say_hi_and_audit_query() {
    let home = isolated_home("shell");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = init(&home).output();

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
    let _ = init(&home).output();

    // audit query --plain should emit zero ANSI bytes even when empty.
    let out = Command::new(bin)
        .args(["audit", "query", "--plain"])
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos audit query");

    // Empty DB is OK for this smoke; just assert no ANSI escape bytes.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.as_bytes().contains(&0x1b),
        "--plain must emit zero ANSI bytes"
    );

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn shell_plain_no_ansi() {
    let home = isolated_home("shell-ansi");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = init(&home).output();

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
    assert!(
        !stdout.as_bytes().contains(&0x1b),
        "--plain shell must emit zero ANSI bytes, got: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn audit_query_returns_shell_turn_rows() {
    let home = isolated_home("audit-shell");
    let bin = env!("CARGO_BIN_EXE_maos");

    // Init.
    let _ = init(&home).output();

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
    assert!(
        out.status.success(),
        "shell should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

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
    assert!(
        !audit_stdout.trim().is_empty(),
        "audit query should return rows after shell interaction, got: {audit_stdout}"
    );

    let _ = std::fs::remove_dir_all(&home);
}
