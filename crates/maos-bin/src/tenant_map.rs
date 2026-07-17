//! Composition-root adapter from verified cohort state to Loom-lite tenancy.
//!
//! This stays out of `api.rs`: it joins the signed cohort authority to the
//! storage consumer port and is daemon wiring, not a kernel-core adapter.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use maos_cohort::{CohortManifest, CohortManifestState, COHORT_SCHEMA_V2};
use maos_domain::ports::registry::SpiritId;
use maos_domain::team::TeamId;
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};

#[derive(Debug, thiserror::Error)]
pub enum TenantMapBootError {
    #[error("tenant mode requires a verified tenant-map source")]
    SourceUnavailable,
    #[error("tenant-map source is not refreshable (peerless/N=1 is unsupported)")]
    SourceUnrefreshable,
    #[error("tenant mode requires a verified schema-v2 cohort manifest")]
    SchemaV2Required,
    #[error("local host {host_id} is absent from the verified cohort roster")]
    LocalHostEvicted { host_id: String },
    #[error(
        "tenant-map local host mismatch: state is bound to {state_host}, requested {requested_host}"
    )]
    LocalHostMismatch {
        state_host: String,
        requested_host: String,
    },
    #[error("invalid configured home team: {0}")]
    InvalidHomeTeam(String),
    #[error("verified tenant-map state unavailable: {0}")]
    StateUnavailable(String),
}

pub struct TenantMapAdapter {
    state: Arc<CohortManifestState>,
    local_host: String,
    spirit_bindings: Mutex<HashMap<u32, SpiritId>>,
}

impl TenantMapAdapter {
    pub fn new(
        state: Arc<CohortManifestState>,
        local_host: impl Into<String>,
        refreshable_source: bool,
    ) -> Result<Self, TenantMapBootError> {
        let local_host = local_host.into();
        if state.local_host().as_str() != local_host {
            return Err(TenantMapBootError::LocalHostMismatch {
                state_host: state.local_host().as_str().to_string(),
                requested_host: local_host,
            });
        }
        if !refreshable_source {
            return Err(TenantMapBootError::SourceUnrefreshable);
        }
        let manifest = state
            .manifest()
            .map_err(|error| TenantMapBootError::StateUnavailable(error.to_string()))?;
        if manifest.members.len() < 2 {
            return Err(TenantMapBootError::SourceUnrefreshable);
        }
        validate_tenant_manifest(&state, &manifest, &local_host)?;
        Ok(Self {
            state,
            local_host,
            spirit_bindings: Mutex::new(HashMap::new()),
        })
    }

    fn current_manifest(&self) -> Result<CohortManifest, TenantMapError> {
        if !self.state.is_fresh() {
            return Err(TenantMapError::Stale {
                reason: "signed manifest lease expired".to_string(),
            });
        }
        let manifest = self
            .state
            .manifest()
            .map_err(|error| TenantMapError::StateUnavailable {
                reason: error.to_string(),
            })?;
        if manifest.schema_version != COHORT_SCHEMA_V2
            || manifest.teams.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(TenantMapError::Stale {
                reason: "verified schema-v2 tenant map is unavailable".to_string(),
            });
        }
        if !manifest
            .members
            .iter()
            .any(|member| member.host_id == self.local_host)
        {
            return Err(TenantMapError::Stale {
                reason: format!("local host {} was evicted from the cohort", self.local_host),
            });
        }
        Ok(manifest)
    }
}

impl TenantMapPort for TenantMapAdapter {
    fn team_of(&self, spirit_pid: u32) -> Result<TeamId, TenantMapError> {
        let spirit_id = self
            .spirit_bindings
            .lock()
            .map_err(|_| TenantMapError::StateUnavailable {
                reason: "spirit binding lock poisoned".to_string(),
            })?
            .get(&spirit_pid)
            .cloned()
            .ok_or(TenantMapError::SpiritUnmapped { spirit_pid })?;
        let manifest = self.current_manifest()?;
        manifest
            .teams
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|team| team.members.iter().any(|member| member == &spirit_id))
            .map(|team| team.team_id.clone())
            .ok_or(TenantMapError::SpiritUnmapped { spirit_pid })
    }

    fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
        let manifest = self.current_manifest()?;
        manifest
            .teams
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|entry| &entry.team_id == team)
            .map(|entry| entry.datname.clone())
            .ok_or_else(|| TenantMapError::TeamUnknown {
                team_id: team.clone(),
            })
    }

    fn register_spirit(&self, spirit_pid: u32, spirit_id: SpiritId) {
        if let Ok(mut bindings) = self.spirit_bindings.lock() {
            bindings.insert(spirit_pid, spirit_id);
        }
    }
}

pub fn tenant_map_for_store(
    home_team: &str,
    source: Option<Arc<dyn TenantMapPort>>,
) -> Result<Option<Arc<dyn TenantMapPort>>, TenantMapBootError> {
    if home_team.is_empty() {
        return Ok(None);
    }
    TeamId::new(home_team)
        .map_err(|error| TenantMapBootError::InvalidHomeTeam(error.to_string()))?;
    source
        .map(Some)
        .ok_or(TenantMapBootError::SourceUnavailable)
}

fn validate_tenant_manifest(
    state: &CohortManifestState,
    manifest: &CohortManifest,
    local_host: &str,
) -> Result<(), TenantMapBootError> {
    if !state.is_fresh()
        || manifest.schema_version != COHORT_SCHEMA_V2
        || manifest.teams.as_ref().is_none_or(Vec::is_empty)
    {
        return Err(TenantMapBootError::SchemaV2Required);
    }
    if !manifest
        .members
        .iter()
        .any(|member| member.host_id == local_host)
    {
        return Err(TenantMapBootError::LocalHostEvicted {
            host_id: local_host.to_string(),
        });
    }
    Ok(())
}
