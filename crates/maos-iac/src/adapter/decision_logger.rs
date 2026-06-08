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

use std::sync::Arc;

use maos_domain::frame::{FramePayload, IacFrame};
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use maos_domain::memory::{MemoryNamespace, MemoryTier};
use maos_domain::ports::MemoryManagerPort;
use maos_spirit_abi::identity::SpiritId;

/// Working-memory key prefix under which a Spirit records the digest refs it is
/// reasoning over at decision time (Story 8.10 AC3). The
/// [`memory_backed_digest_provider`] reads these back so a `decision.*` frame
/// carries what the Spirit actually had in context.
pub const WORKING_MEMORY_DIGEST_KEY_PREFIX: &str = "digest:";

/// Upper bound on digest refs read per decision frame (defensive cap).
const MAX_DIGEST_REFS_SCAN: usize = 256;

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

/// Inspect a frame to determine whether it carries **real** I12 refs.
///
/// Story 8.10 AC3: this is no longer tautological. A `DecisionDispatch` frame
/// carries I12 content iff its `working_memory_digest_refs` is **non-empty**
/// (a decision frame that recorded nothing about what it reasoned over does
/// NOT satisfy I12). Non-decision frames vacuously satisfy I12 (the invariant
/// only applies to `decision.*` frames).
pub fn frame_carries_i12_refs(frame: &IacFrame) -> bool {
    match &frame.payload {
        FramePayload::DecisionDispatch(p) => !p.working_memory_digest_refs.as_slice().is_empty(),
        _ => true,
    }
}

/// Build the **real** I12 digest provider backed by the Memory Manager
/// (Story 8.10 AC3a). Replaces the composition root's default empty-refs
/// closure: given a citing Spirit, it resolves the Spirit's pid and returns the
/// `digest:` refs currently in its private working memory — i.e. the frames it
/// reasoned over at decision time (FR / NFR-Aud-5 source-of-truth, landed by
/// Story 4.3's `MemoryManagerPort`).
///
/// The `resolve_pid` closure maps a `SpiritId` to the kernel-set `spirit_pid`
/// (the composition root holds the registry); an unresolved Spirit yields the
/// default empty refs (structurally still I12-present, just empty).
pub fn memory_backed_digest_provider<R>(
    memory: Arc<dyn MemoryManagerPort + Send + Sync>,
    resolve_pid: R,
) -> impl Fn(&SpiritId) -> WorkingMemoryDigestRefs + Send + Sync + 'static
where
    R: Fn(&SpiritId) -> Option<u32> + Send + Sync + 'static,
{
    move |sid| {
        let Some(pid) = resolve_pid(sid) else {
            return WorkingMemoryDigestRefs::default();
        };
        match memory.scan(
            pid,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            WORKING_MEMORY_DIGEST_KEY_PREFIX,
            MAX_DIGEST_REFS_SCAN,
        ) {
            Ok(entries) => {
                let mut refs: Vec<String> = entries
                    .into_iter()
                    .filter_map(|e| {
                        let stripped = e
                            .key
                            .strip_prefix(WORKING_MEMORY_DIGEST_KEY_PREFIX)
                            .unwrap_or(&e.key);
                        if stripped.is_empty() {
                            None // exact-prefix key "digest:" → meaningless ref
                        } else {
                            Some(stripped.to_string())
                        }
                    })
                    .collect();
                // Deterministic order (scan order is not guaranteed stable).
                refs.sort();
                WorkingMemoryDigestRefs::new(refs)
            }
            // A memory read failure must not poison the frame — fall back to the
            // structurally-present empty set (the field is still present per
            // NFR-Aud-5; `frame_carries_i12_refs` will report it empty).
            Err(_) => WorkingMemoryDigestRefs::default(),
        }
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
    fn frame_carries_i12_refs_empty_decision_frame_false() {
        // Story 8.10 AC3 — a DecisionDispatch with EMPTY refs does NOT carry
        // I12 content (the de-tautologized assertion).
        let frame = make_decision_frame(); // built with empty refs
        assert!(
            !frame_carries_i12_refs(&frame),
            "empty-refs decision frame must NOT satisfy I12"
        );
    }

    #[test]
    fn frame_carries_i12_refs_nonempty_decision_frame_true() {
        // Story 8.10 AC3 — a DecisionDispatch with real refs carries I12.
        let frame = make_decision_frame();
        let decorated =
            decorate_decision_frame(frame, |_| WorkingMemoryDigestRefs::new(vec!["d1".into()]));
        assert!(frame_carries_i12_refs(&decorated));
    }

    #[test]
    fn frame_carries_i12_refs_non_decision_frame_true() {
        let frame = make_task_assign_frame();
        assert!(frame_carries_i12_refs(&frame));
    }
}
