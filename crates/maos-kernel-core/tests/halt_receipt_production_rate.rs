#![forbid(unsafe_code)]

//! AC4 — receipt production rate ≥99.9% on the 1000-termination corpus.
//!
//! Test surface:
//! - `maos_kernel_core::halt::terminate_spirit`
//! - `maos_kernel_core::halt::HaltRegistry::drain_for_spirit`
//! - `maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::query_frames`
//! - `maos_eval::TerminationCorpus::load_from`
//!
//! Exit criteria: ≥999/1000 receipts present (binomial floor for 99.9%).

use maos_eval::TerminationCorpus;
use maos_kernel_core::halt::{terminate_spirit, HaltRegistry};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;

#[test]
fn test_halt_receipt_production_rate() {
    let corpus = TerminationCorpus::load_from(std::path::Path::new(
        "../maos-eval/fixtures/termination-corpus-v0/",
    ))
    .expect("termination-corpus-v0 must exist");
    assert_eq!(
        corpus.len(),
        1000,
        "corpus size lock — 1000 scenarios authoritative"
    );

    let mut receipts_produced = 0usize;
    let mut expected_total = 0usize;

    for (scenario_index, scenario) in corpus.scenarios.iter().enumerate() {
        let seed = scenario
            .scenario_id
            .as_bytes()
            .iter()
            .fold(0u64, |a, b| a.wrapping_add(*b as u64));
        let tl = TransparencyLogAdapter::open_in_memory(seed);
        let registry = HaltRegistry::new();

        // Story 5.3 — collision-free pid allocator + metadata-based per-PID drain
        let spirit_pid = (scenario_index as u64 + 1_000_000) as u32;

        // Pre-seed the registry with the scenario's pending halts (with metadata)
        for halt_id in &scenario.pending_halts {
            registry
                .insert_pending_with_metadata(
                    maos_domain::halt::HaltId::new(halt_id.clone()).unwrap(),
                    maos_domain::halt::HaltState::PendingResolution,
                    maos_kernel_core::halt::PendingHaltMetadata {
                        spirit_pid,
                        spirit_id: scenario.spirit_id.clone(),
                        payload: maos_domain::frame::EpistemicHaltPayload {
                            halt_id: halt_id.clone(),
                            tag: "test".into(),
                            value: 0.0,
                            threshold: None,
                            policy_id: "".into(),
                            derived_from: "".into(),
                        },
                        fired_ns: 0,
                    },
                )
                .unwrap();
        }
        expected_total += scenario.expected_receipts;

        // Run the termination
        let kind = match scenario.kind {
            maos_eval::TerminationKind::PlannedUnload => {
                maos_domain::halt::TerminationKind::PlannedUnload
            }
            maos_eval::TerminationKind::HaltAccepted => {
                maos_domain::halt::TerminationKind::HaltAccepted
            }
            maos_eval::TerminationKind::UnplannedCrash => {
                maos_domain::halt::TerminationKind::UnplannedCrash
            }
            maos_eval::TerminationKind::HaltRejection => {
                maos_domain::halt::TerminationKind::HaltRejection
            }
            maos_eval::TerminationKind::RevocationTerminated => {
                maos_domain::halt::TerminationKind::RevocationTerminated
            }
        };
        let receipts =
            terminate_spirit(&tl, &registry, spirit_pid, &scenario.spirit_id, kind, seed);

        // Count receipts: for scenarios with pending halts, each halt should
        // produce a receipt; for scenarios with no pending halts, one term-
        // receipt is expected.
        if scenario.pending_halts.is_empty() {
            if !receipts.is_empty() {
                receipts_produced += 1;
            }
        } else {
            for expected_id in &scenario.expected_receipt_ids {
                if receipts.iter().any(|r| r.halt_id.as_str() == expected_id) {
                    receipts_produced += 1;
                }
            }
        }
    }

    // 99.9% floor
    let rate = receipts_produced as f64 / expected_total.max(1) as f64;
    assert!(
        rate >= 0.999,
        "receipt rate {rate:.4} below 99.9% floor (produced={receipts_produced} / expected={expected_total})"
    );
}
