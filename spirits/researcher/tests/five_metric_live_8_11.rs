//! Story 8.11 · AC4 (FORK A) — score Researcher's ACTUAL digest with the
//! 4-point assertion contract, WITHOUT reopening Decision D.
//!
//! The hermetic five-metric gate (`maos-eval::distillate_five_metrics_floor`)
//! is UNCHANGED — recall/faithfulness/hedge stay corpus-annotated (Decision D:
//! noise-limited, un-CI-able). What this file adds (party-mode 2026-06-08
//! RATIFIED):
//!
//! - **Point 1 — hard-gate the two DETERMINISTIC properties on the LIVE digest:**
//!   traceability=100% (non-empty resolving `source_log_ref`) + secret-leakage=0%
//!   (real `CorpusBackedRedactionPolicy`). These are properties of the bytes, not
//!   the model — a live digest that cites nothing or leaks a token is fail-closed.
//! - **Point 4 — provider-independence:** the two hard gates pass IDENTICALLY on
//!   the default deterministic path AND the live (Inference-Port) path. If they
//!   diverge, the seam is wrong — caught here, cheaply.
//! - **Vacuous-green guards (Amelia):** a planted 32-hex secret in the LIVE
//!   digest (the model's returned text) AND in a deterministic digest MUST fire
//!   the gate — proving it inspects content on BOTH paths, not passes vacuously.
//!
//! The three SOFT metrics (recall/faithfulness/hedge) on the live digest are
//! REPORTED evidence on the `--live`/Tier-2 nightly path (delta-vs-annotated,
//! N/seed/judge-id, direction-only alarm) — NOT a hermetic CI floor. That pass
//! needs a real judge/replicator LLM and is a release-gate checklist item read by
//! a named owner before Epic 8 Completion (see the story Dev Agent Record). It is
//! intentionally NOT asserted here.

use std::borrow::Cow;
use std::sync::Arc;

use researcher::{ClaimPayload, RecalledFrame, Researcher};

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::LogRecallFilter;
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::{InferencePort, LogRecallPort};
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0x_8B11_5C04;

/// A stub Inference Port returning a fixed text (the "live" digest content).
struct StubProvider {
    text: String,
}
impl InferencePort for StubProvider {
    fn complete(&self, _req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        Ok(InferenceResponse {
            text: self.text.clone(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 8,
                output_tokens: 8,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "stub".into(),
                endpoint_url: "stub://local".into(),
                model_id: Some("stub-1".into()),
            },
        })
    }
}

fn token() -> CapabilityToken {
    CapabilityToken::new(TokenId([7u8; 16]), 10, 0, [0u8; 64])
}

fn seed(tl: &Arc<TransparencyLogAdapter>) {
    for (id, statement) in [
        ("c0", "the effect is likely present in trial A"),
        ("c1", "the mechanism is plausibly mediated by X"),
    ] {
        let claim = ClaimPayload {
            claim_id: id.into(),
            statement: statement.into(),
            topic: "fusion".into(),
            methodology_strength: 0.9,
            confidence: 0.92,
            load_bearing: true,
            polarity: true,
            hedges: vec!["likely".into()],
        };
        let _ = tl.insert_frame_event(
            FrameKind::InferenceCall,
            10,
            None,
            "inform",
            &serde_json::to_vec(&claim).unwrap(),
            FrameOrigin::SpiritAuto,
        );
    }
}

fn assert_traceable(survey: &researcher::SurveyOutput, adapter: &LogRecallAdapter) {
    assert!(!survey.findings.is_empty());
    for f in &survey.findings {
        assert!(!f.source_log_ref.is_empty(), "cite must be non-empty");
        let id = researcher::decode_frame_id_hex(&f.source_log_ref).expect("cite decodes");
        assert!(
            adapter.fetch(10, id).is_ok(),
            "every cite must resolve (traceability=100%) — {}",
            f.source_log_ref
        );
    }
}

