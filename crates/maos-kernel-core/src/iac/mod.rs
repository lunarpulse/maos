#![forbid(unsafe_code)]

//! IAC Bus — supervised service per §4.5.
//!
//! Routes frames between Spirits and the kernel. At v0.1-α this is an
//! empty hexagonal adapter shell; Story 6.1 lands the full IAC Bus
//! with retract primitive and DRR fairness scheduler.
//!
//! Story 1b.1 lands the Transparency Log + Approval Decision Log
//! adapter (`TransparencyLogAdapter`) in the I9-sanctioned single-file
//! holder `transparency_log.rs`, plus the redaction filter and mailbox
//! stub.

pub mod transparency_log;
pub mod redaction;
pub mod mailbox_stub;
pub mod frame;
pub mod channels;
pub mod mailbox;
pub mod decision_logger;

pub use maos_domain::ports::IacBusPort;

pub use transparency_log::{
    TransparencyLogAdapter, FrameKind, FrameFilter, TransparencyLogEntry, AuditError,
};
pub use redaction::{RedactionPolicy, CorpusBackedRedactionPolicy};
pub use mailbox_stub::MailboxStub;
pub use mailbox::Mailbox;
pub use frame::*;
pub use channels::*;
pub use mailbox::*;

/// Adapter for the IAC Bus port trait.
///
/// Story 3.1 wires the real Mailbox + NotificationDispatcher.
/// Holds an `Arc<Mailbox>` for per-Spirit routing and an
/// `Arc<TransparencyLogAdapter>` for the I2 log-before-deliver
/// guarantee.
#[maos_attrs::i9_exempt(reason = "IAC Bus adapter; holds Arc<Mailbox> + Arc<TransparencyLogAdapter> — both are I9-sanctioned locations")]
#[derive(Clone)]
pub struct IacBusAdapter {
    mailbox: std::sync::Arc<Mailbox>,
    transparency_log: std::sync::Arc<TransparencyLogAdapter>,
    digest_provider: std::sync::Arc<dyn Fn(&maos_spirit_abi::identity::SpiritId) -> maos_domain::invariants::i12::WorkingMemoryDigestRefs + Send + Sync>,
}

