//! Story 22-8 (D-22-8-04 A) — Nash's inference seam phrases `proposed_fix` only; `components`
//! and `confidence` stay the deterministic ones, and an unbound seam is exactly `architect`.

use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use nash::{AdvisoryInput, Nash};
use std::sync::{Arc, Mutex};

struct Fake {
    reply: Result<String, ()>,
    prompts: Mutex<Vec<String>>,
}

impl InferencePort for Fake {
    fn complete(&self, request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.prompts.lock().unwrap().push(request.prompt);
        match &self.reply {
            Ok(text) => Ok(InferenceResponse {
                text: text.clone(),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "fake".into(),
                    endpoint_url: "fake://".into(),
                    model_id: None,
                },
            }),
            Err(()) => Err(InferenceError::Timeout),
        }
    }
}

fn advisory() -> AdvisoryInput {
    AdvisoryInput {
        subject: "payments-api".into(),
        finding: "error_rate 0.420 vs 0.020".into(),
        severity: 0.8,
        recommended_action: "architect a fix".into(),
        source_log_ref: "log:1".into(),
    }
}

#[test]
fn the_seam_phrases_the_fix_and_nothing_else() {
    let port = Arc::new(Fake {
        reply: Ok("  Put a circuit breaker in front of payments-api.  ".into()),
        prompts: Mutex::default(),
    });
    let nash = Nash::default().with_inference_port(port.clone());
    let deterministic = nash.architect(&advisory());
    let phrased = nash.architect_phrased(&advisory()).unwrap();
    assert_eq!(
        phrased.proposed_fix,
        "Put a circuit breaker in front of payments-api."
    );
    assert_eq!(phrased.components, deterministic.components);
    assert_eq!(phrased.confidence, deterministic.confidence);
    assert_eq!(phrased.source_log_ref, deterministic.source_log_ref);
    let prompts = port.prompts.lock().unwrap();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains(&deterministic.proposed_fix));
}

#[test]
fn an_unbound_seam_is_the_deterministic_proposal() {
    let nash = Nash::default();
    assert_eq!(
        nash.architect_phrased(&advisory()).unwrap(),
        nash.architect(&advisory())
    );
}

#[test]
fn a_refused_or_empty_completion_is_an_error_not_a_made_up_fix() {
    for reply in [Err(()), Ok("   ".to_string())] {
        let nash = Nash::default().with_inference_port(Arc::new(Fake {
            reply,
            prompts: Mutex::default(),
        }));
        assert!(nash.architect_phrased(&advisory()).is_err());
    }
}
