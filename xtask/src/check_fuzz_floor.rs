#![forbid(unsafe_code)]

//! Story 10.3 NFR-Sec-5 / NFR-Sec-6 — fuzz CPU-hour floor gate (release-time).
//!
//! Enforces the accretive fuzz-coverage floor against `fuzz-ledger.json`:
//!   - per-target floor: >= 72 CPU-hours (259 200 s) per target
//!   - aggregate floor:  >= 1000 CPU-hours (3 600 000 s) across all targets
//! both measured over the **trailing 90-day window**.
//!
//! ## Bootstrap promotion (closes the F3 logical gap)
//!
//! The floor is logically unsatisfiable for the first 90 days after wiring
//! (the ledger is empty/sparse). The gate is therefore **advisory**
//! (warn-only, exit 0) until the ledger spans >= 90 days of history
//! (`now - earliest_record >= 90 d`), then auto-promotes to **hard-fail** if
//! the floor is unmet. This replaces the jq-only manual check in
//! `docs/runbooks/fuzz-cadence.md` (which fail-opened on string `cpu_seconds`
//! and ignored the 90-day window — code-review findings F2/E2/E3/E4).
//!
//! ## Type safety
//!
//! A record whose `cpu_seconds` is not a JSON number, or whose `timestamp` is
//! not parseable RFC 3339, is a hard FAILURE — never silently coerced. This
//! guards against the jq `add`-on-strings fail-open where a string-typed
//! `cpu_seconds` concatenates and compares `>=` as `string > number`.

use std::path::Path;

use serde::Deserialize;

use crate::gate_common::emit_command;

const FUZZ_LEDGER: &str = "fuzz-ledger.json";
/// >= 72 CPU-hours per target (NFR-Sec-5/6 pre-GA floor).
const PER_TARGET_FLOOR_S: i64 = 259_200;
/// >= 1000 CPU-hours aggregate across all targets (NFR-Sec-5/6 pre-GA floor).
const AGGREGATE_FLOOR_S: i64 = 3_600_000;
const WINDOW_DAYS: i64 = 90;
/// Targets the floor applies to. A record for an unknown target is rejected.
const REQUIRED_TARGETS: &[&str] = &["manifest_parser", "frame_deser"];

