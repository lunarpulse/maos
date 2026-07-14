// Clock source: chrono::Utc::now().date_naive() — no time-of-day; pure date comparison.

use std::path::Path;

use chrono::NaiveDate;

use crate::corpus_types::{load_toml, CorpusManifest, CoverageMatrixFile};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub kind: String,
    pub id: String,
    pub valid_until: String,
    pub current_date: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Warning {
    pub id: String,
    pub valid_until: String,
    pub current_date: String,
    pub days_remaining: i64,
    pub message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub warnings: Vec<Warning>,
}

pub fn run(
    config_path: &str,
    manifest_path: &str,
    warn_window_days: i64,
    json: bool,
) -> Result<(), String> {
    if warn_window_days < 0 {
        return Err("warn-window-days must be >= 0".into());
    }
    let today = chrono::Utc::now().date_naive();
    let report = check_staleness(
        Path::new(config_path),
        Path::new(manifest_path),
        today,
        warn_window_days,
    )?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}"))
        );
    } else if report.passed && report.violations.is_empty() && report.warnings.is_empty() {
        println!("corpus-staleness: PASSED (no expired or near-expiry rows)");
    } else {
        for v in &report.violations {
            eprintln!("{v}");
        }
        for w in &report.warnings {
            eprintln!("corpus-staleness warning: {}", w.message);
        }
    }
    if !report.violations.is_empty() {
        return Err("corpus-staleness failed".into());
    }
    Ok(())
}

fn check_staleness(
    config_path: &Path,
    manifest_path: &Path,
    today: NaiveDate,
    warn_window_days: i64,
) -> Result<Report, String> {
    let yaml_src = std::fs::read_to_string(config_path)
        .map_err(|e| format!("cannot read {}: {e}", config_path.display()))?;
    let coverage: CoverageMatrixFile = serde_yaml::from_str(&yaml_src)
        .map_err(|e| format!("yaml parse error in {}: {e}", config_path.display()))?;
    let manifest: CorpusManifest = if manifest_path.exists() {
        load_toml(manifest_path)?
    } else {
        CorpusManifest {
            corpus: std::collections::BTreeMap::new(),
        }
    };

    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    for (id, row) in &coverage.coverage {
        if !crate::coverage_matrix::phase_le(
            &row.phase,
            &coverage.current_phase,
            &coverage.phase_order,
        ) {
            continue;
        }
        push_date_result(
            id,
            &row.valid_until,
            today,
            warn_window_days,
            true,
            &mut violations,
            &mut warnings,
        );
    }

    for (name, entry) in &manifest.corpus {
        push_date_result(
            name,
            &entry.valid_until,
            today,
            warn_window_days,
            false,
            &mut violations,
            &mut warnings,
        );
    }

    let passed = violations.is_empty();
    Ok(Report {
        passed,
        violations,
        warnings,
    })
}

fn push_date_result(
    id: &str,
    valid_until: &str,
    today: NaiveDate,
    warn_window_days: i64,
    is_matrix: bool,
    violations: &mut Vec<Violation>,
    warnings: &mut Vec<Warning>,
) {
    match check_date(valid_until, today, warn_window_days) {
        DateCheck::InvalidFormat => {
            let msg = if is_matrix {
                format!(
                    "NFR-Meta-2 violation: {} valid_until \"{}\" not in YYYY-MM-DD format",
                    id, valid_until
                )
            } else {
                format!(
                    "NFR-Meta-2 violation: corpus {} valid_until \"{}\" not in YYYY-MM-DD format",
                    id, valid_until
                )
            };
            violations.push(Violation {
                kind: "invalid-format".into(),
                id: id.into(),
                valid_until: valid_until.into(),
                current_date: today.to_string(),
                message: msg,
            });
        }
        DateCheck::Expired => {
            let prefix = if is_matrix {
                "NFR-Meta-2 violation"
            } else {
                "NFR-Meta-2 violation: corpus"
            };
            let msg = if is_matrix {
                format!(
                    "{}: {} corpus expired {} (current={}); either extend with assessor sign-off PR or rebuild",
                    prefix, id, valid_until, today
                )
            } else {
                format!(
                    "{} {} expired {}; either extend with assessor sign-off PR or rebuild",
                    prefix, id, valid_until
                )
            };
            violations.push(Violation {
                kind: "expired".into(),
                id: id.into(),
                valid_until: valid_until.into(),
                current_date: today.to_string(),
                message: msg,
            });
        }
        DateCheck::Warning(days) => {
            let msg = if is_matrix {
                format!(
                    "{} corpus expires in {} days ({}); consider extension or rebuild",
                    id, days, valid_until
                )
            } else {
                format!(
                    "corpus {} expires in {} days ({}); consider extension or rebuild",
                    id, days, valid_until
                )
            };
            warnings.push(Warning {
                id: id.into(),
                valid_until: valid_until.into(),
                current_date: today.to_string(),
                days_remaining: days,
                message: msg,
            });
        }
        DateCheck::Ok => {}
    }
}

enum DateCheck {
    Ok,
    Warning(i64),
    Expired,
    InvalidFormat,
}

fn check_date(date_str: &str, today: NaiveDate, warn_window_days: i64) -> DateCheck {
    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return DateCheck::InvalidFormat,
    };
    if date < today {
        DateCheck::Expired
    } else {
        let days = (date - today).num_days();
        if days <= warn_window_days {
            DateCheck::Warning(days)
        } else {
            DateCheck::Ok
        }
    }
}

#[cfg(test)]
mod tests {
    include!("tests/corpus_staleness_tests.rs");
}
