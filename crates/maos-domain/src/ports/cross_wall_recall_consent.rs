#![forbid(unsafe_code)]

//! Consumer-owned directional consent seam for cross-wall log recall.

use crate::team::TeamId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrossWallRecallConsentError {
    #[error("cross-wall recall consent state is stale: {reason}")]
    Stale { reason: String },
    #[error("cross-wall recall consent state is unavailable: {reason}")]
    StateUnavailable { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossWallRecallConsentDecision {
    Granted,
    NoGrant,
    WrongDirection,
}

pub trait CrossWallRecallConsentPort: Send + Sync + 'static {
    /// Resolve the exact directional grant. A reverse-only grant is a distinct
    /// refusal from complete absence.
    /// Class: data-movement
    fn decide(
        &self,
        team: &TeamId,
        intent: &str,
    ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError>;
}
