#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One upgrade-policy scenario from the upgrade-policy corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePolicyScenario {
    pub scenario_id: String,
    pub category: String,
    pub predecessor_manifest_path: String,
    pub successor_manifest_path: String,
    pub policy: String,
    pub expected_outcome: UpgradePolicyExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePolicyExpectedOutcome {
    pub report_outcome: String,
    pub lifecycle_event_journaled: bool,
    pub halt_receipts_produced_min: usize,
}

pub struct UpgradePolicyCorpus {
    pub scenarios: Vec<UpgradePolicyScenario>,
}

impl UpgradePolicyCorpus {
    pub fn load_from(dir: &Path) -> Result<Self, crate::CorpusError> {
        if !dir.is_dir() {
            return Err(crate::CorpusError::NotFound(dir.display().to_string()));
        }

        let mut scenarios = Vec::new();
        for entry in walkdir::WalkDir::new(dir).sort_by_file_name().into_iter() {
            let entry = entry.map_err(|e| {
                let msg = e.to_string();
                crate::CorpusError::Io(
                    e.into_io_error()
                        .unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, msg)),
                )
            })?;
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }
            if path.file_name().map_or(false, |n| n == "methodology-attestation.json") {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let scenario: UpgradePolicyScenario =
                serde_json::from_str(&content).map_err(|e| crate::CorpusError::Parse {
                    path: path.display().to_string(),
                    source: e,
                })?;
            scenarios.push(scenario);
        }

        Ok(Self { scenarios })
    }

    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }
}
