use maos_domain::log_recall::{LogRecallError, LogRecallFilter, LogRecallPage};
use maos_domain::ports::CrossWallLogReadPort;
use maos_domain::team::TeamId;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;

/// Composition-root adapter for consent-governed foreign-team audit reads.
pub struct CrossWallLogReadAdapter {
    collective_configured: bool,
}

impl CrossWallLogReadAdapter {
    pub fn new(collective_configured: bool) -> Self {
        Self {
            collective_configured,
        }
    }

    fn storage(context: &str, error: impl std::fmt::Display) -> LogRecallError {
        LogRecallError::Storage(format!("cross-wall {context}: {error}"))
    }
}

impl CrossWallLogReadPort for CrossWallLogReadAdapter {
    fn read_remote(
        &self,
        spirit_pid: u32,
        remote_team: &TeamId,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        let path = maos_audit::transparency_log_path_for_tenant_mode(
            self.collective_configured,
            Some(remote_team.as_str()),
        )
        .map_err(|error| Self::storage("path derivation failed", error))?;
        maos_audit::validate_transparency_log_path(&path)
            .map_err(|error| Self::storage("path validation failed", error))?;

        // Single read-only NOFOLLOW connection (Story 13.6d P2): the artifact
        // whose binding is verified is the same artifact whose rows are served,
        // closing the binding-vs-rows TOCTOU that two separate opens left open.
        let conn = maos_audit::open_tenant_artifact_readonly(&path)
            .map_err(|error| Self::storage("tenant binding open failed", error))?
            .ok_or_else(|| {
                Self::storage(
                    "tenant binding refused",
                    format_args!("artifact is unbound; requested {remote_team}"),
                )
            })?;
        let artifact = maos_audit::read_tenant_artifact_on(&conn)
            .map_err(|error| Self::storage("tenant binding read failed", error))?;
        let bound_raw = artifact.binding_team.ok_or_else(|| {
            Self::storage(
                "tenant binding refused",
                format_args!("artifact is unbound; requested {remote_team}"),
            )
        })?;
        let bound = TeamId::new(&bound_raw)
            .map_err(|error| Self::storage("tenant binding is non-canonical", error))?;
        if &bound != remote_team {
            return Err(Self::storage(
                "tenant binding refused",
                format_args!("artifact is bound to {bound}; requested {remote_team}"),
            ));
        }

        let reader = TransparencyLogAdapter::from_read_only_connection(conn);
        LogRecallAdapter::query_page(&reader, spirit_pid, filter)
    }
}
