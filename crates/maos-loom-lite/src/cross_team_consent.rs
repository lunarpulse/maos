//! Consumer-owned seam for directional cross-team row consent.
//!
//! Loom-lite owns the narrow decision contract. The signed-manifest adapter
//! lives at the daemon composition root so this crate does not depend on
//! `maos-cohort`.

use maos_domain::team::TeamId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrossTeamConsentError {
    #[error("cross-team consent state is stale: {reason}")]
    Stale { reason: String },
    #[error("cross-team consent state is unavailable: {reason}")]
    StateUnavailable { reason: String },
}

pub trait CrossTeamConsentPort: Send + Sync {
    /// Return `Ok(true)` only for an exact directional grant. `Ok(false)` is a
    /// current no-grant decision; stale/unavailable state is a typed error.
    fn is_granted(
        &self,
        from_team: &TeamId,
        to_team: &TeamId,
        intent: &str,
    ) -> Result<bool, CrossTeamConsentError>;
}