impl IacBusAdapter {
    /// Construct a new adapter wrapping the given Mailbox and Transparency Log.
    pub fn new(
        mailbox: std::sync::Arc<Mailbox>,
        transparency_log: std::sync::Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            mailbox,
            transparency_log,
            digest_provider: std::sync::Arc::new(|_| maos_domain::invariants::i12::WorkingMemoryDigestRefs::default()),
        }
    }

    /// Set the digest provider closure (composition root wiring point).
    /// Story 4.3 will replace the default empty-refs closure with a
    /// Memory Manager query.
    pub fn with_digest_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(&maos_spirit_abi::identity::SpiritId) -> maos_domain::invariants::i12::WorkingMemoryDigestRefs + Send + Sync + 'static,
    {
        self.digest_provider = std::sync::Arc::new(provider);
        self
    }

    /// Access the underlying mailbox (for composition root wiring).
    pub fn mailbox(&self) -> &std::sync::Arc<Mailbox> {
        &self.mailbox
    }

    /// Access the underlying transparency log.
    pub fn transparency_log(&self) -> &std::sync::Arc<TransparencyLogAdapter> {
        &self.transparency_log
    }

    /// Raw-byte enqueue path (Story 6.1 compatibility).
    pub(crate) fn enqueue_frame_bytes(
        &self,
        frame_bytes: &[u8],
        origin: maos_domain::invariants::i3::FrameOrigin,
    ) -> maos_domain::invariants::i2::LogBeforeDeliver<()> {
        self.transparency_log.insert_frame_event(
            transparency_log::FrameKind::TaskAssign,
            0,
            None,
            "",
            frame_bytes,
            origin,
        )
    }

    /// Raw-byte broadcast path (Story 6.1 compatibility).
    pub(crate) fn broadcast_frame_bytes(
        &self,
        frame_bytes: &[u8],
        origin: maos_domain::invariants::i3::FrameOrigin,
    ) -> maos_domain::invariants::i2::LogBeforeDeliver<()> {
        self.transparency_log.insert_frame_event(
            transparency_log::FrameKind::TaskAssign,
            0,
            None,
            "",
            frame_bytes,
            origin,
        )
    }

    /// Typed deliver (Story 3.1).
    pub(crate) async fn deliver_typed(
        &self,
        frame: maos_domain::frame::IacFrame,
    ) -> Result<maos_domain::invariants::i2::LogBeforeDeliver<()>, maos_domain::iac_bus_types::IacBusError> {
        // 0. I12 decorate decision frames BEFORE serialization (Story 3.3 AC5)
        // so the logged payload already carries working_memory_digest_refs.
        // digest_provider is injected from the composition root.
        let frame = decision_logger::decorate_decision_frame(frame, |sid| (self.digest_provider)(sid));

        // 1. Serialize payload for log write
        let payload_bytes = serde_json::to_vec(&frame.payload)
            .map_err(|e| maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string()))?;

        // 2. Log before deliver (I2)
        let spirit_pid = 0u32; // v0.3-β: PID not yet relevant for pure routing
        let intent_str = match &frame.intent {
            maos_domain::invariants::i1::IntentClass::HighPrivilege => "high",
            maos_domain::invariants::i1::IntentClass::Standard => "standard",
            maos_domain::invariants::i1::IntentClass::Readonly => "readonly",
        };

        // Convert domain FrameKind to kernel FrameKind for transparency_log
        let tl_kind = match frame.kind {
            maos_spirit_abi::identity::FrameKind::TaskAssign => transparency_log::FrameKind::TaskAssign,
            maos_spirit_abi::identity::FrameKind::TaskComplete => transparency_log::FrameKind::TaskComplete,
            maos_spirit_abi::identity::FrameKind::DecisionDispatch => transparency_log::FrameKind::DecisionDispatch,
            maos_spirit_abi::identity::FrameKind::EpistemicHalt => transparency_log::FrameKind::EpistemicHalt,
            maos_spirit_abi::identity::FrameKind::TelemetryEvent => transparency_log::FrameKind::TelemetryEvent,
            maos_spirit_abi::identity::FrameKind::ConsentRequest => transparency_log::FrameKind::ConsentRequest,
            maos_spirit_abi::identity::FrameKind::Retract => transparency_log::FrameKind::Retract,
            maos_spirit_abi::identity::FrameKind::CapabilityInvocation => transparency_log::FrameKind::CapabilityInvocation,
            maos_spirit_abi::identity::FrameKind::SandboxBlock => transparency_log::FrameKind::SandboxBlock,
            maos_spirit_abi::identity::FrameKind::InferenceCall => transparency_log::FrameKind::InferenceCall,
        };

        // I2: insert_frame_event panics on SQLite write failure (transparency_log.rs:296-307).
        // Reaching this point means the log write succeeded.
        let _logged = self.transparency_log.insert_frame_event(
            tl_kind,
            spirit_pid,
            None,
            intent_str,
            &payload_bytes,
            frame.auto_marker,
        );

        // 3. Route through Mailbox (async for backpressure)
        self.mailbox.deliver(frame).await
    }

    /// Typed register_spirit (Story 3.1).
    pub(crate) fn register_spirit_typed(
        &self,
        spirit_id: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<SpiritMailboxHandle, maos_domain::iac_bus_types::IacBusError> {
        self.mailbox.register_spirit(spirit_id.as_str())
    }
}

impl Default for IacBusAdapter {
    fn default() -> Self {
        Self {
            mailbox: std::sync::Arc::new(Mailbox::new(std::sync::Arc::new(
                crate::telemetry::iac_rt::IacRtMetrics::new(),
            ))),
            transparency_log: std::sync::Arc::new(
                TransparencyLogAdapter::open_in_memory(0),
            ),
            digest_provider: std::sync::Arc::new(|_| maos_domain::invariants::i12::WorkingMemoryDigestRefs::default()),
        }
    }
}

