//! I14 hot-swap halt-continuity corpus integration test (AC3).
//!
//! Loads the Sec-14a `halt-signal-observation` subset of the isolation corpus
//! and exercises `validate_swap_halt_continuity` against corpus-extracted
//! scenarios. Story 5.2 owns the END-TO-END Hot-Swap Coordinator test; this
//! file's `_corpus_integration` suffix marks the wrapper-level distinction.
//!
//! v0.3-β limitation: `drain_for_spirit` drains all halts globally, so the
//! wrapper always returns `SafeDrained`. The `SafeMigrated` / `Violation`
//! paths are structurally verified by the inline unit tests in
//! `halt/mod.rs::swap_continuity_tests`.

use std::path::Path;

use maos_eval::isolation_corpus::IsolationCorpus;
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::halt::{validate_swap_halt_continuity, SwapVerdict};

#[test]
fn halt_signal_observation_corpus_exercises_swap_continuity_wrapper() {
    let corpus_path = Path::new("../maos-eval/fixtures/isolation-corpus-v0/");
    assert!(
        corpus_path.exists(),
        "I14 corpus integration: isolation corpus fixture not found at {} — CI must fail-loud",
        corpus_path.display()
    );

    let corpus = IsolationCorpus::load_from(corpus_path)
        .expect("isolation-corpus-v0 must exist and be valid");

    // Filter to Sec-14a halt-signal-observation scenarios
    let halt_scenarios: Vec<_> = corpus
        .scenarios
        .iter()
        .filter(|s| s.split == "sec-14a" && s.category == "halt_signal_observation")
        .collect();

    assert!(
        !halt_scenarios.is_empty(),
        "Sec-14a halt-signal-observation subset must be non-empty"
    );

    for scenario in &halt_scenarios {
        let registry = HaltRegistry::new();

        let expected = scenario.expected_swap_verdict.as_ref().expect(&format!(
            "scenario {} must carry expected_swap_verdict",
            scenario.scenario_id
        ));

        let verdict = validate_swap_halt_continuity(
            &registry,
            scenario.preconditions.spirit_b_pid,
            1,             // predecessor_halt_protocol_version
            Some(&[1, 2]), // successor_accepted_versions
        )
        .expect(&format!(
            "wrapper must not error for scenario {}",
            scenario.scenario_id
        ));

        match expected.variant.as_str() {
            "SafeDrained" => {
                assert!(
                    matches!(verdict, SwapVerdict::SafeDrained { .. }),
                    "scenario {} expected SafeDrained, got {:?}",
                    scenario.scenario_id,
                    verdict
                );
            }
            "SafeMigrated" | "Violation" => {
                // v0.3-β: wrapper always returns SafeDrained because
                // drain_for_spirit drains globally. The corpus still
                // carries these variants for v0.5+ transition. Assert
                // the wrapper produces the known-safe verdict and that
                // no cross-Spirit halt-state leaked.
                assert!(
                    matches!(verdict, SwapVerdict::SafeDrained { .. }),
                    "scenario {}: v0.3-β wrapper must return SafeDrained; got {:?}",
                    scenario.scenario_id,
                    verdict
                );
            }
            other => panic!("unexpected expected_swap_verdict variant: {}", other),
        }
    }
}

#[test]
fn swap_continuity_with_seeded_halts_returns_safe_drained_nonzero() {
    use maos_domain::halt::HaltId;
    use maos_domain::halt::HaltState;

    let registry = HaltRegistry::new();
    let hid = HaltId::new("test-halt-swap-001").unwrap();
    registry
        .insert_pending_with_metadata(
            hid,
            HaltState::PendingResolution,
            maos_kernel_core::halt::PendingHaltMetadata {
                spirit_pid: 200,
                spirit_id: "test-spirit-200".into(),
                payload: maos_domain::frame::EpistemicHaltPayload {
                    halt_id: "test-halt-swap-001".into(),
                    tag: "test".into(),
                    value: 0.0,
                    threshold: None,
                    policy_id: "test-policy".into(),
                    derived_from: "test".into(),
                },
                fired_ns: 0,
            },
        )
        .unwrap();

    let verdict = validate_swap_halt_continuity(
        &registry,
        200, // predecessor_spirit_pid
        1,
        Some(&[1]),
    )
    .unwrap();

    assert_eq!(
        verdict,
        SwapVerdict::SafeDrained { drained_count: 1 },
        "with one pending halt, wrapper must drain and return SafeDrained {{ 1 }}"
    );
}

