#![forbid(unsafe_code)]

//! Cross-Spirit isolation corpus loader — NFR-Sec-14 200-scenario
//! adversarial corpus split per ADR-040 into Sec-14a (100 same-Host)
//! + Sec-14b (100 cross-Host), eight categories ≥25 scenarios per
//! category aggregated.
//!
//! Loader pattern mirrors `halt_corpus.rs::HaltCorpus::load_from` and
//! `distillate_corpus.rs::DistillateCorpus::load_from`.

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::CorpusError;

/// Eight attack categories for cross-Spirit isolation testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationAttackCategory {
    NamespaceEnumeration,
    WorkingMemoryReadAcross,
    DecisionFrameObservation,
    HaltSignalObservation,
    TransparencyLogCrossRead,
    WorkingMemoryDigestCrossRead,
    CapabilityTokenForgeryCrossSpirit,
    SandboxEscapeLateral,
}

impl IsolationAttackCategory {
    pub fn all() -> &'static [IsolationAttackCategory] {
        use IsolationAttackCategory::*;
        &[
            NamespaceEnumeration,
            WorkingMemoryReadAcross,
            DecisionFrameObservation,
            HaltSignalObservation,
            TransparencyLogCrossRead,
            WorkingMemoryDigestCrossRead,
            CapabilityTokenForgeryCrossSpirit,
            SandboxEscapeLateral,
        ]
    }
}

/// Per-category attestation — mirrors Story 4.4 iaa-attestation pattern.
#[derive(Debug, Clone, Deserialize)]
pub struct CategoryAttestation {
    pub category: String,
    pub scenario_count: usize,
    pub split: String,
    pub threat_model_reference: String,
    pub authoring_method: String,
    pub reviewer_attestation: ReviewerAttestation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewerAttestation {
    pub attestor_id: String,
    pub attestor_role: String,
    pub attestation_date: String,
    pub attestation_statement: String,
}

/// Root methodology attestation — Epic 2 retro A2 closure.
#[derive(Debug, Clone, Deserialize)]
pub struct MethodologyAttestation {
    pub corpus_version: String,
    pub corpus_tag: String,
    pub total_scenarios: usize,
    pub sec_14a_count: usize,
    pub sec_14b_count: usize,
    pub category_floor_per_split: usize,
    pub authoring_methodology: String,
    pub rationale: String,
    pub scripted_generator_path: String,
    pub generator_seed: u64,
    pub v1_0_promotion_plan: String,
}

/// Expected outcome for a single isolation scenario.
#[derive(Debug, Clone, Deserialize)]
pub struct ExpectedOutcome {
    pub isolation_maintained: bool,
    pub expected_kernel_response: String,
    #[serde(default)]
    pub leak_signal_must_be_absent: Vec<String>,
}

/// Per-scenario preconditions.
#[derive(Debug, Clone, Deserialize)]
pub struct Preconditions {
    pub spirit_a_pid: u32,
    pub spirit_b_pid: u32,
    pub spirit_a_principal_id: String,
    pub spirit_b_principal_id: String,
    #[serde(default)]
    pub seed_data: Vec<SeedDataEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedDataEntry {
    pub namespace: String,
    pub key: String,
    pub value: String,
}

/// A single cross-Spirit isolation test scenario.
#[derive(Debug, Clone, Deserialize)]
pub struct IsolationCorpusScenario {
    pub scenario_id: String,
    pub tier_tag: String,
    pub split: String,
    pub category: String,
    #[serde(default)]
    pub spirit_a_role: String,
    #[serde(default)]
    pub spirit_b_role: String,
    #[serde(default)]
    pub attack_surface: String,
    #[serde(default)]
    pub attack_payload: serde_json::Value,
    pub expected_outcome: ExpectedOutcome,
    pub preconditions: Preconditions,
    /// Optional — only present for halt-signal-observation scenarios (AC3).
    #[serde(default)]
    pub expected_swap_verdict: Option<ExpectedSwapVerdict>,
    /// Optional — tier target for deferred scenarios (T3 → Story 5.5a).
    #[serde(default)]
    pub tier_target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpectedSwapVerdict {
    pub variant: String,
}

/// Container for the full cross-Spirit isolation corpus.
#[derive(Debug, Clone)]
pub struct IsolationCorpus {
    pub scenarios: Vec<IsolationCorpusScenario>,
    pub methodology: MethodologyAttestation,
    pub per_category_attestations: Vec<CategoryAttestation>,
}

impl IsolationCorpus {
    /// Total number of scenarios.
    pub fn total(&self) -> usize {
        self.scenarios.len()
    }

