#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One retract scenario from the retract corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetractScenario {
    pub scenario_id: String,
    pub category: String,
    pub description: String,
    pub original_frame: OriginalFrameDesc,
    pub retract_request: RetractRequestDesc,
    pub expected_outcome: RetractExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalFrameDesc {
    pub frame_id_hex: String,
    pub from_spirit: String,
    pub to_spirit: String,
    pub kind: String,
    pub payload_size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetractRequestDesc {
    pub retracting_spirit: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetractExpectedOutcome {
    pub success: bool,
    pub outcome_variant: String,
    pub error_variant: Option<String>,
}

pub struct RetractCorpus {
    pub scenarios: Vec<RetractScenario>,
}

impl RetractCorpus {
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
            if !path.file_stem().map_or(false, |s| s.to_str().map_or(false, |s| s.starts_with("scenario-"))) {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let scenario: RetractScenario =
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
