#![forbid(unsafe_code)]

//! Story 10.4a — RTO ≤ 4h gate (drilled, not printed).
//!
//! NFR-Ops-9 requires `RTO ≤ 4h` to be GATED, not just reported.
//! A restore drill exceeding 4h goes RED.  This gate is run weekly
//! via the `rpo-rto-cadence.yml` workflow; evidence is collected
//! on a dedicated ledger branch (modeled on `fuzz-cadence.yml`).
//!
//! The gate reads an evidence file (`rto-evidence.toml`) produced by
//! the weekly drill.  If the latest drill's `rto_seconds` exceeds the
//! threshold, the gate fails.

use crate::gate_common;
use std::path::Path;

/// 4 hours in seconds.
const RTO_THRESHOLD_SECS: u64 = 4 * 3600;

/// Evidence record from a weekly RTO drill.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RtoEvidence {
    /// ISO-8601 date of the drill.
    pub drill_date: String,
    /// Measured restore time in seconds.
    pub rto_seconds: u64,
    /// Whether the drill completed successfully.
    pub drill_success: bool,
    /// Optional notes.
    pub notes: Option<String>,
}

/// Evidence file shape.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RtoLedger {
    pub evidence: Vec<RtoEvidence>,
}

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    /// True when the gate could not measure (no evidence / not run).  Distinct
    /// from `passed` so a Skipped gate is never reported as a silent PASS.
    pub skipped: bool,
    pub threshold_secs: u64,
    pub latest_drill: Option<RtoEvidence>,
    pub verdict: String,
}

pub fn run(evidence_path: &str, json: bool) -> Result<(), String> {
    let report = check_rto(Path::new(evidence_path))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| format!("json: {e}"))?
        );
    } else if report.skipped {
        // SKIPPED — cannot measure (no drill evidence).  Non-blocking but
        // clearly labeled, never a silent PASS (Winston 10.2 verdict axis).
        println!("check-rto-gate: SKIPPED — {}", report.verdict);
    } else if report.passed {
        println!("check-rto-gate: PASSED — {}", report.verdict);
    } else {
        eprintln!("check-rto-gate: FAILED — {}", report.verdict);
    }

    // Skipped is non-blocking (Ok); only a measured FAIL blocks.
    if !report.skipped && !report.passed {
        return Err("check-rto-gate failed".into());
    }

    Ok(())
}

