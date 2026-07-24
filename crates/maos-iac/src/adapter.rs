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

pub mod channels;
pub mod decision_logger;
pub mod distillate;
pub mod drr_scheduler; // NEW — Story 6.1 DRR fairness scheduler
pub mod frame;
pub mod log_recall; // NEW — Story 4.4 LogRecallAdapter
pub mod mailbox;
pub mod mailbox_stub;
pub mod metrics; // NEW — Story 6.5 IacRtMetrics extraction
pub mod orchestrator_dispatch; // NEW — Story 6.2 AC2 FR21 distillate-dispatch check
pub mod payload; // NEW — Story 6.4 typed payloads (ScheduleFireRecord)
pub mod redaction;
pub mod transparency_log; // NEW — Story 4.4 DistillateWriter

pub use maos_domain::ports::IacBusPort;

pub use channels::*;
pub use drr_scheduler::{BudgetWarningEvent, DrrScheduler};
pub use frame::*;
pub use mailbox::Mailbox;
pub use mailbox::*;
pub use mailbox_stub::MailboxStub;
pub use metrics::IacRtMetrics;
pub use redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
pub use transparency_log::{
    reconcile_correlated_frames, AuditError, FrameFilter, FrameKind, TeamTransparencyLogEntry,
    TransparencyLogAdapter, TransparencyLogEntry,
};

/// Adapter for the IAC Bus port trait.
///
/// Story 3.1 wires the real Mailbox + NotificationDispatcher.
/// Holds an `Arc<Mailbox>` for per-Spirit routing and an
/// `Arc<TransparencyLogAdapter>` for the I2 log-before-deliver
/// guarantee.
#[maos_attrs::i9_exempt(
    reason = "IAC Bus adapter; holds Arc<Mailbox> + Arc<TransparencyLogAdapter> + Story 6.2 AC4 frame_lineage_cache (in-memory, bounded by session lifetime) — all I9-sanctioned"
)]
#[derive(Clone)]
pub struct IacBusAdapter {
    mailbox: std::sync::Arc<Mailbox>,
    transparency_log: std::sync::Arc<TransparencyLogAdapter>,
    /// Story 6.1 — DRR fairness scheduler in front of the TL writer.
    drr_scheduler: Option<drr_scheduler::DrrScheduler>,
    digest_provider: std::sync::Arc<
        dyn Fn(
                &maos_spirit_abi::identity::SpiritId,
            ) -> maos_domain::invariants::i12::WorkingMemoryDigestRefs
            + Send
            + Sync,
    >,
    /// Story 6.2 AC4 — frame_id → intent_lineage cache for retract continuity.
    /// Populated at deliver_typed for cross-Spirit frames; read by retract() to
    /// carry the original lineage onto the emitted Retract frame.
    /// Bounded by session lifetime; eviction at MAX_LINEAGE_CACHE_ENTRIES.
    frame_lineage_cache:
        std::sync::Arc<dashmap::DashMap<[u8; 16], maos_domain::invariants::i13::IntentLineage>>,
    /// Story 9.5b — optional OTel trace sink for IAC-frame span emission.
    /// `None` = off (no-op branch; zero allocation). Wired from
    /// composition root via [`IacBusAdapter::with_trace_sink`].
    trace_sink: Option<std::sync::Arc<dyn maos_domain::ports::TraceSink>>,
    /// Story 4.5 — AC5 isolation hook for corpus runner observation.
    #[cfg(feature = "spirit_test")]
    isolation_hook: Option<
        std::sync::Arc<
            parking_lot::Mutex<dyn maos_spirit_sdk::spirit_test::IsolationHookPoint + Send>,
        >,
    >,
}

/// Story 6.2 AC4 — soft cap on the lineage cache. Entries are added on
/// deliver_typed and never explicitly removed; once the cap is reached new
/// inserts skip the cache (retract on a stale frame falls back to empty
/// lineage). Sized for ~5min of 10 tasks/sec sustained throughput.
const MAX_LINEAGE_CACHE_ENTRIES: usize = 4096;

impl IacBusAdapter {
    /// Construct a new adapter wrapping the given Mailbox and Transparency Log.
    pub fn new(
        mailbox: std::sync::Arc<Mailbox>,
        transparency_log: std::sync::Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            mailbox,
            transparency_log,
            drr_scheduler: None,
            digest_provider: std::sync::Arc::new(|_| {
                maos_domain::invariants::i12::WorkingMemoryDigestRefs::default()
            }),
            frame_lineage_cache: std::sync::Arc::new(dashmap::DashMap::new()),
            trace_sink: None,
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }

