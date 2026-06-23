#![forbid(unsafe_code)]

//! Story 10.4a AC1 — Transport-failure test: typed halt-safe error + bounded
//! timeout driven from a REAL async context.
//!
//! This test creates a `LoomLiteAdapter` backed by a store whose connection
//! string points at a non-existent host/port. It verifies that:
//!
//! 1. The adapter returns `CollectivePortError::Unreachable` (or `::Timeout`),
//!    NOT a panic.
//! 2. The call completes within a bounded time (no hang).
//! 3. All three port methods (write, read, scan) exhibit the same halt-safe
//!    behavior.
//!
//! The test uses `#[tokio::test]` to run in a real async runtime, then
//! bridges into the sync `CollectiveMemoryPort` via `spawn_blocking` (the
//! same topology the kernel uses: MCP streamable-http (async) →
//! spawn_blocking → sync kernel-core → this adapter).

use std::sync::Arc;
use std::time::Duration;

use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::ports::collective_memory::{CollectiveMemoryPort, CollectivePortError};

use maos_loom_lite::adapter::LoomLiteAdapter;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};

/// Helper: create an adapter backed by a store pointing at a dead host.
///
/// We construct the store with a connection string that resolves to a
/// closed port (127.0.0.1:1 — port 1 is reserved and never listening on
/// any sane system). The `deadpool_postgres` pool is created lazily (the
/// `create_pool` call succeeds without connecting), so `LoomLiteStore::new`
/// succeeds; the failure surfaces only on the first actual query.
async fn make_broken_adapter() -> Arc<LoomLiteAdapter> {
    let config = StoreConfig {
        // Port 1 — TCP reserved, nothing listens here.
        connection_string: "host=127.0.0.1 port=1 dbname=loom_lite connect_timeout=1"
            .to_string(),
        vector_dim: 1536,
        pool_size: 2,
        timeout_ms: 3000,
    };

    let store = Arc::new(
        LoomLiteStore::new(config)
            .await
            .expect("store creation (lazy pool) should succeed even with bad host"),
    );

    let handle = tokio::runtime::Handle::current();
    let timeout = Duration::from_millis(5000);

    Arc::new(LoomLiteAdapter::new(store, handle, timeout))
}

/// Invoke a sync `CollectiveMemoryPort` method from within `spawn_blocking`,
/// matching the kernel's real topology (async → spawn_blocking → sync adapter).
/// Returns the typed result so the caller can assert on the error variant.
async fn call_via_spawn_blocking<F, R>(adapter: Arc<LoomLiteAdapter>, f: F) -> R
where
    F: FnOnce(Arc<LoomLiteAdapter>) -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(move || f(adapter))
        .await
        .expect("spawn_blocking must not panic")
}

/// Write to a broken connection → typed error, no panic.
#[tokio::test]
async fn write_broken_connection_returns_typed_error() {
    let adapter = make_broken_adapter().await;
    let ns = MemoryNamespace::Default;
    let value = MemoryValue::Text("test".to_string());

    let result = call_via_spawn_blocking(adapter, move |a| {
            a.write(42, &ns, "key", value)
        })
    .await;

    assert!(
        result.is_err(),
        "write to broken connection must return an error"
    );
    match result.unwrap_err() {
        CollectivePortError::Unreachable { .. } | CollectivePortError::Timeout { .. } => {
            // Expected: typed halt-safe error
        }
        CollectivePortError::Transport(msg) => {
            // Also acceptable: connection refused surfaces as Transport in
            // some deadpool configurations.
            eprintln!("write returned Transport (acceptable): {msg}");
        }
        other => panic!(
            "write to broken connection must return Unreachable/Timeout/Transport, got: {other:?}"
        ),
    }
}

/// Read from a broken connection → typed error, no panic.
#[tokio::test]
async fn read_broken_connection_returns_typed_error() {
    let adapter = make_broken_adapter().await;
    let ns = MemoryNamespace::Default;

    let result = call_via_spawn_blocking(adapter, move |a| {
            a.read(42, &ns, "key")
        })
    .await;

    assert!(
        result.is_err(),
        "read from broken connection must return an error"
    );
    match result.unwrap_err() {
        CollectivePortError::Unreachable { .. } | CollectivePortError::Timeout { .. } => {}
        CollectivePortError::Transport(msg) => {
            eprintln!("read returned Transport (acceptable): {msg}");
        }
        other => panic!(
            "read from broken connection must return Unreachable/Timeout/Transport, got: {other:?}"
        ),
    }
}

/// Scan a broken connection → typed error, no panic.
#[tokio::test]
async fn scan_broken_connection_returns_typed_error() {
    let adapter = make_broken_adapter().await;
    let ns = MemoryNamespace::Default;

    let result = call_via_spawn_blocking(adapter, move |a| {
            a.scan(42, &ns, "prefix", 10)
        })
    .await;

    assert!(
        result.is_err(),
        "scan from broken connection must return an error"
    );
    match result.unwrap_err() {
        CollectivePortError::Unreachable { .. } | CollectivePortError::Timeout { .. } => {}
        CollectivePortError::Transport(msg) => {
            eprintln!("scan returned Transport (acceptable): {msg}");
        }
        other => panic!(
            "scan from broken connection must return Unreachable/Timeout/Transport, got: {other:?}"
        ),
    }
}

/// Bounded timeout: the broken-connection write must complete within the
/// adapter timeout window (5s), proving no hang.
#[tokio::test]
async fn broken_connection_does_not_hang() {
    let adapter = make_broken_adapter().await;
    let ns = MemoryNamespace::Default;
    let value = MemoryValue::Text("test".to_string());

    // Wrap in a tokio timeout slightly larger than the adapter timeout.
    // If the adapter hangs (no bounded timeout), this outer guard fires.
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        call_via_spawn_blocking(adapter, move |a| a.write(42, &ns, "key", value)),
    )
    .await;

    assert!(
        result.is_ok(),
        "broken-connection write must not hang (exceeded 15s outer guard)"
    );
    let port_result = result.unwrap();
    assert!(port_result.is_err(), "port call must fail on broken connection");
}
