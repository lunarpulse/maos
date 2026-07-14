use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::corpus_types::{load_toml, round_ratio, CorpusManifest};

/// Trait abstracting judge-LLM invocations.
pub trait JudgeRunner {
    fn judge(&self, item: &serde_json::Value, expected: &serde_json::Value)
        -> Result<bool, String>;
}

/// v0.1-alpha shim: compares item["expected_judgment"] == expected (trivially passes).
pub struct OfflineMode;

impl JudgeRunner for OfflineMode {
    fn judge(
        &self,
        item: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<bool, String> {
        Ok(item
            .get("expected_judgment")
            .unwrap_or(&serde_json::Value::Null)
            == expected)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CorpusAgreement {
    pub corpus: String,
    pub items_total: usize,
    pub items_agreed: usize,
    pub agreement_ratio: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RebaselineReport {
    pub passed: bool,
    pub items_total: usize,
    pub items_agreed: usize,
    pub agreement_ratio: f64,
    pub threshold: f64,
    pub per_corpus: Vec<CorpusAgreement>,
}

pub fn run(
    manifest_path: &str,
    corpora_dir: &str,
    _judge_config_path: &str,
    threshold: f64,
    out: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let report = rebaseline_check(Path::new(manifest_path), Path::new(corpora_dir), threshold)?;
    if let Some(out_path) = out {
        fs::write(
            out_path,
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}")),
        )
        .map_err(|e| format!("cannot write report to {}: {e}", out_path))?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}"))
        );
    } else if report.passed {
        println!(
            "rebaseline-check: PASSED ({} corpus entries, agreement={:.4})",
            report.per_corpus.len(),
            report.agreement_ratio
        );
    } else {
        for c in &report.per_corpus {
            if c.agreement_ratio < threshold {
                eprintln!(
                    "NFR-Test-1 violation: corpus {} agreement ratio {:.4} below quarterly threshold 0.98 — open re-baseline review issue",
                    c.corpus, c.agreement_ratio
                );
            }
        }
    }
    if !report.passed {
        return Err("rebaseline-check failed".into());
    }
    Ok(())
}

fn rebaseline_check(
    manifest_path: &Path,
    corpora_dir: &Path,
    threshold: f64,
) -> Result<RebaselineReport, String> {
    let manifest: CorpusManifest = load_toml(manifest_path)?;
    let mut per_corpus = Vec::new();
    let mut items_total = 0usize;
    let mut items_agreed = 0usize;

    for (name, entry) in &manifest.corpus {
        if entry.judge_id.is_none() {
            continue;
        }
        let path = corpora_dir.join(format!("{}.jsonl", name));
        if !path.exists() {
            per_corpus.push(CorpusAgreement {
                corpus: name.clone(),
                items_total: 0,
                items_agreed: 0,
                agreement_ratio: f64::NAN,
            });
            continue;
        }
        let file =
            fs::File::open(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        let reader = BufReader::new(file);
        let judge = OfflineMode;
        let mut corpus_total = 0usize;
        let mut corpus_agreed = 0usize;
        let mut corpus_errors = 0usize;
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("read error at {}: {e}", path.display()))?;
            let val: serde_json::Value = serde_json::from_str(&line)
                .map_err(|e| format!("json parse error in {}: {e}", path.display()))?;
            let expected = val
                .get("expected_judgment")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            corpus_total += 1;
            match judge.judge(&val, &expected) {
                Ok(true) => corpus_agreed += 1,
                Ok(false) => {}
                Err(_) => corpus_errors += 1,
            }
        }
        let ratio = if corpus_total > 0 {
            round_ratio(corpus_agreed as f64 / corpus_total as f64)
        } else {
            1.0
        };
        if corpus_errors > 0 {
            eprintln!(
                "rebaseline-check: corpus {} had {corpus_errors} judge errors counted as disagreements",
                name
            );
        }
        per_corpus.push(CorpusAgreement {
            corpus: name.clone(),
            items_total: corpus_total,
            items_agreed: corpus_agreed,
            agreement_ratio: ratio,
        });
        items_total += corpus_total;
        items_agreed += corpus_agreed;
    }

    let agreement_ratio = if items_total > 0 {
        round_ratio(items_agreed as f64 / items_total as f64)
    } else {
        1.0
    };
    let passed = per_corpus.iter().all(|c| c.agreement_ratio >= threshold)
        && per_corpus.iter().all(|c| !c.agreement_ratio.is_nan());
    Ok(RebaselineReport {
        passed,
        items_total,
        items_agreed,
        agreement_ratio,
        threshold,
        per_corpus,
    })
}

#[cfg(test)]
mod tests {
    include!("tests/rebaseline_check_tests.rs");
}
