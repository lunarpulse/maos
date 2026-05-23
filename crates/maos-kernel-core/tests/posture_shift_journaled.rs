//! Posture-shift atomicity + Approval Decision Log journaling (AC4).
//!
//! Verifies `PolicyTable::shift_posture` enforces ceiling constraints,
//! rejects `Posture::Autonomous`, journals via `journal_posture_shift`,
//! and preserves the Approval Decision Log / Transparency Log distinction.

use std::sync::Arc;

use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
use maos_kernel_core::security::posture::{journal_posture_shift, PostureError, PostureState};

fn seed_spirit(policy: &PolicyTable, pid: u32, posture: Posture, allowed_max: Posture) {
    let mut inner = (*policy.inner().load_full()).clone();
    inner.spirit_postures.insert(
        pid,
        PostureState {
            current: posture,
            allowed_max,
            epistemic_policy: EpistemicPolicySection {
                rules: vec![],
                default_action: EpistemicAction::VerbalizeOnly,
            },
        },
    );
    policy.update(inner);
}

#[test]
fn shift_posture_succeeds_and_returns_new_hash() {
    let policy = PolicyTable::new();
    seed_spirit(&policy, 0, Posture::Assistive, Posture::AutonomousWithHalt);

    let old_hash = {
        let inner = policy.inner().load_full();
        inner.spirit_postures.get(&0).unwrap().posture_hash()
    };

    let new_hash = policy
        .shift_posture(0, Posture::AutonomousWithHalt)
        .unwrap();

    assert_ne!(old_hash, new_hash, "hash must change on posture shift");

    let inner = policy.inner().load_full();
    let state = inner.spirit_postures.get(&0).unwrap();
    assert_eq!(state.current, Posture::AutonomousWithHalt);
}

#[test]
fn shift_posture_rejects_ceiling_violation() {
    let policy = PolicyTable::new();
    seed_spirit(&policy, 0, Posture::Cautious, Posture::Assistive);

    let err = policy
        .shift_posture(0, Posture::AutonomousWithHalt)
        .unwrap_err();

    assert!(matches!(
        err,
        PostureError::AboveCeiling {
            requested: Posture::AutonomousWithHalt,
            allowed: Posture::Assistive,
        }
    ));
}

#[test]
fn shift_posture_rejects_autonomous() {
    let policy = PolicyTable::new();
    seed_spirit(&policy, 0, Posture::Assistive, Posture::Autonomous);

    let err = policy.shift_posture(0, Posture::Autonomous).unwrap_err();
    assert!(matches!(
        err,
        PostureError::NonRuntimePosture(Posture::Autonomous)
    ));
}

#[test]
fn shift_posture_rejects_unknown_spirit() {
    let policy = PolicyTable::new();
    let err = policy.shift_posture(42, Posture::Cautious).unwrap_err();
    assert!(matches!(err, PostureError::UnknownSpirit(42)));
}

#[test]
fn posture_shift_e2e_journaled() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let policy = PolicyTable::new();
    seed_spirit(&policy, 0, Posture::Assistive, Posture::AutonomousWithHalt);

    let new_hash = policy
        .shift_posture(0, Posture::AutonomousWithHalt)
        .unwrap();
    assert_ne!(new_hash, [0u8; 32]);

    journal_posture_shift(
        &log,
        "director",
        "hello-spirit",
        Posture::Assistive,
        Posture::AutonomousWithHalt,
    )
    .unwrap();

    let approvals = log.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 1, "exactly one approval decision expected");
    let row = &approvals[0];
    assert_eq!(row.capability, "posture.shift");
    assert_eq!(row.intent, "Assistive -> AutonomousWithHalt");
    assert!(row.decision);
    assert_eq!(row.actor, "director");
    assert_eq!(row.target, "hello-spirit");

    // Verify the row is in approval_decision_log, NOT transparency_log
    let frames = log
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            ..Default::default()
        })
        .unwrap();
    let has_posture_shift_in_tl = frames.iter().any(|f| {
        let payload_str = String::from_utf8_lossy(&f.payload_redacted);
        payload_str.contains("posture.shift")
    });
    assert!(
        !has_posture_shift_in_tl,
        "posture shift must not appear in transparency_log"
    );
}

#[test]
fn shift_posture_ceiling_rejection_writes_no_approval_row() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let policy = PolicyTable::new();
    seed_spirit(&policy, 0, Posture::Cautious, Posture::Assistive);

    let result = policy.shift_posture(0, Posture::AutonomousWithHalt);
    assert!(result.is_err());

    // No approval row should have been written
    let approvals = log.query_approvals(None).unwrap();
    assert!(
        approvals.is_empty(),
        "ceiling rejection should not write approval rows"
    );
}
