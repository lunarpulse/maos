use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::corpus_types::{load_toml, CorpusManifest};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub kind: String,
    pub corpus: String,
    pub path: String,
    pub detail: String,
    pub expected_hash: Option<String>,
    pub computed_hash: Option<String>,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind.as_str() {
            "integrity" => write!(f, "NFR-Test-1 violation: corpus integrity broken — {} at {}: manifest expected {}, computed {}",
                self.corpus, self.path, self.expected_hash.as_deref().unwrap_or("?"), self.computed_hash.as_deref().unwrap_or("?")),
            "missing" => write!(f, "NFR-Test-1 violation: corpus missing — {} at {}: file does not exist", self.corpus, self.path),
            "unregistered" => write!(f, "NFR-Test-1 violation: corpus unregistered — {} has no manifest entry (use 'cargo xtask check-corpus --register <name>' to compute its SHA-256)", self.path),
            "malformed" => write!(f, "NFR-Test-1 violation: corpus malformed — {} at {}: {}", self.corpus, self.path, self.detail),
            _ => write!(f, "NFR-Test-1 violation: {} — {} at {}: {}", self.kind, self.corpus, self.path, self.detail),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub checked: usize,
}

pub fn run(
    manifest_path: &str,
    corpora_dir: &str,
    register: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if let Some(name) = register {
        return register_corpus(name, corpora_dir);
    }
    let report = check_corpus(Path::new(manifest_path), Path::new(corpora_dir))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}"))
        );
    } else if report.passed {
        println!(
            "check-corpus: PASSED ({} corpus entries checked)",
            report.checked
        );
    } else {
        for v in &report.violations {
            eprintln!("{v}");
        }
    }
    if !report.passed {
        return Err("check-corpus failed".into());
    }
    Ok(())
}

fn check_corpus(manifest_path: &Path, corpora_dir: &Path) -> Result<Report, String> {
    let manifest: CorpusManifest = load_toml(manifest_path)?;
    let mut violations = Vec::new();
    let mut checked = 0usize;
    let manifest_keys: HashSet<String> = manifest.corpus.keys().cloned().collect();

    for (name, entry) in &manifest.corpus {
        // Story 10.4a — deterministically GENERATED corpora (e.g. the
        // `migration-corpus-1e6` SQLite Transparency-Log fixture) are not
        // committed `.jsonl` files; their integrity is verified by the
        // dedicated `check-migration-merkle` triple-oracle gate. Skip the
        // JSONL existence/hash checks here so a generated SQLite corpus is not
        // reported as a missing `.jsonl`.
        if entry.generated {
            continue;
        }
        checked += 1;
        let path = corpora_dir.join(format!("{}.jsonl", name));
        let path_str = path.display().to_string();
        if !path.exists() {
            violations.push(Violation {
                kind: "missing".into(),
                corpus: name.clone(),
                path: path_str,
                detail: String::new(),
                expected_hash: None,
                computed_hash: None,
            });
            continue;
        }
        let file =
            fs::File::open(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        let reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut line_no = 0usize;
        let mut parse_error: Option<String> = None;
        for line_result in reader.lines() {
            line_no += 1;
            let line = line_result
                .map_err(|e| format!("read error at {}:{}: {e}", path.display(), line_no))?;
            hasher.update(line.as_bytes());
            hasher.update(b"\n");
            if parse_error.is_none() {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&line) {
                    parse_error = Some(format!("{}:{}: {e}", path.display(), line_no));
                }
            }
        }
        if let Some(err) = parse_error {
            violations.push(Violation {
                kind: "malformed".into(),
                corpus: name.clone(),
                path: path_str.clone(),
                detail: err,
                expected_hash: None,
                computed_hash: None,
            });
        }
        if line_no != entry.item_count {
            violations.push(Violation {
                kind: "item-count-mismatch".into(),
                corpus: name.clone(),
                path: path_str.clone(),
                detail: format!(
                    "manifest claims item_count={}, file has {} lines",
                    entry.item_count, line_no
                ),
                expected_hash: None,
                computed_hash: None,
            });
        }
        let computed = hex_encode(&hasher.finalize());
        if computed != entry.sha256 {
            violations.push(Violation {
                kind: "integrity".into(),
                corpus: name.clone(),
                path: path_str,
                detail: format!("{}|{}", entry.sha256, computed),
                expected_hash: Some(entry.sha256.clone()),
                computed_hash: Some(computed),
            });
        }
    }

    if corpora_dir.exists() {
        let mut orphan_files = Vec::new();
        collect_jsonl_files(corpora_dir, &mut orphan_files);
        for file in &orphan_files {
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if !manifest_keys.contains(stem) {
                violations.push(Violation {
                    kind: "unregistered".into(),
                    corpus: stem.into(),
                    path: file.display().to_string(),
                    detail: String::new(),
                    expected_hash: None,
                    computed_hash: None,
                });
            }
        }
    }

    let passed = violations.is_empty();
    Ok(Report {
        passed,
        violations,
        checked,
    })
}

fn register_corpus(name: &str, corpora_dir: &str) -> Result<(), String> {
    let path = Path::new(corpora_dir).join(format!("{}.jsonl", name));
    if !path.exists() {
        return Err(format!(
            "cannot register: {} does not exist",
            path.display()
        ));
    }
    let file = fs::File::open(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut item_count = 0usize;
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| format!("read error in {}: {e}", path.display()))?;
        serde_json::from_str::<serde_json::Value>(&line).map_err(|e| {
            format!(
                "json parse error in {} line {}: {e}",
                path.display(),
                item_count + 1
            )
        })?;
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
        item_count += 1;
    }
    let computed = hex_encode(&hasher.finalize());
    println!(
        r#"[corpus.{}]
sha256 = "{}"
schema_version = 1
item_count = {}
valid_until = "YYYY-MM-DD"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "<add description>"
"#,
        name, computed, item_count
    );
    Ok(())
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn collect_jsonl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_jsonl_files(&path, out);
            } else if path.extension() == Some(std::ffi::OsStr::new("jsonl")) {
                out.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    include!("tests/check_corpus_tests.rs");
}
