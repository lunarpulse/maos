//! Inference Port trait per ADR-005 + ADR-010.
//!
//! The Inference Port is the kernel's uniform surface for LLM inference.
//! v0.1-β implements `complete` only with the Anthropic, OpenAI, and Ollama drivers
//! (Story 5.5b). Streaming (`stream`) and embeddings (`embed`) are deferred to a
//! v0.5+ follow-up story; see open-items-carried-forward-to-implementation.md.
//!
//! Per ADR-010, the trait is **sync** — the kernel's async callers wrap it
//! in `tokio::task::spawn_blocking` (consistent with `IoSubsystemPort`).

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::invariants::i1::CapabilityToken;

/// Inference Port — uniform LLM inference surface.
///
/// Adapters in `maos-kernel-core::inference::InferencePortAdapter`.
pub trait InferencePort {
    /// Class: data-movement
    ///
    /// Perform a single completion inference call.
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError>;
}

/// Request payload for a completion inference call.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct InferenceRequest {
    /// Spirit process ID making the request.
    pub spirit_pid: u32,
    /// Capability token presented by the Spirit.
    pub capability_token: CapabilityToken,
    /// The prompt text.
    pub prompt: String,
    /// Inference options (temperature, max_tokens, model override).
    pub options: InferenceOptions,
    /// v0.5-α dispatch key; None → composition-root default.
    #[doc = "Construct via [`InferenceRequest::new`] to ensure all fields are populated."]
    pub provider_id: Option<String>,
    /// v0.5-α fallback chain; empty = no fallback.
    #[doc = "Construct via [`InferenceRequest::new`] to ensure all fields are populated."]
    pub fallback_provider_ids: Vec<String>,
}

impl InferenceRequest {
    pub fn new(
        spirit_pid: u32,
        capability_token: CapabilityToken,
        prompt: String,
        options: InferenceOptions,
        provider_id: Option<String>,
        fallback_provider_ids: Vec<String>,
    ) -> Self {
        Self {
            spirit_pid,
            capability_token,
            prompt,
            options,
            provider_id,
            fallback_provider_ids,
        }
    }
}

/// Options controlling inference behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct InferenceOptions {
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature (0.0 = deterministic, 1.0 = max creativity).
    pub temperature: Option<f32>,
    /// Model ID override. If `None`, the provider's default model is used.
    pub model_id: Option<String>,
}

impl Default for InferenceOptions {
    fn default() -> Self {
        Self {
            max_tokens: 1024,
            temperature: None,
            model_id: None,
        }
    }
}

/// Response from a completion inference call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceResponse {
    /// Generated text.
    pub text: String,
    /// Why the generation stopped.
    pub stop_reason: StopReason,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Provider attribution for Transparency Log recording.
    pub provider_attribution: ProviderAttribution,
}

/// Reason the LLM stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Reached `max_tokens` limit.
    MaxTokens,
    /// Natural end-of-sequence.
    StopSequence,
    /// Provider-specific stop reason.
    ProviderStop(String),
}

/// Token consumption statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenUsage {
    /// Tokens in the prompt.
    pub input_tokens: u32,
    /// Tokens in the generated response.
    pub output_tokens: u32,
}

/// Attribution identifying which provider served the inference call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAttribution {
    /// Provider identifier (e.g., "anthropic", "openai").
    pub provider_id: String,
    /// Endpoint URL that was called.
    pub endpoint_url: String,
    /// Model ID that was used (if known).
    pub model_id: Option<String>,
}

/// Inference error — concrete named variants, no blanket `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    /// Transport-layer failure talking to the provider.
    #[error("provider transport: {0}")]
    ProviderTransport(String),
    /// Provider rejected the request (4xx/5xx).
    #[error("provider rejected request: status={status}, message={message}")]
    ProviderRejected { status: u16, message: String },
    /// Request timed out.
    #[error("inference timeout")]
    Timeout,
    /// Provider response could not be parsed.
    #[error("malformed provider response: {0}")]
    MalformedResponse(String),
    /// Calling Spirit lacks the `Scope::ProviderInfer` capability.
    #[error("capability denied: Spirit lacks ProviderInfer scope")]
    CapabilityDenied,
    /// Provider is not configured (e.g., `MAOS_ANTHROPIC_API_KEY` unset).
    #[error("provider unconfigured")]
    Unconfigured,
}
