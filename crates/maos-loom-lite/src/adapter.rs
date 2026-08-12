#![forbid(unsafe_code)]

//! `CollectiveMemoryPort` adapter — crosses the sync→async boundary via
//! `spawn_blocking` + an injected `tokio::runtime::Handle`.
//!
//! The kernel is invoked from inside a tokio task (MCP-Streamable-HTTP);
//! a naive `Handle::current().block_on()` would deadlock under load
//! (nested-runtime panic).  Instead, the MCP→kernel edge uses
//! `spawn_blocking`, and the adapter holds a runtime handle to re-enter
//! the async world from the blocking thread.
//!
//! Topology: MCP streamable-http (async) → spawn_blocking → sync kernel-core
//!   → this adapter (sync trait) → block_on(async store op) on the injected handle.
//!
//! The `block_on` is guarded with `catch_unwind`: calls from an async worker
//! (nested-runtime panic) and calls through a shut-down handle both map to a
//! typed `CollectivePortError::Unreachable`. A Tokio `spawn_blocking` thread is
//! allowed even though `Handle::try_current()` is available there.

use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use maos_domain::memory::{MemoryEntry, MemoryNamespace, MemoryValue};
use maos_domain::ports::collective_memory::{
    CollectiveEraseReceipt, CollectiveMemoryPort, CollectivePortError, TransportCause,
};

use crate::store::{LoomLiteStore, StoreError};

/// Adapter that bridges the sync `CollectiveMemoryPort` to the async
/// `LoomLiteStore` via an injected `tokio::runtime::Handle`.
pub struct LoomLiteAdapter {
    store: Arc<LoomLiteStore>,
    handle: tokio::runtime::Handle,
    timeout: Duration,
}

impl LoomLiteAdapter {
    /// Create a new adapter with the given store and runtime handle.
    ///
    /// The `handle` is the tokio runtime handle owned at the daemon
    /// composition root.  The adapter uses it to `block_on` async store
    /// operations from within a `spawn_blocking` context.
    pub fn new(
        store: Arc<LoomLiteStore>,
        handle: tokio::runtime::Handle,
        timeout: Duration,
    ) -> Self {
        Self {
            store,
            handle,
            timeout,
        }
    }
}

/// Run `fut` on the injected handle, mapping a nested-runtime or shut-down
/// panic to typed `Unreachable` instead of unwinding into the kernel.
///
/// Do not preflight with `Handle::try_current()`: Tokio makes the current
/// handle available inside `spawn_blocking`, which is precisely the supported
/// bridge topology and can safely call `Handle::block_on`.
fn block_on_or_typed<F, T>(
    handle: &tokio::runtime::Handle,
    fut: F,
) -> Result<T, CollectivePortError>
where
    F: Future<Output = T>,
{
    match catch_unwind(AssertUnwindSafe(|| handle.block_on(fut))) {
        Ok(v) => Ok(v),
        Err(_) => Err(CollectivePortError::Unreachable {
            reason: "collective-tier runtime bridge unavailable (nested runtime or shutdown)"
                .into(),
        }),
    }
}

