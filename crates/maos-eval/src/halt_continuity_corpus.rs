#![forbid(unsafe_code)]

//! Halt-continuity corpus loader (Story 5.2, AC4).
//!
//! Loads the 12-scenario halt-continuity corpus from
//! `crates/maos-eval/fixtures/halt-continuity-corpus-v0/` and exposes
//! per-scenario access for the end-to-end integration test.

use crate::CorpusError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A halt-continuity corpus scenario.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HaltContinuityScenario {
    pub scenario_id: String,
    pub tier_tag: String,
    pub predecessor: PredecessorSpec,
    pub successor: SuccessorSpec,
    pub expected_outcome: ExpectedOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PredecessorSpec {
    pub spirit_class: String,
    pub version: String,
    pub halt_protocol_version: u32,
    pub pending_halts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SuccessorSpec {
    pub spirit_class: String,
    pub version: String,
    pub halt_protocol_compatibility: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedOutcome {
    pub verdict: String,
    pub drained_count: Option<usize>,
    pub migrated_count: Option<usize>,
    pub expected_error: Option<String>,
}

/// A loaded halt-continuity corpus.
pub struct HaltContinuityCorpus {
    pub scenarios: Vec<HaltContinuityScenario>,
}

impl HaltContinuityCorpus {
    /// Load all scenarios from the corpus directory.
    pub fn load(corpus_path: impl AsRef<Path>) -> Result<Self, CorpusError> {
        let scenarios_dir = corpus_path.as_ref().join("scenarios");
        let mut scenarios = Vec::new();

        if !scenarios_dir.is_dir() {
            return Err(CorpusError::NotFound(scenarios_dir.display().to_string()));
        }

        let mut entries: Vec<_> =
            std::fs::read_dir(&scenarios_dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let content = std::fs::read_to_string(&path)?;
                let scenario: HaltContinuityScenario =
                    serde_json::from_str(&content).map_err(|e| CorpusError::Parse {
                        path: format!("{path:?}"),
                        source: e,
                    })?;
                scenarios.push(scenario);
            }
        }

        Ok(Self { scenarios })
    }
}
