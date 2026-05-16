#![forbid(unsafe_code)]

//! Story 1b.5c AC2 — accessibility cascade across all five v0.1
//! subcommands (`install`, `start`, `stop`, `unload`, `run`).
//!
//! Sibling to `audit_no_color_test.rs` (Story 1b.5b — `audit query`
//! cascade) per Decision Register D1: tests for the maosctl surface
//! live in `crates/maos-cli/tests/` because the dispatcher is in
//! `maos-cli` (the dep-direction rule keeps `maos-cli` independent of
//! `maos-kernel-core`).
//!
//! The 15-invocation matrix (5 subcommands × 3 trigger paths
//! [`--plain`, `NO_COLOR=1`, `TERM=dumb`]) asserts that **both** stdout
//! AND stderr contain zero `0x1b` ESC bytes — per the AC text which
//! mandates both streams. The five `#[test]` functions iterate the
//! three triggers inline via a shared helper.
//!
//! ## install dry-run
//!
//! Decision Register D4: `install` is exercised via `MAOS_INSTALL_DRY_RUN=1`
//! which short-circuits the cargo build to a single `eprintln` + exit 0.
//! The real cargo build is covered by `tests/integration/maosctl_smoke.sh`.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

/// Resolve the `maosctl` binary: prefer `CARGO_BIN_EXE_maosctl` (injected
/// by cargo at test build time), then a sibling of the test binary, then
/// PATH. Mirrors `audit_no_color_test::maosctl_path`.
fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maosctl");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maosctl")
}

/// Resolve the `maos-bin` binary: prefer `CARGO_BIN_EXE_maos-bin` if
/// cargo injected it, then sibling-of-test-exe at one level up. Used so
/// the dispatched `start`/`stop`/`unload`/`run` shells out to the right
/// binary in hermetic test runs.
fn maos_bin_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maos-bin") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maos-bin");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maos-bin")
}

/// Run maosctl with a hermetic environment: env_clear + PATH restore +
/// per-test tempfile-backed `MAOS_AUDIT_DB` / `MAOS_JOURNAL_PATH` /
/// `XDG_DATA_HOME`. The `extra_env` slice layers the trigger
/// (`--plain` is passed via `args`, the env triggers via `extra_env`).
fn run_maosctl(extra_env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    let workspace_root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    cmd.current_dir(&workspace_root);
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    // Tell maosctl exactly where to find maos-bin so the dispatched
    // shell-out doesn't depend on PATH for the sibling binary.
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    // Decision D4: install dry-run for unit-test affordance.
    cmd.env("MAOS_INSTALL_DRY_RUN", "1");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.args(args);
    let out = cmd.output().expect("spawn maosctl");
    // Keep tmpdir alive until after the process exits — `tmp` is dropped
    // here automatically.
    drop(tmp);
    out
}

fn assert_no_ansi_both_streams(out: &std::process::Output, scenario: &str) {
    let esc_stdout = out.stdout.iter().filter(|b| **b == 0x1b).count();
    let esc_stderr = out.stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(
        esc_stdout, 0,
        "{scenario}: stdout contains {esc_stdout} ANSI escape byte(s) — NFR-Ops-5 violation"
    );
    assert_eq!(
        esc_stderr, 0,
        "{scenario}: stderr contains {esc_stderr} ANSI escape byte(s) — NFR-Ops-5 violation"
    );
}

/// Drive the three triggers (`--plain` flag, `NO_COLOR=1`, `TERM=dumb`)
/// against the given subcommand-args prefix and assert zero ANSI bytes
/// on both streams for each trigger.
fn cascade(subcmd_args: &[&str], label: &str) {
    // Trigger 1: --plain (CLI flag — passed as global arg before subcmd)
    let mut plain_args: Vec<&str> = vec!["--plain"];
    plain_args.extend_from_slice(subcmd_args);
    let out = run_maosctl(&[], &plain_args);
    let scenario = format!("{label} --plain");
    assert!(
        out.status.success(),
        "{scenario}: expected exit 0 — status={:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi_both_streams(&out, &scenario);

    // Trigger 2: NO_COLOR=1
    let out = run_maosctl(&[("NO_COLOR", "1")], subcmd_args);
    let scenario = format!("{label} NO_COLOR=1");
    assert!(
        out.status.success(),
        "{scenario}: expected exit 0 — status={:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi_both_streams(&out, &scenario);

    // Trigger 3: TERM=dumb
    let out = run_maosctl(&[("TERM", "dumb")], subcmd_args);
    let scenario = format!("{label} TERM=dumb");
    assert!(
        out.status.success(),
        "{scenario}: expected exit 0 — status={:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi_both_streams(&out, &scenario);
}

#[test]
fn install_cascade_emits_zero_ansi_bytes() {
    cascade(&["install", "hello-spirit"], "install");
}

#[test]
fn start_cascade_emits_zero_ansi_bytes() {
    cascade(&["start", "hello-spirit"], "start");
}

#[test]
fn stop_cascade_emits_zero_ansi_bytes() {
    cascade(&["stop", "hello-spirit"], "stop");
}

#[test]
fn unload_cascade_emits_zero_ansi_bytes() {
    cascade(&["unload", "hello-spirit"], "unload");
}

#[test]
fn run_cascade_emits_zero_ansi_bytes() {
    // `run` actually invokes hello-spirit through the maos-bin one-shot
    // path; the existing one-shot emits FR58 JSON on stdout + eprintln
    // tracing on stderr — both ASCII-only paths. No ANSI bytes.
    cascade(&["run", "hello-spirit"], "run");
}
