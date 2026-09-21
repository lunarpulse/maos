//! Integration test: AC5 halt-resolution durability contract (Story 16.5).
//!
//! Two guarantees, proven end to end against the real `KernelHaltResolver` +
//! `HaltRegistry` + `HaltFlow` machinery:
//!
//! 1. A failed `ProvidedContext` context write must leave the halt in
//!    `PendingResolution` (retryable). The resolver validates first, executes
//!    side effects second, and commits the registry transition last — so a
//!    side-effect failure always precedes the state change.
//! 2. A failed approval-journal commit after a successful resolve must be
//!    compensable: the operator door's `rollback_resolution` restores
//!    `PendingResolution` + metadata, and a retry through a healthy journal
//!    lands the approval row and the terminal state exactly once.

use std::path::PathBuf;
use std::sync::Arc;

use maos_director_surface::halt_ui::{HaltFlow, HaltUiError};
use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{
    HaltId, HaltJournal, HaltJournalError, HaltResolver, HaltState, Resolution, ResolveError,
};
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::halt::{
    HaltRegistry, KernelHaltResolver, OutputMarkerRegistry, PendingHaltMetadata,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use tempfile::TempDir;

const SPIRIT_PID: u32 = 4242;
const SPIRIT_ID: &str = "spirit-halt-durability";
const CONTEXT_TEXT: &str = "the operator supplied the missing cite";
const BOOT_NONCE: u64 = 0x1655;

fn halt_payload(halt_id: &str) -> EpistemicHaltPayload {
    EpistemicHaltPayload::new(
        halt_id.to_string(),
        "confidence.below_threshold".to_string(),
        0.31,
        Some(0.4),
        "policy://epistemic/confidence-floor".to_string(),
        "derived::halt-durability-16-5".to_string(),
    )
    .unwrap()
}

fn pending_metadata(halt_id: &str) -> PendingHaltMetadata {
    PendingHaltMetadata {
        spirit_pid: SPIRIT_PID,
        spirit_id: SPIRIT_ID.to_string(),
        payload: halt_payload(halt_id),
        fired_ns: 1_700_000_000_000,
    }
}

/// Resolver wiring mirroring `halt_resolution_writes_memory.rs::make_resolver`.
/// `unavailable_private_memory` swaps the private-memory store for one whose
/// spill root is a RELATIVE path, which `PrivateMemoryStore::open_root`
/// (BH-4 containment) rejects fail-closed before any filesystem access —
/// the deterministic stand-in for "the Spirit has no working-memory backend".
#[allow(clippy::type_complexity)]
fn make_resolver(
    unavailable_private_memory: bool,
) -> (
    Arc<KernelHaltResolver>,
    Arc<HaltRegistry>,
    Arc<TransparencyLogAdapter>,
    Arc<MemoryManagerAdapter>,
    TempDir,
) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("audit.db");
    let private_root = if unavailable_private_memory {
        PathBuf::from("halt-16-5-unavailable-relative-private-root")
    } else {
        tmp.path().join("memory")
    };

    let private = Arc::new(PrivateMemoryStore::new(private_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal,
        Arc::clone(&tl),
    ));

    let halt_registry = Arc::new(HaltRegistry::new());
    let output_markers = Arc::new(OutputMarkerRegistry::new());
    let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));

    // Minimal capability registry + orchestrator for the resolver.
    let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _audit_rx) = cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let working_memory = Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new());
    let telemetry_stream = Arc::new(TelemetryStreamAdapter::default());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        BOOT_NONCE,
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

    let resolver = Arc::new(KernelHaltResolver::new(
        Arc::clone(&halt_registry),
        Arc::clone(&tl),
        output_markers,
        mailbox,
        BOOT_NONCE,
        Arc::clone(&memory),
        orchestrator,
    ));

    (resolver, halt_registry, tl, memory, tmp)
}

/// Audit-journal double that always fails — stands in for a fatal SQLite
/// outage at the Approval Decision Log. `HaltJournal` has exactly one
/// required method; nothing to stub beyond it.
struct FailingJournal;

impl HaltJournal for FailingJournal {
    fn journal_halt_resolution(
        &self,
        _actor: &str,
        _spirit_id: &str,
        _halt_id: &HaltId,
        _resolution: &Resolution,
    ) -> Result<(), HaltJournalError> {
        Err(HaltJournalError::WriteFailed(
            "simulated fatal approval-log write failure".to_string(),
        ))
    }
}

