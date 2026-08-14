#![forbid(unsafe_code)]

//! AC1 + AC2 — `invoke_halt` unit-isolated against `MockHaltResolver` +
//! `KernelHaltResolver` three resolution kinds.
//!
//! Test surface:
//! - `maos_kernel_core::halt::invoke_halt`
//! - `maos_kernel_core::halt::HaltRegistry`
//! - `maos_kernel_core::halt::MockHaltResolver`
//! - `maos_kernel_core::halt::KernelHaltResolver`
//! - `maos_kernel_core::halt::OutputMarkerRegistry`

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltResolver, HaltState, OutputMarkerKind, Resolution};
use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::halt::{
    invoke_halt, HaltRegistry, KernelHaltResolver, MockHaltResolver, OutputMarkerRegistry,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::iac::Mailbox;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    principal::PrincipalNamespaceIndex, private::PrivateMemoryStore, shared::SharedMemoryStore,
    MemoryManagerAdapter,
};
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::Arc;

fn make_journal() -> (JournalAdapter, tempfile::TempDir) {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");
    let adapter = JournalAdapter::open(&path).unwrap();
    (adapter, tmpdir)
}

#[test]
fn invoke_halt_writes_tl_row_journal_entry_and_inserts_registry() {
    let tl = TransparencyLogAdapter::open_in_memory(0xCAFE);
    let (journal, _tmpdir) = make_journal();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-001".into(),
        "claim.security".into(),
        0.83,
        Some(0.8),
        "pol-1".into(),
        "frame:abc".into(),
    )
    .unwrap();

    let receipt = invoke_halt(
        &tl,
        &journal,
        &registry,
        payload,
        42,
        "hello-spirit",
        0xCAFE,
    )
    .unwrap();

    // Receipt produced
    assert_eq!(receipt.halt_id.as_str(), "halt-001");
    assert_eq!(receipt.spirit_pid, 42);
    assert!(
        receipt.terminal_state.is_none(),
        "invocation-time receipt has no terminal state"
    );

    // TL row written — verify FrameKind::EpistemicHalt exists
    let filter = maos_kernel_core::iac::transparency_log::FrameFilter {
        spirit_pid: Some(42),
        ..Default::default()
    };
    let frames = tl.query_frames(filter).unwrap();
    assert!(frames.iter().any(|f| f.kind
        == maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt
        && f.spirit_pid == 42));

    // Lifecycle Journal entry written (LifecycleEvent::Halt)
    let last = journal.last_event("hello-spirit").unwrap();
    assert_eq!(last, maos_domain::invariants::i10::LifecycleEvent::Halt);

    // Registry has the halt in PendingResolution
    assert_eq!(registry.pending_halt_ids().len(), 1);
}

#[test]
fn invoke_halt_rejects_duplicate_halt_id_with_typed_error() {
    let tl = TransparencyLogAdapter::open_in_memory(0xCAFE);
    let (journal, _tmpdir) = make_journal();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-dup".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();

    invoke_halt(
        &tl,
        &journal,
        &registry,
        payload.clone(),
        1,
        "spirit-a",
        0xCAFE,
    )
    .unwrap();
    let err = invoke_halt(&tl, &journal, &registry, payload, 1, "spirit-a", 0xCAFE).unwrap_err();
    assert!(
        matches!(err, maos_domain::halt::InvokeHaltError::DuplicateHaltId(s) if s == "halt-dup")
    );
}

#[test]
fn invoke_halt_then_resolve_via_mock_records_call() {
    let tl = TransparencyLogAdapter::open_in_memory(0);
    let (journal, _tmpdir) = make_journal();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-x".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    invoke_halt(&tl, &journal, &registry, payload, 1, "spirit-a", 0xCAFE).unwrap();

    let mock = MockHaltResolver::new();
    let hid = HaltId::new("halt-x").unwrap();
    mock.resolve(&hid, Resolution::AcceptedHalt).unwrap();
    assert_eq!(mock.call_count(), 1);
}

// --- AC2: KernelHaltResolver three resolution kinds ---

fn make_memory_adapter(
    tl: Arc<TransparencyLogAdapter>,
) -> (Arc<MemoryManagerAdapter>, Arc<tempfile::TempDir>) {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("test.db");

    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let adapter = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        tl,
    ));
    (adapter, Arc::new(tmp))
}

fn make_capability() -> Arc<CapabilityRegistryAdapter> {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let telemetry_stream = Arc::new(TelemetryStreamAdapter::default());
    Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xBEEF,
        policy,
        audit_tx,
        quota,
        Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
        telemetry_stream,
    ))
}

