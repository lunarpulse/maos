//! CCAC — ComplianceClaim Adversarial Corpus generator (Story 7.3, NFR-Aud-9).
//!
//! Produces N=640 deterministic, third-party-reproducible ComplianceClaim
//! envelopes from 64 SHA-pinned seed templates × 10 variations:
//!
//! * **200 well-formed** (5 classes × 4 templates × 10) → expected verdict `admit`
//! * **440 malformed** (44 templates × 10) → expected verdict `reject`, of which
//!   **exactly 140** are context-drift claims (`ContextDrift`, field-named).
//!
//! Every envelope is built with the SAME `maos_compliance::builder` /
//! `canonical_cbor` the admission evaluator uses, so the generator and the
//! evaluator agree byte-for-byte (load-bearing for the ±2% cross-validation).
//!
//! Each envelope is bound to one of 3 reference Spirit contexts — `hello`
//! (`maos-spirit-hello`'s manifest), `template-7-1` (the Story 7.1
//! cargo-generate template output), and `synth-pu` (a corpus-internal
//! synthesized `public_untrusted` reference; Butler from Epic 8 is not yet
//! built). The references are rotated by variation index so each class is
//! spread across all 3 contexts for the cross-validation gate.

use std::collections::BTreeMap;

use maos_compliance::canonical_cbor::sha256;
use maos_compliance::RuntimeExecutionContext;
use maos_spirit_abi::compliance::{
    CapabilityId, CryptoProviderId, ProviderEndpointPin, SandboxTier, TrustTier,
};
use std::collections::BTreeSet;
use std::path::Path;

use crate::{ClassCoverage, CorpusGenerator, CoverageReport, ValidationOutcome};

pub mod expansion;
pub mod seeds;
pub mod validation;

/// SHA-256 hex digest of `seeds/ccac-seeds-v1.0.toml` (pinned; build.rs enforces).
pub const SEED_FILE_SHA256: &str =
    "700d294661d0375138e84fe2f8e7ac50dd46172425f25f777a60445d529e28ee";

/// Expansion-rule version.
pub const RULE_VERSION: &str = "v1.0";

/// Total corpus size.
pub const CORPUS_SIZE: usize = 640;

/// Variations emitted per seed template.
pub const VARIATIONS_PER_SEED: usize = 10;

/// Per-class minimum the generator emits (gate floor is ≥27/30 = ≥90%).
pub const PER_CLASS_FLOOR: usize = 30;

/// The 3 reference Spirit contexts the corpus binds against.
pub const REFERENCES: [&str; 3] = ["hello", "template-7-1", "synth-pu"];

// ---------------------------------------------------------------------------
// Seed + item types
// ---------------------------------------------------------------------------

/// A single CCAC seed template.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CcacSeed {
    pub id: String,
    pub class: String,
    /// `"well_formed"` or `"malformed"`.
    pub kind: String,
    /// For malformed seeds: the mutation operator.
    #[serde(default)]
    pub malform: Option<String>,
    /// For `context_drift` seeds: which `DriftField` drifts.
    #[serde(default)]
    pub drift_field: Option<String>,
    pub rationale: String,
}

/// A single expanded CCAC corpus item (one JSONL line).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CcacItem {
    pub id: String,
    pub class: String,
    /// `"admit"` or `"reject"`.
    pub expected_verdict: String,
    /// `SignatureInvalid` | `MalformedClaim` | `ContextDrift` | `ExpiredClaim` | null.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_rejection_kind: Option<String>,
    /// The drifted `DriftField` (for `ContextDrift` items only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_rejection_field: Option<String>,
    /// Which of the 3 reference contexts this envelope is bound to.
    pub reference_spirit: String,
    /// Hex of the canonical-CBOR `ComplianceClaimEnvelope`.
    pub envelope_cbor_hex: String,
    /// The reference Spirit manifest this envelope is bound to.
    pub manifest_toml: String,
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// Reference contexts
// ---------------------------------------------------------------------------

fn caps(items: &[&str]) -> BTreeSet<CapabilityId> {
    items.iter().map(|s| CapabilityId(s.to_string())).collect()
}

