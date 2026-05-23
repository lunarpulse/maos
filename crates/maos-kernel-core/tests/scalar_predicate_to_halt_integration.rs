#![forbid(unsafe_code)]

//! AC3 integration test — wires `set_scalar` → policy evaluator →
//! `invoke_halt` end-to-end against in-memory TransparencyLog and a
//! tmpdir-backed JournalAdapter.
//!
//! Mirror of `halt_invoke_test.rs` fixture pattern (in-memory TL,
//! tmpdir journal, real HaltRegistry).

use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::Arc;

use maos_domain::halt::HaltState;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::working_memory::policy_runtime::{
    evaluate_after_set_scalar, PolicyEvaluationOutcome,
};
use maos_kernel_core::capability::{
    cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, CapabilityRegistryPort, WorkingMemoryStore,
};
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::{
    EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection, ScalarPredicate,
};

fn make_adapter() -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _) = maos_kernel_core::capability::cap_audit::channel();
    let quota = CapQuotaTracker::new();
    let working_memory = Arc::new(WorkingMemoryStore::new());
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xCAFE,
        policy,
        audit_tx,
        quota,
        working_memory,
        telemetry,
    )
}

fn make_policy() -> EpistemicPolicySection {
    EpistemicPolicySection {
        rules: vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )],
        default_action: EpistemicAction::VerbalizeOnly,
    }
}

#[test]
fn set_scalar_to_halt_end_to_end() {
    let adapter = make_adapter();
    let policy = make_policy();

    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xCAFE));
    let journal_dir = tempfile::tempdir().unwrap();
    let journal_path = journal_dir.path().join("journal.sqlite");
    let journal = JournalAdapter::open(&journal_path).unwrap();
    let registry = HaltRegistry::new();

    let boot_nonce = 0xDEAD_BEEF;
    let spirit_id = "spirit-1";
    let spirit_pid = 1u32;

    // Step 1: set_scalar
    let event = adapter
        .set_scalar(spirit_pid, spirit_id, "uncertainty", 0.85, "frame-001")
        .unwrap();
    assert_eq!(event.tag, "uncertainty");
    assert_eq!(event.value, 0.85);

    // Step 2: evaluate policy
    let outcome = evaluate_after_set_scalar(
        spirit_id,
        spirit_pid,
        boot_nonce,
        "uncertainty",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();

    let payload = match outcome {
        Some(PolicyEvaluationOutcome::Halt(p)) => p,
        other => panic!("expected Halt outcome, got: {other:?}"),
    };

    // Step 3: invoke_halt
    let receipt = invoke_halt(
        &tl, &journal, &registry, payload, spirit_pid, spirit_id, boot_nonce,
    )
    .unwrap();

    assert!(!receipt.halt_id.as_str().is_empty());
    assert_eq!(receipt.spirit_pid, spirit_pid);
    assert_eq!(receipt.boot_nonce, boot_nonce);
    assert_ne!(receipt.frame_id, [0u8; 16]);

    // Verify Transparency Log has the EpistemicHalt row with correct tag
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            limit: Some(1),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(entries.len(), 1, "expected 1 EpistemicHalt TL row");
    assert_eq!(entries[0].kind, FrameKind::EpistemicHalt);

    // Verify registry has the halt in PendingResolution
    let pending = registry.pending_halt_ids();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].as_str(), receipt.halt_id.as_str());

    // Verify Lifecycle Journal has the Halt entry
    assert_eq!(
        journal.last_event("spirit-1"),
        Some(LifecycleEvent::Halt),
        "expected LifecycleEvent::Halt in journal"
    );
}

#[test]
fn set_scalar_no_halt_when_predicate_does_not_fire() {
    let adapter = make_adapter();
    let policy = make_policy();

    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.5,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();

    assert!(
        outcome.is_none(),
        "expected no halt when value is below threshold"
    );
}

#[test]
fn set_scalar_flag_action_does_not_halt() {
    let flag_policy = EpistemicPolicySection {
        rules: vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Flag,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.5 }),
        )],
        default_action: EpistemicAction::VerbalizeOnly,
    };

    let adapter = make_adapter();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xCAFE));
    let journal_dir = tempfile::tempdir().unwrap();
    let journal_path = journal_dir.path().join("journal.sqlite");
    let journal = JournalAdapter::open(&journal_path).unwrap();
    let registry = HaltRegistry::new();

    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.9,
        "frame-001",
        &flag_policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();

    assert!(
        matches!(outcome, Some(PolicyEvaluationOutcome::Flag(_))),
        "expected Flag outcome"
    );

    // Negative side-effect assertions: Flag must NOT produce TL rows, journal entries, or registry state
    let tl_entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        tl_entries.len(),
        0,
        "Flag action must not write to Transparency Log"
    );
    assert_eq!(
        registry.pending_halt_ids().len(),
        0,
        "Flag action must not insert into HaltRegistry"
    );
    assert_ne!(
        journal.last_event("spirit-1"),
        Some(LifecycleEvent::Halt),
        "Flag action must not write Halt to journal"
    );
}

#[test]
fn set_scalar_verbalize_only_action_does_not_halt() {
    let verbalize_policy = EpistemicPolicySection {
        rules: vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::VerbalizeOnly,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.5 }),
        )],
        default_action: EpistemicAction::VerbalizeOnly,
    };

    let adapter = make_adapter();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xCAFE));
    let journal_dir = tempfile::tempdir().unwrap();
    let journal_path = journal_dir.path().join("journal.sqlite");
    let journal = JournalAdapter::open(&journal_path).unwrap();
    let registry = HaltRegistry::new();

    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "uncertainty",
        0.9,
        "frame-001",
        &verbalize_policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();

    assert!(
        matches!(outcome, Some(PolicyEvaluationOutcome::VerbalizeOnly)),
        "expected VerbalizeOnly outcome"
    );

    // Negative side-effect assertions
    let tl_entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        tl_entries.len(),
        0,
        "VerbalizeOnly action must not write to Transparency Log"
    );
    assert_eq!(
        registry.pending_halt_ids().len(),
        0,
        "VerbalizeOnly action must not insert into HaltRegistry"
    );
    assert_ne!(
        journal.last_event("spirit-1"),
        Some(LifecycleEvent::Halt),
        "VerbalizeOnly action must not write Halt to journal"
    );
}

#[test]
fn set_scalar_non_matching_tag_no_halt() {
    let adapter = make_adapter();
    let policy = make_policy();

    let outcome = evaluate_after_set_scalar(
        "spirit-1",
        1,
        0xCAFE,
        "different_tag",
        0.85,
        "frame-001",
        &policy,
        &adapter as &dyn CapabilityRegistryPort,
    )
    .unwrap();

    assert!(outcome.is_none());
}
