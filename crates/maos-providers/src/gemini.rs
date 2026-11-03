//! Google Gemini `generateContent` provider driver (Story 20-4).
//!
//! Translates `InferenceRequest` / `InferenceResponse` to/from the Gemini REST
//! wire format (`POST {endpoint}/v1beta/models/{model}:generateContent`). Hand-
//! rolled over the injected transport: vendor SDKs are denied (FR47,
//! `xtask/fr47-vendor-sdk-denylist.toml`). The key travels in the
//! `x-goog-api-key` header, never in the URL (the URL is journaled and echoed
//! in transport errors).

use maos_domain::ports::inference::{
    InferenceRequest, InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
};
use maos_domain::ports::{IoSubsystemPort, SecretKey, SecretStore};

use crate::provider::{Provider, ProviderError};

/// Gemini provider driver.
pub struct GeminiProvider {
    api_key: String,
    endpoint_url: String,
    model_id: String,
    transport: std::sync::Arc<dyn IoSubsystemPort>,
}

impl GeminiProvider {
    /// Create a Gemini provider from an injected secret store.
    pub fn new(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
        secret_store: Option<std::sync::Arc<dyn SecretStore>>,
    ) -> Result<Self, ProviderError> {
        let Some(secret_store) = secret_store else {
            return Err(ProviderError::Unconfigured);
        };
        let api_key = secret_store
            .get(SecretKey::GeminiApiKey)
            .map_err(|_| ProviderError::Unconfigured)?
            .filter(|key| !key.is_empty())
            .ok_or(ProviderError::Unconfigured)?;
        Self::with_api_key(transport, endpoint_url, model_id, api_key)
    }

    /// Create from an explicit API key (for tests).
    #[doc(hidden)]
    pub fn with_api_key(
        transport: std::sync::Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        model_id: String,
        api_key: String,
    ) -> Result<Self, ProviderError> {
        crate::provider::validate_api_key(SecretKey::GeminiApiKey, &api_key)?;
        Ok(Self {
            api_key,
            endpoint_url,
            model_id,
            transport,
        })
    }
}

impl Provider for GeminiProvider {
    fn credential_fingerprint(&self) -> u64 {
        crate::rate_limit::fingerprint_credential(&self.api_key)
    }

    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        let model_id = req.options.model_id.as_deref().unwrap_or(&self.model_id);
        let body = build_request_body(req);
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| ProviderError::Serde(e.to_string()))?;
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.endpoint_url, model_id
        );
        let exchange = self
            .transport
            .http_post_exchange(
                &url,
                &body_bytes,
                &[
                    ("x-goog-api-key", &self.api_key),
                    ("content-type", "application/json"),
                ],
                "retry-after",
            )
            .map_err(|e| ProviderError::Transport(e.to_string()))?;
        if !(200..=299).contains(&exchange.status) {
            return Err(ProviderError::ProviderRejected {
                status: exchange.status,
                body: String::from_utf8_lossy(&exchange.body).into_owned(),
            });
        }
        let json: serde_json::Value = serde_json::from_slice(&exchange.body)
            .map_err(|e| ProviderError::Serde(e.to_string()))?;
        parse_response(&json, &self.endpoint_url, model_id)
    }
}

fn build_request_body(req: &InferenceRequest) -> serde_json::Value {
    let mut config = serde_json::json!({ "maxOutputTokens": req.options.max_tokens });
    if let Some(t) = req.options.temperature {
        config["temperature"] = serde_json::json!(t);
    }
    serde_json::json!({
        "contents": [{ "role": "user", "parts": [{ "text": req.prompt }] }],
        "generationConfig": config,
    })
}

fn parse_response(
    json: &serde_json::Value,
    endpoint_url: &str,
    model_id: &str,
) -> Result<InferenceResponse, ProviderError> {
    let candidate = json
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| ProviderError::Serde("missing candidates[0]".into()))?;
    let text: String = candidate
        .get("content")
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .ok_or_else(|| ProviderError::Serde("missing candidates[0].content.parts".into()))?
        .iter()
        .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
        .collect();
    let stop_reason = match candidate.get("finishReason").and_then(|s| s.as_str()) {
        Some("STOP") | None => StopReason::StopSequence,
        Some("MAX_TOKENS") => StopReason::MaxTokens,
        Some(other) => StopReason::ProviderStop(other.into()),
    };
    let usage = json.get("usageMetadata");
    let count = |k: &str| {
        usage
            .and_then(|u| u.get(k))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
    };
    Ok(InferenceResponse {
        text,
        stop_reason,
        usage: TokenUsage {
            input_tokens: count("promptTokenCount"),
            output_tokens: count("candidatesTokenCount"),
        },
        provider_attribution: ProviderAttribution {
            provider_id: "gemini".into(),
            endpoint_url: endpoint_url.into(),
            model_id: Some(model_id.into()),
        },
    })
}
