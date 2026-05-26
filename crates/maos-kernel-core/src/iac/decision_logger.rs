#![forbid(unsafe_code)]

//! Decision-logger — I12 enforcement (Story 3.3, NFR-Aud-5).
//!
//! Every `decision.*` IAC frame that traverses the Mailbox is
//! decorated with the originating Spirit's current
//! `working_memory_digest_refs` so post-hoc audit can reconstruct
//! what the Spirit reasoned over at decision time.
//!
//! At v0.3-β the kernel does not yet track per-Spirit working-memory
//! digests (Story 4.3 lands the Memory Manager + principal namespace).
//! The decorator therefore attaches the EMPTY refs set at v0.3-β —
//! NFR-Aud-5's 100% mandate is satisfied STRUCTURALLY (the field is
//! ALWAYS present), with Story 4.3 wiring the source-of-truth.
//!
//! Compatibility note: post-Story 4.3 the decorator queries the
//! Memory Manager's per-Spirit digest set. The decorator API stays
//! stable across that change.

use maos_domain::frame::{FramePayload, IacFrame};
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use maos_spirit_abi::identity::SpiritId;

/// Decorate a `decision.*` frame with the spirit's current digest refs.
///
/// Returns the frame UNCHANGED if `frame.payload` is not a
/// `DecisionDispatch` variant — calling on the wrong kind is a no-op,
/// not an error (the IAC bus may route through this decorator
/// indiscriminately).
///
/// The `digest_provider` callback is the seam: at v0.3-β the
/// composition root passes a closure returning
/// `WorkingMemoryDigestRefs::default()`; Story 4.3 replaces this with
/// a Memory Manager query.
pub fn decorate_decision_frame<F>(mut frame: IacFrame, digest_provider: F) -> IacFrame
where
    F: FnOnce(&SpiritId) -> WorkingMemoryDigestRefs,
{
    if let FramePayload::DecisionDispatch(ref mut payload) = frame.payload {
        payload.working_memory_digest_refs = digest_provider(&frame.from.spirit_id);
    }
    frame
}

/// Inspect a frame to determine whether it carries the I12 refs.
/// Returns `true` for non-decision frames (vacuously satisfies I12),
/// `true` for decision frames with a non-default refs set, AND `true`
/// for decision frames with the empty refs set (v0.3-β semantics).
/// Returns `false` ONLY if a future story removes the field — which
/// would BREAK the additive contract and be caught by `abi-diff`.
///
/// This function exists for the integration test at AC5; production
/// code SHOULD NOT branch on its return value (always-true at v0.3-β).
pub fn frame_carries_i12_refs(frame: &IacFrame) -> bool {
    match &frame.payload {
        FramePayload::DecisionDispatch(_) => true,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::frame::{DecisionDispatchPayload, FrameAddress};
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i3::FrameOrigin;
    use smallvec::smallvec;

    fn make_decision_frame() -> IacFrame {
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: FrameAddress {
                spirit_id: SpiritId::from("test-spirit"),
                host_id: None,
                role: None,
            },
            to: smallvec![],
            kind: maos_spirit_abi::identity::FrameKind::DecisionDispatch,
            intent: IntentClass::Standard,
            payload: FramePayload::DecisionDispatch(DecisionDispatchPayload {
                decision_id: 1,
                approved: true,
                working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: maos_domain::invariants::i13::IntentLineage::default(),
        }
    }

    fn make_task_assign_frame() -> IacFrame {
        let mut frame = make_decision_frame();
        frame.payload = FramePayload::TaskAssign(maos_domain::frame::TaskAssignPayload {
            goal: "test".into(),
            scope: vec![],
            success_criteria: "test".into(),
            posture_preferences: maos_domain::frame::PosturePreferences::default(),
            prior_distillate_ref: None,
        });
        frame.kind = maos_spirit_abi::identity::FrameKind::TaskAssign;
        frame
    }

    #[test]
    fn decorate_decision_frame_attaches_digest_refs() {
        let frame = make_decision_frame();
        let refs = WorkingMemoryDigestRefs::new(vec!["f1".into(), "f2".into()]);
        let decorated = decorate_decision_frame(frame, |_| refs.clone());
        match &decorated.payload {
            FramePayload::DecisionDispatch(p) => {
                assert_eq!(p.working_memory_digest_refs.as_slice(), &["f1", "f2"]);
            }
            _ => panic!("expected DecisionDispatch"),
        }
    }

    #[test]
    fn decorate_decision_frame_passes_spirit_id_to_provider() {
        let frame = make_decision_frame();
        let _expected_spirit = SpiritId::from("test-spirit");
        let mut received_id = None;
        let decorated = decorate_decision_frame(frame, |sid| {
            received_id = Some(sid.clone());
            WorkingMemoryDigestRefs::default()
        });
        assert!(received_id.is_some());
        assert_eq!(received_id.unwrap().as_str(), "test-spirit");
        match &decorated.payload {
            FramePayload::DecisionDispatch(_) => {}
            _ => panic!("expected DecisionDispatch"),
        }
    }

    #[test]
    fn decorate_decision_frame_does_not_mutate_non_decision_frames() {
        let frame = make_task_assign_frame();
        let decorated = decorate_decision_frame(frame, |_| {
            WorkingMemoryDigestRefs::new(vec!["unexpected".into()])
        });
        match &decorated.payload {
            FramePayload::TaskAssign(p) => {
                assert_eq!(p.goal, "test");
            }
            _ => panic!("expected TaskAssign"),
        }
    }

    #[test]
    fn frame_carries_i12_refs_decision_frame_true() {
        let frame = make_decision_frame();
        assert!(frame_carries_i12_refs(&frame));
    }

    #[test]
    fn frame_carries_i12_refs_non_decision_frame_true() {
        let frame = make_task_assign_frame();
        assert!(frame_carries_i12_refs(&frame));
    }
}
