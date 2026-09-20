#![forbid(unsafe_code)]

//! Spirit Scheduler — supervisor body. Replaces the v0.1-β zero-size
//! placeholder with the real implementation.
//!
//! Architecture §4.1 + Story 5.1 Task 4.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::lifecycle::LifecycleError;
use maos_domain::ports::scheduler::SpiritSchedulerPort;

use crate::halt::terminate_spirit;
use crate::iac::transparency_log::TransparencyLogAdapter;
use crate::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use crate::scheduler::hook_dispatch::{HookDispatcher, HookOutcome};
use maos_domain::halt::TerminationKind;
use maos_domain::invariants::i3::FrameOrigin;
use maos_spirit_abi::lifecycle::Spirit;

/// Quantum size for deficit round-robin scheduling.
pub const SCHEDULER_QUANTUM: u32 = 64;

static NEXT_SPIRIT_PID: AtomicU32 = AtomicU32::new(1);

pub(crate) fn allocate_pid() -> u32 {
    NEXT_SPIRIT_PID.fetch_add(1, Ordering::Relaxed)
}

/// Stand-alone DRR picker — operates on a slice of SCBs so integration
/// tests can exercise fairness without constructing a full adapter.
///
/// Returns the `pid` of the chosen Spirit and deducts `SCHEDULER_QUANTUM`
/// from its deficit counter.  The caller must ensure the work-unit is
/// actually dispatched to that Spirit.
pub fn pick_next_spirit_from_slice(scbs: &[Arc<SpiritControlBlock>]) -> Option<u32> {
    // Pass 1: increment all Running deficits by weight.
    for scb in scbs {
        if scb.current_state() != ScbLifecycleState::Running {
            continue;
        }
        let weight = scb.runtime_snapshot().priority_weight as u32;
        scb.deficit_counter.fetch_add(weight, Ordering::SeqCst);
    }

    // Pass 2: find the best candidate (highest deficit ≥ quantum).
    let mut best: Option<u32> = None;
    let mut best_deficit = 0;
    for scb in scbs {
        if scb.current_state() != ScbLifecycleState::Running {
            continue;
        }
        let deficit = scb.deficit_counter.load(Ordering::SeqCst);
        if deficit >= SCHEDULER_QUANTUM && deficit > best_deficit {
            best = Some(scb.pid);
            best_deficit = deficit;
        }
    }
    let winner = best?;
    let scb = scbs.iter().find(|scb| scb.pid == winner)?;
    // Selection and charge are one operation.  A concurrent picker cannot
    // underflow the deficit or return a winner without paying its quantum.
    scb.deficit_counter
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |deficit| {
            deficit.checked_sub(SCHEDULER_QUANTUM)
        })
        .ok()
        .map(|_| winner)
}

/// The Spirit Scheduler — kernel-side supervisor.
#[maos_attrs::i9_exempt(
    reason = "kernel-side Spirit scheduler / supervisor; owns the supervised Spirit-control-block set under the supervision tree (P3 supervisor-exception per §4.0.8) — structural-state per I9, keyed by spirit_pid (Story 7.1.7 baseline-reset)"
)]
pub struct SpiritSchedulerAdapter {
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    tl: Arc<TransparencyLogAdapter>,
    dispatcher: Arc<HookDispatcher>,
    capability: Arc<crate::capability::CapabilityRegistryAdapter>,
    _memory: Arc<crate::memory::MemoryManagerAdapter>,
    _iac: Arc<crate::iac::IacBusAdapter>,
    _halt_registry: Arc<crate::halt::HaltRegistry>,
    security_manager: Option<Arc<crate::security::SecurityManagerAdapter>>,
    orchestrator_registry: Option<Arc<crate::orchestrator::OrchestratorBufferRegistry>>,
    crash_detector: Option<Arc<crate::supervision::CrashDetector>>,
    /// Resolver used to route FR50 disposition after an unload-hook failure;
    /// late-bound at the composition root with CrashDetector.
    replica: Option<Arc<dyn maos_domain::supervision::ReplicaResolver>>,
    /// Shared validated CRL rules. When configured by the revocation
    /// composition root, admission and CRL application serialize on its gate.
    revocation_rules: Arc<RwLock<Option<Arc<crate::revocation::rules::ValidatedRevocationRules>>>>,
}

