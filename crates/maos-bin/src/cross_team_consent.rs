use std::collections::HashMap;
use std::sync::Arc;

use maos_audit::sealed_export::derive_team_pubkey;
use maos_cohort::CohortManifestState;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_loom_lite::cross_team_consent::{CrossTeamConsentError, CrossTeamConsentPort};

/// Composition-root adapter from the verified cohort lease to Loom-lite's
/// consumer-owned directional consent seam.
pub struct CrossTeamConsentAdapter {
    state: Arc<CohortManifestState>,
}

impl CrossTeamConsentAdapter {
    pub fn new(state: Arc<CohortManifestState>) -> Self {
        Self { state }
    }
}

impl CrossTeamConsentPort for CrossTeamConsentAdapter {
    fn is_granted(
        &self,
        from_team: &TeamId,
        to_team: &TeamId,
        intent: &str,
    ) -> Result<bool, CrossTeamConsentError> {
        // Freshness and the manifest come from ONE locked snapshot: a lease
        // turning stale between two separate lock acquisitions must not
        // admit a grant (13.3 review).
        let manifest = self
            .state
            .manifest_if_fresh()
            .map_err(|error| CrossTeamConsentError::StateUnavailable {
                reason: error.to_string(),
            })?
            .ok_or_else(|| CrossTeamConsentError::Stale {
                reason: "signed cohort manifest lease expired".to_string(),
            })?;
        Ok(manifest.cross_team_admits(from_team, to_team, intent))
    }
}

/// Derive the public team-key set at the composition root. The optional
/// `MAOS_CROSS_TEAM_BASE_SEED` input is a 32-byte hex seed; only derived public
/// keys cross into Loom-lite. Absence leaves the map empty, making cross-team
/// reads fail closed while the crossing remains unwired.
pub fn team_verifying_keys_from_env(
    state: &CohortManifestState,
) -> Result<HashMap<(Region, TeamId), [u8; 32]>, String> {
    let raw_seed = match std::env::var("MAOS_CROSS_TEAM_BASE_SEED") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(HashMap::new()),
        Err(error) => return Err(format!("MAOS_CROSS_TEAM_BASE_SEED is unreadable: {error}")),
    };
    let decoded = hex::decode(raw_seed.trim())
        .map_err(|error| format!("MAOS_CROSS_TEAM_BASE_SEED must be 64 hex characters: {error}"))?;
    let base_seed = <[u8; 32]>::try_from(decoded.as_slice())
        .map_err(|_| "MAOS_CROSS_TEAM_BASE_SEED must decode to exactly 32 bytes".to_string())?;
    derive_team_verifying_keys(state, &base_seed)
}

pub fn derive_team_verifying_keys(
    state: &CohortManifestState,
    base_seed: &[u8; 32],
) -> Result<HashMap<(Region, TeamId), [u8; 32]>, String> {
    let manifest = state
        .manifest()
        .map_err(|error| format!("verified cohort manifest unavailable: {error}"))?;
    Ok(manifest
        .teams
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|team| {
            (
                (team.region.clone(), team.team_id.clone()),
                derive_team_pubkey(base_seed, &team.region, &team.team_id),
            )
        })
        .collect())
}
