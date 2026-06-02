//! AC2 (halt production) + AC3 (corpus floors) + AC4 (seam closure).
//!
//! The 30-scenario corpus is **self-validating**: each row's `observed_halt`
//! is re-derived here by running Butler's real `assess()` proxy through the
//! real kernel `WorkingMemoryOrchestrator::process_scalar_write` (the kernel
//! does the universal-arithmetic comparison against Butler's manifest
//! `[epistemic_policy]` and produces the `HaltReceipt`). We assert the kernel
//! result equals the baked `observed_halt`, then score the candidate and prove
//! `resolve_corpus` now returns `Butler` with `provisional = false`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use butler::{Butler, ScenarioInput};

use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::EpistemicPolicySection;
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

use maos_eval::onboarding_gate_corpus::{
    resolve_corpus, score_candidate, validate_corpus_size, CandidateInput, CorpusSource,
    OnboardingCorpus, BUTLER_CORPUS_REL,
};

const MANIFEST: &str = include_str!("../manifest.toml");
const BOOT_NONCE: u64 = 0x_B17_1E5;

// ── workspace + corpus paths ────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/spirits/butler
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn corpus_path() -> PathBuf {
    workspace_root().join(BUTLER_CORPUS_REL)
}

// ── Butler's manifest epistemic policy → kernel EpistemicPolicySection ───────

fn butler_policy() -> EpistemicPolicySection {
    let v: toml::Value = toml::from_str(MANIFEST).unwrap();
    let ep = v.get("epistemic_policy").expect("[epistemic_policy] present");
    let ep_str = toml::to_string(ep).unwrap();
    EpistemicPolicySection::from_toml_str(&ep_str).expect("Butler [epistemic_policy] must parse")
}

// ── a fresh, isolated kernel world per scenario ──────────────────────────────

struct World {
    tl: TransparencyLogAdapter,
    journal: JournalAdapter,
    orchestrator: WorkingMemoryOrchestrator,
    _tmp: tempfile::TempDir,
}

fn make_world() -> World {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy_table = Arc::new(PolicyTable::new());
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        BOOT_NONCE,
        policy_table,
        audit_tx,
        quota,
        Arc::new(WorkingMemoryStore::new()),
        telemetry,
    ));
    let orchestrator =
        WorkingMemoryOrchestrator::new(Arc::clone(&capability), Arc::new(HaltRegistry::new()));

    let tmp = tempfile::TempDir::new().unwrap();
    let journal = JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap();
    let tl = TransparencyLogAdapter::open_in_memory(BOOT_NONCE);
    World {
        tl,
        journal,
        orchestrator,
        _tmp: tmp,
    }
}

/// Run one scenario through Butler + the kernel; returns whether a halt fired.
fn kernel_observed_halt(scenario: &ScenarioInput, policy: &EpistemicPolicySection) -> bool {
    let butler = Butler::new();
    let assessment = butler.assess(scenario);
    let (tag, value, derived_from) = assessment.primary_scalar();
    let world = make_world();
    world
        .orchestrator
        .process_scalar_write(
            &world.tl,
            &world.journal,
            0,
            "butler",
            BOOT_NONCE,
            tag,
            value,
            &derived_from,
            policy,
        )
        .expect("process_scalar_write must succeed")
        .is_some()
}

// ── the corpus rows (with the non-scored `input` Decision D carries) ─────────

#[derive(serde::Deserialize)]
struct CorpusRow {
    scenario_id: String,
    // Present for completeness of the row contract; scored by maos-eval, not here.
    #[allow(dead_code)]
    calendar_conflict: bool,
    #[allow(dead_code)]
    expected_halt: bool,
    observed_halt: bool,
    input: ScenarioInput,
}

