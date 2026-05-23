//! Adversarial-Spirit red-team corpus generator.
//!
//! Produces ≥640 deduplicated adversarial scenario descriptions from 80
//! canonical seed scenarios across 8 §8.1 attack classes, expanded 8× via
//! deterministic parameter variation.
//!
//! Consumed by NFR-Sec-10 ship-gate (lands v1.5).

use std::collections::BTreeMap;
use std::path::Path;

use crate::{ClassCoverage, CorpusGenerator, CoverageReport, ValidationOutcome};

pub mod expansion;
pub mod seeds;
pub mod validation;

// ---------------------------------------------------------------------------
// Compile-time pinned constants
// ---------------------------------------------------------------------------

/// SHA-256 hex digest of `seeds/red-team-seeds-v0.1.toml`.
pub const SEED_FILE_SHA256: &str =
    "f4a5988b2c622686e78c4c698ff0af575c766bbfa77f505d94b62d41fa742f2e";

/// Expansion-rule version.
pub const RULE_VERSION: &str = "v0.1";

// ---------------------------------------------------------------------------
// Binding types
// ---------------------------------------------------------------------------

/// A single canonical red-team attack scenario.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedTeamSeed {
    pub id: String,
    pub class: String,
    pub attack_summary: String,
    pub kernel_defense_mechanism: String,
    pub expected_detection_surface: String,
    pub parameter_axes: Vec<String>,
    pub canonical_assertion: String,
}

/// A single expanded red-team corpus item.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedTeamItem {
    pub id: String,
    pub class: String,
    pub scenario_description: String,
    pub parameters: BTreeMap<String, String>,
    pub expected_kernel_response: String,
    pub expected_audit_signal: String,
    pub seed_id: String,
    pub canonical_assertion: String,
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// The red-team generator.
#[derive(Debug, Clone)]
pub struct RedTeamGenerator {
    seeds: Vec<RedTeamSeed>,
}

impl RedTeamGenerator {
    /// Build from the default (pinned) seed TOML.
    pub fn default() -> Self {
        let seeds: Vec<RedTeamSeed> =
            seeds::load_seeds(&include_bytes!("../../seeds/red-team-seeds-v0.1.toml")[..])
                .expect("failed to parse bundled red-team seeds");
        Self { seeds }
    }

    /// Build from a fixture seed file.
    pub fn with_fixture_seeds(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("failed to read fixture: {}", e))?;
        let seeds: Vec<RedTeamSeed> = seeds::load_seeds(&data)?;
        Ok(Self { seeds })
    }

    /// Validate all expanded items. Uses `n` items from `expand(n)`.
    pub fn validate_all_n(&self, n: usize) -> Vec<ValidationOutcome> {
        let items = self.expand(n);
        items.iter().map(|item| self.validate(item)).collect()
    }

    /// Convenience: validate the canonical 640-item corpus.
    pub fn validate_all(&self) -> Vec<ValidationOutcome> {
        self.validate_all_n(640)
    }

    /// Build a coverage report for `n` items.
    pub fn coverage_report_n(&self, n: usize) -> CoverageReport {
        let items = self.expand(n);
        build_coverage_report("red-team-640", &items, &self.seeds)
    }

    /// Filter items by class (convenience accessor for downstream stories).
    pub fn filter_by_class(&self, class: &str) -> Vec<RedTeamItem> {
        let items = self.expand(640);
        items.into_iter().filter(|i| i.class == class).collect()
    }
}

impl CorpusGenerator for RedTeamGenerator {
    type Item = RedTeamItem;
    type Seed = RedTeamSeed;

    fn seed_corpus(&self) -> Vec<Self::Seed> {
        self.seeds.clone()
    }

    fn expand(&self, n: usize) -> Vec<Self::Item> {
        expansion::expand_deterministic(&self.seeds, n)
    }

    fn validate(&self, item: &Self::Item) -> ValidationOutcome {
        validation::validate_item(item, &self.seeds)
    }

    fn coverage_report(&self) -> CoverageReport {
        let items = self.expand(640);
        build_coverage_report("red-team-640", &items, &self.seeds)
    }

    fn seed_sha256(&self) -> String {
        SEED_FILE_SHA256.to_string()
    }

    fn rule_version(&self) -> &'static str {
        RULE_VERSION
    }
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

