#![forbid(unsafe_code)]

//! Per-frame-kind channel-class router per architecture §7.1.1.
//!
//! Architecture §7.1.1 declares the normative channel-class table. This
//! module provides the const-table lookup that `Mailbox::deliver` uses
//! to select the right channel for each frame.
//!
//! Uses the canonical `FrameKind` from `maos-spirit-abi::identity` (the
//! ABI-wire-stable source of truth). Kernel-internal frame kinds (7/8/9)
//! return `None` — they do NOT flow through the IAC router.

use maos_spirit_abi::identity::FrameKind;

/// Channel class — dictates how frames of a given kind are routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelClass {
    Mpsc,
    Broadcast,
}

/// The §7.1.1 normative channel-class table.
pub const CHANNEL_CLASSES: &[(FrameKind, ChannelClass, usize)] = &[
    (FrameKind::TaskAssign, ChannelClass::Mpsc, 64),
    (FrameKind::TaskComplete, ChannelClass::Mpsc, 64),
    (FrameKind::DecisionDispatch, ChannelClass::Mpsc, 128),
    (FrameKind::EpistemicHalt, ChannelClass::Mpsc, 16),
    (FrameKind::TelemetryEvent, ChannelClass::Broadcast, 256),
    (FrameKind::ConsentRequest, ChannelClass::Mpsc, 32),
    (FrameKind::Retract, ChannelClass::Mpsc, 32),
    // Story 6.4 — ADR-034 binding-v0.9: partial-consent failure event.
    // §7.1.1 cardinality matches `consent.request` (1:1 sender ← bus).
    (FrameKind::ConsentRupture, ChannelClass::Mpsc, 32),
    // Story 6.4 — NFR-Scale-4: per-(provider, credential) rate-limit event.
    // §7.1.1 cardinality matches `consent.request` (1:1 sender ← bus).
    (FrameKind::RateLimited, ChannelClass::Mpsc, 32),
];

/// Look up the channel class and capacity floor for a given frame kind.
pub fn channel_class_for(kind: FrameKind) -> Option<(ChannelClass, usize)> {
    for &(k, class, capacity) in CHANNEL_CLASSES {
        if k == kind {
            return Some((class, capacity));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use maos_spirit_abi::identity::FrameKind;

    use super::*;

    /// §7.1.1 contract gate: every row in CHANNEL_CLASSES matches the
    /// architecture doc's normative table verbatim. If the doc changes,
    /// this test changes; if code drifts from doc without test change,
    /// CI catches it. Architecture file:
    /// `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md`
    /// §7.1.1.
    #[test]
    fn channel_classes_match_addendum() {
        // Encode the §7.1.1 table inline as the source of truth.
        let expected: &[(FrameKind, ChannelClass, usize)] = &[
            (FrameKind::TaskAssign, ChannelClass::Mpsc, 64),
            (FrameKind::TaskComplete, ChannelClass::Mpsc, 64),
            (FrameKind::DecisionDispatch, ChannelClass::Mpsc, 128),
            (FrameKind::EpistemicHalt, ChannelClass::Mpsc, 16),
            (FrameKind::TelemetryEvent, ChannelClass::Broadcast, 256),
            (FrameKind::ConsentRequest, ChannelClass::Mpsc, 32),
            (FrameKind::Retract, ChannelClass::Mpsc, 32),
            (FrameKind::ConsentRupture, ChannelClass::Mpsc, 32),
            (FrameKind::RateLimited, ChannelClass::Mpsc, 32),
        ];

        assert_eq!(
            CHANNEL_CLASSES.len(),
            expected.len(),
            "CHANNEL_CLASSES length mismatch with §7.1.1 table"
        );

        for (i, (kind, class, capacity)) in CHANNEL_CLASSES.iter().enumerate() {
            let (exp_kind, exp_class, exp_capacity) = &expected[i];
            assert_eq!(
                kind, exp_kind,
                "CHANNEL_CLASSES[{i}]: kind mismatch (code={kind:?}, spec={exp_kind:?})"
            );
            assert_eq!(
                class, exp_class,
                "CHANNEL_CLASSES[{i}]: class mismatch for {kind:?}"
            );
            assert_eq!(
                capacity, exp_capacity,
                "CHANNEL_CLASSES[{i}]: capacity mismatch for {kind:?} (code={capacity}, spec={exp_capacity})"
            );
        }

        // Verify the lookup function returns the correct values
        for (kind, class, capacity) in expected {
            let (found_class, found_cap) =
                channel_class_for(*kind).expect("IAC frame kind should be routable");
            assert_eq!(
                found_class, *class,
                "channel_class_for({kind:?}) class mismatch"
            );
            assert_eq!(
                found_cap, *capacity,
                "channel_class_for({kind:?}) capacity mismatch"
            );
        }
    }

    /// Kernel-internal audit kinds (7/8/9) MUST NOT flow through the IAC
    /// router. They continue to write directly to the Transparency Log
    /// via the cap-audit path.
    #[test]
    fn audit_frame_kinds_reject_router() {
        assert!(
            channel_class_for(FrameKind::CapabilityInvocation).is_none(),
            "CapabilityInvocation MUST NOT be routable via IAC router"
        );
        assert!(
            channel_class_for(FrameKind::SandboxBlock).is_none(),
            "SandboxBlock MUST NOT be routable via IAC router"
        );
        assert!(
            channel_class_for(FrameKind::InferenceCall).is_none(),
            "InferenceCall MUST NOT be routable via IAC router"
        );
    }

    #[test]
    fn all_iac_frame_kinds_are_routable() {
        for kind in &[
            FrameKind::TaskAssign,
            FrameKind::TaskComplete,
            FrameKind::DecisionDispatch,
            FrameKind::EpistemicHalt,
            FrameKind::TelemetryEvent,
            FrameKind::ConsentRequest,
            FrameKind::Retract,
            FrameKind::ConsentRupture,
            FrameKind::RateLimited,
        ] {
            assert!(
                channel_class_for(*kind).is_some(),
                "IAC frame kind {kind:?} MUST be routable"
            );
        }
    }
}
