#![forbid(unsafe_code)]

//! `worker-cli-fixture` — the real in-crate fixture-CLI the Worker
//! CliWrapperSpirit wraps (Story 8.4, Decision B).
//!
//! Behavior:
//! - With `--maos-bridge-probe` anywhere in argv: print the JSON output-shape
//!   envelope as the FIRST stdout line and exit 0. This is what the kernel's
//!   `probe_and_verify_shape` reads to assert the declared `output_shape_version`.
//! - Otherwise: echo the deterministic canned output (one line each) and exit 0.
//!   This is the fixture-replayed "work product" captured to the Transparency
//!   Log as `FrameKind::CliSubprocessOutput=21` rows.
//!
//! The reported shape version defaults to [`worker::OUTPUT_SHAPE_VERSION`] but
//! can be overridden via `MAOS_FIXTURE_SHAPE_VERSION` so tests can drive a
//! shape MISMATCH against a stable binary (no write-then-exec race).
//!
//! Hermetic by construction: no network, no filesystem, deterministic stdout.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let version = std::env::var("MAOS_FIXTURE_SHAPE_VERSION")
        .unwrap_or_else(|_| worker::OUTPUT_SHAPE_VERSION.to_string());

    if args.iter().any(|a| a == worker::PROBE_FLAG) {
        // First stdout line MUST be the probe envelope.
        println!("{}", worker::probe_envelope(&version));
        return;
    }

    for line in worker::CANNED_OUTPUT_LINES {
        println!("{line}");
    }
}
