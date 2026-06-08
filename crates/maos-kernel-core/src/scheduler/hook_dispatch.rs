#![forbid(unsafe_code)]

//! Per-hook dispatch — kernel calls each lifecycle hook through the
//! type-erased `AnySpiritObj` after checking (a) manifest `[lifecycle]`
//! declares it AND (b) the per-hook budget envelope. Emits
//! `BudgetWarning` at 80% of `time_cap_seconds` per NFR-Perf-6.
//!
//! Architecture §4.1 + Story 5.1 Task 5.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::scheduler::control_block::SpiritControlBlock;
use crate::scheduler::kernel_ctx::KernelCtx;
use crate::telemetry::iac_rt::IacRtMetrics;
use maos_domain::invariants::i3::FrameOrigin;
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Outcome of a hook fire.
#[derive(Debug, Clone, PartialEq)]
pub enum HookOutcome {
    Fired {
        wall_ns: u64,
    },
    SkippedManifest,
    BudgetExceeded {
        wall_ns: u64,
        cap_seconds: u64,
    },
    BudgetWarning80 {
        wall_ns: u64,
        cap_seconds: u64,
        fired: bool,
    },
    Panicked {
        panic_payload_preview: String,
    },
    DeferredToNextStory,
}

/// Dispatches lifecycle hooks through the Spirit's AnySpiritObj,
/// enforcing manifest gates and budget envelopes.
#[derive(Clone)]
#[maos_attrs::i9_exempt(
    reason = "lifecycle-hook dispatcher holding Arc handles to the already-exempt transparency-log adapter + iac-rt metrics; supervised composite per I9 (Story 7.1.7 baseline-reset)"
)]
pub struct HookDispatcher {
    tl: Arc<TransparencyLogAdapter>,
    pub metrics: Arc<IacRtMetrics>,
    memory_manager: Option<Arc<crate::memory::MemoryManagerAdapter>>,
    capability: Option<Arc<crate::capability::CapabilityRegistryAdapter>>,
    working_memory_orchestrator:
        Option<Arc<crate::capability::working_memory::orchestrator::WorkingMemoryOrchestrator>>,
    iac: Option<Arc<crate::iac::IacBusAdapter>>,
    halt_registry: Option<Arc<crate::halt::HaltRegistry>>,
    log_recall: Option<Arc<crate::iac::log_recall::LogRecallAdapter>>,
    distillate_writer: Option<Arc<crate::iac::distillate::DistillateWriter>>,
    self_telemetry: Option<Arc<crate::memory::self_telemetry::SelfTelemetryAggregator>>,
    /// Story 5.3 — shared SCB map for wiring KernelCtx::heartbeat
    spirits: Option<Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>>,
    /// Injectable time cap for testing (default = 30s).
    pub time_cap_seconds: u64,
}

impl HookDispatcher {
    pub fn new(tl: Arc<TransparencyLogAdapter>, metrics: Arc<IacRtMetrics>) -> Self {
        Self {
            tl,
            metrics,
            memory_manager: None,
            capability: None,
            working_memory_orchestrator: None,
            iac: None,
            halt_registry: None,
            log_recall: None,
            distillate_writer: None,
            self_telemetry: None,
            spirits: None,
            time_cap_seconds: Self::DEFAULT_TIME_CAP_SECONDS,
        }
    }

    pub const DEFAULT_TIME_CAP_SECONDS: u64 = 30;

    pub fn with_memory_manager(mut self, m: Arc<crate::memory::MemoryManagerAdapter>) -> Self {
        self.memory_manager = Some(m);
        self
    }

    pub fn with_capability(mut self, c: Arc<crate::capability::CapabilityRegistryAdapter>) -> Self {
        self.capability = Some(c);
        self
    }

    pub fn with_working_memory_orchestrator(
        mut self,
        o: Arc<crate::capability::working_memory::orchestrator::WorkingMemoryOrchestrator>,
    ) -> Self {
        self.working_memory_orchestrator = Some(o);
        self
    }

    pub fn with_iac(mut self, i: Arc<crate::iac::IacBusAdapter>) -> Self {
        self.iac = Some(i);
        self
    }

    pub fn with_halt_registry(mut self, h: Arc<crate::halt::HaltRegistry>) -> Self {
        self.halt_registry = Some(h);
        self
    }

