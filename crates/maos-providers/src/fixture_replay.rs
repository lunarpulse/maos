//! Fixture-replay provider for multi-provider CI matrix testing.
//!
//! Implements `Provider` and serves `InferenceResponse` from a ring buffer
//! in declaration order. Used by the multi-provider CI matrix and smoke
//! arms so the matrix never depends on live API keys in CI.

#[cfg(any(test, feature = "fixture_replay"))]
use std::collections::VecDeque;

#[cfg(any(test, feature = "fixture_replay"))]
use crate::provider::{Provider, ProviderError};
#[cfg(any(test, feature = "fixture_replay"))]
use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};

/// A `Provider` that replays canned responses from a ring buffer.
///
/// Records every request it receives for later inspection.
#[cfg(any(test, feature = "fixture_replay"))]
#[derive(Debug)]
pub struct FixtureReplayProvider {
    responses: std::sync::Mutex<VecDeque<Result<InferenceResponse, ProviderError>>>,
    calls: std::sync::Mutex<Vec<InferenceRequest>>,
}

#[cfg(any(test, feature = "fixture_replay"))]
impl FixtureReplayProvider {
    /// Create a new replay provider with the given response ring.
    pub fn new(responses: Vec<Result<InferenceResponse, ProviderError>>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses.into_iter().collect()),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Return a snapshot of all recorded requests.
    pub fn recorded_calls(&self) -> Vec<InferenceRequest> {
        self.calls.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "fixture_replay"))]
impl Provider for FixtureReplayProvider {
    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        self.calls.lock().unwrap().push(req.clone());
        let mut responses = self.responses.lock().unwrap();
        responses.pop_front().unwrap_or_else(|| {
            panic!("FixtureReplayProvider: response ring empty — all canned responses consumed")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::inference::{
        InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
    };

    fn ok_response(text: &str) -> InferenceResponse {
        InferenceResponse {
            text: text.into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "test".into(),
                endpoint_url: "http://test".into(),
                model_id: None,
            },
        }
    }

    fn sample_request() -> InferenceRequest {
        InferenceRequest::new(
            1,
            maos_domain::invariants::i1::CapabilityToken::new(
                maos_domain::invariants::i1::TokenId::ZERO,
                1,
                0,
                [0u8; 64],
            ),
            "test".into(),
            maos_domain::ports::inference::InferenceOptions::default(),
            None,
            vec![],
        )
    }

    #[test]
    fn round_trip_records_request() {
        let provider = FixtureReplayProvider::new(vec![Ok(ok_response("r1"))]);
        let req = sample_request();
        let resp = provider.complete(&req).unwrap();
        assert_eq!(resp.text, "r1");
        let calls = provider.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "test");
    }

    #[test]
    #[should_panic(expected = "response ring empty")]
    fn empty_ring_panics() {
        let provider = FixtureReplayProvider::new(vec![]);
        let req = sample_request();
        let _ = provider.complete(&req);
    }

    #[test]
    fn ring_replays_in_order() {
        let provider =
            FixtureReplayProvider::new(vec![Ok(ok_response("first")), Ok(ok_response("second"))]);
        let req = sample_request();
        let r1 = provider.complete(&req).unwrap();
        let r2 = provider.complete(&req).unwrap();
        assert_eq!(r1.text, "first");
        assert_eq!(r2.text, "second");
    }

    #[test]
    fn error_response_propagated() {
        let provider = FixtureReplayProvider::new(vec![Err(ProviderError::Unconfigured)]);
        let req = sample_request();
        let err = provider.complete(&req).unwrap_err();
        assert!(matches!(err, ProviderError::Unconfigured));
    }
}
