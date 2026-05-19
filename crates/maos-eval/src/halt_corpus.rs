#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One halt scenario from the N=50 synthetic-v0 corpus.
///
/// The fixture format is JSON-per-file under
/// `crates/maos-eval/fixtures/halt-corpus-v0/`; each file
/// is a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaltScenario {
    pub scenario_id: String,
    pub tag: String,
    pub spirit_class: String,
    pub epistemic_policy_rules: Vec<PolicyRule>,
    pub scalar_writes: Vec<ScalarWrite>,
    pub expected_halt_invocation: bool,
    pub expected_halt_payload: ExpectedHaltPayload,
    pub expected_resolution: String,
    pub ground_truth_class: HaltScenarioOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub tag: String,
    pub rule: String,
    pub threshold: f64,
    /// Story 4.2 — optional lower/upper bounds for `on_value_within`
    /// and `on_value_outside` predicates.
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarWrite {
    pub tag: String,
    pub value: f64,
    pub derived_from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedHaltPayload {
    pub tag: String,
    pub value: f64,
    pub threshold: f64,
    pub policy_id: String,
    pub derived_from: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HaltScenarioOutcome {
    TruePositive,
    TrueNegative,
    FalsePositive,
    FalseNegative,
}

pub struct HaltCorpus {
    pub scenarios: Vec<HaltScenario>,
}

impl HaltCorpus {
    pub fn load_from(dir: &Path) -> Result<Self, crate::CorpusError> {
        if !dir.is_dir() {
            return Err(crate::CorpusError::NotFound(
                dir.display().to_string(),
            ));
        }

        let mut scenarios = Vec::new();
        for entry in walkdir::WalkDir::new(dir)
            .sort_by_file_name()
            .into_iter()
        {
            let entry = entry.map_err(|e| {
                let msg = e.to_string();
                crate::CorpusError::Io(
                    e.into_io_error()
                        .unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, msg))
                )
            })?;
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let scenario: HaltScenario = serde_json::from_str(&content).map_err(|e| {
                crate::CorpusError::Parse {
                    path: path.display().to_string(),
                    source: e,
                }
            })?;
            scenarios.push(scenario);
        }

        Ok(Self { scenarios })
    }

    pub fn len(&self) -> usize {
        self.scenarios.len()
    }
}
