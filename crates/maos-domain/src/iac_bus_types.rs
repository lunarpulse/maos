//! Shared types for the IAC Bus port trait.
//!
//! These types are referenced by `IacBusPort` methods and live in
//! `maos-domain` so the port trait can reference them without a
//! dependency on `maos-kernel-core`.

use maos_spirit_abi::identity::FrameKind;

use crate::frame::RetractPayloadError;
use crate::invariants::i3::FrameOrigin;

/// Typed error for IAC bus operations.
#[derive(Debug, thiserror::Error)]
pub enum IacBusError {
    #[error("spirit {0} is not registered — call register_spirit first")]
    UnknownSpirit(String),
    #[error(
        "epistemic halt queue overflow for spirit {0} — kernel MUST raise watchdog (Story 3.3)"
    )]
    HaltQueueOverflow(String),
    #[error("channel closed for spirit {0} kind {1:?}")]
    ChannelClosed(String, FrameKind),
    #[error("frame serialization failed: {0}")]
    SerializationFailed(String),
    /// Story 6.3 — cross-host routing requested but no A2A peer configured
    /// for the named `host_id`. Replaces the v0.3-β `CrossHostUnsupported`
    /// blanket reject — the kernel-core mailbox now routes through the
    /// composition-root-installed `A2ARouter` when one is present, and only
    /// fires this variant when the operator has not declared a peer.
    #[error("cross-host routing requires an A2A peer configured for host_id {host_id}")]
    CrossHostNotConfigured { host_id: String },
    /// Story 6.3 — ADR-012 typed-intent consent denied by the A2A router
    /// (send-side or accept-side). The direction distinguishes SENDER-side
    /// outbound rejection from RECEIVER-side intake rejection.
    #[error("cross-host intent denied ({direction:?}): {intent} for peer {peer}")]
    CrossHostIntentDenied {
        peer: String,
        intent: String,
        direction: CrossHostIntentDirection,
    },
    /// Story 6.3 — TOFU pin mismatch or not-pinned on cross-host delivery.
    #[error("cross-host pin mismatch for peer {peer}: {detail}")]
    CrossHostPinMismatch { peer: String, detail: String },
    /// Story 6.3 — consent envelope expired on cross-host delivery.
    #[error("cross-host consent expired for peer {peer} at {expired_at_ns} (now {now_ns})")]
    CrossHostConsentExpired {
        peer: String,
        expired_at_ns: u64,
        now_ns: u64,
    },
    /// Story 6.3 — outbound timed out awaiting receiver ACK — partition
    /// behavior per architecture §7.2.
    #[error("cross-host partition timeout for peer {peer} after {timeout_secs}s (frame {frame_id:?})")]
    CrossHostPartitionTimeout {
        peer: String,
        frame_id: [u8; 16],
        timeout_secs: u64,
    },
    /// Story 6.3 — cross-host transport failure (serialization / I/O / framing).
    #[error("cross-host transport failure for peer {peer}: {detail}")]
    CrossHostTransportFailure { peer: String, detail: String },
    /// Story 6.3 — the A2A router returned a failure when routing a
    /// cross-host frame (intent denied / TOFU mismatch / partition / etc.).
    /// String-bearing per ADR-010 hexagonal layering (maos-domain MUST NOT
    /// depend on maos-a2a; the maos-a2a adapter formats `A2AError` into the
    /// string carrier when constructing this variant).
    /// DEPRECATED — use the typed sub-variants above instead. This variant
    /// is retained for backward compatibility with existing test stubs.
    #[error("cross-host A2A route failed: {0}")]
    CrossHostRouteFailure(String),
    #[error("channel full for spirit {0} kind {1:?} — backpressure")]
    QueueFull(String, FrameKind),
    #[error("spirit {0} is already registered — deregister before re-registering")]
    AlreadyRegistered(String),
    /// Story 4.5 — NFR-Aud-14: cross-Spirit frame arrived with no
    /// intent_lineage AND non-human origin. The kernel auto-computes
    /// lineage for `FrameOrigin::HumanAuthored` originating frames
    /// (single-class lineage from `frame.intent`), so this variant
    /// fires for Spirit-emitted cross-Spirit frames missing lineage —
    /// the structural sign of consent-laundering through re-emission.
    #[error("intent_lineage chain broken on cross-Spirit frame from {from} to {to}: empty lineage on non-human origin {origin:?}")]
    EIntentLineageBroken {
        from: String,
        to: String,
        origin: FrameOrigin,
    },
    /// Story 6.1 — retract authority violation: only the original sender
    /// can retract their own frame in v0.5-α.
    #[error("retract authority violation: spirit {caller} cannot retract frame from spirit {original_sender}")]
    RetractAuthorityViolation { caller: String, original_sender: String },
    /// Story 6.1 — retract payload validation failed.
    #[error("retract payload validation failed: {0}")]
    RetractPayloadInvalid(#[from] RetractPayloadError),
    /// Story 6.2 — FR21: Orchestrator emitted a follow-up `task.assign` referencing
    /// raw Worker output (or no predecessor at all) when a prior Worker
    /// `TaskComplete` exists in the session's log_recall window. Closes the
    /// raw-output context-overflow loophole — the Orchestrator MUST dispatch
    /// against a `DistillationReceipt::digest_frame_id`, not against raw frame ids.
    #[error("orchestrator dispatch references raw worker output not a distillate: orchestrator {orchestrator} task {task_id}")]
    EOrchestratorDispatchRawOutput {
        orchestrator: String,
        task_id: String,
    },
}

/// Direction context for cross-host intent denial — per ADR-012 defense-in-depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossHostIntentDirection {
    Send,
    Accept,
}

/// Outcome of a `retract` operation — Story 6.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetractOutcome {
    /// Retract emitted; original frame marked retracted in TL.
    Retracted { retract_frame_id: [u8; 16] },
    /// Already retracted earlier — idempotent re-emission.
    Already { existing_retract_frame_id: [u8; 16] },
    /// Original frame_id not found in TL — return error rather than silently emit.
    OriginalNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eintent_lineage_broken_display() {
        let err = IacBusError::EIntentLineageBroken {
            from: "spirit-a".into(),
            to: "spirit-b".into(),
            origin: FrameOrigin::SpiritAuto,
        };
        let msg = format!("{err}");
        assert!(msg.contains("spirit-a"));
        assert!(msg.contains("spirit-b"));
        assert!(msg.contains("SpiritAuto"));
    }

    #[test]
    fn eintent_lineage_broken_spirit_drafted() {
        let err = IacBusError::EIntentLineageBroken {
            from: "s1".into(),
            to: "s2".into(),
            origin: FrameOrigin::SpiritDraftedHumanApproved,
        };
        let msg = format!("{err}");
        assert!(msg.contains("SpiritDraftedHumanApproved"));
    }
}
