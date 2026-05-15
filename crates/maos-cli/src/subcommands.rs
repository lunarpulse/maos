//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::{Subcommand, AuditQuery, RunArgs, InstallArgs};

pub fn dispatch(cmd: &Subcommand, color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(args) => install(args, color),
        Subcommand::Start(_) => stub("start", "Story 5.1"),
        Subcommand::Stop(_) => stub("stop", "Story 5.1"),
        Subcommand::Unload(_) => stub("unload", "Story 5.1"),
        Subcommand::Run(args) => run(args, color),
        Subcommand::Audit(args) => audit_dispatch(&args.query),
    }
}

fn run(args: &RunArgs, _color: ColorChoice) -> ExitCode {
    let spirit = match &args.spirit {
        Some(s) if s == "hello-spirit" => s,
        Some(s) => {
            eprintln!("maosctl: unknown spirit '{s}' — only 'hello-spirit' is available at v0.1-α");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("maosctl: run requires a spirit argument, e.g. 'maosctl run hello-spirit'");
            return ExitCode::from(2);
        }
    };

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", spirit);

    // Honor the accessibility cascade: pass NO_COLOR through if set
    if std::env::var_os("NO_COLOR").is_some() {
        cmd.env("NO_COLOR", "1");
    }
    // --plain flag also disables color
    if _color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!("maosctl: failed to execute maos-bin at '{}': {e}", bin.display());
            ExitCode::from(2)
        }
    }
}

fn install(args: &InstallArgs, _color: ColorChoice) -> ExitCode {
    // At v0.1-α, install is a compilation check: build the hello-Spirit crate.
    let spirit_crate = match &args.source {
        Some(s) if s == "hello-spirit" => "maos-spirit-hello",
        Some(s) => {
            eprintln!("maosctl: unknown spirit '{s}' — only 'hello-spirit' is available at v0.1-α");
            return ExitCode::from(2);
        }
        None => {
            // Default: install hello-spirit (the only reference Spirit at v0.1)
            "maos-spirit-hello"
        }
    };

    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "-p", spirit_crate, "--locked"]);

    match cmd.status() {
        Ok(s) if s.success() => {
            eprintln!("maosctl: {spirit_crate} compiled successfully");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("maosctl: cargo build {spirit_crate} failed");
            ExitCode::from(s.code().unwrap_or(2) as u8)
        }
        Err(e) => {
            eprintln!("maosctl: failed to execute cargo build: {e}");
            ExitCode::from(2)
        }
    }
}

/// Resolve `maos-bin` binary path.
///
/// Priority: `MAOS_BIN_PATH` env var → sibling of current exe → PATH.
fn maos_bin_path() -> PathBuf {
    // 1. Explicit override
    if let Ok(p) = std::env::var("MAOS_BIN_PATH") {
        return PathBuf::from(p);
    }
    // 2. Sibling of current exe (same target directory)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("maos-bin");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    // 3. Fallback to PATH
    PathBuf::from("maos-bin")
}

fn audit_dispatch(query_kind: &Option<AuditQuery>) -> ExitCode {
    match query_kind {
        None | Some(AuditQuery::Query) => audit_query(),
    }
}

fn audit_query() -> ExitCode {
    let db_path = default_transparency_log_path();
    let filter = maos_audit::AuditFilter::default();
    let entries = match maos_audit::query(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit query — no Transparency Log found at {}. \
                 Run `maosctl run hello-spirit` first to seed the log.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit query — error: {e}");
            return ExitCode::from(2);
        }
    };
    let stdout = std::io::stdout();
    let lock = stdout.lock();
    if let Err(e) = maos_audit::to_ndjson(entries, lock) {
        eprintln!("maosctl: audit query — output error: {e}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// Resolve the default Transparency Log SQLite path.
/// XDG-compliant: `$XDG_DATA_HOME/maos/audit/transparency.sqlite`
/// Override via `MAOS_AUDIT_DB` environment variable.
fn default_transparency_log_path() -> PathBuf {
    if let Ok(p) = std::env::var("MAOS_AUDIT_DB") {
        return PathBuf::from(p);
    }
    // Hand-rolled XDG resolution (avoids dirs-next dep blast).
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("audit").join("transparency.sqlite")
}

fn stub(name: &str, future_story: &str) -> ExitCode {
    eprintln!(
        "maosctl: {name} not yet implemented at v0.1-α — landing at {future_story}"
    );
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Subcommand, RunArgs, InstallArgs};
    use crate::accessibility::ColorChoice;
    use clap::Parser;

    #[test]
    fn dispatch_run_hello_spirit() {
        // Verify the dispatch maps 'run hello-spirit' to the run handler
        let cli = Cli::try_parse_from(["maosctl", "run", "hello-spirit"]).unwrap();
        match &cli.command {
            Subcommand::Run(args) => {
                assert_eq!(args.spirit.as_deref(), Some("hello-spirit"));
            }
            _ => panic!("expected Run subcommand"),
        }
    }

    #[test]
    fn dispatch_install() {
        let cli = Cli::try_parse_from(["maosctl", "install"]).unwrap();
        match &cli.command {
            Subcommand::Install(_args) => {}
            _ => panic!("expected Install subcommand"),
        }
    }

    #[test]
    fn dispatch_unknown_spirit_run() {
        // Verify the dispatch handles unknown spirit names gracefully
        let color = ColorChoice::Auto;
        let args = RunArgs {
            spirit: Some("nonexistent-spirit".into()),
            args: vec![],
        };
        let result = run(&args, color);
        // Non-zero exit code expected
        assert_ne!(result, ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_unknown_spirit_install() {
        let color = ColorChoice::Auto;
        let args = InstallArgs {
            source: Some("nonexistent-spirit".into()),
        };
        let result = install(&args, color);
        // Non-zero exit code expected
        assert_ne!(result, ExitCode::SUCCESS);
    }
}

