#![forbid(unsafe_code)]

//! v0.3-β Mailbox — per-Spirit bounded channel routing.
//!
//! Replaces the v0.1-β `mailbox_stub.rs` placeholder for production use.
//! The stub stays in-tree (referenced by Story 6.1 scaffolding), but the
//! production path calls `Mailbox::deliver`.
//!
//! Architecture §7.1.1 + §7.1.2 + Invariant I2 (log-before-deliver) are
//! enforced as a single pipeline: the adapter's `deliver_typed` performs
//! the Transparency Log write, then `Mailbox::deliver` routes to the
//! per-Spirit MPSC or broadcast sender.
//!
//! # I9 status
//!
//! The `DashMap<(String, FrameKind), mpsc::Sender<IacFrame>>` is transient
//! per-process state, not persistent — no I9 exemption needed.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};

use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i2::LogBeforeDeliver;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::IacBusPort;
use maos_spirit_abi::identity::{FrameKind, SpiritId};

use super::channels::{channel_class_for, ChannelClass};
use crate::telemetry::iac_rt::IacRtMetrics;

/// Per-Host Mailbox — the same-Host IAC router.
#[maos_attrs::i9_exempt(reason = "per-Spirit mailbox router; DashMap holds transient per-process mpsc senders, not persistent state")]
#[derive(Debug)]
pub struct Mailbox {
    mpsc_senders: DashMap<(String, FrameKind), mpsc::Sender<IacFrame>>,
    broadcast_sender: broadcast::Sender<IacFrame>,
    metrics: Arc<IacRtMetrics>,
}

impl Mailbox {
    pub fn new(metrics: Arc<IacRtMetrics>) -> Self {
        let (broadcast_sender, _) = broadcast::channel(256);
        Self {
            mpsc_senders: DashMap::new(),
            broadcast_sender,
            metrics,
        }
    }

    pub fn register_spirit(&self, spirit_id: &str) -> Result<SpiritMailboxHandle, IacBusError> {
        // Guard against double-registration (F14)
        if self.mpsc_senders.contains_key(&(spirit_id.to_string(), FrameKind::TaskAssign)) {
            return Err(IacBusError::AlreadyRegistered(spirit_id.to_string()));
        }
        let mut receivers = Vec::with_capacity(6);
        let kinds: &[FrameKind] = &[
            FrameKind::TaskAssign,
            FrameKind::TaskComplete,
            FrameKind::DecisionDispatch,
            FrameKind::EpistemicHalt,
            FrameKind::ConsentRequest,
            FrameKind::Retract,
        ];
        for &kind in kinds {
            let (class, capacity) = channel_class_for(kind)
                .expect("IAC frame kind must be routable");
            assert_eq!(class, ChannelClass::Mpsc, "all 1:1 kinds must use mpsc");
            let (tx, rx) = mpsc::channel(capacity);
            self.mpsc_senders.insert((spirit_id.to_string(), kind), tx);
            receivers.push((kind, rx));
        }
        Ok(SpiritMailboxHandle {
            spirit_id: spirit_id.to_string(),
            receivers,
            metrics: Arc::clone(&self.metrics),
        })
    }