impl SpiritSchedulerAdapter {
    pub fn new(
        tl: Arc<TransparencyLogAdapter>,
        capability: Arc<crate::capability::CapabilityRegistryAdapter>,
        memory: Arc<crate::memory::MemoryManagerAdapter>,
        iac: Arc<crate::iac::IacBusAdapter>,
        halt_registry: Arc<crate::halt::HaltRegistry>,
        telemetry: Arc<crate::telemetry::iac_rt::IacRtMetrics>,
        working_memory_orchestrator: Option<
            Arc<crate::capability::working_memory::orchestrator::WorkingMemoryOrchestrator>,
        >,
        log_recall: Option<Arc<crate::iac::log_recall::LogRecallAdapter>>,
        distillate_writer: Option<Arc<crate::iac::distillate::DistillateWriter>>,
        self_telemetry: Option<Arc<crate::memory::self_telemetry::SelfTelemetryAggregator>>,
        security_manager: Option<Arc<crate::security::SecurityManagerAdapter>>,
        orchestrator_registry: Option<Arc<crate::orchestrator::OrchestratorBufferRegistry>>,
        crash_detector: Option<Arc<crate::supervision::CrashDetector>>,
    ) -> Self {
        let spirits = Arc::new(RwLock::new(BTreeMap::new()));
        let mut dispatcher = HookDispatcher::new(Arc::clone(&tl), Arc::clone(&telemetry))
            .with_memory_manager(Arc::clone(&memory))
            .with_capability(Arc::clone(&capability))
            .with_iac(Arc::clone(&iac))
            .with_halt_registry(Arc::clone(&halt_registry))
            .with_spirits(Arc::clone(&spirits));
        if let Some(ref o) = working_memory_orchestrator {
            dispatcher = dispatcher.with_working_memory_orchestrator(Arc::clone(o));
        }
        if let Some(ref l) = log_recall {
            dispatcher = dispatcher.with_log_recall(Arc::clone(l));
        }
        if let Some(ref d) = distillate_writer {
            dispatcher = dispatcher.with_distillate_writer(Arc::clone(d));
        }
        if let Some(ref s) = self_telemetry {
            dispatcher = dispatcher.with_self_telemetry(Arc::clone(s));
        }
        Self {
            spirits,
            dispatcher: Arc::new(dispatcher),
            tl,
            capability,
            _memory: memory,
            _iac: iac,
            _halt_registry: halt_registry,
            security_manager,
            orchestrator_registry,
            crash_detector,
            replica: None,
            revocation_rules: Arc::new(RwLock::new(None)),
        }
    }

    pub fn scbs(&self) -> Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>> {
        Arc::clone(&self.spirits)
    }

    pub fn dispatcher_ref(&self) -> &HookDispatcher {
        self.dispatcher.as_ref()
    }

    /// Story 5.3 — late-bind the CrashDetector after composition-root construction.
    pub fn set_crash_detector(&mut self, cd: Arc<crate::supervision::CrashDetector>) {
        self.crash_detector = Some(cd);
    }

    /// Story 16-6 — late-bind the resolver used by unload-hook disposition.
    /// Keeps constructor call sites stable alongside `set_crash_detector`.
    pub fn set_replica_resolver(
        &mut self,
        replica: Option<Arc<dyn maos_domain::supervision::ReplicaResolver>>,
    ) {
        self.replica = replica;
    }

    /// Shared dispatcher for the IdleWatchdog.
    pub fn dispatcher_arc(&self) -> Arc<HookDispatcher> {
        Arc::clone(&self.dispatcher)
    }

