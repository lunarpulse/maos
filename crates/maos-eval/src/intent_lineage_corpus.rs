#![forbid(unsafe_code)]

//! Story 6.2 AC4 — Intent lineage corpus (NFR-Aud-14 100% coverage).
//!
//! Loads scenarios from `fixtures/intent-lineage-corpus-v0/`. The corpus has
//! four scenario classes mirroring the spec:
//!
//! | Class | Count | Asserts |
//! |---|---|---|
//! | `lineage_chain_uninterrupted` | 15 | single-hop `HumanAuthored` → `SpiritAuto` re-emission carries originating intent |
//! | `lineage_union_via_distillate` | 15 | distillate's `intent_lineage` = UNION of source frames |
//! | `lineage_broken_spirit_auto_strips_field` | 10 | adversarial: empty lineage on SpiritAuto cross-Spirit → REJECTED |
//! | `lineage_continuity_across_retract` | 10 | Retract frame copies original lineage |
//!
//! Mirror of `retract_corpus.rs` pattern for consistency with Story 6.1.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentLineageClass {
    LineageChainUninterrupted,
    LineageUnionViaDistillate,
    LineageBrokenSpiritAutoStripsField,
    LineageContinuityAcrossRetract,
    LineageViaGatewayInbound,
    LineageViaGatewayOutbound,
}

/// One scenario in the intent lineage corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentLineageScenario {
    pub scenario_id: String,
    pub class: IntentLineageClass,
    pub description: String,
    /// The originating principal intent label (e.g. "consult", "delegate").
    pub originating_intent: String,
    /// Number of cross-Spirit hops in the scenario.
    pub hop_count: u32,
    /// `auto_marker` for the emitted re-emission frame.
    pub origin: String,
    /// Optional second intent (for `LineageUnionViaDistillate`).
    #[serde(default)]
    pub secondary_intent: Option<String>,
    /// Expected outcome for the corpus runner.
    pub expected_outcome: IntentLineageExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentLineageExpectedOutcome {
    /// `true` if the scenario asserts the kernel accepts the emission;
    /// `false` if the kernel must reject with `EIntentLineageBroken`.
    pub accepted: bool,
    /// Expected non-empty lineage on the emitted frame (or on the retract
    /// continuation). When `accepted == false` this is the lineage the
    /// adversarial frame ATTEMPTED to carry — typically empty.
    pub expected_lineage_intents: Vec<String>,
}

pub struct IntentLineageCorpus {
    pub scenarios: Vec<IntentLineageScenario>,
}

impl IntentLineageCorpus {
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
            if !path.file_stem().map_or(false, |s| {
                s.to_str().map_or(false, |s| s.starts_with("scenario-"))
            }) {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let scenario: IntentLineageScenario =
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

    /// Per-class breakdown count.
    pub fn count_by_class(&self) -> [(IntentLineageClass, usize); 4] {
        let mut counts = [
            (IntentLineageClass::LineageChainUninterrupted, 0),
            (IntentLineageClass::LineageUnionViaDistillate, 0),
            (IntentLineageClass::LineageBrokenSpiritAutoStripsField, 0),
            (IntentLineageClass::LineageContinuityAcrossRetract, 0),
        ];
        for s in &self.scenarios {
            for entry in &mut counts {
                if entry.0 == s.class {
                    entry.1 += 1;
                }
            }
        }
        counts
    }

    /// NFR-Aud-14 coverage gate.
    ///
    /// Returns `(passed, total_cross_spirit_frames, non_empty_lineage_frames)`.
    /// Coverage = non_empty / total × 100%. Floor: 100%.
    pub fn coverage_for_accepted_scenarios(&self) -> (bool, usize, usize) {
        let mut total = 0;
        let mut non_empty = 0;
        for s in &self.scenarios {
            if !s.expected_outcome.accepted {
                continue;
            }
            total += 1;
            if !s.expected_outcome.expected_lineage_intents.is_empty() {
                non_empty += 1;
            }
        }
        (non_empty == total && total > 0, total, non_empty)
    }
}
