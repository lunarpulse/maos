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
        validate_model_id(&model_id)?;
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
        // 20-4 review F1 (operator-adjudicated, kept by S1/Q6): the id is a URL
        // path segment, so a request may name only the admitted model — a
        // Gemini manifest pin must equal the configured model.
        let model_id = req.options.model_id.as_deref().unwrap_or(&self.model_id);
        validate_model_id(model_id)?;
        if model_id != self.model_id {
            return Err(ProviderError::Serde(
                "request model differs from admitted model pin".into(),
            ));
        }
        let body = build_request_body(req);
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.endpoint_url, model_id
        );
        let response_bytes = crate::provider::post_json(
            &*self.transport,
            &url,
            &body,
            &[
                ("x-goog-api-key", &self.api_key),
                ("content-type", "application/json"),
            ],
        )?;
        let json: serde_json::Value = serde_json::from_slice(&response_bytes)
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
    let reason = candidate
        .get("finishReason")
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ProviderError::Serde("missing or empty candidates[0].finishReason".into())
        })?;
    let stop_reason = match reason {
        "STOP" => StopReason::StopSequence,
        "MAX_TOKENS" => StopReason::MaxTokens,
        other => crate::provider::vendor_stop(other),
    };
    let parts = candidate.get("content").and_then(|c| c.get("parts"));
    let text = match parts {
        Some(value) => value
            .as_array()
            .ok_or_else(|| ProviderError::Serde("invalid candidates[0].content.parts".into()))?
            .iter()
            .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
            .collect(),
        None if matches!(stop_reason, StopReason::ProviderStop(_)) => String::new(),
        None => {
            return Err(ProviderError::Serde(
                "missing candidates[0].content.parts".into(),
            ))
        }
    };
    let usage = json.get("usageMetadata");
    Ok(InferenceResponse {
        text,
        stop_reason,
        usage: TokenUsage {
            input_tokens: crate::provider::usage_count(usage, "promptTokenCount")?,
            output_tokens: billed_output_tokens(usage)?,
        },
        provider_attribution: ProviderAttribution {
            provider_id: "gemini".into(),
            endpoint_url: endpoint_url.into(),
            model_id: Some(model_id.into()),
        },
    })
}

/// Billed output tokens: `candidatesTokenCount + thoughtsTokenCount`. Gemini
/// prices "the sum of output tokens and thinking tokens", and reports thoughts
/// separately from the candidates (https://ai.google.dev/gemini-api/docs/thinking).
///
/// `candidatesTokenCount` is required. `thoughtsTokenCount` is absent when the
/// model produced no thoughts (non-thinking models, thinking off), so an absent
/// field counts 0; a field that IS present must be a `u32` integer, never
/// silently treated as 0. The sum is overflow-checked.
fn billed_output_tokens(usage: Option<&serde_json::Value>) -> Result<u32, ProviderError> {
    let candidates = crate::provider::usage_count(usage, "candidatesTokenCount")?;
    let thoughts = if usage.and_then(|u| u.get("thoughtsTokenCount")).is_none() {
        0
    } else {
        crate::provider::usage_count(usage, "thoughtsTokenCount")?
    };
    candidates.checked_add(thoughts).ok_or_else(|| {
        ProviderError::Serde("usageMetadata candidates + thoughts tokens exceed u32".into())
    })
}

fn validate_model_id(model: &str) -> Result<(), ProviderError> {
    if !model.is_empty()
        && model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        Ok(())
    } else {
        Err(ProviderError::Serde(
            "invalid Gemini model id syntax".into(),
        ))
    }
}
