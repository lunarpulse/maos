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
//! - **Crypto provider:** `Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider)`
//!   per FR48 / NFR-Sec-15. Default `ring`/`rustls` adapter at v0.1-α;
//!   FIPS / HSM / post-quantum providers swap by changing this one line.
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
//! - Does NOT verify any actual signed binary at runtime (Story 1b.1 wires
//!   `CryptoProvider::verify_signature` into the journal-replay path; this
//!   story only DECLARES the seam).
//!
//! Running `maos-bin` at v0.1-α prints a startup banner, blocks on the
//! shutdown selector, and exits cleanly on Ctrl+C. This validates the
//! runtime topology only.

use std::sync::Arc;
use std::thread::available_parallelism;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::api::{
    CapabilityRegistryAdapter, IacBusAdapter, IoSubsystemAdapter,
    MemoryManagerAdapter, RingCryptoProvider, SecurityManagerAdapter,
    SpiritSchedulerAdapter, TelemetryStreamAdapter,
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
    let _io = IoSubsystemAdapter::default();
    let _telemetry = TelemetryStreamAdapter::default();

    // ─────────────────────────────────────────────────────────────
    // Story 1a.3 — FR48 / NFR-Sec-15 crypto-provider seam.
    // Story 1b.2 — Capability Registry composite construction.
    //
    // Construct the default `ring`/`rustls`-backed CryptoProvider.
    // This is the FR48 architectural-commitment SWAP POINT.
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    eprintln!("maos: crypto provider = ring-default (FR48 swap point: maos-bin/src/main.rs)");

    // Construct the Capability Registry with all four sub-modules.
    // FIXME(1b.3): signing key MUST come from OS keyring / maos-secrets.
    // v0.1-β scaffold: deterministic key for testing ONLY. A random key
    // is used to prevent trivial forgery from published source.
    let signing_key_bytes: [u8; 32] = {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("failed to generate signing key");
        seed
    };
    let signing_key = maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new(signing_key_bytes);
    let policy = maos_kernel_core::capability::cap_policy::PolicyTable::new();
    let (audit_tx, audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let boot_nonce: u64 = {
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).expect("failed to generate boot nonce");
        u64::from_ne_bytes(buf)
    };
    let _capability = CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        boot_nonce,
        policy,
        audit_tx,
        quota,
    );
    eprintln!("maos: capability registry initialized (Story 1b.2)");

    // Spawn the audit writer task (Story 1b.2).
    // At v0.1-β the transparency log is in-memory for the scaffold;
    // Story 1b.1 lands the persistent SQLite adapter.
    let _audit_writer = maos_kernel_core::capability::cap_audit::CapAuditWriter::spawn(
        audit_rx,
        Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(boot_nonce)),
    );
    // ─────────────────────────────────────────────────────────────

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
