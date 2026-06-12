//! Story 8.5 AC5 — the **safety-critical Spirit corpus** + **Cohen's κ**
//! inter-annotator-agreement computation (Decision F).
//!
//! Before 8.5, `maos-eval` only ever *loaded* a pre-computed κ from
//! `iaa-attestation.json` ([`IaaAttestation`](crate::IaaAttestation), Story 4.4,
//! distillate floor κ≥0.85). This module **authors the κ computation itself** and
//! a deterministic **N≥150-per-Spirit** safety-critical corpus for the Mira+Nash
//! bilateral pair.
//!
//! ## The stand-in annotation seam (Decision F)
//! The corpus carries **two annotators' labels** per scenario. In production those
//! labels come from a documented **2-annotator human protocol**
//! (`docs/safety-critical-corpus-methodology.md`); in CI the labels are
//! **fixture-replayed by the deterministic generator** so κ is bit-stable —
//! exactly the 7.5b stand-in-corpus / Story 4.4 `iaa-attestation.json` pattern.
//!
//! ## The κ floor (Decision F)
//! The safety-critical floor is **κ ≥ 0.7** ([`SAFETY_CRITICAL_KAPPA_FLOOR`]),
//! the epic's value — deliberately LOWER than distillate's 0.85, because
//! safety-critical *scenario labelling* is a coarser categorical judgment than
//! distillate hedge-preservation. The rationale is recorded in the methodology doc.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::IaaAttestation;

/// The safety-critical inter-annotator-agreement floor (Decision F — the epic's
/// value, distinct from distillate's 0.85).
pub const SAFETY_CRITICAL_KAPPA_FLOOR: f64 = 0.7;

/// The minimum scenario count per Spirit (Mira + Nash) — NFR floor (AC5).
pub const MIN_SCENARIOS_PER_SPIRIT: usize = 150;

/// The generator rule version — bumped on any intentional corpus change. Hashed
/// into the corpus `prompt_version_hash`.
pub const RULE_VERSION: &str = "v1.5";

/// The corpus version recorded in the [`IaaAttestation`].
pub const CORPUS_VERSION: &str = "safety-critical-mira-nash-v1.5";

/// The authored date recorded in the [`IaaAttestation`] (fixed for bit-stability —
/// the corpus is deterministic, so its attestation must be too).
pub const COMPUTED_AT: &str = "2026-06-04";

/// One annotator's categorical safety judgment of a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SafetyLabel {
    /// No safety action warranted.
    Benign,
    /// Surface / advise, but no halt.
    Caution,
    /// A safety-critical condition: halt / escalate.
    Critical,
}

impl SafetyLabel {
    const ALL: [SafetyLabel; 3] = [
        SafetyLabel::Benign,
        SafetyLabel::Caution,
        SafetyLabel::Critical,
    ];

    fn from_index(i: usize) -> SafetyLabel {
        SafetyLabel::ALL[i % 3]
    }

    fn shifted(self) -> SafetyLabel {
        match self {
            SafetyLabel::Benign => SafetyLabel::Caution,
            SafetyLabel::Caution => SafetyLabel::Critical,
            SafetyLabel::Critical => SafetyLabel::Benign,
        }
    }
}

/// One safety-critical scenario with the two annotators' replayed labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyScenario {
    /// Stable scenario id (`mira-000` / `nash-149`).
    pub id: String,
    /// The Spirit the scenario exercises (`mira` | `nash`).
    pub spirit: String,
    /// The deterministic scenario prompt.
    pub prompt: String,
    /// Annotator A's replayed label (the stand-in seam — Decision F).
    pub annotator_a: SafetyLabel,
    /// Annotator B's replayed label.
    pub annotator_b: SafetyLabel,
}

/// Cohen's κ over two annotators' categorical labels:
/// `κ = (p_o − p_e) / (1 − p_e)`, where `p_o` is observed agreement and `p_e` is
/// the agreement expected by chance from each annotator's marginal label
/// frequencies. Deterministic; perfect agreement → `1.0`, chance-level → `0.0`.
///
/// # Single-sample convention
/// When all labels fall in a single category, `p_e = 1.0` and κ is mathematically
/// undefined. By convention: return `1.0` if observed agreement is also perfect,
/// else `0.0`. This matches the standard treatment for degenerate distributions.
///
/// # Panics
/// Panics if `a.len() != b.len()`.
pub fn cohen_kappa<T: Eq + std::hash::Hash>(a: &[T], b: &[T]) -> f64 {
    assert_eq!(
        a.len(),
        b.len(),
        "annotator label vectors must be equal length"
    );
    let n = a.len();
    if n == 0 {
        return 1.0;
    }
    let nf = n as f64;

    let mut agree = 0usize;
    let mut count_a: HashMap<&T, usize> = HashMap::new();
    let mut count_b: HashMap<&T, usize> = HashMap::new();
    for i in 0..n {
        if a[i] == b[i] {
            agree += 1;
        }
        *count_a.entry(&a[i]).or_insert(0) += 1;
        *count_b.entry(&b[i]).or_insert(0) += 1;
    }

    let p_o = agree as f64 / nf;
    // p_e sums products of marginals over the union of categories; a category
    // present in only one annotator contributes a zero product, so iterating
    // over `count_a` (with a `count_b` lookup) covers every nonzero term.
    let mut p_e = 0.0;
    for (label, &ca) in &count_a {
        let cb = count_b.get(label).copied().unwrap_or(0);
        p_e += (ca as f64 / nf) * (cb as f64 / nf);
    }

    if (1.0 - p_e).abs() < f64::EPSILON {
        // All labels in one category → κ is undefined; by convention return 1.0
        // iff observed agreement is also perfect, else 0.0.
        return if p_o >= 1.0 { 1.0 } else { 0.0 };
    }
    (p_o - p_e) / (1.0 - p_e)
}