/// Return the `(manifest_toml, RuntimeExecutionContext)` for a named reference.
///
/// The SAME function is used by the generator (to build envelopes) and the
/// ship gate (to reconstruct the runtime context at replay), so the two agree.
///
/// Returns `Err` for unknown names so callers can collect the failure into
/// a structured report instead of crashing.
pub fn reference_context(name: &str) -> Result<(String, RuntimeExecutionContext), String> {
    let (manifest, version, tier, sandbox, cap_list, provider, endpoint, crypto) = match name {
        "hello" => (
            "[spirit]\nname = \"maos-spirit-hello\"\nversion = \"1.0.0\"\ntrust_tier = \"local\"\nsandbox_tier = \"t1\"\nprovider_id = \"anthropic\"\nendpoint_url = \"https://api.anthropic.com\"\ncrypto_provider = \"ring\"\ncapability_scope = [\"fs.read\"]\n",
            "1.0.0", TrustTier::Local, SandboxTier::T1, vec!["fs.read"],
            "anthropic", "https://api.anthropic.com", "ring",
        ),
        "template-7-1" => (
            "[spirit]\nname = \"template-spirit\"\nversion = \"0.2.0\"\ntrust_tier = \"org_internal\"\nsandbox_tier = \"t2\"\nprovider_id = \"openai\"\nendpoint_url = \"https://api.openai.com\"\ncrypto_provider = \"ring\"\ncapability_scope = [\"net.connect\"]\n",
            "0.2.0", TrustTier::OrgInternal, SandboxTier::T2, vec!["net.connect"],
            "openai", "https://api.openai.com", "ring",
        ),
        "synth-pu" => (
            "[spirit]\nname = \"synth-public-untrusted\"\nversion = \"3.1.4\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\nprovider_id = \"ollama\"\nendpoint_url = \"http://localhost:11434\"\ncrypto_provider = \"ring\"\ncapability_scope = []\n",
            "3.1.4", TrustTier::PublicUntrusted, SandboxTier::T3, vec![],
            "ollama", "http://localhost:11434", "ring",
        ),
        other => return Err(format!("unknown CCAC reference context '{other}'")),
    };
    let manifest = manifest.to_string();
    let ctx = RuntimeExecutionContext {
        manifest_hash: sha256(manifest.as_bytes()),
        spirit_version: version.to_string(),
        effective_trust_tier: tier,
        effective_sandbox_tier: sandbox,
        runtime_provider_endpoint: ProviderEndpointPin {
            provider_id: provider.to_string(),
            endpoint_url: endpoint.to_string(),
            model_id: None,
        },
        runtime_crypto_provider: CryptoProviderId(crypto.to_string()),
        capability_scope: caps(&cap_list),
    };
    Ok((manifest, ctx))
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// The CCAC generator.
#[derive(Debug, Clone)]
pub struct CcacGenerator {
    seeds: Vec<CcacSeed>,
}

impl CcacGenerator {
    /// Build from the bundled (pinned) seed TOML.
    pub fn new() -> Self {
        let seeds = seeds::load_seeds(&include_bytes!("../../seeds/ccac-seeds-v1.0.toml")[..])
            .expect("failed to parse bundled CCAC seeds");
        Self { seeds }
    }

    /// Build from a fixture seed file.
    pub fn with_fixture_seeds(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("failed to read fixture: {e}"))?;
        let seeds = seeds::load_seeds(&data)?;
        Ok(Self { seeds })
    }

    /// Coverage report for the canonical N=600 corpus.
    pub fn coverage_report_full(&self) -> CoverageReport {
        let items = self.expand(CORPUS_SIZE);
        build_coverage_report("ccac-v1.0", &items, &self.seeds)
    }

    /// Count of context-drift items in the canonical corpus (arithmetic from seeds).
    pub fn drift_count(&self) -> usize {
        self.seeds
            .iter()
            .filter(|s| s.malform.as_deref() == Some("context_drift"))
            .count()
            * VARIATIONS_PER_SEED
    }
}

impl Default for CcacGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CorpusGenerator for CcacGenerator {
    type Item = CcacItem;
    type Seed = CcacSeed;

    fn seed_corpus(&self) -> Vec<Self::Seed> {
        self.seeds.clone()
    }

    fn expand(&self, n: usize) -> Vec<Self::Item> {
        expansion::expand_deterministic(&self.seeds, n)
            .expect("CCAC expansion is deterministic and the envelopes are serializable by construction")
    }

    fn validate(&self, item: &Self::Item) -> ValidationOutcome {
        validation::validate_item(item)
    }

