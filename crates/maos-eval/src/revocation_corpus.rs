#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One revocation scenario from the revocation corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationScenario {
    pub scenario_id: String,
    pub category: String,
    pub crl_blob_path: String,
    pub trust_anchor_pub_path: String,
    pub expected_outcome: RevocationExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationExpectedOutcome {
    pub accepted: bool,
    pub propagation_latency_ms: Option<u64>,
    pub revoked_spirit_count: usize,
    pub error_variant: Option<String>,
}

pub struct RevocationCorpus {
    pub scenarios: Vec<RevocationScenario>,
    root: std::path::PathBuf,
}

impl RevocationCorpus {
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
            if path.extension().is_none_or(|ext| ext != "json")
                || path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with(".signed.json"))
                || path
                    .file_name()
                    .is_some_and(|name| name == "methodology-attestation.json")
            {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let scenario: RevocationScenario =
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

    /// Materialize every descriptor dependency before a kernel integration
    /// runner attempts parsing/signature verification. Metadata-only corpus
    /// checks are insufficient: a missing CRL or anchor used to pass.
    pub fn read_assets(
        &self,
    ) -> Result<Vec<(RevocationScenario, Vec<u8>, Vec<u8>)>, crate::CorpusError> {
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
                    std::fs::read(resolved)
                };
                let crl = asset(&scenario.crl_blob_path)?;
                let anchor = asset(&scenario.trust_anchor_pub_path)?;
                Ok((scenario, crl, anchor))
            })
            .collect()
    }
}