    pub fn with_log_recall(mut self, l: Arc<crate::iac::log_recall::LogRecallAdapter>) -> Self {
        self.log_recall = Some(l);
        self
    }

    pub fn with_distillate_writer(
        mut self,
        d: Arc<crate::iac::distillate::DistillateWriter>,
    ) -> Self {
        self.distillate_writer = Some(d);
        self
    }

    pub fn with_self_telemetry(
        mut self,
        s: Arc<crate::memory::self_telemetry::SelfTelemetryAggregator>,
    ) -> Self {
        self.self_telemetry = Some(s);
        self
    }

    pub fn with_spirits(
        mut self,
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    ) -> Self {
        self.spirits = Some(spirits);
        self
    }

    fn build_kernel_ctx<'a>(
        &self,
        ctx: &'a mut maos_spirit_abi::ctx::Ctx,
        spirit_pid: u32,
    ) -> KernelCtx<'a> {
        let mut kctx = KernelCtx::new(ctx).with_spirit_pid(spirit_pid);
        if let Some(ref m) = self.memory_manager {
            kctx = kctx.with_memory_manager(Arc::clone(m));
        }
        if let Some(ref c) = self.capability {
            kctx = kctx.with_capability(Arc::clone(c));
        }
        if let Some(ref o) = self.working_memory_orchestrator {
            kctx = kctx.with_working_memory_orchestrator(Arc::clone(o));
        }
        if let Some(ref i) = self.iac {
            kctx = kctx.with_iac(Arc::clone(i));
        }
        if let Some(ref h) = self.halt_registry {
            kctx = kctx.with_halt_registry(Arc::clone(h));
        }
        if let Some(ref l) = self.log_recall {
            kctx = kctx.with_log_recall(Arc::clone(l));
        }
        if let Some(ref d) = self.distillate_writer {
            kctx = kctx.with_distillate_writer(Arc::clone(d));
        }
        if let Some(ref s) = self.self_telemetry {
            kctx = kctx.with_self_telemetry(Arc::clone(s));
        }
        if let Some(ref spirits) = self.spirits {
            kctx = kctx.with_spirits(Arc::clone(spirits));
        }
        kctx
    }

    fn hook_allowed(scb: &SpiritControlBlock, hook_name: &str) -> bool {
        let enabled: Vec<&str> = scb
            .manifest
            .lifecycle
            .enabled_hooks
            .iter()
            .map(|s| s.as_str())
            .collect();
        maos_spirit_abi::lifecycle::kernel_invocation_allowed(&enabled, hook_name)
    }

    fn emit_budget_frame(
        &self,
        scb: &SpiritControlBlock,
        hook_name: &str,
        wall_ns: u64,
        cap_seconds: u64,
        kind: FrameKind,
    ) {
        let ratio = if cap_seconds > 0 {
            (wall_ns as f64) / (cap_seconds as f64 * 1_000_000_000.0)
        } else {
            1.0
        };
        let payload = serde_json::json!({
            "spirit_pid": scb.pid,
            "hook_name": hook_name,
            "wall_ns": wall_ns,
            "cap_seconds": cap_seconds,
            "ratio_breached": ratio,
        });
        let _ = self.tl.insert_frame_event(
            kind,
            scb.pid,
            None,
            &format!("hook.budget.{}", hook_name),
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
    }

    pub async fn fire_on_load(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_load", |obj, ctx| obj.on_load(ctx))
            .await
    }

    pub async fn fire_on_start(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_start", |obj, ctx| obj.on_start(ctx))
            .await
    }

    pub async fn fire_on_idle(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_idle", |obj, ctx| obj.on_idle(ctx))
            .await
    }

    pub async fn fire_on_pause(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_pause", |obj, ctx| obj.on_pause(ctx))
            .await
    }

    pub async fn fire_on_resume(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_resume", |obj, ctx| obj.on_resume(ctx))
            .await
    }

    pub async fn fire_on_unload(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_unload", |obj, ctx| obj.on_unload(ctx))
            .await
    }

    pub async fn fire_on_frame(&self, scb: &SpiritControlBlock, payload: &[u8]) -> HookOutcome {
        self.fire_payload_hook(scb, "on_frame", payload, |obj, ctx, p| obj.on_frame(ctx, p))
            .await
    }

    pub async fn fire_on_telemetry_event(
        &self,
        scb: &SpiritControlBlock,
        payload: &[u8],
    ) -> HookOutcome {
        self.fire_payload_hook(scb, "on_telemetry_event", payload, |obj, ctx, p| {
            obj.on_telemetry_event(ctx, p)
        })
        .await
    }

    pub async fn fire_on_schedule(&self, scb: &SpiritControlBlock, payload: &[u8]) -> HookOutcome {
        self.fire_payload_hook(scb, "on_schedule", payload, |obj, ctx, p| {
            obj.on_schedule(ctx, p)
        })
        .await
    }

    pub async fn fire_on_swap_in(&self, scb: &SpiritControlBlock, payload: &[u8]) -> HookOutcome {
        self.fire_payload_hook(scb, "on_swap_in", payload, |obj, ctx, p| {
            obj.on_swap_in(ctx, p)
        })
        .await
    }

    pub async fn fire_on_consolidate(
        &self,
        scb: &SpiritControlBlock,
        payload: &[u8],
    ) -> HookOutcome {
        self.fire_payload_hook(scb, "on_consolidate", payload, |obj, ctx, p| {
            obj.on_consolidate(ctx, p)
        })
        .await
    }

    /// Story 5.2 — Fire the on_swap_out hook on the predecessor.
    pub async fn fire_on_swap_out(&self, scb: &SpiritControlBlock) -> HookOutcome {
        self.fire_no_payload_hook(scb, "on_swap_out", |obj, ctx| obj.on_swap_out(ctx))
            .await
    }

    /// Story 8.11 / AC3 — the effective hook time cap for a Spirit: its declared
    /// `[budget].time_cap_seconds` when present, else the dispatcher default
    /// (`DEFAULT_TIME_CAP_SECONDS`). The kernel never learns what the budget
    /// bounds — only that this Spirit's hooks must finish within it.
    fn effective_cap_seconds(&self, scb: &SpiritControlBlock) -> u64 {
        scb.manifest
            .budget
            .as_ref()
            .map(|b| b.time_cap_seconds as u64)
            .unwrap_or(self.time_cap_seconds)
    }

    /// Story 5.2 — Fire the snapshot hook on the predecessor.
    /// Returns the CBOR-encoded state blob on success.
    pub async fn fire_snapshot(&self, scb: &SpiritControlBlock) -> Result<Vec<u8>, HookOutcome> {
        if !Self::hook_allowed(scb, "snapshot") {
            return Err(HookOutcome::SkippedManifest);
        }
        let cap_seconds = self.effective_cap_seconds(scb);
        let wall_start = crate::capability::cap_tokens::monotonic_now_ns();
        let spirit_obj = Arc::clone(&scb.spirit_obj);

        let spirit_pid = scb.pid;
        let spirits = self.spirits.clone();
        let snapshot_future = timeout(
            Duration::from_secs(cap_seconds),
            tokio::task::spawn_blocking(move || {
                let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
                    maos_spirit_abi::ctx::CapabilityHandle(0),
                    maos_spirit_abi::ctx::MailboxHandle(0),
                );
                let mut kernel_ctx = KernelCtx::new(&mut ctx).with_spirit_pid(spirit_pid);
                if let Some(ref s) = spirits {
                    kernel_ctx = kernel_ctx.with_spirits(Arc::clone(s));
                }
                spirit_obj.snapshot(&mut kernel_ctx)
            }),
        );

        let result = snapshot_future.await;
        let wall_ns = crate::capability::cap_tokens::monotonic_now_ns() - wall_start;

        // Determine telemetry outcome from the inner result, not the outer timeout.
        let telemetry_outcome = match &result {
            Ok(Ok(_)) => crate::telemetry::iac_rt::Outcome::Ok,
            Ok(Err(_)) => crate::telemetry::iac_rt::Outcome::Err,
            Err(_) => crate::telemetry::iac_rt::Outcome::Timeout,
        };
        self.metrics.record_iac_rt(
            crate::telemetry::iac_rt::Service::SpiritScheduler,
            telemetry_outcome,
            wall_ns / 1000,
        );

        match result {
            Ok(Ok(state_blob)) => Ok(state_blob),
            Ok(Err(join_err)) => {
                let msg = if let Ok(panic_msg) = join_err.try_into_panic() {
                    if let Some(s) = panic_msg.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_msg.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "unknown panic payload".into()
                    }
                } else {
                    "spawn_blocking cancelled".into()
                };
                Err(HookOutcome::Panicked {
                    panic_payload_preview: msg,
                })
            }
            Err(_elapsed) => {
                self.emit_budget_frame(
                    scb,
                    "snapshot",
                    wall_ns,
                    cap_seconds,
                    crate::iac::transparency_log::FrameKind::BudgetExceeded,
                );
                Err(HookOutcome::BudgetExceeded {
                    wall_ns,
                    cap_seconds,
                })
            }
        }
    }

    /// Story 5.2 — Fire the migrate hook on the successor.
    /// Returns the migrated state blob on success.
    pub async fn fire_migrate(
        &self,
        scb: &SpiritControlBlock,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, maos_spirit_abi::lifecycle::MigratorError> {
        if !Self::hook_allowed(scb, "migrate") {
            return Err(maos_spirit_abi::lifecycle::MigratorError::new_internal(
                "manifest does not permit migrate hook",
            ));
        }
        let cap_seconds = self.effective_cap_seconds(scb);
        let wall_start = crate::capability::cap_tokens::monotonic_now_ns();
        let spirit_obj = Arc::clone(&scb.spirit_obj);
        let predecessor_state = predecessor_state.to_vec();

        let spirit_pid = scb.pid;
        let spirits = self.spirits.clone();
        let migrate_future = timeout(
            Duration::from_secs(cap_seconds),
            tokio::task::spawn_blocking(move || {
                let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
                    maos_spirit_abi::ctx::CapabilityHandle(0),
                    maos_spirit_abi::ctx::MailboxHandle(0),
                );
                let mut kernel_ctx = KernelCtx::new(&mut ctx).with_spirit_pid(spirit_pid);
                if let Some(ref s) = spirits {
                    kernel_ctx = kernel_ctx.with_spirits(Arc::clone(s));
                }
                spirit_obj.migrate(&mut kernel_ctx, &predecessor_state)
            }),
        );

        let result = migrate_future.await;
        let wall_ns = crate::capability::cap_tokens::monotonic_now_ns() - wall_start;

        // Determine telemetry outcome from the inner result, not the outer timeout.
        let telemetry_outcome = match &result {
            Ok(Ok(Ok(_))) => crate::telemetry::iac_rt::Outcome::Ok,
            Ok(Ok(Err(_))) => crate::telemetry::iac_rt::Outcome::Err,
            Ok(Err(_)) => crate::telemetry::iac_rt::Outcome::Err,
            Err(_) => crate::telemetry::iac_rt::Outcome::Timeout,
        };
        self.metrics.record_iac_rt(
            crate::telemetry::iac_rt::Service::SpiritScheduler,
            telemetry_outcome,
            wall_ns / 1000,
        );

        match result {
            Ok(Ok(Ok(migrated_blob))) => Ok(migrated_blob),
            Ok(Ok(Err(migrator_err))) => Err(migrator_err),
            Ok(Err(join_err)) => {
                let msg = if let Ok(panic_msg) = join_err.try_into_panic() {
                    if let Some(s) = panic_msg.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_msg.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "unknown panic payload".into()
                    }
                } else {
                    "spawn_blocking cancelled".into()
                };
                Err(maos_spirit_abi::lifecycle::MigratorError::new_internal(
                    format!("hook panicked: {msg}"),
                ))
            }
            Err(_elapsed) => {
                self.emit_budget_frame(
                    scb,
                    "migrate",
                    wall_ns,
                    cap_seconds,
                    crate::iac::transparency_log::FrameKind::BudgetExceeded,
                );
                Err(maos_spirit_abi::lifecycle::MigratorError::new_internal(
                    "migrate hook timed out",
                ))
            }
        }
    }

    async fn fire_no_payload_hook<F>(
        &self,
        scb: &SpiritControlBlock,
        hook_name: &'static str,
        fire_fn: F,
    ) -> HookOutcome
    where
        F: Fn(&Arc<dyn crate::scheduler::control_block::AnySpiritObj>, &mut KernelCtx)
            + Send
            + Sync
            + 'static,
    {
        self.fire_payload_hook(scb, hook_name, &[], move |obj, ctx, _| fire_fn(obj, ctx))
            .await
    }

    async fn fire_payload_hook<F>(
        &self,
        scb: &SpiritControlBlock,
        hook_name: &'static str,
        payload: &[u8],
        fire_fn: F,
    ) -> HookOutcome
    where
        F: Fn(&Arc<dyn crate::scheduler::control_block::AnySpiritObj>, &mut KernelCtx, &[u8])
            + Send
            + Sync
            + 'static,
    {
        if !Self::hook_allowed(scb, hook_name) {
            return HookOutcome::SkippedManifest;
        }

        let cap_seconds = self.effective_cap_seconds(scb);
        let wall_start = crate::capability::cap_tokens::monotonic_now_ns();

        let spirit_obj = Arc::clone(&scb.spirit_obj);
        let payload = payload.to_vec(); // clone for 'static closure

        let spirit_pid = scb.pid;
        let spirits = self.spirits.clone();
        let hook_future = timeout(
            Duration::from_secs(cap_seconds),
            tokio::task::spawn_blocking(move || {
                let mut ctx = maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
                    maos_spirit_abi::ctx::CapabilityHandle(0),
                    maos_spirit_abi::ctx::MailboxHandle(0),
                );
                let mut kernel_ctx = KernelCtx::new(&mut ctx).with_spirit_pid(spirit_pid);
                if let Some(ref s) = spirits {
                    kernel_ctx = kernel_ctx.with_spirits(Arc::clone(s));
                }
                fire_fn(&spirit_obj, &mut kernel_ctx, &payload);
            }),
        );

        let warn_seconds = std::cmp::max(1, cap_seconds * 4 / 5);
        let warn_sleep = tokio::time::sleep(Duration::from_secs(warn_seconds));
        tokio::pin!(hook_future);
        tokio::pin!(warn_sleep);

        let mut warned = false;

        let result = tokio::select! {
            biased;
            result = &mut hook_future => {
                result
            }
            _ = &mut warn_sleep => {
                warned = true;
                self.emit_budget_frame(
                    scb,
                    hook_name,
                    crate::capability::cap_tokens::monotonic_now_ns() - wall_start,
                    cap_seconds,
                    FrameKind::BudgetWarning,
                );
                // Continue waiting for the hook future to complete or time out.
                hook_future.await
            }
        };

        let wall_ns = crate::capability::cap_tokens::monotonic_now_ns() - wall_start;

        // Record iac_rt_duration_us metric.
        let outcome_label = match &result {
            Ok(Ok(())) => "ok",
            Ok(Err(_)) => "err",
            Err(_) => "timeout",
        };
        self.metrics.record_iac_rt(
            crate::telemetry::iac_rt::Service::SpiritScheduler,
            match outcome_label {
                "ok" => crate::telemetry::iac_rt::Outcome::Ok,
                "err" => crate::telemetry::iac_rt::Outcome::Err,
                _ => crate::telemetry::iac_rt::Outcome::Timeout,
            },
            wall_ns / 1000,
        );

        match result {
            Ok(Ok(())) => {
                if warned {
                    HookOutcome::BudgetWarning80 {
                        wall_ns,
                        cap_seconds,
                        fired: true,
                    }
                } else {
                    HookOutcome::Fired { wall_ns }
                }
            }
            Ok(Err(join_err)) => {
                let msg = if let Ok(panic_msg) = join_err.try_into_panic() {
                    if let Some(s) = panic_msg.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_msg.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "unknown panic payload".into()
                    }
                } else {
                    "spawn_blocking cancelled".into()
                };
                HookOutcome::Panicked {
                    panic_payload_preview: msg,
                }
            }
            Err(_elapsed) => {
                if !warned {
                    // Emit BudgetWarning if we haven't already.
                    self.emit_budget_frame(
                        scb,
                        hook_name,
                        wall_ns,
                        cap_seconds,
                        FrameKind::BudgetWarning,
                    );
                }
                self.emit_budget_frame(
                    scb,
                    hook_name,
                    wall_ns,
                    cap_seconds,
                    FrameKind::BudgetExceeded,
                );
                HookOutcome::BudgetExceeded {
                    wall_ns,
                    cap_seconds,
                }
            }
        }
    }
}