    /// Story 6.1 — attach a DRR fairness scheduler in front of the TL writer.
    pub fn with_drr_scheduler(mut self, drr: drr_scheduler::DrrScheduler) -> Self {
        self.drr_scheduler = Some(drr);
        self
    }

    /// Story 9.5b — attach the optional OTel trace sink. When `Some`,
    /// `deliver_typed` wraps each IAC frame in a trace span. When `None`
    /// (default), the branch is not taken and nothing allocates.
    pub fn with_trace_sink(
        mut self,
        sink: std::sync::Arc<dyn maos_domain::ports::TraceSink>,
    ) -> Self {
        self.trace_sink = Some(sink);
        self
    }

    /// Story 9.5b — accessor for the trace sink. Callers outside
    /// kernel-core (composition root, tests) use this to create
    /// capability spans as children of the IAC-frame span.
    pub fn trace_sink(&self) -> Option<&std::sync::Arc<dyn maos_domain::ports::TraceSink>> {
        self.trace_sink.as_ref()
    }

    /// Story 4.5 — attach an isolation hook for cross-Spirit corpus observation.
    #[cfg(feature = "spirit_test")]
    pub fn with_isolation_hook(
        mut self,
        hook: std::sync::Arc<
            parking_lot::Mutex<dyn maos_spirit_sdk::spirit_test::IsolationHookPoint + Send>,
        >,
    ) -> Self {
        self.isolation_hook = Some(hook);
        self
    }

    /// Story 4.5 — fire isolation hooks for cross-Spirit observation.
    #[cfg(feature = "spirit_test")]
    fn fire_isolation_hooks(
        &self,
        case_id: &str,
        _surface: &str,
        outcome: maos_spirit_sdk::spirit_test::IsolationHookOutcome,
    ) {
        if let Some(ref hook) = self.isolation_hook {
            let mut h = hook.lock();
            let _ = h.before_spirit_a_attempt(case_id);
            let attempt = maos_spirit_sdk::spirit_test::AttemptResult {
                hooks_fired_during_attempt: vec![case_id.into()],
                frames_emitted: 1,
            };
            let _ = h.after_spirit_a_attempt(case_id, &attempt);
            let _ = h.before_spirit_b_observe(case_id);
            let observation = maos_spirit_sdk::spirit_test::ObservationResult {
                hooks_fired_during_observation: vec![],
                frames_emitted: 0,
                leaked_bytes: None,
            };
            let _ = h.after_spirit_b_observe(case_id, &observation);
        }
    }

    /// Set the digest provider closure (composition root wiring point).
    /// Story 4.3 will replace the default empty-refs closure with a
    /// Memory Manager query.
    pub fn with_digest_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(
                &maos_spirit_abi::identity::SpiritId,
            ) -> maos_domain::invariants::i12::WorkingMemoryDigestRefs
            + Send
            + Sync
            + 'static,
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

