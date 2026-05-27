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

    /// Story 6.4 / NFR-Scale-4 — return a 64-bit fingerprint of this driver's
    /// credential bytes for cross-credential isolation in the rate-limit
    /// bucket map. The default returns `u64::MAX` as a sentinel; each concrete
    /// provider MUST override with `first-8-bytes-of-sha256(api_key)`. The
    /// sentinel ensures that a driver forgetting to override shares a bucket
    /// only with other unconfigured drivers (same type), NOT with real keys.
    /// Ollama (which has no api_key) overrides with a hash of `base_url`.
    fn credential_fingerprint(&self) -> u64 {
        u64::MAX
    }
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
