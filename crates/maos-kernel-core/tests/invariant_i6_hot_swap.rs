//! I6 ledger proof: a hot swap preserves the Capability Tokens of in-flight calls, inherits
//! in-flight A2A frames and accepts only a compatible state schema.
//!
//! Seeded swaps (same-major, cross-major with and without a declared migrator, a migrator that
//! does not name the predecessor) run through the real `UpgradeOrchestrator` and
//! `HotSwapCoordinator` with tokens minted and A2A frames queued at the predecessor's pid; what
//! survived is judged by `invariants::i6::check`, as is the adversarial corpus. Plant: perform
//! the swap as unload + load (revokes the pid's tokens) or accept a cross-major schema without
//! running the migrator check.

use std::sync::Arc;

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i6::{check, Observation, Swap};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::IacBusPort;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{ManifestCapabilityScope, PolicyTableInner};
use maos_kernel_core::lifecycle::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOutcome, UpgradePolicy,
};
use maos_kernel_core::scheduler::{control_block::make_spirit_obj, AnySpiritObj};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

mod common;
use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};

const SEEDS: [u64; 4] = [8, 0xABCD, 0x600D, 606];
const TRIALS: usize = 4;
const BEFORE_SCHEMA: u32 = 65536;
const HEADER: &str = "[class]\nname = \"witness\"\nversion = \"VERSION\"\nabi = \"1.0\"\nmin_substrate_version = \"0.1.0\"\nmanifest_schema_version = 1\ntrust_tier = \"local\"\nforms = [\"rust-inproc\"]\ndescription = \"I6 witness\"\n";

/// `(successor schema, successor migrates_from pattern)` per variant.
const VARIANTS: [(u32, Option<&str>); 4] = [
    (65537, None),
    (131073, Some("0.1.0")),
    (131073, None),
    (196609, Some("9.9.9")),
];

fn manifest(version: &str, schema: u32, migrates: Option<&str>) -> String {
    let mut text = HEADER.replace("VERSION", version);
    // Story 21-9 (D-21-PF-14): a swap installs the successor's declared scopes as the pid's
    // effective policy, so the tokens at the pid survive only if the successor keeps the scope.
    text += "[capabilities.required]\nprovider.complete = [\"witness\"]\n";
    if migrates.is_some() {
        text += "[lifecycle]\nenabled_hooks = [\"migrate\", \"on_swap_in\"]\n";
    }
    text += &format!(
        "[hot_swap]\nstate_schema_uri = \"maos://schemas/witness\"\nstate_schema_version = {schema}\n"
    );
    if let Some(from) = migrates {
        text += &format!("[migrates_from]\nversions = [\"{from}\"]\n");
    }
    text
}

fn frame(goal: u64) -> IacFrame {
    let address = |id: &str| FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: [goal as u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: address(maos_kernel_core::iac::mailbox::KERNEL_SENDER_SPIRIT_ID),
        to: smallvec![address("witness")],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("in-flight-{goal}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

async fn trial(variant: usize, tokens: u32, frames: u32) -> Swap {
    let (schema_after, migrates) = VARIANTS[variant];
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_: &maos_kernel_core::scheduler::control_block::SpiritManifestBundle| -> Result<Arc<dyn AnySpiritObj>, UpgradeError> {
            Ok(make_spirit_obj(CorpusSpirit))
        },
    );
    let harness = UpgradeHarness::new(factory);
    let predecessor =
        parse_manifest_bundle(&manifest("0.1.0", BEFORE_SCHEMA, None)).expect("predecessor");
    let successor =
        harness.write_manifest("successor.toml", &manifest("0.2.0", schema_after, migrates));
    let pid = harness
        .scheduler
        .load("witness", predecessor, CorpusSpirit, 0x1906)
        .await
        .expect("load");
    harness.scheduler.start(pid).await.expect("start");
    let scope = Scope::ProviderInfer {
        provider: "witness".into(),
    };
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes: vec![scope.clone()],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    harness.policy.update(inner);
    let held: Vec<_> = (0..tokens)
        .map(|_| {
            harness
                .capability
                .issue_with_mediation(pid, scope.clone(), 600, [0u8; 32], IntentClass::Standard)
                .expect("mint")
        })
        .collect();
    let mut inbox = harness
        .iac
        .mailbox()
        .register_spirit("witness")
        .expect("register");
    for n in 0..frames {
        harness
            .iac
            .deliver(frame(n as u64 + 1))
            .await
            .expect("in-flight frame");
    }
    let outcome = harness
        .orchestrator
        .upgrade("witness", &successor, UpgradePolicy::HotSwap)
        .await;
    let tokens_kept = held
        .iter()
        .filter(|t| {
            harness
                .capability
                .verify_and_audit(t, [0u8; 32], SandboxTier(0))
                .is_ok()
        })
        .count() as u32;
    let mut frames_kept = 0;
    while matches!(inbox.try_recv(), Ok(Some(_))) {
        frames_kept += 1;
    }
    Swap {
        tokens_held: tokens,
        tokens_kept,
        frames_held: frames,
        frames_kept,
        schema_before: BEFORE_SCHEMA,
        schema_after,
        migrator_declared: migrates == Some("0.1.0"),
        accepted: matches!(outcome, Ok(report) if report.outcome == UpgradeOutcome::Completed),
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
            let (tokens, frames) = (1 + (next() % 3) as u32, (next() % 3) as u32);
            runtime.block_on(trial(n % VARIANTS.len(), tokens, frames))
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
        if !obs.swaps.iter().any(|s| s.accepted) || obs.swaps.iter().all(|s| s.accepted) {
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
fn i6_a_swap_keeps_tokens_and_frames_and_accepts_only_a_compatible_schema() {
    let corpus = include_str!("fixtures/invariants/i6_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