    /// Bind the validated CRL store once at the composition root.  The
    /// revocation applier installs this before any daemon admission occurs.
    pub(crate) fn set_revocation_rules(
        &self,
        rules: Arc<crate::revocation::rules::ValidatedRevocationRules>,
    ) {
        *self
            .revocation_rules
            .write()
            .expect("revocation rules binding lock poisoned") = Some(rules);
    }

    pub fn resolve_pid(&self, spirit_id: &str) -> Option<u32> {
        let spirits = self.spirits.read().unwrap();
        for (pid, scb) in spirits.iter() {
            if scb.spirit_id == spirit_id {
                return Some(*pid);
            }
        }
        None
    }

    /// Read an exact sandbox report from the live SCB.  No report means that
    /// this Spirit has not completed a concrete sandbox application.
    pub fn sandbox_report_by_spirit_id(
        &self,
        spirit_id: &str,
    ) -> Option<maos_domain::sandbox::SandboxInspectReport> {
        self.spirits
            .read()
            .unwrap()
            .values()
            .find(|scb| scb.spirit_id == spirit_id)
            .and_then(|scb| scb.sandbox_report())
    }

    /// Attach the concrete spawn/admission report to its existing live SCB.
    /// Returns false instead of fabricating state when the Spirit is absent.
    pub fn record_sandbox_report(
        &self,
        report: maos_domain::sandbox::SandboxInspectReport,
    ) -> bool {
        let spirits = self.spirits.read().unwrap();
        let Some(scb) = spirits
            .values()
            .find(|scb| scb.spirit_id == report.spirit_id)
        else {
            return false;
        };
        scb.record_sandbox_report(report);
        true
    }

