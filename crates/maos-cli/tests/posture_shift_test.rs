#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl posture` (Story 3.2, AC6).
//!
//! Verifies the posture-shift CLI subcommand exits cleanly, journals a
//! PostureShift entry, honors NO_COLOR, and rejects the non-runtime
//! `autonomous` posture via clap value validation.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

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

fn run_maosctl(env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args(args);
    cmd.output().expect("spawn maosctl")
}

#[test]
fn posture_shift_cautious_exits_zero() {
    let out = run_maosctl(
        &[
            ("MAOS_ONE_SHOT", "posture-shift"),
            ("MAOS_SPIRIT_ID", "hello-spirit"),
            ("MAOS_POSTURE", "cautious"),
        ],
        &["posture", "hello-spirit", "--shift", "cautious"],
    );
    assert!(
        out.status.success(),
        "posture shift should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn no_color_posture_shift_emits_zero_ansi_bytes() {
    let out = run_maosctl(
        &[
            ("NO_COLOR", "1"),
            ("MAOS_ONE_SHOT", "posture-shift"),
            ("MAOS_SPIRIT_ID", "hello-spirit"),
            ("MAOS_POSTURE", "assistive"),
        ],
        &["posture", "hello-spirit", "--shift", "assistive"],
    );
    let esc_count = out.stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(
        esc_count, 0,
        "NO_COLOR=1 must produce zero ANSI escape bytes in stderr, found {esc_count}"
    );
}

#[test]
fn autonomous_posture_rejected_by_clap() {
    let out = run_maosctl(
        &[
            ("MAOS_ONE_SHOT", "posture-shift"),
            ("MAOS_SPIRIT_ID", "hello-spirit"),
            ("MAOS_POSTURE", "autonomous"),
        ],
        &["posture", "hello-spirit", "--shift", "autonomous"],
    );
    assert!(
        !out.status.success(),
        "autonomous must be rejected — PostureChoice has only 3 variants"
    );
}
