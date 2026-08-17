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
use maos_kernel_core::iac::{FrameRowWrite, IacBusAdapter, Mailbox};
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

/// What host B's intake consumer found in one inbound frame
/// (j1-crosshost-2b AC1.3/AC3.2).
///
/// A typed outcome, not a bool and not a log line: the two-daemon proof asserts
/// on this, and the `Duplicate` arm is the observable that proves a peer replay
/// no longer halts the receiving Host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundDelegation {
    /// A first-delivery `TaskAssign`: journaled, delivered, drained.
    TaskAssign {
        /// The RECEIVED `frame_id` — the 16 bytes both Hosts' logs now share, and
        /// the key `j1-crosshost-2c` reconciles on. Deterministic
        /// `seq ‖ run_nonce`, minted by host A (`spirits/orchestrator/src/lib.rs`,
        /// `delegation.rs` `journal_completion`), never re-minted here.
        frame_id: [u8; 16],
        /// The delegated goal, drained from the typed payload.
        goal: String,
        /// Carried, deliberately unused, and DISPOSED OF IN THE RECORD rather than
        /// silently dropped (j1-crosshost-2b AC3.4(c), handed over by `2a`'s F15):
        /// `TaskAssignPayload::success_criteria` is circular — the Orchestrator
        /// writes criteria that nothing evaluates, so treating it as a contract
        /// here would invent an oracle the sender never agreed to. Host B's verdict
        /// comes from the worker CLI's own machine-readable output
        /// (`WorkerCli::parse_completion`), which IS a real contract. Surfacing it
        /// keeps it observable to the test that records the disposition.
        success_criteria: String,
    },
    /// The Transparency Log already held this `frame_id`: a peer replay. The row
    /// is present, nothing was delivered, no worker was spawned, and — the point —
    /// the process is still alive.
    Duplicate { frame_id: [u8; 16] },
}

/// Which A2A router the J1 composition root installs on the mailbox
/// (j1-crosshost-2b AC2.1).
///
/// **Both arms ship, and the loopback one is not dead.** It is the in-process
/// rehearsal rung 1 proved refusals on, it is what `demo-j1` drives, and it is
/// what the `loopback-from-host-unverified` gate leg reports a permanent, honest
/// `true` about: on loopback there is nothing to bind `frame.from.host_id` to, so
/// the frame selects its own `accept_allowlist`. Adding the cross-host arm does not
/// change that fact about the loopback arm — it adds a *different* path beside it.
pub enum DelegationRouter {
    /// The in-process loopback pair. No socket exists: `LoopbackEndpoint::config`
    /// emits `tls://127.0.0.1:7451`/`7452` as strings, and `LoopbackA2ARouter` calls
    /// `handle_intake` DIRECTLY — never `handle_intake_verified` — so no wire
    /// identity is bound and the boot-nonce restart check is skipped via the
    /// `boot_nonce = 0` sentinel.
    LoopbackRehearsal,
    /// A live cross-host transport, TLS-verified end to end. Pass
    /// `Arc<TcpA2ATransport>`: it already implements
    /// `maos_domain::ports::a2a::A2ARouter`, so this costs **zero** new adapter code
    /// and the trait's single `route_outbound` method needs no change.
    ///
    /// On this arm the receiver reaches `handle_intake_verified`, which binds
    /// `frame.from.host_id` to the TLS-verified peer before any trust-bearing work,
    /// and a real per-process boot nonce makes restart detection reachable for the
    /// first time (see the `CODE_SPIRIT_RESTART_DETECTED` arm in
    /// `A2ARouterCore::interpret_response`).
    CrossHostVerified(Arc<dyn maos_domain::ports::a2a::A2ARouter>),
}

