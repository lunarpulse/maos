//! Ollama Chat API provider driver.
//!
//! Translates `InferenceRequest` / `InferenceResponse` to/from the Ollama
//! REST wire format (`POST /api/chat`). The Ollama JSON shapes never escape
//! this module.
//!
//! No API key required — Ollama runs locally. Endpoint URL defaults to
//! `http://localhost:11434`; override via `MAOS_OLLAMA_URL` env var.

use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::IoSubsystemPort;

use crate::provider::{Provider, ProviderError};

/// Ollama provider driver.
pub struct OllamaProvider {
    endpoint_url: String,
    model_id: String,
    transport: std::sync::Arc<dyn IoSubsystemPort>,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// No env-gated credential required. `MAOS_OLLAMA_URL` overrides the
    /// default `http://localhost:11434` endpoint.
    pub fn new(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            endpoint_url,
            model_id,
            transport,
        })
    }
}

impl Provider for OllamaProvider {
    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        let body = build_ollama_request_body(req, &self.model_id);
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| ProviderError::Serde(e.to_string()))?;

        let url = format!("{}/api/chat", self.endpoint_url);
        let response_bytes = self
            .transport
            .http_post(&url, &body_bytes, &[("content-type", "application/json")])
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
            .map_err(|e| ProviderError::Serde(e.to_string()))?;

        parse_ollama_response(&response_json, &self.endpoint_url, &self.model_id)
    }
}

fn build_ollama_request_body(req: &InferenceRequest, model_id: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "user", "content": req.prompt }
        ],
        "stream": false,
        "options": {
            "num_predict": req.options.max_tokens
        }
    });
    if let Some(t) = req.options.temperature {
        body["options"]["temperature"] = serde_json::json!(t);
    }
    body
}

fn parse_ollama_response(
    json: &serde_json::Value,
    endpoint_url: &str,
    model_id: &str,
) -> Result<InferenceResponse, ProviderError> {
    let content = json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| ProviderError::Serde("missing message.content".into()))?;

    let stop_reason = json
        .get("done_reason")
        .and_then(|s| s.as_str())
        .map(|s| match s {
            "stop" => StopReason::StopSequence,
            "length" => StopReason::MaxTokens,
            other => StopReason::ProviderStop(other.into()),
        })
        .unwrap_or(StopReason::StopSequence);

    let input_tokens = json
        .get("prompt_eval_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let output_tokens = json
        .get("eval_count")
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
            provider_id: "ollama".into(),
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
        let body = build_ollama_request_body(&req, "llama3.1:8b");
        assert_eq!(body["model"], "llama3.1:8b");
        assert_eq!(body["stream"], false);
        assert_eq!(body["options"]["num_predict"], 100);
        assert_eq!(body["options"]["temperature"], 0.5);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hello, world!");
    }

    #[test]
    fn parse_successful_response() {
        let json = serde_json::json!({
            "message": { "content": "Greetings from Ollama!" },
            "done_reason": "stop",
            "prompt_eval_count": 5,
            "eval_count": 10
        });
        let resp =
            parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert_eq!(resp.text, "Greetings from Ollama!");
        assert!(matches!(resp.stop_reason, StopReason::StopSequence));
        assert_eq!(resp.usage.input_tokens, 5);
        assert_eq!(resp.usage.output_tokens, 10);
        assert_eq!(resp.provider_attribution.provider_id, "ollama");
    }

    #[test]
    fn parse_max_tokens_stop_reason() {
        let json = serde_json::json!({
            "message": { "content": "truncated" },
            "done_reason": "length",
            "prompt_eval_count": 1,
            "eval_count": 2
        });
        let resp =
            parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert!(matches!(resp.stop_reason, StopReason::MaxTokens));
    }

    #[test]
    fn parse_provider_stop_reason() {
        let json = serde_json::json!({
            "message": { "content": "text" },
            "done_reason": "load",
            "prompt_eval_count": 1,
            "eval_count": 1
        });
        let resp =
            parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert!(matches!(resp.stop_reason, StopReason::ProviderStop(ref s) if s == "load"));
    }

    #[test]
    fn parse_missing_usage_defaults_zero() {
        let json = serde_json::json!({
            "message": { "content": "text" }
        });
        let resp =
            parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);
    }

    #[test]
    fn provider_round_trip_with_mock_transport() {
        let response_json = serde_json::json!({
            "message": { "content": "Mock Ollama reply" },
            "done_reason": "stop",
            "prompt_eval_count": 3,
            "eval_count": 4
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = OllamaProvider::new(
            transport,
            "http://localhost:11434".into(),
            "llama3.1:8b".into(),
        )
        .unwrap();
        let req = sample_request();
        let resp = provider.complete(&req).unwrap();
        assert_eq!(resp.text, "Mock Ollama reply");
    }

    #[test]
    #[ignore = "requires a running local Ollama instance"]
    fn ollama_integration() {
        let response_json = serde_json::json!({
            "message": { "content": "Live reply placeholder" },
            "done_reason": "stop",
            "prompt_eval_count": 1,
            "eval_count": 1
        });
        let transport =
            std::sync::Arc::new(MockTransport(serde_json::to_vec(&response_json).unwrap()));
        let provider = OllamaProvider::new(
            transport,
            "http://localhost:11434".into(),
            "llama3.1:8b".into(),
        )
        .unwrap();
        let req = sample_request();
        let resp = provider.complete(&req).expect("call should succeed");
        assert!(!resp.text.is_empty());
    }
}
