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
use maos_domain::ports::collective_memory::{CollectiveMemoryPort, CollectivePortError};

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
}

/// Map store errors to port errors with halt-safe semantics.
fn store_error_to_port_error(e: StoreError) -> CollectivePortError {
    match e {
        StoreError::Pool(reason) => CollectivePortError::Unreachable { reason },
        StoreError::Timeout { timeout_ms } => CollectivePortError::Timeout { timeout_ms },
        StoreError::Query(msg) => CollectivePortError::Transport(msg),
        StoreError::Schema(msg) => CollectivePortError::Transport(msg),
        StoreError::Serialization(msg) => CollectivePortError::Transport(msg),
        StoreError::AtRestSeal(msg) => CollectivePortError::Transport(msg),
        StoreError::TenantMapStale(reason) => CollectivePortError::Transport(reason),
        StoreError::TenantConnectionMismatch(reason) => CollectivePortError::Transport(reason),
        StoreError::TenantSpiritUnmapped(spirit_pid) => CollectivePortError::Transport(format!(
            "tenant spirit pid {spirit_pid} is not registered"
        )),
    }
}
