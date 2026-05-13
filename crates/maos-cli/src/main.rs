#![forbid(unsafe_code)]

//! `maosctl` binary entrypoint — thin shim over `maos_cli::run`.

use std::process::ExitCode;

fn main() -> ExitCode {
    maos_cli::run(std::env::args_os().collect())
}
