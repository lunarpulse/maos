//! Story 7.5b AC8 — `smoke-onb-1-7-5b`: ONE end-to-end dry-run self-trial that
//! proves the NFR-Onb-1 harness is correctly wired.
//!
//! This is a **wiring proof, explicitly NOT the N=12 gate.** It scaffolds a
//! single Spirit via the Story 2.3 `#[spirit]` macro, runs it through
//! `LocalRunner`, scores it against the resolved corpus (the fixture stand-in at
//! v0.3), emits ONE `outcomes.jsonl` row, and runs the cohort gate evaluator on
//! that N=1 sample. The test asserts **N=1 ∧ provisional** so a green self-trial
//! can never be mistaken for the real gate.

use std::time::Instant;

use maos_eval::onboarding_gate_corpus::{
    evaluate_cohort, resolve_corpus, score_candidate, sha256_hex, validate_corpus_size,
    CandidateInput, CorpusSource, OnboardingCorpus, ResolvedCorpus, CORPUS_SCENARIO_COUNT,
    FIXTURE_CORPUS_REL,
};
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};
use maos_spirit_sdk::{spirit, Ctx, Spirit};

/// A minimal first-Spirit scaffolded exactly like `examples/example-spirit`
/// (the Story 2.3 template output) — the candidate under self-trial.
pub struct SelfTrialSpirit;

#[spirit]
impl SelfTrialSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // A first Spirit's idle behavior would go here.
    }
}

#[test]
fn smoke_onb_1_self_trial_seam_closed_butler_nonprovisional() {
    // Story 8.1 update of the 7.5b `smoke-onb-1-7-5b` self-trial: now that the
    // canonical Butler corpus exists, the resolver prefers it and the live
    // self-trial is NON-provisional (AC4 "any new Butler-sourced self-trial is
    // non-provisional"). The 7.5b FIXTURE dry-run remains valid + provisional —
    // asserted directly at the end of this test (AC4 "the 7.5b self-trial
    // artifacts remain valid as the documented dry-run"). This is the
    // documented maos-eval TEST edit (not a public-surface edit) for AC4.
    //
    // Integration tests run with CWD = crate dir (crates/maos-eval); the
    // workspace root is two levels up.
    let workspace_root = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // 1. Resolve the corpus via the seam. Butler PRESENT (Story 8.1) → butler.
    let resolved = resolve_corpus(&workspace_root).expect("resolve corpus");
    assert_eq!(
        resolved.source,
        CorpusSource::Butler,
        "Story 8.1 landed the Butler corpus → the resolver must prefer it (seam closed)"
    );
    eprintln!(
        "smoke-onb-1: scoring against corpus_source={} sha256={} path={}",
        resolved.source.as_str(),
        resolved.sha256,
        resolved.path.display()
    );

    let corpus = OnboardingCorpus::load_jsonl(&resolved.path).expect("load corpus");
    validate_corpus_size(&corpus).expect("corpus size validation");
    assert_eq!(corpus.scenarios.len(), CORPUS_SCENARIO_COUNT);

    // 2. Run the candidate Spirit through LocalRunner (it compiled against the
    //    published ABI by virtue of building this test).
    let spirit = SelfTrialSpirit;
    let vtable = __maos_spirit_vtable_SelfTrialSpirit();
    let mut fixture = LocalRunnerFixture::default();
    fixture.invoke_on_load = true;
    fixture.invoke_on_start = true;
    fixture.invoke_on_idle = true;
    let start = Instant::now();
    let report = LocalRunner::run(&spirit, &vtable, &fixture);
    let elapsed_min = start.elapsed().as_secs_f64() / 60.0;
    assert_eq!(
        report.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once"
    );

    // 3. Score the candidate. observations=None → the harness uses the fixture's
    //    baked observed_halt (the v0.3 stand-in; Story 8.1 supplies real bus
    //    observations here).
    let outcome = score_candidate(
        &corpus,
        &resolved,
        &CandidateInput {
            participant_id: "self-trial-7-5b".into(),
            compiles_against_abi: true,
            time_to_success_min: elapsed_min,
            within_window: true,
        },
        None,
    );

    // 4. Emit ONE outcomes.jsonl row (to a temp dir — live outcomes are private).
    let tmp = tempfile::tempdir().expect("tempdir");
    let outcomes_path = tmp.path().join("outcomes.jsonl");
    let row = match serde_json::to_string(&outcome) {
        Ok(r) => r,
        Err(e) => panic!("serialize outcome: {e}"),
    };
    std::fs::write(&outcomes_path, format!("{row}\n")).expect("write outcomes.jsonl");
    eprintln!("smoke-onb-1-7-5b: emitted outcome → {}", row);

    // The self-trial candidate succeeds against the Butler corpus (recall 1.0,
    // precision 1.0) and — because the source is now Butler — is NON-provisional.
    assert!(outcome.succeed, "butler self-trial candidate should succeed");
    assert!(
        !outcome.provisional,
        "Butler-sourced outcome must NOT be provisional (seam closed)"
    );
    assert_eq!(outcome.corpus_source, "butler");

    // 5. Run the gate evaluator on the N=1 sample. It must NOT panic; with a
    //    Butler source the verdict is non-provisional, and N=1 < 10 → it still
    //    necessarily FAILS the success-count floor, so a green self-trial can
    //    never pose as the real N=12 gate.
    let outcomes = vec![outcome];
    let verdict = evaluate_cohort(&outcomes);

    assert_eq!(outcomes.len(), 1, "self-trial is N=1, NOT the N=12 gate");
    assert!(!verdict.provisional, "Butler-sourced verdict is non-provisional");
    assert!(
        !verdict.passed,
        "an N=1 sample can never pass the ≥10/12 cohort floor — guards against \
         the self-trial masquerading as the live gate"
    );
    assert_eq!(verdict.cohort_size, 1);
    assert_eq!(verdict.success_count, 1);
    eprintln!("smoke-onb-1: N=1 non-provisional verdict = {:?}", verdict);

    // 6. AC4 — the 7.5b FIXTURE dry-run remains a valid, provisional artifact.
    //    Score the same candidate directly against the fixture (bypassing the
    //    resolver, which now prefers Butler) and assert it is still provisional.
    let fixture_path = workspace_root.join(FIXTURE_CORPUS_REL);
    let fixture_bytes = std::fs::read(&fixture_path).expect("read fixture");
    let fixture_resolved = ResolvedCorpus {
        source: CorpusSource::Fixture,
        path: fixture_path.clone(),
        sha256: sha256_hex(&fixture_bytes),
    };
    let fixture_corpus = OnboardingCorpus::load_jsonl(&fixture_path).expect("load fixture");
    let fixture_outcome = score_candidate(
        &fixture_corpus,
        &fixture_resolved,
        &CandidateInput {
            participant_id: "self-trial-fixture-dry-run".into(),
            compiles_against_abi: true,
            time_to_success_min: elapsed_min,
            within_window: true,
        },
        None,
    );
    assert!(
        fixture_outcome.provisional,
        "the documented 7.5b fixture dry-run must remain provisional"
    );
    assert_eq!(fixture_outcome.corpus_source, "fixture");
    eprintln!("smoke-onb-1: fixture dry-run remains provisional (7.5b artifact preserved)");
}