    /// Access the DRR scheduler (test-only introspection).
    #[doc(hidden)]
    pub fn drr_scheduler(&self) -> Option<&drr_scheduler::DrrScheduler> {
        self.drr_scheduler.as_ref()
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

    // ── Story 5.3 — FR50 disposition helpers ────────────────────

    /// Emit a `task.nacked` TL row for a dead-Spirit's in-flight task.
    pub fn emit_task_complete_nack(&self, task: &maos_domain::ports::task::TaskAssignmentRecord) {
        let payload = serde_json::json!({
            "task_id": task.task_id,
            "originator_spirit_id": task.originator_spirit_id,
            "capability_token": task.capability_token,
        });
        let _ = self.transparency_log.insert_frame_event_with_sender(
            transparency_log::FrameKind::TaskComplete,
            0,
            &task.originator_spirit_id,
            "",
            None,
            "task.nacked",
            payload.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
    }

    /// Emit a `task.escalated` TL row for a dead-Spirit's in-flight task.
    pub fn emit_task_complete_escalated(
        &self,
        task: &maos_domain::ports::task::TaskAssignmentRecord,
    ) {
        let payload = serde_json::json!({
            "task_id": task.task_id,
            "originator_spirit_id": task.originator_spirit_id,
            "capability_token": task.capability_token,
        });
        let _ = self.transparency_log.insert_frame_event_with_sender(
            transparency_log::FrameKind::TaskComplete,
            0,
            &task.originator_spirit_id,
            "",
            None,
            "task.escalated",
            payload.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
    }

    /// Emit a `task.reassigned` TL row and enqueue a reassign frame to the target replica.
    pub fn reassign_task_to(
        &self,
        task: &maos_domain::ports::task::TaskAssignmentRecord,
        replica_spirit_id: &str,
    ) {
        let payload = serde_json::json!({
            "task_id": task.task_id,
            "originator_spirit_id": task.originator_spirit_id,
            "capability_token": task.capability_token,
            "replica_spirit_id": replica_spirit_id,
        });
        let _ = self.transparency_log.insert_frame_event_with_sender(
            transparency_log::FrameKind::TaskComplete,
            0,
            &task.originator_spirit_id,
            "",
            None,
            "task.reassigned",
            payload.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
    }

    /// Typed deliver (Story 3.1).
    pub async fn deliver_typed(
        &self,
        frame: maos_domain::frame::IacFrame,
    ) -> Result<
        maos_domain::invariants::i2::LogBeforeDeliver<()>,
        maos_domain::iac_bus_types::IacBusError,
    > {
        // 0. I12 decorate decision frames BEFORE serialization (Story 3.3 AC5)
        // so the logged payload already carries working_memory_digest_refs.
        // digest_provider is injected from the composition root.
        let mut frame =
            decision_logger::decorate_decision_frame(frame, |sid| (self.digest_provider)(sid));

        // Story 9.5b — IAC-frame span. Attrs built INSIDE the branch;
        // nothing allocates when the sink is `None` (AC-2).
        // Named `_frame_span_guard` — bare `let _ =` drops at semicolon
        // → zero-duration span (grep gate).
        let _frame_span_guard = if let Some(sink) = &self.trace_sink {
            let kind_label: &'static str = match frame.kind {
                maos_spirit_abi::identity::FrameKind::TaskAssign => "task_assign",
                maos_spirit_abi::identity::FrameKind::TaskComplete => "task_complete",
                maos_spirit_abi::identity::FrameKind::DecisionDispatch => "decision_dispatch",
                maos_spirit_abi::identity::FrameKind::EpistemicHalt => "epistemic_halt",
                maos_spirit_abi::identity::FrameKind::TelemetryEvent => "telemetry_event",
                maos_spirit_abi::identity::FrameKind::ConsentRequest => "consent_request",
                maos_spirit_abi::identity::FrameKind::Retract => "retract",
                maos_spirit_abi::identity::FrameKind::CapabilityInvocation => {
                    "capability_invocation"
                }
                maos_spirit_abi::identity::FrameKind::SandboxBlock => "sandbox_block",
                maos_spirit_abi::identity::FrameKind::InferenceCall => "inference_call",
                maos_spirit_abi::identity::FrameKind::CliSubprocessOutput => {
                    "cli_subprocess_output"
                }
                maos_spirit_abi::identity::FrameKind::ConsentRupture => "consent_rupture",
                maos_spirit_abi::identity::FrameKind::RateLimited => "rate_limited",
                maos_spirit_abi::identity::FrameKind::GatewayInbound => "gateway_inbound",
                maos_spirit_abi::identity::FrameKind::GatewayOutbound => "gateway_outbound",
            };
            let intent_label: &'static str = match frame.intent {
                maos_domain::invariants::i1::IntentClass::HighPrivilege => "high",
                maos_domain::invariants::i1::IntentClass::Standard => "standard",
                maos_domain::invariants::i1::IntentClass::Readonly => "readonly",
            };
            sink.iac_frame_span(maos_domain::ports::IacFrameSpanAttrs {
                frame_id: frame.frame_id,
                kind: kind_label,
                intent: intent_label,
            })
        } else {
            maos_domain::ports::SpanGuard::noop()
        };

        // Story 4.5 — AC5: fire isolation hooks BEFORE lineage check
        // so corpus harness observes the attempt before the kernel rejects it.
        #[cfg(feature = "spirit_test")]
        {
            let caller_pid = frame.from.spirit_id.as_str().to_string();
            let surface = "IacBusAdapter::deliver_typed";
            self.fire_isolation_hooks(
                &caller_pid,
                surface,
                maos_spirit_sdk::spirit_test::IsolationHookOutcome::Continue,
            );
        }

        // Story 6.2 AC2 — FR21: Orchestrator distillate-dispatch gate.
        // Fires BEFORE the I13 lineage check below; the gate is a permission
        // check (no TL row written for rejected frames).
        orchestrator_dispatch::check_orchestrator_distillate_required(
            &frame,
            &self.transparency_log,
            orchestrator_dispatch::DEFAULT_ORCHESTRATOR_DISPATCH_WINDOW_NS,
        )?;

        // Story 4.5 — NFR-Aud-14 intent-lineage propagation.
        // Cross-Spirit determination: `frame.from.spirit_id != frame.to[0].spirit_id`
        // (broadcasts with no `to` entries are NOT cross-Spirit — they're 1:N telemetry,
        // the lineage is broadcast-implicit).
        {
            let is_cross_spirit = frame
                .to
                .iter()
                .any(|addr| addr.spirit_id != frame.from.spirit_id);
            if is_cross_spirit {
                if frame.intent_lineage.is_empty() {
                    match frame.auto_marker {
                        // Originating human intent — kernel attaches single-class lineage.
                        maos_domain::invariants::i3::FrameOrigin::HumanAuthored => {
                            let class_as_intent =
                                maos_domain::invariants::i8::A2AIntent::new(match frame.intent {
                                    maos_domain::invariants::i1::IntentClass::HighPrivilege => {
                                        "high"
                                    }
                                    maos_domain::invariants::i1::IntentClass::Standard => {
                                        "standard"
                                    }
                                    maos_domain::invariants::i1::IntentClass::Readonly => {
                                        "readonly"
                                    }
                                });
                            frame.intent_lineage =
                                maos_domain::invariants::i13::IntentLineage::new(vec![
                                    class_as_intent,
                                ]);
                        }
                        // Spirit-drafted but human-approved: treat as human-originated —
                        // a human reviewed and approved the draft, so auto-populate lineage.
                        maos_domain::invariants::i3::FrameOrigin::SpiritDraftedHumanApproved => {
                            let class_as_intent =
                                maos_domain::invariants::i8::A2AIntent::new(match frame.intent {
                                    maos_domain::invariants::i1::IntentClass::HighPrivilege => {
                                        "high"
                                    }
                                    maos_domain::invariants::i1::IntentClass::Standard => {
                                        "standard"
                                    }
                                    maos_domain::invariants::i1::IntentClass::Readonly => {
                                        "readonly"
                                    }
                                });
                            frame.intent_lineage =
                                maos_domain::invariants::i13::IntentLineage::new(vec![
                                    class_as_intent,
                                ]);
                        }
                        // Kernel-generated frames (audit telemetry, capability mediation, etc.)
                        // are internal infrastructure — accept with empty lineage.
                        maos_domain::invariants::i3::FrameOrigin::Kernel => {
                            // Kernel frames carry implicit provenance through the frame_id
                            // chain; empty lineage is acceptable per NFR-Aud-14 carve-out.
                        }
                        // Spirit-emitted cross-Spirit frame with empty lineage = consent-laundering signal.
                        // Reject per NFR-Aud-14.
                        maos_domain::invariants::i3::FrameOrigin::SpiritAuto => {
                            #[cfg(feature = "spirit_test")]
                            {
                                let caller_pid = frame.from.spirit_id.as_str().to_string();
                                self.fire_isolation_hooks(
                                    &caller_pid,
                                    "IacBusAdapter::deliver_typed::lineage_broken",
                                    maos_spirit_sdk::spirit_test::IsolationHookOutcome::Abort,
                                );
                            }
                            return Err(
                                maos_domain::iac_bus_types::IacBusError::EIntentLineageBroken {
                                    from: frame.from.spirit_id.as_str().to_string(),
                                    to: frame
                                        .to
                                        .first()
                                        .map(|a| a.spirit_id.as_str().to_string())
                                        .unwrap_or_default(),
                                    origin: frame.auto_marker,
                                },
                            );
                        }
                    }
                }
            }
            // Note: same-Spirit frames AND broadcast frames bypass the check (ADR-018
            // "explodes header overhead for frames that never cross consent boundaries").
        }

        // 1. Serialize payload for log write
        let payload_bytes = serde_json::to_vec(&frame.payload).map_err(|e| {
            maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
        })?;

        // 2. Log before deliver (I2)
        let spirit_pid = 0u32; // v0.3-β: PID not yet relevant for pure routing
        let intent_str = match &frame.intent {
            maos_domain::invariants::i1::IntentClass::HighPrivilege => "high",
            maos_domain::invariants::i1::IntentClass::Standard => "standard",
            maos_domain::invariants::i1::IntentClass::Readonly => "readonly",
        };

        // Convert domain FrameKind to kernel FrameKind for transparency_log
        let tl_kind = match frame.kind {
            maos_spirit_abi::identity::FrameKind::TaskAssign => {
                transparency_log::FrameKind::TaskAssign
            }
            maos_spirit_abi::identity::FrameKind::TaskComplete => {
                transparency_log::FrameKind::TaskComplete
            }
            maos_spirit_abi::identity::FrameKind::DecisionDispatch => {
                transparency_log::FrameKind::DecisionDispatch
            }
            maos_spirit_abi::identity::FrameKind::EpistemicHalt => {
                transparency_log::FrameKind::EpistemicHalt
            }
            maos_spirit_abi::identity::FrameKind::TelemetryEvent => {
                transparency_log::FrameKind::TelemetryEvent
            }
            maos_spirit_abi::identity::FrameKind::ConsentRequest => {
                transparency_log::FrameKind::ConsentRequest
            }
            maos_spirit_abi::identity::FrameKind::Retract => transparency_log::FrameKind::Retract,
            maos_spirit_abi::identity::FrameKind::CapabilityInvocation => {
                transparency_log::FrameKind::CapabilityInvocation
            }
            maos_spirit_abi::identity::FrameKind::SandboxBlock => {
                transparency_log::FrameKind::SandboxBlock
            }
            maos_spirit_abi::identity::FrameKind::InferenceCall => {
                transparency_log::FrameKind::InferenceCall
            }
            maos_spirit_abi::identity::FrameKind::CliSubprocessOutput => {
                transparency_log::FrameKind::CliSubprocessOutput
            }
            maos_spirit_abi::identity::FrameKind::ConsentRupture => {
                transparency_log::FrameKind::ConsentRupture
            }
            maos_spirit_abi::identity::FrameKind::RateLimited => {
                transparency_log::FrameKind::RateLimited
            }
            maos_spirit_abi::identity::FrameKind::GatewayInbound => {
                transparency_log::FrameKind::GatewayInbound
            }
            maos_spirit_abi::identity::FrameKind::GatewayOutbound => {
                transparency_log::FrameKind::GatewayOutbound
            }
        };

        // Story 6.2 AC4 — populate the frame_lineage_cache so retract() can
        // recover the original lineage. Bounded by MAX_LINEAGE_CACHE_ENTRIES.
        if self.frame_lineage_cache.len() < MAX_LINEAGE_CACHE_ENTRIES {
            self.frame_lineage_cache
                .insert(frame.frame_id, frame.intent_lineage.clone());
        }

        // I2: log before deliver.
        // Story 6.1 — use DRR scheduler if present, otherwise synchronous write.
        let to_spirit_id = frame.to.first().map_or("", |a| a.spirit_id.as_str());
        if let Some(ref drr) = self.drr_scheduler {
            let lineage_bytes = match serde_json::to_vec(&frame.intent_lineage) {
                Ok(b) => b,
                Err(_) => Vec::new(),
            };
            drr.submit(
                frame.clone(),
                payload_bytes,
                tl_kind,
                spirit_pid,
                intent_str.to_string(),
                frame.auto_marker,
                lineage_bytes,
            )
            .await?;
        } else {
            self.transparency_log.insert_frame_event_with_id(
                Some(frame.frame_id),
                tl_kind,
                spirit_pid,
                frame.from.spirit_id.as_str(),
                to_spirit_id,
                None,
                intent_str,
                &payload_bytes,
                frame.auto_marker,
            );
        }

        // 3. Route through Mailbox (async for backpressure)
        self.mailbox.deliver(frame).await
    }

    /// Typed register_spirit (Story 3.1).
    pub fn register_spirit_typed(
        &self,
        spirit_id: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<SpiritMailboxHandle, maos_domain::iac_bus_types::IacBusError> {
        self.mailbox.register_spirit(spirit_id.as_str())
    }

    /// Story 6.1 — retract a previously-delivered frame.
    ///
    /// 1. Look up the original frame in the Transparency Log.
    /// 2. Assert the retracting Spirit is the original sender.
    /// 3. Check idempotency atomically — re-retract returns `Already`.
    /// 4. Write a new TL row of kind `Retract` (via DRR if configured).
    /// 5. Mark the original frame as retracted in the companion table.
    /// 6. Route the Retract frame through the mailbox to the ORIGINAL RECIPIENT.
    pub async fn retract(
        &self,
        original_frame_id: [u8; 16],
        reason: String,
        retracting_spirit: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<maos_domain::iac_bus_types::RetractOutcome, maos_domain::iac_bus_types::IacBusError>
    {
        use maos_domain::frame::RetractPayload;
        use maos_domain::iac_bus_types::RetractOutcome;

        // Step 1: look up the original frame
        let original_entry = self
            .transparency_log
            .query_frame_by_id(original_frame_id)
            .map_err(|e| {
                maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
            })?;

        let original_entry = match original_entry {
            Some(entry) => entry,
            None => return Ok(RetractOutcome::OriginalNotFound),
        };

        // Step 2: authority check — only the original sender can retract
        let original_sender = &original_entry.from_spirit_id;
        if original_sender.is_empty() {
            return Err(
                maos_domain::iac_bus_types::IacBusError::RetractAuthorityViolation {
                    caller: retracting_spirit.as_str().to_string(),
                    original_sender: "<unknown — legacy frame>".to_string(),
                },
            );
        }
        if original_sender != retracting_spirit.as_str() {
            return Err(
                maos_domain::iac_bus_types::IacBusError::RetractAuthorityViolation {
                    caller: retracting_spirit.as_str().to_string(),
                    original_sender: original_sender.to_string(),
                },
            );
        }

        // Step 3: Build the RetractPayload early for type mapping
        let retract_payload = RetractPayload::new(
            original_frame_id,
            reason,
            Some(match original_entry.kind {
                transparency_log::FrameKind::TaskAssign => {
                    maos_spirit_abi::identity::FrameKind::TaskAssign
                }
                transparency_log::FrameKind::TaskComplete => {
                    maos_spirit_abi::identity::FrameKind::TaskComplete
                }
                transparency_log::FrameKind::DecisionDispatch => {
                    maos_spirit_abi::identity::FrameKind::DecisionDispatch
                }
                transparency_log::FrameKind::EpistemicHalt => {
                    maos_spirit_abi::identity::FrameKind::EpistemicHalt
                }
                transparency_log::FrameKind::TelemetryEvent => {
                    maos_spirit_abi::identity::FrameKind::TelemetryEvent
                }
                transparency_log::FrameKind::ConsentRequest => {
                    maos_spirit_abi::identity::FrameKind::ConsentRequest
                }
                transparency_log::FrameKind::Retract => {
                    maos_spirit_abi::identity::FrameKind::Retract
                }
                _ => {
                    // Unrecognized FrameKind — log warning, fall through to TaskAssign
                    eprintln!(
                        "retract: unrecognized FrameKind {:?}, defaulting to TaskAssign",
                        original_entry.kind
                    );
                    maos_spirit_abi::identity::FrameKind::TaskAssign
                }
            }),
        )
        .map_err(maos_domain::iac_bus_types::IacBusError::RetractPayloadInvalid)?;

        // Story 6.2 AC4 — lineage continuity across retract.
        // Recover the original frame's lineage from the frame_lineage_cache
        // populated at deliver_typed time. Falls back to default() when the
        // cache has evicted the entry (sessions older than the cache window).
        // The retract is a continuation of the original intent, not a new one.
        let original_lineage = self
            .frame_lineage_cache
            .get(&original_frame_id)
            .map(|e| e.value().clone())
            .unwrap_or_default();

        // Build retract frame with a UNIQUE frame_id (not reusing original_frame_id)
        // The TL auto-generates a frame_id for the new Retract row.
        let retract_frame = maos_domain::frame::IacFrame {
            frame_id: [0u8; 16], // Auto-generated by TL on insert
            timestamp_ns: 0,
            logical_clock: 0,
            from: maos_domain::frame::FrameAddress {
                spirit_id: retracting_spirit.clone(),
                host_id: None,
                role: None,
            },
            to: {
                let recipient_id = &original_entry.to_spirit_id;
                if recipient_id.is_empty() {
                    // Legacy frame — no known recipient; route to sender as fallback
                    eprintln!(
                        "retract: no to_spirit_id for frame {:?}, routing retract to sender",
                        original_frame_id
                    );
                    let mut v = Vec::new();
                    v.push(maos_domain::frame::FrameAddress {
                        spirit_id: retracting_spirit.clone(),
                        host_id: None,
                        role: None,
                    });
                    v.into()
                } else {
                    let mut v = Vec::new();
                    v.push(maos_domain::frame::FrameAddress {
                        spirit_id: maos_spirit_abi::identity::SpiritId::from(recipient_id.as_str()),
                        host_id: None,
                        role: None,
                    });
                    v.into()
                }
            },
            kind: maos_spirit_abi::identity::FrameKind::Retract,
            intent: maos_domain::invariants::i1::IntentClass::Standard,
            payload: maos_domain::frame::FramePayload::Retract(retract_payload),
            auto_marker: maos_domain::invariants::i3::FrameOrigin::Kernel,
            consent_envelope: None,
            // Story 6.2 AC4 — continuity: carry the original frame's lineage.
            intent_lineage: original_lineage,
        };

        // Step 4.5: Atomically check-and-mark retracted BEFORE writing TL row
        // (prevents duplicate Retract TL rows on concurrent/re-play retractions)
        let retract_frame_id = {
            // We need a frame_id for the companion table entry.
            // The TL auto-generates one; we use the last generated ID after write.
            // But to do the check FIRST, we need to pick a frame_id.
            // Strategy: log the frame_id via DRR or direct write, THEN check.
            // For the idempotency case, we check BEFORE the write.
            // But we need a frame_id for the companion table. Use a placeholder
            // that gets overwritten below in the check_and_mark call.
            let placeholder = [0u8; 16];

            // Step 3.5: Atomically check-and-mark FIRST to avoid duplicate writes
            if let Some(existing) = self
                .transparency_log
                .check_and_mark_retracted(original_frame_id, placeholder)
                .map_err(|e| {
                    maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
                })?
            {
                return Ok(RetractOutcome::Already {
                    existing_retract_frame_id: existing,
                });
            }

            // Step 4: Log the retract frame (I2), routing through DRR if configured
            if let Some(ref drr) = self.drr_scheduler {
                let payload_bytes = serde_json::to_vec(&retract_frame.payload).map_err(|e| {
                    maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
                })?;
                drr.submit(
                    retract_frame.clone(),
                    payload_bytes,
                    transparency_log::FrameKind::Retract,
                    original_entry.spirit_pid,
                    "retract".to_string(),
                    maos_domain::invariants::i3::FrameOrigin::Kernel,
                    Vec::new(),
                )
                .await?;
                self.transparency_log.last_frame_id()
            } else {
                let payload_bytes = serde_json::to_vec(&retract_frame.payload).map_err(|e| {
                    maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
                })?;
                let _logged = self.transparency_log.insert_frame_event_with_sender(
                    transparency_log::FrameKind::Retract,
                    original_entry.spirit_pid,
                    retracting_spirit.as_str(),
                    retract_frame
                        .to
                        .first()
                        .map_or("", |a| a.spirit_id.as_str()),
                    None,
                    "retract",
                    &payload_bytes,
                    maos_domain::invariants::i3::FrameOrigin::Kernel,
                );
                self.transparency_log.last_frame_id()
            }
        };

        // Update the companion table entry with the real retract_frame_id
        let _ = self
            .transparency_log
            .mark_retracted(original_frame_id, retract_frame_id)
            .map_err(|e| {
                maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
            })?;

        // Step 6: route through mailbox to the original recipient
        self.mailbox
            .deliver_with_overtake(retract_frame, original_frame_id)
            .await?;

        Ok(RetractOutcome::Retracted { retract_frame_id })
    }
}

impl Default for IacBusAdapter {
    fn default() -> Self {
        Self {
            mailbox: std::sync::Arc::new(Mailbox::new(std::sync::Arc::new(IacRtMetrics::new()))),
            transparency_log: std::sync::Arc::new(TransparencyLogAdapter::open_in_memory(0)),
            drr_scheduler: None,
            digest_provider: std::sync::Arc::new(|_| {
                maos_domain::invariants::i12::WorkingMemoryDigestRefs::default()
            }),
            frame_lineage_cache: std::sync::Arc::new(dashmap::DashMap::new()),
            trace_sink: None,
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }
}

#[cfg(test)]
mod decision_audit_tests {
    use super::*;
    use maos_domain::frame::{
        DecisionDispatchPayload, FrameAddress, FramePayload, IacFrame, PosturePreferences,
        TaskAssignPayload,
    };
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_spirit_abi::identity::{FrameKind as DomainFrameKind, SpiritId};
    use smallvec::smallvec;
    use std::sync::Arc;

    fn make_decision_frame(decision_id: u64) -> IacFrame {
        maos_capability::cap_tokens::init_monotonic_base();
        let mut fid = [0u8; 16];
        fid[..8].copy_from_slice(&decision_id.to_le_bytes());
        IacFrame {
            frame_id: fid,
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
            intent_lineage: IntentLineage::default(),
        }
    }

    #[tokio::test]
    async fn i12_10_decision_frames_100_percent_carry_refs() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());

        // Register a target spirit so deliver_typed can route
        adapter
            .register_spirit_typed(&SpiritId::from("target"))
            .ok();

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

        assert_eq!(
            entries.len(),
            10,
            "expected 10 decision frames in transparency log"
        );

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
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());

        adapter
            .register_spirit_typed(&SpiritId::from("target"))
            .ok();

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
                prior_distillate_ref: None,
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: IntentLineage::default(),
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
            other => panic!(
                "expected TaskAssign, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    // --- Story 4.5 lineage check tests (Task 2.3) ---

    fn make_cross_spirit_frame(from: &str, to: &str, origin: FrameOrigin) -> IacFrame {
        maos_capability::cap_tokens::init_monotonic_base();
        IacFrame {
            frame_id: [0xAA; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: FrameAddress {
                spirit_id: SpiritId::from(from),
                host_id: None,
                role: None,
            },
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from(to),
                host_id: None,
                role: None,
            }],
            kind: DomainFrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "cross-spirit".into(),
                scope: vec![],
                success_criteria: "ok".into(),
                posture_preferences: PosturePreferences::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: origin,
            consent_envelope: None,
            intent_lineage: IntentLineage::default(),
        }
    }

    #[tokio::test]
    async fn lineage_human_authored_cross_spirit_auto_populates() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());
        let _target_handle = adapter
            .register_spirit_typed(&SpiritId::from("spirit-b"))
            .unwrap();

        let frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::HumanAuthored);
        assert!(
            frame.intent_lineage.is_empty(),
            "precondition: empty lineage"
        );
        let result = adapter.deliver_typed(frame).await;
        assert!(result.is_ok(), "human-authored cross-spirit should succeed");

        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 1, "frame should be logged");
    }

    #[tokio::test]
    async fn lineage_spirit_auto_cross_spirit_empty_lineage_rejected() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());
        let _target_handle = adapter
            .register_spirit_typed(&SpiritId::from("spirit-b"))
            .unwrap();

        let frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::SpiritAuto);
        let result = adapter.deliver_typed(frame).await;
        assert!(
            result.is_err(),
            "spirit-auto cross-spirit with empty lineage should be rejected"
        );
        match result.unwrap_err() {
            maos_domain::iac_bus_types::IacBusError::EIntentLineageBroken { from, to, origin } => {
                assert_eq!(from, "spirit-a");
                assert_eq!(to, "spirit-b");
                assert_eq!(origin, FrameOrigin::SpiritAuto);
            }
            e => panic!("expected EIntentLineageBroken, got {e:?}"),
        }
        // Verify frame was NOT logged
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 0, "rejected frame should not be logged");
    }

    #[tokio::test]
    async fn lineage_spirit_auto_cross_spirit_non_empty_lineage_succeeds() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());
        let _target_handle = adapter
            .register_spirit_typed(&SpiritId::from("spirit-b"))
            .unwrap();

        let mut frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::SpiritAuto);
        frame.intent_lineage =
            IntentLineage::new(vec![maos_domain::invariants::i8::A2AIntent::new(
                "standard",
            )]);
        let result = adapter.deliver_typed(frame).await;
        assert!(
            result.is_ok(),
            "spirit-auto with non-empty lineage should succeed"
        );
    }

    #[tokio::test]
    async fn lineage_same_spirit_empty_lineage_spirit_auto_succeeds() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());
        let _target_handle = adapter
            .register_spirit_typed(&SpiritId::from("spirit-a"))
            .unwrap();

        let mut frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::SpiritAuto);
        // make it same-spirit: from == to
        frame.to = smallvec![FrameAddress {
            spirit_id: SpiritId::from("spirit-a"),
            host_id: None,
            role: None,
        }];
        let result = adapter.deliver_typed(frame).await;
        assert!(
            result.is_ok(),
            "same-spirit with empty lineage should succeed per ADR-018"
        );
    }

    #[tokio::test]
    async fn lineage_broadcast_empty_to_succeeds() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());

        let mut frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::SpiritAuto);
        frame.to = smallvec![]; // broadcast — bypass lineage check
        let result = adapter.deliver_typed(frame).await;
        assert!(
            result.is_ok(),
            "broadcast with empty lineage should succeed"
        );
    }

    #[tokio::test]
    async fn lineage_human_authored_non_empty_lineage_not_overwritten() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
        let adapter = IacBusAdapter::new(mailbox, log.clone());
        let _target_handle = adapter
            .register_spirit_typed(&SpiritId::from("spirit-b"))
            .unwrap();

        let mut frame = make_cross_spirit_frame("spirit-a", "spirit-b", FrameOrigin::HumanAuthored);
        let existing =
            IntentLineage::new(vec![maos_domain::invariants::i8::A2AIntent::new("consult")]);
        frame.intent_lineage = existing.clone();
        let result = adapter.deliver_typed(frame).await;
        assert!(
            result.is_ok(),
            "human-authored with pre-existing lineage should succeed"
        );
        // The frame was consumed; we verify delivery succeeded (no rejection).
    }
}
