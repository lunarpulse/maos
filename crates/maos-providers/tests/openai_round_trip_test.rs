#![forbid(unsafe_code)]

//! Story 5.5b AC1 — OpenAI provider round-trip integration test.
//!
//! Validates the OpenAI driver against fixture JSON files.

use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_providers::Provider;
use maos_providers::openai::OpenAiProvider;

struct MockTransport(Vec<u8>);

impl maos_domain::ports::IoSubsystemPort for MockTransport {
    fn http_get(&self, _url: &str) -> Result<Vec<u8>, maos_domain::ports::io_subsystem::IoError> {
        unimplemented!()
    }
    fn http_post(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, maos_domain::ports::io_subsystem::IoError> {
        Ok(self.0.clone())
    }
}

fn sample_request() -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0u8; 64]),
        "Hello, world!".into(),
        InferenceOptions::default(),
        None,
        vec![],
    )
}

#[test]
fn openai_round_trip_with_fixture_response() {
    let fixture_bytes =
        std::fs::read("tests/fixtures/openai_success_response.json").unwrap();
    let transport = std::sync::Arc::new(MockTransport(fixture_bytes));
    let provider =
        OpenAiProvider::with_api_key(transport, "https://api.openai.com".into(), "gpt-4o-mini".into(), "fake-key".into());
    let resp = provider.complete(&sample_request()).unwrap();
    assert_eq!(resp.text, "Greetings from OpenAI!");
    assert!(matches!(resp.stop_reason, StopReason::StopSequence));
    assert_eq!(resp.usage.input_tokens, 5);
    assert_eq!(resp.usage.output_tokens, 8);
    assert_eq!(resp.provider_attribution.provider_id, "openai");
}

#[test]
fn openai_round_trip_with_max_tokens_fixture() {
    let fixture_bytes =
        std::fs::read("tests/fixtures/openai_max_tokens_response.json").unwrap();
    let transport = std::sync::Arc::new(MockTransport(fixture_bytes));
    let provider =
        OpenAiProvider::with_api_key(transport, "https://api.openai.com".into(), "gpt-4o-mini".into(), "fake-key".into());
    let resp = provider.complete(&sample_request()).unwrap();
    assert!(matches!(resp.stop_reason, StopReason::MaxTokens));
    assert_eq!(resp.usage.input_tokens, 2);
    assert_eq!(resp.usage.output_tokens, 10);
}

#[test]
fn openai_round_trip_with_error_fixture() {
    let fixture_bytes =
        std::fs::read("tests/fixtures/openai_error_response.json").unwrap();
    let transport = std::sync::Arc::new(MockTransport(fixture_bytes));
    let provider =
        OpenAiProvider::with_api_key(transport, "https://api.openai.com".into(), "gpt-4o-mini".into(), "fake-key".into());
    let result = provider.complete(&sample_request());
    assert!(result.is_err(), "error fixture should produce a ProviderError");
}