impl CollectiveMemoryPort for LoomLiteAdapter {
    fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), CollectivePortError> {
        let store = Arc::clone(&self.store);
        let ns = namespace.clone();
        let k = key.to_string();
        let timeout = self.timeout;
        let timeout_ms = self.timeout.as_millis() as u64;

        block_on_or_typed(&self.handle, async move {
            tokio::time::timeout(timeout, store.write(spirit_pid, &ns, &k, value)).await
        })?
        .map_err(|_| CollectivePortError::Timeout { timeout_ms })?
        .map_err(store_error_to_port_error)
    }

    fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, CollectivePortError> {
        let store = Arc::clone(&self.store);
        let ns = namespace.clone();
        let k = key.to_string();
        let timeout = self.timeout;
        let timeout_ms = self.timeout.as_millis() as u64;

        block_on_or_typed(&self.handle, async move {
            tokio::time::timeout(timeout, store.read(spirit_pid, &ns, &k)).await
        })?
        .map_err(|_| CollectivePortError::Timeout { timeout_ms })?
        .map_err(store_error_to_port_error)
    }

    fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, CollectivePortError> {
        let store = Arc::clone(&self.store);
        let ns = namespace.clone();
        let p = prefix.to_string();
        let timeout = self.timeout;
        let timeout_ms = self.timeout.as_millis() as u64;

        block_on_or_typed(&self.handle, async move {
            tokio::time::timeout(timeout, store.scan(spirit_pid, &ns, &p, limit)).await
        })?
        .map_err(|_| CollectivePortError::Timeout { timeout_ms })?
        .map_err(store_error_to_port_error)
    }

    fn erase(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<CollectiveEraseReceipt, CollectivePortError> {
        let store = Arc::clone(&self.store);
        let ns = namespace.clone();
        let k = key.to_string();
        let timeout = self.timeout;
        let timeout_ms = self.timeout.as_millis() as u64;

        block_on_or_typed(&self.handle, async move {
            tokio::time::timeout(timeout, store.erase(spirit_pid, &ns, &k)).await
        })?
        .map_err(|_| CollectivePortError::Timeout { timeout_ms })?
        .map_err(store_error_to_port_error)
    }
}