fn check_rto(evidence_path: &Path) -> Result<Report, String> {
    if !evidence_path.exists() {
        // No evidence — SKIPPED (not a silent PASS).
        return Ok(Report {
            passed: false,
            skipped: true,
            threshold_secs: RTO_THRESHOLD_SECS,
            latest_drill: None,
            verdict: "skipped — no RTO drill evidence yet (weekly cadence not run)".into(),
        });
    }

    let content = std::fs::read_to_string(evidence_path)
        .map_err(|e| format!("read evidence: {e}"))?;
    let ledger: RtoLedger =
        toml::from_str(&content).map_err(|e| format!("parse evidence: {e}"))?;

    // Pick the MOST RECENT drill by date (not just ledger.last()), so a stale
    // or cherry-picked ordering cannot satisfy the gate.
    let latest = ledger
        .evidence
        .iter()
        .max_by_key(|e| e.drill_date.clone())
        .ok_or("evidence file exists but has no entries")?;

    // Recency: the latest drill must be within 7 days (weekly cadence).  A
    // stale entry (too old) OR a future-dated entry (clock skew / a manipulated
    // ledger — a negative day count must NOT pass the recency check) means the
    // cadence is broken — a measured FAIL.  (P21: future dates bypassed the
    // old `num_days() > 7` check because a negative day-count is not > 7.)
    let days = match chrono::NaiveDate::parse_from_str(&latest.drill_date, "%Y-%m-%d") {
        Ok(d) => (chrono::Utc::now().date_naive() - d).num_days(),
        Err(_) => {
            return Ok(Report {
                passed: false,
                skipped: false,
                threshold_secs: RTO_THRESHOLD_SECS,
                latest_drill: Some(latest.clone()),
                verdict: format!("latest drill_date '{}' is not a valid date", latest.drill_date),
            });
        }
    };
    let stale = days < 0 || days > 7;

    let within = latest.drill_success && latest.rto_seconds <= RTO_THRESHOLD_SECS;
    let passed = within && !stale;

    let verdict = if days < 0 {
        format!(
            "latest drill on {} is FUTURE-DATED (clock skew or manipulated ledger)",
            latest.drill_date
        )
    } else if days > 7 {
        format!(
            "latest drill on {} is STALE (>7 days — weekly cadence broke)",
            latest.drill_date
        )
    } else if !latest.drill_success {
        format!("drill on {} FAILED (did not complete)", latest.drill_date)
    } else if latest.rto_seconds > RTO_THRESHOLD_SECS {
        format!(
            "RTO {}s > {}s threshold (drill {})",
            latest.rto_seconds, RTO_THRESHOLD_SECS, latest.drill_date
        )
    } else {
        format!(
            "RTO {}s ≤ {}s threshold (drill {})",
            latest.rto_seconds, RTO_THRESHOLD_SECS, latest.drill_date
        )
    };

    Ok(Report {
        passed,
        skipped: false,
        threshold_secs: RTO_THRESHOLD_SECS,
        latest_drill: Some(latest.clone()),
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn today() -> String {
        chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string()
    }

    #[test]
    fn rto_within_threshold_passes() {
    let today = today();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rto-evidence.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[[evidence]]
drill_date = "{today}"
rto_seconds = 3600
drill_success = true
"#
        )
        .unwrap();
        let report = check_rto(&path).unwrap();
        assert!(report.passed && !report.skipped, "verdict: {}", report.verdict);
    }

    #[test]
    fn rto_exceeds_threshold_fails() {
    let today = today();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rto-evidence.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[[evidence]]
drill_date = "{today}"
rto_seconds = 20000
drill_success = true
"#
        )
        .unwrap();
        let report = check_rto(&path).unwrap();
        assert!(!report.passed && !report.skipped);
    }

    #[test]
    fn rto_no_evidence_is_skipped_not_silent_pass() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let report = check_rto(&path).unwrap();
        assert!(report.skipped);
        assert!(!report.passed);
        assert!(report.verdict.contains("skipped"));
    }

    #[test]
    fn rto_stale_evidence_fails_recency() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rto-evidence.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        let stale = (chrono::Utc::now().date_naive() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        writeln!(
            f,
            r#"
[[evidence]]
drill_date = "{stale}"
rto_seconds = 60
drill_success = true
"#
        )
        .unwrap();
        let report = check_rto(&path).unwrap();
        assert!(!report.passed, "stale evidence must fail: {}", report.verdict);
        assert!(report.verdict.contains("STALE"));
    }

    #[test]
    fn rto_future_dated_evidence_fails_recency() {
        // P21: a future-dated drill entry must NOT pass the recency check.  A
        // negative day-count (drill_date ahead of today) is a broken cadence
        // (clock skew / manipulated ledger), never a PASS.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rto-evidence.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        let future = (chrono::Utc::now().date_naive() + chrono::Duration::days(10))
            .format("%Y-%m-%d")
            .to_string();
        writeln!(
            f,
            r#"
[[evidence]]
drill_date = "{future}"
rto_seconds = 60
drill_success = true
"#
        )
        .unwrap();
        let report = check_rto(&path).unwrap();
        assert!(
            !report.passed,
            "future-dated evidence must fail: {}",
            report.verdict
        );
        assert!(
            report.verdict.contains("FUTURE-DATED"),
            "future-dated verdict: {}",
            report.verdict
        );
    }

    #[test]
    fn rto_drill_failure_fails() {
    let today = today();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rto-evidence.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[[evidence]]
drill_date = "{today}"
rto_seconds = 60
drill_success = false
"#
        )
        .unwrap();
        let report = check_rto(&path).unwrap();
        assert!(!report.passed);
    }
}
