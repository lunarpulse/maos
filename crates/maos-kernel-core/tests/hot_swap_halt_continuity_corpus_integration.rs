//! I14 hot-swap halt-continuity corpus integration test (AC3).
//!
//! Loads the Sec-14a `halt-signal-observation` subset of the isolation corpus
//! and exercises `validate_swap_halt_continuity` against corpus-extracted
//! scenarios. Story 5.2 owns the END-TO-END Hot-Swap Coordinator test; this
//! file's `_corpus_integration` suffix marks the wrapper-level distinction.
//!
//! 20-3c (FR53, K1p): the wrapper CARRIES the predecessor's pending halts and
//! never drains them. The corpus scenarios run against an empty registry, so
//! they all return `SafeDrained { 0 }`; the `SafeMigrated` / `Violation` paths
//! are exercised by the seeded-halt vectors below, the inline unit tests in
//! `halt/mod.rs::swap_continuity_tests` and `halt_carry_20_3c.rs`.

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

        // An empty registry can only prove the empty-set verdict (SafeDrained{0});
        // it cannot exhibit a cross-Spirit leak (`halt_carry_20_3c.rs` proves
        // isolation with seeded halts). A fixture that expects any other variant
        // cannot be witnessed here and MUST fail rather than pass vacuously.
        let got = match &verdict {
            SwapVerdict::SafeDrained { .. } => "SafeDrained",
            SwapVerdict::SafeMigrated { .. } => "SafeMigrated",
        };
        assert_eq!(
            expected.variant, got,
            "scenario {}: expected {}, gate returned {:?}",
            scenario.scenario_id, expected.variant, verdict
        );
        assert!(
            matches!(verdict, SwapVerdict::SafeDrained { drained_count: 0 }),
            "scenario {}: an empty registry carries nothing; got {:?}",
            scenario.scenario_id,
            verdict
        );
    }
}

#[test]
fn swap_continuity_with_seeded_halts_carries_them_instead_of_draining() {
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
        SwapVerdict::SafeMigrated {
            migrated_count: 1,
            predecessor_version: 1,
            successor_versions: vec![1],
        },
        "with one pending halt the wrapper carries it: SafeMigrated {{ 1 }}"
    );
    assert_eq!(
        registry.pending_halt_ids().len(),
        1,
        "FR53: the halt is carried, not drained"
    );
}

/// Story 12.5's chain loop reuses the I14 gate for EVERY single-major hop. An
/// in-place hot-swap preserves the spirit pid, so each hop's gate CARRIES that
/// hop's own real pending halt (20-3c K1p: carry, never drain): hop one carries
/// one halt, hop two carries both. Zero-dropped = every seeded halt is still
/// pending after the chain; NO hop is a vacuous `SafeDrained{0}`.
#[test]
fn chain_walk_carries_every_real_pending_halt_at_every_hop_with_zero_dropped() {
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
    let carried = |verdict: &SwapVerdict| match verdict {
        SwapVerdict::SafeMigrated { migrated_count, .. } => *migrated_count,
        other => panic!("expected SafeMigrated, got {other:?}"),
    };

    // Hop 1 (1.0 -> 2.0): a real pending halt, carried.
    seed("story-12-5-hop-1");
    let hop_1 =
        validate_swap_halt_continuity(&registry, PID, 1, Some(&[1])).expect("hop-1 I14 verdict");
    assert_eq!(carried(&hop_1), 1, "hop one must carry a real pending halt");

    // Hop 2 (2.0 -> 3.0): the in-place swap kept the pid, the first halt is
    // STILL pending and the successor accrued its own — both are carried.
    seed("story-12-5-hop-2");
    let hop_2 =
        validate_swap_halt_continuity(&registry, PID, 1, Some(&[1])).expect("hop-2 I14 verdict");
    assert_eq!(
        carried(&hop_2),
        2,
        "hop two carries BOTH real pending halts"
    );

    // Zero dropped across the whole chain: nothing was drained.
    assert_eq!(
        registry.pending_halt_ids().len(),
        2,
        "I14 carries halts; continuity IS read from this live registry"
    );
}
