//! v0.1-α subcommand stubs. Each emits a deterministic
//! "not-yet-implemented" message and exits with code 2.

use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::Subcommand;

pub fn dispatch(cmd: &Subcommand, _color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(_) => stub("install", "Story 1b.5b"),
        Subcommand::Start(_) => stub("start", "Story 5.1"),
        Subcommand::Stop(_) => stub("stop", "Story 5.1"),
        Subcommand::Unload(_) => stub("unload", "Story 5.1"),
        Subcommand::Run(_) => stub("run", "Story 1b.5b"),
        Subcommand::Audit(_) => stub("audit", "Story 1b.5b"),
    }
}

fn stub(name: &str, future_story: &str) -> ExitCode {
    eprintln!(
        "maosctl: {name} not yet implemented at v0.1-α — landing at {future_story}"
    );
    ExitCode::from(2)
}
