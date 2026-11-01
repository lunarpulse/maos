#![forbid(unsafe_code)]

//! Integration test: I14 halt-continuity enforcement at swap boundary (AC4).
//!
//! Exercises ≥10 scenarios from `crates/maos-eval/fixtures/halt-continuity-corpus-v0/`
//! via the HaltContinuityCorpus loader.

use maos_eval::halt_continuity_corpus::HaltContinuityCorpus;
use std::path::Path;

#[test]
fn halt_continuity_corpus_loads() {
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../maos-eval/fixtures/halt-continuity-corpus-v0");
    assert!(
        corpus_path.is_dir(),
        "missing halt-continuity corpus: {}",
        corpus_path.display()
    );
    let corpus = HaltContinuityCorpus::load(&corpus_path).expect("load corpus");
    assert!(
        corpus.scenarios.len() >= 10,
        "AC4 requires ≥10 halt-continuity scenarios"
    );
}

/// 20-3c (FR53, K1p): each scenario's `expected_outcome` is COMPARED against
/// what the real I14 gate returns for that scenario's seeded registry. A hot
/// swap keeps the pid, so pending halts are carried (`SafeMigrated{n}`), never
/// drained; an empty set is `SafeDrained{0}`; an incompatible or empty
/// successor list is `EHaltContinuityViolation`.
#[test]
fn every_scenario_expected_outcome_matches_the_gate_verdict() {
    use maos_domain::frame::EpistemicHaltPayload;
    use maos_domain::halt::{HaltContinuityError, HaltId, HaltState};
    use maos_kernel_core::halt::{
        validate_swap_halt_continuity, HaltRegistry, PendingHaltMetadata, SwapVerdict,
    };

    const PID: u32 = 4242;
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../maos-eval/fixtures/halt-continuity-corpus-v0");
    let corpus = HaltContinuityCorpus::load(&corpus_path).expect("load corpus");
    for s in &corpus.scenarios {
        let registry = HaltRegistry::new();
        for (i, id) in s.predecessor.pending_halts.iter().enumerate() {
            registry
                .insert_pending_with_metadata(
                    HaltId::new(id.clone()).expect("fixture halt id"),
                    HaltState::PendingResolution,
                    PendingHaltMetadata {
                        spirit_pid: PID,
                        spirit_id: "corpus-spirit".into(),
                        payload: EpistemicHaltPayload {
                            halt_id: id.clone(),
                            tag: "corpus".into(),
                            value: 0.0,
                            threshold: None,
                            policy_id: "corpus-policy".into(),
                            derived_from: "corpus".into(),
                        },
                        fired_ns: i as u64,
                    },
                )
                .expect("seed halt");
        }
        let got = validate_swap_halt_continuity(
            &registry,
            PID,
            s.predecessor.halt_protocol_version,
            Some(&s.successor.halt_protocol_compatibility),
        );
        let (id, want) = (&s.scenario_id, &s.expected_outcome);
        match (want.verdict.as_str(), got) {
            ("SafeDrained", Ok(SwapVerdict::SafeDrained { drained_count })) => {
                assert_eq!(
                    Some(drained_count),
                    want.drained_count,
                    "{id}: drained_count"
                );
            }
            ("SafeMigrated", Ok(SwapVerdict::SafeMigrated { migrated_count, .. })) => {
                assert_eq!(
                    Some(migrated_count),
                    want.migrated_count,
                    "{id}: migrated_count"
                );
                assert_eq!(
                    registry.pending_halt_ids().len(),
                    migrated_count,
                    "{id}: carried"
                );
            }
            ("Violation", Err(e)) => {
                let name = match &e {
                    HaltContinuityError::EHaltContinuityViolation { orphan_count, .. } => {
                        format!("EHaltContinuityViolation{{orphan_count:{orphan_count}}}")
                    }
                    HaltContinuityError::MissingHaltProtocolCompatibility => {
                        "MissingHaltProtocolCompatibility".to_string()
                    }
                };
                let want_err = want.expected_error.as_deref().expect("expected_error");
                assert!(
                    name == want_err || name.split('{').next() == Some(want_err),
                    "{id}: expected {want_err}, got {name}"
                );
            }
            (verdict, got) => panic!("{id}: expected {verdict}, gate returned {got:?}"),
        }
    }
}
