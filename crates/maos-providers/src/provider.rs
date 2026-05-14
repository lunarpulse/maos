//! Internal provider-driver abstraction.
//!
//! `Provider` is the uniform surface that `maos-providers` drivers implement.
//! It is deliberately narrower than `maos-domain::ports::InferencePort` —
//! the domain port carries capability/telemetry/audit concerns; the driver
//! trait is pure request/response translation over an injected transport.

use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};

/// Provider driver trait — implemented by each LLM backend (Anthropic, OpenAI, …).
pub trait Provider {
    /// Perform a single completion call.
    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError>;
}

/// Provider-level error — concrete variants, no blanket `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// HTTP transport failure.
    #[error("transport: {0}")]
    Transport(String),
    /// Provider returned a non-success status code.
    #[error("provider rejected: status={status}, body={body}")]
    ProviderRejected { status: u16, body: String },
    /// Request or response body could not be serialized / deserialized.
    #[error("serde: {0}")]
    Serde(String),
    /// Provider is not configured (missing API key, etc.).
    #[error("unconfigured")]
    Unconfigured,
}
