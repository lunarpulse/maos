use std::path::Path;

use crate::corpus_types::{load_toml, CorpusManifest, CoverageMatrixFile, GateRegistry, PhaseConfig};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub id: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeferredRow {
    pub id: String,
    pub phase: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub mode: String,
    pub violations: Vec<Violation>,
    pub out_of_scope_deferred: Vec<DeferredRow>,
    pub checked: usize,
}

pub fn run(config_path: &str, phase_config_path: &str, manifest_path: &str, gate_registry_path: &str, json: bool) -> Result<(), String> {
    let report = check_coverage_matrix(Path::new(config_path), Path::new(phase_config_path), Path::new(manifest_path), Path::new(gate_registry_path))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}")));
    } else {
        if report.passed && report.violations.is_empty() {
            println!("coverage-matrix: PASSED ({} rows checked)", report.checked);
        } else {
            for v in &report.violations { eprintln!("{v}"); }
        }
        for d in &report.out_of_scope_deferred { eprintln!("coverage-matrix: deferred {} at phase {}", d.id, d.phase); }
    }
    if !report.violations.is_empty() && report.mode == "hard" { return Err("coverage-matrix failed".into()); }
    Ok(())
}

fn check_coverage_matrix(config_path: &Path, phase_config_path: &Path, manifest_path: &Path, gate_registry_path: &Path) -> Result<Report, String> {
    let yaml_src = std::fs::read_to_string(config_path).map_err(|e| format!("cannot read {}: {e}", config_path.display()))?;
    let coverage: CoverageMatrixFile = serde_yaml::from_str(&yaml_src).map_err(|e| format!("yaml parse error in {}: {e}", config_path.display()))?;
    let phase_config: PhaseConfig = load_toml(phase_config_path)?;
    let manifest: CorpusManifest = if manifest_path.exists() { load_toml(manifest_path)? } else { CorpusManifest { corpus: std::collections::BTreeMap::new() } };
    let registry: GateRegistry = load_toml(gate_registry_path)?;

    let manifest_keys: std::collections::HashSet<String> = manifest.corpus.keys().cloned().collect();
    let registry_keys: std::collections::HashSet<String> = registry.gates.into_iter().collect();
    let mut violations = Vec::new();
    let mut deferred = Vec::new();
    let mut checked = 0usize;

    if coverage.current_phase != phase_config.current_phase {
        violations.push(Violation { id: "phase-mismatch".into(), message: format!("NFR-Meta-3 violation: phase mismatch — coverage-matrix.yaml says {}, phase-config.toml says {}", coverage.current_phase, phase_config.current_phase) });
    }
    if !coverage.phase_order.contains(&coverage.current_phase) {
        violations.push(Violation { id: "invalid-current-phase".into(), message: format!("NFR-Meta-3 violation: current_phase '{}' not in phase_order", coverage.current_phase) });
    }
    if !matches!(coverage.mode.as_str(), "warning" | "hard") {
        violations.push(Violation { id: "invalid-mode".into(), message: format!("NFR-Meta-3 violation: mode must be 'warning' or 'hard', got '{}'", coverage.mode) });
    }
    if phase_config.phase_order != coverage.phase_order {
        violations.push(Violation { id: "phase-order-drift".into(), message: format!("NFR-Meta-3 violation: phase_order mismatch — coverage-matrix.yaml and phase-config.toml diverge") });
    }

    for (id, row) in &coverage.coverage {
        checked += 1;
        if !coverage.phase_order.contains(&row.phase) {
            violations.push(Violation { id: id.clone(), message: format!("NFR-Meta-3 violation: {} has invalid phase '{}' not in phase_order", id, row.phase) });
            continue;
        }
        for corpus_name in &row.corpora {
            if !manifest_keys.contains(corpus_name) {
                violations.push(Violation { id: id.clone(), message: format!("NFR-Meta-3 violation: {} references unknown corpus '{}' (not in tests/corpora/MANIFEST.toml)", id, corpus_name) });
            }
        }
        for gate_name in &row.gates {
            if !registry_keys.contains(gate_name) {
                violations.push(Violation { id: id.clone(), message: format!("NFR-Meta-3 violation: {} references unknown gate '{}' (not in xtask/gate-registry.toml)", id, gate_name) });
            }
        }
        if phase_le(&row.phase, &coverage.current_phase, &coverage.phase_order) {
            if row.gates.is_empty() && row.corpora.is_empty() {
                violations.push(Violation { id: id.clone(), message: format!("NFR-Meta-3 violation: {} delivered at {} has zero corpus and zero gate coverage", id, row.phase) });
            }
        } else {
            deferred.push(DeferredRow { id: id.clone(), phase: row.phase.clone() });
        }
    }

    let passed = violations.is_empty();
    Ok(Report { passed, mode: coverage.mode.clone(), violations, out_of_scope_deferred: deferred, checked })
}

pub fn phase_le(a: &str, b: &str, order: &[String]) -> bool {
    let a_idx = order.iter().position(|p| p == a);
    let b_idx = order.iter().position(|p| p == b);
    match (a_idx, b_idx) { (Some(ai), Some(bi)) => ai <= bi, _ => false }
}

#[cfg(test)]
mod tests { include!("tests/coverage_matrix_tests.rs"); }
