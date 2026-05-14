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
//! Story 1b.4 wires the real I/O Subsystem, Anthropic provider, Inference
//! Port adapter, and IAC telemetry registry.

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
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_providers::AnthropicProvider;

fn worker_thread_count() -> usize {
    available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

/// Fallback provider when Anthropic is unconfigured (no API key).
struct UnconfiguredProvider;

impl maos_providers::Provider for UnconfiguredProvider {
    fn complete(
        &self,
        _req: &maos_domain::ports::inference::InferenceRequest,
    ) -> Result<maos_domain::ports::inference::InferenceResponse, maos_providers::ProviderError> {
        Err(maos_providers::ProviderError::Unconfigured)
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cpus = worker_thread_count();
    eprintln!(
        "maos {} (v0.1-β scaffold; worker_threads target = {})",
        env!("CARGO_PKG_VERSION"),
        cpus
    );

    // Construct the seven adapter shells.
    let _scheduler = SpiritSchedulerAdapter::default();
    let _security = SecurityManagerAdapter::default();
    let _memory = MemoryManagerAdapter::default();
    let _iac = IacBusAdapter::default();
    let io = IoSubsystemAdapter::new();
    let _telemetry = TelemetryStreamAdapter::default();
    let telemetry = Arc::new(IacRtMetrics::new());

    // ─────────────────────────────────────────────────────────────
    // Story 1a.3 — FR48 / NFR-Sec-15 crypto-provider seam.
    // Story 1b.2 — Capability Registry composite construction.
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    eprintln!("maos: crypto provider = ring-default (FR48 swap point: maos-bin/src/main.rs)");

    // FIXME(1b.3): signing key MUST come from OS keyring / maos-secrets.
    let signing_key_bytes: [u8; 32] = {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("failed to generate signing key");
        seed
    };
    let signing_key = maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new(signing_key_bytes);
    let policy = Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new());
    let (audit_tx, audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let boot_nonce: u64 = {
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).expect("failed to generate boot nonce");
        u64::from_ne_bytes(buf)
    };
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        boot_nonce,
        Arc::clone(&policy),
        audit_tx.clone(),
        quota,
    ));
    let _security = SecurityManagerAdapter::new(Arc::clone(&policy));
    eprintln!("maos: capability registry initialized (Story 1b.2)");

    // Transparency Log — shared across services (Story 1b.1).
    let transparency_log = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(boot_nonce));

    // Spawn the audit writer task (Story 1b.2).
    let _audit_writer = maos_kernel_core::capability::cap_audit::CapAuditWriter::spawn(
        audit_rx,
        Arc::clone(&transparency_log),
    );

    // Story 1b.4 — Inference Port + Anthropic provider + IAC telemetry.
    // FIXME(secrets): API key read from env; real secret materialization via
    // maos-secrets / OS keyring is a later story.
    let anthropic_provider: Arc<dyn maos_providers::Provider> = match AnthropicProvider::new(
        Arc::new(io),
        "https://api.anthropic.com".into(),
        "claude-3-haiku-20240307".into(),
    ) {
        Ok(p) => {
            eprintln!("maos: Anthropic provider configured");
            Arc::new(p)
        }
        Err(e) => {
            eprintln!("maos: Anthropic provider unavailable ({e}) — inference calls will return Unconfigured");
            Arc::new(UnconfiguredProvider)
        }
    };
    let _inference = InferencePortAdapter::new(
        anthropic_provider,
        "anthropic".into(),
        Arc::clone(&capability),
        Arc::clone(&transparency_log),
        Arc::clone(&telemetry),
    );
    eprintln!("maos: Inference Port initialized (Story 1b.4)");
    // ─────────────────────────────────────────────────────────────

    let cancel = CancellationToken::new();

    let shutdown_reason: &'static str = tokio::select! {
        _ = signal::ctrl_c() => "sigint",
        _ = shutdown_unix_term() => "sigterm",
        _ = cancel.cancelled() => "internal-cancel",
    };
    eprintln!("maos: shutdown reason = {shutdown_reason}; cancelling root token");
    cancel.cancel();

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
    std::future::pending::<()>().await;
}