    /// Deliver a frame through the per-Spirit MPSC or broadcast sender.
    ///
    /// The I2 log-before-deliver guarantee is satisfied by the caller
    /// (`IacBusAdapter::deliver_typed`) BEFORE calling this method.
    pub async fn deliver(&self, frame: IacFrame) -> Result<LogBeforeDeliver<()>, IacBusError> {
        let kind = frame.kind;

        if kind == FrameKind::TelemetryEvent {
            // F12: track broadcast telemetry in pending gauge
            self.metrics.inc_pending("broadcast", kind);
            // TODO: broadcast receiver drain does not call dec_pending —
            // pending gauge for "broadcast" is monotonic (append-only) until
            // a future story wires per-subscriber ACK tracking.
            let _ = self.broadcast_sender.send(frame);
            return Ok(LogBeforeDeliver::new(()));
        }

        // Phase 1: Validate all recipients exist
        for addr in &frame.to {
            let spirit_id = addr.spirit_id.as_str().to_string();
            if addr.host_id.is_some() {
                return Err(IacBusError::CrossHostUnsupported);
            }
            if !self.mpsc_senders.contains_key(&(spirit_id.clone(), kind)) {
                return Err(IacBusError::UnknownSpirit(spirit_id));
            }
        }

        // Phase 2: Send to all validated recipients (backpressure via send().await)
        for addr in &frame.to {
            let spirit_id = addr.spirit_id.as_str().to_string();
            let sender = self
                .mpsc_senders
                .get(&(spirit_id.clone(), kind))
                .expect("validated in phase 1");

            match sender.send(frame.clone()).await {
                Ok(()) => {
                    self.metrics.inc_pending(&spirit_id, kind);
                }
                Err(_) => {
                    if kind == FrameKind::EpistemicHalt {
                        return Err(IacBusError::HaltQueueOverflow(spirit_id));
                    }
                    return Err(IacBusError::ChannelClosed(spirit_id, kind));
                }
            }
        }

        Ok(LogBeforeDeliver::new(()))
    }

    pub fn subscribe_telemetry(&self) -> broadcast::Receiver<IacFrame> {
        self.broadcast_sender.subscribe()
    }

    pub fn metrics(&self) -> &Arc<IacRtMetrics> {
        &self.metrics
    }
}

/// Handle returned by `Mailbox::register_spirit`.
#[maos_attrs::i9_exempt(reason = "per-Spirit mailbox handle; Vec<mpsc::Receiver> is transient per-process state")]
pub struct SpiritMailboxHandle {
    pub spirit_id: String,
    receivers: Vec<(FrameKind, mpsc::Receiver<IacFrame>)>,
    metrics: Arc<IacRtMetrics>,
}

impl std::fmt::Debug for SpiritMailboxHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiritMailboxHandle")
            .field("spirit_id", &self.spirit_id)
            .finish()
    }
}

