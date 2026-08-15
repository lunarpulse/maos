#![forbid(unsafe_code)]

//! Frame-borne `developer-remote` delegation — the loopback A2A leg of J1
//! (story `j1-crosshost-1a`).
//!
//! ## Why this module exists
//!
//! Before this story the founder loop handed its Worker a task by reading
//! `MAOS_WORKER_TASK` from the environment. Nothing about the delegation touched
//! A2A, the Orchestrator, or a frame. This module replaces that shortcut with the
//! real wire: the Orchestrator emits a `task.assign` carrying a
//! [`orchestrator::DELEGATION_CONSENT_INTENT`] consent envelope, the frame routes
//! through the loopback A2A router, and an **in-process consumer** receives it and
//! hands the payload's `goal` to the Worker spawn.
//!
//! ## There was no consumer precedent
//!
//! Nothing in MAOS had ever drained a mailbox handle and acted on the frame. The
//! production `maos run` path built the mailbox, gave it a Transparency Log and a
//! tracker, and never called `register_spirit` or `deliver`. The closest template,
//! `smoke_orchestrator_fanout_6_2`, binds **every** handle to `_` — it proves
//! frames are *delivered and journaled*, never that one is *received and acted
//! on*. [`DelegationLeg::recipient`] is the first real binding.
//!
//! ## Why a translator is required at all
//!
//! The Worker is a **subprocess, not a mailbox peer**. `"worker"` is a plain
//! string in `BridgeSpawnSpec::from_spirit_id`; it has no mailbox, no handle, and
//! cannot be a delivery target. "Deliver the frame to the worker" is a category
//! error. The frame is delivered to this consumer, which then *spawns* the worker
//! with the payload's goal.
//!
//! ## Ordering is decided, not open
//!
//! emit → route → pump → drain, all completed **before** the worker admit.
//! `run_cli_wrapper_manifest` stays synchronous and 7+1-arg; it is called with the
//! already-drained goal string. Restructuring the topology loop to `await`
//! mid-call would ripple through every `[cli_wrapper]` admission path for no gain
//! at a rung where the whole exchange is in-process anyway.

use std::sync::Arc;

use maos_a2a::pairing::LoopbackEndpoint;
use maos_a2a::LoopbackA2ARouter;
use maos_domain::frame::{FramePayload, IacFrame, TaskCompletePayload};
use maos_domain::invariants::i8::A2AIntent;
use maos_kernel_core::iac::mailbox::SpiritMailboxHandle;
use maos_kernel_core::iac::{IacBusAdapter, Mailbox};
use maos_spirit_abi::identity::FrameKind;

/// The registered recipient of the delegation frame — a real mailbox identity,
/// distinct from the `"worker"` subprocess label.
pub const RECIPIENT_SPIRIT: &str = "developer-remote";
/// The emitting Spirit. Matches the `Orchestrator::new(...)` id the composition
/// root constructs, because the consent envelope's granter must equal
/// `frame.from` on both `spirit_id` and `host_id`.
pub const FROM_SPIRIT: &str = "orchestrator";
/// The topology `host` on the worker entry, AND the destination `peer_id`
/// (selects `send_allowlist` at `prepare_outbound`).
pub const TO_HOST: &str = "developer-remote-host";
/// The sender's host, AND the `peer_id` whose `accept_allowlist` intake applies:
/// on loopback the peer is resolved from `frame.from.host_id`
/// (`maos-a2a-core/src/router.rs:1087-1090`), so the accept side is keyed by the
/// SOURCE host, not the destination.
pub const FROM_HOST: &str = "founder-loop-host";

/// The installed loopback delegation leg: router, intake pump, and the consumer's
/// own mailbox handles.
///
/// Held by the composition root for the lifetime of the run. Dropping it drops
/// the `developer-remote` handle, which would make the mailbox's mpsc sender
/// disconnect — so it must outlive the delegation.
pub struct DelegationLeg {
    mailbox: Arc<Mailbox>,
    /// The delegation recipient's handle — bound to a real name, **never** `_`.
    /// A `_`-bound `SpiritMailboxHandle` is dropped at the end of the statement
    /// and the frame is silently discarded; that is the 6.2 smoke's mistake.
    recipient: SpiritMailboxHandle,
    /// The Orchestrator's own handle, so the `TaskComplete` reply has a
    /// registered recipient (Phase 1 errors `UnknownSpirit` otherwise).
    orchestrator: SpiritMailboxHandle,
    /// The router's intake sink. The loopback "wire" pushes accepted frames here
    /// after every validation passes.
    intake_rx: tokio::sync::mpsc::UnboundedReceiver<IacFrame>,
    /// Retained so the set-once router install cannot be garbage-collected out
    /// from under the mailbox, and so tests can assert against it.
    _router: Arc<LoopbackA2ARouter>,
}