    /// Count scenarios in a given split ("sec-14a" or "sec-14b").
    pub fn count_split(&self, split: &str) -> usize {
        self.scenarios.iter().filter(|s| s.split == split).count()
    }

    /// Count scenarios for a given category across both splits.
    pub fn scenarios_per_category(&self, category: IsolationAttackCategory) -> usize {
        let cat_str = serde_variant::to_snake_case(&category);
        self.scenarios
            .iter()
            .filter(|s| s.category == cat_str)
            .count()
    }
}

/// Loader helpers — snake_case conversion without extra dependency.
pub mod serde_variant {
    use super::IsolationAttackCategory;

    pub fn to_snake_case(cat: &IsolationAttackCategory) -> &'static str {
        match cat {
            IsolationAttackCategory::NamespaceEnumeration => "namespace_enumeration",
            IsolationAttackCategory::WorkingMemoryReadAcross => "working_memory_read_across",
            IsolationAttackCategory::DecisionFrameObservation => "decision_frame_observation",
            IsolationAttackCategory::HaltSignalObservation => "halt_signal_observation",
            IsolationAttackCategory::TransparencyLogCrossRead => "transparency_log_cross_read",
            IsolationAttackCategory::WorkingMemoryDigestCrossRead => {
                "working_memory_digest_cross_read"
            }
            IsolationAttackCategory::CapabilityTokenForgeryCrossSpirit => {
                "capability_token_forgery_cross_spirit"
            }
            IsolationAttackCategory::SandboxEscapeLateral => "sandbox_escape_lateral",
        }
    }
}

impl IsolationCorpus {
    /// Load the corpus from the standard directory layout:
    ///
    /// ```text
    /// dir/
    /// ├── methodology-attestation.json
    /// ├── sec-14a/
    /// │   ├── <category>/
    /// │   │   ├── category-attestation.json
    /// │   │   └── scenario-*.json
    /// │   └── ...
    /// └── sec-14b/
    ///     └── ...
    /// ```
    pub fn load_from(dir: &Path) -> Result<Self, CorpusError> {
        if !dir.is_dir() {
            return Err(CorpusError::NotFound(dir.display().to_string()));
        }

        // Load root methodology attestation
        let methodology_path = dir.join("methodology-attestation.json");
        let methodology_bytes =
            std::fs::read_to_string(&methodology_path).map_err(|e| CorpusError::Parse {
                path: methodology_path.display().to_string(),
                source: serde_json::Error::io(e),
            })?;
        let methodology: MethodologyAttestation = serde_json::from_str(&methodology_bytes)
            .map_err(|e| CorpusError::Parse {
                path: methodology_path.display().to_string(),
                source: e,
            })?;

        // The methodology declares expected counts; we validate against actual loaded counts
        // later. For the real v0.3-β corpus, methodology.total_scenarios is 200.

        let mut scenarios: Vec<IsolationCorpusScenario> = Vec::with_capacity(200);
        let mut per_category_attestations: Vec<CategoryAttestation> = Vec::with_capacity(16);

        for split in &["sec-14a", "sec-14b"] {
            let split_dir = dir.join(split);
            if !split_dir.is_dir() {
                return Err(CorpusError::NotFound(split_dir.display().to_string()));
            }

            let entries: Vec<_> = std::fs::read_dir(&split_dir)
                .map_err(|e| CorpusError::Io(e))?
                .filter_map(|e| match e {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        eprintln!(
                            "isolation-corpus loader: read_dir entry error in {}: {err}",
                            split_dir.display()
                        );
                        None
                    }
                })
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();

            for category_entry in &entries {
                let cat_dir = category_entry.path();
                let cat_name = category_entry.file_name().to_string_lossy().to_string();

                // Load category-attestation.json
                let attestation_path = cat_dir.join("category-attestation.json");
                let att_bytes =
                    std::fs::read_to_string(&attestation_path).map_err(|e| CorpusError::Parse {
                        path: attestation_path.display().to_string(),
                        source: serde_json::Error::io(e),
                    })?;
                let cat_attestation: CategoryAttestation = serde_json::from_str(&att_bytes)
                    .map_err(|e| CorpusError::Parse {
                        path: attestation_path.display().to_string(),
                        source: e,
                    })?;

                // Load scenario JSONs
                let mut scenario_files: Vec<_> = std::fs::read_dir(&cat_dir)
                    .map_err(|e| CorpusError::Io(e))?
                    .filter_map(|e| match e {
                        Ok(entry) => Some(entry),
                        Err(err) => {
                            eprintln!(
                                "isolation-corpus loader: read_dir entry error in {}: {err}",
                                cat_dir.display()
                            );
                            None
                        }
                    })
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.starts_with("scenario-") && n.ends_with(".json"))
                            .unwrap_or(false)
                    })
                    .collect();
                scenario_files.sort_by_key(|e| e.file_name());

