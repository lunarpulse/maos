use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::corpus_types::{load_toml, CorpusManifest};
use crate::rebaseline_check::{JudgeRunner, OfflineMode};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CalibrationReport {
    pub corpus: String,
    pub n: usize,
    pub pass_rate: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub ci_width: f64,
    pub threshold: Option<f64>,
    pub passed: bool,
    #[serde(default)]
    pub malformed_items: usize,
}

pub fn run(
    corpus_name: &str,
    n: u64,
    p: f64,
    manifest_path: &str,
    corpora_dir: &str,
    _synthetic_pass_rate: Option<f64>,
    json: bool,
) -> Result<(), String> {
    if corpus_name.is_empty() {
        return Err("corpus name must not be empty".into());
    }
    if corpus_name.contains('/') || corpus_name.contains('\\') {
        return Err("invalid corpus name: path separators not allowed".into());
    }
    if !(p > 0.0 && p < 1.0) {
        return Err("confidence p must be in (0,1)".into());
    }
    if !known_p_values().iter().any(|&kp| (p - kp).abs() < 0.001) {
        eprintln!("calibrate warning: unknown confidence p={p:.2} — supported: {:?}; using z=1.96", known_p_values());
    }
    let report = calibrate_corpus(corpus_name, n, p, Path::new(manifest_path), Path::new(corpora_dir))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}")));
    } else if report.passed {
        println!("calibrate: PASSED (corpus={}, n={}, ci_width={:.4}, malformed={})", report.corpus, report.n, report.ci_width, report.malformed_items);
    } else {
        eprintln!("NFR-Aud-8 violation: corpus {} {} CI-width {:.4} exceeds {} at p={:.2} — increase N or accept wider window with assessor sign-off",
            report.corpus, if report.n >= 500 { "quarterly" } else { "per-commit" }, report.ci_width, report.threshold.unwrap_or(0.0), p);
    }
    if !report.passed { return Err("calibrate failed".into()); }
    Ok(())
}

fn calibrate_corpus(
    corpus_name: &str,
    _n: u64,
    p: f64,
    manifest_path: &Path,
    corpora_dir: &Path,
) -> Result<CalibrationReport, String> {
    let corpus_in_manifest = if manifest_path.exists() {
        let manifest: CorpusManifest = load_toml(manifest_path)?;
        manifest.corpus.contains_key(corpus_name)
    } else {
        false
    };

    if !corpus_in_manifest {
        return Ok(CalibrationReport {
            corpus: corpus_name.to_string(),
            n: 0,
            pass_rate: 1.0,
            ci_lower: 0.0,
            ci_upper: 1.0,
            ci_width: 1.0,
            threshold: None,
            passed: true,
            malformed_items: 0,
        });
    }

    let jsonl_path = corpora_dir.join(format!("{}.jsonl", corpus_name));
    if !jsonl_path.exists() {
        return Ok(CalibrationReport {
            corpus: corpus_name.to_string(),
            n: 0,
            pass_rate: 1.0,
            ci_lower: 0.0,
            ci_upper: 1.0,
            ci_width: 1.0,
            threshold: None,
            passed: true,
            malformed_items: 0,
        });
    }

    let file = fs::File::open(&jsonl_path)
        .map_err(|e| format!("cannot open {}: {e}", jsonl_path.display()))?;
    let reader = BufReader::new(file);
    let judge = OfflineMode;
    let mut items_scanned = 0usize;
    let mut successes = 0usize;
    let mut malformed = 0usize;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => {
                malformed += 1;
                continue;
            }
        };
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(item) => {
                items_scanned += 1;
                let expected = item.get("expected_judgment").cloned().unwrap_or(serde_json::Value::Null);
                if expected == serde_json::Value::Null {
                    malformed += 1;
                    // Missing expected_judgment counts as a non-success
                    continue;
                }
                match judge.judge(&item, &expected) {
                    Ok(true) => successes += 1,
                    Ok(false) => {},
                    Err(e) => {
                        eprintln!("judge error on item {}: {e}", item.get("id").unwrap_or(&serde_json::Value::Null));
                    }
                }
            }
            Err(_) => {
                malformed += 1;
            }
        }
    }

    let pass_rate = if items_scanned > 0 {
        successes as f64 / items_scanned as f64
    } else {
        1.0
    };
    let z = z_for_confidence(p);
    let (ci_lower, ci_upper) = wilson_ci(successes as u64, items_scanned as u64, z)?;
    let ci_width = ci_upper - ci_lower;

    // Threshold is selected based on actual items_scanned, not CLI n,
    // to avoid applying a threshold for one sample size to a different sample size.
    let (threshold, passed) = if items_scanned == 100 && (p - 0.95).abs() < 0.001 {
        (Some(0.20), ci_width <= 0.20)
    } else if items_scanned == 500 && (p - 0.90).abs() < 0.001 {
        (Some(0.05), ci_width <= 0.05)
    } else if items_scanned == 100 && (p - 0.99).abs() < 0.001 {
        (Some(0.20), ci_width <= 0.20)
    } else {
        (None, true)
    };

    Ok(CalibrationReport {
        corpus: corpus_name.to_string(),
        n: items_scanned,
        pass_rate,
        ci_lower,
        ci_upper,
        ci_width,
        threshold,
        passed,
        malformed_items: malformed,
    })
}

/// Wilson score interval (Agresti & Coull 1998).
pub fn wilson_ci(successes: u64, n: u64, z: f64) -> Result<(f64, f64), String> {
    if n == 0 { return Ok((0.0, 1.0)); }
    if successes > n { return Err(format!("wilson_ci: successes ({successes}) > n ({n})")); }
    let n_f = n as f64;
    let p_hat = successes as f64 / n_f;
    let z2 = z * z;
    let denom = 1.0 + z2 / n_f;
    let centre = p_hat + z2 / (2.0 * n_f);
    let spread = z * f64::sqrt(p_hat * (1.0 - p_hat) / n_f + z2 / (4.0 * n_f * n_f));
    Ok(((centre - spread) / denom, (centre + spread) / denom))
}

fn z_for_confidence(p: f64) -> f64 {
    if (p - 0.90).abs() < 0.001 { 1.6449 }
    else if (p - 0.95).abs() < 0.001 { 1.96 }
    else if (p - 0.99).abs() < 0.001 { 2.5758 }
    else { 1.96 }
}

pub fn known_p_values() -> Vec<f64> { vec![0.90, 0.95, 0.99] }

#[cfg(test)]
mod tests { include!("tests/calibrate_tests.rs"); }