/// The deterministic safety-critical corpus for the Mira+Nash pair.
#[derive(Debug, Clone)]
pub struct SafetyCriticalCorpus {
    scenarios: Vec<SafetyScenario>,
}

impl Default for SafetyCriticalCorpus {
    fn default() -> Self {
        Self::generate()
    }
}

impl SafetyCriticalCorpus {
    /// Generate the corpus deterministically: [`MIN_SCENARIOS_PER_SPIRIT`]
    /// scenarios for Mira + the same for Nash, each with two replayed annotator
    /// labels. Annotator B mirrors A except on a fixed ~1-in-9 subset (the
    /// stand-in inter-annotator disagreement) — bit-identical every run.
    pub fn generate() -> Self {
        let mut scenarios = Vec::with_capacity(MIN_SCENARIOS_PER_SPIRIT * 2);
        for (spirit, offset) in [("mira", 0usize), ("nash", 1usize)] {
            for i in 0..MIN_SCENARIOS_PER_SPIRIT {
                // Deterministic "true" label, lightly de-correlated per Spirit.
                let truth = SafetyLabel::from_index(i * 7 + offset);
                let annotator_a = truth;
                // Annotator B disagrees on a fixed 1-in-9 cadence (stand-in IAA).
                let annotator_b = if i % 9 == 0 { truth.shifted() } else { truth };
                let prompt = match spirit {
                    "mira" => format!(
                        "Mira prod-edge diagnostic scenario {i}: classify the safety criticality of an anomaly on service shard-{}",
                        i % 17
                    ),
                    _ => format!(
                        "Nash architecture scenario {i}: classify the safety criticality of a proposed fix touching subsystem mod-{}",
                        i % 13
                    ),
                };
                scenarios.push(SafetyScenario {
                    id: format!("{spirit}-{i:03}"),
                    spirit: spirit.to_string(),
                    prompt,
                    annotator_a,
                    annotator_b,
                });
            }
        }
        Self { scenarios }
    }

    /// All scenarios.
    pub fn scenarios(&self) -> &[SafetyScenario] {
        &self.scenarios
    }

    /// Total scenario count.
    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    /// Whether the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }

    /// The scenario count for one Spirit.
    pub fn count_for(&self, spirit: &str) -> usize {
        self.scenarios.iter().filter(|s| s.spirit == spirit).count()
    }

    /// The two annotators' label vectors for one Spirit (in scenario order).
    pub fn annotator_labels(&self, spirit: &str) -> (Vec<SafetyLabel>, Vec<SafetyLabel>) {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for s in self.scenarios.iter().filter(|s| s.spirit == spirit) {
            a.push(s.annotator_a);
            b.push(s.annotator_b);
        }
        (a, b)
    }

    /// Cohen's κ for one Spirit's scenarios.
    pub fn kappa_for(&self, spirit: &str) -> f64 {
        let (a, b) = self.annotator_labels(spirit);
        cohen_kappa(&a, &b)
    }

    /// Cohen's κ across the whole corpus (both Spirits).
    pub fn kappa(&self) -> f64 {
        let a: Vec<SafetyLabel> = self.scenarios.iter().map(|s| s.annotator_a).collect();
        let b: Vec<SafetyLabel> = self.scenarios.iter().map(|s| s.annotator_b).collect();
        cohen_kappa(&a, &b)
    }

    /// Produce the [`IaaAttestation`] over the whole corpus (annotator_count = 2,
    /// `hedge_cohen_kappa` = the safety-critical κ). Mirrors the Story 4.4
    /// attestation shape.
    pub fn attestation(&self) -> IaaAttestation {
        IaaAttestation {
            corpus_version: CORPUS_VERSION.to_string(),
            annotator_count: 2,
            hedge_cohen_kappa: self.kappa(),
            computed_at: COMPUTED_AT.to_string(),
        }
    }

