//! Secret-redaction corpus generator.
//!
//! Produces 10⁴ deduplicated synthetic secret-leakage test items from 200
//! SHA-pinned seed patterns distributed across 11 named secret classes.
//!
//! Consumed by NFR-Sec-4 redaction filter (lands v0.5).

use std::collections::BTreeMap;
use std::path::Path;

use crate::{ClassCoverage, CorpusGenerator, CoverageReport, ValidationOutcome};

pub mod expansion;
pub mod seeds;
pub mod validation;

// ---------------------------------------------------------------------------
// Compile-time pinned constants
// ---------------------------------------------------------------------------

/// SHA-256 hex digest of `seeds/secret-redaction-seeds-v0.1.toml`.
/// This constant is verified at build time by `build.rs`.
pub const SEED_FILE_SHA256: &str = "a9dce6273711e44ffc20157cd026dcc94fbfcb95bf1b8989a0e1451a62799aec";

/// Expansion-rule version.  Bump this when expansion axes change and the
/// JSONL corpus must be regenerated.
pub const RULE_VERSION: &str = "v0.1";

// ---------------------------------------------------------------------------
// Binding types
// ---------------------------------------------------------------------------

/// A single seed pattern describing a class of secrets to detect.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretRedactionSeed {
    pub id: String,
    pub class: String,
    pub pattern_regex: String,
    pub false_positive_negative_anchors: Vec<String>,
    pub example_redacted_form: String,
}

/// A single expanded corpus item (one JSONL line).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SecretRedactionItem {
    pub id: String,
    pub class: String,
    pub raw: String,
    pub expected_redacted: String,
    pub seed_id: String,
    pub variant_combo: String,
}

/// HMAC-SHA256 using only the sha2 crate (no hmac dependency).
/// Standard H(K XOR opad || H(K XOR ipad || message)) construction.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let key_block = {
        let mut kb = [0u8; 64];
        if key.len() > 64 {
            let h = Sha256::digest(key);
            kb[..32].copy_from_slice(&h);
        } else {
            kb[..key.len()].copy_from_slice(key);
        }
        kb
    };
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    let result = outer.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// The secret-redaction generator.
#[derive(Debug, Clone)]
pub struct SecretRedactionGenerator {
    seeds: Vec<SecretRedactionSeed>,
}

impl SecretRedactionGenerator {
    /// Build a generator from the default (pinned) seed TOML.
    pub fn default() -> Self {
        let seeds: Vec<SecretRedactionSeed> =
            seeds::load_seeds(&include_bytes!("../../seeds/secret-redaction-seeds-v0.1.toml")[..])
                .expect("failed to parse bundled secret-redaction seeds");
        Self { seeds }
    }

    /// Build a generator from a fixture seed file (for integration tests).
    pub fn with_fixture_seeds(path: &Path) -> Result<Self, String> {
        let data =
            std::fs::read(path).map_err(|e| format!("failed to read fixture: {}", e))?;
        let seeds: Vec<SecretRedactionSeed> = seeds::load_seeds(&data)?;
        Ok(Self { seeds })
    }

    /// Validate every item via the FalseNegativeRisk detector.
    /// Uses `n` items from `expand(n)`. Defaults to 10_000 for the canonical corpus.
    pub fn validate_all_n(&self, n: usize) -> Vec<ValidationOutcome> {
        let items = self.expand(n);
        items.iter().map(|item| self.validate(item)).collect()
    }

    /// Convenience: validate the canonical 10_000-item corpus.
    pub fn validate_all(&self) -> Vec<ValidationOutcome> {
        self.validate_all_n(10_000)
    }

    /// Build a coverage report for `n` items.
    pub fn coverage_report_n(&self, n: usize) -> CoverageReport {
        let items = self.expand(n);
        build_coverage_report("secret-redaction-1e4", &items, &self.seeds)
    }

    /// Generate `n` canary-leak items with cryptographic markers.
    pub fn generate_canary_batch(
        &self,
        n: usize,
        rng_seed: u64,
        marker_namespace: &str,
    ) -> Vec<SecretRedactionItem> {
        let mut items = Vec::with_capacity(n);
        for i in 0..n {
            let marker = hmac_sha256(
                b"maos-canary-v0.1",
                &[
                    rng_seed.to_le_bytes().as_slice(),
                    i.to_le_bytes().as_slice(),
                    marker_namespace.as_bytes(),
                ]
                .concat(),
            );
            let marker_hex: String = marker[..16].iter().map(|b| format!("{:02x}", b)).collect();

            let class = "canary_marker";
            let raw = format!(
                "<CANARY-{}-{:04}-{}>",
                marker_namespace, i, marker_hex
            );
            let expected_redacted =
                format!("<REDACTED:type=canary,len={},hash={}>", raw.len(), &marker_hex[..8]);
            items.push(SecretRedactionItem {
                id: format!("secret-red-cnry-{:05}", i),
                class: class.to_string(),
                raw,
                expected_redacted,
                seed_id: format!("canary-seed-{}", i % self.seeds.len().max(1)),
                variant_combo: format!("canary-{:04}", i),
            });
        }
        items
    }
}

