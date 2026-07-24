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
    #[error("tenant mode requires a verified cohort manifest at schema v2 or newer")]
    SchemaV2FloorRequired,
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
    #[error(
        "ETenantAuditPathMismatch: Transparency Log opened at {opened}, but validated team \
         {team_id} requires {expected}"
    )]
    TenantAuditPathMismatch {
        opened: String,
        expected: String,
        team_id: TeamId,
    },
    #[error(
        "ETenantAuditArtifactMismatch: Transparency Log artifact is not bound to validated team \
         {team_id}: {reason}"
    )]
    TenantAuditArtifactMismatch { team_id: TeamId, reason: String },
}

pub struct TenantMapAdapter {
    state: Arc<CohortManifestState>,
    local_host: String,
    spirit_bindings: Mutex<HashMap<u32, SpiritId>>,
}

fn tenant_schema_meets_floor(schema_version: u64) -> bool {
    schema_version >= COHORT_SCHEMA_V2
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
        if !tenant_schema_meets_floor(manifest.schema_version)
            || manifest.teams.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(TenantMapError::Stale {
                reason: "verified schema-v2-or-newer tenant map is unavailable".to_string(),
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

/// Defense-in-depth path-consistency self-check: confirms the opened TL path
/// matches the team the process is configured to serve.
///
/// The manifest-team AUTHORITY is the store's `connection_assignment_guard`
/// (datname validated against the verified cohort manifest, fail-closed in
/// `init_schema` before this runs). This reconcile is a SECONDARY drift check,
/// not an independent team control: today both operands derive from the
/// configured home team, so it guards against path/team drift, not against a
/// manifest disagreement. See Story 13.5e review (D1 re-scope).
pub fn reconcile_tenant_audit_path(
    opened: &std::path::Path,
    validated_team: &TeamId,
) -> Result<(), TenantMapBootError> {
    let expected = maos_audit::transparency_log_path_for_team(validated_team);
    if opened == expected {
        return Ok(());
    }
    Err(TenantMapBootError::TenantAuditPathMismatch {
        opened: opened.display().to_string(),
        expected: expected.display().to_string(),
        team_id: validated_team.clone(),
    })
}

/// Atomically bind a newly created team artifact, or validate its existing
/// manifest-derived binding. A copied or renamed foreign shard therefore
/// refuses even when its destination path has the expected team spelling.
pub fn bind_tenant_audit_artifact(
    path: &std::path::Path,
    validated_team: &TeamId,
) -> Result<(), TenantMapBootError> {
    use std::io::Write;

    let binding_path = maos_audit::transparency_log_team_binding_path(path);
    maos_audit::validate_transparency_log_path(&binding_path).map_err(|error| {
        TenantMapBootError::TenantAuditArtifactMismatch {
            team_id: validated_team.clone(),
            reason: error.to_string(),
        }
    })?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&binding_path)
    {
        Ok(mut file) => {
            let result = writeln!(file, "{validated_team}").and_then(|_| file.sync_all());
            if let Err(error) = result {
                drop(file);
                let _ = std::fs::remove_file(&binding_path);
                return Err(TenantMapBootError::TenantAuditArtifactMismatch {
                    team_id: validated_team.clone(),
                    reason: format!("failed to persist {}: {error}", binding_path.display()),
                });
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            maos_audit::validate_transparency_log_team_binding(path, validated_team).map_err(
                |error| TenantMapBootError::TenantAuditArtifactMismatch {
                    team_id: validated_team.clone(),
                    reason: error.to_string(),
                },
            )
        }
        Err(error) => Err(TenantMapBootError::TenantAuditArtifactMismatch {
            team_id: validated_team.clone(),
            reason: format!("failed to create {}: {error}", binding_path.display()),
        }),
    }
}

fn validate_tenant_manifest(
    state: &CohortManifestState,
    manifest: &CohortManifest,
    local_host: &str,
) -> Result<(), TenantMapBootError> {
    if !state.is_fresh()
        || !tenant_schema_meets_floor(manifest.schema_version)
        || manifest.teams.as_ref().is_none_or(Vec::is_empty)
    {
        return Err(TenantMapBootError::SchemaV2FloorRequired);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_schema_support_is_a_v2_floor() {
        assert!(!tenant_schema_meets_floor(1));
        assert!(tenant_schema_meets_floor(2));
        assert!(tenant_schema_meets_floor(3));
    }

    #[test]
    fn tenant_audit_path_must_match_manifest_validated_team() {
        let security = TeamId::new("security").unwrap();
        let support = TeamId::new("support").unwrap();
        let opened = maos_audit::transparency_log_path_for_team(&security);

        assert!(reconcile_tenant_audit_path(&opened, &security).is_ok());
        assert!(matches!(
            reconcile_tenant_audit_path(&opened, &support),
            Err(TenantMapBootError::TenantAuditPathMismatch { .. })
        ));
    }

    #[test]
    fn tenant_audit_artifact_binding_rejects_operator_rename_confusion() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("transparency.sqlite");
        std::fs::write(&path, b"sqlite-placeholder").unwrap();
        let security = TeamId::new("security").unwrap();
        let support = TeamId::new("support").unwrap();

        bind_tenant_audit_artifact(&path, &security).unwrap();
        maos_audit::validate_transparency_log_team_binding(&path, &security).unwrap();
        assert!(matches!(
            bind_tenant_audit_artifact(&path, &support),
            Err(TenantMapBootError::TenantAuditArtifactMismatch { .. })
        ));
    }
}
