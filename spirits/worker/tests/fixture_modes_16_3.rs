//! Story 16-3 (ruling D-16-3-L) — the `--maos-fixture-mode` vectors for the
//! in-crate fixture-CLI binary:
//!
//! - (a) `--maos-bridge-probe` combined with a mode flag still emits the probe
//!   envelope as the FIRST stdout line and exits 0 without hanging (mode
//!   parsing stays AFTER the probe check, so admission's 2s probe is answered);
//! - (b) an unknown mode exits 2;
//! - (c) the default (no mode flag) invocation is byte-identical to the
//!   SHA-pinned canned output (`fixtures/canned-cli-output.json`, pinned by
//!   `fixtures_pin.rs`).
//!
//! Each vector reds if its behaviour regresses: (a) runs under a wall-clock
//! deadline — a mode parsed before the probe would hang and be killed red;
//! (b) pins the exit code; (c) pins the exact stdout bytes against the pinned
//! fixture.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The path to the real in-crate fixture-CLI binary (resolved by Cargo).
const FIXTURE_CLI: &str = env!("CARGO_BIN_EXE_worker-cli-fixture");

/// Generous wall-clock bound for an invocation that MUST exit promptly; a
/// regression that hangs is killed at the deadline and reds (never wedges the
/// suite).
const MUST_EXIT_DEADLINE: Duration = Duration::from_secs(10);

/// Runs the fixture with `extra` argv under [`MUST_EXIT_DEADLINE`], returning
/// `(exit code, stdout, stderr)`. The fixture writes at most one line (or the
/// 3 canned lines) before sleeping, so draining the pipes after exit cannot
/// deadlock on a full pipe.
fn run_bounded(extra: &[&str]) -> (Option<i32>, Vec<u8>, Vec<u8>) {
    let mut child = Command::new(FIXTURE_CLI)
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn worker-cli-fixture");
    let deadline = Instant::now() + MUST_EXIT_DEADLINE;
    loop {
        match child.try_wait().expect("try_wait fixture CLI") {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                child
                    .stdout
                    .take()
                    .expect("piped stdout")
                    .read_to_end(&mut stdout)
                    .expect("drain stdout");
                child
                    .stderr
                    .take()
                    .expect("piped stderr")
                    .read_to_end(&mut stderr)
                    .expect("drain stderr");
                return (status.code(), stdout, stderr);
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "worker-cli-fixture {extra:?} exceeded the deadline — mode parsing \
                     must stay AFTER the `--maos-bridge-probe` check (D-16-3-L)"
                );
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

#[test]
fn probe_still_answers_with_a_mode_flag_and_exits_without_hanging() {
    let (code, stdout, _stderr) = run_bounded(&[
        "--maos-worker",
        "--maos-bridge-probe",
        "--maos-fixture-mode=hang",
    ]);
    assert_eq!(
        code,
        Some(0),
        "probe + mode flag must exit 0 promptly, not hang"
    );
    let envelope = worker::probe_envelope(worker::OUTPUT_SHAPE_VERSION);
    let first_line = stdout.split(|b| *b == b'\n').next().unwrap_or(&[]);
    assert_eq!(
        first_line,
        envelope.as_bytes(),
        "FIRST stdout line must remain the probe envelope bytes even when a \
         fixture mode flag is present"
    );
}

#[test]
fn unknown_fixture_mode_exits_2() {
    let (code, stdout, _stderr) = run_bounded(&["--maos-fixture-mode=bogus-mode"]);
    assert_eq!(code, Some(2), "an unknown fixture mode must exit 2");
    assert!(
        stdout.is_empty(),
        "an unknown mode must not fall through to the canned output"
    );
}

#[test]
fn default_invocation_stdout_is_byte_identical_to_the_pinned_fixture() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/canned-cli-output.json"
    ))
    .expect("read pinned fixture");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("fixture is JSON");
    let lines: Vec<&str> = v["canned_output_lines"]
        .as_array()
        .expect("canned_output_lines array")
        .iter()
        .map(|x| x.as_str().expect("string line"))
        .collect();

    let (code, stdout, _stderr) = run_bounded(&["--maos-worker"]);
    assert_eq!(code, Some(0));

    let mut expected = lines.join("\n");
    expected.push('\n');
    assert_eq!(
        stdout,
        expected.as_bytes(),
        "default invocation (no mode flag) must stay byte-identical to the \
         SHA-pinned canned output"
    );
}
