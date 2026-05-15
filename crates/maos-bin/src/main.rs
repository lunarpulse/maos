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
//!
//! Story 1b.5a adds the one-shot mode: set `MAOS_ONE_SHOT=hello-spirit`
//! to run the reference Spirit once and print JSON to stdout.

use std::sync::Arc;
use std::thread::available_parallelism;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i1::Scope;
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

/// Resolve the on-disk Transparency Log SQLite path.
///
/// Delegates to [`maos_audit::default_transparency_log_path`] — the single
/// source of truth shared by `maos-bin` (write side) and `maos-cli` (read
/// side) to prevent path-drift data loss.
fn default_transparency_log_path() -> std::path::PathBuf {
    maos_audit::default_transparency_log_path()
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
    //
    // Story 1b.5b: switched from `open_in_memory` to on-disk SQLite at the
    // XDG-resolved path so `maosctl audit query` can read back the rows the
    // one-shot path just wrote. The path resolves identically to the
    // CLI-side `default_transparency_log_path()` (see
    // crates/maos-cli/src/subcommands.rs:148-162). Server mode (non-one-shot)
    // benefits from the same change — a single line covers both paths; no
    // branch on `MAOS_ONE_SHOT` is needed for storage selection.
    let audit_db_path = default_transparency_log_path();
    if let Some(parent) = audit_db_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "maos: failed to create audit DB parent directory {}: {e}",
                parent.display()
            );
            return Err(format!("audit-db parent create failed: {e}").into());
        }
    }
    let transparency_log = Arc::new(
        maos_kernel_core::iac::TransparencyLogAdapter::open(&audit_db_path, boot_nonce)
            .map_err(|e| format!("failed to open audit DB at {}: {e}", audit_db_path.display()))?,
    );
    eprintln!(
        "maos: Transparency Log opened on-disk at {}",
        audit_db_path.display()
    );

    // Spawn the audit writer task (Story 1b.2). Held by name so the one-shot
    // exit path (Story 1b.5b) can drain the cap-audit channel deterministically
    // before process exit — `drop(audit_tx); drop(...senders); audit_writer.await.ok();`.
    let audit_writer = maos_kernel_core::capability::cap_audit::CapAuditWriter::spawn(
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
    let inference = InferencePortAdapter::new(
        anthropic_provider,
        "anthropic".into(),
        Arc::clone(&capability),
        Arc::clone(&transparency_log),
        Arc::clone(&telemetry),
    );
    eprintln!("maos: Inference Port initialized (Story 1b.4)");
    // ─────────────────────────────────────────────────────────────

    // ─────────────────────────────────────────────────────────────
    // Story 1b.5c — Lifecycle one-shot verbs (start/stop/unload).
    //
    // Per Decision Register D2 the discriminator is fused into the
    // existing `MAOS_ONE_SHOT` env-var rather than introducing a parallel
    // `MAOS_LIFECYCLE_VERB`. This keeps the composition-root branch count
    // at one. The lifecycle verbs at v0.1-β do NOT spawn supervised
    // children, do NOT touch the Inference Port, and do NOT need the
    // cap-audit drain (the 1b.5b drain stays only in the `hello-spirit`
    // arm). They write exactly one Lifecycle Journal entry and exit.
    if let Ok(mode) = std::env::var("MAOS_ONE_SHOT") {
        // Lifecycle verbs are handled first — they're cheap and exit
        // without engaging the Inference Port / capability registry.
        if let Some(event) = match mode.as_str() {
            "start" => Some(maos_domain::invariants::i10::LifecycleEvent::Start),
            "stop" => Some(maos_domain::invariants::i10::LifecycleEvent::Halt),
            "unload" => Some(maos_domain::invariants::i10::LifecycleEvent::Unload),
            _ => None,
        } {
            // Initialize the monotonic clock so journal timestamps are
            // comparable to the kernel-side `Load` transitions emitted
            // by `SecurityManagerAdapter::admit_spirit`.
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!(
                        "maos: failed to create journal parent directory {}: {e}",
                        parent.display()
                    );
                    return Err(format!("journal parent create failed: {e}").into());
                }
            }
            let adapter = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| {
                    format!("failed to open Lifecycle Journal at {}: {e}", journal_path.display())
                })?;
            let spirit_id = std::env::var("MAOS_SPIRIT_ID")
                .unwrap_or_else(|_| "hello-spirit".into());
            adapter.append_transition(maos_domain::invariants::i10::JournalEntry {
                timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                lifecycle_event: event,
                spirit_id: spirit_id.clone(),
                effective_sandbox_tier: None,
            });
            // Adapter's `Drop` impl signals the drain thread and fsyncs
            // (journal/mod.rs:195-203). No cap-audit drain required.
            drop(adapter);

            // Diagnostic copy mirrors the AC1 table verbatim.
            let diag = match mode.as_str() {
                "start" => "started",
                "stop" => "stopped",
                "unload" => "unloaded",
                _ => unreachable!(),
            };
            eprintln!(
                "maos: {diag} {spirit_id} (journal: {})",
                journal_path.display()
            );
            return Ok(());
        }

        if mode != "hello-spirit" {
            eprintln!("maos: unknown MAOS_ONE_SHOT mode '{mode}' — only 'hello-spirit' is supported");
            return Err(format!("unknown MAOS_ONE_SHOT mode: {mode}").into());
        }

        // Pre-populate the policy table with hello-Spirit's declared capabilities
        {
            let mut inner = maos_kernel_core::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                0,
                maos_kernel_core::capability::cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::ProviderInfer {
                        provider: "anthropic".into(),
                    }],
                    declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                    trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }

        // Initialize monotonic counter for token issuance
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();

        // Issue a valid capability token for the in-process hello-Spirit
        let token = capability.issue_with_mediation(
            0,
            Scope::ProviderInfer {
                provider: "anthropic".into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        ).map_err(|e| format!("failed to issue capability token: {e}"))?;

        eprintln!("maos: one-shot mode — executing hello-Spirit");

        // Call hello-Spirit (sync call on async runtime — fine for one-shot)
        let resp = maos_spirit_hello::run(&inference, token)
            .map_err(|e| format!("hello-Spirit error: {e}"))?;

        let json = serde_json::to_string(&resp)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{json}");

        // Story 1b.5b — drain the cap-audit channel deterministically before
        // exit. `audit_tx` is cloned into `CapabilityRegistryAdapter::new`
        // (line ~110) so dropping the local `audit_tx` is not enough — the
        // adapter holds the surviving sender. Drop all owners in sequence so
        // the writer task sees channel-close and the inference.call row is
        // guaranteed to reach SQLite. Without this, the row is intermittently
        // lost (the writer is `tokio::spawn`-ed and the runtime drops mid-flush
        // on process exit).
        drop(audit_tx);
        drop(inference);
        drop(capability);
        // `transparency_log` is moved into the writer task's closure (Arc), so
        // awaiting the writer drains the queue and releases its Arc clone.
        if let Err(e) = audit_writer.await {
            eprintln!("maos: audit writer task failed during drain: {e}");
        }

        eprintln!("maos: one-shot complete — exiting cleanly");
        return Ok(());
    }
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
