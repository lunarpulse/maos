#![forbid(unsafe_code)]

//! Shared utilities for conditional ship-gate modules (Stories 10.1b, 10.2).
//! Extracted during 10.2 review-patch deferred-item #32/#33 to DRY up date
//! validation and workflow-command emission across all gate modules.

use chrono::NaiveDate;

/// Validate that date strings are non-empty, parseable as ISO-8601 (YYYY-MM-DD),
/// and that `start <= end` (chronological ordering).
///
/// #32 (was deferred): the prior copy only checked `contains('-') && len >= 10`,
/// accepting impossible dates like `'2026-99-99'` and ignoring start<=end ordering.
/// Now uses `chrono::NaiveDate::parse_from_str` for real ISO-8601 validation.
pub fn validate_dates(start_label: &str, start: &str, end_label: &str, end: &str) -> Result<(), String> {
    let parse = |label: &str, s: &str| -> Result<NaiveDate, String> {
        if s.is_empty() {
            return Err(format!("{label} is empty"));
        }
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| format!("{label} is not a valid ISO-8601 date (YYYY-MM-DD): {s}"))
    };
    let start_date = parse(start_label, start)?;
    let end_date = parse(end_label, end)?;
    if end_date < start_date {
        return Err(format!(
            "{end_label} ({end}) is before {start_label} ({start}) — dates must be ordered"
        ));
    }
    Ok(())
}

/// #33: in JSON mode, commands go to stderr (stdout stays clean for JSON parsing);
/// the structured warning/error is also carried in the JSON payload fields so
/// programmatic consumers assert on the JSON, not stderr. In non-JSON mode
/// (production CI), commands go to stdout where Actions parses them.
pub fn emit_command(json: bool, level: &str, msg: &str) {
    if json {
        // #33: in JSON mode, workflow commands go to stderr so stdout stays clean
        // for JSON parsing. The structured warning/error is ALSO in the JSON payload
        // (callers add `advisory: true` / `failures: [...]` fields), so programmatic
        // consumers don't need to parse stderr. Actions only parses stdout commands
        // in non-JSON mode (production CI), which uses the else branch below.
        eprintln!("::{level}::{msg}");
    } else {
        // Production (non-JSON): stdout, where GitHub Actions parses workflow commands.
        println!("::{level}::{msg}");
    }
}
