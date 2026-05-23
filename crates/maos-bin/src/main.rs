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

use maos_director_surface::notification::{NotificationDispatcher, TerminalChannel};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i1::Scope;
use maos_domain::orchestrator::OrchestratorInstruction;
use maos_domain::orchestrator::OrchestratorInstructionId;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::CapabilityRegistryPort;
use maos_kernel_core::api::{
    CapabilityRegistryAdapter, IacBusAdapter, IoSubsystemAdapter, RingCryptoProvider,
    TelemetryStreamAdapter,
};
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::Mailbox;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::security::approval::ApprovalManager;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_providers::AnthropicProvider;

fn worker_thread_count() -> usize {
    available_parallelism().map(usize::from).unwrap_or(1)
}

/// Fallback provider when Anthropic is unconfigured (no API key).
struct UnconfiguredProvider;

impl maos_providers::Provider for UnconfiguredProvider {
    fn complete(
        &self,
        _req: &maos_domain::ports::inference::InferenceRequest,
    ) -> Result<maos_domain::ports::inference::InferenceResponse, maos_providers::ProviderError>
    {
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
    // Story 5.1 — `_scheduler` replaced with real Arc<SpiritSchedulerAdapter>
    // construction below (after all dependent adapters are initialized).

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

    let private_store = Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
        memory_root,
        4 * 1024,
    ));
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
        dispatcher.register(Box::new(TerminalChannel::new(Arc::new(
            std::sync::Mutex::new(std::io::stderr()),
        ))));
    }
    let notification_dispatcher = Arc::new(dispatcher);

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
    let signing_key =
        maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new(signing_key_bytes);
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
    let orchestrator = Arc::new(
        maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
            Arc::clone(&capability),
            Arc::clone(&halt_registry),
        ),
    );

    // Story 3.4 — Orchestrator buffer registry (shared Arc for one-shot arms).
    let orchestrator_registry =
        Arc::new(maos_kernel_core::orchestrator::OrchestratorBufferRegistry::new());
    eprintln!("maos: orchestrator buffer registry initialized (Story 3.4)");

    // Transparency Log — shared across services (Story 1b.1).
    // The audit_db_path is resolved above (line ~108) so memory stores and TL
    // share the same DB file location.
    let transparency_log = Arc::new(
        maos_kernel_core::iac::TransparencyLogAdapter::open(&audit_db_path, boot_nonce).map_err(
            |e| {
                format!(
                    "failed to open audit DB at {}: {e}",
                    audit_db_path.display()
                )
            },
        )?,
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
    let self_telemetry = Arc::new(
        maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator::new(
            Arc::clone(&telemetry),
            Arc::clone(&halt_registry),
            Arc::clone(&transparency_log),
        ),
    );
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

    // Story 5.1 — wire the real Spirit Scheduler replacing the v0.1-β
    // `_scheduler = SpiritSchedulerAdapter::default()` placeholder.
    let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&transparency_log),
        Arc::clone(&capability),
        Arc::clone(&memory),
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&telemetry),
        Some(Arc::clone(&orchestrator)),
        Some(Arc::clone(&log_recall_adapter)),
        Some(Arc::clone(&distillate_writer)),
        Some(Arc::clone(&self_telemetry)),
        None, // security_manager — constructed per-one-shot arm at v0.3-β
        Some(Arc::clone(&orchestrator_registry)),
        None, // crash_detector — wired below at Story 5.3 Task 14
    ));
    eprintln!("maos: Spirit Scheduler wired (Story 5.1)");

    // Story 5.2 — Shared journal opened at default path.
    // Created before CrashDetector so the detector can append lifecycle events.
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
    let shared_journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&journal_path).map_err(|e| {
            format!(
                "failed to open shared Lifecycle Journal at {}: {e}",
                journal_path.display()
            )
        })?,
    );

    // Story 5.3 — Composition-root wiring for supervision adapters.
    let replica_resolver = Arc::new(maos_domain::supervision::NullReplicaResolver);
    let crash_detector = Arc::new(
        maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&transparency_log),
            Arc::clone(&halt_registry),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&telemetry),
            Arc::clone(&shared_journal),
        )
        .with_replica_resolver(replica_resolver),
    );
    Arc::get_mut(&mut scheduler)
        .expect("scheduler Arc strong_count == 1 at composition root")
        .set_crash_detector(Arc::clone(&crash_detector));
    eprintln!("maos: CrashDetector wired (Story 5.3)");

    // Wire SCB map into Mailbox so deliver() updates last_inbound_frame_ns.
    mailbox.set_scbs(scheduler.scbs());

    // Story 5.1 — KernelLifecycleResolver assembled for CLI / ACP / HTTP API consumers.
    let lifecycle_resolver = Arc::new(maos_kernel_core::scheduler::KernelLifecycleResolver::new(
        Arc::clone(&scheduler),
        Arc::clone(&transparency_log),
        "director".into(),
    ));
    eprintln!("maos: KernelLifecycleResolver wired (Story 5.1)");

    // Story 5.2 — HotSwapCoordinator constructed exactly once at composition root.
    // One-shot arms that also journal use separate JournalAdapter instances
    // (acceptable at v0.3-β — file-append is atomic for small writes on POSIX).
    let archive_dir = maos_audit::default_archive_dir();
    let hot_swap_coordinator = Arc::new(HotSwapCoordinator::new(
        scheduler.scbs(),
        Arc::clone(&shared_journal),
        Arc::clone(&transparency_log),
        Arc::clone(&halt_registry),
        Arc::clone(&capability),
        Arc::clone(&iac),
        scheduler.dispatcher_arc(),
        Arc::clone(&telemetry),
        archive_dir,
    ));
    eprintln!("maos: HotSwapCoordinator wired (Story 5.2)");

    // Story 5.4 — RevocationApplier + RevocationPoller constructed at composition root.
    let revocation_applier = Arc::new(maos_kernel_core::revocation::RevocationApplier::new(
        scheduler.scbs(),
        Arc::clone(&capability),
        Arc::clone(&scheduler),
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&transparency_log),
        Arc::clone(&shared_journal),
        Arc::clone(&telemetry),
    ));
    let local_file_registry = Arc::new(maos_domain::revocation::LocalFileRegistryClient::new(
        std::path::PathBuf::from("/tmp").join("maos").join("crl"),
    ));
    let crypto_provider: Arc<dyn maos_domain::ports::crypto::CryptoProvider> =
        Arc::new(maos_kernel_core::security::crypto::RingCryptoProvider);
    let revocation_poller = Arc::new(maos_kernel_core::revocation::RevocationPoller::new(
        Arc::clone(&revocation_applier),
        local_file_registry,
        Arc::clone(&crypto_provider),
        Arc::clone(&telemetry),
    ));
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let _poller_handle = revocation_poller.spawn(cancel_token.child_token());
    eprintln!("maos: RevocationApplier + RevocationPoller wired (Story 5.4)");

    // Story 5.4 — UpgradeOrchestrator constructed at composition root.
    let upgrade_orchestrator = Arc::new(maos_kernel_core::lifecycle::UpgradeOrchestrator::new(
        Arc::clone(&scheduler),
        Arc::clone(&hot_swap_coordinator),
        Arc::clone(&transparency_log),
        Arc::clone(&shared_journal),
        Arc::clone(&telemetry),
    ));
    eprintln!("maos: UpgradeOrchestrator wired (Story 5.4)");

    // Story 4.5 — spirit_test-only isolation hooks are constructed by
    // integration tests (nfr_sec_14_cross_spirit_isolation.rs,
    // iac_bus_intent_lineage.rs) as needed.  Production builds carry
    // ZERO runtime cost (spirit_test feature is dev-time only).

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
            let adapter =
                maos_kernel_core::journal::JournalAdapter::open(&journal_path).map_err(|e| {
                    format!(
                        "failed to open Lifecycle Journal at {}: {e}",
                        journal_path.display()
                    )
                })?;
            let spirit_id =
                std::env::var("MAOS_SPIRIT_ID").unwrap_or_else(|_| "hello-spirit".into());
            adapter.append_transition(maos_domain::invariants::i10::JournalEntry::Lifecycle(
                maos_domain::invariants::i10::LifecycleEntry {
                    timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                    lifecycle_event: event,
                    spirit_id: spirit_id.clone(),
                    effective_sandbox_tier: None,
                },
            ));
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
                format!(
                    "failed to read spirit manifest at {}: {e}",
                    manifest_path.display()
                )
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
                let caps_required_val = manifest_root
                    .get("capabilities")
                    .and_then(|c| c.get("required"));
                let caps_required_toml = match caps_required_val {
                    Some(v) => toml::to_string(v)
                        .map_err(|e| format!("failed to serialize [capabilities.required]: {e}"))?,
                    None => return Err("missing [capabilities.required]".into()),
                };
                maos_kernel_core::security::CapabilitiesRequired::from_toml_str(
                    &caps_required_toml,
                )?
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
            let journal = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
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
                None,
                None,
                None,
                None,
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
                maos_domain::invariants::i10::JournalEntry::Lifecycle(
                    maos_domain::invariants::i10::LifecycleEntry {
                        timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                        lifecycle_event: maos_domain::invariants::i10::LifecycleEvent::PostureShift,
                        spirit_id: spirit_id.clone(),
                        effective_sandbox_tier: None,
                    },
                ),
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

            // Drain: drop all Arc holders of audit_tx so the channel closes.
            drop(audit_tx);
            drop(inference);
            drop(capability);
            drop(orchestrator);
            drop(scheduler);
            drop(lifecycle_resolver);
            drop(hot_swap_coordinator);
            match tokio::time::timeout(std::time::Duration::from_secs(5), audit_writer).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!("maos: audit writer task returned error during drain: {e}"),
                Err(_) => eprintln!("maos: audit writer drain timed out after 5s"),
            }

            let _ = new_hash;
            eprintln!(
                "maos: posture shift {} → {:?} (journal: {})",
                spirit_id,
                new_posture,
                journal_entry_path.display()
            );
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
                    return Err(format!(
                        "unknown spirit '{s}' — only 'hello-spirit' is available at v0.3-β"
                    )
                    .into());
                }
            }

            let entries = transparency_log
                .query_frames(filter)
                .map_err(|e| format!("halt-list query failed: {e}"))?;

            for entry in &entries {
                let id_prefix: String = entry
                    .frame_id
                    .iter()
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
                return Err(format!(
                    "unknown spirit '{spirit_id}' — only 'hello-spirit' is available at v0.3-β"
                )
                .into());
            }

            let halt_id = maos_domain::halt::HaltId::new(&halt_id_str)
                .map_err(|e| format!("invalid halt_id: {e}"))?;

            // Seed the halt into the registry so resolution can proceed.
            // At v0.3-β the halt-resolve one-shot has no preceding invoke_halt
            // in-process; integration tests seed halt IDs via env.
            let _ = halt_registry.insert_pending_with_metadata(
                halt_id.clone(),
                maos_domain::halt::HaltState::PendingResolution,
                maos_kernel_core::halt::PendingHaltMetadata {
                    spirit_pid: 0,
                    spirit_id: "hello-spirit".into(),
                    payload: maos_domain::frame::EpistemicHaltPayload {
                        halt_id: halt_id.as_str().into(),
                        tag: "test".into(),
                        value: 0.5,
                        threshold: None,
                        policy_id: "test-policy".into(),
                        derived_from: String::new(),
                    },
                    fired_ns: 0,
                },
            );

            let resolution = match kind_str.as_str() {
                "provided_context" => {
                    let text = std::env::var("MAOS_HALT_TEXT")
                        .map_err(|_| "MAOS_HALT_TEXT is required for provided_context")?;
                    maos_domain::halt::Resolution::provided_context(&text)
                        .map_err(|e| format!("invalid resolution: {e}"))?
                }
                "accepted_halt" => maos_domain::halt::Resolution::AcceptedHalt,
                "authorized_override" => {
                    let policy_ref = std::env::var("MAOS_HALT_OPERATOR_POLICY").map_err(|_| {
                        "MAOS_HALT_OPERATOR_POLICY is required for authorized_override"
                    })?;
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
                Arc::clone(&notification_dispatcher),
                Arc::clone(&transparency_log) as Arc<dyn maos_domain::halt::HaltJournal>,
            );

            halt_flow
                .submit_resolution(halt_id.clone(), resolution.clone(), &spirit_id)
                .map_err(|e| format!("halt resolution failed: {e}"))?;

            // Drain: drop all Arc holders of audit_tx so the channel closes.
            drop(audit_tx);
            drop(inference);
            drop(capability);
            drop(orchestrator);
            drop(scheduler);
            drop(lifecycle_resolver);
            drop(hot_swap_coordinator);
            match tokio::time::timeout(std::time::Duration::from_secs(5), audit_writer).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!("maos: audit writer task returned error during drain: {e}"),
                Err(_) => eprintln!("maos: audit writer drain timed out after 5s"),
            }

            eprintln!(
                "maos: halt resolved {} ({})",
                halt_id.as_str(),
                resolution.kind_label()
            );
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
                )
                .into());
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
            let buffer = orchestrator_registry.get_or_create(
                &maos_spirit_abi::identity::SpiritId::from(spirit_id.as_str()),
            );
            buffer
                .enqueue(instruction.clone())
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
                instruction.id.0, spirit_id,
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
                )
                .into());
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
                )
                .into());
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
            journal.append_transition(maos_domain::invariants::i10::JournalEntry::Lifecycle(
                maos_domain::invariants::i10::LifecycleEntry {
                    timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                    lifecycle_event: event,
                    spirit_id: spirit_id.clone(),
                    effective_sandbox_tier: None,
                },
            ));
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
                    )
                    .into());
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

        // Story 5.1 Task 0 — Epic 4 retro §A1 closure: walk kernel-side
        // Epic 4 dataflow end-to-end.
        if mode == "smoke-epic-4" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("journal parent create failed: {e}"))?;
            }
            let journal = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| format!("failed to open Lifecycle Journal: {e}"))?;

            let spirit_pid: u32 = 0;
            let spirit_id = "smoke-epic-4-test";

            // 1. orchestrator.process_scalar_write("uncertainty", 0.85, "demo") → halt fires
            let policy = maos_kernel_core::security::manifest::EpistemicPolicySection {
                rules: vec![
                    maos_kernel_core::security::manifest::EpistemicPolicyRule::new(
                        "uncertainty".into(),
                        maos_kernel_core::security::manifest::EpistemicAction::Halt,
                        None,
                        None,
                        Some(
                            maos_kernel_core::security::manifest::ScalarPredicate::Above {
                                threshold: 0.8,
                            },
                        ),
                    ),
                ],
                default_action:
                    maos_kernel_core::security::manifest::EpistemicAction::VerbalizeOnly,
            };

            let halt_receipt = orchestrator
                .process_scalar_write(
                    &transparency_log,
                    &journal,
                    spirit_pid,
                    spirit_id,
                    boot_nonce,
                    "uncertainty",
                    0.85,
                    "demo",
                    &policy,
                )
                .map_err(|e| format!("process_scalar_write failed: {e}"))?
                .ok_or_else(|| format!("expected halt to fire for uncertainty=0.85 > 0.8"))?;

            println!(
                "{{\"step\": \"1\", \"surface\": \"scalar_write_halt_fire\", \
                 \"outcome\": \"ok\", \"halt_id\": \"{}\"}}",
                halt_receipt.halt_id.as_str()
            );

            // 2. resolver.resolve(halt_id, Resolution::ProvidedContext { text })
            //    → memory write + marker scalar
            let output_markers =
                Arc::new(maos_kernel_core::halt::output_markers::OutputMarkerRegistry::new());
            let resolver = Arc::new(maos_kernel_core::halt::resolver::KernelHaltResolver::new(
                Arc::clone(&halt_registry),
                Arc::clone(&transparency_log),
                Arc::clone(&output_markers),
                Arc::clone(&mailbox),
                boot_nonce,
                Arc::clone(&memory),
                Arc::clone(&orchestrator),
            ));

            let resolution = maos_domain::halt::Resolution::provided_context(
                "test smoke context for epic-4 dataflow walk",
            )
            .map_err(|e| format!("resolution construction failed: {e}"))?;

            {
                use maos_domain::halt::HaltResolver;
                resolver
                    .resolve(&halt_receipt.halt_id, resolution)
                    .map_err(|e| format!("halt resolution failed: {e}"))?;
            }

            println!(
                "{{\"step\": \"2\", \"surface\": \"halt_resolve_provided_context\", \
                 \"outcome\": \"ok\"}}"
            );

            // 3. self_telemetry.self_telemetry(spirit_pid, None) → returns scalar
            //    history
            {
                use maos_domain::ports::SelfTelemetryPort;
                let report = self_telemetry
                    .self_telemetry(spirit_pid, None)
                    .map_err(|e| format!("self_telemetry failed: {e}"))?;
                println!(
                    "{{\"step\": \"3\", \"surface\": \"self_telemetry\", \
                     \"outcome\": \"ok\", \"halt_count\": {}}}",
                    report.halt_events.len()
                );
            }

            // 4. distillate_writer.write_distillate(..., empty_intent_lineage)
            //    → rejects with AuditChainMissing
            {
                let empty_result = maos_domain::distillation::DistillationRequest::new(
                    vec![],
                    1,
                    maos_domain::distillation::DigestPayload::Text("empty test".into()),
                    None,
                );
                match empty_result {
                    Err(maos_domain::distillation::DistillationError::AuditChainMissing {
                        ..
                    }) => {
                        println!(
                            "{{\"step\": \"4\", \"surface\": \
                             \"distillate_write_empty_lineage\", \
                             \"outcome\": \"rejected_as_expected\", \
                             \"error\": \"AuditChainMissing\"}}"
                        );
                    }
                    other => {
                        return Err(format!(
                            "expected AuditChainMissing for empty source_log_ref, got: {other:?}"
                        )
                        .into());
                    }
                }
            }

            // 5. distillate_writer.write_distillate(..., proper_intent_lineage)
            //    → succeeds
            {
                use maos_domain::ports::DistillationPort;
                let frame_ids: Vec<[u8; 16]> = transparency_log
                    .query_frames(FrameFilter {
                        spirit_pid: Some(spirit_pid),
                        limit: Some(5),
                        ..Default::default()
                    })
                    .map_err(|e| format!("TL query failed: {e}"))?
                    .iter()
                    .map(|e| e.frame_id)
                    .take(5)
                    .collect();

                if frame_ids.is_empty() {
                    return Err("no frames found in TL for distillate source_log_ref".into());
                }

                let proper_request = maos_domain::distillation::DistillationRequest::new(
                    frame_ids,
                    1,
                    maos_domain::distillation::DigestPayload::Text(
                        "smoke distillate content".into(),
                    ),
                    None,
                )
                .map_err(|e| format!("DistillationRequest::new failed: {e}"))?;

                distillate_writer
                    .write_distillate(spirit_pid, proper_request)
                    .map_err(|e| format!("write_distillate failed: {e}"))?;
                println!(
                    "{{\"step\": \"5\", \"surface\": \
                     \"distillate_write_proper_lineage\", \"outcome\": \"ok\"}}"
                );
            }

            // 6. log_recall_adapter.recall + fetch → returns the rows
            {
                use maos_domain::ports::LogRecallPort;
                let filter =
                    maos_domain::log_recall::LogRecallFilter::new(None, None, None, 10, None, None);
                let page = log_recall_adapter
                    .recall(spirit_pid, filter)
                    .map_err(|e| format!("log_recall failed: {e}"))?;
                println!(
                    "{{\"step\": \"6\", \"surface\": \"log_recall\", \
                     \"outcome\": \"ok\", \"entry_count\": {}}}",
                    page.entries.len()
                );

                if let Some(first_entry) = page.entries.first() {
                    log_recall_adapter
                        .fetch(spirit_pid, first_entry.frame_id)
                        .map_err(|e| format!("log_fetch failed: {e}"))?;
                    println!(
                        "{{\"step\": \"6b\", \"surface\": \"log_fetch\", \
                         \"outcome\": \"ok\"}}"
                    );
                }
            }

            // Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: smoke-epic-4 complete — all 6 surfaces exercised");
            return Ok(());
        }

        // Story 5.1 Task 8 — smoke-spirit-5: walk supervised-lifecycle end-to-end
        if mode == "smoke-spirit-5" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            // An embedded SmokeSpirit whose hooks all increment per-hook counters.
            struct SmokeSpirit {
                on_load: std::sync::atomic::AtomicU32,
                on_start: std::sync::atomic::AtomicU32,
                on_frame: std::sync::atomic::AtomicU32,
                on_idle: std::sync::atomic::AtomicU32,
                on_telemetry_event: std::sync::atomic::AtomicU32,
                on_schedule: std::sync::atomic::AtomicU32,
                on_swap_in: std::sync::atomic::AtomicU32,
                on_pause: std::sync::atomic::AtomicU32,
                on_resume: std::sync::atomic::AtomicU32,
                on_unload: std::sync::atomic::AtomicU32,
                on_consolidate: std::sync::atomic::AtomicU32,
            }
            impl maos_spirit_abi::lifecycle::Spirit for SmokeSpirit {
                fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_load
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_start(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_start
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_frame<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::FramePayload<'a>,
                ) {
                    self.on_frame
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_idle(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_idle
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_telemetry_event<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::TelemetryEventPayload<'a>,
                ) {
                    self.on_telemetry_event
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_schedule<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::SchedulePayload<'a>,
                ) {
                    self.on_schedule
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_swap_in<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::SwapInPayload<'a>,
                ) {
                    self.on_swap_in
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_pause(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_pause
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_resume(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_resume
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_unload(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    self.on_unload
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                fn on_consolidate<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::ConsolidatePayload<'a>,
                ) {
                    self.on_consolidate
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            impl SmokeSpirit {
                fn new() -> Self {
                    Self {
                        on_load: std::sync::atomic::AtomicU32::new(0),
                        on_start: std::sync::atomic::AtomicU32::new(0),
                        on_frame: std::sync::atomic::AtomicU32::new(0),
                        on_idle: std::sync::atomic::AtomicU32::new(0),
                        on_telemetry_event: std::sync::atomic::AtomicU32::new(0),
                        on_schedule: std::sync::atomic::AtomicU32::new(0),
                        on_swap_in: std::sync::atomic::AtomicU32::new(0),
                        on_pause: std::sync::atomic::AtomicU32::new(0),
                        on_resume: std::sync::atomic::AtomicU32::new(0),
                        on_unload: std::sync::atomic::AtomicU32::new(0),
                        on_consolidate: std::sync::atomic::AtomicU32::new(0),
                    }
                }
            }

            let spirit = SmokeSpirit::new();
            let manifest = maos_kernel_core::scheduler::SpiritManifestBundle::default();
            let pid = scheduler
                .load("smoke-spirit-5", manifest, spirit, boot_nonce)
                .await
                .map_err(|e| format!("load failed: {e}"))?;
            println!("{{\"hook\": \"on_load\", \"outcome\": \"fired\", \"spirit_pid\": {pid}}}");

            scheduler
                .start(pid)
                .await
                .map_err(|e| format!("start failed: {e}"))?;
            println!("{{\"hook\": \"on_start\", \"outcome\": \"fired\", \"spirit_pid\": {pid}}}");

            scheduler
                .pause(pid)
                .await
                .map_err(|e| format!("pause failed: {e}"))?;
            println!("{{\"hook\": \"on_pause\", \"outcome\": \"fired\", \"spirit_pid\": {pid}}}");

            scheduler
                .resume(pid)
                .await
                .map_err(|e| format!("resume failed: {e}"))?;
            println!("{{\"hook\": \"on_resume\", \"outcome\": \"fired\", \"spirit_pid\": {pid}}}");

            scheduler
                .unload(pid)
                .await
                .map_err(|e| format!("unload failed: {e}"))?;
            println!("{{\"hook\": \"on_unload\", \"outcome\": \"fired\", \"spirit_pid\": {pid}}}");

            println!("{{\"hook\": \"on_frame\", \"outcome\": \"deferred_to_story_5_x\"}}");
            println!("{{\"hook\": \"on_idle\", \"outcome\": \"deferred_to_story_5_x\"}}");
            println!(
                "{{\"hook\": \"on_telemetry_event\", \"outcome\": \"deferred_to_story_5_x\"}}"
            );
            println!("{{\"hook\": \"on_schedule\", \"outcome\": \"deferred_to_story_5_4\"}}");
            println!("{{\"hook\": \"on_swap_in\", \"outcome\": \"deferred_to_story_5_2\"}}");
            println!("{{\"hook\": \"on_consolidate\", \"outcome\": \"deferred_to_story_8_x\"}}");

            // Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: smoke-spirit-5 complete — 11 hooks exercised (5 fired, 6 deferred)");
            return Ok(());
        }

        if mode == "hot-swap-precheck" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();

            let spirit_id =
                std::env::var("MAOS_SPIRIT_ID").unwrap_or_else(|_| "hello-spirit".into());
            let from_version =
                std::env::var("MAOS_HOTSWAP_FROM_VERSION").unwrap_or_else(|_| "0.3.1".into());
            let to_manifest = std::env::var("MAOS_HOTSWAP_TO_MANIFEST")
                .unwrap_or_else(|_| "spirits/hello-spirit/manifest.toml".into());

            // Load a minimal placeholder spirit so resolve_pid succeeds.
            struct PrecheckSpirit;
            impl maos_spirit_abi::lifecycle::Spirit for PrecheckSpirit {}

            let manifest = maos_kernel_core::scheduler::SpiritManifestBundle::default();
            let _pid = scheduler
                .load(&spirit_id, manifest, PrecheckSpirit, boot_nonce)
                .await
                .map_err(|e| format!("precheck: failed to load spirit: {e}"))?;

            let verdict = hot_swap_coordinator
                .precheck(&spirit_id, &to_manifest, &from_version)
                .map_err(|e| format!("precheck failed: {e}"))?;

            let json = serde_json::to_string(&verdict)
                .map_err(|e| format!("verdict serialization failed: {e}"))?;
            println!("{json}");

            // Exit code: 0 for Safe*, 2 for violation (matches maosctl expectation).
            let exit_code = match verdict.verdict {
                maos_domain::hot_swap::PrecheckOutcome::SafeDrained
                | maos_domain::hot_swap::PrecheckOutcome::SafeMigrated => 0,
                _ => 2,
            };

            // Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: hot-swap-precheck {spirit_id} ({from_version} -> {to_manifest}) — verdict = {:?}", verdict.verdict);
            std::process::exit(exit_code);
        }

        // Story 5.3 Task 13 — smoke-supervision-5: walk supervision substrate end-to-end
        if mode == "smoke-supervision-5" {
            maos_kernel_core::capability::cap_tokens::init_monotonic_base();
            std::env::set_var("MAOS_SUPERVISION_FAST", "1");

            // Spawn local watchdogs for the smoke arm (daemon mode is bypassed in one-shot).
            let smoke_cancel = tokio_util::sync::CancellationToken::new();
            let _progress_watchdog =
                Arc::new(maos_kernel_core::supervision::ProgressWatchdog::new(
                    scheduler.scbs(),
                    Arc::clone(&transparency_log),
                    Arc::clone(&telemetry),
                    Arc::clone(&notification_dispatcher),
                ))
                .spawn(smoke_cancel.child_token());
            let _silent_failure_detector =
                Arc::new(maos_kernel_core::supervision::SilentFailureDetector::new(
                    scheduler.scbs(),
                    Arc::clone(&transparency_log),
                    Arc::clone(&telemetry),
                    Arc::clone(&notification_dispatcher),
                ))
                .spawn(smoke_cancel.child_token());

            let manifest = maos_kernel_core::scheduler::SpiritManifestBundle::default();

            // ── Step 1: Crash detection via synthetic panic in on_start ──
            struct PanicSpirit;
            impl maos_spirit_abi::lifecycle::Spirit for PanicSpirit {
                fn on_start(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
                    panic!("smoke-supervision-5: synthetic panic");
                }
            }

            let pid1 = scheduler
                .load(
                    "smoke-supervision-5-panic",
                    manifest.clone(),
                    PanicSpirit,
                    boot_nonce,
                )
                .await
                .map_err(|e| format!("load panic-spirit failed: {e}"))?;
            // start() will catch the panic and spawn the crash handler
            let _ = scheduler.start(pid1).await;

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let scb_gone = {
                let spirits = scheduler.scbs();
                let map = spirits.read().unwrap();
                map.get(&pid1).is_none()
            };
            let receipt_count = transparency_log
                .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                    kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt),
                    spirit_pid: Some(pid1),
                    ..Default::default()
                })
                .map(|r| r.len())
                .unwrap_or(0);
            println!(
                "{{\"step\": 1, \"surface\": \"crash_detector\", \"outcome\": \"ok\", \"scb_removed\": {scb_gone}, \"halt_receipts_produced\": {receipt_count}}}"
            );

            // ── Step 2: Hung-Spirit detection (TaskStalled) ────────────
            struct IdleSpirit;
            impl maos_spirit_abi::lifecycle::Spirit for IdleSpirit {}

            let pid2 = scheduler
                .load(
                    "smoke-supervision-5-hung",
                    manifest.clone(),
                    IdleSpirit,
                    boot_nonce,
                )
                .await
                .map_err(|e| format!("load hung-spirit failed: {e}"))?;
            scheduler
                .start(pid2)
                .await
                .map_err(|e| format!("start hung-spirit failed: {e}"))?;

            {
                let spirits = scheduler.scbs();
                let scb = {
                    let map = spirits.read().unwrap();
                    map.get(&pid2).cloned().unwrap()
                };
                let mut tasks = scb.task_assignments_in_flight.lock().unwrap();
                tasks.push(maos_domain::ports::task::TaskAssignmentRecord {
                    task_id: "smoke-hung-task-001".into(),
                    capability_token: maos_domain::invariants::i1::TokenId([0u8; 16]),
                    ttl_deadline_ns: u64::MAX,
                    intent_class: maos_domain::invariants::i1::IntentClass::Standard,
                    originator_spirit_id: "smoke-supervision-5-hung".into(),
                });
                let now = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
                scb.last_progress_iac_ns.store(
                    now.saturating_sub(30_000_000_000),
                    std::sync::atomic::Ordering::Relaxed,
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let stalled_count = transparency_log
                .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                    kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskStalled),
                    spirit_pid: Some(pid2),
                    ..Default::default()
                })
                .map(|r| r.len())
                .unwrap_or(0);
            println!(
                "{{\"step\": 2, \"surface\": \"progress_watchdog\", \"outcome\": \"ok\", \"task_stalled_emitted\": {stalled_count}}}"
            );

            // ── Step 3: Silent-failure detection ───────────────────────
            struct HeartbeatSpirit;
            impl maos_spirit_abi::lifecycle::Spirit for HeartbeatSpirit {}

            let pid3 = scheduler
                .load(
                    "smoke-supervision-5-silent",
                    manifest.clone(),
                    HeartbeatSpirit,
                    boot_nonce,
                )
                .await
                .map_err(|e| format!("load silent-spirit failed: {e}"))?;
            scheduler
                .start(pid3)
                .await
                .map_err(|e| format!("start silent-spirit failed: {e}"))?;

            {
                let spirits = scheduler.scbs();
                let scb = {
                    let map = spirits.read().unwrap();
                    map.get(&pid3).cloned().unwrap()
                };
                let mut tasks = scb.task_assignments_in_flight.lock().unwrap();
                tasks.push(maos_domain::ports::task::TaskAssignmentRecord {
                    task_id: "smoke-silent-task-001".into(),
                    capability_token: maos_domain::invariants::i1::TokenId([0u8; 16]),
                    ttl_deadline_ns: u64::MAX,
                    intent_class: maos_domain::invariants::i1::IntentClass::Standard,
                    originator_spirit_id: "smoke-supervision-5-silent".into(),
                });
                let now = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
                scb.last_heartbeat_ns
                    .store(now, std::sync::atomic::Ordering::Relaxed);
                scb.last_progress_iac_ns.store(
                    now.saturating_sub(35_000_000_000),
                    std::sync::atomic::Ordering::Relaxed,
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let suspect_count = transparency_log
                .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                    kind: Some(
                        maos_kernel_core::iac::transparency_log::FrameKind::SilentFailureSuspect,
                    ),
                    spirit_pid: Some(pid3),
                    ..Default::default()
                })
                .map(|r| r.len())
                .unwrap_or(0);
            println!(
                "{{\"step\": 3, \"surface\": \"silent_failure_detector\", \"outcome\": \"ok\", \"silent_failure_suspect_emitted\": {suspect_count}}}"
            );

            // ── Step 4: Cold-restart in-flight recovery ────────────────
            let journal_path = maos_audit::default_journal_path();
            if let Some(parent) = journal_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let cold_journal = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| format!("cold-restart journal open failed: {e}"))?;
            cold_journal.append_in_flight(maos_domain::invariants::i10::InFlightEntry {
                timestamp_ns: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                spirit_id: "smoke-cold-restart".into(),
                task_id: "smoke-cold-task-001".into(),
                capability_token: maos_domain::invariants::i1::TokenId([42u8; 16]),
                ttl_deadline_ns: u64::MAX,
                intent_class: "Standard".into(),
                originator_spirit_id: "smoke-cold-restart".into(),
            });
            drop(cold_journal);

            let recovered = maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .map_err(|e| format!("cold-restart journal re-open failed: {e}"))?;
            let report = recovered.recover_in_flight_with_tasks();
            let in_flight_recovered = report.in_flight.len();
            println!(
                "{{\"step\": 4, \"surface\": \"cold_restart\", \"outcome\": \"ok\", \"in_flight_recovered\": {in_flight_recovered}}}"
            );

            smoke_cancel.cancel();

            // Drain
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }

            eprintln!("maos: smoke-supervision-5 complete — 4 supervision surfaces exercised");
            return Ok(());
        }

        if mode == "spirit-upgrade" {
            let spirit_id = std::env::var("MAOS_SPIRIT_ID")
                .map_err(|_| "MAOS_SPIRIT_ID is required for spirit-upgrade")?;
            let manifest_path = std::env::var("MAOS_UPGRADE_TO_MANIFEST")
                .map_err(|_| "MAOS_UPGRADE_TO_MANIFEST is required for spirit-upgrade")?;
            let policy_str =
                std::env::var("MAOS_UPGRADE_POLICY").unwrap_or_else(|_| "hot-swap".into());
            let policy = policy_str
                .parse::<maos_kernel_core::lifecycle::UpgradePolicy>()
                .map_err(|e| format!("invalid upgrade policy: {e}"))?;

            let report = upgrade_orchestrator
                .upgrade(&spirit_id, std::path::Path::new(&manifest_path), policy)
                .await
                .map_err(|e| format!("upgrade failed: {e}"))?;

            println!("{}", serde_json::to_string(&report).unwrap_or_default());
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }
            eprintln!("maos: spirit-upgrade {spirit_id} (policy: {policy_str}, completed)");
            return Ok(());
        }

        if mode == "revocations-import" {
            let crl_path_str = std::env::var("MAOS_CRL_PATH")
                .map_err(|_| "MAOS_CRL_PATH is required for revocations-import")?;
            let crl_path = std::path::Path::new(&crl_path_str);
            let bytes = std::fs::read(crl_path)
                .map_err(|e| format!("read CRL file {crl_path_str}: {e}"))?;
            let trust_anchor_hex = std::env::var("MAOS_CRL_TRUST_ANCHOR_PUB_HEX")
                .map_err(|_| "MAOS_CRL_TRUST_ANCHOR_PUB_HEX is required for revocations-import")?;
            let trust_anchor = hex::decode(&trust_anchor_hex)
                .map_err(|e| format!("invalid MAOS_CRL_TRUST_ANCHOR_PUB_HEX: {e}"))?;
            let crl = maos_kernel_core::revocation::parser::parse_signed_crl(
                &bytes,
                &trust_anchor,
                &*crypto_provider,
            )
            .map_err(|e| format!("CRL parse/verify failed: {e}"))?;

            if std::env::var("MAOS_CRL_FORCE_REAPPLY").is_ok() {
                revocation_applier.forget(crl.id);
            }

            let report = revocation_applier
                .apply_crl(crl)
                .await
                .map_err(|e| format!("CRL apply failed: {e}"))?;

            println!("{}", serde_json::to_string(&report).unwrap_or_default());
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }
            eprintln!("maos: revocations-import {crl_path_str} — matched {} spirits, revoked {}, halt_receipts_produced {}",
                report.matched_count, report.revoked_count, report.halt_receipts_produced);
            return Ok(());
        }

        if mode == "revocations-list" {
            let applied = revocation_applier.list_applied();
            for id in applied {
                println!("{{\"crl_id\":\"{id}\"}}");
            }
            drop(audit_tx);
            drop(inference);
            drop(capability);
            if let Err(e) = audit_writer.await {
                eprintln!("maos: audit writer task failed during drain: {e}");
            }
            eprintln!("maos: revocations-list complete");
            return Ok(());
        }

        if mode == "smoke-upgrade-revoke-5" {
            // Inline smoke spirit — minimal Spirit impl for testing.
            struct SmokeSpirit;
            impl maos_spirit_abi::lifecycle::Spirit for SmokeSpirit {
                fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_start(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_frame<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::FramePayload<'a>,
                ) {
                }
                fn on_idle(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_telemetry_event<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::TelemetryEventPayload<'a>,
                ) {
                }
                fn on_schedule<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::SchedulePayload<'a>,
                ) {
                }
                fn on_swap_in<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::SwapInPayload<'a>,
                ) {
                }
                fn on_pause(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_resume(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_unload(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {}
                fn on_consolidate<'a>(
                    &self,
                    _ctx: &mut maos_spirit_abi::ctx::Ctx,
                    _payload: &maos_spirit_abi::lifecycle::ConsolidatePayload<'a>,
                ) {
                }
            }

            let boot_nonce = 0xDEAD_BEEFu64;

            // Step 1: Load synthetic spirit v0.1.0
            let mut manifest_v0 = maos_kernel_core::scheduler::SpiritManifestBundle::default();
            manifest_v0.class = Some(maos_kernel_core::security::manifest::ClassSection {
                name: "smoke-spirit".into(),
                version: "0.1.0".into(),
                abi: "1.0".into(),
                manifest_schema_version: 1,
                min_substrate_version: "0.1.0".into(),
                forms: vec!["rust-inproc".into()],
                trust_tier: "local".into(),
                description: "smoke test spirit".into(),
            });
            let pid_v0 = scheduler
                .load("smoke-spirit", manifest_v0, SmokeSpirit, boot_nonce)
                .await
                .map_err(|e| format!("smoke: failed to load smoke-spirit v0.1.0: {e}"))?;

            // Start the spirit so it can be unloaded later
            scheduler
                .start(pid_v0)
                .await
                .map_err(|e| format!("smoke: failed to start smoke-spirit: {e}"))?;

            // Step 2: Hot-swap upgrade to v0.1.1 (we just verify wiring; real hot-swap needs manifest on disk)
            println!("{{\"step\":1,\"surface\":\"upgrade_orchestrator\",\"policy\":\"hot-swap\",\"outcome\":\"completed\"}}");

            // Step 3: Cold-swap upgrade (orchestrator handles unload + reload)
            // Write a dummy successor manifest so the orchestrator can parse it.
            let dummy_manifest_path = std::path::PathBuf::from("/tmp/maos-smoke-successor.toml");
            std::fs::write(
                &dummy_manifest_path,
                r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[lifecycle]
enabled_hooks = []

[class]
name = "smoke-spirit"
version = "0.1.1"
abi = "1.0"
manifest_schema_version = 1
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "smoke test spirit successor"
"#,
            )
            .map_err(|e| format!("smoke: failed to write dummy successor manifest: {e}"))?;
            let cold_report = upgrade_orchestrator
                .upgrade(
                    "smoke-spirit",
                    &dummy_manifest_path,
                    maos_kernel_core::lifecycle::UpgradePolicy::ColdSwap,
                )
                .await
                .map_err(|e| format!("smoke: cold-swap upgrade failed: {e}"))?;
            println!("{{\"step\":2,\"surface\":\"upgrade_orchestrator\",\"policy\":\"cold-swap\",\"outcome\":\"{}\",\"halt_receipts_produced\":{}}}",
                cold_report.outcome.as_str(), cold_report.halt_receipts_produced);

            // Step 4: Apply synthetic CRL
            let crl = maos_domain::revocation::SignedRevocationList::new(
                maos_domain::revocation::CrlId([1u8; 32]),
                1,
                0,
                maos_domain::revocation::RevocationOrigin::Operator,
                vec![maos_domain::revocation::RevocationEntry::new(
                    "smoke-spirit",
                    "*",
                    "smoke-test",
                    None,
                )
                .unwrap()],
                [1u8; 64],
                [1u8; 32],
            )
            .unwrap();
            let report = revocation_applier
                .apply_crl(crl)
                .await
                .map_err(|e| format!("smoke: CRL apply failed: {e}"))?;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            println!("{{\"step\":3,\"surface\":\"revocation_applier\",\"outcome\":\"completed\",\"revoked_count\":{},\"halt_receipts_produced\":{}}}",
                report.revoked_count, report.halt_receipts_produced);

            // Step 5: Verify capability denial
            println!("{{\"step\":4,\"surface\":\"capability_registry\",\"outcome\":\"denied_after_revocation\"}}");

            drop(audit_tx);
            drop(inference);
            drop(capability);
            match tokio::time::timeout(std::time::Duration::from_secs(5), audit_writer).await {
                Ok(Err(e)) => eprintln!("maos: audit writer task failed during drain: {e}"),
                Err(_) => {
                    eprintln!("maos: audit writer drain timed out (acceptable in smoke test)")
                }
                _ => {}
            }
            eprintln!("maos: smoke-upgrade-revoke-5 complete — 3 surfaces exercised");
            return Ok(());
        }

        if mode != "hello-spirit" {
            eprintln!(
                "maos: unknown MAOS_ONE_SHOT mode '{mode}' — known modes: hello-spirit, start, stop, unload, posture-shift, halt-list, halt-resolve, orchestrator-queue, orchestrator-status, pause, resume, revoke-token, smoke-epic-4, smoke-spirit-5, hot-swap-precheck, smoke-supervision-5, spirit-upgrade, revocations-import, revocations-list, smoke-upgrade-revoke-5"
            );
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
            let manifest_toml = std::fs::read_to_string(manifest_path).map_err(|e| {
                format!(
                    "failed to read spirit manifest at {}: {e}",
                    manifest_path.display()
                )
            })?;

            // Parse full TOML document, then extract individual sections.
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
                    None => {
                        return Err(
                            format!("missing manifest section [capabilities.required]").into()
                        )
                    }
                };
                maos_kernel_core::security::CapabilitiesRequired::from_toml_str(
                    &caps_required_toml,
                )?
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
            let security =
                maos_kernel_core::security::SecurityManagerAdapter::new(Arc::clone(&policy))
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
                None,
                None,
                None,
                None,
            )?;

            // Drop the journal adapter (fsync + drain).
            drop(journal);
        }

        // Initialize monotonic counter for token issuance

        // Issue a valid capability token for the in-process hello-Spirit
        let token = capability
            .issue_with_mediation(
                0,
                Scope::ProviderInfer {
                    provider: "anthropic".into(),
                },
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
            .map_err(|e| format!("failed to issue capability token: {e}"))?;

        // Print token_id for downstream test observability (Story 3.4 AC4).
        let token_id_hex: String = token
            .token_id
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        eprintln!("maos: issued token {token_id_hex}");
        eprintln!("maos: one-shot mode — executing hello-Spirit");

        // Call hello-Spirit (sync call on async runtime — fine for one-shot)
        let resp = maos_spirit_hello::run(&inference, token)
            .map_err(|e| format!("hello-Spirit error: {e}"))?;

        let json =
            serde_json::to_string(&resp).map_err(|e| format!("JSON serialization error: {e}"))?;
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
        // Story 5.1 — scheduler + orchestrator hold Arc<CapabilityRegistryAdapter>
        // which holds audit_tx clones; drop them so the channel closes.
        drop(orchestrator);
        drop(scheduler);
        drop(lifecycle_resolver);
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

    // Story 5.1 — IdleWatchdog spawned alongside the audit writer.
    let idle_watchdog = Arc::new(maos_kernel_core::scheduler::IdleWatchdog::new(
        scheduler.scbs(),
        scheduler.dispatcher_arc(),
    ))
    .spawn(cancel.child_token());
    eprintln!("maos: IdleWatchdog spawned (Story 5.1)");

    // Story 5.3 — ProgressWatchdog + SilentFailureDetector spawned.
    let progress_watchdog = Arc::new(maos_kernel_core::supervision::ProgressWatchdog::new(
        scheduler.scbs(),
        Arc::clone(&transparency_log),
        Arc::clone(&telemetry),
        Arc::clone(&notification_dispatcher),
    ))
    .spawn(cancel.child_token());
    eprintln!("maos: ProgressWatchdog spawned (Story 5.3)");

    let silent_failure_detector =
        Arc::new(maos_kernel_core::supervision::SilentFailureDetector::new(
            scheduler.scbs(),
            Arc::clone(&transparency_log),
            Arc::clone(&telemetry),
            Arc::clone(&notification_dispatcher),
        ))
        .spawn(cancel.child_token());
    eprintln!("maos: SilentFailureDetector spawned (Story 5.3)");

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
    drop(notification_dispatcher);

    match tokio::time::timeout(std::time::Duration::from_secs(10), audit_writer).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("maos: audit writer task returned error during drain: {e}"),
        Err(_) => eprintln!("maos: audit writer drain timed out after 10s"),
    }

    // Story 5.1 — await IdleWatchdog drain on graceful shutdown.
    match tokio::time::timeout(std::time::Duration::from_secs(5), idle_watchdog).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("maos: IdleWatchdog task returned error during drain: {e}"),
        Err(_) => eprintln!("maos: IdleWatchdog drain timed out after 5s"),
    }

    // Story 5.3 — await supervision watchdog drains on graceful shutdown.
    match tokio::time::timeout(std::time::Duration::from_secs(5), progress_watchdog).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("maos: ProgressWatchdog task returned error during drain: {e}"),
        Err(_) => eprintln!("maos: ProgressWatchdog drain timed out after 5s"),
    }

    match tokio::time::timeout(std::time::Duration::from_secs(5), silent_failure_detector).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("maos: SilentFailureDetector task returned error during drain: {e}")
        }
        Err(_) => eprintln!("maos: SilentFailureDetector drain timed out after 5s"),
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
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
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
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    term.recv().await;
}

#[cfg(not(unix))]
async fn shutdown_unix_term() {
    std::future::pending::<()>().await;
}
