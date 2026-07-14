//! `equiv-native-twin` — native subprocess twin of the WASM fixture Spirits
//! for the Story 11.1b cross-form equivalence gate.
//!
//! This binary is the NATIVE form. It speaks the EXACT same ADR-032 wire
//! protocol (Content-Length + canonical CBOR) as the `maos-wasm-runner` driving
//! a WASM component, and it applies the EXACT same field transforms via
//! [`equiv_fixture_logic`] (the shared source the WASM guests also compile in).
//! So the only difference between the two forms is HOW they execute — one runs
//! as a wasmtime component, the other as a plain native process — never WHAT
//! they compute.
//!
//! # Usage
//!
//! ```text
//! equiv-native-twin [--mode identity|divergent|cosmetic]
//! ```
//!
//! Default mode is `identity`. The harness drives the twin by writing ADR-032
//! frames to its stdin and reading the emitted frames from its stdout.
//!
//! # Modes (mirror the three WASM fixtures 1:1)
//!
//! - `identity`    — echo (frame in == frame out).        PASS case.
//! - `divergent`   — `logical_clock += 1`.                FAIL case.
//! - `cosmetic`    — 5 ms latency, frame otherwise unchanged. PASS case.

use std::io::{self, BufReader, BufWriter, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use equiv_fixture_logic::{should_delay, transform_logical_clock, FixtureMode};
use maos_domain::frame::IacFrame;
use maos_wasm_host::codec::{decode_cbor, encode_cbor, read_frame, write_frame};

/// Cosmetic (non-invariant) wall-clock delay applied under `--mode cosmetic`.
/// Large enough to be observable above scheduling jitter, small enough to keep
/// the gate fast.
const COSMETIC_DELAY: Duration = Duration::from_millis(5);

fn main() -> ExitCode {
    let mode = match parse_mode() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("equiv-native-twin: {e}");
            eprintln!("usage: equiv-native-twin [--mode identity|divergent|cosmetic]");
            return ExitCode::from(2);
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        let frame_bytes = match read_frame(&mut reader) {
            Ok(Some(b)) => b,
            Ok(None) => {
                // Clean EOF — maps to ADR-032 `Halt::Voluntary`. Flush any
                // buffered output before a clean exit.
                let _ = writer.flush();
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("equiv-native-twin: stdin read error: {e}");
                let _ = writer.flush();
                return ExitCode::from(1);
            }
        };

        let mut frame: IacFrame = match decode_cbor(&frame_bytes) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("equiv-native-twin: inbound frame decode error: {e}");
                let _ = writer.flush();
                return ExitCode::from(1);
            }
        };

        // Apply the shared logical transform — the SAME code path the WASM
        // guests compile in. `DivergentLogicalClock` bumps `logical_clock` by
        // exactly 1; every other mode leaves it untouched.
        frame.logical_clock = transform_logical_clock(frame.logical_clock, &mode);

        // Cosmetic latency AFTER the (no-op under this mode) transform: it must
        // not perturb any invariant field. The gate classifies this as a PASS
        // precisely because only the wall-clock differs from `identity`.
        if should_delay(&mode) {
            thread::sleep(COSMETIC_DELAY);
        }

        let out_bytes = match encode_cbor(&frame) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("equiv-native-twin: outbound frame encode error: {e}");
                let _ = writer.flush();
                return ExitCode::from(1);
            }
        };

        if let Err(e) = write_frame(&mut writer, &out_bytes) {
            eprintln!("equiv-native-twin: stdout write error: {e}");
            return ExitCode::from(1);
        }
        // Flush per frame so the harness can read the response promptly; the
        // ADR-032 peer is block-oriented, so an unflushed buffer would stall.
        if let Err(e) = writer.flush() {
            eprintln!("equiv-native-twin: stdout flush error: {e}");
            return ExitCode::from(1);
        }
    }
}

/// Parse `--mode <identity|divergent|cosmetic>` from the process args.
///
/// Unknown flags or values are hard errors (exit 2) so a misconfigured harness
/// fails loudly rather than silently falling back to `identity`.
fn parse_mode() -> Result<FixtureMode, String> {
    let mut mode = FixtureMode::Identity;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--mode" {
            let value = args
                .next()
                .ok_or_else(|| "--mode requires a value (identity|divergent|cosmetic)".to_string())?;
            mode = match value.as_str() {
                "identity" => FixtureMode::Identity,
                "divergent" => FixtureMode::DivergentLogicalClock,
                "cosmetic" => FixtureMode::CosmeticDelay,
                other => return Err(format!("unknown --mode '{other}' (expected identity|divergent|cosmetic)")),
            };
        } else if arg == "-h" || arg == "--help" {
            return Err("--help: prints usage on error exit".to_string());
        } else {
            return Err(format!("unknown argument '{arg}'"));
        }
    }
    Ok(mode)
}
