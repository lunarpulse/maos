use std::collections::HashMap;
use std::sync::Arc;

use maos_audit::sealed_export::derive_team_pubkey;
use maos_cohort::CohortManifestState;
use maos_domain::ports::{
    CrossWallRecallConsentDecision, CrossWallRecallConsentError, CrossWallRecallConsentPort,
};
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

/// Directional cross-wall recall consent over the same verified manifest state
/// used by tenant placement and collective crossings.
pub struct CrossWallRecallConsentAdapter {
    state: Arc<CohortManifestState>,
    home_team: TeamId,
}

impl CrossWallRecallConsentAdapter {
    pub fn new(state: Arc<CohortManifestState>, home_team: TeamId) -> Self {
        Self { state, home_team }
    }
}

impl CrossWallRecallConsentPort for CrossWallRecallConsentAdapter {
    fn decide(
        &self,
        remote_team: &TeamId,
        intent: &str,
    ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
        let manifest = self
            .state
            .manifest_if_fresh()
            .map_err(|error| CrossWallRecallConsentError::StateUnavailable {
                reason: error.to_string(),
            })?
            .ok_or_else(|| CrossWallRecallConsentError::Stale {
                reason: "signed cohort manifest lease expired".to_string(),
            })?;
        if manifest.cross_team_admits(&self.home_team, remote_team, intent) {
            Ok(CrossWallRecallConsentDecision::Granted)
        } else if manifest.cross_team_admits(remote_team, &self.home_team, intent) {
            Ok(CrossWallRecallConsentDecision::WrongDirection)
        } else {
            Ok(CrossWallRecallConsentDecision::NoGrant)
        }
    }
}

/// Read the optional `MAOS_CROSS_TEAM_BASE_SEED` root: a 32-byte hex seed.
///
/// ⚠ **Story 13.6b widened this from a verify-side input to a SIGN-side one
/// (D-7).** Before 13.6b production read the seed only to derive *public*
/// cross-team row-verification keys — a host that held it could check other
/// teams' signatures but never make one. The crossing emitter needs it on the
/// sign side (`build_replication_bundle_v2` → `derive_team_signing_seed`), and
/// because that derivation works for **any** `(region, team)`, every emitter can
/// now produce a validly-signed bundle under every team's key. That is why the
/// applier's envelope/payload weld exists at all, and why the surviving limit is
/// an operator-trust limit rather than a cryptographic one — see
/// `docs/security/loom-threat-model.md` T1.
pub fn cross_team_base_seed_from_env() -> Result<Option<[u8; 32]>, String> {
    let raw_seed = match std::env::var("MAOS_CROSS_TEAM_BASE_SEED") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("MAOS_CROSS_TEAM_BASE_SEED is unreadable: {error}")),
    };
    let decoded = hex::decode(raw_seed.trim())
        .map_err(|error| format!("MAOS_CROSS_TEAM_BASE_SEED must be 64 hex characters: {error}"))?;
    <[u8; 32]>::try_from(decoded.as_slice())
        .map(Some)
        .map_err(|_| "MAOS_CROSS_TEAM_BASE_SEED must decode to exactly 32 bytes".to_string())
}

/// Derive the public team-key set at the composition root. Only derived public
/// keys cross into Loom-lite. Absence leaves the map empty, making cross-team
/// reads fail closed while the crossing remains unwired.
pub fn team_verifying_keys_from_env(
    state: &CohortManifestState,
) -> Result<HashMap<(Region, TeamId), [u8; 32]>, String> {
    match cross_team_base_seed_from_env()? {
        Some(base_seed) => derive_team_verifying_keys(state, &base_seed),
        None => Ok(HashMap::new()),
    }
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
