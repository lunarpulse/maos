//! Story 27-1 closure — an ordinary door load binds from the manifest text it admitted.
//!
//! The door reads a manifest once, gates and pin-checks that text, then builds the Spirit.
//! The constructor used to reread the (mutable) pathname for the provider/model bindings,
//! so a file that changed between the two reads could bind a different model than the one
//! admitted. The witness is the real `prepare_spirit_from` over real kernel adapters: the
//! admitted text declares a provider the router registers, the file on disk declares one it
//! does not. A binding taken from the file would fail the build.
use super::{prepare_spirit_from, SpiritBuildDeps};
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse,
};
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::{Arc, Mutex};

const NONCE: u64 = 27_1;
const ORCHESTRATOR: &str = include_str!("../../../../spirits/orchestrator/manifest.toml");
const BOTH: &str = r#"provider.complete = ["anthropic.claude-haiku-5-5", "openai.gpt-4o-mini"]"#;
const ANTHROPIC_ONLY: &str = r#"provider.complete = ["anthropic.claude-haiku-5-5"]"#;

struct NoPort;

impl InferencePort for NoPort {
    fn complete(&self, _: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        Err(InferenceError::ProviderTransport("not called".into()))
    }
}

#[derive(Debug)]
struct NullSink;

impl ::observer::ActivitySink for NullSink {
    fn emit(&self, _: ::observer::ObservedActivity) {}
}

/// The registered inference provider is `openai` only.
fn seam() -> butler::ButlerSeam {
    butler::ButlerSeam {
        inference: Some((
            Arc::new(|| Some(Arc::new(NoPort) as Arc<dyn InferencePort + Send + Sync>)),
            vec!["openai".to_owned()],
        )),
        mint: Arc::new(|_, _| Err("not minted".into())),
        digest: Arc::new(butler::MorningDigestRenderer::new(
            std::path::PathBuf::new(),
            std::path::PathBuf::new(),
            Arc::new(|_, _| {}),
            Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        )),
        rollover: false,
        clock: Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        acceptance: None,
        scene_butler: Arc::default(),
        context: None,
    }
}

fn deps(dir: &std::path::Path) -> SpiritBuildDeps {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(
            &dir.join("transparency.sqlite"),
            &dir.join("transparency.sqlite"),
            NONCE,
        )
        .expect("transparency log"),
    );
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        Ed25519SigningKey::new([0x27; 32]),
        NONCE,
        policy,
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::clone(&telemetry),
    ));
    let database = dir.join("memory.sqlite");
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(dir.join("private"), 4)),
        Arc::new(SharedMemoryStore::open(&database).expect("shared memory")),
        Arc::new(PrincipalNamespaceIndex::open(&database).expect("principal index")),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        Arc::clone(&memory),
        iac,
        Arc::clone(&halt_registry),
        metrics,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson"))
            .expect("journal"),
    );
    let memory_any: Arc<dyn std::any::Any + Send + Sync> = Arc::clone(&memory) as _;
    SpiritBuildDeps {
        orchestrator: Arc::new(
            maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
                capability,
                halt_registry,
            ),
        ),
        transparency_log: Arc::clone(&tl),
        shared_journal: journal,
        scheduler,
        boot_nonce: NONCE,
        activity_sink: Arc::new(NullSink),
        butler_seam: Arc::new(Mutex::new(Some(seam()))),
        distillate_writer: Arc::new(maos_kernel_core::iac::distillate::DistillateWriter::new(
            tl,
            memory_any,
        )),
        researcher: Arc::default(),
        redistill_sink: Arc::new(|_, _| {}),
    }
}

#[test]
fn a_file_that_differs_after_admission_cannot_change_what_the_door_binds() {
    let dir = tempfile::tempdir().expect("scratch dir");
    let deps = deps(dir.path());
    let on_disk = ORCHESTRATOR.replace(BOTH, ANTHROPIC_ONLY);
    assert_ne!(on_disk, ORCHESTRATOR, "the manifest line moved");
    let path = dir.path().join("manifest.toml");
    std::fs::write(&path, &on_disk).expect("write the differing file");

    // Control: the file's own bytes cannot be built (no declared provider is registered),
    // so a binding taken from the file fails the build.
    let control = prepare_spirit_from(None, &deps, &path, &on_disk);
    assert!(
        matches!(&control, Err(refusal) if refusal.to_string().contains("no inference provider")),
        "the differing file must not bind: {:?}",
        control.as_ref().map(|_| ()).map_err(ToString::to_string)
    );

    // The admitted text binds `openai` even though the pathname now holds other bytes.
    let admitted = prepare_spirit_from(None, &deps, &path, ORCHESTRATOR);
    assert!(
        admitted.is_ok(),
        "the admitted text must bind, whatever the file now holds: {:?}",
        admitted.map(|_| ()).map_err(|refusal| refusal.to_string())
    );
}
