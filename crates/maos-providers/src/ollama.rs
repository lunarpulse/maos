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
    fn credential_fingerprint(&self) -> u64 {
        // Ollama has no api_key; hash the endpoint_url so per-instance
        // isolation still applies if the operator runs two Ollama servers.
        crate::rate_limit::fingerprint_credential(&self.endpoint_url)
    }

    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        // S1: an admitted pin (the request's `model_id`) is served — and
        // attributed — by that model; else the configured one.
        let model_id = req.options.model_id.as_deref().unwrap_or(&self.model_id);
        let body = build_ollama_request_body(req, model_id);

        let url = format!("{}/api/chat", self.endpoint_url);
        let response_bytes = crate::provider::post_json(
            &*self.transport,
            &url,
            &body,
            &[("content-type", "application/json")],
        )?;

        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
            .map_err(|e| ProviderError::Serde(e.to_string()))?;

        parse_ollama_response(&response_json, &self.endpoint_url, model_id)
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

    // Fail closed (NFR-Cost-1) on a PRESENT count: Ollama reports its counts at
    // the top level of the reply; a non-integer, negative or > u32 value is a
    // typed error, never a silent 0-token cost frame. An ABSENT count is 0:
    // Ollama's `api.ChatResponse` / `Metrics` serialize `prompt_eval_count` and
    // `eval_count` with `omitempty` (github.com/ollama/ollama api/types.go), so
    // the vendor omits a zero value (issue #3427: a cached prompt returned zero
    // with `prompt_eval_count` absent). Requiring both would reject successful
    // replies.
    let input_tokens = ollama_count(json, "prompt_eval_count")?;
    let output_tokens = ollama_count(json, "eval_count")?;

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

/// Ollama usage counter: absent -> 0 (vendor `omitempty`); present -> must be
/// a non-negative integer fitting `u32`, else a typed error.
fn ollama_count(json: &serde_json::Value, key: &str) -> Result<u32, ProviderError> {
    if json.get(key).is_none() {
        return Ok(0);
    }
    crate::provider::usage_count(Some(json), key)
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
        let resp = parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
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
        let resp = parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
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
        let resp = parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert!(matches!(resp.stop_reason, StopReason::ProviderStop(ref s) if s == "load"));
    }

    #[test]
    fn parse_absent_usage_counters_are_zero_and_present_invalid_is_refused() {
        // Ollama serializes both counters with `omitempty`: absent means 0.
        let absent = serde_json::json!({ "message": { "content": "text" } });
        let resp = parse_ollama_response(&absent, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);

        // Cached prompt (issue #3427): only `eval_count` present.
        let cached = serde_json::json!({
            "message": { "content": "text" },
            "eval_count": 7
        });
        let resp = parse_ollama_response(&cached, "http://localhost:11434", "llama3.1:8b").unwrap();
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 7);

        // A PRESENT value that is not a u32 integer still fails closed.
        for bad in [
            serde_json::json!("5"),
            serde_json::json!(5.5),
            serde_json::json!(null),
            serde_json::json!(-1),
            serde_json::json!(u64::from(u32::MAX) + 1),
        ] {
            let json = serde_json::json!({
                "message": { "content": "text" },
                "prompt_eval_count": bad,
                "eval_count": 1
            });
            let err =
                parse_ollama_response(&json, "http://localhost:11434", "llama3.1:8b").unwrap_err();
            assert!(matches!(err, ProviderError::Serde(_)), "{bad}");
        }
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