impl DelegationLeg {
    /// Build the paired loopback router, install it on the **already-`Arc`'d**
    /// mailbox, and register the two mailbox identities.
    ///
    /// The peer/TOFU prologue lives in [`maos_a2a::pairing`] because it is pure
    /// A2A surface — and because the send/accept asymmetry it encodes (accept is
    /// keyed by the SOURCE host on loopback) is the thing every hand-rolled copy
    /// gets wrong.
    ///
    /// `intent` is passed in rather than hard-coded so `j1-crosshost-1b` can drive
    /// refusal legs through this same production path instead of a hand-built
    /// router.
    pub async fn install(mailbox: Arc<Mailbox>, intent: &A2AIntent) -> Result<Self, String> {
        let (router, intake_rx) = maos_a2a::pairing::paired_loopback_router(&[
            LoopbackEndpoint::sender_of(TO_HOST, 7451, intent),
            LoopbackEndpoint::acceptor_of(FROM_HOST, 7452, intent),
        ])
        .await
        .map_err(|e| format!("delegation: loopback pairing failed: {e}"))?;
        mailbox
            .install_a2a_router(Arc::clone(&router) as Arc<dyn maos_domain::ports::a2a::A2ARouter>)
            .map_err(|()| {
                "delegation: an A2A router is already installed — the OnceLock is set-once so the \
                 cross-host router cannot be swapped after boot"
                    .to_string()
            })?;

        let recipient = mailbox
            .register_spirit(RECIPIENT_SPIRIT)
            .map_err(|e| format!("delegation: register {RECIPIENT_SPIRIT}: {e}"))?;
        let orchestrator = mailbox
            .register_spirit(FROM_SPIRIT)
            .map_err(|e| format!("delegation: register {FROM_SPIRIT}: {e}"))?;

        Ok(Self {
            mailbox,
            recipient,
            orchestrator,
            intake_rx,
            _router: router,
        })
    }

    /// emit → route → **pump** → drain, returning the delegated goal.
    ///
    /// The emit goes through [`IacBusAdapter::deliver_typed`] so the frame is
    /// journaled before delivery (I2). Phase 3 of `Mailbox::deliver` hands it to
    /// the installed router because every recipient carries a `host_id`; without
    /// a router that same branch fails closed with `CrossHostNotConfigured`.
    ///
    /// The pump re-delivers with `to[..].host_id` **stripped**. Without the strip
    /// the frame re-enters the cross-host branch forever. It deliberately does
    /// NOT re-journal: the emit already wrote this `frame_id`'s row, and a second
    /// row for the same frame would be a duplicate claim, not extra evidence.
    pub async fn delegate(
        &mut self,
        iac: &IacBusAdapter,
        frame: IacFrame,
    ) -> Result<String, String> {
        iac.deliver_typed(frame).await.map_err(|e| {
            // FOUND BY THIS STORY, owned elsewhere: FR21's gate
            // (`maos-iac/src/adapter/orchestrator_dispatch.rs`) treats ANY
            // `TaskComplete` row inside a 60s WALL-CLOCK window as a predecessor of
            // an Orchestrator `TaskAssign`. That proxy cannot tell "a follow-up
            // inside one fan-out" from "the first dispatch of a NEW process", so a
            // second `maos run` on the same data home within 60s is refused even
            // though this dispatch references nothing (`prior_distillate_ref: None`,
            // empty `scope`) — which is not the case
            // `docs-site/docs/errors/EOrchestratorDispatchRawOutput.md` describes.
            // This story is the FIRST production emitter of such a frame, so it is
            // where the false positive becomes reachable. It is NOT relaxed, faked
            // with a synthetic distillate, or routed around here: the gate's
            // semantics belong to Story 6.2's owners and the real fix is rung-2's
            // stable `task_id` correlation. Fail-closed and self-explaining.
            format!(
                "delegation emit/route failed: {e}\n  \
                 hint: if this is EOrchestratorDispatchRawOutput, a previous run's \
                 TaskComplete row is inside FR21's 60s window on this MAOS_HOME. Use a \
                 fresh data home or wait out the window; see the j1-crosshost-1a dev \
                 record (filed against the Epic-14 decision queue)."
            )
        })?;

        let mut routed = self.intake_rx.try_recv().map_err(|e| {
            format!(
                "delegation did not reach the peer's intake sink ({e}) — the frame was accepted by \
                 the mailbox but never admitted by the router"
            )
        })?;
        for addr in routed.to.iter_mut() {
            addr.host_id = None;
        }
        self.mailbox
            .deliver(routed)
            .await
            .map_err(|e| format!("delegation local re-delivery failed: {e}"))?;

        // Recursion guard, asserted directly: the strip must make the routed
        // frame terminal. A frame still carrying `host_id` would have re-entered
        // Phase 3 and pushed a second copy onto the intake sink.
        if self.intake_rx.try_recv().is_ok() {
            return Err(
                "delegation pump re-emitted a routed frame — the `to[..].host_id` strip \
                        failed and the frame is looping"
                    .to_string(),
            );
        }

        // Drain the consumer's OWN handle and extract the goal. This is the piece
        // with no precedent in the repo.
        match self
            .recipient
            .try_recv()
            .map_err(|e| format!("delegation recipient channel closed: {e}"))?
        {
            Some((FrameKind::TaskAssign, delivered)) => match delivered.payload {
                FramePayload::TaskAssign(p) => Ok(p.goal),
                other => Err(format!(
                    "delegation frame carried {other:?}, not a TaskAssign payload"
                )),
            },
            Some((kind, _)) => Err(format!(
                "delegation recipient received {kind:?}, not TaskAssign"
            )),
            None => Err(format!(
                "delegation frame never reached {RECIPIENT_SPIRIT}'s handle — a `_`-bound handle \
                 silently drops the frame"
            )),
        }
    }

