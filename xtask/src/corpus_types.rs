use std::collections::BTreeMap;
use std::path::Path;

/// Shared corpus manifest types used by check-corpus, coverage-matrix,
/// corpus-staleness, and rebaseline-check.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CorpusManifest {
    pub corpus: BTreeMap<String, CorpusEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CorpusEntry {
    pub sha256: String,
    pub schema_version: u32,
    pub item_count: usize,
    pub valid_until: String,
    pub prompt_version_hash: String,
    pub description: String,
    pub judge_id: Option<String>,
}

/// Shared coverage-matrix types used by coverage-matrix and corpus-staleness.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CoverageMatrixFile {
    pub schema_version: u32,
    pub current_phase: String,
    pub mode: String,
    pub phase_order: Vec<String>,
    pub coverage: BTreeMap<String, CoverageRow>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CoverageRow {
    pub gates: Vec<String>,
    pub corpora: Vec<String>,
    pub phase: String,
    pub valid_until: String,
    pub notes: Option<String>,
}

/// Phase configuration TOML shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PhaseConfig {
    pub current_phase: String,
    pub phase_order: Vec<String>,
}

/// Judge configuration TOML shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct JudgeConfig {
    pub judge: BTreeMap<String, JudgeEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct JudgeEntry {
    pub model: String,
    pub temperature: f64,
    pub top_p: f64,
    #[serde(default)]
    pub seed: Option<u64>,
    pub retry_budget: u32,
    pub prompt_version_hash: String,
    pub added_in_story: String,
}

/// Gate registry TOML shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GateRegistry {
    pub gates: Vec<String>,
}

/// D3/F3→B: per-gate phase disposition. A gate's verdict blocks ship only at/after
/// the phase where its disposition becomes "blocking". The map is keyed by phase
/// identifier (e.g. "v1.0", "v1.5"); absent phases inherit the nearest prior phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ShipGateEntry {
    pub name: String,
    pub disposition: std::collections::HashMap<String, String>,
}

/// Extended registry shape: the flat `gates` list (for coverage-matrix validation)
/// plus the structured `[[ship_gate]]` entries (for phase-graduation enforcement).
/// `ship_gates` is optional for backward compat with registries that haven't migrated.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ShipGateRegistry {
    pub gates: Vec<String>,
    #[serde(default, rename = "ship_gate")]
    pub ship_gates: Vec<ShipGateEntry>,
}

/// Judge direct-call identifiers TOML shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct JudgeDirectCallIdentifiers {
    pub direct_calls: Vec<String>,
}

pub fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))
}

pub fn round_ratio(r: f64) -> f64 {
    (r * 10000.0).round() / 10000.0
}
