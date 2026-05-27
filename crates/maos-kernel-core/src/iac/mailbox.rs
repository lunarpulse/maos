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

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};

use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i2::LogBeforeDeliver;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::a2a::A2ARouter; // Story 6.3
use maos_domain::ports::IacBusPort;
use maos_spirit_abi::identity::{FrameKind, SpiritId};

use super::channels::{channel_class_for, ChannelClass};
use crate::scheduler::control_block::SpiritControlBlock;
use crate::telemetry::iac_rt::IacRtMetrics;

/// Per-Host Mailbox — the same-Host IAC router.
#[maos_attrs::i9_exempt(
    reason = "per-Spirit mailbox router; DashMap holds transient per-process mpsc senders, not persistent state"
)]
pub struct Mailbox {
    mpsc_senders: DashMap<(String, FrameKind), mpsc::Sender<IacFrame>>,
    broadcast_sender: broadcast::Sender<IacFrame>,
    metrics: Arc<IacRtMetrics>,
    scbs: Mutex<Option<Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>>>,
    /// Story 6.3 — A2A router installed at composition root. `None` means
    /// no peer is configured; cross-host frames fire `CrossHostNotConfigured`.
    a2a_router: Option<Arc<dyn A2ARouter>>,
}

impl std::fmt::Debug for Mailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mailbox")
            .field("mpsc_senders", &self.mpsc_senders)
            .field("broadcast_sender", &self.broadcast_sender)
            .field("metrics", &self.metrics)
            .field("scbs", &self.scbs)
            .field("a2a_router_present", &self.a2a_router.is_some())
            .finish()
    }
}

impl Mailbox {
    pub fn new(metrics: Arc<IacRtMetrics>) -> Self {
        let (broadcast_sender, _) = broadcast::channel(256);
        Self {
            mpsc_senders: DashMap::new(),
            broadcast_sender,
            metrics,
            scbs: Mutex::new(None),
            a2a_router: None,
        }
    }

    pub fn new_with_scbs(
        metrics: Arc<IacRtMetrics>,
        scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    ) -> Self {
        let (broadcast_sender, _) = broadcast::channel(256);
        Self {
            mpsc_senders: DashMap::new(),
            broadcast_sender,
            metrics,
            scbs: Mutex::new(Some(scbs)),
            a2a_router: None,
        }
    }

    /// Story 6.3 — install the A2A router at composition root. Returns
    /// `self` to allow chained builders.
    pub fn with_a2a_router(mut self, router: Arc<dyn A2ARouter>) -> Self {
        self.a2a_router = Some(router);
        self
    }

