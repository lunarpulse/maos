//! Secret materialization port (Story 16-4, ADR-051 / NFR-Sec-19).
//!
//! The domain owns the closed credential names and object-safe seam. OS
//! keyrings, process-environment fallback, and encrypted files remain adapters
//! outside the kernel dependency closure.

/// Credentials MAOS may read and remove. The set is intentionally closed so
/// purge never enumerates or deletes operator-owned keyring entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretKey {
    AnthropicApiKey,
    OpenAiApiKey,
}

impl SecretKey {
    pub const ALL: [Self; 2] = [Self::AnthropicApiKey, Self::OpenAiApiKey];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicApiKey => "anthropic-api-key",
            Self::OpenAiApiKey => "openai-api-key",
        }
    }

    pub const fn environment_variable(self) -> &'static str {
        match self {
            Self::AnthropicApiKey => "MAOS_ANTHROPIC_API_KEY",
            Self::OpenAiApiKey => "MAOS_OPENAI_API_KEY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretDeleteStatus {
    Removed,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret store unavailable: {0}")]
    Unavailable(String),
    #[error("secret store access failed: {0}")]
    Access(String),
    #[error("secret value is not valid UTF-8")]
    InvalidEncoding,
}

pub trait SecretStore: Send + Sync {
    /// Class: data-movement
    ///
    /// Materialize one known credential. `None` means the backend is healthy
    /// but carries no value for that name.
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError>;
    /// Store or replace one known credential.
    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError>;

    /// Class: data-movement
    ///
    /// Delete exactly one known credential. There is deliberately no list or
    /// collection-delete operation.
    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError>;

    /// Class: supervision
    ///
    /// Whether the backend can currently accept an operation.
    fn is_healthy(&self) -> bool;
}
