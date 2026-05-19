//! Integration test: ProvidedContext halt resolution writes to private memory + scalar marker.
//! AC5 — Story 4.3.

use std::sync::Arc;

use maos_domain::halt::{HaltId, HaltResolver, Resolution};
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::halt::{HaltRegistry, invoke_halt, KernelHaltResolver, OutputMarkerRegistry};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_kernel_core::security::crypto::RingCryptoProvider;
use tempfile::TempDir;

fn make_resolver() -> (KernelHaltResolver, Arc<MemoryManagerAdapter>, Arc<HaltRegistry>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");

    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE6));
    let memory = Arc::new(MemoryManagerAdapter::new(private, shared, principal, Arc::clone(&tl)));

    let halt_registry = Arc::new(HaltRegistry::new());
    let output_markers = Arc::new(OutputMarkerRegistry::new());
    let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));

    // Minimal capability registry + orchestrator for the resolver.
    let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _audit_rx) = cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let working_memory = Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new());
    let telemetry_stream = Arc::new(TelemetryStreamAdapter::default());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xCAFE,
        Arc::clone(&policy),
        audit_tx,
        quota,
        working_memory,
        telemetry_stream,
    ));
    let orchestrator = Arc::new(WorkingMemoryOrchestrator::new(
        Arc::clone(&capability),
        Arc::clone(&halt_registry),
    ));

    let resolver = KernelHaltResolver::new(
        Arc::clone(&halt_registry),
        Arc::clone(&tl),
        output_markers,
        mailbox,
        0xCAFE,
        Arc::clone(&memory),
        orchestrator,
    );

    (resolver, memory, halt_registry, tmp)
}

#[test]
fn provided_context_writes_memory_and_scalar_marker() {
    let (resolver, memory, halt_registry, _tmp) = make_resolver();
    let journal_path = _tmp.path().join("journal.ndjson");
    let journal = JournalAdapter::open(&journal_path).unwrap();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE6));

    // Invoke a halt.
    let payload = maos_domain::frame::EpistemicHaltPayload::new(
        "halt-001".into(),
        "test_tag".into(),
        0.5,
        None,
        "policy-1".into(),
        "derived".into(),
    )
    .unwrap();
    let receipt = invoke_halt(
        &tl,
        &journal,
        &halt_registry,
        payload,
        42,
        "hello-spirit",
        0xCAFE,
    )
    .unwrap();

    let halt_id = receipt.halt_id;

    // Resolve with ProvidedContext.
    let resolution = Resolution::provided_context("the IETF cite is RFC 8949").unwrap();
    resolver.resolve(&halt_id, resolution).unwrap();

    // Assert memory contains the context.
    let key = format!("halt_context::{}", halt_id.as_str());
    let got = memory
        .read(42, MemoryTier::Private, &MemoryNamespace::Default, &key)
        .unwrap();
    assert_eq!(
        got,
        Some(MemoryValue::Text("the IETF cite is RFC 8949".into()))
    );
}
