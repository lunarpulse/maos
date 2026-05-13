#![forbid(unsafe_code)]

//! `maos-bin` — MAOS Host composition root.
//!
//! Wires the supervisor (Spirit Scheduler) and the four supervised services
//! (Security / Memory / IAC / Capability) plus two internal modules (I/O /
//! Telemetry) under a single multi-threaded Tokio runtime per ADR-011.
//!
//! ## Runtime topology
//!
//! - **Runtime flavor:** `#[tokio::main(flavor = "multi_thread")]`
//! - **Worker threads:** `worker_threads = std::thread::available_parallelism()`
//!   (i.e., `num_cpus` equivalent without an external crate; Rust 1.59+).
//! - **Shutdown channel:** root `tokio_util::sync::CancellationToken`;
//!   every long-lived coordination task receives a clone via
//!   `CancellationToken::child_token()`.
//! - **Graceful shutdown:** `tokio::select!` arms on (a) SIGINT (via
//!   `tokio::signal::ctrl_c`), (b) SIGTERM (Unix; `tokio::signal::unix`),
//!   (c) root-token cancellation. Any arm triggers root-token cancel,
//!   then the program awaits all spawned tasks to drain.
//!
//! At v0.1-α the seven adapter shells are constructed but their port-trait
//! implementations are deferred (Story 1b.x). The composition root demonstrates
//! the wiring shape; runtime behavior is structural-only.
//!
//! ## What this binary does NOT do at v0.1-α
//!
//! - Does NOT load any Spirit (Story 5.1 lifecycle verbs deferred).
//! - Does NOT open any control-plane port (Story 1a.4 ships maosctl).
//! - Does NOT initialize the Transparency Log (Story 1b.1 audit spine).
//! - Does NOT verify any signed binary (Story 1a.3 CryptoProvider deferred).
//!
//! Running `maos-bin` at v0.1-α prints a startup banner, blocks on the
//! shutdown selector, and exits cleanly on Ctrl+C. This validates the
//! runtime topology only.

use std::thread::available_parallelism;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use maos_kernel_core::api::{
    CapabilityRegistryAdapter, IacBusAdapter, IoSubsystemAdapter,
    MemoryManagerAdapter, SecurityManagerAdapter, SpiritSchedulerAdapter,
    TelemetryStreamAdapter,
};

fn worker_thread_count() -> usize {
    available_parallelism()
        .map(usize::from)
        .unwrap_or(1) // single-thread fallback if parallelism query fails
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Per ADR-011: single multi-threaded runtime. `worker_threads` is
    // configured at process start via `tokio::runtime::Builder` when an
    // explicit count is needed; at v0.1-α the `#[tokio::main]` attribute's
    // default is good enough and the `worker_thread_count()` helper
    // records the resolution path for the dev record.
    let cpus = worker_thread_count();
    eprintln!(
        "maos {} (v0.1-α scaffold; worker_threads target = {})",
        env!("CARGO_PKG_VERSION"),
        cpus
    );

    // Construct the seven adapter shells. At v0.1-α these are zero-size
    // placeholders; Story 1b.x replaces them with real adapter state.
    let _scheduler = SpiritSchedulerAdapter::default();
    let _security = SecurityManagerAdapter::default();
    let _memory = MemoryManagerAdapter::default();
    let _iac = IacBusAdapter::default();
    let _capability = CapabilityRegistryAdapter::default();
    let _io = IoSubsystemAdapter::default();
    let _telemetry = TelemetryStreamAdapter::default();

    // Root cancellation token. Every long-lived coordination task gets a
    // child token via `cancel.child_token()`. Cancelling the root cancels
    // all children (per tokio-util semantics).
    let cancel = CancellationToken::new();

    // Wire the graceful-shutdown selector. At v0.1-α we arm on SIGINT
    // (Ctrl+C), SIGTERM (Unix), and root-token cancellation. Any arm
    // triggers root-token cancel; the program then awaits drain.
    let shutdown_reason: &'static str = tokio::select! {
        _ = signal::ctrl_c() => "sigint",
        _ = shutdown_unix_term() => "sigterm",
        _ = cancel.cancelled() => "internal-cancel",
    };
    eprintln!("maos: shutdown reason = {shutdown_reason}; cancelling root token");
    cancel.cancel();

    // v0.1-α has no spawned tasks to drain yet (Story 1b.x adds them).
    // The drain loop is a structural placeholder so 1b.x slots into a
    // working scaffold rather than rewriting the shutdown semantics.
    eprintln!("maos: drained 0 child tasks; exiting cleanly");
    Ok(())
}

#[cfg(unix)]
async fn shutdown_unix_term() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate())
        .expect("install SIGTERM handler");
    term.recv().await;
}

#[cfg(not(unix))]
async fn shutdown_unix_term() {
    // Non-Unix targets (Windows): never resolves; Ctrl+C arm covers shutdown.
    std::future::pending::<()>().await;
}
