//! Story 18-2 AC4.3/AC4.5 (D-18-2-05 A; CR-5) — the Researcher halt class.
//!
//! The 30-row corpus `tests/fixtures/halt-corpus-v0.5.jsonl` is pinned by
//! SHA-256. Each row's `expected_halt` is the manifest's two rules over the
//! claim set (`methodology_conflict` > 0.7 or `load_bearing_confidence` < 0.7,
//! `manifest.toml:115-131`), derived here without `primary_scalar`; each
//! `observed_halt` is what `Researcher::survey` → `primary_scalar` → the
//! kernel's `process_scalar_write` actually does. They differ on `mx01` by
//! construction: a conflict at exactly the 0.7 floor is reported instead of a
//! load-bearing confidence of 0.5 (`primary_scalar` reports one scalar), the
//! measured miss the class number carries. FR15: one halting row resolves
//! once per kind through the kernel path, journaled.

#[path = "../../../tests/harness/fr15_resolution.rs"]
mod fr15_resolution;

use std::path::PathBuf;

use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::EpistemicPolicySection;
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use researcher::{ClaimPayload, RecalledFrame, Researcher};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MANIFEST: &str = include_str!("../manifest.toml");
const CORPUS: &str = "tests/fixtures/halt-corpus-v0.5.jsonl";
const PINNED: &str = "b4b3e33ea45d175e947b30b6a0a301d0fb71fde92e4abcc4e21e31f3f1977604";

#[derive(serde::Deserialize)]
struct Row {
    scenario_id: String,
    calendar_conflict: bool,
    expected_halt: bool,
    observed_halt: bool,
    claims: Vec<ClaimPayload>,
}

fn rows() -> Vec<Row> {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CORPUS))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn policy() -> EpistemicPolicySection {
    let manifest: toml::Value = toml::from_str(MANIFEST).unwrap();
    EpistemicPolicySection::from_toml_str(&toml::to_string(&manifest["epistemic_policy"]).unwrap())
        .unwrap()
}

fn frames(claims: &[ClaimPayload]) -> Vec<RecalledFrame> {
    claims
        .iter()
        .enumerate()
        .map(|(i, claim)| RecalledFrame {
            frame_id: [i as u8 + 1; 16],
            intent: "inform".into(),
            payload: serde_json::to_vec(claim).unwrap(),
        })
        .collect()
}

/// The manifest's two rules over the whole claim set.
fn rules_fire(claims: &[ClaimPayload]) -> bool {
    let mut conflict = 0.0f64;
    for (i, a) in claims.iter().enumerate() {
        for b in &claims[i + 1..] {
            let strong = a.methodology_strength >= 0.7 && b.methodology_strength >= 0.7;
            if a.topic == b.topic && a.polarity != b.polarity && strong {
                conflict = conflict.max(a.methodology_strength.min(b.methodology_strength) as f64);
            }
        }
    }
    let load_bearing = claims
        .iter()
        .filter(|c| c.load_bearing)
        .map(|c| c.confidence as f64)
        .fold(1.0f64, f64::min);
    conflict > 0.7 || load_bearing < 0.7
}

fn kernel_halts(claims: &[ClaimPayload], policy: &EpistemicPolicySection) -> bool {
    let (tag, value, derived) = Researcher::new().survey(&frames(claims)).primary_scalar();
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        1,
        Arc::new(PolicyTable::new()),
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let orchestrator = WorkingMemoryOrchestrator::new(capability, Arc::new(HaltRegistry::new()));
    let tmp = tempfile::TempDir::new().unwrap();
    let journal = JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap();
    let tl = TransparencyLogAdapter::open_in_memory(1);
    orchestrator
        .process_scalar_write(
            &tl,
            &journal,
            1,
            "researcher",
            1,
            tag,
            value,
            &derived,
            policy,
        )
        .unwrap()
        .is_some()
}

#[test]
fn researcher_halt_corpus_sha_is_pinned() {
    let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CORPUS)).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        PINNED,
        "halt-corpus-v0.5.jsonl SHA-256 drift — if intentional, update PINNED"
    );
}

#[test]
fn rows_are_thirty_with_fifteen_expected_halts_from_the_manifest_rules() {
    let rows = rows();
    assert_eq!(rows.len(), 30);
    assert_eq!(rows.iter().filter(|r| r.expected_halt).count(), 15);
    for row in &rows {
        assert!(!row.calendar_conflict, "{}", row.scenario_id);
        assert_eq!(
            row.expected_halt,
            rules_fire(&row.claims),
            "{}: expected_halt is the manifest rules over the claim set",
            row.scenario_id
        );
    }
}

#[test]
fn observed_halts_are_self_validating_against_the_real_kernel() {
    let policy = policy();
    let rows = rows();
    for row in &rows {
        assert_eq!(
            kernel_halts(&row.claims, &policy),
            row.observed_halt,
            "{}: survey → primary_scalar → kernel must match the baked observed_halt",
            row.scenario_id
        );
    }
    let misses: Vec<&str> = rows
        .iter()
        .filter(|r| r.expected_halt != r.observed_halt)
        .map(|r| r.scenario_id.as_str())
        .collect();
    assert_eq!(misses, ["mx01"], "the one measured miss");
}

#[test]
fn fr15_a_researcher_halt_resolves_once_per_kind_journaled() {
    let row = rows()
        .into_iter()
        .find(|r| r.scenario_id == "rc01")
        .unwrap();
    let scalar = Researcher::new()
        .survey(&frames(&row.claims))
        .primary_scalar();
    assert_eq!(scalar.0, "methodology_conflict");
    fr15_resolution::resolve_each_kind("researcher", &policy(), scalar);
}