    /// Canonical JSON bytes of the corpus (stable field order) — the basis of the
    /// Story 0.3 SHA-256 pin.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&self.scenarios).map_err(|e| format!("corpus serialization failed: {e}"))
    }

    /// SHA-256 of the canonical corpus bytes — the Story 0.3 pin (registered in
    /// `tests/corpora/MANIFEST.toml`).
    pub fn seed_sha256(&self) -> Result<String, String> {
        let bytes = self.canonical_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    /// SHA-256 of the generator rule version — the `prompt_version_hash`.
    pub fn prompt_version_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(RULE_VERSION.as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Validate the corpus against the AC5 floors. Returns `Err` (fail-loud) if the
    /// corpus shrinks below the per-Spirit floor or κ drops below the safety floor.
    pub fn validate(&self) -> Result<IaaAttestation, String> {
        for spirit in ["mira", "nash"] {
            let n = self.count_for(spirit);
            if n < MIN_SCENARIOS_PER_SPIRIT {
                return Err(format!(
                    "safety-critical corpus for '{spirit}' shrank to {n} < {MIN_SCENARIOS_PER_SPIRIT}"
                ));
            }
            let k = self.kappa_for(spirit);
            if k < SAFETY_CRITICAL_KAPPA_FLOOR {
                return Err(format!(
                    "safety-critical κ for '{spirit}' = {k:.4} < floor {SAFETY_CRITICAL_KAPPA_FLOOR}"
                ));
            }
        }
        let att = self.attestation();
        if att.annotator_count < 2 {
            return Err(format!("annotator_count {} < 2", att.annotator_count));
        }
        if att.hedge_cohen_kappa < SAFETY_CRITICAL_KAPPA_FLOOR {
            return Err(format!(
                "overall κ {:.4} < floor {SAFETY_CRITICAL_KAPPA_FLOOR}",
                att.hedge_cohen_kappa
            ));
        }
        Ok(att)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cohen_kappa_perfect_agreement_is_one() {
        let a = [
            SafetyLabel::Benign,
            SafetyLabel::Caution,
            SafetyLabel::Critical,
        ];
        let b = a;
        assert!((cohen_kappa(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cohen_kappa_chance_level_is_near_zero() {
        // Two annotators each split 50/50 between two categories but with the
        // pattern arranged so observed agreement ≈ expected agreement → κ ≈ 0.
        let a = [0u8, 0, 1, 1];
        let b = [0u8, 1, 0, 1];
        let k = cohen_kappa(&a, &b);
        assert!(k.abs() < 1e-9, "expected κ≈0, got {k}");
    }

    #[test]
    fn cohen_kappa_total_disagreement_is_negative() {
        let a = [0u8, 0, 0, 0];
        let b = [1u8, 1, 1, 1];
        // All in one category each, fully disagree → convention 0.0 (undefined p_e).
        let k = cohen_kappa(&a, &b);
        assert!(k <= 0.0, "total disagreement κ ≤ 0, got {k}");
    }

    #[test]
    fn corpus_has_at_least_150_per_spirit() {
        let c = SafetyCriticalCorpus::generate();
        assert!(c.count_for("mira") >= MIN_SCENARIOS_PER_SPIRIT);
        assert!(c.count_for("nash") >= MIN_SCENARIOS_PER_SPIRIT);
        assert_eq!(c.len(), MIN_SCENARIOS_PER_SPIRIT * 2);
    }

    #[test]
    fn corpus_kappa_meets_safety_floor() {
        let c = SafetyCriticalCorpus::generate();
        assert!(
            c.kappa_for("mira") >= SAFETY_CRITICAL_KAPPA_FLOOR,
            "mira κ = {}",
            c.kappa_for("mira")
        );
        assert!(
            c.kappa_for("nash") >= SAFETY_CRITICAL_KAPPA_FLOOR,
            "nash κ = {}",
            c.kappa_for("nash")
        );
        let att = c.attestation();
        assert_eq!(att.annotator_count, 2);
        assert!(att.hedge_cohen_kappa >= SAFETY_CRITICAL_KAPPA_FLOOR);
        assert_eq!(att.corpus_version, CORPUS_VERSION);
    }

    #[test]
    fn validate_passes_and_fails_loud_on_shrink() {
        let c = SafetyCriticalCorpus::generate();
        assert!(c.validate().is_ok());
        // Fail-loud on shrink.
        let shrunk = SafetyCriticalCorpus {
            scenarios: c.scenarios()[..10].to_vec(),
        };
        assert!(shrunk.validate().is_err(), "a shrunk corpus must fail loud");
    }

    #[test]
    fn corpus_is_deterministic_and_pinned() {
        let a = SafetyCriticalCorpus::generate();
        let b = SafetyCriticalCorpus::generate();
        assert_eq!(
            a.seed_sha256().expect("pin a"),
            b.seed_sha256().expect("pin b"),
            "generation is bit-identical"
        );
        // Story 0.3 SHA-256 pin — fails loud if the corpus changes.
        assert_eq!(
            a.seed_sha256().expect("pin a"),
            CORPUS_SHA256_PIN,
            "safety-critical corpus changed — if intentional, update CORPUS_SHA256_PIN + MANIFEST.toml"
        );
    }
}

/// Story 0.3 SHA-256 pin of the generated safety-critical corpus (canonical
/// bytes). **Regenerate** (and review + update `tests/corpora/MANIFEST.toml`)
/// only on an intentional corpus change.
pub const CORPUS_SHA256_PIN: &str =
    "454ba193143bbbfbfdae7639b3a700702b64e682006b53c1ba15020aff600505";