/// Where a delegation ended up (j1-crosshost-2b AC2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationOutcome {
    /// Loopback rehearsal: the frame never left the process, so THIS Host runs the
    /// worker with the drained goal.
    RehearsedLocally { goal: String },
    /// The frame left over mTLS. The far Host runs the worker; this Host holds one
    /// journaled emit row whose sixteen bytes are the cross-log join key.
    ///
    /// Deliberately carries **no goal**: returning one would imply this Host still
    /// has work to do, which is exactly the confusion a "round trip" claim would
    /// create. There is no return hop in this story — that is `j1-crosshost-2c`.
    SentCrossHost { frame_id: [u8; 16] },
}

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
    /// The loopback router's intake sink. `None` on the cross-host arm: there is
    /// no local sink to pump from, because the frame genuinely leaves the process.
    intake_rx: Option<tokio::sync::mpsc::Receiver<IacFrame>>,
    /// Retained so the set-once router install cannot be garbage-collected out
    /// from under the mailbox, and so tests can assert against it. `None` on the
    /// cross-host arm — the transport is owned by the composition root there.
    _router: Option<Arc<LoopbackA2ARouter>>,
}

impl DelegationLeg {
    /// Build the delegation leg and install its A2A router on the
    /// **already-`Arc`'d** mailbox, choosing between the loopback rehearsal pair
    /// and a live cross-host transport.
    ///
    /// The peer/TOFU prologue lives in [`maos_a2a::pairing`] because it is pure
    /// A2A surface — and because the send/accept asymmetry it encodes (accept is
    /// keyed by the SOURCE host on loopback) is the thing every hand-rolled copy
    /// gets wrong.
    ///
    /// `intent` is passed in rather than hard-coded so `j1-crosshost-1b` can drive
    /// refusal legs through this same production path instead of a hand-built
    /// router.
    ///
    /// **j1-crosshost-2b AC2.1 — the fork is HERE, not at the call site.**
    /// `Mailbox::install_a2a_router` is a bare `OnceLock::set` with exactly ONE
    /// production caller (the line below), and `main.rs`'s call to this function is
    /// ~5,000 lines BEFORE the `MAOS_ONE_SHOT` dispatch — so by the time any
    /// daemon arm runs, the mailbox's single router slot is already spent. Moving
    /// the call past the dispatch puts it after the daemon arm returns: a dead end.
    /// One caller means the cheap correct shape is to let `install` decide.
    ///
    /// Host A's cross-host arm needs **zero new adapter code**:
    /// `impl maos_domain::ports::a2a::A2ARouter for TcpA2ATransport` already exists,
    /// and the trait has exactly one method (`route_outbound`) — it is outbound-only,
    /// so an inbound pump needs no domain or kernel trait change.
    pub async fn install(mailbox: Arc<Mailbox>, intent: &A2AIntent) -> Result<Self, String> {
        Self::install_with_router(mailbox, intent, DelegationRouter::LoopbackRehearsal).await
    }

