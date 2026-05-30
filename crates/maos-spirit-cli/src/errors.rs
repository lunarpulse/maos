//! Typed errors for the `maos-spirit` CLI.
//!
//! Exit-code semantics per Story 7.2 AC2:
//!   * `0` on success
//!   * `1` on transport / signing / config error
//!   * `2` on `TrustTierFloorViolated` / `OrgSignatureInvalid` (operator-side rejection)
//!   * `3` on `RegistryError::Unconfigured`

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("signing key load failure: {0}")]
    SigningKeyLoad(String),

    #[error("signing key derivation failure: {0}")]
    SigningKeyDerive(String),

    #[error("manifest parse failure: {0}")]
    ManifestParse(String),

    #[error("tier mismatch: --tier='{cli_tier}' but manifest declares trust_tier='{manifest_tier}'")]
    TierMismatch {
        cli_tier: String,
        manifest_tier: String,
    },

    #[error("invalid tier '{0}'")]
    InvalidTier(String),

    #[error("compliance claim load failure: {0}")]
    ComplianceClaimLoad(String),

    #[error("registry transport: {0}")]
    Transport(String),

    #[error("trust-tier floor violated: {0}")]
    TrustTierFloorViolated(String),

    #[error("org signature invalid: {0}")]
    OrgSignatureInvalid(String),

    #[error("registry unconfigured: {0}")]
    Unconfigured(String),

    #[error("serde_json: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("serde_cbor: {0}")]
    SerdeCbor(#[from] serde_cbor::Error),

    #[error("{0}")]
    Other(String),
}

impl CliError {
    /// Exit-code mapping per Story 7.2 AC2 narrative.
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Unconfigured(_) => 3,
            CliError::TrustTierFloorViolated(_) | CliError::OrgSignatureInvalid(_) => 2,
            _ => 1,
        }
    }
}
