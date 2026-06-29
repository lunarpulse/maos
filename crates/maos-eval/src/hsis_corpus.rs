#![forbid(unsafe_code)]

//! HSIS corpus loader (Story 5.2, AC5).
//!
//! Loads the 300-scenario Hot-Swap Invariant Strength corpus from
//! `crates/maos-eval/fixtures/hsis-corpus-v0/` with per-class accessors
//! and category attestation validation.

use crate::CorpusError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// An HSIS corpus scenario.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisScenario {
    pub scenario_id: String,
    pub tier_tag: String,
    pub spirit_class: String,
    pub swap_kind: SwapKind,
    pub predecessor: HsisPredecessor,
    pub successor: HsisSuccessor,
    pub preconditions: HsisPreconditions,
    pub expected_outcome: HsisExpectedOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwapKind {
    SameMajor,
    CrossMajor,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisPredecessor {
    pub spirit_class: String,
    pub version: String,
    pub state_schema_version: u32,
    pub halt_protocol_version: u32,
    pub pending_halts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisSuccessor {
    pub spirit_class: String,
    pub version: String,
    pub state_schema_version: u32,
    pub halt_protocol_compatibility: Vec<u32>,
    #[serde(default)]
    pub migrates_from: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisPreconditions {
    pub spirit_pid: u32,
    pub swap_invariants: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisExpectedOutcome {
    pub verdict: String,
    #[serde(default)]
    pub post_swap_invariants_held: Option<bool>,
    #[serde(default)]
    pub auto_revert_fired: Option<bool>,
    pub expected_error: Option<String>,
}

/// HSIS attack categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsisAttackCategory {
    SameMajorSwap,
    CrossMajorMigration,
    HaltContinuity,
    OutputShapeRegression,
    TokenRebinding,
    AutoRevertWindow,
}

/// Category attestation per Spirit class.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisCategoryAttestation {
    pub scenario_count: usize,
    pub authoring_method: String,
    pub reviewer_attestation: String,
    pub threat_model_reference: String,
}

/// Methodology attestation for the whole corpus.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HsisMethodologyAttestation {
    pub corpus_id: String,
    pub version: String,
    pub scenario_count: usize,
    pub class_list: Vec<String>,
    pub per_class_count: usize,
    pub pass_threshold: String,
    pub categories: Vec<String>,
}

/// A loaded HSIS corpus with per-class access.
pub struct HsisCorpus {
    pub scenarios: Vec<HsisScenario>,
}

impl HsisCorpus {
    /// Load all scenarios from the corpus directory structure.
    pub fn load(corpus_path: impl AsRef<Path>) -> Result<Self, CorpusError> {
        let base = corpus_path.as_ref();
        let mut scenarios = Vec::new();

        for class in &[
            "butler",
            "researcher",
            "observer",
            "orchestrator",
            "worker",
            "cliwrapper",
        ] {
            let class_dir = base.join(class);
            if !class_dir.is_dir() {
                return Err(CorpusError::NotFound(format!(
                    "HSIS class directory missing: {class} (expected at {})",
                    class_dir.display()
                )));
            }
            let mut entries: Vec<_> =
                std::fs::read_dir(&class_dir)?.collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    let content = std::fs::read_to_string(&path)?;
                    let scenario: HsisScenario =
                        serde_json::from_str(&content).map_err(|e| CorpusError::Parse {
                            path: format!("{path:?}"),
                            source: e,
                        })?;
                    scenarios.push(scenario);
                }
            }
        }

        Ok(Self { scenarios })
    }

    /// Get scenarios for a specific Spirit class.
    pub fn scenarios_for_class(&self, class: &str) -> Vec<&HsisScenario> {
        self.scenarios
            .iter()
            .filter(|s| s.spirit_class == class)
            .collect()
    }
}
