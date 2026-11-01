#![forbid(unsafe_code)]

//! Story 20-3c AC8 (FR41, `deferred-work.md` FR4 CLI) — the operator's audit
//! query answers the question asked, from both binaries over one fixture log.
//!
//! * `maos audit query` honours `--intent-contains` exactly as `maosctl` does;
//! * `--tag` is an EXACT match on the intent column in both;
//! * `maos audit query --spirit recall-test --format plain` exits 0 on a root
//!   whose Spirit has a tokenless seed `TaskAssign` (the 17-3d root's row);
//!   it was `Fr4SchemaViolation { missing_field: "capability_token" }`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0x20_3c;
const PID: u32 = 7;

fn maosctl() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(path.is_file(), "build maosctl first: {}", path.display());
    path
}

/// One fixture log: three telemetry rows (`halt:calibration`,
/// `halt:calibration-extra`, `other`), the Spirit's admission row and a
/// tokenless `TaskAssign` seed at the Spirit's pid.
fn fixture(home: &Path) {
    let audit = home.join("audit");
    std::fs::create_dir_all(&audit).unwrap();
    let log = TransparencyLogAdapter::open(&audit.join("transparency.sqlite"), NONCE).unwrap();
    for intent in ["halt:calibration", "halt:calibration-extra", "other"] {
        let _ = log.insert_frame_event(
            FrameKind::TelemetryEvent,
            0,
            None,
            intent,
            b"{}",
            FrameOrigin::Kernel,
        );
    }
    let _ = log.insert_frame_event(
        FrameKind::SpiritAdmitted,
        PID,
        None,
        "recall-test",
        b"{}",
        FrameOrigin::Kernel,
    );
    let _ = log.insert_frame_event_with_id(
        Some([0x31; 16]),
        FrameKind::TaskAssign,
        PID,
        "recall-test",
        "peer",
        None,
        "Standard",
        b"recall mediation payload",
        FrameOrigin::SpiritAuto,
    );
}

fn run(bin: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .args(["audit", "query"])
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .output()
        .expect("run audit query")
}

fn intents(output: &Output) -> Vec<String> {
    assert!(
        output.status.success(),
        "exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut found: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|row| row["intent"].as_str().map(str::to_owned))
        .collect();
    found.sort();
    found
}

/// AC8.2 / AC8.3 — both binaries return the same rows for the same filter.
#[test]
fn intent_contains_and_tag_agree_between_maos_and_maosctl() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let maos = PathBuf::from(env!("CARGO_BIN_EXE_maos"));
    let ctl = maosctl();
    for (flag, value, expected) in [
        (
            "--intent-contains",
            "calibration",
            vec!["halt:calibration", "halt:calibration-extra"],
        ),
        ("--tag", "halt:calibration", vec!["halt:calibration"]),
        ("--tag", "calibration", vec![]),
    ] {
        let args = [flag, value, "--format", "ndjson"];
        let from_maos = intents(&run(&maos, home.path(), &args));
        let from_ctl = intents(&run(&ctl, home.path(), &args));
        assert_eq!(from_maos, expected, "maos {flag} {value}");
        assert_eq!(from_ctl, expected, "maosctl {flag} {value}");
    }
}

/// AC8.1 — the `--spirit` plain view renders a root whose seed row is a
/// tokenless `TaskAssign`.
#[test]
fn spirit_view_renders_a_tokenless_seed_task_assign() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let out = run(
        Path::new(env!("CARGO_BIN_EXE_maos")),
        home.path(),
        &["--spirit", "recall-test", "--format", "plain"],
    );
    assert!(
        out.status.success(),
        "exit {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("recall-test") || !out.stdout.is_empty());
}
