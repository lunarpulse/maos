//! Internal provider-driver abstraction.
//!
//! `Provider` is the uniform surface that `maos-providers` drivers implement.
//! It is deliberately narrower than `maos-domain::ports::InferencePort` —
//! the domain port carries capability/telemetry/audit concerns; the driver
//! trait is pure request/response translation over an injected transport.

use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};

/// Provider driver trait — implemented by each LLM backend (Anthropic, OpenAI, …).
///
/// Story 8.11 — `Send + Sync` so an `Arc<dyn Provider>` (hence `MultiProviderRouter`
/// and `InferencePortAdapter`) is `Send + Sync` and can be injected into a Spirit
/// dispatched across the multi-threaded runtime (the Researcher live-inference
/// seam). Every concrete driver is already thread-safe (stateless HTTP clients /
/// `Mutex`-wrapped state), so this only makes the existing guarantee explicit.
pub trait Provider: Send + Sync {
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
    #[error("transport: {}", BoundedError(.0))]
    Transport(String),
    /// Provider returned a non-success status code.
    #[error("provider rejected: status={status}, body={}", BoundedError(.body))]
    ProviderRejected { status: u16, body: String },
    /// Request or response body could not be serialized / deserialized.
    #[error("serde: {}", BoundedError(.0))]
    Serde(String),
    /// Provider is not configured (missing API key, etc.).
    #[error("unconfigured")]
    Unconfigured,
}

/// Refuse header-unsafe credentials without formatting their bytes.
pub(crate) fn validate_api_key(
    key: maos_domain::ports::SecretKey,
    value: &str,
) -> Result<(), ProviderError> {
    if !value.is_empty() && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        Ok(())
    } else {
        Err(ProviderError::Transport(format!(
            "{}: invalid credential bytes (value withheld)",
            key.as_str()
        )))
    }
}

struct BoundedError<'a>(&'a str);
impl std::fmt::Display for BoundedError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        let mut bytes = 0;
        for ch in self.0.chars() {
            bytes += ch.len_utf8();
            if bytes > 512 {
                break;
            }
            if !ch.is_control() {
                f.write_char(ch)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn error_text(text: &str) -> String {
    BoundedError(text).to_string()
}

pub(crate) fn transport_error(error: maos_domain::ports::io_subsystem::IoError) -> ProviderError {
    let text = match error {
        maos_domain::ports::io_subsystem::IoError::Transport(text) => text,
        other => other.to_string(),
    };
    ProviderError::Transport(error_text(&text))
}

pub(crate) fn usage_count(
    usage: Option<&serde_json::Value>,
    key: &str,
) -> Result<u32, ProviderError> {
    let count = usage
        .and_then(|u| u.get(key))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ProviderError::Serde(format!("missing or invalid usage.{key}")))?;
    u32::try_from(count).map_err(|_| ProviderError::Serde(format!("usage.{key} exceeds u32")))
}

pub(crate) fn vendor_stop(reason: &str) -> maos_domain::ports::inference::StopReason {
    maos_domain::ports::inference::StopReason::ProviderStop(
        if reason.len() > 64 {
            "vendor stop reason exceeded 64 bytes"
        } else {
            reason
        }
        .into(),
    )
}
