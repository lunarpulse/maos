//! Authoritative inference-mode selection for the composition root.
//!
//! This module is public so integration tests exercise the production lattice;
//! an in-`src` test module would be budget-charged and CI-invisible.

use std::path::{Path, PathBuf};

/// The single resolved inference mode shared by every inference consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedInferenceMode {
    /// No live flag and no compatibility cassette were supplied.
    Deterministic,
    /// Live inference selected explicitly or by the legacy `--live` flag.
    Live { explicit: bool },
    /// Replay from a cassette. `explicit` distinguishes authoritative mode from
    /// the legacy cassette-presence compatibility path.
    Replay {
        cassette: PathBuf,
        strict: bool,
        explicit: bool,
    },
    /// Record real provider responses to a cassette.
    Record { cassette: PathBuf },
}

impl ResolvedInferenceMode {
    /// Resolve ADR-064's precedence lattice exactly once.
    pub fn resolve(
        mode: Option<&str>,
        cassette: Option<PathBuf>,
        strict: bool,
        cli_live: bool,
        live_provider_available: bool,
    ) -> Result<Self, InferenceModeError> {
        let Some(mode) = mode else {
            return Ok(if cli_live {
                Self::Live { explicit: false }
            } else if let Some(cassette) = cassette {
                Self::Replay {
                    cassette,
                    strict,
                    explicit: false,
                }
            } else {
                Self::Deterministic
            });
        };

        match mode {
            "live" if live_provider_available => Ok(Self::Live { explicit: true }),
            "live" => Err(InferenceModeError::LiveProviderUnavailable),
            "record" => cassette
                .map(|cassette| Self::Record { cassette })
                .ok_or(InferenceModeError::CassetteRequired { mode: "record" }),
            "replay" if cli_live => Err(InferenceModeError::ReplayLiveConflict),
            "replay" => cassette
                .map(|cassette| Self::Replay {
                    cassette,
                    strict,
                    explicit: true,
                })
                .ok_or(InferenceModeError::CassetteRequired { mode: "replay" }),
            value => Err(InferenceModeError::InvalidMode {
                value: value.to_owned(),
            }),
        }
    }

    pub fn is_explicit(&self) -> bool {
        match self {
            Self::Live { explicit } | Self::Replay { explicit, .. } => *explicit,
            Self::Record { .. } => true,
            Self::Deterministic => false,
        }
    }

    pub fn uses_provider_seam(&self) -> bool {
        matches!(
            self,
            Self::Replay { explicit: true, .. } | Self::Record { .. }
        )
    }

    pub fn cassette(&self) -> Option<&Path> {
        match self {
            Self::Replay { cassette, .. } | Self::Record { cassette } => Some(cassette),
            Self::Deterministic | Self::Live { .. } => None,
        }
    }

    pub fn is_replay(&self) -> bool {
        matches!(self, Self::Replay { .. })
    }

    pub fn uses_rate_limiter(&self) -> bool {
        !matches!(self, Self::Replay { explicit: true, .. })
    }
}

/// Fail-closed configuration errors surfaced by the existing main error chain.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InferenceModeError {
    #[error(
        "MAOS_INFERENCE_MODE has unsupported value '{value}' (expected live, record, or replay)"
    )]
    InvalidMode { value: String },
    #[error(
        "MAOS_INFERENCE_MODE is not valid UTF-8 (expected live, record, or replay)"
    )]
    NonUtf8Mode,
    #[error("MAOS_INFERENCE_MODE={mode} requires MAOS_REPLAY_CASSETTE")]
    CassetteRequired { mode: &'static str },
    #[error("MAOS_INFERENCE_MODE=replay conflicts with --live")]
    ReplayLiveConflict,
    #[error("MAOS_INFERENCE_MODE=live requires an explicitly configured inference provider")]
    LiveProviderUnavailable,
}
