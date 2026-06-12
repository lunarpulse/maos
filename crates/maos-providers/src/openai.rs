//! OpenAI Chat Completions API provider driver.
//!
//! Translates `InferenceRequest` / `InferenceResponse` to/from the OpenAI
//! REST wire format (`POST /v1/chat/completions`). The OpenAI JSON shapes
//! never escape this module.
//!
//! Environment-gated: `MAOS_OPENAI_API_KEY` must be set; otherwise
//! construction returns `ProviderError::Unconfigured`.

use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::IoSubsystemPort;

use crate::provider::{Provider, ProviderError};

/// OpenAI provider driver.
pub struct OpenAiProvider {
    api_key: String,
    endpoint_url: String,
    model_id: String,
    transport: std::sync::Arc<dyn IoSubsystemPort>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    ///
    /// Reads the API key from `MAOS_OPENAI_API_KEY` env var.
    pub fn new(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
    ) -> Result<Self, ProviderError> {
        let api_key =
            std::env::var("MAOS_OPENAI_API_KEY").map_err(|_| ProviderError::Unconfigured)?;
        if api_key.is_empty() {
            return Err(ProviderError::Unconfigured);
        }
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

impl Provider for OpenAiProvider {
    fn credential_fingerprint(&self) -> u64 {
        crate::rate_limit::fingerprint_credential(&self.api_key)
    }

    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        let body = build_openai_request_body(req, &self.model_id);
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| ProviderError::Serde(e.to_string()))?;

        let url = format!("{}/v1/chat/completions", self.endpoint_url);
        let response_bytes = self
            .transport
            .http_post(
                &url,
                &body_bytes,
                &[
                    ("Authorization", &format!("Bearer {}", self.api_key)),
                    ("content-type", "application/json"),
                ],
            )
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
            .map_err(|e| ProviderError::Serde(e.to_string()))?;

        parse_openai_response(&response_json, &self.endpoint_url, &self.model_id)
    }
}

fn build_openai_request_body(req: &InferenceRequest, model_id: &str) -> serde_json::Value {
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

fn parse_openai_response(
    json: &serde_json::Value,
    endpoint_url: &str,
    model_id: &str,
) -> Result<InferenceResponse, ProviderError> {
    let content = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| ProviderError::Serde("missing choices[0].message.content".into()))?;

    let stop_reason = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("finish_reason"))
        .and_then(|s| s.as_str())
        .map(|s| match s {
            "stop" => StopReason::StopSequence,
            "length" => StopReason::MaxTokens,
            other => StopReason::ProviderStop(other.into()),
        })
        .unwrap_or(StopReason::StopSequence);

    let usage = json.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
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
            provider_id: "openai".into(),
            endpoint_url: endpoint_url.into(),
            model_id: Some(model_id.into()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::io_subsystem::IoError;

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
        let body = build_openai_request_body(&req, "gpt-4o-mini");
        assert_eq!(body["model"], "gpt-4o-mini");
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
            "choices": [{ "message": { "content": "Greetings!" }, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 10 }
        });
        let resp = parse_openai_response(&json, "https://api.openai.com", "gpt-4o-mini").unwrap();
        assert_eq!(resp.text, "Greetings!");
        assert!(matches!(resp.stop_reason, StopReason::StopSequence));
        assert_eq!(resp.usage.input_tokens, 5);
        assert_eq!(resp.usage.output_tokens, 10);
        assert_eq!(resp.provider_attribution.provider_id, "openai");
    }

    #[test]
    fn parse_max_tokens_stop_reason() {
        let json = serde_json::json!({
            "choices": [{ "message": { "content": "truncated" }, "finish_reason": "length" }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 2 }
        });
        let resp = parse_openai_response(&json, "https://api.openai.com", "gpt-4o-mini").unwrap();
        assert!(matches!(resp.stop_reason, StopReason::MaxTokens));
    }

    #[test]
    fn parse_provider_stop_reason() {
        let json = serde_json::json!({
            "choices": [{ "message": { "content": "filtered" }, "finish_reason": "content_filter" }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
        });
        let resp = parse_openai_response(&json, "https://api.openai.com", "gpt-4o-mini").unwrap();
        assert!(
            matches!(resp.stop_reason, StopReason::ProviderStop(ref s) if s == "content_filter")
        );
    }

    #[test]
    fn parse_missing_usage_defaults_zero() {
        let json = serde_json::json!({
            "choices": [{ "message": { "content": "text" }, "finish_reason": "stop" }]
        });
        let resp = parse_openai_response(&json, "https://api.openai.com", "gpt-4o-mini").unwrap();
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);
    }

    #[test]
    fn provider_round_trip_with_mock_transport() {
        let response_json = serde_json::json!({
            "choices": [{ "message": { "content": "Mock reply" }, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 3, "completion_tokens": 4 }
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = OpenAiProvider::with_api_key(
            transport,
            "https://api.openai.com".into(),
            "gpt-4o-mini".into(),
            "fake-key".into(),
        );
        let req = sample_request();
        let resp = provider.complete(&req).unwrap();
        assert_eq!(resp.text, "Mock reply");
    }

    #[test]
    fn provider_missing_api_key_is_unconfigured() {
        let saved = std::env::var("MAOS_OPENAI_API_KEY").ok();
        std::env::remove_var("MAOS_OPENAI_API_KEY");
        let transport = std::sync::Arc::new(MockTransport(vec![]));
        let result = OpenAiProvider::new(
            transport,
            "https://api.openai.com".into(),
            "gpt-4o-mini".into(),
        );
        assert!(matches!(result, Err(ProviderError::Unconfigured)));
        if let Some(key) = saved {
            std::env::set_var("MAOS_OPENAI_API_KEY", key);
        }
    }

    #[test]
    #[ignore = "requires live OpenAI API key + real transport"]
    fn openai_integration() {
        let response_json = serde_json::json!({
            "choices": [{ "message": { "content": "Live reply placeholder" }, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = OpenAiProvider::new(
            transport,
            "https://api.openai.com".into(),
            "gpt-4o-mini".into(),
        )
        .expect("MAOS_OPENAI_API_KEY must be set");
        let req = sample_request();
        let resp = provider.complete(&req).expect("call should succeed");
        assert!(!resp.text.is_empty());
    }
}