/// Map store errors to port errors with halt-safe semantics.
///
/// `pub` since Story 13.6b: the cross-team crossing applier
/// (`maos-bin/src/cross_team_crossing.rs`) runs its refusals through this SAME
/// mapping before projecting them onto the wire, so the five-cause matrix keeps
/// holding inside the applier process — the only boundary `TransportCause` has
/// ever crossed (D-15).
pub fn store_error_to_port_error(error: StoreError) -> CollectivePortError {
    match error {
        StoreError::Pool(reason) => CollectivePortError::Unreachable { reason },
        StoreError::Timeout { timeout_ms } => CollectivePortError::Timeout { timeout_ms },
        StoreError::Query(reason)
        | StoreError::Schema(reason)
        | StoreError::Serialization(reason)
        | StoreError::AtRestSeal(reason) => {
            CollectivePortError::Transport(TransportCause::Other { reason })
        }
        StoreError::PrincipalNamespaceForbidden {
            principal_id,
            schema,
        } => CollectivePortError::Transport(TransportCause::PartitionRefused {
            namespace: format!("principal:{principal_id}:{schema}"),
        }),
        StoreError::ErasureTombstoneDominates {
            key,
            erased_at_source_ts,
            erased_at_source_region,
        } => CollectivePortError::Transport(TransportCause::ErasureTombstoneDominates {
            key,
            erased_at_source_ts,
            erased_at_source_region,
        }),
        StoreError::StaleGeneration => CollectivePortError::Transport(TransportCause::Other {
            reason: "collective row generation changed before erase".to_string(),
        }),
        StoreError::TenantMapStale { team_id, reason } => {
            CollectivePortError::Transport(TransportCause::MapStale { team_id, reason })
        }
        // NOTE (13.3 review): ConsentDenied/ConsentStateStale currently have
        // no PRODUCTION constructor — the crossing refuses with `BundleError`
        // below the port, and no initiator is wired until 13.5c. The mapping
        // is proven by the five-cause matrix; runtime reachability arrives
        // with the crossing initiator.
        StoreError::ConsentStateStale { reason } => {
            CollectivePortError::Transport(TransportCause::MapStale {
                team_id: None,
                reason,
            })
        }
        StoreError::TenantConnectionMismatch {
            configured_team,
            caller_team,
            reason,
        } => CollectivePortError::Transport(TransportCause::ConnectionMismatch {
            configured_team,
            caller_team,
            reason,
        }),
        StoreError::TenantSpiritUnmapped { spirit_pid } => {
            CollectivePortError::Transport(TransportCause::UnmappedSpirit { spirit_pid })
        }
        StoreError::ConsentDenied {
            from_team,
            to_team,
            intent,
        } => CollectivePortError::Transport(TransportCause::ConsentDenied {
            from_team,
            to_team,
            intent,
        }),
        StoreError::AttestationInvalid { team_id, reason } => {
            CollectivePortError::Transport(TransportCause::AttestationInvalid { team_id, reason })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::collective_memory::TransportCause;
    use maos_domain::team::TeamId;

    #[test]
    fn five_tenant_consent_causes_remain_distinguishable() {
        let team_a = TeamId::new("team-a").unwrap();
        let team_b = TeamId::new("team-b").unwrap();
        let errors = [
            StoreError::ConsentDenied {
                from_team: team_a.clone(),
                to_team: team_b.clone(),
                intent: "collective:share".to_string(),
            },
            StoreError::TenantMapStale {
                team_id: Some(team_a.clone()),
                reason: "lease expired".to_string(),
            },
            StoreError::AttestationInvalid {
                team_id: team_a.clone(),
                reason: "proof mismatch".to_string(),
            },
            StoreError::TenantSpiritUnmapped { spirit_pid: 7 },
            StoreError::TenantConnectionMismatch {
                configured_team: team_b.clone(),
                caller_team: Some(team_a.clone()),
                reason: "caller team differs".to_string(),
            },
        ];
        let mapped: Vec<CollectivePortError> =
            errors.into_iter().map(store_error_to_port_error).collect();
        // Payload-field assertions (13.3 review): the structured fields are
        // AC4's replacement for string-matching, so the matrix must prove
        // every field survives the mapping — not just each discriminant.
        let [consent_denied, map_stale, attestation_invalid, unmapped_spirit, connection_mismatch] =
            mapped.try_into().expect("five mapped causes");
        assert!(
            matches!(
                &consent_denied,
                CollectivePortError::Transport(TransportCause::ConsentDenied {
                    from_team,
                    to_team,
                    intent,
                }) if from_team == &team_a
                    && to_team == &team_b
                    && intent == "collective:share"
            ),
            "consent-denied must carry the ordered pair and intent: {consent_denied:?}"
        );
        assert!(
            matches!(
                &map_stale,
                CollectivePortError::Transport(TransportCause::MapStale {
                    team_id,
                    reason,
                }) if team_id.as_ref() == Some(&team_a) && reason == "lease expired"
            ),
            "map-stale must carry the team identity and reason: {map_stale:?}"
        );
        assert!(
            matches!(
                &attestation_invalid,
                CollectivePortError::Transport(TransportCause::AttestationInvalid {
                    team_id,
                    reason,
                }) if team_id == &team_a && reason == "proof mismatch"
            ),
            "attestation-invalid must carry the team and reason: {attestation_invalid:?}"
        );
        assert!(
            matches!(
                &unmapped_spirit,
                CollectivePortError::Transport(TransportCause::UnmappedSpirit { spirit_pid: 7 })
            ),
            "unmapped-spirit must carry the spirit pid: {unmapped_spirit:?}"
        );
        assert!(
            matches!(
                &connection_mismatch,
                CollectivePortError::Transport(TransportCause::ConnectionMismatch {
                    configured_team,
                    caller_team,
                    reason,
                }) if configured_team == &team_b
                    && caller_team.as_ref() == Some(&team_a)
                    && reason == "caller team differs"
            ),
            "connection-mismatch must carry both teams and reason: {connection_mismatch:?}"
        );
    }

    #[test]
    fn erasure_tombstone_cause_remains_typed_across_port() {
        let mapped = store_error_to_port_error(StoreError::ErasureTombstoneDominates {
            key: "erased-key".into(),
            erased_at_source_ts: 42,
            erased_at_source_region: "region-a".into(),
        });
        assert!(matches!(
            mapped,
            CollectivePortError::Transport(TransportCause::ErasureTombstoneDominates {
                ref key,
                erased_at_source_ts: 42,
                ref erased_at_source_region,
            }) if key == "erased-key" && erased_at_source_region == "region-a"
        ));
    }
}
