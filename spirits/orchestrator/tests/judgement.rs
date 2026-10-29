#![forbid(unsafe_code)]

//! Story 19-1 (AC3, D-19-1-12 A) — the Orchestrator's completion → scalar
//! `dispatch_ambiguity` judgement over its deferred Inference Port, driven by
//! a deterministic fake port. The port serves only the judgement: judging
//! never builds, records or changes a dispatch. Every failure is typed; none
//! falls back to a default value.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::halt::{HaltId, HaltReceipt};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::context_budget::{
    ContextBudgetPort, ContextLevel, ContextReportError, ContextStatus,
};
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::{EpistemicScalarPort, InferencePort};
use orchestrator::{
    ambiguity_prompt, parse_ambiguity, split_epic, EpicSection, Orchestrator,
    OrchestratorInferenceBinding, OrchestratorInferenceError, ProviderMint,
    SCALAR_TAG_DISPATCH_AMBIGUITY,
};

const PID: u32 = 19;
const PROVIDER: &str = "anthropic.claude-haiku-4-5-20251001";

/// Records every request; answers `text` or fails with `error`.
struct Port {
    seen: Mutex<Vec<InferenceRequest>>,
    text: &'static str,
    error: Option<fn() -> InferenceError>,
}

impl Port {
    fn answering(text: &'static str) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            text,
            error: None,
        })
    }

    fn failing(error: fn() -> InferenceError) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            text: "",
            error: Some(error),
        })
    }

    fn requests(&self) -> Vec<InferenceRequest> {
        self.seen.lock().unwrap().clone()
    }
}

impl InferencePort for Port {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.seen.lock().unwrap().push(req);
        if let Some(error) = self.error {
            return Err(error());
        }
        Ok(InferenceResponse {
            text: self.text.into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 120,
                output_tokens: 9,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "stub".into(),
                endpoint_url: "stub://".into(),
                model_id: None,
            },
        })
    }
}

/// Records every report against an 8192-token window; `refuse` answers
/// `NotAdmitted`.
#[derive(Default)]
struct Context {
    reports: Mutex<Vec<u32>>,
    refuse: bool,
}

impl ContextBudgetPort for Context {
    fn report_context_used(&self, used_tokens: u32) -> Result<ContextStatus, ContextReportError> {
        self.reports.lock().unwrap().push(used_tokens);
        if self.refuse {
            return Err(ContextReportError::NotAdmitted);
        }
        Ok(ContextStatus {
            used: used_tokens,
            window: 8192,
            level: ContextLevel::Within,
            crossed: None,
        })
    }
}

fn section() -> EpicSection {
    let epic = "### Story 1: parse\nAccept a CSV.\n\n### Story 2: validate\nReject bad rows.\n";
    split_epic(epic, 2).unwrap().remove(1)
}

/// A typed `ProviderMint` (the closure's `&str` stays higher-ranked).
fn provider_mint(
    mint: impl Fn(u32, &str) -> Result<CapabilityToken, String> + Send + Sync + 'static,
) -> ProviderMint {
    Arc::new(mint)
}

/// A per-pass binding at [`PID`] whose mint is counted.
fn binding(mints: &Arc<AtomicUsize>) -> OrchestratorInferenceBinding {
    let mints = Arc::clone(mints);
    OrchestratorInferenceBinding::per_pass(
        PROVIDER.into(),
        provider_mint(move |pid, provider| {
            assert_eq!(provider, PROVIDER, "the mint names the bound provider");
            mints.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityToken::new(TokenId([19; 16]), pid, 0, [0; 64]))
        }),
        Arc::new(|| Some(PID)),
    )
}

fn bound(port: Arc<Port>, binding: Option<OrchestratorInferenceBinding>) -> Orchestrator {
    Orchestrator::new("orchestrator")
        .with_deferred_inference_port(port, Arc::new(Mutex::new(binding)))
}