impl SpiritMailboxHandle {
    pub async fn recv(&mut self) -> Option<(FrameKind, IacFrame)> {
        if self.receivers.is_empty() {
            return None;
        }
        loop {
            // Fair non-blocking drain: try all receivers in round-robin order
            for (kind, rx) in &mut self.receivers {
                match rx.try_recv() {
                    Ok(frame) => {
                        self.metrics.dec_pending(&self.spirit_id, *kind);
                        return Some((*kind, frame));
                    }
                    Err(mpsc::error::TryRecvError::Empty) => continue,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        // F20: drain remaining receivers before returning None
                        for (k2, rx2) in &mut self.receivers {
                            if let Ok(f) = rx2.try_recv() {
                                self.metrics.dec_pending(&self.spirit_id, *k2);
                                return Some((*k2, f));
                            }
                        }
                        return None;
                    }
                }
            }
            // All empty — yield to let producers make progress, then retry
            tokio::task::yield_now().await;
        }
    }

    pub fn try_recv(&mut self) -> Result<Option<(FrameKind, IacFrame)>, mpsc::error::TryRecvError> {
        for (kind, rx) in &mut self.receivers {
            match rx.try_recv() {
                Ok(frame) => {
                    self.metrics.dec_pending(&self.spirit_id, *kind);
                    return Ok(Some((*kind, frame)));
                }
                Err(mpsc::error::TryRecvError::Empty) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

// Implement IacBusPort for IacBusAdapter
impl IacBusPort for super::IacBusAdapter {
    type MailboxHandle = SpiritMailboxHandle;

    fn enqueue_frame(
        &self,
        frame_bytes: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        // TODO(F5): raw-byte path always logs FrameKind::TaskAssign — legacy callers
        // only. Use deliver() for typed frames. Story 6.1 owns the raw-byte path.
        self.enqueue_frame_bytes(frame_bytes, origin)
    }

    fn broadcast_frame(
        &self,
        frame_bytes: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        // TODO(F5): raw-byte path always logs FrameKind::TaskAssign — legacy callers
        // only. Use deliver() for typed frames. Story 6.1 owns the raw-byte path.
        self.broadcast_frame_bytes(frame_bytes, origin)
    }

    async fn deliver(
        &self,
        frame: IacFrame,
    ) -> Result<LogBeforeDeliver<()>, IacBusError> {
        self.deliver_typed(frame).await
    }

    fn register_spirit(
        &self,
        spirit_id: &SpiritId,
    ) -> Result<Self::MailboxHandle, IacBusError> {
        self.register_spirit_typed(spirit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::frame::{
        FrameAddress, FramePayload, TaskAssignPayload, TelemetryEventPayload,
    };
    use maos_spirit_abi::identity::{HostId, SpiritId, SpiritRole};
    use smallvec::smallvec;

    fn make_test_frame() -> IacFrame {
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: FrameAddress {
                spirit_id: SpiritId::from("director"),
                host_id: None,
                role: Some(SpiritRole::Director),
            },
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("test-spirit"),
                host_id: None,
                role: None,
            }],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "do something".into(),
                scope: vec![],
                success_criteria: "done".into(),
                posture_preferences: Default::default(),
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: maos_domain::invariants::i13::IntentLineage::default(),
        }
    }

    #[tokio::test]
    async fn register_spirit_creates_channels() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let handle = mailbox.register_spirit("test-spirit").unwrap();
        assert_eq!(handle.spirit_id, "test-spirit");
        assert_eq!(handle.receivers.len(), 6);
    }

    #[tokio::test]
    async fn register_spirit_rejects_double_registration() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let _handle = mailbox.register_spirit("dup-spirit").unwrap();
        let result = mailbox.register_spirit("dup-spirit");
        assert!(matches!(result, Err(IacBusError::AlreadyRegistered(_))));
    }

    #[tokio::test]
    async fn deliver_to_unregistered_spirit_fails() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let frame = make_test_frame();
        let result = mailbox.deliver(frame).await;
        assert!(matches!(result, Err(IacBusError::UnknownSpirit(_))));
    }

    #[tokio::test]
    async fn deliver_and_receive_round_trip() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let mut handle = mailbox.register_spirit("test-spirit").unwrap();
        let frame = make_test_frame();
        mailbox.deliver(frame).await.unwrap();
        let (kind, received) = handle.try_recv().unwrap().unwrap();
        assert_eq!(kind, FrameKind::TaskAssign);
        assert_eq!(received.to[0].spirit_id.as_str(), "test-spirit");
    }

    #[tokio::test]
    async fn telemetry_event_uses_broadcast() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let mut sub = mailbox.subscribe_telemetry();
        let mut frame = make_test_frame();
        frame.kind = FrameKind::TelemetryEvent;
        frame.payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
            event_type: "test".into(),
            data: "data".into(),
        });
        mailbox.deliver(frame).await.unwrap();
        let received = sub.try_recv().unwrap();
        assert_eq!(received.kind, FrameKind::TelemetryEvent);
    }

    #[tokio::test]
    async fn broadcast_slow_subscriber_sees_lagged() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let mut sub = mailbox.subscribe_telemetry();
        for i in 0..300u64 {
            let mut frame = make_test_frame();
            frame.kind = FrameKind::TelemetryEvent;
            frame.timestamp_ns = i;
            frame.payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
                event_type: "flood".into(),
                data: i.to_string(),
            });
            let _ = mailbox.deliver(frame).await;
        }
        // After flooding 300 frames into a 256-capacity broadcast channel,
        // the slow subscriber should see evidence of lagging or receive
        // an available frame. Either outcome is acceptable per §7.1.1.
        match sub.try_recv() {
            Ok(frame) => {
                assert_eq!(frame.kind, FrameKind::TelemetryEvent);
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                assert!(n > 0, "lagged count must be positive");
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                panic!("channel should not be empty after 300 sends");
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                panic!("channel should not be closed");
            }
        }
    }

    #[tokio::test]
    async fn cross_host_addressing_rejected() {
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let _handle = mailbox.register_spirit("test-spirit").unwrap();
        let mut frame = make_test_frame();
        frame.to[0].host_id = Some(HostId("remote".into()));
        let result = mailbox.deliver(frame).await;
        assert!(matches!(result, Err(IacBusError::CrossHostUnsupported)));
    }
}