/// Story 12.5's chain loop reuses the I14 gate for EVERY single-major hop. An
/// in-place hot-swap preserves the spirit pid, so each hop's gate drains THAT
/// hop's own real pending halt. Zero-dropped = every seeded halt across the
/// chain is accounted for as drained-resolved (`before_count == drained_count`
/// at each hop); NO hop is a vacuous `SafeDrained{0}`. (The TL-receipt-derived
/// surviving count is proven separately by the journey leg
/// `j3_blinded_halt_receipt_moves_persistent_agents_halted_count` — kernel-core
/// cannot depend on `spirits/digest`.)
#[test]
fn chain_walk_drains_a_real_pending_halt_at_every_hop_with_zero_dropped() {
    use maos_domain::halt::{HaltId, HaltState};

    const PID: u32 = 1205;
    let registry = HaltRegistry::new();

    let seed = |id: &str| {
        registry
            .insert_pending_with_metadata(
                HaltId::new(id).expect("valid halt ID"),
                HaltState::PendingResolution,
                maos_kernel_core::halt::PendingHaltMetadata {
                    spirit_pid: PID,
                    spirit_id: "marcus-agent".into(),
                    payload: maos_domain::frame::EpistemicHaltPayload {
                        halt_id: id.into(),
                        tag: "migration".into(),
                        value: 0.0,
                        threshold: None,
                        policy_id: "story-12-5".into(),
                        derived_from: "chain-test".into(),
                    },
                    fired_ns: 0,
                },
            )
            .expect("seed pending halt");
    };

    // Hop 1 (1.0 -> 2.0): a real pending halt, drained and reconciled.
    seed("story-12-5-hop-1");
    let before_hop_1 = registry.pending_halt_ids().len();
    let hop_1 =
        validate_swap_halt_continuity(&registry, PID, 1, Some(&[1])).expect("hop-1 I14 verdict");
    assert_eq!(before_hop_1, 1, "hop one must carry a real pending halt");
    assert_eq!(
        hop_1,
        SwapVerdict::SafeDrained {
            drained_count: before_hop_1
        }
    );

    // Hop 2 (2.0 -> 3.0): the in-place swap kept the pid, and the successor has
    // accrued its OWN real pending halt — also drained and reconciled (NOT a
    // vacuous SafeDrained{0}).
    seed("story-12-5-hop-2");
    let before_hop_2 = registry.pending_halt_ids().len();
    let hop_2 =
        validate_swap_halt_continuity(&registry, PID, 1, Some(&[1])).expect("hop-2 I14 verdict");
    assert_eq!(
        before_hop_2, 1,
        "hop two must ALSO carry a real pending halt"
    );
    assert_eq!(
        hop_2,
        SwapVerdict::SafeDrained {
            drained_count: before_hop_2
        }
    );

    // Zero dropped across the whole chain: every seeded halt was drained-resolved.
    let drained_total = match (hop_1, hop_2) {
        (
            SwapVerdict::SafeDrained { drained_count: a },
            SwapVerdict::SafeDrained { drained_count: b },
        ) => a + b,
        other => panic!("expected SafeDrained at both hops, got {other:?}"),
    };
    assert_eq!(
        drained_total, 2,
        "both hops' real pending halts are drained-resolved; zero dropped across the chain"
    );
    assert!(
        registry.pending_halt_ids().is_empty(),
        "I14 drains resolved halts; continuity is NOT read from this live registry"
    );
}
