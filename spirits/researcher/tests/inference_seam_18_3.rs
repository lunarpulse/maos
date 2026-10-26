#![forbid(unsafe_code)]

//! Story 18-3 — the Researcher's bound pass (AC1.2, AC4.1, AC4.3, AC5;
//! D-18-3-01/02/03/05/09 A/B), driven through the real `on_idle` hook.
//!
//! - A bound pass mints ONCE (D-18-3-03 A), names the bound provider in every
//!   request (D-18-3-02 B) and consumes exactly one completion per claim.
//! - An unbound binding, any `InferenceError` and (bound) a failed MCP survey
//!   refuse the pass typed: no `last_output`, no scalar, no distillate
//!   (D-18-3-05 A). Unbound, an MCP failure stays a logged no-op.
//! - With no MCP port the bound pass surveys the embedded seed, journaled once
//!   through the root's sink (D-18-3-01 A); its contradiction writes
//!   `methodology_conflict` 0.8 (> the manifest's 0.7) and its distillate
//!   resolves through the kernel `DistillateWriter` (I11).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::halt::HaltReceipt;
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::{EpistemicScalarPort, InferencePort};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_spirit_sdk::spirit_test::SpiritTest;
use researcher::{
    __maos_spirit_vtable_Researcher, declared_provider, seed_claims, synthesis_prompt,
    ClaimPayload, FetchedClaim, RecalledFrame, Researcher, ResearcherError,
    ResearcherInferenceBinding, ResearcherMcpError, ResearcherMcpPort, SEED_CLAIM_INTENT,
};

const PID: u32 = 18;

/// Records every request; answers `text` or fails with `error`.
struct Port {
    seen: Mutex<Vec<InferenceRequest>>,
    error: Option<fn() -> InferenceError>,
}

impl Port {
    fn answering() -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            error: None,
        })
    }
    fn failing(error: fn() -> InferenceError) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            error: Some(error),
        })
    }
}

impl InferencePort for Port {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.seen.lock().unwrap().push(req);
        if let Some(error) = self.error {
            return Err(error());
        }
        Ok(InferenceResponse {
            text: "a model digest sentence".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "stub".into(),
                endpoint_url: "stub://".into(),
                model_id: None,
            },
        })
    }
}

#[derive(Default)]
struct Scalars(Mutex<Vec<(String, f64)>>);

impl EpistemicScalarPort for Scalars {
    fn write_scalar(
        &self,
        _pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        _derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        assert_eq!(spirit_id, researcher::RESEARCHER_SPIRIT_ID);
        self.0.lock().unwrap().push((tag.into(), value));
        Ok(None)
    }
}

/// A binding for `provider` whose per-pass mint is counted.
fn binding(provider: &str, mints: &Arc<AtomicUsize>) -> ResearcherInferenceBinding {
    let mints = Arc::clone(mints);
    ResearcherInferenceBinding::per_pass(
        provider.into(),
        Arc::new(move |pid, _provider| {
            mints.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityToken::new(TokenId([3; 16]), pid, 0, [0; 64]))
        }),
        Arc::new(|| Some(PID)),
    )
}