    pub fn register_spirit(&self, spirit_id: &str) -> Result<SpiritMailboxHandle, IacBusError> {
        // Guard against double-registration (F14)
        if self
            .mpsc_senders
            .contains_key(&(spirit_id.to_string(), FrameKind::TaskAssign))
        {
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
            let (class, capacity) =
                channel_class_for(kind).expect("IAC frame kind must be routable");
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

        // Phase 1: Partition recipients into same-host and cross-host.
        // Story 6.3 — cross-host frames route via the installed A2ARouter;
        // when no router is installed, fire `CrossHostNotConfigured`.
        let mut cross_host_targets: Vec<maos_spirit_abi::identity::HostId> = Vec::new();
        for addr in &frame.to {
            let spirit_id = addr.spirit_id.as_str().to_string();
            if let Some(host_id) = &addr.host_id {
                cross_host_targets.push(host_id.clone());
                continue; // same-host validation skipped for cross-host addresses
            }
            if !self.mpsc_senders.contains_key(&(spirit_id.clone(), kind)) {
                return Err(IacBusError::UnknownSpirit(spirit_id));
            }
        }

        // Phase 2: Deliver to same-host recipients FIRST (independent from
        // cross-host). Per D3: cross-host failures MUST NOT block same-host
        // delivery — these are separate concerns.
        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();

        // Story 5.3 — update sender's last_progress_iac_ns for spirit-origin frames.
        if frame.auto_marker.is_spirit_origin() {
            let guard = self
                .scbs
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if let Some(ref scbs) = *guard {
                if let Ok(scbs) = scbs.read() {
                    let sender_spirit_id = frame.from.spirit_id.as_str();
                    for (_, scb) in scbs.iter() {
                        if scb.spirit_id == sender_spirit_id {
                            scb.last_progress_iac_ns.store(now_ns, Ordering::Relaxed);
                        }
                    }
                }
            }
            drop(guard);
        }

        for addr in &frame.to {
            // Story 6.3 — cross-host addresses are routed in Phase 3; skip
            // the same-host mailbox path for those.
            if addr.host_id.is_some() {
                continue;
            }
            let spirit_id = addr.spirit_id.as_str().to_string();
            let sender = self
                .mpsc_senders
                .get(&(spirit_id.clone(), kind))
                .expect("validated in phase 1");

            // Update last_inbound_frame_ns for the recipient SCB.
            let guard = self
                .scbs
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if let Some(ref scbs) = *guard {
                if let Ok(scbs) = scbs.read() {
                    for (_, scb) in scbs.iter() {
                        if scb.spirit_id == spirit_id {
                            scb.last_inbound_frame_ns.store(now_ns, Ordering::Relaxed);
                        }
                    }
                }
            }
            drop(guard);

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

        // Phase 3: Route cross-host frames through the A2A router.
        // Per-architecture §7.2 + AC2/AC3: cross-host delivery is independent
        // from same-host; same-host recipients already got their copy above.
        // Per-target error collection: deliver to all reachable peers, return
        // the first error (or Ok if all succeed).
        if !cross_host_targets.is_empty() {
            let router = self.a2a_router.as_ref().ok_or_else(|| {
                IacBusError::CrossHostNotConfigured {
                    host_id: cross_host_targets
                        .iter()
                        .map(|h| h.as_str().to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                }
            })?;
            let mut first_err: Option<IacBusError> = None;
            for host_id in &cross_host_targets {
                if let Err(e) = router.route_outbound(frame.clone(), host_id).await {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
            if let Some(e) = first_err {
                return Err(e);
            }
        }

        Ok(LogBeforeDeliver::new(()))
    }

    pub fn subscribe_telemetry(&self) -> broadcast::Receiver<IacFrame> {
        self.broadcast_sender.subscribe()
    }

    /// Story 6.1 — deliver a retract frame with in-queue overtake.
    ///
    /// NOTE: True in-queue overtake (scanning the recipient's channel and
    /// dropping the original frame before delivery) is not implementable
    /// with tokio::sync::mpsc from the sender side. The practical v0.5
    /// semantics are:
    ///   1. Mark the original frame as retracted in the Transparency Log.
    ///   2. Deliver the Retract frame through the normal pipeline.
    ///   3. If the original frame is still in the recipient's channel, it
    ///      will be delivered post-hoc; the recipient checks the TL retraction
    ///      marker and handles it appropriately (per ADR-022).
    pub async fn deliver_with_overtake(
        &self,
        retract_frame: IacFrame,
        _original_frame_id: [u8; 16],
    ) -> Result<LogBeforeDeliver<()>, IacBusError> {
        // Deliver the retract frame normally
        self.deliver(retract_frame).await
    }

    pub fn metrics(&self) -> &Arc<IacRtMetrics> {
        &self.metrics
    }

    pub fn set_scbs(&self, scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>) {
        let mut guard = self
            .scbs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *guard = Some(scbs);
    }
}

/// Handle returned by `Mailbox::register_spirit`.
#[maos_attrs::i9_exempt(
    reason = "per-Spirit mailbox handle; Vec<mpsc::Receiver> is transient per-process state"
)]
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

    fn enqueue_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()> {
        // TODO(F5): raw-byte path always logs FrameKind::TaskAssign — legacy callers
        // only. Use deliver() for typed frames. Story 6.1 owns the raw-byte path.
        self.enqueue_frame_bytes(frame_bytes, origin)
    }

    fn broadcast_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()> {
        // TODO(F5): raw-byte path always logs FrameKind::TaskAssign — legacy callers
        // only. Use deliver() for typed frames. Story 6.1 owns the raw-byte path.
        self.broadcast_frame_bytes(frame_bytes, origin)
    }

    async fn deliver(&self, frame: IacFrame) -> Result<LogBeforeDeliver<()>, IacBusError> {
        self.deliver_typed(frame).await
    }

    fn register_spirit(&self, spirit_id: &SpiritId) -> Result<Self::MailboxHandle, IacBusError> {
        self.register_spirit_typed(spirit_id)
    }

    async fn retract(
        &self,
        original_frame_id: [u8; 16],
        reason: String,
        retracting_spirit: &SpiritId,
    ) -> Result<maos_domain::iac_bus_types::RetractOutcome, IacBusError> {
        self.retract(original_frame_id, reason, retracting_spirit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::frame::{
        FrameAddress, FramePayload, TaskAssignPayload, TelemetryEventPayload,
    };
    use maos_domain::invariants::i1::IntentClass;
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
                prior_distillate_ref: None,
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
    async fn cross_host_addressing_rejected_when_no_router_configured() {
        // Story 6.3 AC2 — replaces `CrossHostUnsupported` blanket reject with
        // `CrossHostNotConfigured` for the operator-not-configured case.
        let metrics = Arc::new(IacRtMetrics::new());
        let mailbox = Mailbox::new(metrics);
        let _handle = mailbox.register_spirit("test-spirit").unwrap();
        let mut frame = make_test_frame();
        frame.to[0].host_id = Some(HostId("remote".into()));
        let result = mailbox.deliver(frame).await;
        assert!(
            matches!(
                result,
                Err(IacBusError::CrossHostNotConfigured { ref host_id }) if host_id == "remote"
            ),
            "expected CrossHostNotConfigured, got {result:?}"
        );
    }

    /// Story 6.3 AC2 — when the A2A router IS installed, cross-host frames
    /// route through it. The test fixture's router is a stub that records the
    /// outbound call and returns `Ok`; verify the router is consulted and
    /// no `CrossHostNotConfigured` error fires.
    #[tokio::test]
    async fn cross_host_routes_through_installed_a2a_router() {
        use async_trait::async_trait;
        use maos_domain::ports::a2a::A2ARouter;

        // Initialize the monotonic clock base (kernel-core's runtime
        // invariant; the production path initializes at composition root).
        crate::capability::cap_tokens::init_monotonic_base();

        struct StubRouter {
            calls: tokio::sync::Mutex<Vec<String>>,
        }
        #[async_trait]
        impl A2ARouter for StubRouter {
            async fn route_outbound(
                &self,
                _frame: IacFrame,
                peer: &HostId,
            ) -> Result<(), IacBusError> {
                let mut g = self.calls.lock().await;
                g.push(peer.as_str().to_string());
                Ok(())
            }
        }

        let metrics = Arc::new(IacRtMetrics::new());
        let stub = Arc::new(StubRouter {
            calls: tokio::sync::Mutex::new(Vec::new()),
        });
        let mailbox = Mailbox::new(metrics).with_a2a_router(stub.clone());
        let _handle = mailbox.register_spirit("test-spirit").unwrap();
        let mut frame = make_test_frame();
        frame.to[0].host_id = Some(HostId("remote-peer".into()));
        let result = mailbox.deliver(frame).await;
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let calls = stub.calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "remote-peer");
    }
}
