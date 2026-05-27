//! Anthropic Messages API provider driver.
//!
//! Translates `InferenceRequest` / `InferenceResponse` to/from the Anthropic
//! REST wire format. The Anthropic JSON shapes never escape this module.
//!
//! Environment-gated: `MAOS_ANTHROPIC_API_KEY` must be set; otherwise
//! construction returns `ProviderError::Unconfigured`.

use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::IoSubsystemPort;

use crate::provider::{Provider, ProviderError};

/// Anthropic provider driver.
pub struct AnthropicProvider {
    api_key: String,
    endpoint_url: String,
    model_id: String,
    transport: std::sync::Arc<dyn IoSubsystemPort>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    ///
    /// Reads the API key from `MAOS_ANTHROPIC_API_KEY` env var.
    /// `FIXME(secrets)`: real secret materialization via `maos-secrets` / OS
    /// keyring is a later story (mirrors `main.rs:93` signing-key pattern).
    pub fn new(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
    ) -> Result<Self, ProviderError> {
        let api_key =
            std::env::var("MAOS_ANTHROPIC_API_KEY").map_err(|_| ProviderError::Unconfigured)?;
        Ok(Self {
            api_key,
            endpoint_url,
            model_id,
            transport,
        })
    }

    /// Create from an explicit API key (for tests).
    #[doc(hidden)]
    pub fn with_api_key(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
        api_key: String,
    ) -> Self {
        Self {
            api_key,
            endpoint_url,
            model_id,
            transport,
        }
    }
}

impl Provider for AnthropicProvider {
    fn credential_fingerprint(&self) -> u64 {
        crate::rate_limit::fingerprint_credential(&self.api_key)
    }

    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        let body = build_request_body(req, &self.model_id);
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| ProviderError::Serde(e.to_string()))?;

        let url = format!("{}/v1/messages", self.endpoint_url);
        let response_bytes = self
            .transport
            .http_post(
                &url,
                &body_bytes,
                &[
                    ("x-api-key", &self.api_key),
                    ("anthropic-version", "2023-06-01"),
                    ("content-type", "application/json"),
                ],
            )
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
            .map_err(|e| ProviderError::Serde(e.to_string()))?;

        parse_response(&response_json, &self.endpoint_url, &self.model_id)
    }
}

/// Build the Anthropic Messages API request body.
fn build_request_body(req: &InferenceRequest, model_id: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model_id,
        "max_tokens": req.options.max_tokens,
        "messages": [
            { "role": "user", "content": req.prompt }
        ]
    });
    if let Some(t) = req.options.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    body
}