fn bound(r: Researcher, port: Arc<Port>, b: Option<ResearcherInferenceBinding>) -> Researcher {
    r.with_deferred_inference_port(port, Arc::new(Mutex::new(b)))
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

fn idle(researcher: &Researcher) {
    let vtable = __maos_spirit_vtable_Researcher();
    let mut harness = SpiritTest::new(researcher, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    assert_eq!(harness.run().base.hooks_fired.get("on_idle"), Some(&1));
}

#[test]
fn a_bound_pass_mints_once_names_its_provider_and_consumes_one_completion_per_claim() {
    let (port, mints) = (Port::answering(), Arc::new(AtomicUsize::new(0)));
    let claims = seed_claims();
    let r = bound(
        Researcher::with_frames(frames(&claims)),
        Arc::clone(&port),
        Some(binding("openai", &mints)),
    );
    idle(&r);
    assert!(r.take_pass_error().is_none());
    let out = r.last_output().expect("the bound pass surveyed");
    assert_eq!(out.findings.len(), claims.len());
    assert!(out
        .findings
        .iter()
        .all(|f| f.statement == "a model digest sentence"));
    assert_eq!(r.completions_consumed(), claims.len());
    assert_eq!(mints.load(Ordering::SeqCst), 1, "one mint per pass");
    let seen = port.seen.lock().unwrap();
    for (req, claim) in seen.iter().zip(&claims) {
        assert_eq!(req.provider_id.as_deref(), Some("openai"));
        assert_eq!(req.spirit_pid, PID);
        assert_eq!(req.prompt, synthesis_prompt(&claim.statement));
    }
    drop(seen);
    idle(&r);
    assert_eq!(mints.load(Ordering::SeqCst), 2, "the next pass mints again");
}

#[test]
fn an_unbound_binding_refuses_the_pass_typed() {
    let r = bound(
        Researcher::with_frames(frames(&seed_claims())),
        Port::answering(),
        None,
    );
    idle(&r);
    assert!(matches!(
        r.take_pass_error(),
        Some(ResearcherError::Unbound(_))
    ));
    assert!(r.last_output().is_none());
}

#[test]
fn an_inference_error_refuses_the_pass_with_no_output_scalar_or_distillate() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_03));
    let scalars = Arc::new(Scalars::default());
    let r = bound(
        Researcher::with_frames(frames(&seed_claims())),
        Port::failing(|| InferenceError::MalformedResponse("cassette replay: exhausted".into())),
        Some(binding("anthropic", &Arc::new(AtomicUsize::new(0)))),
    )
    .with_scalar_port(Arc::clone(&scalars) as Arc<dyn EpistemicScalarPort>)
    .with_distillation_port(Arc::new(DistillateWriter::new(
        Arc::clone(&tl),
        Arc::new(0u8),
    )));
    idle(&r);
    let err = r.take_pass_error().expect("refused");
    assert!(matches!(err, ResearcherError::Inference(_)), "{err:?}");
    assert!(err.to_string().contains("exhausted"), "{err}");
    assert!(r.last_output().is_none());
    assert!(
        scalars.0.lock().unwrap().is_empty(),
        "no scalar on a refusal"
    );
    assert_eq!(r.completions_consumed(), 0);
}

struct FailingMcp;
impl ResearcherMcpPort for FailingMcp {
    fn survey_literature(&self, _q: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        Err(ResearcherMcpError::NoResults)
    }
}

#[test]
fn a_bound_mcp_failure_is_a_typed_refusal_and_unbound_stays_a_no_op() {
    let mcp = || Researcher::new().with_mcp_port(Arc::new(FailingMcp));
    let r = bound(
        mcp(),
        Port::answering(),
        Some(binding("anthropic", &Arc::new(AtomicUsize::new(0)))),
    );
    idle(&r);
    assert!(matches!(
        r.take_pass_error(),
        Some(ResearcherError::McpSurvey(ResearcherMcpError::NoResults))
    ));
    let unbound = mcp();
    idle(&unbound);
    assert!(unbound.take_pass_error().is_none());
    assert!(unbound.last_output().is_none());
}

/// Review EC-8 — a bound pass that surveys no claims refuses typed before
/// any scalar or distillate is written.
#[test]
fn a_zero_claim_bound_pass_refuses_typed_and_writes_no_scalar() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_03_08));
    let scalars = Arc::new(Scalars::default());
    let r = bound(
        Researcher::with_frames(Vec::new()),
        Port::answering(),
        Some(binding("anthropic", &Arc::new(AtomicUsize::new(0)))),
    )
    .with_scalar_port(Arc::clone(&scalars) as Arc<dyn EpistemicScalarPort>)
    .with_distillation_port(Arc::new(DistillateWriter::new(
        Arc::clone(&tl),
        Arc::new(0u8),
    )));
    idle(&r);
    let err = r.take_pass_error().expect("refused");
    assert!(matches!(err, ResearcherError::NoClaims), "{err:?}");
    assert!(r.last_output().is_none());
    assert!(
        scalars.0.lock().unwrap().is_empty(),
        "no scalar on a zero-claim pass"
    );
}

/// Review ruling D-18-3-R2 — a bound pass with no MCP port and no seed journal
/// (the `--live` host) refuses typed; it never surveys the seed claims.
#[test]
fn a_bound_pass_without_mcp_or_seed_journal_refuses_unconfigured() {
    let r = bound(
        Researcher::new(),
        Port::answering(),
        Some(binding("anthropic", &Arc::new(AtomicUsize::new(0)))),
    );
    idle(&r);
    assert!(matches!(
        r.take_pass_error(),
        Some(ResearcherError::McpSurvey(ResearcherMcpError::Unconfigured))
    ));
    assert_eq!(r.completions_consumed(), 0);
}