fn redaction_fires(researcher: &Researcher, survey: &researcher::SurveyOutput) -> bool {
    let request = researcher.to_distillation_request(survey, 1).unwrap();
    let payload_bytes = serde_json::to_vec(&request.digest_payload).unwrap();
    let policy = CorpusBackedRedactionPolicy::new();
    matches!(policy.redact(&payload_bytes), Cow::Owned(b) if b != payload_bytes)
}

/// Point 1 + Point 4 — the two deterministic hard gates pass IDENTICALLY on the
/// deterministic path AND the live (Inference-Port) path. Provider-independent.
#[test]
fn traceability_and_leakage_hard_gate_pass_on_both_fixture_and_live_paths() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    seed(&tl);
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let frames: Vec<RecalledFrame> = Researcher::new()
        .walk(&adapter, 10, LogRecallFilter::default())
        .unwrap();

    // Deterministic path (default).
    let det = Researcher::new();
    let det_survey = det.survey(&frames);
    assert_traceable(&det_survey, &adapter);
    assert!(
        !redaction_fires(&det, &det_survey),
        "deterministic digest must have zero secret leakage"
    );

    // Live path (clean stub text) — cites unchanged, so traceability holds and a
    // clean model output leaks nothing. Identical verdict to the deterministic path.
    let live = Researcher::new().with_inference_port(
        Arc::new(StubProvider {
            text: "a clean one-sentence digest of the claim".into(),
        }) as Arc<dyn InferencePort + Send + Sync>,
        token(),
        10,
    );
    let live_survey = live.survey(&frames);
    assert_traceable(&live_survey, &adapter);
    assert!(
        !redaction_fires(&live, &live_survey),
        "a clean live digest must ALSO have zero secret leakage (provider-independence)"
    );
}

/// Vacuous-green guard on the LIVE path: a model that returns a 32-hex secret
/// MUST trip the secret-leakage hard gate on the live digest (proves the gate
/// inspects LIVE content, not just the fixture).
#[test]
fn digest_secret_leakage_hard_gate_fires_on_live_digest() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0x1));
    seed(&tl);
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let frames = Researcher::new()
        .walk(&adapter, 10, LogRecallFilter::default())
        .unwrap();

    let live = Researcher::new().with_inference_port(
        Arc::new(StubProvider {
            text: "leaked integration key sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4".into(),
        }) as Arc<dyn InferencePort + Send + Sync>,
        token(),
        10,
    );
    let survey = live.survey(&frames);
    assert!(
        redaction_fires(&live, &survey),
        "a live digest carrying a 32-hex secret MUST fire the redaction hard gate"
    );
}

/// Vacuous-green guard on the deterministic/fixture path: a planted 32-hex secret
/// in a digest MUST fire the gate (proving the hermetic hard-gate inspects content
/// and does not pass vacuously — it only "bites on --live" otherwise).
#[test]
fn digest_secret_leakage_hard_gate_fires_on_fixture() {
    use maos_domain::distillation::DigestPayload;
    use researcher::{Finding, SurveyOutput};
    use std::collections::BTreeMap;

    let secret = "sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4";
    let out = SurveyOutput {
        findings: vec![Finding {
            claim_id: "leak".into(),
            statement: format!("the source embeds {secret}"),
            confidence: 0.9,
            hedges: vec![],
            source_log_ref: researcher::encode_frame_id_hex(&[0x11; 16]),
        }],
        open_questions: vec![],
        confidence_map: BTreeMap::new(),
        bibliography: vec![],
        scalars: BTreeMap::new(),
    };
    let payload = DigestPayload::Json(serde_json::to_value(&out).unwrap());
    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let policy = CorpusBackedRedactionPolicy::new();
    assert!(
        matches!(policy.redact(&payload_bytes), Cow::Owned(b) if b != payload_bytes),
        "a planted 32-hex secret in a fixture digest MUST fire redaction (no vacuous green)"
    );
}