/// Parse the Anthropic Messages API response into `InferenceResponse`.
fn parse_response(
    json: &serde_json::Value,
    endpoint_url: &str,
    model_id: &str,
) -> Result<InferenceResponse, ProviderError> {
    let content = json
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| ProviderError::Serde("missing content.text".into()))?;

    let stop_reason = json
        .get("stop_reason")
        .and_then(|s| s.as_str())
        .map(|s| match s {
            "max_tokens" => StopReason::MaxTokens,
            "end_turn" => StopReason::StopSequence,
            other => StopReason::ProviderStop(other.into()),
        })
        .unwrap_or(StopReason::StopSequence);

    let usage = json.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let output_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    Ok(InferenceResponse {
        text: content.into(),
        stop_reason,
        usage: TokenUsage {
            input_tokens,
            output_tokens,
        },
        provider_attribution: ProviderAttribution {
            provider_id: "anthropic".into(),
            endpoint_url: endpoint_url.into(),
            model_id: Some(model_id.into()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::io_subsystem::IoError;

    /// Mock transport that returns canned bytes.
    struct MockTransport(Vec<u8>);

    impl IoSubsystemPort for MockTransport {
        fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
            unimplemented!()
        }
        fn http_post(
            &self,
            _url: &str,
            _body: &[u8],
            _headers: &[(&str, &str)],
        ) -> Result<Vec<u8>, IoError> {
            Ok(self.0.clone())
        }
    }

    fn sample_request() -> InferenceRequest {
        InferenceRequest::new(
            42,
            maos_domain::invariants::i1::CapabilityToken::new(
                maos_domain::invariants::i1::TokenId::ZERO,
                42,
                0,
                [0u8; 64],
            ),
            "Hello, world!".into(),
            InferenceOptions {
                max_tokens: 100,
                temperature: Some(0.5),
                model_id: None,
            },
            None,
            vec![],
        )
    }

    #[test]
    fn request_body_has_expected_shape() {
        let req = sample_request();
        let body = build_request_body(&req, "claude-3-haiku-20240307");
        assert_eq!(body["model"], "claude-3-haiku-20240307");
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["temperature"], 0.5);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hello, world!");
    }

    #[test]
    fn parse_successful_response() {
        let json = serde_json::json!({
            "content": [{ "type": "text", "text": "Greetings!" }],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 5, "output_tokens": 10 }
        });
        let resp = parse_response(
            &json,
            "https://api.anthropic.com",
            "claude-3-haiku-20240307",
        )
        .unwrap();
        assert_eq!(resp.text, "Greetings!");
        assert!(matches!(resp.stop_reason, StopReason::StopSequence));
        assert_eq!(resp.usage.input_tokens, 5);
        assert_eq!(resp.usage.output_tokens, 10);
        assert_eq!(resp.provider_attribution.provider_id, "anthropic");
    }

    #[test]
    fn parse_max_tokens_stop_reason() {
        let json = serde_json::json!({
            "content": [{ "type": "text", "text": "truncated" }],
            "stop_reason": "max_tokens",
            "usage": { "input_tokens": 1, "output_tokens": 2 }
        });
        let resp = parse_response(
            &json,
            "https://api.anthropic.com",
            "claude-3-haiku-20240307",
        )
        .unwrap();
        assert!(matches!(resp.stop_reason, StopReason::MaxTokens));
    }

    #[test]
    fn provider_round_trip_with_mock_transport() {
        let response_json = serde_json::json!({
            "content": [{ "type": "text", "text": "Mock reply" }],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 3, "output_tokens": 4 }
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = AnthropicProvider::with_api_key(
            transport,
            "https://api.anthropic.com".into(),
            "claude-3-haiku-20240307".into(),
            "fake-key".into(),
        );
        let req = sample_request();
        let resp = provider.complete(&req).unwrap();
        assert_eq!(resp.text, "Mock reply");
    }

    #[test]
    fn provider_missing_api_key_is_unconfigured() {
        // Ensure MAOS_ANTHROPIC_API_KEY is unset for this test.
        std::env::remove_var("MAOS_ANTHROPIC_API_KEY");
        let transport = std::sync::Arc::new(MockTransport(vec![]));
        let result = AnthropicProvider::new(
            transport,
            "https://api.anthropic.com".into(),
            "claude-3-haiku-20240307".into(),
        );
        assert!(matches!(result, Err(ProviderError::Unconfigured)));
    }

    /// Integration smoke test — requires `MAOS_ANTHROPIC_API_KEY` AND
    /// a real HTTP transport. Run with:
    ///   `cargo test -p maos-providers -- --ignored anthropic_integration`
    /// NOTE: This test currently uses MockTransport; replace with a real
    /// IoSubsystemPort adapter when a non-circular dependency path exists.
    #[test]
    #[ignore = "requires live Anthropic API key + real transport (currently scaffold)"]
    fn anthropic_integration() {
        let response_json = serde_json::json!({
            "content": [{ "type": "text", "text": "Live reply placeholder" }],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = AnthropicProvider::new(
            transport,
            "https://api.anthropic.com".into(),
            "claude-3-haiku-20240307".into(),
        )
        .expect("MAOS_ANTHROPIC_API_KEY must be set");
        let req = sample_request();
        let resp = provider.complete(&req).expect("call should succeed");
        assert!(!resp.text.is_empty());
    }
}
