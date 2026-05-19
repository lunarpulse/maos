#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One termination scenario from the 1000-termination corpus.
///
/// The fixture format is JSON-per-file under
/// `crates/maos-eval/fixtures/termination-corpus-v0/`; each file
/// is a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationScenario {
    pub scenario_id: String,
    pub kind: TerminationKind,
    pub spirit_id: String,
    pub pending_halts: Vec<String>,
    pub expected_receipts: usize,
    pub expected_receipt_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationKind {
    /// Director-initiated unload (FR51-class)
    PlannedUnload,
    /// `accepted_halt` resolution
    HaltAccepted,
    /// SIGKILL / process death (Story 5.3's domain; scaffolded here for
    /// the receipt-rate measurement)
    UnplannedCrash,
    /// `[epistemic_policy]` rejected the halt (predicate fired but
    /// policy declared `verbalize_only`); receipt still produced
    HaltRejection,
}

pub struct TerminationCorpus {
    pub scenarios: Vec<TerminationScenario>,
}

impl TerminationCorpus {
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
            let scenario: TerminationScenario = serde_json::from_str(&content).map_err(|e| {
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
