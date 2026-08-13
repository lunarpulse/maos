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
    root: std::path::PathBuf,
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
            if path
                .file_name()
                .map_or(false, |n| n == "methodology-attestation.json")
            {
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

        Ok(Self {
            scenarios,
            root: dir.canonicalize().map_err(crate::CorpusError::Io)?,
        })
    }

    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }

    /// Open the predecessor and successor manifests for every scenario.
    /// Corpus validity includes executable inputs, not descriptor cardinality.
    pub fn read_assets(
        &self,
    ) -> Result<Vec<(UpgradePolicyScenario, String, String)>, crate::CorpusError> {
        let workspace_root = self.root.ancestors().nth(4).ok_or_else(|| {
            crate::CorpusError::NotFound(format!(
                "workspace root above corpus {}",
                self.root.display()
            ))
        })?;
        self.scenarios
            .iter()
            .cloned()
            .map(|scenario| {
                let asset = |path: &str| {
                    let direct = std::path::Path::new(path);
                    let resolved = if direct.is_absolute() || direct.exists() {
                        direct.to_path_buf()
                    } else {
                        workspace_root.join(direct)
                    };
                    std::fs::read_to_string(resolved)
                };
                let predecessor = asset(&scenario.predecessor_manifest_path)?;
                let successor = asset(&scenario.successor_manifest_path)?;
                Ok((scenario, predecessor, successor))
            })
            .collect()
    }
}
