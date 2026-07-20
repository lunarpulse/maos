//! Collective Memory Port — sync trait for the out-of-kernel Loom-lite
//! collective tier (Story 10.4a, ADR-006).
//!
//! # Architecture
//!
//! The kernel mediates collective-tier memory access via this injected port
//! trait.  The implementation lives in `maos-loom-lite` (user-space,
//! MCP-Streamable-HTTP, Postgres+pgvector backend).  The kernel stays
//! runtime-agnostic and sync; the async boundary (`spawn_blocking` +
//! runtime handle) is owned by the adapter in `maos-loom-lite`.
//!
//! # Zero-async-dependency guarantee
//!
//! This trait follows the `maos-domain` zero-async contract (`lib.rs:11`):
//! no `async fn`, no tokio/sqlx types.  Only sync trait method signatures.

use crate::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue};
use crate::team::TeamId;

/// Structured causes carried inside the existing transport category so the
/// kernel's tuple pattern stays byte-for-byte unchanged while callers can
/// distinguish tenant and cross-team refusals without parsing strings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportCause {
    #[error("{reason}")]
    Other { reason: String },
    #[error("cross-team consent denied: {from_team}->{to_team}, intent={intent}")]
    ConsentDenied {
        from_team: TeamId,
        to_team: TeamId,
        intent: String,
    },
    #[error("tenant map stale{team_suffix}: {reason}", team_suffix = team_id.as_ref().map(|team| format!(" for {team}")).unwrap_or_default())]
    MapStale {
        team_id: Option<TeamId>,
        reason: String,
    },
    #[error("cross-team attestation invalid for {team_id}: {reason}")]
    AttestationInvalid { team_id: TeamId, reason: String },
    #[error("tenant spirit pid {spirit_pid} is not registered")]
    UnmappedSpirit { spirit_pid: u32 },
    #[error("tenant connection mismatch for store {configured_team}: {reason}")]
    ConnectionMismatch {
        configured_team: TeamId,
        caller_team: Option<TeamId>,
        reason: String,
    },
}

/// Error returned when the collective port is unreachable or times out.
///
/// Per AC1: typed, halt-safe error with a bounded timeout — no panic, no hang.
#[derive(Debug, thiserror::Error)]
pub enum CollectivePortError {
    /// The Loom-lite service is unreachable (connection refused, DNS failure).
    #[error("collective tier unreachable: {reason}")]
    Unreachable { reason: String },

    /// The operation timed out waiting for the Loom-lite service.
    #[error("collective tier timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// A memory-layer error forwarded from the backing store.
    #[error(transparent)]
    Memory(#[from] MemoryError),

    /// Structured transport refusal; the outer variant remains unchanged for
    /// kernel compatibility.
    /// An internal transport or protocol error.
    #[error("collective tier transport error: {0}")]
    Transport(TransportCause),
}

/// Sync port trait for the collective memory tier (Postgres+pgvector Loom-lite).
///
/// Injected into the kernel's `MemoryManagerAdapter` as
/// `Option<Arc<dyn CollectiveMemoryPort>>`.  When `None`, collective-tier
/// operations return `MemoryError::CollectiveNotYetAvailable` (the `:709`
/// variant stays).  When `Some`, the three `MemoryTier::Collective` arms
/// delegate to this port.
///
/// Per architecture:
/// - ADR-006 / I9: user-space, replaceable; the kernel mediates + audits,
///   stores/learns nothing.
/// - I1: capability check BEFORE the port call.
/// - I2: TL log BEFORE the response is delivered.
/// - I11: Loom-persisted patterns carry `source_log_ref` + `distillation_depth`.
///
/// # Class annotations
///
/// All methods are `data-movement` — the port moves frames/values between
/// the kernel mediation layer and the external Loom-lite backing store.
pub trait CollectiveMemoryPort: Send + Sync {
    /// Class: data-movement
    ///
    /// Write a value to the collective tier.  The adapter crosses the
    /// async boundary internally (via `spawn_blocking` + runtime handle).
    ///
    /// `spirit_pid` is kernel-set from the calling context.
    fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), CollectivePortError>;

    /// Class: data-movement
    ///
    /// Read a value from the collective tier.  Returns `Ok(None)` when
    /// no write has been recorded for this key.
    fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, CollectivePortError>;

    /// Class: data-movement
    ///
    /// Scan entries matching a key prefix within the collective tier,
    /// up to `limit` entries.
    fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, CollectivePortError>;
}