#[test]
fn failed_context_write_leaves_halt_retryable() {
    let (resolver, registry, _tl, _memory, _tmp) = make_resolver(true);

    let hid = HaltId::new("halt-16-5-failed-write").unwrap();
    registry
        .insert_pending_with_metadata(
            hid.clone(),
            HaltState::PendingResolution,
            pending_metadata(hid.as_str()),
        )
        .unwrap();
    assert_eq!(
        registry.lookup_state(&hid),
        Some(HaltState::PendingResolution)
    );

    let err = resolver
        .resolve(
            &hid,
            Resolution::ProvidedContext {
                text: CONTEXT_TEXT.to_string(),
            },
        )
        .unwrap_err();
    match &err {
        ResolveError::Internal(msg) => assert!(
            msg.starts_with("memory.write failed"),
            "expected the context write to fail, got: {msg}"
        ),
        other => panic!("expected ResolveError::Internal, got {other:?}"),
    }

    // Durability claim: the failed write preceded the state transition, so
    // the halt is STILL pending and its metadata is intact — a retry can
    // re-run the exact same resolution path.
    assert_eq!(
        registry.lookup_state(&hid),
        Some(HaltState::PendingResolution),
        "failed context write must not move the halt out of PendingResolution"
    );
    let metadata = registry
        .lookup_pending_metadata(&hid)
        .expect("metadata must survive a failed side effect for retry");
    assert_eq!(metadata.spirit_pid, SPIRIT_PID);
    assert_eq!(registry.pending_halt_ids(), vec![hid.clone()]);
}

#[test]
fn journal_failure_rolls_back_and_retry_succeeds() {
    let (resolver, registry, tl, memory, _tmp) = make_resolver(false);
    let dispatcher = Arc::new(NotificationDispatcher::new());

    let hid = HaltId::new("halt-16-5-journal-rollback").unwrap();
    registry
        .insert_pending_with_metadata(
            hid.clone(),
            HaltState::PendingResolution,
            pending_metadata(hid.as_str()),
        )
        .unwrap();

    // Capture BEFORE the resolve: the terminal transition removes the
    // registry's copy, so the rollback needs this one to restore it.
    let metadata_before = registry.lookup_pending_metadata(&hid).unwrap();
    let resolution = Resolution::ProvidedContext {
        text: CONTEXT_TEXT.to_string(),
    };

    // Operator-door attempt #1: the resolver succeeds (registry → Resumed)
    // but the approval-journal commit fails; submit_resolution surfaces it.
    let failing_flow = HaltFlow::new(
        Arc::clone(&resolver),
        Arc::clone(&dispatcher),
        Arc::new(FailingJournal),
    );
    let err = failing_flow
        .submit_resolution(hid.clone(), resolution.clone(), SPIRIT_ID)
        .unwrap_err();
    assert!(
        matches!(err, HaltUiError::Audit(HaltJournalError::WriteFailed(_))),
        "expected HaltUiError::Audit, got {err:?}"
    );

    // The unsanctioned window the door must compensate: registry terminal,
    // nothing journaled.
    assert_eq!(registry.lookup_state(&hid), Some(HaltState::Resumed));
    assert!(
        tl.query_approvals(None).unwrap().is_empty(),
        "failed journal commit must not leave an approval row"
    );

    // Compensation: roll the resolution back and restore the metadata.
    assert!(registry.rollback_resolution(&hid, Some(metadata_before.clone())));
    assert_eq!(
        registry.lookup_state(&hid),
        Some(HaltState::PendingResolution)
    );
    assert_eq!(
        registry.lookup_pending_metadata(&hid),
        Some(metadata_before),
        "rollback must restore the metadata the retry's ProvidedContext arm needs"
    );

    // Retry through the healthy in-memory journal.
    let good_journal: Arc<dyn HaltJournal> = Arc::clone(&tl) as Arc<dyn HaltJournal>;
    let good_flow = HaltFlow::new(Arc::clone(&resolver), dispatcher, good_journal);
    good_flow
        .submit_resolution(hid.clone(), resolution, SPIRIT_ID)
        .unwrap();

    // Terminal exactly once, approval row landed exactly once, context write
    // visible in the Spirit's private tier.
    assert_eq!(registry.lookup_state(&hid), Some(HaltState::Resumed));
    assert!(registry.pending_halt_ids().is_empty());
    let approvals = tl.query_approvals(None).unwrap();
    assert_eq!(
        approvals.len(),
        1,
        "exactly one journaled approval decision"
    );
    assert_eq!(approvals[0].intent, "provided_context");
    assert_eq!(approvals[0].target, SPIRIT_ID);
    let reasoning = approvals[0].reasoning.as_deref().unwrap();
    assert!(reasoning.contains(hid.as_str()));
    assert!(reasoning.contains(CONTEXT_TEXT));

    let stored = memory
        .read(
            SPIRIT_PID,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            &format!("halt_context::{}", hid.as_str()),
        )
        .unwrap();
    match stored {
        Some(MemoryValue::Text(text)) => assert_eq!(text, CONTEXT_TEXT),
        other => panic!("expected supplied context in private tier, got {other:?}"),
    }
}
