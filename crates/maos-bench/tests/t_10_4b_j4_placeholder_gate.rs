//! Story 10.4b — J4 proven-RED placeholder gate (AC: Mira-Nash bilateral
//! 2-Host live deployment proof).
//!
//! J4 is the §13.1 Mira-Nash Observer colocation latency journey. At Story
//! 10.4b the real in-kernel scalar.tap measurement is **deferred** (see
//! `harness/j4.rs` DEFERRED note + ADR-040): every run takes the canned smoke
//! path, which emits a loud WARNING that the results are NOT real. A budget
//! pass on canned data proves nothing, so the gate is **RED by construction** —
//! it must never read as a green latency gate.
//!
//! This file pins that contract mechanically:
//!   1. `t_10_4b_j4_smoke_mode_emits_warning` — the smoke path genuinely emits
//!      the NOT-real marker to stderr (proves the run produced no real numbers).
//!   2. `t_10_4b_j4_placeholder_gate_red` — a `#[ignore]`'d SKIPPED-with-reason
//!      verdict. It verifies the placeholder is intact and then PANICS, so it
//!      cannot be mistaken for a green pass. The `check-j4-placeholder-red`
//!      discipline job runs it with `--ignored` and asserts it FAILS; a PASS
//!      means the placeholder silently turned green without a real measurement
//!      (the regression this gate exists to prevent).
//!
//! The harness that flips this gate GREEN — a real in-kernel scalar.tap loop
//! that emits NO warning — is Story 10.4c. When it lands, remove the
//! `#[ignore]`, drop the panic, and add the real-measurement assertion; that
//! forced edit is the proven-RED → GREEN cutover.

use std::io::Write;
use std::process::Command;

use maos_bench::harness::j4::{J4Config, run_j4_measurement};
use maos_bench::report::JourneyResult;

/// Marker prefix the producer prints its serialized result after, so the parent
/// can recover the `JourneyResult` from libtest's own stdout noise.
const PRODUCER_MARKER: &str = "__J4_RESULT_JSON__";

/// Inner producer: runs the measurement (emitting the smoke-mode WARNING to
/// stderr) and prints the resulting `JourneyResult` as one JSON line on stdout.
///
/// This is a normal `#[test]`, but it is primarily driven as a subprocess of
/// [`t_10_4b_j4_smoke_mode_emits_warning`] / [`t_10_4b_j4_placeholder_gate_red`]
/// so the parent can observe the process-global stderr sink that libtest
/// otherwise captures and swallows.
#[test]
fn j4_measurement_producer_inner() {
    let result = run_j4_measurement(&J4Config::default()).expect("j4 measurement");
    let json = serde_json::to_string(&result).expect("serialize journey result");
    let mut out = std::io::stdout();
    writeln!(out, "{PRODUCER_MARKER}{json}").expect("write result line");
    out.flush().expect("flush stdout");
}

/// Re-run the J4 measurement in a fresh subprocess (capture disabled) and
/// return `(result, stderr)`. Stderr is a process-global sink that libtest
/// captures by default, so to observe the smoke-mode WARNING we must run the
/// measurement in a child whose stderr the parent owns.
fn run_j4_in_subprocess() -> (JourneyResult, String) {
    let exe = std::env::current_exe().expect("resolve test bin");
    let output = Command::new(exe)
        .args(["j4_measurement_producer_inner", "--exact", "--nocapture"])
        .output()
        .expect("spawn J4 producer subprocess");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "J4 producer subprocess failed; stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = stdout
        .lines()
        .find(|line| line.starts_with(PRODUCER_MARKER))
        .map(|line| &line[PRODUCER_MARKER.len()..])
        .unwrap_or_else(|| {
            panic!("no {PRODUCER_MARKER} line in producer stdout; full stdout:\n{stdout}");
        });
    let result: JourneyResult =
        serde_json::from_str(json).expect("deserialize journey result from producer");
    (result, stderr)
}

/// AC: the smoke-mode WARNING is genuinely emitted to stderr. Without the
/// `kernel_measurement` feature the canned path runs and warns that results are
/// NOT real measurements. Both placeholder paths (feature-off smoke and the
/// feature-on DEFERRED kernel fallback) funnel through the same marker, so this
/// assertion is robust regardless of which path ran.
#[test]
fn t_10_4b_j4_smoke_mode_emits_warning() {
    let (_result, stderr) = run_j4_in_subprocess();
    assert!(
        stderr.contains("WARNING:"),
        "expected a WARNING: banner on stderr; got:\n{stderr}"
    );
    assert!(
        stderr.contains("NOT real measurements"),
        "the marker must state the results are NOT real; got:\n{stderr}"
    );
}

/// AC: the J4 placeholder gate is mechanically RED — NOT green.
///
/// At Story 10.4b the in-kernel scalar.tap measurement is deferred (ADR-040):
/// every run takes the canned/smoke path and emits the NOT-real marker, so a
/// budget pass proves nothing and the gate is RED by construction. This test is
/// `#[ignore]`'d (SKIPPED-with-reason) in normal runs so it cannot be mistaken
/// for a green latency pass. The `check-j4-placeholder-red` discipline job runs
/// it with `--ignored` and asserts it FAILS; a PASS means the placeholder
/// silently turned green without a real measurement (the regression this gate
/// prevents). Story 10.4c lands the real path, removes this `#[ignore]`, and
/// flips the verdict to GREEN; that forced edit is the proven-RED → GREEN cutover.
#[ignore = "J4 proven-RED placeholder (Story 10.4b): in-kernel measurement deferred \
            to Story 10.4c per ADR-040. Do NOT un-ignore until run_j4_measurement \
            emits no WARNING (real path landed). The discipline job runs this with \
            --ignored and expects it to fail (proven-RED)."]
#[test]
fn t_10_4b_j4_placeholder_gate_red() {
    // Proven-RED by construction: at Story 10.4b the measurement always takes
    // the canned/smoke path and emits the NOT-real marker. Verify the placeholder
    // is intact, then fail — the gate is RED, not green.
    let (_result, stderr) = run_j4_in_subprocess();
    assert!(
        stderr.contains("NOT real measurements"),
        "J4 placeholder marker missing — if the real measurement path landed, \
         flip this gate GREEN at Story 10.4c; got stderr:\n{stderr}"
    );
    panic!(
        "J4 proven-RED placeholder: the measurement path is canned/deferred (it \
         emits the NOT-real marker), so the J4 latency gate is RED, not green. \
         Real in-kernel measurement lands at Story 10.4c — flip this gate GREEN there."
    );
}
