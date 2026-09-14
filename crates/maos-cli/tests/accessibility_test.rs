#![forbid(unsafe_code)]

//! Story 1b.5c AC2 — accessibility cascade across the v0.1 subcommands
//! (`install`, `start`, `stop`, `unload`, `run`).
//!
//! Sibling to `audit_no_color_test.rs` (Story 1b.5b — `audit query`
//! cascade) per Decision Register D1: tests for the maosctl surface
//! live in `crates/maos-cli/tests/` because the dispatcher is in
//! `maos-cli` (the dep-direction rule keeps `maos-cli` independent of
//! `maos-kernel-core`).
//!
//! The trigger matrix (3 paths [`--plain`, `NO_COLOR=1`, `TERM=dumb`])
//! asserts that **both** stdout AND stderr contain zero `0x1b` ESC bytes
//! — per the AC text which mandates both streams.
//!
//! ## Story 16-1 changes
//!
//! - `start`/`unload`/`run` keep the accessibility-smoke short-circuit and
//!   their zero-ANSI cascades unchanged.
//! - `stop` no longer HAS a smoke path: D-16-1-I refuses it CLIENT-SIDE
//!   BEFORE the smoke short-circuit, so the old `stop smoke ok` cascade
//!   (exit 0) became the refusal — exit **2**, zero ANSI, and the refusal
//!   must fire even with `MAOS_ACCESSIBILITY_SMOKE` set (that ordering IS
//!   the decision).
//! - Door ERROR output is ANSI-free too: a typed 409 driven through the
//!   fixture door (`support/fixture_door.rs` — hand-rolled because
//!   `maos-cli` must not gain a `maos-control` edge) with NO colour
//!   trigger engaged must still emit zero escape bytes on both streams.
//!
//! ## accessibility smoke
//!
//! Decision Register D4: `install`, `start`, `unload` and `run`
//! short-circuit when `MAOS_ACCESSIBILITY_SMOKE=1` is set, emitting a
//! single ASCII-only `eprintln` and exiting 0. `stop` is the exception —
//! see above.

#[path = "support/fixture_door.rs"]
mod fixture_door;

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

/// Resolve the `maos-bin` binary: prefer `CARGO_BIN_EXE_maos` if
/// cargo injected it, then sibling-of-test-exe at one level up. Used so
/// the dispatched `start`/`unload`/`run` shells out to the right
/// binary in hermetic test runs.
fn maos_bin_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maos") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maos");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maos")
}

/// Run maosctl with a hermetic environment: env_clear + PATH restore +
/// per-test tempfile-backed `MAOS_AUDIT_DB` / `MAOS_JOURNAL_PATH` /
/// `XDG_DATA_HOME`. The `extra_env` slice layers the trigger
/// (`--plain` is passed via `args`, the env triggers via `extra_env`).
///
/// The smoke short-circuit keeps the cascade asserting CLI-level color
/// handling without paying the cost (or pipe-deadlock risk) of spawning
/// the full composition root. D-16-1-Q: `HOME` is isolated too, so no
/// developer `control.json` can redirect these runs at a live door.
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
    cmd.env("HOME", tmp.path().join("home"));
    let workspace_root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    cmd.current_dir(&workspace_root);
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    // Tell maosctl exactly where to find maos-bin so the dispatched
    // shell-out doesn't depend on PATH for the sibling binary.
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    // Short-circuit `install`, `run`, `start`, and `unload` to
    // deterministic, colorless diagnostics for the accessibility cascade.
    // `stop` refuses BEFORE this short-circuit (D-16-1-I) — asserted below.
    cmd.env("MAOS_ACCESSIBILITY_SMOKE", "1");
    // Decision D4: `install` also checks its legacy dry-run flag.
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
fn unload_cascade_emits_zero_ansi_bytes() {
    cascade(&["unload", "hello-spirit"], "unload");
}

#[test]
fn run_cascade_emits_zero_ansi_bytes() {
    cascade(&["run", "hello-spirit"], "run");
}

/// D-16-1-I: `stop` is refused CLIENT-SIDE, exit 2, BEFORE the smoke
/// short-circuit — so the refusal must fire on every trigger EVEN WITH
/// `MAOS_ACCESSIBILITY_SMOKE` set (that precedence is the decision), and
/// the refusal text must be ANSI-free on both streams.
#[test]
fn stop_cascade_refuses_with_exit_two_and_zero_ansi() {
    for (trigger_env, trigger_args, label) in [
        (vec![], vec!["--plain"], "stop --plain"),
        (vec![("NO_COLOR", "1")], vec![], "stop NO_COLOR=1"),
        (vec![("TERM", "dumb")], vec![], "stop TERM=dumb"),
    ] {
        let mut args: Vec<&str> = trigger_args;
        args.extend_from_slice(&["stop", "hello-spirit"]);
        let env_refs: Vec<(&str, &str)> = trigger_env;
        let out = run_maosctl(&env_refs, &args);
        let scenario = label;
        assert_eq!(
            out.status.code(),
            Some(2),
            "{scenario}: the stop refusal must exit 2 — status={:?}\nstderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("no kernel transition is named stop"),
            "{scenario}: stderr must carry the D-16-1-I refusal — got: {stderr}"
        );
        assert_no_ansi_both_streams(&out, scenario);
    }
}

/// Door ERROR output is ANSI-free: a typed 409 through the fixture door,
/// with `MAOS_ACCESSIBILITY_SMOKE` unset (the fixture's env is
/// `env_clear`ed — no colour trigger engaged at all), still produces zero
/// escape bytes on both streams.
#[test]
fn door_error_output_is_ansi_free() {
    let door =
        fixture_door::FixtureDoor::spawn_replying(409, r#"{"error":"invalid_state_transition"}"#);
    // `run` env_clears and sets HOME/MAOS_HOME/XDG to the fixture's
    // scratch home: NO colour trigger, NO smoke short-circuit — the raw
    // production mapping is what must be clean.
    let out = door.run(&["pause", "hello-spirit"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a typed 409 must exit 1 — stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid_state_transition"),
        "the typed error must be named — got: {stderr}"
    );
    assert_no_ansi_both_streams(&out, "door 409");
    // Belt and braces: the escape SEQUENCE, not just the lone ESC byte.
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("\x1b["),
        "stdout must not contain an ANSI escape sequence"
    );
    assert!(
        !stderr.contains("\x1b["),
        "stderr must not contain an ANSI escape sequence"
    );
}