    /// Journal the Worker's completion as the **existing** `TaskComplete` frame,
    /// delivered from the delegation recipient back to the Orchestrator.
    ///
    /// Not a new audit kind: a freshly minted kind would be write-only (nothing
    /// reads it), and the raw-`INSERT` precedent already shipped that defect —
    /// `maos-audit::kind_to_string` did not map `CliSubprocessOutput` (21), which
    /// is why most rows in the signed Tier-2 bundle rendered `unknown`.
    /// `FrameKind::TaskComplete` is a real frame kind, so it renders for free.
    ///
    /// Returns the number of frames the Orchestrator actually drained, so the
    /// caller can prove the loop closed rather than assume it. Only a confirmed
    /// completion can be emitted: the payload is fixed to `"completed"` rather
    /// than accepting an arbitrary adapter outcome label.
    pub async fn journal_completion(
        &mut self,
        iac: &IacBusAdapter,
        seq: u64,
        run_nonce: u64,
    ) -> Result<usize, String> {
        let mut frame_id = [0u8; 16];
        frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
        frame_id[8..16].copy_from_slice(&run_nonce.to_le_bytes());
        let frame = completion_frame(frame_id, seq, "completed".to_string());
        iac.deliver_typed(frame)
            .await
            .map_err(|e| format!("completion journal failed: {e}"))?;
        let mut drained = 0usize;
        while let Ok(Some(_)) = self.orchestrator.try_recv() {
            drained += 1;
        }
        Ok(drained)
    }
}

/// The `developer-remote` → `orchestrator` completion frame. Same-host on both
/// ends: the completion is journaled on the founder host, so it needs no second
/// A2A hop and no second consent grant.
fn completion_frame(frame_id: [u8; 16], seq: u64, result: String) -> IacFrame {
    use maos_domain::frame::FrameAddress;
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_spirit_abi::identity::{SpiritId, SpiritRole};

    let mut to = smallvec::SmallVec::<[FrameAddress; 1]>::new();
    to.push(FrameAddress {
        spirit_id: SpiritId::from(FROM_SPIRIT),
        host_id: None,
        role: Some(SpiritRole::Orchestrator),
    });
    IacFrame {
        frame_id,
        timestamp_ns: seq,
        logical_clock: seq,
        from: FrameAddress {
            spirit_id: SpiritId::from(RECIPIENT_SPIRIT),
            host_id: None,
            role: Some(SpiritRole::Worker),
        },
        to,
        kind: FrameKind::TaskComplete,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskComplete(TaskCompletePayload { result }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: IntentLineage::new(vec![A2AIntent::new(
            orchestrator::DELEGATION_CONSENT_INTENT,
        )]),
    }
}