#[derive(Debug, Default)]
pub struct Report {
    pub passed: bool,
    /// `true` when the ledger has < 90 days of history — the floor is still
    /// accumulating and not yet enforceable. The gate passes (warn-only).
    pub advisory: bool,
    pub failures: Vec<String>,
    pub summary: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Ledger {
    #[serde(default)]
    records: Vec<Record>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Record {
    target: serde_json::Value,
    cpu_seconds: serde_json::Value,
    timestamp: serde_json::Value,
}

/// Run the floor check against `workspace_root` at instant `now`. `now` is a
/// parameter so tests can inject a fixed clock.
pub fn check_fuzz_floor(workspace_root: &Path, now: chrono::DateTime<chrono::Utc>) -> Report {
    let mut failures = Vec::new();
    let mut summary = Vec::new();

    let ledger_path = workspace_root.join(FUZZ_LEDGER);
    let body = match std::fs::read_to_string(&ledger_path) {
        Ok(s) => s,
        Err(e) => {
            // No ledger yet → the nightly job has not produced evidence.
            // Advisory: the floor is accumulating, not enforceable.
            return Report {
                passed: true,
                advisory: true,
                failures: Vec::new(),
                summary: vec![format!(
                    "{FUZZ_LEDGER} not found — floor accumulating ({e})"
                )],
            };
        }
    };
    let ledger: Ledger = match serde_json::from_str(&body) {
        Ok(l) => l,
        Err(e) => {
            return Report {
                passed: false,
                advisory: false,
                failures: vec![format!("{FUZZ_LEDGER} is not valid JSON: {e}")],
                summary: Vec::new(),
            };
        }
    };

    let cutoff = now - chrono::Duration::days(WINDOW_DAYS);
    let mut per_target: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut earliest: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut aggregate_in_window: i64 = 0;

    for (i, rec) in ledger.records.iter().enumerate() {
        let target = match &rec.target {
            serde_json::Value::String(s) => s.clone(),
            v => {
                failures.push(format!("record {i}: 'target' is not a string ({v})"));
                continue;
            }
        };
        if !REQUIRED_TARGETS.contains(&target.as_str()) {
            failures.push(format!(
                "record {i}: unknown target '{target}' (expected one of {REQUIRED_TARGETS:?})"
            ));
            continue;
        }
        let cpu_s: i64 = match &rec.cpu_seconds {
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(v) => v,
                None => match n.as_f64() {
                    Some(f) => f as i64,
                    None => {
                        failures.push(format!(
                            "record {i} (target={target}): 'cpu_seconds' out of i64 range"
                        ));
                        continue;
                    }
                },
            },
            v => {
                failures.push(format!(
                    "record {i} (target={target}): 'cpu_seconds' is not a number ({v}) — refusing to coerce (jq fail-open guard)"
                ));
                continue;
            }
        };
        let ts_str = match &rec.timestamp {
            serde_json::Value::String(s) => s.as_str(),
            v => {
                failures.push(format!(
                    "record {i} (target={target}): 'timestamp' is not a string ({v})"
                ));
                continue;
            }
        };
        let ts = match chrono::DateTime::parse_from_rfc3339(ts_str) {
            Ok(t) => t.with_timezone(&chrono::Utc),
            Err(e) => {
                failures.push(format!(
                    "record {i} (target={target}): 'timestamp' not RFC 3339 ({e})"
                ));
                continue;
            }
        };
        earliest = Some(earliest.map_or(ts, |e| e.min(ts)));
        if ts >= cutoff {
            *per_target.entry(target.clone()).or_insert(0) += cpu_s;
            aggregate_in_window += cpu_s;
        }
    }

    // Bootstrap promotion: advisory until the ledger spans >= 90 days.
    let advisory = match earliest {
        None => true,
        Some(e) => (now - e).num_days() < WINDOW_DAYS,
    };

    for target in REQUIRED_TARGETS {
        let secs = per_target.get(*target).copied().unwrap_or(0);
        summary.push(format!(
            "{target}: {secs} s / {PER_TARGET_FLOOR_S} s (90d window)"
        ));
        if !advisory && secs < PER_TARGET_FLOOR_S {
            failures.push(format!(
                "target '{target}' below floor: {secs} s < {PER_TARGET_FLOOR_S} s (72 CPU-hr / 90d)"
            ));
        }
    }
    summary.push(format!(
        "aggregate: {aggregate_in_window} s / {AGGREGATE_FLOOR_S} s (90d window)"
    ));
    if !advisory && aggregate_in_window < AGGREGATE_FLOOR_S {
        failures.push(format!(
            "aggregate below floor: {aggregate_in_window} s < {AGGREGATE_FLOOR_S} s (1000 CPU-hr / 90d)"
        ));
    }
    if advisory {
        summary.push(format!(
            "ADVISORY: ledger spans < {WINDOW_DAYS} days — floor accumulating, not yet enforced"
        ));
    }

    Report {
        passed: failures.is_empty(),
        advisory,
        failures,
        summary,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root = std::env::current_dir().expect("failed to get current dir");
    let report = check_fuzz_floor(&workspace_root, chrono::Utc::now());

    // Advisory warnings go to stderr in JSON mode (#33) — stdout stays clean.
    if report.advisory {
        for line in &report.summary {
            emit_command(json, "warning", &format!("check-fuzz-floor: {line}"));
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": report.passed,
                "advisory": report.advisory,
                "failures": report.failures,
                "summary": report.summary,
            })
        );
    } else if report.passed {
        let label = if report.advisory {
            "PASS (advisory — floor accumulating, < 90d history)"
        } else {
            "PASS (floor met)"
        };
        eprintln!("check-fuzz-floor: {label}");
        for line in &report.summary {
            eprintln!("  {line}");
        }
    } else {
        for f in &report.failures {
            emit_command(json, "error", &format!("check-fuzz-floor: {f}"));
        }
        eprintln!(
            "check-fuzz-floor: FAIL — {} issue(s)",
            report.failures.len()
        );
    }

    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "check-fuzz-floor: {} issue(s) — see annotations",
            report.failures.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn now() -> chrono::DateTime<chrono::Utc> {
        // Fixed clock so the 90-day math is deterministic.
        chrono::DateTime::parse_from_rfc3339("2026-06-22T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn ts_iso(days_ago: i64) -> String {
        let t = now() - chrono::Duration::days(days_ago);
        t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    fn write_ledger(dir: &Path, records_json: &str) {
        fs::write(
            dir.join(FUZZ_LEDGER),
            format!("{{\"schema_version\":1,\"records\":{records_json}}}"),
        )
        .unwrap();
    }

    fn rec(target: &str, cpu: i64, days_ago: i64) -> String {
        format!(
            "{{\"target\":\"{target}\",\"cpu_seconds\":{cpu},\"timestamp\":\"{}\"}}",
            ts_iso(days_ago)
        )
    }

    #[test]
    fn advisory_when_ledger_absent() {
        let tmp = TempDir::new().unwrap();
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(r.passed, "no ledger must not block: {:?}", r.failures);
        assert!(r.advisory, "no ledger must be advisory");
    }

    #[test]
    fn advisory_when_ledger_empty() {
        let tmp = TempDir::new().unwrap();
        write_ledger(tmp.path(), "[]");
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(r.passed);
        assert!(r.advisory, "empty ledger must be advisory");
    }

    #[test]
    fn advisory_when_history_under_90_days_even_if_below_floor() {
        let tmp = TempDir::new().unwrap();
        // 10 days of history, far below floor — still advisory (bootstrapping).
        write_ledger(
            tmp.path(),
            &format!(
                "[{},{}]",
                rec("manifest_parser", 100, 10),
                rec("frame_deser", 100, 10)
            ),
        );
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(
            r.passed,
            "must not hard-fail during bootstrap: {:?}",
            r.failures
        );
        assert!(r.advisory);
    }

    #[test]
    fn fails_below_floor_once_history_is_mature() {
        let tmp = TempDir::new().unwrap();
        // A record 100 days ago matures the ledger (>= 90 d), but it is OUT of
        // the 90-day window → in-window sum is 0 → hard-fail on every target.
        write_ledger(
            tmp.path(),
            &format!(
                "[{},{}]",
                rec("manifest_parser", 1000, 100),
                rec("frame_deser", 1000, 100)
            ),
        );
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(!r.advisory, "100 d of history must promote to hard-enforce");
        assert!(
            !r.passed,
            "must fail when in-window sum is 0: {:?}",
            r.failures
        );
        assert!(r.failures.iter().any(|f| f.contains("manifest_parser")));
        assert!(r.failures.iter().any(|f| f.contains("frame_deser")));
        assert!(r.failures.iter().any(|f| f.contains("aggregate")));
    }

    #[test]
    fn passes_when_floor_met_with_mature_history() {
        let tmp = TempDir::new().unwrap();
        // Old record matures the ledger (100 d). Recent in-window records meet
        // both per-target and aggregate floors.
        let manifest_in_window = 2_000_000; // >= per-target (259 200) + contributes to aggregate
        let frame_in_window = 2_000_000; // 2 targets × 2 000 000 = 4 000 000 >= 3 600 000 aggregate
        write_ledger(
            tmp.path(),
            &format!(
                "[{},{},{},{}]",
                rec("manifest_parser", 1, 100), // matures, out of window
                rec("frame_deser", 1, 100),     // matures, out of window
                rec("manifest_parser", manifest_in_window, 5),
                rec("frame_deser", frame_in_window, 5),
            ),
        );
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(!r.advisory);
        assert!(r.passed, "floor met must pass: {:?}", r.failures);
    }

    #[test]
    fn hard_fails_on_string_cpu_seconds_no_advisory_coercion() {
        let tmp = TempDir::new().unwrap();
        // A string-typed cpu_seconds must be a hard FAILURE, never silently
        // coerced to satisfy the floor (the jq fail-open this gate replaces).
        write_ledger(
            tmp.path(),
            &format!(
                "[{{\"target\":\"manifest_parser\",\"cpu_seconds\":\"999999999\",\"timestamp\":\"{}\"}}]",
                ts_iso(100)
            ),
        );
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(!r.passed, "string cpu_seconds must fail the gate");
        assert!(
            r.failures.iter().any(|f| f.contains("not a number")),
            "failure must cite the type violation: {:?}",
            r.failures
        );
    }

    #[test]
    fn ignores_records_outside_90_day_window() {
        let tmp = TempDir::new().unwrap();
        // 200-day-old records do NOT count toward the in-window sum, but they DO
        // mature the ledger → hard-enforce → fail at 0 in-window seconds.
        write_ledger(
            tmp.path(),
            &format!(
                "[{},{}]",
                rec("manifest_parser", 9_999_999, 200),
                rec("frame_deser", 9_999_999, 200)
            ),
        );
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(!r.advisory);
        assert!(
            !r.passed,
            "out-of-window CPU time must not satisfy the floor"
        );
    }

    #[test]
    fn rejects_unknown_target() {
        let tmp = TempDir::new().unwrap();
        write_ledger(tmp.path(), &format!("[{}]", rec("bogus_target", 9999, 5)));
        let r = check_fuzz_floor(tmp.path(), now());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("unknown target")));
    }
}
