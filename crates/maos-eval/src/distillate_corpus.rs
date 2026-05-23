#![forbid(unsafe_code)]

//! Distillate corpus loader — N=100 synthetic-v0 scenarios for the
//! five-metric distillation gate (NFR-Aud-7).
//!
//! Loader pattern mirrors `halt_corpus.rs::HaltCorpus::load_from`.

use serde::Deserialize;
use std::path::Path;

use crate::CorpusError;

/// Container for the full distillation corpus.
#[derive(Debug, Clone)]
pub struct DistillateCorpus {
    pub scenarios: Vec<DistillateScenario>,
    pub iaa_attestation: IaaAttestation,
}

/// A single synthetic distillation scenario.
#[derive(Debug, Clone, Deserialize)]
pub struct DistillateScenario {
    pub scenario_id: String,
    /// Corpus tier marker — "synthetic-v0" for the v0.3-β slice.
    pub tag: String,
    /// Informational — the Spirit class this scenario mimics.
    pub spirit_class: String,
    /// Synthesized raw frames the corpus author specified.
    pub source_raw_frames: Vec<RawFrameStub>,
    /// The digest under test.
    pub digest_payload: String,
    /// Hex-encoded frame_ids referencing source_raw_frames.
    pub source_log_ref: Vec<String>,
    /// Self-declared distillation depth.
    pub distillation_depth: u32,
    /// Expected intent lineage.
    pub intent_lineage_expected: Vec<String>,
    /// Expected recall annotation.
    pub expected_recall: f64,
    /// Expected faithfulness annotation.
    pub expected_faithfulness: f64,
    /// Expected hedge-preservation annotation.
    pub expected_hedge_preservation: f64,
    /// Any literal secret tokens the digest is forbidden to contain.
    #[serde(default)]
    pub planted_secrets: Vec<String>,
}

/// A stub raw frame for corpus author reference.
#[derive(Debug, Clone, Deserialize)]
pub struct RawFrameStub {
    /// 32-char hex frame_id.
    pub frame_id_hex: String,
    /// Matches A2AIntent::new.
    pub intent: String,
    /// For corpus authors' reference only — not evaluated.
    pub payload_summary: String,
}

/// Inter-annotator agreement attestation.
#[derive(Debug, Clone, Deserialize)]
pub struct IaaAttestation {
    pub corpus_version: String,
    pub annotator_count: u32,
    pub hedge_cohen_kappa: f64,
    pub computed_at: String,
}

impl DistillateCorpus {
    /// Load the corpus from a directory.
    /// Scans for `scenario-*.json` files and `iaa-attestation.json`.
    pub fn load_from(dir: &Path) -> Result<Self, CorpusError> {
        if !dir.is_dir() {
            return Err(CorpusError::NotFound(dir.display().to_string()));
        }

        // Load IAA attestation
        let iaa_path = dir.join("iaa-attestation.json");
        let iaa_bytes = std::fs::read_to_string(&iaa_path).map_err(|e| CorpusError::Parse {
            path: iaa_path.display().to_string(),
            source: serde_json::Error::io(e),
        })?;
        let iaa_attestation: IaaAttestation =
            serde_json::from_str(&iaa_bytes).map_err(|e| CorpusError::Parse {
                path: iaa_path.display().to_string(),
                source: e,
            })?;

        // Load scenarios
        let mut scenario_files: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| CorpusError::Io(e))?
            .filter_map(|entry| entry.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with("scenario-") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();

        scenario_files.sort_by_key(|e| e.file_name());

        let mut scenarios = Vec::with_capacity(scenario_files.len());
        for entry in &scenario_files {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                CorpusError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("non-UTF-8 filename in corpus directory: {path:?}"),
                ))
            })?;
            let content = std::fs::read_to_string(&path).map_err(|e| CorpusError::Parse {
                path: path.display().to_string(),
                source: serde_json::Error::io(e),
            })?;
            let scenario: DistillateScenario =
                serde_json::from_str(&content).map_err(|e| CorpusError::Parse {
                    path: path.display().to_string(),
                    source: e,
                })?;
            scenarios.push(scenario);
        }

        Ok(Self {
            scenarios,
            iaa_attestation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_synthetic_v0_corpus() {
        let dir = std::path::Path::new("fixtures/distillate-corpus-v0");
        if !dir.exists() {
            // Skip if corpus directory not yet created
            return;
        }
        let corpus = DistillateCorpus::load_from(dir).expect("load corpus");
        assert!(!corpus.scenarios.is_empty());
        assert!(corpus.iaa_attestation.hedge_cohen_kappa >= 0.85);
    }
}