#[test]
fn a_bound_judgement_mints_once_and_returns_the_dispatch_ambiguity_scalar() {
    let port = Port::answering(r#"{"dispatch_ambiguity": 0.8}"#);
    let mints = Arc::new(AtomicUsize::new(0));
    let context = Arc::new(Context::default());
    let o = bound(Arc::clone(&port), Some(binding(&mints)))
        .with_context_port(Arc::clone(&context) as Arc<dyn ContextBudgetPort>);
    assert!(o.is_inference_bound());

    let section = section();
    let (tag, value, derived_from) = o.judge_ambiguity(&section).expect("bound judgement");
    assert_eq!(tag, SCALAR_TAG_DISPATCH_AMBIGUITY);
    assert_eq!(tag, "dispatch_ambiguity");
    assert_eq!(value, 0.8);
    assert_eq!(derived_from, format!("completion:{PROVIDER}:section-2"));

    assert_eq!(mints.load(Ordering::SeqCst), 1, "one mint per judgement");
    let requests = port.requests();
    assert_eq!(requests.len(), 1, "one completion per judgement");
    let request = &requests[0];
    assert_eq!(request.spirit_pid, PID);
    assert_eq!(request.provider_id.as_deref(), Some(PROVIDER));
    assert!(request.fallback_provider_ids.is_empty());
    assert_eq!(request.options.temperature, Some(0.0));
    assert_eq!(request.prompt, ambiguity_prompt(&section.text));

    assert_eq!(*context.reports.lock().unwrap(), vec![129]);
    assert_eq!(o.last_context().map(|s| s.used), Some(129));
}

#[test]
fn judging_never_builds_or_changes_a_dispatch() {
    let o = bound(
        Port::answering(r#"{"dispatch_ambiguity": 0.95}"#),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    );
    o.judge_ambiguity(&section()).unwrap();
    assert_eq!(o.dispatched_count(), 0);
    assert!(o.last_dispatch().is_none());
    assert!(o.is_safe_point());
}

#[test]
fn the_prompt_is_deterministic_and_carries_the_section_text() {
    let section = section();
    let prompt = ambiguity_prompt(&section.text);
    assert_eq!(prompt, ambiguity_prompt(&section.text));
    assert!(prompt.ends_with("Section:\n### Story 2: validate\nReject bad rows."));
    assert!(prompt.contains(r#"{"dispatch_ambiguity": number in [0,1]}"#));
    assert_ne!(
        prompt,
        ambiguity_prompt("### Story 2: validate\nReject rows.")
    );
}

/// ADR-064 / D-D — unset mode wires no port: the judgement refuses typed.
#[test]
fn an_unwired_orchestrator_refuses_the_judgement() {
    let o = Orchestrator::new("orchestrator");
    assert!(!o.is_inference_bound());
    assert!(matches!(
        o.judge_ambiguity(&section()),
        Err(OrchestratorInferenceError::NotWired)
    ));
}

#[test]
fn an_unbound_or_refused_mint_requests_no_completion() {
    let port = Port::answering(r#"{"dispatch_ambiguity": 0.1}"#);
    let o = bound(Arc::clone(&port), None);
    assert!(matches!(
        o.judge_ambiguity(&section()),
        Err(OrchestratorInferenceError::Unbound(_))
    ));

    let no_pid = OrchestratorInferenceBinding::per_pass(
        PROVIDER.into(),
        provider_mint(|pid, _| Ok(CapabilityToken::new(TokenId([1; 16]), pid, 0, [0; 64]))),
        Arc::new(|| None::<u32>),
    );
    let o = bound(Arc::clone(&port), Some(no_pid));
    match o.judge_ambiguity(&section()) {
        Err(OrchestratorInferenceError::Unbound(reason)) => {
            assert!(reason.contains("no admitted pid"), "{reason}")
        }
        other => panic!("expected Unbound, got {other:?}"),
    }

    let refusing = OrchestratorInferenceBinding::per_pass(
        PROVIDER.into(),
        provider_mint(|_, _| Err("issuance denied".into())),
        Arc::new(|| Some(PID)),
    );
    let o = bound(Arc::clone(&port), Some(refusing));
    match o.judge_ambiguity(&section()) {
        Err(OrchestratorInferenceError::Unbound(reason)) => assert_eq!(reason, "issuance denied"),
        other => panic!("expected Unbound, got {other:?}"),
    }
    assert!(port.requests().is_empty(), "no completion without a token");
}

/// D-19-1-04 A — the root backfills the binding after admission; a clone
/// taken before the backfill judges with it.
#[test]
fn a_clone_judges_with_the_binding_backfilled_after_construction() {
    let slot = Arc::new(Mutex::new(None));
    let loaded = Orchestrator::new("orchestrator").with_deferred_inference_port(
        Port::answering(r#"{"dispatch_ambiguity": 0.3}"#),
        Arc::clone(&slot),
    );
    let emitter = loaded.clone();
    assert!(matches!(
        emitter.judge_ambiguity(&section()),
        Err(OrchestratorInferenceError::Unbound(_))
    ));
    *slot.lock().unwrap() = Some(binding(&Arc::new(AtomicUsize::new(0))));
    assert_eq!(emitter.judge_ambiguity(&section()).unwrap().1, 0.3);
}

#[test]
fn an_inference_error_is_typed_and_reports_no_context() {
    let context = Arc::new(Context::default());
    let o = bound(
        Port::failing(|| InferenceError::MalformedResponse("exhausted".into())),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    )
    .with_context_port(Arc::clone(&context) as Arc<dyn ContextBudgetPort>);
    assert!(matches!(
        o.judge_ambiguity(&section()),
        Err(OrchestratorInferenceError::Inference(
            InferenceError::MalformedResponse(_)
        ))
    ));
    assert!(context.reports.lock().unwrap().is_empty());
    assert!(o.last_context().is_none());
}

#[test]
fn a_refused_context_report_refuses_the_judgement() {
    let o = bound(
        Port::answering(r#"{"dispatch_ambiguity": 0.2}"#),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    )
    .with_context_port(Arc::new(Context {
        reports: Mutex::new(Vec::new()),
        refuse: true,
    }));
    assert!(matches!(
        o.judge_ambiguity(&section()),
        Err(OrchestratorInferenceError::Context(
            ContextReportError::NotAdmitted
        ))
    ));
}

#[test]
fn a_completion_that_is_not_a_judgement_is_refused_never_defaulted() {
    for text in [
        "the section is fairly clear",
        "{}",
        r#"{"dispatch_ambiguity": "high"}"#,
        r#"{"dispatch_ambiguity": 1.5}"#,
        r#"{"dispatch_ambiguity": -0.1}"#,
        r#"[0.4]"#,
    ] {
        assert!(
            matches!(
                parse_ambiguity(text),
                Err(OrchestratorInferenceError::Completion(_))
            ),
            "{text:?} must refuse"
        );
    }
    let o = bound(
        Port::answering(r#"{"ambiguity": 0.9}"#),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    );
    let err = o
        .judge_ambiguity(&section())
        .expect_err("wrong field refuses");
    assert!(matches!(err, OrchestratorInferenceError::Completion(_)));
    assert!(err.to_string().contains("dispatch_ambiguity"), "{err}");
}

#[test]
fn a_well_formed_completion_parses_at_both_bounds() {
    assert_eq!(
        parse_ambiguity(r#"{"dispatch_ambiguity": 0}"#).unwrap(),
        0.0
    );
    assert_eq!(
        parse_ambiguity("  {\"dispatch_ambiguity\": 1.0}\n").unwrap(),
        1.0
    );
    assert_eq!(
        parse_ambiguity(r#"{"dispatch_ambiguity": 0.42, "reason": "two readings"}"#).unwrap(),
        0.42
    );
}

// ── Story 19-4 (AC2.1, D-19-4-01 A) — the judged scalar reaches the kernel ──

/// One recorded `write_scalar` call: (pid, spirit id, tag, value, derived_from).
type ScalarWrite = (u32, String, String, f64, String);
/// What the stand-in kernel answers a write with.
type KernelAnswer = fn() -> Result<Option<HaltReceipt>, ScalarPortError>;

/// A scalar port standing in for the root's kernel adapter: records every
/// write and answers with the receipt (or refusal) the KERNEL would — the
/// Spirit decides nothing about the halt.
struct Scalar {
    writes: Mutex<Vec<ScalarWrite>>,
    answer: KernelAnswer,
}

impl Scalar {
    fn answering(answer: KernelAnswer) -> Arc<Self> {
        Arc::new(Self {
            writes: Mutex::new(Vec::new()),
            answer,
        })
    }

    fn writes(&self) -> Vec<ScalarWrite> {
        self.writes.lock().unwrap().clone()
    }
}

impl EpistemicScalarPort for Scalar {
    fn write_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        self.writes.lock().unwrap().push((
            spirit_pid,
            spirit_id.to_owned(),
            tag.to_owned(),
            value,
            derived_from.to_owned(),
        ));
        (self.answer)()
    }
}

fn kernel_halted() -> Result<Option<HaltReceipt>, ScalarPortError> {
    Ok(Some(HaltReceipt::new(
        HaltId::new("01J19D4AMBIGU0USSECT10N200").unwrap(),
        1,
        PID,
        7,
        [0x19; 16],
    )))
}

fn kernel_passed() -> Result<Option<HaltReceipt>, ScalarPortError> {
    Ok(None)
}

fn kernel_refused() -> Result<Option<HaltReceipt>, ScalarPortError> {
    Err(ScalarPortError::Backend("policy path unavailable".into()))
}

/// The judged `dispatch_ambiguity` is written ONCE through the scalar port —
/// the judgement's tag, value and provenance, under the Orchestrator's id and
/// pid 0 (the host binds the pid, D-16-3-N) — and the kernel's receipt is
/// returned as-is: `Some` when its policy halted, `None` when it did not.
#[test]
fn assess_writes_the_judged_scalar_once_and_returns_the_kernels_receipt() {
    for (answer, halted) in [
        (kernel_halted as KernelAnswer, true),
        (kernel_passed as KernelAnswer, false),
    ] {
        let scalar = Scalar::answering(answer);
        let o = bound(
            Port::answering(r#"{"dispatch_ambiguity": 0.86}"#),
            Some(binding(&Arc::new(AtomicUsize::new(0)))),
        )
        .with_scalar_port(Arc::clone(&scalar) as Arc<dyn EpistemicScalarPort>);
        let (value, receipt) = o.assess_ambiguity(&section()).expect("assessed");
        assert_eq!(value, 0.86);
        assert_eq!(
            receipt,
            answer().unwrap(),
            "the kernel's receipt, untouched"
        );
        assert_eq!(receipt.is_some(), halted);
        assert_eq!(
            scalar.writes(),
            vec![(
                0,
                "orchestrator".to_owned(),
                SCALAR_TAG_DISPATCH_AMBIGUITY.to_owned(),
                0.86,
                format!("completion:{PROVIDER}:section-2"),
            )]
        );
        assert_eq!(o.dispatched_count(), 0, "assessing never dispatches");
        assert!(o.is_safe_point());
    }
}

/// No scalar port ⇒ `NotWired` before any completion is spent; a scalar port
/// with no Inference Port (unset mode, ADR-064) ⇒ `NotWired` and no write.
#[test]
fn assess_without_either_port_is_not_wired_and_spends_nothing() {
    let port = Port::answering(r#"{"dispatch_ambiguity": 0.9}"#);
    let o = bound(
        Arc::clone(&port),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    );
    assert!(matches!(
        o.assess_ambiguity(&section()),
        Err(OrchestratorInferenceError::NotWired)
    ));
    assert!(
        port.requests().is_empty(),
        "no completion without a scalar port"
    );

    let scalar = Scalar::answering(kernel_halted);
    let o = Orchestrator::new("orchestrator")
        .with_scalar_port(Arc::clone(&scalar) as Arc<dyn EpistemicScalarPort>);
    assert!(matches!(
        o.assess_ambiguity(&section()),
        Err(OrchestratorInferenceError::NotWired)
    ));
    assert!(scalar.writes().is_empty(), "no write without a judgement");
}

/// A refused scalar write is a typed refusal (never a default "no halt"),
/// and a refused judgement never reaches the scalar port.
#[test]
fn a_refused_scalar_write_or_judgement_is_typed_never_defaulted() {
    let scalar = Scalar::answering(kernel_refused);
    let o = bound(
        Port::answering(r#"{"dispatch_ambiguity": 0.2}"#),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    )
    .with_scalar_port(Arc::clone(&scalar) as Arc<dyn EpistemicScalarPort>);
    let err = o.assess_ambiguity(&section()).unwrap_err();
    assert!(
        matches!(
            err,
            OrchestratorInferenceError::Scalar(ScalarPortError::Backend(_))
        ),
        "{err:?}"
    );
    assert!(err.to_string().contains("scalar write refused"), "{err}");
    assert_eq!(scalar.writes().len(), 1);

    let scalar = Scalar::answering(kernel_halted);
    let o = bound(
        Port::answering("I think it is fine"),
        Some(binding(&Arc::new(AtomicUsize::new(0)))),
    )
    .with_scalar_port(Arc::clone(&scalar) as Arc<dyn EpistemicScalarPort>);
    assert!(matches!(
        o.assess_ambiguity(&section()),
        Err(OrchestratorInferenceError::Completion(_))
    ));
    assert!(
        scalar.writes().is_empty(),
        "a non-judgement is never written"
    );
}