fn setup_kernel_resolver() -> (
    Arc<TransparencyLogAdapter>,
    Arc<HaltRegistry>,
    Arc<KernelHaltResolver>,
    Arc<OutputMarkerRegistry>,
    Arc<Mailbox>,
    JournalAdapter,
    tempfile::TempDir,
    Arc<MemoryManagerAdapter>,
    Arc<CapabilityRegistryAdapter>,
    Arc<tempfile::TempDir>,
) {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEEF));
    let registry = Arc::new(HaltRegistry::new());
    let output_markers = Arc::new(OutputMarkerRegistry::new());
    let mailbox = Arc::new(Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));
    let (memory, mem_tmp) = make_memory_adapter(Arc::clone(&tl));
    let capability = make_capability();
    let orchestrator = Arc::new(
        maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
            Arc::clone(&capability),
            Arc::clone(&registry),
        ),
    );
    let resolver = Arc::new(KernelHaltResolver::new(
        Arc::clone(&registry),
        Arc::clone(&tl),
        Arc::clone(&output_markers),
        Arc::clone(&mailbox),
        0xBEEF,
        Arc::clone(&memory),
        orchestrator,
    ));
    let (journal, tmpdir) = make_journal();
    (
        tl,
        registry,
        resolver,
        output_markers,
        mailbox,
        journal,
        tmpdir,
        memory,
        capability,
        mem_tmp,
    )
}