fn load_rows() -> Vec<CorpusRow> {
    let content = std::fs::read_to_string(corpus_path()).expect("corpus file present");
    content
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            serde_json::from_str::<CorpusRow>(l).unwrap_or_else(|e| {
                panic!("corpus row {} (1-based) parses: {e}", i + 1)
            })
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn corpus_observed_halts_are_self_validating_against_the_real_kernel() {
    let policy = butler_policy();
    let rows = load_rows();
    assert_eq!(rows.len(), 30, "exactly 30 scenarios");

    for row in &rows {
        let observed = kernel_observed_halt(&row.input, &policy);
        assert_eq!(
            observed, row.observed_halt,
            "scenario {} — kernel-observed halt ({observed}) must match the baked \
             observed_halt ({}); Butler's behavior drifted from the corpus",
            row.scenario_id, row.observed_halt
        );
    }
}

#[test]
fn calendar_conflict_produces_journaled_halt_receipt() {
    // AC2 final bullet: a calendar conflict → uncertainty scalar → epistemic
    // predicate fires a halt → EpistemicHaltPayload journaled + HaltReceipt.
    let policy = butler_policy();
    let rows = load_rows();
    let cc = rows
        .iter()
        .find(|r| r.scenario_id == "cc01")
        .expect("cc01 present");

    let butler = Butler::new();
    let (tag, value, derived) = butler.assess(&cc.input).primary_scalar();
    assert_eq!(tag, "belief_variance");

    let world = make_world();
    let receipt = world
        .orchestrator
        .process_scalar_write(
            &world.tl,
            &world.journal,
            7,
            "butler",
            BOOT_NONCE,
            tag,
            value,
            &derived,
            &policy,
        )
        .unwrap()
        .expect("a calendar conflict must produce a HaltReceipt");

    assert_eq!(receipt.spirit_pid, 7);
    assert!(receipt.terminal_state.is_none(), "invocation-time receipt");

    // EpistemicHalt frame journaled to the Transparency Log.
    let frames = world
        .tl
        .query_frames(FrameFilter {
            spirit_pid: Some(7),
            ..Default::default()
        })
        .unwrap();
    assert!(
        frames
            .iter()
            .any(|f| f.kind == FrameKind::EpistemicHalt && f.spirit_pid == 7),
        "an EpistemicHalt frame must be journaled"
    );

    // Lifecycle Journal records the Halt.
    let last = world.journal.last_event("butler").unwrap();
    assert_eq!(last, maos_domain::invariants::i10::LifecycleEvent::Halt);
}

#[test]
fn preference_drift_fires_the_second_rule() {
    // Exercise the user_preference_drift Below-0.6 rule (df0x scenarios).
    let policy = butler_policy();
    let rows = load_rows();
    let df = rows
        .iter()
        .find(|r| r.scenario_id == "df01")
        .expect("df01 present");
    let butler = Butler::new();
    let (tag, _v, _d) = butler.assess(&df.input).primary_scalar();
    assert_eq!(tag, "user_preference_drift");
    assert!(kernel_observed_halt(&df.input, &policy), "df01 must halt via drift rule");
}

#[test]
fn decision_d_corpus_loads_via_onboarding_loader() {
    // AC3 final bullet / Decision D: OnboardingCorpus::load_jsonl tolerates the
    // extra `input` field, has NO meta line, and is exactly 30 scenarios.
    let corpus = OnboardingCorpus::load_jsonl(&corpus_path()).expect("corpus loads");
    assert!(corpus.meta.is_none(), "real corpus has NO stand_in_for meta line");
    assert_eq!(corpus.scenarios.len(), 30);
    validate_corpus_size(&corpus).expect("exactly 30 scenarios");
}

#[test]
fn ac3_ac4_corpus_scores_above_floors_and_closes_the_seam() {
    let policy = butler_policy();
    let rows = load_rows();

    // Bus-observed halts (Story 8.1 real path): the seam's `Some(map)` branch.
    let mut observations: BTreeMap<String, bool> = BTreeMap::new();
    for row in &rows {
        observations.insert(row.scenario_id.clone(), kernel_observed_halt(&row.input, &policy));
    }

    // AC4: the resolver now prefers the Butler corpus (file exists) → not provisional.
    let resolved = resolve_corpus(&workspace_root()).expect("resolve");
    assert_eq!(resolved.source, CorpusSource::Butler, "seam flipped Fixture → Butler");
    assert!(!resolved.source.is_provisional());

    let corpus = OnboardingCorpus::load_jsonl(&resolved.path).expect("load resolved");
    let input = CandidateInput {
        participant_id: "butler-self-trial".into(),
        compiles_against_abi: true,
        time_to_success_min: 18.0,
        within_window: true,
    };
    let outcome = score_candidate(&corpus, &resolved, &input, Some(&observations));

    assert_eq!(outcome.corpus_source, "butler");
    assert!(!outcome.provisional, "Butler-sourced ⇒ provisional:false (seam closed)");
    assert!(outcome.corpus_pass, "a decision for all 30 scenarios");
    assert!(
        outcome.halt_recall_calendar_conflict >= 0.90,
        "halt-recall {} must be ≥ 0.90",
        outcome.halt_recall_calendar_conflict
    );
    assert!(
        outcome.halt_precision_overall >= 0.85,
        "halt-precision {} must be ≥ 0.85",
        outcome.halt_precision_overall
    );
    // bmad-eval baseline ≥0.85: both quality metrics clear the bar AND the
    // candidate succeeds end-to-end.
    assert!(outcome.succeed, "Butler succeeds against its own corpus");
}