#[test]
fn the_seed_is_journaled_once_halts_on_its_contradiction_and_distills_through_i11() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_03_01));
    let journaled = Arc::new(Mutex::new(Vec::new()));
    let (sink_tl, sink_ids) = (Arc::clone(&tl), Arc::clone(&journaled));
    let scalars = Arc::new(Scalars::default());
    let r = bound(
        Researcher::new(),
        Port::answering(),
        Some(binding("anthropic", &Arc::new(AtomicUsize::new(0)))),
    )
    .with_seed_sink(Arc::new(move |pid, payload| {
        let _ = sink_tl.insert_frame_event(
            FrameKind::TelemetryEvent,
            pid,
            None,
            SEED_CLAIM_INTENT,
            payload,
            FrameOrigin::SpiritAuto,
        );
        let id = sink_tl.last_frame_id();
        sink_ids.lock().unwrap().push(id);
        Ok(id)
    }))
    .with_scalar_port(Arc::clone(&scalars) as Arc<dyn EpistemicScalarPort>)
    .with_distillation_port(Arc::new(DistillateWriter::new(
        Arc::clone(&tl),
        Arc::new(0u8),
    )));
    idle(&r);
    assert!(r.take_pass_error().is_none());
    idle(&r);
    assert!(r.take_pass_error().is_none());
    let ids = journaled.lock().unwrap().clone();
    assert_eq!(
        ids.len(),
        seed_claims().len(),
        "journaled once, not per pass"
    );
    let out = r.last_output().unwrap();
    let cited: Vec<_> = out
        .findings
        .iter()
        .map(|f| researcher::decode_frame_id_hex(&f.source_log_ref).unwrap())
        .collect();
    assert_eq!(cited, ids, "every finding cites its journaled seed frame");
    let scalars = scalars.0.lock().unwrap().clone();
    assert_eq!(scalars.len(), 2, "one scalar per surveyed pass");
    assert_eq!(scalars[0].0, "methodology_conflict");
    assert!(scalars[0].1 > 0.7, "the seed contradiction crosses 0.7");
    assert_eq!(r.completions_consumed(), 2 * seed_claims().len());
}

#[test]
fn the_declared_provider_rule_prefers_the_first_registered_and_refuses_none() {
    let declared = ["anthropic".to_owned(), "openai".to_owned()];
    let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        declared_provider(&declared, &ids(&["ollama", "openai", "anthropic"])).unwrap(),
        "anthropic"
    );
    assert_eq!(
        declared_provider(&declared, &ids(&["openai", "ollama"])).unwrap(),
        "openai"
    );
    let refused = declared_provider(&declared, &ids(&["ollama"])).unwrap_err();
    assert_eq!(
        refused,
        "researcher inference: no inference provider is configured for its declared providers \
         `anthropic`, `openai`"
    );
    assert!(declared_provider(&[], &ids(&["anthropic"])).is_err());
}

#[test]
fn the_deterministic_survey_is_unchanged_by_the_unbound_infer_path() {
    let fs = frames(&seed_claims());
    let r = Researcher::new();
    assert_eq!(r.infer_survey(&fs).unwrap(), r.survey(&fs));
    assert!(r
        .survey(&fs)
        .findings
        .iter()
        .all(|f| f.statement != "a model digest sentence"));
}

/// The root-bound cassette pins each seed synthesis prompt and the following
/// model-authored skill prompt. Library passes without a skill port still
/// consume only the first three entries.
#[test]
fn the_exit_line_3_cassette_pins_real_prompt_hashes_and_no_wildcard() {
    use sha2::{Digest, Sha256};
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/maos-journey-test/cassettes/j-researcher/survey-distill.json"
    );
    let cassette: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(cassette["provenance"], "seed");
    let entries = cassette["entries"].as_array().unwrap();
    let claims = seed_claims();
    assert_eq!(
        entries.len(),
        claims.len() + 1,
        "survey followed by skill generation"
    );
    for (entry, claim) in entries.iter().zip(&claims) {
        let prompt = synthesis_prompt(&claim.statement);
        let hash = format!("{:x}", Sha256::digest(prompt.as_bytes()));
        assert_ne!(entry["prompt_sha256"], "0".repeat(64), "wildcard hash");
        assert_eq!(entry["prompt_sha256"], hash.as_str(), "{}", claim.claim_id);
        assert_eq!(entry["prompt_len"], prompt.len());
    }
    let mut output = Researcher::new().survey(&frames(&claims));
    for (finding, entry) in output.findings.iter_mut().zip(entries) {
        finding.statement = entry["response"]["text"].as_str().unwrap().into();
    }
    let prompt = researcher::skill_author::authoring_prompt(&output, None);
    let author = &entries[claims.len()];
    assert_eq!(
        author["prompt_sha256"],
        format!("{:x}", Sha256::digest(prompt.as_bytes()))
    );
    assert_eq!(author["prompt_len"], prompt.len());
    maos_skill::parse_skill(author["response"]["text"].as_str().unwrap()).unwrap();
}