impl CorpusGenerator for SecretRedactionGenerator {
    type Item = SecretRedactionItem;
    type Seed = SecretRedactionSeed;

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
        let items = self.expand(10_000);
        build_coverage_report("secret-redaction-1e4", &items, &self.seeds)
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

/// Build a `CoverageReport` from expanded items and the seed list.
pub fn build_coverage_report(
    corpus_name: &str,
    items: &[SecretRedactionItem],
    seeds: &[SecretRedactionSeed],
) -> CoverageReport {
    let total_items = items.len();
    let mut classes: BTreeMap<String, ClassCoverage> = BTreeMap::new();
    let mut param_space: BTreeMap<String, f64> = BTreeMap::new();

    // Count items per class.
    let mut class_counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in items {
        *class_counts.entry(item.class.clone()).or_insert(0) += 1;
    }

    // Count seeds per class.
    let mut seed_counts: BTreeMap<String, usize> = BTreeMap::new();
    for seed in seeds {
        *seed_counts.entry(seed.class.clone()).or_insert(0) += 1;
    }

    // Compute per-class coverage with proportional floor.
    let total_seeds = seeds.len();
    let items_per_seed = total_items / total_seeds.max(1);
    for (class, &seed_count) in &seed_counts {
        let expanded_count = class_counts.get(class).copied().unwrap_or(0);
        let floor = seed_count * items_per_seed;
        let theoretical_max = seed_count.saturating_mul(items_per_seed);
        let dedup_drop_count = theoretical_max.saturating_sub(expanded_count);
        let floor_satisfied = expanded_count >= floor;

        classes.insert(
            class.clone(),
            ClassCoverage {
                seed_count,
                expanded_count,
                dedup_drop_count,
                floor_satisfied,
            },
        );
    }

    // Compute parameter-space coverage per class.
    for class in classes.keys() {
        // Each seed has N variant combos possible. Rough estimate.
        let combos_possible = seeds
            .iter()
            .filter(|s| &s.class == class)
            .count()
            .saturating_mul(items_per_seed);
        let combos_observed = class_counts.get(class).copied().unwrap_or(0);
        let ratio = if combos_possible > 0 {
            (combos_observed as f64 / combos_possible as f64).min(1.0)
        } else {
            0.0
        };
        param_space.insert(class.clone(), ratio);
    }

    // Find unexpanded seed slots.
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
    let gen = SecretRedactionGenerator::default();
    let report = gen.coverage_report();

    // Check floor violations (AC5: proportional floor with ±10% tolerance).
    // Floor per class = seed_count * items_per_seed (proportional allocation).
    let gen_seeds = gen.seed_corpus();
    let items_per_seed = if !report.classes.is_empty() {
        report.total_items / gen_seeds.len().max(1)
    } else {
        0
    };
    for (class, cc) in &report.classes {
        let floor = cc.seed_count * items_per_seed;
        if cc.expanded_count < floor {
            eprintln!(
                "NFR-Sec-4 floor violation: class {} has {} items, floor is {}",
                class, cc.expanded_count, floor
            );
            return Err(format!(
                "NFR-Sec-4 floor violation: class {} has {} items, floor is {}",
                class, cc.expanded_count, floor
            ));
        }
    }

    // Check unexpanded seed slots.
    if !report.unexpanded_seed_slots.is_empty() {
        for sid in &report.unexpanded_seed_slots {
            eprintln!(
                "generator coverage drift: seed {} produced 0 expanded items after dedup — widen parameter axes in src/secret_redaction/expansion.rs",
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
        "{:<30} {:>6} {:>14} {:>12} {:>16}",
        "Class", "Seeds", "Expanded", "DedupDrops", "FloorSatisfied"
    );
    println!("{}", "-".repeat(80));
    for (class, cc) in &report.classes {
        println!(
            "{:<30} {:>6} {:>14} {:>12} {:>16}",
            class, cc.seed_count, cc.expanded_count, cc.dedup_drop_count, cc.floor_satisfied
        );
    }
    if !report.unexpanded_seed_slots.is_empty() {
        println!();
        println!("Unexpanded seed slots: {:?}", report.unexpanded_seed_slots);
    }
}
