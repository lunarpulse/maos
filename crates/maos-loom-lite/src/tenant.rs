//! Consumer-owned tenant-map seam for the Loom-lite physical tenant wall.
//!
//! The storage adapter defines only the narrow lookup contract. The signed
//! cohort-manifest implementation lives at the daemon composition root so
//! Loom-lite gains no dependency on `maos-cohort`.

use maos_domain::ports::registry::SpiritId;
use maos_domain::team::TeamId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TenantMapError {
    #[error("tenant map is stale: {reason}")]
    Stale { reason: String },
    #[error("tenant spirit pid {spirit_pid} is not registered")]
    SpiritUnmapped { spirit_pid: u32 },
    #[error("tenant team {team_id} is absent from the verified map")]
    TeamUnknown { team_id: TeamId },
    #[error("tenant map state unavailable: {reason}")]
    StateUnavailable { reason: String },
}

pub trait TenantMapPort: Send + Sync {
    fn team_of(&self, spirit_pid: u32) -> Result<TeamId, TenantMapError>;
    fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError>;
    fn register_spirit(&self, spirit_pid: u32, spirit_id: SpiritId);
}