    fn coverage_report(&self) -> CoverageReport {
        self.coverage_report_full()
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

/// Build a [`CoverageReport`] from expanded items + seeds.
pub fn build_coverage_report(
    corpus_name: &str,
    items: &[CcacItem],
    seeds: &[CcacSeed],
) -> CoverageReport {
    let total_items = items.len();
    let mut classes: BTreeMap<String, ClassCoverage> = BTreeMap::new();

    let mut class_counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in items {
        *class_counts.entry(item.class.clone()).or_insert(0) += 1;
    }
    let mut seed_counts: BTreeMap<String, usize> = BTreeMap::new();
    for seed in seeds {
        *seed_counts.entry(seed.class.clone()).or_insert(0) += 1;
    }

    for (class, &seed_count) in &seed_counts {
        let expanded_count = class_counts.get(class).copied().unwrap_or(0);
        classes.insert(
            class.clone(),
            ClassCoverage {
                seed_count,
                expanded_count,
                dedup_drop_count: 0,
                floor_satisfied: expanded_count >= PER_CLASS_FLOOR,
            },
        );
    }

    let expanded_ids: BTreeSet<String> = items.iter().map(|i| i.class.clone()).collect();
    let unexpanded_seed_slots: Vec<String> = seeds
        .iter()
        .filter(|s| !expanded_ids.contains(&s.class))
        .map(|s| s.id.clone())
        .collect();

    let mut parameter_space_coverage = BTreeMap::new();
    for (class, cc) in &classes {
        parameter_space_coverage.insert(class.clone(), if cc.floor_satisfied { 1.0 } else { 0.0 });
    }

    CoverageReport {
        corpus_name: corpus_name.to_string(),
        total_items,
        classes,
        unexpanded_seed_slots,
        parameter_space_coverage,
    }
}

/// CLI entry for the `coverage --corpus ccac-600` subcommand.
pub fn run_coverage(_corpus_name: &str, json: bool) -> Result<(), String> {
    let gen = CcacGenerator::new();
    let report = gen.coverage_report();

    for (class, cc) in &report.classes {
        if !cc.floor_satisfied {
            eprintln!(
                "NFR-Aud-9 floor violation: class {} has {} items, floor is {}",
                class, cc.expanded_count, PER_CLASS_FLOOR
            );
            return Err(format!("NFR-Aud-9 floor violation: class {class}"));
        }
    }

    if report.total_items != CORPUS_SIZE {
        return Err(format!(
            "NFR-Aud-9 size violation: corpus has {} items, expected {}",
            report.total_items, CORPUS_SIZE
        ));
    }

    let drift = gen.drift_count();
    if drift != 140 {
        return Err(format!(
            "NFR-Aud-9 drift-count violation: {drift} context-drift items, expected exactly 140"
        ));
    }

    if json {
        let out = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{out}");
    } else {
        print_text_report(&report, drift);
    }
    Ok(())
}

/// CLI entry for `coverage --corpus ccac-600 --seeds-fixture <path>`.
pub fn run_coverage_with_fixture(
    _corpus_name: &str,
    json: bool,
    fixture_path: &Path,
) -> Result<(), String> {
    let gen = CcacGenerator::with_fixture_seeds(fixture_path)?;
    let report = gen.coverage_report();
    for (class, cc) in &report.classes {
        if !cc.floor_satisfied {
            return Err(format!(
                "NFR-Aud-9 floor violation: class {} has {} items, floor is {}",
                class, cc.expanded_count, PER_CLASS_FLOOR
            ));
        }
    }
    let drift = gen.drift_count();
    if json {
        let out = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{out}");
        eprintln!("Context-drift items: {drift}");
    } else {
        print_text_report(&report, drift);
    }
    Ok(())
}

pub fn print_text_report(report: &CoverageReport, drift_count: usize) {
    println!("Corpus: {}", report.corpus_name);
    println!("Total items: {}", report.total_items);
    println!("Context-drift items: {drift_count} (target 140)");
    println!();
    println!(
        "{:<28} {:>6} {:>10} {:>16}",
        "Class", "Seeds", "Expanded", "FloorSatisfied"
    );
    println!("{}", "-".repeat(64));
    for (class, cc) in &report.classes {
        println!(
            "{:<28} {:>6} {:>10} {:>16}",
            class, cc.seed_count, cc.expanded_count, cc.floor_satisfied
        );
    }
    if !report.unexpanded_seed_slots.is_empty() {
        println!(
            "\nUnexpanded seed slots: {:?}",
            report.unexpanded_seed_slots
        );
    }
}
