#![forbid(unsafe_code)]

//! Kernel-side context wrapper — routes Spirit-side calls through
//! Epic-4 adapters. The `Ctx` itself (no_std, ABI surface) stays
//! unchanged at v0.3-β; `KernelCtx` is the std-aware wrapper the
//! kernel consumes during hook dispatch.
//!
//! Architecture §4.1 + Story 5.1 Task 7.

use std::sync::Arc;

use maos_spirit_abi::ctx::Ctx;

/// Wraps the ABI `Ctx` with `Arc` handles to all Epic-4 adapters,
/// routing Spirit-author-facing convenience calls into the kernel.
pub struct KernelCtx<'a> {
    pub ctx: &'a mut Ctx,
    pub memory_manager: Option<Arc<crate::memory::MemoryManagerAdapter>>,
    pub capability: Option<Arc<crate::capability::CapabilityRegistryAdapter>>,
    pub working_memory_orchestrator: Option<
        Arc<crate::capability::working_memory::orchestrator::WorkingMemoryOrchestrator>,
    >,
    pub iac: Option<Arc<crate::iac::IacBusAdapter>>,
    pub halt_registry: Option<Arc<crate::halt::HaltRegistry>>,
    pub log_recall: Option<Arc<crate::iac::log_recall::LogRecallAdapter>>,
    pub distillate_writer: Option<Arc<crate::iac::distillate::DistillateWriter>>,
    pub self_telemetry: Option<Arc<crate::memory::self_telemetry::SelfTelemetryAggregator>>,
    pub transparency_log: Option<Arc<crate::iac::transparency_log::TransparencyLogAdapter>>,
}

impl<'a> KernelCtx<'a> {
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self {
            ctx,
            memory_manager: None,
            capability: None,
            working_memory_orchestrator: None,
            iac: None,
            halt_registry: None,
            log_recall: None,
            distillate_writer: None,
            self_telemetry: None,
            transparency_log: None,
        }
    }

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

    pub fn with_transparency_log(
        mut self,
        tl: Arc<crate::iac::transparency_log::TransparencyLogAdapter>,
    ) -> Self {
        self.transparency_log = Some(tl);
        self
    }

    pub fn memory(&self) -> Option<&crate::memory::MemoryManagerAdapter> {
        self.memory_manager.as_deref()
    }
}
