//! I14 ledger proof: a hot swap with a non-empty halt set carries or drains every halt, and the
//! successor declares a matching `halt_protocol_compatibility`.
//!
//! Seeded swaps (matching, mismatching and undeclared successors; one to three pending halts
//! each) run through the real `UpgradeOrchestrator` / `HotSwapCoordinator`; the halt registry
//! before and after is judged by `invariants::i14::check`, as is the adversarial corpus. The
//! spawned vectors `bin/tests/spawned_swap_20_9.rs::a_hot_swap_keeps_the_pid_tokens_memory_and_state_and_waits_for_the_active_turn`
//! and `::a_predecessor_that_exits_after_its_snapshot_still_completes_its_swap` cover the
//! spawned form. Plant: default an undeclared successor to version 1 again (the pre-21-10 gate).

use std::sync::Arc;

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState};
use maos_domain::hot_swap::HotSwapError;
use maos_domain::invariants::i14::{check, Observation, Swap};
use maos_kernel_core::halt::PendingHaltMetadata;
use maos_kernel_core::hot_swap::{HotSwapPrecheck, PrecheckOutcome};
use maos_kernel_core::lifecycle::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOutcome, UpgradePolicy,
};
use maos_kernel_core::scheduler::{control_block::make_spirit_obj, AnySpiritObj};

mod common;
use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};

const SEEDS: [u64; 4] = [14, 0xA1A1, 0x14C0DE, 1414];
const TRIALS: usize = 6;
const PREDECESSOR_VERSION: u32 = 1;
/// The successor's declared `halt_protocol_compatibility` per variant.
const VARIANTS: [Option<u32>; 3] = [Some(1), Some(2), None];

fn manifest(version: &str, compat: Option<u32>) -> String {
    let mut text = format!(
        "[class]\nname = \"witness\"\nversion = \"{version}\"\nabi = \"1.0\"\nmin_substrate_version = \"0.1.0\"\nmanifest_schema_version = 1\ntrust_tier = \"local\"\nforms = [\"rust-inproc\"]\ndescription = \"I14 witness\"\n[hot_swap]\nstate_schema_uri = \"maos://schemas/witness\"\nstate_schema_version = 65536\n"
    );
    if let Some(v) = compat {
        text += &format!("[halt_protocol_compatibility]\nversion = {v}\n");
    }
    text
}

fn halt(name: &str, pid: u32) -> (HaltId, PendingHaltMetadata) {
    let metadata = PendingHaltMetadata {
        spirit_pid: pid,
        spirit_id: "witness".into(),
        payload: EpistemicHaltPayload {
            halt_id: name.into(),
            tag: "i14".into(),
            value: 0.9,
            threshold: Some(0.8),
            policy_id: "policy:i14".into(),
            derived_from: "frame:i14".into(),
        },
        fired_ns: 7,
    };
    (HaltId::new(name).expect("halt id"), metadata)
}

async fn trial(compat: Option<u32>, halts: usize) -> Swap {
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_: &maos_kernel_core::scheduler::control_block::SpiritManifestBundle| -> Result<Arc<dyn AnySpiritObj>, UpgradeError> {
            Ok(make_spirit_obj(CorpusSpirit))
        },
    );
    let harness = UpgradeHarness::new(factory);
    let predecessor = parse_manifest_bundle(&manifest("0.1.0", None)).expect("predecessor");
    let successor = harness.write_manifest("successor.toml", &manifest("0.2.0", compat));
    let pid = harness
        .scheduler
        .load("witness", predecessor, CorpusSpirit, 0x1914)
        .await
        .expect("load");
    harness.scheduler.start(pid).await.expect("start");
    let before: Vec<_> = (0..halts)
        .map(|n| halt(&format!("i14-halt-{n}"), pid))
        .collect();
    for (id, metadata) in &before {
        harness
            .halt_registry
            .insert_pending_with_metadata(
                id.clone(),
                HaltState::PendingResolution,
                metadata.clone(),
            )
            .expect("pending halt");
    }
    // The standalone precheck (no coordinator cutover) must predict the swap gate's verdict.
    // Plant: make `validate_halt_set` accept an undeclared successor (the precheck stays safe).
    let declared: Vec<u32> = compat.into_iter().collect();
    let precheck = HotSwapPrecheck::check(
        &harness.halt_registry,
        pid,
        PREDECESSOR_VERSION,
        &declared,
        65536,
        65536,
    );
    let compatible = compat == Some(PREDECESSOR_VERSION);
    assert_eq!(
        precheck.verdict,
        if compatible {
            PrecheckOutcome::SafeMigrated
        } else {
            PrecheckOutcome::HaltContinuityViolation
        },
        "precheck verdict for successor {compat:?}"
    );
    let outcome = harness
        .orchestrator
        .upgrade("witness", &successor, UpgradePolicy::HotSwap)
        .await;
    // The refusal is the typed continuity error, not any error (plant: refuse with another error).
    match &outcome {
        Ok(report) => assert!(compatible && report.outcome == UpgradeOutcome::Completed),
        Err(UpgradeError::HotSwap(HotSwapError::HaltContinuityViolation(_))) => {
            assert!(!compatible, "a compatible successor was refused")
        }
        Err(other) => panic!("untyped refusal for successor {compat:?}: {other:?}"),
    }
    let kept = before.iter().all(|(id, metadata)| {
        harness.halt_registry.lookup_state(id) == Some(HaltState::PendingResolution)
            && harness.halt_registry.lookup_pending_metadata(id).as_ref() == Some(metadata)
    });
    Swap {
        halts_before: halts as u32,
        halts_after: harness.halt_registry.pending_halt_ids().len() as u32,
        resolution_state_kept: kept,
        // The kernel never drains a halt on a swap: a halt that vanished has no receipt.
        receipted_drains: 0,
        accepted: matches!(outcome, Ok(report) if report.outcome == UpgradeOutcome::Completed),
        predecessor_version: PREDECESSOR_VERSION,
        successor_version: compat,
    }
}

fn generated(seed: u64) -> Observation {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let swaps = (0..TRIALS)
        .map(|n| {
            runtime.block_on(trial(
                VARIANTS[n % VARIANTS.len()],
                1 + (next() % 3) as usize,
            ))
        })
        .collect();
    Observation { swaps }
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    for seed in SEEDS {
        let obs = generated(seed);
        let accepted = obs.swaps.iter().filter(|s| s.accepted).count();
        if accepted == 0 || accepted == obs.swaps.len() {
            failures.push(format!(
                "seed {seed}: needs accepted and refused swaps: {obs:?}"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i14_a_swap_carries_every_halt_and_needs_a_declared_compatible_successor() {
    let corpus = include_str!("fixtures/invariants/i14_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
