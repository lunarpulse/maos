//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::{AuditFormat, AuditQuery, InstallArgs, RunArgs, Subcommand};

pub fn dispatch(cmd: &Subcommand, color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(args) => install(args, color),
        Subcommand::Start(_) => stub("start", "Story 5.1"),
        Subcommand::Stop(_) => stub("stop", "Story 5.1"),
        Subcommand::Unload(_) => stub("unload", "Story 5.1"),
        Subcommand::Run(args) => run(args, color),
        Subcommand::Audit(args) => audit_dispatch(&args.query, color),
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

fn audit_dispatch(query_kind: &Option<AuditQuery>, color: ColorChoice) -> ExitCode {
    match query_kind {
        // Bare `maosctl audit` — defaults to ndjson over all entries.
        None => audit_query(None, AuditFormat::Ndjson, color),
        Some(AuditQuery::Query { spirit, format }) => {
            audit_query(spirit.as_deref(), *format, color)
        }
    }
}

/// Resolve a Spirit name to its `spirit_pid` for filtering. At v0.1-β only
/// `hello-spirit` is resolvable (maps to `0` per Story 1b.5a's one-shot path).
/// Other names exit non-zero with a clear diagnostic — full Spirit registry
/// lookup is Epic 5.
fn resolve_spirit_pid(name: &str) -> Result<u32, String> {
    match name {
        "hello-spirit" => Ok(0),
        other => Err(format!(
            "unknown spirit, only 'hello-spirit' is available at v0.1-β (got '{other}')"
        )),
    }
}

fn audit_query(spirit: Option<&str>, format: AuditFormat, _color: ColorChoice) -> ExitCode {
    let db_path = default_transparency_log_path();

    let mut filter = maos_audit::AuditFilter::default();
    if let Some(name) = spirit {
        match resolve_spirit_pid(name) {
            Ok(pid) => filter.spirit_pid = Some(pid),
            Err(diag) => {
                eprintln!("maosctl: audit query — {diag}");
                return ExitCode::from(2);
            }
        }
    }

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
    // FR4 projection engages only when the operator scopes the query to a
    // Spirit (`--spirit <name>`). Bare `maosctl audit query` keeps the
    // legacy raw `AuditEntry` NDJSON surface (Story 1b.1 / `to_ndjson`) so
    // existing tooling (e.g. `tests/integration/audit_spine_smoke.sh`)
    // observing `frame_id`/`intent` continues to work. AC1 mandates the
    // FR4 six-key schema for the `--spirit` form specifically; the bare
    // form remains Story 9.1's territory.
    //
    // `_color` is currently advisory — both formats already emit zero ANSI
    // bytes unconditionally. Wired through for future colored ndjson keys
    // (Story 9.1) and to document the contract.
    let fr4_mode = spirit.is_some();
    let write_result = match (fr4_mode, format) {
        (true, AuditFormat::Ndjson) => maos_audit::to_fr4_ndjson(entries, lock),
        (true, AuditFormat::Plain) => maos_audit::to_fr4_plain(entries, lock),
        (false, AuditFormat::Ndjson) => maos_audit::to_ndjson(entries, lock),
        (false, AuditFormat::Plain) => maos_audit::to_plain(entries, lock),
    };
    match write_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(maos_audit::AuditError::Fr4SchemaViolation { line, missing_field }) => {
            eprintln!(
                "maosctl: audit query — FR4 schema violation at line {line}: missing field '{missing_field}'"
            );
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("maosctl: audit query — output error: {e}");
            ExitCode::from(2)
        }
    }
}

/// Resolve the default Transparency Log SQLite path.
///
/// Delegates to [`maos_audit::default_transparency_log_path`] — the single
/// source of truth shared by `maos-bin` (write side) and `maos-cli` (read
/// side) to prevent path-drift data loss.
fn default_transparency_log_path() -> PathBuf {
    maos_audit::default_transparency_log_path()
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

    // ── FR4 audit query dispatch parsing tests (Story 1b.5b) ─────────

    #[test]
    fn audit_query_accepts_spirit_and_format_flags() {
        use crate::cli::{AuditQuery, AuditFormat};
        let cli = Cli::try_parse_from([
            "maosctl", "audit", "query", "--spirit", "hello-spirit", "--format", "ndjson",
        ])
        .expect("audit query --spirit / --format must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit, format }) => {
                    assert_eq!(spirit.as_deref(), Some("hello-spirit"));
                    assert_eq!(*format, AuditFormat::Ndjson);
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_accepts_plain_format() {
        use crate::cli::{AuditQuery, AuditFormat};
        let cli = Cli::try_parse_from([
            "maosctl", "audit", "query", "--spirit", "hello-spirit", "--format", "plain",
        ])
        .expect("audit query --format plain must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit: _, format }) => {
                    assert_eq!(*format, AuditFormat::Plain);
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_defaults_format_to_ndjson() {
        use crate::cli::{AuditQuery, AuditFormat};
        let cli = Cli::try_parse_from(["maosctl", "audit", "query"])
            .expect("audit query with no flags must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit, format }) => {
                    assert!(spirit.is_none(), "no --spirit means None");
                    assert_eq!(*format, AuditFormat::Ndjson, "default format is ndjson");
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn resolve_spirit_pid_maps_hello_spirit_to_zero() {
        assert_eq!(resolve_spirit_pid("hello-spirit").unwrap(), 0);
    }

    #[test]
    fn resolve_spirit_pid_rejects_other_names_with_clear_diagnostic() {
        let err = resolve_spirit_pid("orchestrator").unwrap_err();
        assert!(
            err.contains("only 'hello-spirit' is available at v0.1-β"),
            "diagnostic must name the v0.1-β scope: got {err}"
        );
    }
}