                // Validate scenario_count matches on-disk count
                if cat_attestation.scenario_count != scenario_files.len() {
                    return Err(CorpusError::Parse {
                        path: attestation_path.display().to_string(),
                        source: serde_json::Error::custom(format!(
                            "category-attestation scenario_count {} != on-disk count {} for {}/{}",
                            cat_attestation.scenario_count,
                            scenario_files.len(),
                            split,
                            cat_name
                        )),
                    });
                }

                for sf in &scenario_files {
                    let sf_path = sf.path();
                    let content =
                        std::fs::read_to_string(&sf_path).map_err(|e| CorpusError::Parse {
                            path: sf_path.display().to_string(),
                            source: serde_json::Error::io(e),
                        })?;
                    let scenario: IsolationCorpusScenario = serde_json::from_str(&content)
                        .map_err(|e| CorpusError::Parse {
                            path: sf_path.display().to_string(),
                            source: e,
                        })?;

                    // Validate scenario_id matches file path (category directory + filename)
                    let expected_prefix = format!("{}/{}/", split, cat_name);
                    let expected_id = format!(
                        "{}{}",
                        expected_prefix,
                        sf.file_name()
                            .to_string_lossy()
                            .strip_suffix(".json")
                            .unwrap_or("")
                    );
                    if scenario.scenario_id != expected_id {
                        return Err(CorpusError::Parse {
                            path: sf_path.display().to_string(),
                            source: serde_json::Error::custom(format!(
                                "scenario_id '{}' does not match expected '{}'",
                                scenario.scenario_id, expected_id
                            )),
                        });
                    }

                    // v0.3-β: no known-vulnerable scenarios allowed
                    if !scenario.expected_outcome.isolation_maintained {
                        return Err(CorpusError::Parse {
                            path: sf_path.display().to_string(),
                            source: serde_json::Error::custom(
                                "isolation_maintained must be true at v0.3-β",
                            ),
                        });
                    }

                    scenarios.push(scenario);
                }