    /// [`Self::install`] with the router choice made explicit.
    ///
    /// Kept as a separate entry point so every j1-crosshost-1a/1b caller and
    /// control — including `mailbox_a2a_router_installer_1a.rs`'s set-once contract
    /// — keeps its exact shape, which is the same no-caller-churn discipline 12.3
    /// and 13.6b applied when they added transport seams.
    pub async fn install_with_router(
        mailbox: Arc<Mailbox>,
        intent: &A2AIntent,
        router: DelegationRouter,
    ) -> Result<Self, String> {
        let (installable, loopback_router, intake_rx) = match router {
            DelegationRouter::LoopbackRehearsal => {
                let (router, intake_rx) = maos_a2a::pairing::paired_loopback_router(&[
                    LoopbackEndpoint::sender_of(TO_HOST, 7451, intent),
                    LoopbackEndpoint::acceptor_of(FROM_HOST, 7452, intent),
                ])
                .await
                .map_err(|e| format!("delegation: loopback pairing failed: {e}"))?;
                (
                    Arc::clone(&router) as Arc<dyn maos_domain::ports::a2a::A2ARouter>,
                    Some(router),
                    Some(intake_rx),
                )
            }
            DelegationRouter::CrossHostVerified(router) => (router, None, None),
        };
        mailbox.install_a2a_router(installable).map_err(|()| {
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
            _router: loopback_router,
        })
    }
    /// Emit the delegation, and — on the loopback arm only — pump and drain it.
    ///
    /// The emit goes through [`IacBusAdapter::deliver_typed`] so the frame is
    /// journaled before delivery (I2). Phase 3 of `Mailbox::deliver` hands it to
    /// the installed router because every recipient carries a `host_id`; without
    /// a router that same branch fails closed with `CrossHostNotConfigured`.
    ///
    /// **The two arms end in different places, and that is the point
    /// (j1-crosshost-2b AC2.1).**
    ///
    /// * [`DelegationRouter::LoopbackRehearsal`] — the frame never leaves the
    ///   process. The pump re-delivers it with `to[..].host_id` **stripped**;
    ///   without the strip it re-enters the cross-host branch forever. The pump
    ///   deliberately does NOT re-journal: the emit already wrote this `frame_id`'s
    ///   row, and a second row for the same frame would be a duplicate claim, not
    ///   extra evidence. THIS Host then runs the worker with the drained goal.
    /// * [`DelegationRouter::CrossHostVerified`] — `route_outbound` puts the frame
    ///   on a real mTLS socket and the far Host runs the worker. There is no local
    ///   intake sink to pump from and no goal to return; what this Host keeps is one
    ///   journaled emit row whose sixteen `frame_id` bytes are the join key for the
    ///   receiver's own row (AC3.1/AC3.3).
    pub async fn delegate(
        &mut self,
        iac: &IacBusAdapter,
        frame: IacFrame,
    ) -> Result<DelegationOutcome, String> {
        let frame_id = frame.frame_id;
        iac.deliver_typed(frame).await.map_err(|e| {
            // FOUND BY j1-crosshost-1a, owned elsewhere: FR21's gate
            // (`maos-iac/src/adapter/orchestrator_dispatch.rs`) treats ANY
            // `TaskComplete` row inside a 60s WALL-CLOCK window as a predecessor of
            // an Orchestrator `TaskAssign`. That proxy cannot tell "a follow-up
            // inside one fan-out" from "the first dispatch of a NEW process", so a
            // second `maos run` on the same data home within 60s is refused even
            // though this dispatch references nothing (`prior_distillate_ref: None`,
            // empty `scope`) — which is not the case
            // `docs-site/docs/errors/EOrchestratorDispatchRawOutput.md` describes.
            // j1-crosshost-1a was the FIRST production emitter of such a frame, so
            // it is where the false positive became reachable. It is NOT relaxed,
            // faked with a synthetic distillate, or routed around here: the gate's
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

        // The cross-host arm ends HERE. `route_outbound` has put the frame on a
        // real mTLS socket; there is no local intake sink to pump from and no goal
        // to drain, because the far Host is the one that runs the worker. What this
        // Host keeps is the journaled emit row above, whose `frame_id` is the join
        // key for the receiver's own row.
        let Some(intake_rx) = self.intake_rx.as_mut() else {
            return Ok(DelegationOutcome::SentCrossHost { frame_id });
        };

        let mut routed = intake_rx.try_recv().map_err(|e| {
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
        if self
            .intake_rx
            .as_mut()
            .is_some_and(|rx| rx.try_recv().is_ok())
        {
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
                FramePayload::TaskAssign(p) => {
                    Ok(DelegationOutcome::RehearsedLocally { goal: p.goal })
                }
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

    /// **HOST B** — act on a frame another Host sent (j1-crosshost-2b AC1.3/AC1.4).
    ///
    /// This is the first production code path in MAOS that consumes a frame off a
    /// verified wire. Everything upstream already ran on the receiving side before
    /// the frame got here: mTLS peer authentication, `frame.from.host_id` binding to
    /// the TLS-verified peer, TOFU, the boot-nonce restart check, consent
    /// granter/expiry, the accept-allowlist and the Lamport advance
    /// (`crates/maos-a2a-core/src/router.rs:1494` onward). What was missing was
    /// only this: a sink to push into and a consumer behind it (G1).
    ///
    /// Steps, and why each is not optional:
    ///
    /// 1. **Strip `to[..].host_id`** — the same strip
    ///    [`Self::delegate`] applies on the sending side. Without it the frame
    ///    re-enters `Mailbox::deliver`'s cross-host branch and loops.
    /// 2. **Journal through [`IacBusAdapter::deliver_typed`]** — the typed path, so
    ///    the row carries the RECEIVED `frame_id` (`crates/maos-iac/src/adapter.rs`
    ///    writes `Some(frame.frame_id)`). Not a raw TL insert, and never the
    ///    raw-byte `enqueue_frame`/`broadcast_frame`, which mislabels every row
    ///    `TaskAssign` regardless of the real kind (`mailbox.rs` `TODO(F5)`).
    ///    Those 16 bytes are what AC3.1/AC3.3 join host A's log to host B's — no
    ///    new frame field, no `correlation_id` wiring.
    /// 3. **Honour the typed [`FrameRowWrite::Duplicate`]** — a peer that re-sends
    ///    one frame used to HALT this Host (deterministic `seq ‖ run_nonce` id +
    ///    `BLOB NOT NULL PRIMARY KEY` + a plain `INSERT` + `panic!`). Installing the
    ///    intake sink is what made that reachable, so the caller gets
    ///    [`InboundDelegation::Duplicate`] and the worker is NOT re-spawned.
    /// 4. **Dispatch on `FrameKind`, failing closed and loudly** — copying the
    ///    precedent in [`Self::delegate`] rather than inventing one. A silent
    ///    `_ => {}` here would reproduce G1 one layer up: admitted, acknowledged,
    ///    ignored.
    pub async fn accept_inbound(
        &mut self,
        iac: &IacBusAdapter,
        mut frame: IacFrame,
    ) -> Result<InboundDelegation, String> {
        let frame_id = frame.frame_id;
        // §A6 review P11 — verify THIS host is the addressed recipient BEFORE
        // journaling. `frame_id` is a peer-supplied primary key: once the row is
        // written, host B durably holds evidence for a frame it was never the
        // addressee of, and the loud failure below arrives too late to un-journal
        // it. The router bound `frame.from` to the TLS-verified peer; the `to`
        // side deserves the same check at the consumer.
        let addressed_here = frame.to.iter().any(|addr| {
            addr.spirit_id.as_str() == RECIPIENT_SPIRIT
                && addr
                    .host_id
                    .as_ref()
                    .is_none_or(|host| host.as_str() == TO_HOST)
        });
        if !addressed_here {
            return Err(format!(
                "host B intake received a TaskAssign addressed to {:?}, not \
                 {RECIPIENT_SPIRIT}@{TO_HOST} — refusing before journaling. A TLS-authenticated \
                 peer must not be able to make THIS host journal evidence for a frame it was \
                 never the addressee of",
                frame
                    .to
                    .iter()
                    .map(|a| (a.spirit_id.as_str(), a.host_id.clone()))
                    .collect::<Vec<_>>()
            ));
        }
        // §A6 review P3 — the high bit of the id head is RESERVED for host B's
        // derived outcome rows (see [`Self::outcome_frame_id`]). Frame ids are
        // peer-supplied, so a peer CAN send an id with the high bit set; accepting
        // it would let that frame's outcome row collide with a later inbound id
        // (or vice versa). Host A's real ids are small counters — high bit clear —
        // so refusing here closes the namespace without touching any honest id.
        let head = u64::from_le_bytes(
            frame.frame_id[0..8]
                .try_into()
                .expect("8-byte head of a 16-byte frame id"),
        );
        if head & (1 << 63) != 0 {
            return Err(format!(
                "host B intake refuses frame {}: the high bit of the id head is host B's \
                 outcome-row namespace (outcome_frame_id). A peer-supplied id that sets it \
                 would collide with another inbound frame's derived outcome id",
                frame
                    .frame_id
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ));
        }
        for addr in frame.to.iter_mut() {
            addr.host_id = None;
        }
        match iac
            .deliver_typed(frame)
            .await
            .map_err(|e| format!("host B intake journal/delivery failed: {e}"))?
            .into_inner()
        {
            FrameRowWrite::Duplicate => return Ok(InboundDelegation::Duplicate { frame_id }),
            FrameRowWrite::Written => {}
        }

        match self
            .recipient
            .try_recv()
            .map_err(|e| format!("host B intake recipient channel closed: {e}"))?
        {
            Some((FrameKind::TaskAssign, delivered)) => match delivered.payload {
                FramePayload::TaskAssign(p) => Ok(InboundDelegation::TaskAssign {
                    frame_id,
                    goal: p.goal,
                    success_criteria: p.success_criteria,
                }),
                other => Err(format!(
                    "host B intake frame carried {other:?}, not a TaskAssign payload"
                )),
            },
            Some((kind, _)) => Err(format!(
                "host B intake recipient received {kind:?}, not TaskAssign"
            )),
            None => Err(format!(
                "host B intake frame never reached {RECIPIENT_SPIRIT}'s handle — a `_`-bound \
                 handle silently drops the frame"
            )),
        }
    }

    ///
    /// Distinct from [`Self::journal_completion`], which hardcodes `"completed"`
    /// and is correct on host A precisely because 2a hoisted
    /// `if !completion.is_completed() { return Err(…) }` ABOVE its call site — so
    /// on host A only `Completed` can reach it and the literal is a redundancy, not
    /// a lie.
    ///
    /// Host B is where the vocabulary is genuinely reachable. A remote worker's
    /// failure must become **evidence**, not an aborted process: host B cannot
    /// abort the delegation on host A's behalf (that is FR20's in-flight
    /// semantics, owned by Story 6.2 — see [`Self::delegate`]), so it journals
    /// whichever of the SIX `WorkerCompletion::label()` values actually happened
    /// and lets the operator join it to host A's row on `frame_id`.
    ///
    /// `label` is the adapter's own verdict string, never a literal minted here.
    pub async fn journal_inbound_outcome(
        &mut self,
        iac: &IacBusAdapter,
        inbound_frame_id: [u8; 16],
        label: &str,
    ) -> Result<[u8; 16], String> {
        let outcome_frame_id = Self::outcome_frame_id(inbound_frame_id);
        let seq = u64::from_le_bytes(
            outcome_frame_id[0..8]
                .try_into()
                .expect("8-byte head of a 16-byte frame id"),
        );
        let frame = completion_frame(outcome_frame_id, seq, label.to_string());
        // §A6 review P3 — the write verdict is CHECKED, not discarded. A
        // `Duplicate` here means the derived outcome id already has a row —
        // either a namespace collision with a peer-supplied inbound id (which
        // `accept_inbound` now refuses on arrival) or a replayed outcome. Both
        // are "the evidence you are about to claim does not exist": returning
        // the id as if persisted would repeat G1's lie one layer up.
        match iac
            .deliver_typed(frame)
            .await
            .map_err(|e| format!("host B outcome journal failed: {e}"))?
            .into_inner()
        {
            FrameRowWrite::Duplicate => {
                return Err(format!(
                    "host B outcome row {} was already present — the worker's verdict was NOT \
                     journaled. Namespace collision or outcome replay; audit both ids before \
                     claiming this evidence",
                    outcome_frame_id
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ));
            }
            FrameRowWrite::Written => {}
        }
        while let Ok(Some(_)) = self.orchestrator.try_recv() {}
        Ok(outcome_frame_id)
    }

    /// Derive host B's outcome-row id from the inbound frame's id.
    ///
    /// J1 ids are `seq ‖ run_nonce` (`spirits/orchestrator/src/lib.rs`,
    /// [`Self::journal_completion`]) — deterministic, no ULID entropy. Host B
    /// cannot reuse the inbound id: `frame_id` is `BLOB NOT NULL PRIMARY KEY` and
    /// the inbound row is already under it, so the outcome write would come back as
    /// [`InboundDelegation::Duplicate`] and journal nothing.
    ///
    /// So the **tail is preserved** — bytes 8..16 carry `run_nonce`, which is the
    /// dimension that identifies the run and lets an operator group host B's
    /// outcome with the crossing it belongs to — and the **head's high bit is
    /// flipped**. That is deterministic (no clock, no randomness, replayable),
    /// distinct from every host-A `seq` for the same run (host A's `seq` values are
    /// small counters, so the high bit is clear), and reversible, so the inbound id
    /// can be recovered from the outcome row by flipping it back.
    ///
    /// The AC3.1/AC3.3 join is NOT this row: it is host A's emit row and host B's
    /// intake row carrying the SAME sixteen bytes. This row is host B's additional
    /// outcome evidence (AC3.4(a)).
    pub fn outcome_frame_id(inbound_frame_id: [u8; 16]) -> [u8; 16] {
        const HOST_B_OUTCOME_MARK: u64 = 1 << 63;
        let mut out = inbound_frame_id;
        let head = u64::from_le_bytes(
            inbound_frame_id[0..8]
                .try_into()
                .expect("8-byte head of a 16-byte frame id"),
        );
        out[0..8].copy_from_slice(&(head ^ HOST_B_OUTCOME_MARK).to_le_bytes());
        out
    }
}

/// Everything host B needs to spawn its own worker for an inbound delegation.
///
/// Owned and `Clone`-free: the drain task holds one behind an `Arc` and hands
/// clones of the cheap parts into `spawn_blocking`.
pub struct HostBWorkerContext {
    /// The parsed `[cli_wrapper]` manifest host B runs. Configured by the operator
    /// as `worker_manifest` in `MAOS_COHORT_DAEMON_CONFIG`; absent means this
    /// daemon has no J1 consumer and no intake sink is installed at all.
    pub manifest_root: toml::Value,
    pub run: crate::worker_spawn::RunArgs,
    pub transparency_log: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    pub capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    pub spirit_host: Option<Arc<dyn maos_host::SpiritHostPort>>,
    /// Threaded, NOT dropped. Host B mints a `Scope::CliSubprocessSpawn` cap-token
    /// for a task a REMOTE host asked for; running that mint without the SSO/PDP
    /// governance the local `maos run` path applies would make host B the weaker
    /// endpoint precisely where the trust boundary is.
    /// §A6 review P16 (D1) — always `true` for the host-B drain: a governance
    /// failure on a REMOTE-requested spawn refuses instead of downgrading.
    pub remote_requested: bool,
    pub enterprise_runtime: Option<Arc<crate::enterprise_identity::EnterpriseRuntime>>,
    pub enterprise_pdp_runtime: Option<Arc<crate::enterprise_pdp_runtime::EnterprisePdpRuntime>>,
}

/// What host B actually did with one inbound frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostBOutcome {
    /// The worker ran. `label` is one of `WorkerCompletion::label()`'s **six**
    /// values — the worker's real verdict, never a literal (AC3.4(a)).
    Ran {
        frame_id: [u8; 16],
        outcome_frame_id: [u8; 16],
        label: String,
    },
    /// A peer replay: the row was already journaled, so no worker ran and the
    /// process is still alive (AC3.2).
    Duplicate { frame_id: [u8; 16] },
}

/// Handle ONE inbound frame end to end: journal it, spawn host B's worker, journal
/// the worker's real outcome.
///
/// Separate from [`serve_host_b_intake`]'s loop on purpose — the two-daemon proof
/// and the unit vectors drive this directly, in-process, with typed assertions.
/// That in-process reachability is the whole reason AC1.1 moved the worker-spawn
/// surface under the library.
pub async fn handle_one_inbound(
    leg: &mut DelegationLeg,
    iac: &IacBusAdapter,
    ctx: &HostBWorkerContext,
    frame: IacFrame,
) -> Result<HostBOutcome, String> {
    let (frame_id, goal) = match leg.accept_inbound(iac, frame).await? {
        InboundDelegation::Duplicate { frame_id } => {
            return Ok(HostBOutcome::Duplicate { frame_id })
        }
        InboundDelegation::TaskAssign { frame_id, goal, .. } => (frame_id, goal),
    };

    // j1-crosshost-2b AC1.5 — `run_cli_wrapper_manifest` is SYNCHRONOUS and blocks
    // for the worker's entire lifetime: `spawn_and_bridge`, then the blocking
    // `pump_to_journal` and `wait_and_finalize` on the returned bridge. Calling it
    // directly from this task would park a `multi_thread` reactor worker thread for
    // as long as a remote agent CLI takes to finish — on the daemon that also
    // serves the accept loop, the cohort pull ticker and the digest replier.
    // `spawn_blocking` moves it to the blocking pool, which is what that pool is
    // for. Nothing inside is `async`, so there is no runtime to re-enter.
    let manifest_root = ctx.manifest_root.clone();
    let run = ctx.run.clone();
    let transparency_log = Arc::clone(&ctx.transparency_log);
    let capability = Arc::clone(&ctx.capability);
    let spirit_host = ctx.spirit_host.clone();
    let enterprise_runtime = ctx.enterprise_runtime.clone();
    let enterprise_pdp_runtime = ctx.enterprise_pdp_runtime.clone();
    let remote_requested = ctx.remote_requested;
    let completion = tokio::task::spawn_blocking(move || {
        crate::worker_spawn::run_cli_wrapper_manifest(
            &manifest_root,
            &run,
            transparency_log,
            capability,
            spirit_host,
            enterprise_runtime,
            enterprise_pdp_runtime.as_deref(),
            Some(goal.as_str()),
            remote_requested,
        )
        // `Box<dyn Error>` is not `Send`, so it cannot cross the join. Flatten to a
        // string INSIDE the blocking closure rather than widening the error type.
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("host B worker spawn_blocking join failed: {e}"))?
    .map_err(|e| format!("host B worker run failed: {e}"))?;

    let label = completion.label().to_string();
    let outcome_frame_id = leg.journal_inbound_outcome(iac, frame_id, &label).await?;
    Ok(HostBOutcome::Ran {
        frame_id,
        outcome_frame_id,
        label,
    })
}

/// Drain host B's installed intake sink until the daemon is cancelled.
///
/// A per-frame failure is LOUD and does not kill the daemon: the receiving Host
/// must keep serving its peers (manifest pulls, halt receipts, digest replies)
/// after refusing one bad frame. A `None` from `recv()` means the transport that
/// owns the sender is gone, which ends the loop.
pub async fn serve_host_b_intake(
    mut leg: DelegationLeg,
    iac: Arc<IacBusAdapter>,
    ctx: Arc<HostBWorkerContext>,
    mut intake_rx: tokio::sync::mpsc::Receiver<IacFrame>,
    cancel: tokio_util::sync::CancellationToken,
) {
    loop {
        let frame = tokio::select! {
            // §A6 review P13 — `biased;`: when cancellation and a queued frame
            // are both ready, shutdown WINS. Without it the random branch pick
            // can start another remote worker after the daemon began draining.
            biased;
            _ = cancel.cancelled() => return,
            received = intake_rx.recv() => match received {
                Some(frame) => frame,
                None => return,
            },
        };
        match handle_one_inbound(&mut leg, &iac, &ctx, frame).await {
            Ok(HostBOutcome::Ran {
                frame_id,
                label,
                outcome_frame_id,
            }) => println!(
                "{}",
                serde_json::json!({
                    "event": "host_b_delegation_served",
                    "frame_id": hex16(frame_id),
                    "outcome_frame_id": hex16(outcome_frame_id),
                    "worker_outcome": label,
                })
            ),
            Ok(HostBOutcome::Duplicate { frame_id }) => eprintln!(
                "host B intake: replayed frame_id {} already journaled — no worker spawned, \
                 process alive (j1-crosshost-2b AC3.2)",
                hex16(frame_id)
            ),
            Err(error) => eprintln!("host B intake refused a frame: {error}"),
        }
    }
}

/// Lower-hex a frame id for operator-facing events. Non-secret by construction —
/// a frame id carries no payload.
fn hex16(frame_id: [u8; 16]) -> String {
    frame_id.iter().map(|b| format!("{b:02x}")).collect()
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