    /// Commit a concrete sandbox application to the live SCB and lifecycle
    /// journal.  The payload is the exact report served by operator HTTP.
    pub fn record_sandbox_application(
        &self,
        report: maos_domain::sandbox::SandboxInspectReport,
        journal: &dyn SpiritSchedulerPort,
    ) -> Result<(), String> {
        let spirit_id = report.spirit_id.clone();
        let payload = serde_json::to_vec(&report)
            .map_err(|error| format!("serialize sandbox report: {error}"))?;
        if !self.record_sandbox_report(report) {
            return Err(format!("sandbox report SCB not found for {spirit_id}"));
        }
        journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
            lifecycle_event: LifecycleEvent::SandboxApplied,
            spirit_id,
            payload: Some(payload),
            effective_sandbox_tier: Some(maos_domain::invariants::i9::SandboxTier::T3),
        }));
        Ok(())
    }

    /// Load a Spirit: create SCB, insert into map, fire on_load.
    ///
    /// Monomorphising wrapper over [`Self::load_obj`]. Behaviour is
    /// unchanged for every existing caller.
    pub async fn load<T: Spirit + Send + Sync + 'static>(
        &self,
        spirit_id: &str,
        manifest: SpiritManifestBundle,
        spirit: T,
        boot_nonce: u64,
    ) -> Result<u32, LifecycleError> {
        self.load_obj(spirit_id, manifest, make_spirit_obj(spirit), boot_nonce)
            .await
    }

    /// Story 16-6 — the type-erased load primitive.
    ///
    /// `load` is generic over a compile-time `T: Spirit`, so a caller that
    /// has already performed class dispatch (and therefore holds an
    /// `Arc<dyn AnySpiritObj>`, the shape `SuccessorSpiritFactory` and the
    /// hot-swap coordinator already speak) could not reach it without a
    /// second, per-class copy of the whole admission triple. This entry point
    /// is what lets `maos-bin` hold ONE extracted `load → admit → start`
    /// function shared by `maos run`'s two file-manifest arms and the
    /// operator door.
    pub async fn load_obj(
        &self,
        spirit_id: &str,
        manifest: SpiritManifestBundle,
        spirit_obj: Arc<dyn crate::scheduler::control_block::AnySpiritObj>,
        boot_nonce: u64,
    ) -> Result<u32, LifecycleError> {
        let revocation_rules = self
            .revocation_rules
            .read()
            .expect("revocation rules binding lock poisoned")
            .clone();
        let _revocation_admission_guard = match revocation_rules.as_ref() {
            Some(rules) => Some(rules.admission_guard().await),
            None => None,
        };
        if revocation_rules
            .as_ref()
            .is_some_and(|rules| rules.rejects_manifest_locked(&manifest))
        {
            return Err(LifecycleError::Admission(
                "spirit class/version is revoked by a validated CRL".into(),
            ));
        }

        if self.resolve_pid(spirit_id).is_some() {
            return Err(LifecycleError::AlreadyLoaded {
                spirit_id: spirit_id.into(),
            });
        }

        let pid = allocate_pid();

        // Story 1b.3 — admit the Spirit through the security manager.
        if let Some(security) = &self.security_manager {
            use crate::security::{
                CapabilitiesRequired, McpCapabilities, Posture, PostureSection,
                ProviderCapabilities, ResourceCaps, SandboxConfig,
            };
            use maos_domain::invariants::i9::SandboxTier;
            let sandbox_cfg = SandboxConfig {
                tier: SandboxTier::T2,
                image_pin: None,
            };
            let caps = ResourceCaps {
                cpu_max_pct: None,
                memory_max_mb: None,
                fd_max: None,
            };
            let caps_required = CapabilitiesRequired {
                provider: ProviderCapabilities { complete: vec![] },
                mcp: McpCapabilities { servers: vec![] },
                loom: crate::security::manifest::LoomCapabilities::default(),
            };
            let posture = PostureSection {
                default: Posture::Cautious,
                allowed_max: Posture::AutonomousWithHalt,
            };
            security
                .admit_spirit(
                    pid,
                    spirit_id,
                    &sandbox_cfg,
                    &caps,
                    &caps_required,
                    None,
                    self,
                    &posture,
                    None,
                    Some(&manifest.scheduling),
                    Some(&manifest.lifecycle),
                    manifest.on_crash.as_ref(),
                    manifest.supervision.as_ref(),
                    None,
                    // Story 7.5a — ABI Stability Triple enforcement at admit.
                    // The bundle already carries the parsed `[class]` section
                    // (control_block.rs); thread it so min_substrate_version +
                    // manifest_schema_version are enforced on this load path.
                    manifest.class.as_ref(),
                )
                .map_err(|e| LifecycleError::Admission(e.to_string()))?;
        } else {
            // v0.3-β test harnesses may omit the security manager.
            let _ = self.tl.insert_frame_event(
                crate::iac::transparency_log::FrameKind::CapabilityInvocation,
                pid,
                None,
                "lifecycle.admit",
                serde_json::json!({"spirit_id": spirit_id})
                    .to_string()
                    .as_bytes(),
                FrameOrigin::SpiritAuto,
            );
        }
        let scb = Arc::new(SpiritControlBlock::new(
            pid,
            spirit_id.into(),
            manifest,
            spirit_obj,
            boot_nonce,
        ));

        {
            let mut spirits = self.spirits.write().unwrap();
            spirits.insert(pid, Arc::clone(&scb));
        }

        // Journal
        let payload = serde_json::json!({
            "lifecycle_event": "Load",
            "spirit_id": spirit_id,
            "spirit_pid": pid,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            pid,
            None,
            "lifecycle.load",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        // Fire on_load synchronously (rust-inproc)
        let outcome = self.dispatcher.fire_on_load(&scb).await;
        self.check_hook_outcome("on_load", pid, outcome)?;

        Ok(pid)
    }

    /// AC1 verb: start a loaded Spirit.
    pub async fn start(&self, spirit_pid: u32) -> Result<(), LifecycleError> {
        let scb = self.get_scb(spirit_pid)?;
        scb.transition(
            ScbLifecycleState::Running,
            maos_domain::lifecycle::LifecycleVerb::Start,
        )?;

        let payload = serde_json::json!({
            "lifecycle_event": "Start",
            "spirit_pid": spirit_pid,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "lifecycle.start",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        let outcome = self.dispatcher.fire_on_start(&scb).await;
        self.check_hook_outcome("on_start", spirit_pid, outcome)?;

        Ok(())
    }

    /// AC1 verb: pause a running Spirit.
    pub async fn pause(&self, spirit_pid: u32) -> Result<(), LifecycleError> {
        let scb = self.get_scb(spirit_pid)?;
        scb.transition(
            ScbLifecycleState::Paused,
            maos_domain::lifecycle::LifecycleVerb::Pause,
        )?;

        let payload = serde_json::json!({
            "lifecycle_event": "Pause",
            "spirit_pid": spirit_pid,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "lifecycle.pause",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        let outcome = self.dispatcher.fire_on_pause(&scb).await;
        self.check_hook_outcome("on_pause", spirit_pid, outcome)?;

        Ok(())
    }

    /// AC1 verb: resume a paused Spirit.
    pub async fn resume(&self, spirit_pid: u32) -> Result<(), LifecycleError> {
        let scb = self.get_scb(spirit_pid)?;
        scb.transition(
            ScbLifecycleState::Running,
            maos_domain::lifecycle::LifecycleVerb::Resume,
        )?;

        let payload = serde_json::json!({
            "lifecycle_event": "Resume",
            "spirit_pid": spirit_pid,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "lifecycle.resume",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        // Story 3.4 — replay buffered Orchestrator instructions.
        if let Some(ref registry) = self.orchestrator_registry {
            use maos_spirit_abi::identity::SpiritId;
            let sid = SpiritId::from(scb.spirit_id.as_str());
            if let Some(buffer) = registry.get(&sid) {
                let pending = buffer.recall_all_pending();
                // v0.3-β: re-enqueue so the Orchestrator spirit can dequeue
                // them at a safe point. Frame-level delivery deferred to
                // Story 5.4 when the orchestrator wire protocol stabilizes.
                for instr in pending {
                    if let Err(e) = buffer.enqueue(instr) {
                        eprintln!("maos: resume re-enqueue failed: {e}");
                    }
                }
            }
        }

        let outcome = self.dispatcher.fire_on_resume(&scb).await;
        self.check_hook_outcome("on_resume", spirit_pid, outcome)?;

        Ok(())
    }

    /// AC1 verb: unload a Spirit (idempotent).
    ///
    /// Story 16-6 (AC2(g), mechanism (x)) — the receipt, the token
    /// revocation, the halt drain and the map removal run **before** the
    /// fallible `on_unload` dispatch, not after it.
    ///
    /// Why: the state already flipped to `Unloaded` above, so a hook failure
    /// used to abort all five cleanup steps while leaving the SCB in the map
    /// with no `term-…` receipt and its capability tokens live — and the
    /// operator's retry then hit the idempotent early return and answered
    /// `Ok(())`, writing no receipt either. A Spirit the kernel has already
    /// released is released whatever its hook does.
    ///
    /// The `PlannedUnload` receipt attests the kernel's release (§18a, ruled
    /// (A)), not the death of an OS child. In-process classes have no child;
    /// the subprocess Worker's child is signalled by its post-release
    /// `on_unload` hook. A failing Worker hook leaves that child to
    /// supervision as the documented residual.
    pub async fn unload(&self, spirit_pid: u32) -> Result<(), LifecycleError> {
        let scb = match self.get_scb_optional(spirit_pid) {
            Some(scb) => scb,
            None => return Ok(()),
        };

        let current = scb.current_state();
        if current == ScbLifecycleState::Unloaded {
            return Ok(());
        }

        scb.transition(
            ScbLifecycleState::Unloaded,
            maos_domain::lifecycle::LifecycleVerb::Unload,
        )?;

        let payload = serde_json::json!({
            "lifecycle_event": "Unload",
            "spirit_pid": spirit_pid,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "lifecycle.unload",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        // Story 16-6 — UNCONDITIONAL cleanup, ahead of the hook.
        // Story 5.3 — produce halt-receipts for planned unload (NFR-Rel-11)
        let _receipts = terminate_spirit(
            &self.tl,
            &self._halt_registry,
            spirit_pid,
            &scb.spirit_id,
            TerminationKind::PlannedUnload,
            scb.boot_nonce,
        );
        self.capability.revoke_all_for_pid(spirit_pid);
        // Story 5.3 — per-PID drain (closes Story 4.1 deferred §1)
        let _ = self._halt_registry.drain_for_spirit(spirit_pid);
        {
            let mut spirits = self.spirits.write().unwrap();
            spirits.remove(&spirit_pid);
        }

        let outcome = self.dispatcher.fire_on_unload(&scb).await;
        if let Err(error) = self.check_hook_outcome("on_unload", spirit_pid, outcome) {
            // Story 16-6 (AC2(g)(ii)) — FR50's disposition and orphan frames
            // inline. `check_hook_outcome` no longer spawns `handle_crash`
            // for `on_unload` (it would double-teardown the same pid), and
            // with the SCB already removed that spawn would in any case have
            // returned `NotLoaded` into a `let _ =`, silently losing frames
            // that fire today.
            self.orphan_in_flight_tasks(&scb, &error);
            return Err(error);
        }

        Ok(())
    }

    /// DRR picker — select next Running Spirit to dispatch.
    ///
    /// Correct two-pass DRR: (1) increment every Running Spirit's deficit by
    /// its weight; (2) pick the Spirit with the highest deficit ≥ quantum;
    /// (3) deduct quantum from the winner.  Ensures proportional fairness.
    pub fn pick_next_spirit(&self) -> Option<u32> {
        let spirits = self.spirits.read().unwrap();
        let scbs: Vec<_> = spirits.values().cloned().collect();
        pick_next_spirit_from_slice(&scbs)
    }

    fn get_scb(&self, pid: u32) -> Result<Arc<SpiritControlBlock>, LifecycleError> {
        let spirits = self.spirits.read().unwrap();
        spirits
            .get(&pid)
            .cloned()
            .ok_or_else(|| LifecycleError::NotLoaded {
                spirit_id: format!("pid-{}", pid),
            })
    }

    fn get_scb_optional(&self, pid: u32) -> Option<Arc<SpiritControlBlock>> {
        let spirits = self.spirits.read().unwrap();
        spirits.get(&pid).cloned()
    }

    /// Map a hook outcome to a lifecycle result.
    /// `BudgetWarning80` is informational — it does NOT fail the verb.
    fn check_hook_outcome(
        &self,
        hook_name: &'static str,
        spirit_pid: u32,
        outcome: HookOutcome,
    ) -> Result<(), LifecycleError> {
        match outcome {
            HookOutcome::Fired { .. }
            | HookOutcome::SkippedManifest
            | HookOutcome::BudgetWarning80 { .. }
            | HookOutcome::DeferredToNextStory => Ok(()),
            HookOutcome::BudgetExceeded {
                wall_ns,
                cap_seconds,
            } => Err(LifecycleError::HookBudgetExceeded {
                hook_name,
                wall_ns,
                cap_seconds,
            }),
            HookOutcome::Panicked {
                panic_payload_preview,
            } => {
                // Story 5.3 — fire-and-forget crash handler BEFORE propagating error.
                //
                // Story 16-6 (AC2(g)(i)) — NEVER for `on_unload`. `unload`
                // now owns the whole teardown (receipt + revoke + drain +
                // map removal) and `handle_crash` performs the SAME teardown
                // in parallel. `terminate_spirit` is not idempotent — its
                // `HaltId` is `term-{spirit_id}-{pid}-{SystemTime::now()}`
                // with no `boot_nonce` — so two calls mint two ids and four
                // `EpistemicHalt` frames with mismatched `kind_str`. The
                // weaker guard "spawn only if the pid is still in the map"
                // does NOT work: `handle_crash` is also how a cancelled hook
                // arrives here (`Ok(Err(join_err))` maps cancellation to
                // `Panicked` too), so this is a hook-identity guard, not a
                // panic-only one.
                if hook_name != "on_unload" {
                    if let Some(cd) = &self.crash_detector {
                        let cd_clone = Arc::clone(cd);
                        let hook_name_clone = hook_name.to_string();
                        let payload_clone = panic_payload_preview.clone();
                        let _ = tokio::spawn(async move {
                            let _ = cd_clone
                                .handle_crash(
                                    spirit_pid,
                                    maos_domain::supervision::CrashCause::Fault(
                                        maos_domain::supervision::FaultCause::Panic {
                                            hook_name: hook_name_clone,
                                            payload_preview: payload_clone,
                                        },
                                    ),
                                )
                                .await;
                        });
                    }
                }
                Err(LifecycleError::HookPanicked {
                    hook_name: hook_name.to_string(),
                    preview: panic_payload_preview,
                })
            }
        }
    }

    /// Story 16-6 (AC2(g)(ii)) — FR50 disposition and `task.orphaned` frames
    /// for an `on_unload` that failed after the Spirit was already released.
    ///
    /// This is the work `CrashDetector::handle_crash` steps 5 and 6 do. It is
    /// performed inline because the crash handler is deliberately NOT spawned
    /// for `on_unload` (see `check_hook_outcome`) and, with the SCB already
    /// out of the map, would return `NotLoaded` from its Step-1 lookup into a
    /// discarded `Result` — silently dropping frames that fire today.
    fn orphan_in_flight_tasks(&self, scb: &Arc<SpiritControlBlock>, error: &LifecycleError) {
        let drained: Vec<_> = {
            let mut tasks = scb
                .task_assignments_in_flight
                .lock()
                .expect("task ledger lock poisoned");
            std::mem::take(&mut *tasks)
        };
        if drained.is_empty() {
            return;
        }
        let runtime = scb.runtime_snapshot();
        for task in &drained {
            let payload = serde_json::json!({
                "task_id": task.task_id,
                "originator_spirit_id": task.originator_spirit_id,
                "exit_signal": serde_json::Value::Null,
                "exit_code": serde_json::Value::Null,
                "stderr_tail": error.to_string(),
                "cause": "on_unload_hook_failure",
                "in_flight_tokens": task
                    .capability_token
                    .as_ref()
                    .into_iter()
                    .collect::<Vec<_>>(),
                "disposition": runtime.on_crash_action.to_string(),
            });
            self.tl.insert_frame_event(
                crate::iac::transparency_log::FrameKind::TaskComplete,
                scb.pid,
                None,
                "task.orphaned",
                &serde_json::to_vec(&payload).unwrap_or_default(),
                FrameOrigin::Kernel,
            );
        }
        let _ = crate::supervision::disposition::enforce_disposition(
            runtime.on_crash_action.clone(),
            &drained,
            &self._iac,
            self.replica.as_deref(),
        );
    }
}

impl SpiritSchedulerPort for SpiritSchedulerAdapter {
    fn journal_lifecycle(&self, entry: JournalEntry) {
        let (lifecycle_event, spirit_id) = match &entry {
            JournalEntry::Lifecycle(le) => (le.lifecycle_event, le.spirit_id.as_str()),
            JournalEntry::InFlight(_) => return,
            #[allow(unreachable_patterns)]
            _ => return,
        };
        let payload = serde_json::json!({
            "lifecycle_event": format!("{:?}", lifecycle_event),
            "spirit_id": spirit_id,
        });
        let _ = self.tl.insert_frame_event(
            crate::iac::transparency_log::FrameKind::CapabilityInvocation,
            0,
            None,
            "lifecycle.journal",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
    }

    fn last_lifecycle_event(&self, _spirit_id: &str) -> Option<LifecycleEvent> {
        None
    }
}