                per_category_attestations.push(cat_attestation);
            }
        }

        // Validate split counts against methodology declaration
        let sec_14a_count = scenarios.iter().filter(|s| s.split == "sec-14a").count();
        let sec_14b_count = scenarios.iter().filter(|s| s.split == "sec-14b").count();
        if sec_14a_count != methodology.sec_14a_count {
            return Err(CorpusError::Parse {
                path: methodology_path.display().to_string(),
                source: serde_json::Error::custom(format!(
                    "sec_14a_count mismatch: methodology says {}, loaded {}",
                    methodology.sec_14a_count, sec_14a_count
                )),
            });
        }
        if sec_14b_count != methodology.sec_14b_count {
            return Err(CorpusError::Parse {
                path: methodology_path.display().to_string(),
                source: serde_json::Error::custom(format!(
                    "sec_14b_count mismatch: methodology says {}, loaded {}",
                    methodology.sec_14b_count, sec_14b_count
                )),
            });
        }
        let total = sec_14a_count + sec_14b_count;
        if total != methodology.total_scenarios {
            return Err(CorpusError::Parse {
                path: methodology_path.display().to_string(),
                source: serde_json::Error::custom(format!(
                    "total_scenarios mismatch: methodology says {}, loaded {}",
                    methodology.total_scenarios, total
                )),
            });
        }

        Ok(Self {
            scenarios,
            methodology,
            per_category_attestations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_minimal_scenario_json(
        scenario_id: &str,
        split: &str,
        category: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "scenario_id": scenario_id,
            "tier_tag": "scripted-v0",
            "split": split,
            "category": category,
            "attack_payload": {},
            "expected_outcome": {
                "isolation_maintained": true,
                "expected_kernel_response": "ScopeViolation",
                "leak_signal_must_be_absent": []
            },
            "preconditions": {
                "spirit_a_pid": 100,
                "spirit_b_pid": 200,
                "spirit_a_principal_id": "a@test",
                "spirit_b_principal_id": "b@test",
                "seed_data": []
            }
        })
    }

    fn make_category_attestation_json(
        category: &str,
        count: usize,
        split: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "category": category,
            "scenario_count": count,
            "split": split,
            "threat_model_reference": "arch/8.md#8.1",
            "authoring_method": "scripted",
            "reviewer_attestation": {
                "attestor_id": "test",
                "attestor_role": "Tester",
                "attestation_date": "2026-05-20",
                "attestation_statement": "test attestation"
            }
        })
    }

    fn make_methodology_json(sec_14a: usize, sec_14b: usize) -> serde_json::Value {
        serde_json::json!({
            "corpus_version": "v0",
            "corpus_tag": "scripted-v0",
            "total_scenarios": (sec_14a + sec_14b),
            "sec_14a_count": sec_14a,
            "sec_14b_count": sec_14b,
            "category_floor_per_split": 1,
            "authoring_methodology": "scripted",
            "rationale": "test",
            "scripted_generator_path": "xtask/src/gen_isolation_corpus.rs",
            "generator_seed": 0,
            "v1_0_promotion_plan": "test"
        })
    }

    fn make_temp_corpus(scenarios_per_cat: usize) -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let categories = [
            "namespace_enumeration",
            "working_memory_read_across",
            "decision_frame_observation",
            "halt_signal_observation",
            "transparency_log_cross_read",
            "working_memory_digest_cross_read",
            "capability_token_forgery_cross_spirit",
            "sandbox_escape_lateral",
        ];

        let mut total_14a = 0;
        let mut total_14b = 0;

        for split in &["sec-14a", "sec-14b"] {
            let split_dir = root.join(split);
            std::fs::create_dir_all(&split_dir).unwrap();

            for cat in &categories {
                let cat_dir = split_dir.join(cat);
                std::fs::create_dir_all(&cat_dir).unwrap();

                for i in 1..=scenarios_per_cat {
                    let scenario_id = format!("{}/{}/scenario-{:03}", split, cat, i);
                    let json = make_minimal_scenario_json(&scenario_id, split, cat);
                    let path = cat_dir.join(format!("scenario-{:03}.json", i));
                    let mut f = std::fs::File::create(&path).unwrap();
                    write!(f, "{}", serde_json::to_string_pretty(&json).unwrap()).unwrap();
                }

                let att = make_category_attestation_json(cat, scenarios_per_cat, split);
                let att_path = cat_dir.join("category-attestation.json");
                let mut f = std::fs::File::create(&att_path).unwrap();
                write!(f, "{}", serde_json::to_string_pretty(&att).unwrap()).unwrap();

                if *split == "sec-14a" {
                    total_14a += scenarios_per_cat;
                } else {
                    total_14b += scenarios_per_cat;
                }
            }
        }

        let methodology = make_methodology_json(total_14a, total_14b);
        let meth_path = root.join("methodology-attestation.json");
        let mut f = std::fs::File::create(&meth_path).unwrap();
        write!(f, "{}", serde_json::to_string_pretty(&methodology).unwrap()).unwrap();

        (tmp, root)
    }

    #[test]
    fn load_minimal_8_scenario_corpus() {
        let (_tmp, root) = make_temp_corpus(1);
        let corpus = IsolationCorpus::load_from(&root).expect("load minimal corpus");
        assert_eq!(corpus.total(), 16); // 8 categories × 2 splits × 1 = 16
        assert_eq!(corpus.count_split("sec-14a"), 8);
        assert_eq!(corpus.count_split("sec-14b"), 8);
        assert_eq!(
            corpus.scenarios_per_category(IsolationAttackCategory::NamespaceEnumeration),
            2
        );
    }

    #[test]
    fn rejects_scenario_id_path_mismatch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let sec14a = root.join("sec-14a");
        let cat_dir = sec14a.join("namespace_enumeration");
        std::fs::create_dir_all(&cat_dir).unwrap();

        let json = make_minimal_scenario_json("wrong/id/path", "sec-14a", "namespace_enumeration");
        let mut f = std::fs::File::create(cat_dir.join("scenario-001.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&json).unwrap()).unwrap();

        let att = make_category_attestation_json("namespace_enumeration", 1, "sec-14a");
        let mut f = std::fs::File::create(cat_dir.join("category-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&att).unwrap()).unwrap();

        let methodology = make_methodology_json(1, 0);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        // Need sec-14b dir too (loader iterates both splits, sec-14b can be empty)
        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject scenario_id mismatch");
    }

    #[test]
    fn rejects_category_attestation_count_mismatch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let sec14a = root.join("sec-14a");
        let cat_dir = sec14a.join("namespace_enumeration");
        std::fs::create_dir_all(&cat_dir).unwrap();

        let json = make_minimal_scenario_json(
            "sec-14a/namespace_enumeration/scenario-001",
            "sec-14a",
            "namespace_enumeration",
        );
        let mut f = std::fs::File::create(cat_dir.join("scenario-001.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&json).unwrap()).unwrap();

        // attestation claims 5 but only 1 on disk
        let att = make_category_attestation_json("namespace_enumeration", 5, "sec-14a");
        let mut f = std::fs::File::create(cat_dir.join("category-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&att).unwrap()).unwrap();

        let methodology = make_methodology_json(1, 0);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject count mismatch");
    }

    #[test]
    fn rejects_methodology_total_mismatch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("sec-14a")).unwrap();
        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        // methodology claims 200 but no scenarios at all
        let methodology = make_methodology_json(100, 100);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject total mismatch");
    }

    #[test]
    fn rejects_isolation_maintained_false() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let sec14a = root.join("sec-14a");
        let cat_dir = sec14a.join("namespace_enumeration");
        std::fs::create_dir_all(&cat_dir).unwrap();

        let mut json = make_minimal_scenario_json(
            "sec-14a/namespace_enumeration/scenario-001",
            "sec-14a",
            "namespace_enumeration",
        );
        json["expected_outcome"]["isolation_maintained"] = serde_json::json!(false);
        let mut f = std::fs::File::create(cat_dir.join("scenario-001.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&json).unwrap()).unwrap();

        let att = make_category_attestation_json("namespace_enumeration", 1, "sec-14a");
        let mut f = std::fs::File::create(cat_dir.join("category-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&att).unwrap()).unwrap();

        let methodology = make_methodology_json(1, 0);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject isolation_maintained: false");
    }

    #[test]
    fn rejects_malformed_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let sec14a = root.join("sec-14a");
        let cat_dir = sec14a.join("namespace_enumeration");
        std::fs::create_dir_all(&cat_dir).unwrap();

        let mut f = std::fs::File::create(cat_dir.join("scenario-001.json")).unwrap();
        write!(f, "{{broken json").unwrap();

        let att = make_category_attestation_json("namespace_enumeration", 1, "sec-14a");
        let mut f = std::fs::File::create(cat_dir.join("category-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&att).unwrap()).unwrap();

        let methodology = make_methodology_json(1, 0);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject malformed JSON");
    }

    #[test]
    fn rejects_missing_category_attestation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let sec14a = root.join("sec-14a");
        let cat_dir = sec14a.join("namespace_enumeration");
        std::fs::create_dir_all(&cat_dir).unwrap();

        let json = make_minimal_scenario_json(
            "sec-14a/namespace_enumeration/scenario-001",
            "sec-14a",
            "namespace_enumeration",
        );
        let mut f = std::fs::File::create(cat_dir.join("scenario-001.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&json).unwrap()).unwrap();

        // No category-attestation.json

        let methodology = make_methodology_json(1, 0);
        let mut f = std::fs::File::create(root.join("methodology-attestation.json")).unwrap();
        write!(f, "{}", serde_json::to_string(&methodology).unwrap()).unwrap();

        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        let result = IsolationCorpus::load_from(&root);
        assert!(
            result.is_err(),
            "should reject missing category attestation"
        );
    }

    #[test]
    fn rejects_missing_methodology_attestation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("sec-14a")).unwrap();
        std::fs::create_dir_all(root.join("sec-14b")).unwrap();

        // No methodology-attestation.json
        let result = IsolationCorpus::load_from(&root);
        assert!(result.is_err(), "should reject missing methodology");
    }

    #[test]
    fn rejects_nonexistent_directory() {
        let result = IsolationCorpus::load_from(Path::new("/nonexistent/corpus/path"));
        assert!(result.is_err());
    }
}
