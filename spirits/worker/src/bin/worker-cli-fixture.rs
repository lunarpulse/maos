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

    // Non-probe: the FIRST line acknowledges the routed task. When the bridge
    // routes a task (a trailing non-flag argv after the `--maos-worker` prefix),
    // echo it — this is what makes task routing PROVABLE hermetically (the task
    // text lands in the Transparency Log). With no task arg the line falls back
    // to the canned acknowledgement, so the SHA-pinned 3-line shape and the
    // `CANNED_OUTPUT_LINES` constant are preserved unchanged. The terminal line
    // stays `worker: task complete` (the completion oracle's marker).
    let routed_task = args.iter().skip(1).find(|a| !a.starts_with("--"));
    match routed_task {
        Some(task) => println!("worker: received task assignment: {task}"),
        None => println!("{}", worker::CANNED_OUTPUT_LINES[0]),
    }
    println!("{}", worker::CANNED_OUTPUT_LINES[1]);
    println!("{}", worker::CANNED_OUTPUT_LINES[2]);
}