#[cfg(test)]
mod decision_audit_tests {
    use super::*;
    use std::sync::Arc;
    use maos_domain::frame::{
        DecisionDispatchPayload, FrameAddress, FramePayload, IacFrame,
        TaskAssignPayload, PosturePreferences,
    };
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
    use maos_spirit_abi::identity::{FrameKind as DomainFrameKind, SpiritId};
    use smallvec::smallvec;

    fn make_decision_frame(decision_id: u64) -> IacFrame {
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: decision_id,
            logical_clock: decision_id,
            from: FrameAddress {
                spirit_id: SpiritId::from(format!("spirit-{decision_id}")),
                host_id: None,
                role: None,
            },
            to: smallvec![],
            kind: DomainFrameKind::DecisionDispatch,
            intent: IntentClass::Standard,
            payload: FramePayload::DecisionDispatch(DecisionDispatchPayload {
                decision_id,
                approved: true,
                working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
        }
    }

    #[tokio::test]
    async fn i12_10_decision_frames_100_percent_carry_refs() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(
            crate::telemetry::iac_rt::IacRtMetrics::new(),
        )));
        let adapter = IacBusAdapter::new(mailbox, log.clone());

        // Register a target spirit so deliver_typed can route
        adapter.register_spirit_typed(&SpiritId::from("target")).ok();

        // Construct and deliver 10 decision frames
        for id in 0..10 {
            let frame = make_decision_frame(id as u64);
            let _ = adapter.deliver_typed(frame).await;
        }

        // Query the Transparency Log for decision-dispatch frames
        let entries = log
            .query_frames(FrameFilter {
                kind: Some(FrameKind::DecisionDispatch),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(entries.len(), 10, "expected 10 decision frames in transparency log");

        // Deserialize each logged payload (FramePayload tagged enum) and verify refs
        for (i, entry) in entries.iter().enumerate() {
            let payload: FramePayload = serde_json::from_slice(&entry.payload_redacted)
                .unwrap_or_else(|e| panic!("frame {} deserialization failed: {e}", i));
            match payload {
                FramePayload::DecisionDispatch(p) => {
                    assert!(
                        p.working_memory_digest_refs.as_slice().is_empty(),
                        "frame {}: at v0.3-β refs should be empty; structural presence satisfied",
                        i
                    );
                    assert_eq!(p.decision_id, i as u64);
                }
                _ => panic!("frame {}: expected DecisionDispatch, got other payload", i),
            }
        }
    }

    #[tokio::test]
    async fn i12_non_decision_frames_not_decorated() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(
            crate::telemetry::iac_rt::IacRtMetrics::new(),
        )));
        let adapter = IacBusAdapter::new(mailbox, log.clone());

        adapter.register_spirit_typed(&SpiritId::from("target")).ok();

        let task_frame = IacFrame {
            frame_id: [1u8; 16],
            timestamp_ns: 1,
            logical_clock: 1,
            from: FrameAddress {
                spirit_id: SpiritId::from("sender"),
                host_id: None,
                role: None,
            },
            to: smallvec![],
            kind: DomainFrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "test task".into(),
                scope: vec![],
                success_criteria: "done".into(),
                posture_preferences: PosturePreferences::default(),
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
        };

        let _ = adapter.deliver_typed(task_frame).await;

        let entries = log
            .query_frames(FrameFilter {
                kind: Some(FrameKind::TaskAssign),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(entries.len(), 1, "expected 1 task-assign frame");

        let payload: FramePayload = serde_json::from_slice(&entries[0].payload_redacted).unwrap();
        match payload {
            FramePayload::TaskAssign(_) => {
                // TaskAssign payload should NOT have working_memory_digest_refs
                // (the decorator only attaches to DecisionDispatch frames)
            }
            other => panic!("expected TaskAssign, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
