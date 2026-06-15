#![forbid(unsafe_code)]

//! `maos-cli` — maosctl command-line interface (Story 1a.4 scaffold).
//!
//! The library wraps clap parsing + subcommand dispatch + accessibility
//! resolution. The `maosctl` binary at `src/main.rs` is a 3-line shim
//! over `run()`. Subcommand bodies at v0.1-α emit deterministic
//! "not-yet-implemented" diagnostics and exit with code 2.

use std::ffi::OsString;
use std::process::ExitCode;

pub mod accessibility;
pub mod backup;
pub mod cli;
pub mod subcommands;
use clap::Parser;

/// Library-level entry point. Returns a `std::process::ExitCode`
/// for the binary `main.rs` to propagate.
pub fn run(args: Vec<OsString>) -> ExitCode {
    let parsed = match cli::Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            // clap's own error rendering. Note: clap honors NO_COLOR via
            // its own anstyle dep, but we also pass `--plain` through to
            // the color-choice resolver for consistency.
            e.exit();
        }
    };

    let color = accessibility::ColorChoice::resolve(parsed.plain, &accessibility::RealEnv);

    subcommands::dispatch(&parsed.command, color)
}
