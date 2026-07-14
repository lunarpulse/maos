//! Key-management port for opt-in enterprise at-rest encryption (Story 11.4c,
//! ADR-051 / NFR-Sec-19).
//!
//! This port wraps and unwraps data keys outside the kernel. Adapter crates own
//! concrete KMS transports; kernel-core never depends on KMS SDKs.

/// Fail-closed key-management errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KmsError {
    #[error("key management service unavailable: {0}")]
    Unavailable(String),
    #[error("malformed KMS key material: {0}")]
    MalformedKey(&'static str),
    #[error("KMS cryptographic operation failed: {0}")]
    Crypto(String),
    #[error("malformed sealed payload: {0}")]
    MalformedPayload(&'static str),
}

/// Sync object-safe port for envelope key management.
pub trait KeyManagementPort: Send + Sync {
    /// Class: data-movement
    ///
    /// Wrap a raw data key under the configured org KMS/master key. The returned
    /// bytes are opaque to callers and must fail closed if the KMS is unhealthy.
    fn wrap_data_key(&self, data_key: &[u8]) -> Result<Vec<u8>, KmsError>;

    /// Class: data-movement
    ///
    /// Unwrap an opaque wrapped data key. Wrong master keys or corrupted wraps
    /// return an error; callers must never fall back to plaintext.
    fn unwrap_data_key(&self, wrapped_data_key: &[u8]) -> Result<Vec<u8>, KmsError>;

    /// Class: supervision
    ///
    /// Whether KMS key material/configuration is currently usable.
    fn is_healthy(&self) -> bool;
}
