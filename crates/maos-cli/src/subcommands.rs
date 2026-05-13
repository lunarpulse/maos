//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). All others remain stubs.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::{Subcommand, AuditQuery};

pub fn dispatch(cmd: &Subcommand, _color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(_) => stub("install", "Story 1b.5b"),
        Subcommand::Start(_) => stub("start", "Story 5.1"),
        Subcommand::Stop(_) => stub("stop", "Story 5.1"),
        Subcommand::Unload(_) => stub("unload", "Story 5.1"),
        Subcommand::Run(_) => stub("run", "Story 1b.5b"),
        Subcommand::Audit(args) => audit_dispatch(&args.query),
    }
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
