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
use maos_domain::ports::CapabilityRegistryPort;
use maos_domain::orchestrator::OrchestratorInstruction;
use maos_domain::orchestrator::OrchestratorInstructionId;
use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::api::{
    CapabilityRegistryAdapter, IacBusAdapter, IoSubsystemAdapter,
    MemoryManagerAdapter, RingCryptoProvider,
    SpiritSchedulerAdapter, TelemetryStreamAdapter,
};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::Mailbox;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::security::approval::ApprovalManager;
use maos_director_surface::notification::{
    NotificationDispatcher, TerminalChannel,
};
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

    // Story 4.3 — construct real MemoryManagerAdapter with Private + Shared + Principal stores.
    // Resolve paths first so both memory and TL share the same DB location.
    let audit_db_path = maos_audit::default_transparency_log_path();
    if let Some(parent) = audit_db_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "maos: failed to create audit DB parent directory {}: {e}",
                parent.display()
            );
            return Err(format!("audit-db parent create failed: {e}").into());
        }
    }

    let memory_root = maos_audit::default_memory_root();
    if let Err(e) = std::fs::create_dir_all(&memory_root) {
        eprintln!(
            "maos: failed to create memory root directory {}: {e}",
            memory_root.display()
        );
        return Err(format!("memory-root create failed: {e}").into());
    }

    let private_store = Arc::new(
        maos_kernel_core::memory::private::PrivateMemoryStore::new(memory_root, 4 * 1024),
    );
    let shared_store = Arc::new(
        maos_kernel_core::memory::shared::SharedMemoryStore::open(&audit_db_path)
            .map_err(|e| format!("failed to open shared memory store: {e}"))?,
    );
    let principal_index = Arc::new(
        maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&audit_db_path)
            .map_err(|e| format!("failed to open principal index: {e}"))?,
    );

    // Memory Manager adapter is assembled below after TL init (needs TL for forget-receipt audit).


    let telemetry = Arc::new(IacRtMetrics::new());

    // Story 3.1 — Mailbox replaces the v0.1-β stub.
    let mailbox = Arc::new(Mailbox::new(Arc::clone(&telemetry)));

    // Story 3.1 — NotificationDispatcher with TerminalChannel.
    let mut dispatcher = NotificationDispatcher::new();
    if std::env::var_os("MAOS_NOTIFY_DISABLE").is_none() {
        dispatcher.register(Box::new(TerminalChannel::new(Arc::new(std::sync::Mutex::new(
            std::io::stderr(),
        )))));
    }

    let io = IoSubsystemAdapter::new();
    let telemetry_stream = Arc::new(TelemetryStreamAdapter::default());

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
    let working_memory = Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        boot_nonce,
        Arc::clone(&policy),
        audit_tx.clone(),
        quota,
        working_memory,
        Arc::clone(&telemetry_stream),
    ));
    eprintln!("maos: capability registry initialized (Story 1b.2)");

    // Story 4.2 — HaltRegistry + WorkingMemoryOrchestrator for scalar-write pipeline.
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let orchestrator = Arc::new(maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
        Arc::clone(&capability),
        Arc::clone(&halt_registry),
    ));

    // Story 3.4 — Orchestrator buffer registry (shared Arc for one-shot arms).
    let orchestrator_registry = Arc::new(
        maos_kernel_core::orchestrator::OrchestratorBufferRegistry::new(),
    );
    eprintln!("maos: orchestrator buffer registry initialized (Story 3.4)");

    // Transparency Log — shared across services (Story 1b.1).
    // The audit_db_path is resolved above (line ~108) so memory stores and TL
    // share the same DB file location.
    let transparency_log = Arc::new(
        maos_kernel_core::iac::TransparencyLogAdapter::open(&audit_db_path, boot_nonce)
            .map_err(|e| format!("failed to open audit DB at {}: {e}", audit_db_path.display()))?,
    );
    eprintln!(
        "maos: Transparency Log opened on-disk at {}",
        audit_db_path.display()
    );

    // Story 4.3 — assemble the full MemoryManagerAdapter.
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private_store,
        shared_store,
        principal_index,
        Arc::clone(&transparency_log),
    ));

    // Story 4.3 — SelfTelemetryAggregator (FR56).
    let self_telemetry = Arc::new(maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator::new(
        Arc::clone(&telemetry),
        Arc::clone(&halt_registry),
        Arc::clone(&transparency_log),
    ));
    eprintln!("maos: Memory Manager initialized (three tiers + principal namespace, Story 4.3)");

    // Story 4.4 — LogRecallAdapter + DistillateWriter (log-recall + I11 audit chain).
    let log_recall_adapter = Arc::new(maos_kernel_core::iac::log_recall::LogRecallAdapter::new(
        Arc::clone(&transparency_log),
    ));
    let distillate_writer = Arc::new(maos_kernel_core::iac::distillate::DistillateWriter::new(
        Arc::clone(&transparency_log),
        Arc::clone(&memory),
    ));
    eprintln!("maos: LogRecallAdapter + DistillateWriter initialized (Story 4.4)");

    // Story 3.1 — wire IacBusAdapter with real Mailbox + Transparency Log.
    let iac = Arc::new(IacBusAdapter::new(
        Arc::clone(&mailbox),
        Arc::clone(&transparency_log),
    ));
    eprintln!("maos: IAC Bus wired (Mailbox + Transparency Log, Story 3.1)");

    // Story 4.5 — spirit_test-only hook wiring for cross-Spirit isolation
    // corpus observation. Production builds carry ZERO runtime cost.
    // The hooked adapters are constructed here for composition-root parity
    // but are NOT wired into the production supervisor tree; integration
    // tests construct their own hooked adapters as needed.
    #[cfg(feature = "spirit_test")]
    {
        let _isolation_hook = std::sync::Arc::new(parking_lot::Mutex::new(
            maos_spirit_sdk::spirit_test::DefaultIsolationHook::default(),
        ));
        let _halt_registry_with_hook = std::sync::Arc::new(
            maos_kernel_core::halt::HaltRegistry::new()
                .with_isolation_hook(_isolation_hook.clone()),
        );
        let _distillate_writer_with_hook =
            maos_kernel_core::iac::distillate::DistillateWriter::new(
                Arc::clone(&transparency_log),
                Arc::clone(&memory),
            )
            .with_isolation_hook(_isolation_hook.clone());
        eprintln!("maos: Story 4.5 isolation hooks wired (spirit_test only)");
    }

    // Story 3.1 — Approval Manager (v0.3-β auto-allow).
    let _approval = ApprovalManager::new(Arc::clone(&transparency_log));
    eprintln!("maos: Approval Manager initialized (v0.3-β auto-allow)");

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

        if mode == "posture-shift" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let spirit_id = std::env::var("MAOS_SPIRIT_ID")
                .map_err(|_| "MAOS_SPIRIT_ID is required for posture-shift")?;
            let posture_str = std::env::var("MAOS_POSTURE")
                .map_err(|_| "MAOS_POSTURE is required for posture-shift")?;

            let new_posture = match posture_str.as_str() {
                "cautious" => maos_kernel_core::security::manifest::Posture::Cautious,
                "assistive" => maos_kernel_core::security::manifest::Posture::Assistive,
                "autonomous-with-halt" => {
                    maos_kernel_core::security::manifest::Posture::AutonomousWithHalt
                }
                other => {
                    return Err(format!(
                        "unknown posture '{other}' — expected cautious|assistive|autonomous-with-halt"
                    )
                    .into());
                }
            };

            // Parse manifest and admit the Spirit to seed PolicyTable state
            let manifest_path_str = format!("spirits/{spirit_id}/manifest.toml");
            let manifest_path = std::path::Path::new(&manifest_path_str);
            let manifest_toml = std::fs::read_to_string(manifest_path).map_err(|e| {
                format!("failed to read spirit manifest at {}: {e}", manifest_path.display())
            })?;
            let manifest_root: toml::Value = toml::from_str(&manifest_toml)
                .map_err(|e| format!("manifest TOML parse error: {e}"))?;

            fn extract_section(
                root: &toml::Value,
                section: &str,
            ) -> Result<String, Box<dyn std::error::Error>> {
                let value = root
                    .get(section)
                    .ok_or_else(|| format!("missing manifest section [{section}]"))?;
                let serialized = toml::to_string(value)
                    .map_err(|e| format!("failed to serialize [{section}] section: {e}"))?;
                Ok(serialized)
            }

            let sandbox_cfg = maos_kernel_core::security::SandboxConfig::from_toml_str(
                &extract_section(&manifest_root, "sandbox")?,
            )?;
            let resource_caps = maos_kernel_core::security::ResourceCaps::from_toml_str(
                &extract_section(&manifest_root, "resources")?,
            )?;
            let caps_required = {
                let caps_required_val = manifest_root.get("capabilities").and_then(|c| c.get("required"));
                let caps_required_toml = match caps_required_val {
                    Some(v) => toml::to_string(v)
                        .map_err(|e| format!("failed to serialize [capabilities.required]: {e}"))?,
                    None => return Err("missing [capabilities.required]".into()),
                };
                maos_kernel_core::security::CapabilitiesRequired::from_toml_str(&caps_required_toml)?
            };
            let output_shape = maos_kernel_core::security::OutputShape::from_toml_str(
                &extract_section(&manifest_root, "output_shape")?,
            )?;
            let posture_section =
                maos_kernel_core::security::manifest::PostureSection::from_toml_str(
                    &extract_section(&manifest_root, "posture")?,
                )
                .map_err(|e| format!("posture parse: {e}"))?;
            let epistemic_policy = manifest_root
                .get("epistemic_policy")
                .map(|v| {
                    let s = toml::to_string(v)
                        .map_err(|e| format!("epistemic_policy serialize: {e}"))?;
                    maos_kernel_core::security::EpistemicPolicySection::from_toml_str(&s)
                        .map_err(|e| format!("epistemic_policy parse: {e}"))
                })
                .transpose()?;

            // Open journal for Load event
            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("journal parent create failed: {e}"))?;
            }
            let journal =
                maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                    .map_err(|e| format!("failed to open Lifecycle Journal: {e}"))?;

            let security =
                maos_kernel_core::security::SecurityManagerAdapter::new(Arc::clone(&policy));
            let _spec = security.admit_spirit(
                0,
                &spirit_id,
                &sandbox_cfg,
                &resource_caps,
                &caps_required,
                Some(&output_shape),
                &journal,
                &posture_section,
                epistemic_policy.as_ref(),
            )?;

            // Perform the posture shift
            let new_hash = policy
                .shift_posture(0, new_posture)
                .map_err(|e| format!("posture shift failed: {e}"))?;

            // Journal to Lifecycle Journal
            let journal_entry_path = maos_audit::default_journal_path();
            let journal_adapter =
                maos_kernel_core::journal::JournalAdapter::open(&journal_entry_path)
                    .map_err(|e| format!("failed to open Lifecycle Journal: {e}"))?;
            journal_adapter.append_transition(
                maos_domain::invariants::i10::JournalEntry {
                    timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                    lifecycle_event: maos_domain::invariants::i10::LifecycleEvent::PostureShift,
                    spirit_id: spirit_id.clone(),
                    effective_sandbox_tier: None,
                },
            );
            drop(journal_adapter);

            // Journal to Approval Decision Log
            maos_kernel_core::security::posture::journal_posture_shift(
                &transparency_log,
                "director",
                &spirit_id,
                posture_section.default,
                new_posture,
            )
            .map_err(|e| format!("approval log write failed: {e}"))?;

            drop(journal);

            // Drain: drop senders in order, await audit writer
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            let _ = new_hash;
            eprintln!("maos: posture shift {} → {:?} (journal: {})", spirit_id, new_posture, journal_entry_path.display());
            return Ok(());
        }

        // Story 3.3 AC7 — halt-list: read Transparency Log for recent halt frames.
        if mode == "halt-list" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let limit: usize = std::env::var("MAOS_HALT_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20);
            let spirit_filter = std::env::var("MAOS_HALT_SPIRIT").ok();

            let mut filter = FrameFilter {
                kind: Some(FrameKind::EpistemicHalt),
                limit: Some(limit),
                ..Default::default()
            };

            // If spirit filter specified, resolve to pid (v0.3-β: only hello-spirit → 0)
            if let Some(ref s) = spirit_filter {
                if s == "hello-spirit" {
                    filter.spirit_pid = Some(0);
                } else {
                    return Err(format!("unknown spirit '{s}' — only 'hello-spirit' is available at v0.3-β").into());
                }
            }

            let entries = transparency_log.query_frames(filter)
                .map_err(|e| format!("halt-list query failed: {e}"))?;

            for entry in &entries {
                let id_prefix: String = entry.frame_id.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join("");
                let id_display: String = id_prefix.chars().take(8).collect();
                let json_line = match serde_json::to_string(&serde_json::json!({
                    "frame_id": id_display,
                    "timestamp_ns": entry.timestamp_ns,
                    "kind": format!("{:?}", entry.kind),
                    "intent": entry.intent,
                })) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("maos: serialization error for frame: {e}");
                        continue;
                    }
                };
                println!("{json_line}");
            }

            eprintln!("maos: halt-list — {} halts shown", entries.len());
            return Ok(());
        }

        // Story 3.3 AC7 — halt-resolve: write resolution to Approval Decision Log.
        if mode == "halt-resolve" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let halt_id_str = std::env::var("MAOS_HALT_ID")
                .map_err(|_| "MAOS_HALT_ID is required for halt-resolve")?;
            let spirit_id = std::env::var("MAOS_HALT_SPIRIT")
                .map_err(|_| "MAOS_HALT_SPIRIT is required for halt-resolve")?;
            let kind_str = std::env::var("MAOS_HALT_KIND")
                .map_err(|_| "MAOS_HALT_KIND is required for halt-resolve")?;

            // Validate spirit (v0.3-β: only hello-spirit)
            if spirit_id != "hello-spirit" {
                return Err(format!("unknown spirit '{spirit_id}' — only 'hello-spirit' is available at v0.3-β").into());
            }

            let halt_id = maos_domain::halt::HaltId::new(&halt_id_str)
                .map_err(|e| format!("invalid halt_id: {e}"))?;

            let resolution = match kind_str.as_str() {
                "provided_context" => {
                    let text = std::env::var("MAOS_HALT_TEXT")
                        .map_err(|_| "MAOS_HALT_TEXT is required for provided_context")?;
                    maos_domain::halt::Resolution::provided_context(&text)
                        .map_err(|e| format!("invalid resolution: {e}"))?
                }
                "accepted_halt" => maos_domain::halt::Resolution::AcceptedHalt,
                "authorized_override" => {
                    let policy_ref = std::env::var("MAOS_HALT_OPERATOR_POLICY")
                        .map_err(|_| "MAOS_HALT_OPERATOR_POLICY is required for authorized_override")?;
                    maos_domain::halt::Resolution::authorized_override(&policy_ref)
                        .map_err(|e| format!("invalid resolution: {e}"))?
                }
                other => {
                    return Err(format!(
                        "unknown halt kind '{other}' — expected provided_context|accepted_halt|authorized_override"
                    ).into());
                }
            };

            // Story 4.1 — production KernelHaltResolver replaces the v0.3-β MockHaltResolver bootstrap.
            // Composition root owns the shared HaltRegistry + OutputMarkerRegistry so all
            // invoke_halt callers and the resolver agree on a single source of truth.
            let output_markers = Arc::new(maos_kernel_core::halt::OutputMarkerRegistry::new());
            let kernel_resolver = Arc::new(maos_kernel_core::halt::KernelHaltResolver::new(
                Arc::clone(&halt_registry),
                Arc::clone(&transparency_log),
                Arc::clone(&output_markers),
                Arc::clone(&mailbox),
                boot_nonce,
                Arc::clone(&memory),
                Arc::clone(&orchestrator),
            ));
            let halt_flow = maos_director_surface::halt_ui::HaltFlow::new(
                kernel_resolver,
                Arc::new(dispatcher),
                Arc::clone(&transparency_log) as Arc<dyn maos_domain::halt::HaltJournal>,
            );

            halt_flow.submit_resolution(halt_id.clone(), resolution.clone(), &spirit_id)
                .map_err(|e| format!("halt resolution failed: {e}"))?;

            // Drain: drop senders in order, await audit writer
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: halt resolved {} ({})", halt_id.as_str(), resolution.kind_label());
            return Ok(());
        }

        // Story 3.4 AC2 — orchestrator-queue: enqueue an instruction
        // onto the per-Spirit Orchestrator buffer.
        if mode == "orchestrator-queue" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let spirit_id = std::env::var("MAOS_ORCHESTRATOR_SPIRIT")
                .map_err(|_| "MAOS_ORCHESTRATOR_SPIRIT is required for orchestrator-queue")?;
            let instruction_text = std::env::var("MAOS_ORCHESTRATOR_INSTRUCTION")
                .map_err(|_| "MAOS_ORCHESTRATOR_INSTRUCTION is required for orchestrator-queue")?;

            // Validate spirit (v0.3-β: only hello-spirit)
            if spirit_id != "hello-spirit" {
                return Err(format!(
                    "unknown spirit '{spirit_id}' — only 'hello-spirit' is available at v0.3-β"
                ).into());
            }

            // Mint a per-process monotonic instruction ID
            static INSTRUCTION_COUNTER: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(1);
            let id = OrchestratorInstructionId(
                INSTRUCTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            );
            let instruction = OrchestratorInstruction::new(
                id,
                &instruction_text,
                maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
            )
            .map_err(|e| format!("invalid orchestrator instruction: {e}"))?;

            // Get-or-create the per-Spirit buffer
            let buffer = orchestrator_registry.get_or_create(&maos_spirit_abi::identity::SpiritId::from(spirit_id.as_str()));
            buffer.enqueue(instruction.clone())
                .map_err(|e| format!("orchestrator queue error: {e}"))?;

            // Journal to Approval Decision Log
            maos_kernel_core::orchestrator::journal_orchestrator_queue(
                &transparency_log,
                "director",
                &spirit_id,
                &instruction,
            )
            .map_err(|e| format!("approval log write failed: {e}"))?;

            // Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!(
                "maos: queued orchestrator instruction id={} for {}",
                instruction.id.0,
                spirit_id,
            );
            return Ok(());
        }

        // Story 3.4 AC2 — orchestrator-status: show pending count for a Spirit.
        if mode == "orchestrator-status" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let spirit_id = std::env::var("MAOS_ORCHESTRATOR_SPIRIT")
                .map_err(|_| "MAOS_ORCHESTRATOR_SPIRIT is required for orchestrator-status")?;

            // Validate spirit (v0.3-β: only hello-spirit)
            if spirit_id != "hello-spirit" {
                return Err(format!(
                    "unknown spirit '{spirit_id}' — only 'hello-spirit' is available at v0.3-β"
                ).into());
            }

            let sid = maos_spirit_abi::identity::SpiritId::from(spirit_id.as_str());
            let (count, capacity) = match orchestrator_registry.get(&sid) {
                Some(buf) => (buf.pending_count(), buf.capacity()),
                None => (0, 32),
            };

            eprintln!(
                "maos: orchestrator status {spirit_id}: {count}/{capacity} pending instructions"
            );

            // No drain needed — read-only path, no audit rows written.
            return Ok(());
        }

        // Story 3.4 AC3 — pause: write Lifecycle Journal entry + Approval Decision row.
        if mode == "pause" || mode == "resume" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let spirit_id = std::env::var("MAOS_SPIRIT_ID")
                .map_err(|_| format!("MAOS_SPIRIT_ID is required for {mode}"))?;
            if spirit_id != "hello-spirit" {
                return Err(format!(
                    "unknown spirit '{spirit_id}' — only 'hello-spirit' is available at v0.3-β"
                ).into());
            }

            // 1. Write Lifecycle Journal entry (Pause or Resume)
            let event = if mode == "pause" {
                maos_domain::invariants::i10::LifecycleEvent::Pause
            } else {
                maos_domain::invariants::i10::LifecycleEvent::Resume
            };
            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("journal parent create failed: {e}"))?;
            }
            let journal = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| format!("failed to open Lifecycle Journal: {e}"))?;
            journal.append_transition(maos_domain::invariants::i10::JournalEntry {
                timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                lifecycle_event: event,
                spirit_id: spirit_id.clone(),
                effective_sandbox_tier: None,
            });
            drop(journal);

            // 2. Journal to Approval Decision Log (director-action audit, FR42)
            maos_kernel_core::orchestrator::journal_director_lifecycle_action(
                &transparency_log,
                "director",
                &spirit_id,
                mode.as_str(),
            )
            .map_err(|e| format!("approval log write failed: {e}"))?;

            // 3. FR51 c — on resume, recall buffered Orchestrator instructions
            if mode == "resume" {
                let sid = maos_spirit_abi::identity::SpiritId::from(spirit_id.as_str());
                if let Some(buffer) = orchestrator_registry.get(&sid) {
                    let pending = buffer.recall_all_pending();
                    eprintln!(
                        "maos: resume {spirit_id} — recalled {} pending Orchestrator instructions",
                        pending.len()
                    );
                    for instr in pending {
                        if let Err(e) = buffer.enqueue(instr) {
                            eprintln!("maos: resume re-enqueue failed: {e}");
                        }
                    }
                }
            }

            // 4. Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!(
                "maos: {mode} {spirit_id} (journal: {})",
                journal_path.display()
            );
            return Ok(());
        }

        // Story 3.4 AC4 — revoke-token: revoke a capability token via the
        // existing CapabilityRegistryAdapter::revoke path, then journal.
        if mode == "revoke-token" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let token_id_hex = std::env::var("MAOS_REVOKE_TOKEN_ID")
                .map_err(|_| "MAOS_REVOKE_TOKEN_ID is required for revoke-token")?;
            let reason_text = std::env::var("MAOS_REVOKE_REASON").ok();

            // Parse 32-char hex into [u8; 16]
            let token_bytes = parse_token_id_hex(&token_id_hex)
                .map_err(|e| format!("invalid token_id '{token_id_hex}': {e}"))?;
            let token_id = maos_domain::invariants::i1::TokenId(token_bytes);

            // Revoke through the canonical CapabilityRegistryAdapter::revoke path.
            match capability.revoke(token_id) {
                Ok(()) => {}
                Err(maos_domain::ports::capability::CapError::UnknownToken) => {
                    return Err(format!(
                        "token {token_id_hex} not found (already revoked or never issued)"
                    ).into());
                }
                Err(e) => {
                    return Err(format!("revoke failed: {e}").into());
                }
            }

            // Journal to Approval Decision Log per FR42 (director identity + reason).
            maos_kernel_core::orchestrator::journal_token_revocation(
                &transparency_log,
                "director",
                &token_id_hex,
                reason_text.as_deref(),
            )
            .map_err(|e| format!("approval log write failed: {e}"))?;

            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: revoked token {token_id_hex}");
            return Ok(());
        }

        if mode != "hello-spirit" {
            eprintln!("maos: unknown MAOS_ONE_SHOT mode '{mode}' — only 'hello-spirit' is supported");
            return Err(format!("unknown MAOS_ONE_SHOT mode: {mode}").into());
        }

        // Story 2.1 AC4 — parse manifest and admit via SecurityManagerAdapter
        // instead of hardcoding capability scopes. The adapter reads the
        // manifest's [capabilities.required] section and registers scopes
        // in the policy table.
        {
            // Initialize monotonic counter for journal timestamps and token issuance.
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let manifest_path = std::path::Path::new("spirits/hello-spirit/manifest.toml");
            let manifest_toml = std::fs::read_to_string(manifest_path)
                .map_err(|e| format!("failed to read spirit manifest at {}: {e}", manifest_path.display()))?;

            // Parse full TOML document, then extract individual sections.
            let manifest_root: toml::Value = toml::from_str(&manifest_toml)
                .map_err(|e| format!("manifest TOML parse error: {e}"))?;

            fn extract_section(root: &toml::Value, section: &str) -> Result<String, Box<dyn std::error::Error>> {
                let value = root
                    .get(section)
                    .ok_or_else(|| format!("missing manifest section [{section}]"))?;
                let serialized = toml::to_string(value)
                    .map_err(|e| format!("failed to serialize [{section}] section: {e}"))?;
                Ok(serialized)
            }

            // Parse each manifest section individually.
            let sandbox_cfg = maos_kernel_core::security::SandboxConfig::from_toml_str(
                &extract_section(&manifest_root, "sandbox")?,
            )?;
            let resource_caps = maos_kernel_core::security::ResourceCaps::from_toml_str(
                &extract_section(&manifest_root, "resources")?,
            )?;
            let caps_required = {
                let caps_required_val = manifest_root
                    .get("capabilities")
                    .and_then(|c| c.get("required"));
                let caps_required_toml = match caps_required_val {
                    Some(v) => toml::to_string(v)
                        .map_err(|e| format!("failed to serialize [capabilities.required]: {e}"))?,
                    None => return Err(format!("missing manifest section [capabilities.required]").into()),
                };
                maos_kernel_core::security::CapabilitiesRequired::from_toml_str(&caps_required_toml)?
            };
            let output_shape = maos_kernel_core::security::OutputShape::from_toml_str(
                &extract_section(&manifest_root, "output_shape")?,
            )?;

            // Open the Lifecycle Journal for admission (the Load event).
            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("journal parent create failed: {e}"))?;
            }
            let journal = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| format!("failed to open Lifecycle Journal: {e}"))?;

            // Construct SecurityManagerAdapter with drift channel (Story 2.1 AC4).
            // Drift channel: receiver declared first so it outlives the
            // sender (held by security). Rust drops in reverse declaration
            // order — security drops first, then _drift_rx.
            let (drift_tx, _drift_rx) = maos_kernel_core::security::make_drift_channel();
            let security = maos_kernel_core::security::SecurityManagerAdapter::new(Arc::clone(&policy))
                .with_drift_sender(drift_tx);

            // Admit hello-spirit through the canonical admission path.
            let posture_section =
                maos_kernel_core::security::manifest::PostureSection::from_toml_str(
                    &extract_section(&manifest_root, "posture")?,
                )
                .map_err(|e| format!("posture parse: {e}"))?;
            let epistemic_policy = manifest_root
                .get("epistemic_policy")
                .map(|v| {
                    let s = toml::to_string(v)
                        .map_err(|e| format!("epistemic_policy serialize: {e}"))?;
                    maos_kernel_core::security::EpistemicPolicySection::from_toml_str(&s)
                        .map_err(|e| format!("epistemic_policy parse: {e}"))
                })
                .transpose()?;

            let _spec = security.admit_spirit(
                0,
                "hello-spirit",
                &sandbox_cfg,
                &resource_caps,
                &caps_required,
                Some(&output_shape),
                &journal,
                &posture_section,
                epistemic_policy.as_ref(),
            )?;

            // Drop the journal adapter (fsync + drain).
            drop(journal);
        }

        // Initialize monotonic counter for token issuance

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

        // Print token_id for downstream test observability (Story 3.4 AC4).
        let token_id_hex: String = token.token_id.0.iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        eprintln!("maos: issued token {token_id_hex}");
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

    // Story 2.5 (A7 / D11) — drain the cap-audit channel deterministically
    // on graceful shutdown, replicating the one-shot arm's drain pattern
    // (lines 357–372). `audit_tx` is cloned into `CapabilityRegistryAdapter`
    // at construction (line ~118) so dropping the local `audit_tx` is not
    // enough — the adapter's clone must also be released. Drop all owners
    // in the same sequence as the one-shot path so the writer task sees
    // channel-close and every in-flight row reaches SQLite.
    drop(audit_tx);
    drop(inference);
    drop(capability);
    // Story 3.1 — drop the IAC adapter (drops Arc<Mailbox>), then the
    // dispatcher (no async tasks at v0.3-β, but slot future-proofs).
    drop(iac);
    drop(mailbox);
    drop(dispatcher);

    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        audit_writer,
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("maos: audit writer task returned error during drain: {e}"),
        Err(_) => eprintln!("maos: audit writer drain timed out after 10s"),
    }

    let cap_audit_rows = match transparency_log.query_frames(FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    }) {
        Ok(rows) => rows.len(),
        Err(e) => {
            eprintln!("maos: failed to query cap-audit rows after drain: {e}");
            0
        }
    };
    eprintln!("maos: drained {cap_audit_rows} cap-audit row(s); exiting cleanly");
    Ok(())
}

/// Parse a 32-char lowercase hex string into a 16-byte TokenId.
/// Rejects invalid lengths and non-hex characters.
fn parse_token_id_hex(s: &str) -> Result<[u8; 16], String> {
    if s.len() != 32 {
        return Err(format!("expected 32 hex chars, got {}", s.len()));
    }
    if !s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)) {
        return Err("non-hex characters in token_id".into());
    }
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        let byte_str = &s[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|e| format!("hex decode error at byte {i}: {e}"))?;
    }
    Ok(bytes)
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