/// Build a `CoverageReport` from expanded items and seeds.
pub fn build_coverage_report(
    corpus_name: &str,
    items: &[RedTeamItem],
    seeds: &[RedTeamSeed],
) -> CoverageReport {
    let total_items = items.len();
    let mut classes: BTreeMap<String, ClassCoverage> = BTreeMap::new();
    let mut param_space: BTreeMap<String, f64> = BTreeMap::new();

    let mut class_counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in items {
        *class_counts.entry(item.class.clone()).or_insert(0) += 1;
    }

    let mut seed_counts: BTreeMap<String, usize> = BTreeMap::new();
    for seed in seeds {
        *seed_counts.entry(seed.class.clone()).or_insert(0) += 1;
    }

    let floor = 80; // ≥80 per class post-dedup

    for (class, &seed_count) in &seed_counts {
        let expanded_count = class_counts.get(class).copied().unwrap_or(0);
        let dedup_drop_count = seed_count.saturating_mul(8).saturating_sub(expanded_count);
        let floor_satisfied = expanded_count >= floor;

        classes.insert(
            class.clone(),
            ClassCoverage {
                seed_count,
                expanded_count,
                dedup_drop_count: dedup_drop_count.min(expanded_count.saturating_sub(1)),
                floor_satisfied,
            },
        );
    }

    for class in classes.keys() {
        let combos_observed = class_counts.get(class).copied().unwrap_or(0);
        // Each class has 10 seeds × ~8 variants ≈ 80 possible combos
        let combos_possible = seed_counts
            .get(class)
            .copied()
            .unwrap_or(0)
            .saturating_mul(8);
        let ratio = if combos_possible > 0 {
            (combos_observed as f64 / combos_possible as f64).min(1.0)
        } else {
            0.0
        };
        param_space.insert(class.clone(), ratio);
    }

    let expanded_seed_ids: std::collections::BTreeSet<String> =
        items.iter().map(|i| i.seed_id.clone()).collect();
    let unexpanded_seed_slots: Vec<String> = seeds
        .iter()
        .filter(|s| !expanded_seed_ids.contains(&s.id))
        .map(|s| s.id.clone())
        .collect();

    CoverageReport {
        corpus_name: corpus_name.to_string(),
        total_items,
        classes,
        unexpanded_seed_slots,
        parameter_space_coverage: param_space,
    }
}

/// CLI entry point for the `coverage` subcommand.
pub fn run_coverage(_corpus_name: &str, json: bool) -> Result<(), String> {
    let gen = RedTeamGenerator::default();
    let report = gen.coverage_report();

    // Floor checks: ≥80 per class
    for (class, cc) in &report.classes {
        if !cc.floor_satisfied {
            eprintln!(
                "NFR-Sec-10 floor violation: class {} has {} items, floor is 80",
                class, cc.expanded_count
            );
            return Err(format!("NFR-Sec-10 floor violation: class {}", class));
        }
    }

    // Unexpanded seed slots
    if !report.unexpanded_seed_slots.is_empty() {
        for sid in &report.unexpanded_seed_slots {
            eprintln!(
                "generator coverage drift: seed {} produced 0 expanded items after dedup — widen parameter axes in src/red_team/expansion.rs",
                sid
            );
        }
        return Err("coverage drift: unexpanded seed slots present".to_string());
    }

    if json {
        let out = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("JSON serialization error: {}", e))?;
        println!("{}", out);
    } else {
        print_text_report(&report);
    }
    Ok(())
}

pub fn print_text_report(report: &CoverageReport) {
    println!("Corpus: {}", report.corpus_name);
    println!("Total items: {}", report.total_items);
    println!();
    println!(
        "{:<40} {:>6} {:>14} {:>12} {:>16}",
        "Class", "Seeds", "Expanded", "DedupDrops", "FloorSatisfied"
    );
    println!("{}", "-".repeat(90));
    for (class, cc) in &report.classes {
        println!(
            "{:<40} {:>6} {:>14} {:>12} {:>16}",
            class, cc.seed_count, cc.expanded_count, cc.dedup_drop_count, cc.floor_satisfied
        );
    }
    if !report.unexpanded_seed_slots.is_empty() {
        println!();
        println!("Unexpanded seed slots: {:?}", report.unexpanded_seed_slots);
    }
}
