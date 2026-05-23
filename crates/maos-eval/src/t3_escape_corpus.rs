#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T3EscapeScenario {
    pub scenario_id: String,
    pub category: String,
    pub attack_payload: T3AttackPayload,
    pub attack_surface: String,
    pub expected_outcome: T3ExpectedOutcome,
    pub tier_target: String,
    pub split: String,
    pub preconditions: T3Preconditions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T3AttackPayload {
    pub command: Vec<String>,
    pub expected_runtime_blocker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T3ExpectedOutcome {
    pub block_observed: bool,
    pub audit_event_emitted: String,
    pub frame_kind: String,
    pub attempted_syscall_substring: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T3Preconditions {
    pub linux: bool,
    pub container_runtime_available: bool,
}

pub struct T3EscapeCorpus {
    pub scenarios: Vec<T3EscapeScenario>,
}

impl T3EscapeCorpus {
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
            let scenario: T3EscapeScenario =
                serde_json::from_str(&content).map_err(|e| crate::CorpusError::Parse {
                    path: path.display().to_string(),
                    source: e,
                })?;
            scenarios.push(scenario);
        }

        Ok(Self { scenarios })
    }
}