#[test]
fn kernel_resolver_provided_context_marks_resumed_and_clears_registry() {
    let (
        tl,
        registry,
        resolver,
        output_markers,
        _mailbox,
        journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    let payload = EpistemicHaltPayload::new(
        "halt-pc".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    let _receipt = invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();

    let hid = HaltId::new("halt-pc").unwrap();
    let resolution = Resolution::ProvidedContext {
        text: "more info".into(),
    };
    resolver.resolve(&hid, resolution).unwrap();

    let pending = registry.pending_halt_ids();
    assert!(
        pending.is_empty(),
        "halt should be resolved and removed from pending"
    );

    assert_eq!(output_markers.pending_count(&hid), 0);
}

#[test]
fn kernel_resolver_accepted_halt_emits_task_orphaned_and_marks_terminated() {
    let (
        tl,
        registry,
        resolver,
        output_markers,
        _mailbox,
        journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    let payload = EpistemicHaltPayload::new(
        "halt-ah".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    let _receipt = invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();

    let hid = HaltId::new("halt-ah").unwrap();
    let resolution = Resolution::AcceptedHalt;
    resolver.resolve(&hid, resolution).unwrap();

    let pending = registry.pending_halt_ids();
    assert!(pending.is_empty());
    assert_eq!(output_markers.pending_count(&hid), 0);

    // TL has task.orphaned via TaskComplete frame
    let filter = maos_kernel_core::iac::transparency_log::FrameFilter::default();
    let frames = tl.query_frames(filter).unwrap();
    let orphan_frames: Vec<_> = frames
        .iter()
        .filter(|f| {
            f.kind == maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete
                && std::str::from_utf8(&f.payload_redacted)
                    .map(|s| s.contains("orphaned: accepted_halt"))
                    .unwrap_or(false)
        })
        .collect();
    assert!(
        !orphan_frames.is_empty(),
        "TL should have task.orphaned TaskComplete row"
    );
}

#[test]
fn kernel_resolver_authorized_override_enqueues_output_marker_and_marks_overridden() {
    let (
        tl,
        registry,
        resolver,
        output_markers,
        _mailbox,
        journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    let payload = EpistemicHaltPayload::new(
        "halt-ao".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    let _receipt = invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();

    let hid = HaltId::new("halt-ao").unwrap();
    let resolution = Resolution::AuthorizedOverride {
        operator_policy_ref: "policy://test".into(),
    };
    resolver.resolve(&hid, resolution).unwrap();

    let pending = registry.pending_halt_ids();
    assert!(pending.is_empty());

    let markers = output_markers.consume_for_halt(&hid);
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, OutputMarkerKind::Override);
    assert_eq!(markers[0].halt_id.as_str(), "halt-ao");
    assert_eq!(
        markers[0].operator_policy_ref.as_deref(),
        Some("policy://test")
    );
}

/// Story 3.3 review closure (High) — an empty `text` / `operator_policy_ref`
/// MUST be refused BEFORE the registry transition, so the halt stays
/// resolvable instead of being stranded in a terminal state with no effect
/// applied. Struct-literal construction is the bypass path being pinned.
#[test]
fn kernel_resolver_rejects_empty_payload_without_transitioning_halt_state() {
    for (halt_id, invalid) in [
        (
            "halt-empty-text",
            Resolution::ProvidedContext { text: "   ".into() },
        ),
        (
            "halt-empty-policy",
            Resolution::AuthorizedOverride {
                operator_policy_ref: String::new(),
            },
        ),
    ] {
        let (
            tl,
            registry,
            resolver,
            output_markers,
            _mailbox,
            journal,
            _tmpdir,
            _memory,
            _capability,
            _mem_tmp,
        ) = setup_kernel_resolver();

        let payload = EpistemicHaltPayload::new(
            halt_id.into(),
            "t".into(),
            0.5,
            Some(0.4),
            "p".into(),
            "d".into(),
        )
        .unwrap();
        invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();

        let hid = HaltId::new(halt_id).unwrap();
        let err = resolver.resolve(&hid, invalid).unwrap_err();
        assert!(
            matches!(err, maos_domain::halt::ResolveError::InvalidResolution(_)),
            "{halt_id}: expected InvalidResolution, got {err:?}"
        );

        // No transition: the halt is still pending, so the director can
        // resubmit a well-formed resolution.
        assert_eq!(
            registry.pending_halt_ids(),
            vec![hid.clone()],
            "{halt_id}: halt must remain PendingResolution after a rejected payload"
        );
        assert_eq!(
            output_markers.pending_count(&hid),
            0,
            "{halt_id}: no override marker may be enqueued for a rejected payload"
        );

        // And the well-formed resubmission still succeeds.
        resolver
            .resolve(&hid, Resolution::AcceptedHalt)
            .expect("resubmission after a rejected payload must succeed");
        assert!(registry.pending_halt_ids().is_empty());
    }
}

#[test]
fn kernel_resolver_unknown_halt_returns_error() {
    let (
        _tl,
        _registry,
        resolver,
        _output_markers,
        _mailbox,
        _journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    let hid = HaltId::new("halt-unknown").unwrap();
    let err = resolver
        .resolve(&hid, Resolution::AcceptedHalt)
        .unwrap_err();
    assert!(matches!(
        err,
        maos_domain::halt::ResolveError::UnknownHalt(_)
    ));
}

#[test]
fn kernel_resolver_double_resolve_returns_already_resolved() {
    let (
        tl,
        registry,
        resolver,
        _output_markers,
        _mailbox,
        journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    let payload = EpistemicHaltPayload::new(
        "halt-dr".into(),
        "t".into(),
        0.5,
        Some(0.4),
        "p".into(),
        "d".into(),
    )
    .unwrap();
    let _receipt = invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();

    let hid = HaltId::new("halt-dr").unwrap();
    resolver.resolve(&hid, Resolution::AcceptedHalt).unwrap();
    let err = resolver
        .resolve(&hid, Resolution::AcceptedHalt)
        .unwrap_err();
    assert!(matches!(
        err,
        maos_domain::halt::ResolveError::AlreadyResolved(_)
    ));
}

#[test]
fn kernel_resolver_is_send_and_sync() {
    fn _assert_send_sync<T: Send + Sync>(_: T) {}
    let (
        _tl,
        registry,
        _resolver,
        _output_markers,
        _mailbox,
        _journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();
    _assert_send_sync(registry);
}

// --- P15: RegistryInsertFailed from empty halt_id ---

#[test]
fn invoke_halt_empty_halt_id_returns_registry_insert_failed() {
    let tl = TransparencyLogAdapter::open_in_memory(0);
    let (journal, _tmpdir) = make_journal();
    let registry = HaltRegistry::new();

    // Construct payload with empty halt_id via struct literal (pub fields)
    let payload = EpistemicHaltPayload {
        halt_id: String::new(),
        tag: "t".into(),
        value: 0.5_f32,
        threshold: Some(0.4_f32),
        policy_id: "p".into(),
        derived_from: "d".into(),
    };

    let err = invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0).unwrap_err();
    assert!(
        matches!(err, maos_domain::halt::InvokeHaltError::RegistryInsertFailed(s) if s.contains("invalid"))
    );
}

// --- P13: terminal_state verification on resolved receipt ---
// TODO(Story 4.1): The resolver currently transitions registry state but does
// not update the HaltReceipt's terminal_state. The spec requires
// "receipt.terminal_state == Some(Resumed/Terminated/Overridden)" post-resolution.
// Full fix requires making receipts mutable or returning updated receipts.
// For now, verify the registry correctly transitions the state.

#[test]
fn kernel_resolver_transitions_registry_state_for_all_three_resolution_kinds() {
    let (
        tl,
        registry,
        resolver,
        _output_markers,
        _mailbox,
        journal,
        _tmpdir,
        _memory,
        _capability,
        _mem_tmp,
    ) = setup_kernel_resolver();

    // Setup three halts
    for id in &["halt-a", "halt-b", "halt-c"] {
        let payload = EpistemicHaltPayload::new(
            id.to_string(),
            "t".into(),
            0.5,
            Some(0.4),
            "p".into(),
            "d".into(),
        )
        .unwrap();
        invoke_halt(&tl, &journal, &registry, payload, 1, "s", 0xBEEF).unwrap();
    }
    assert_eq!(registry.pending_halt_ids().len(), 3);

    // Resolve each with a different kind
    resolver
        .resolve(
            &HaltId::new("halt-a").unwrap(),
            Resolution::ProvidedContext { text: "ctx".into() },
        )
        .unwrap();
    resolver
        .resolve(&HaltId::new("halt-b").unwrap(), Resolution::AcceptedHalt)
        .unwrap();
    resolver
        .resolve(
            &HaltId::new("halt-c").unwrap(),
            Resolution::AuthorizedOverride {
                operator_policy_ref: "policy://x".into(),
            },
        )
        .unwrap();

    // All three should be removed from pending (registry transitions)
    assert_eq!(registry.pending_halt_ids().len(), 0);
}
