//! Story 22-8 (D-22-8-02/03/06 A) — the native directed leg: `maosctl ask mira "<incident>"`.
//!
//! Mira is a native, in-process class: no mailbox turn answers a task frame. This module is the
//! host-side wiring only. [`MiraRunner`] is the [`TaskRunner`] the registry's pump calls; it
//! hands the text to Mira's own task handler (`spirits/mira`: prompt adapter, deterministic
//! diagnosis, memory record, advisory), journals the answer as the `TaskComplete` reply row
//! (`maos.task-reply/<hex>`, as a component's reply is) before it returns, so `task show` reads
//! the same bytes after a restart. Nash stays a bare class object: it is not directable.

use crate::directed_task::{RunFailure, TaskRunner};
use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse,
};
use maos_iac::adapter::mailbox::{reply_correlation, ReplyLabel};
use maos_kernel_core::iac::transparency_log::FrameKind;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::memory::MemoryManagerAdapter;
use std::sync::{Arc, Weak};

/// The Spirit's admitted pid, once the scheduler assigned one.
pub type PidSource = Arc<dyn Fn() -> Option<u32> + Send + Sync>;

/// A [`PidSource`] over the scheduler, resolving `spirit_id` at each call (an upgrade successor
/// or a restart keeps the binding honest). Weak: a source a Spirit retains cannot hold the
/// root's audit senders open past its drain.
pub fn scheduler_pid_source(
    scheduler: &Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    spirit_id: &str,
) -> PidSource {
    let (scheduler, spirit_id) = (Arc::downgrade(scheduler), spirit_id.to_owned());
    Arc::new(move || {
        scheduler
            .upgrade()?
            .resolve_pid(&spirit_id)
            .filter(|pid| *pid != 0)
    })
}

/// D-22-8-04 A — the Inference Port of a native Spirit's manifest-declared provider. `None`
/// under the Deterministic mode (the seam carries no port, ADR-064); a declared provider the
/// host router does not register is a typed boot refusal naming every declared provider.
pub fn bind_inference(
    seam: &butler::ButlerSeam,
    declared: &[String],
    pid_of: PidSource,
) -> Result<Option<Arc<dyn InferencePort + Send + Sync>>, String> {
    let Some((build, registered)) = seam.inference.as_ref() else {
        return Ok(None);
    };
    let providers: Vec<String> = declared
        .iter()
        .map(|entry| maos_manifest::split_provider_pin(entry).0.to_owned())
        .collect();
    let provider = researcher::declared_provider(&providers, registered)?;
    let model = maos_manifest::pinned_model(declared, &provider);
    Ok(Some(Arc::new(MintedInference::new(
        seam,
        Arc::clone(build),
        pid_of,
        provider,
        model,
    ))))
}

/// The host's Inference Port for one native Spirit (D-22-8-04 A): every request is re-bound to
/// the Spirit's admitted pid, a freshly minted `ProviderInfer` token and its declared provider
/// and pinned model; the Spirit supplies none of them. The port itself is built per call by
/// the seam's builder, which holds the router and the capability registry weakly, so a native
/// Spirit never keeps the root's audit senders open past its drain.
pub struct MintedInference {
    build: butler::PortBuilder,
    mint: butler::ProviderMint,
    pid_of: PidSource,
    provider: String,
    model: Option<String>,
}

impl MintedInference {
    pub fn new(
        seam: &butler::ButlerSeam,
        build: butler::PortBuilder,
        pid_of: PidSource,
        provider: String,
        model: Option<String>,
    ) -> Self {
        Self {
            build,
            mint: Arc::clone(&seam.mint),
            pid_of,
            provider,
            model,
        }
    }
}

impl InferencePort for MintedInference {
    fn complete(&self, mut request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let port = (self.build)().ok_or(InferenceError::Unconfigured)?;
        let pid = (self.pid_of)().ok_or(InferenceError::CapabilityDenied)?;
        let token: CapabilityToken =
            (self.mint)(pid, &self.provider).map_err(|_| InferenceError::CapabilityDenied)?;
        request.spirit_pid = pid;
        request.capability_token = token;
        request.provider_id = Some(self.provider.clone());
        request.options.model_id.clone_from(&self.model);
        port.complete(request)
    }
}

/// Mira's durable per-subject memory: the Private tier, the Default namespace, Markdown (always
/// spilled, so it survives the process), written as Mira's own pid through the kernel's
/// memory manager. The adapter is held weakly: the root's teardown drops it before the drain.
pub struct PidBoundDiagnosisMemory {
    memory: Weak<MemoryManagerAdapter>,
    pid_of: PidSource,
}

impl PidBoundDiagnosisMemory {
    pub fn new(memory: Weak<MemoryManagerAdapter>, pid_of: PidSource) -> Self {
        Self { memory, pid_of }
    }

    fn bound(&self) -> Result<(Arc<MemoryManagerAdapter>, u32), String> {
        let memory = self
            .memory
            .upgrade()
            .ok_or("diagnosis memory: the host shut down")?;
        let pid = (self.pid_of)().ok_or("diagnosis memory: the Spirit has no admitted pid")?;
        Ok((memory, pid))
    }
}

impl mira::DiagnosisMemory for PidBoundDiagnosisMemory {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        let (memory, pid) = self.bound()?;
        match memory
            .for_spirit(pid)
            .read(MemoryTier::Private, &MemoryNamespace::Default, key)
            .map_err(|e| format!("diagnosis memory read (pid {pid}): {e}"))?
        {
            None => Ok(None),
            Some(MemoryValue::Markdown(markdown)) => Ok(Some(markdown)),
            Some(other) => Err(format!(
                "diagnosis memory read (pid {pid}): `{key}` holds {:?}, not Markdown",
                other.kind()
            )),
        }
    }

    fn write(&self, key: &str, markdown: &str) -> Result<(), String> {
        let (memory, pid) = self.bound()?;
        memory
            .for_spirit(pid)
            .write(
                MemoryTier::Private,
                &MemoryNamespace::Default,
                key,
                MemoryValue::Markdown(markdown.into()),
            )
            .map_err(|e| format!("diagnosis memory write (pid {pid}): {e}"))
    }
}

/// Mira's door-task runner (registered with the directed-task registry beside the installed
/// components': the same bounds, accept row, idempotency key and halt correlation apply).
pub struct MiraRunner {
    mira: mira::Mira,
    inference: Option<Arc<dyn InferencePort + Send + Sync>>,
    memory: PidBoundDiagnosisMemory,
    route: Option<Arc<dyn mira::AdvisoryRoute>>,
    log: Arc<TransparencyLogAdapter>,
    pid_of: PidSource,
}

impl MiraRunner {
    pub fn new(
        mira: mira::Mira,
        inference: Option<Arc<dyn InferencePort + Send + Sync>>,
        memory: PidBoundDiagnosisMemory,
        route: Option<Arc<dyn mira::AdvisoryRoute>>,
        log: Arc<TransparencyLogAdapter>,
        pid_of: PidSource,
    ) -> Self {
        Self {
            mira,
            inference,
            memory,
            route,
            log,
            pid_of,
        }
    }
}

impl TaskRunner for MiraRunner {
    fn run(
        &self,
        task_id: [u8; 16],
        text: &str,
        assertion: Option<&maos_control::Assertion>,
    ) -> Result<String, RunFailure> {
        let pid = (self.pid_of)().ok_or_else(|| RunFailure {
            reason: "dispatch_refused",
            detail: "Mira has no admitted pid".into(),
        })?;
        // The task's assertion governs the mints for Mira's pid for this turn (Story 23-2).
        let _scope = crate::assertion_supply::enter(pid, assertion).map_err(|_| RunFailure {
            reason: "dispatch_refused",
            detail: "an assertion scope is already active for this Spirit".into(),
        })?;
        let answer = self
            .mira
            .handle_task(
                text,
                self.inference
                    .as_deref()
                    .map(|port| port as &dyn InferencePort),
                &self.memory,
                self.route.as_deref(),
            )
            .map_err(|error| RunFailure {
                reason: error.code(),
                detail: error.detail().to_owned(),
            })?
            .to_json();
        // The reply row precedes `completed` (log before deliver): `task show` reads this row,
        // live and after a restart.
        let payload = serde_json::json!({ "TaskComplete": { "result": answer } });
        self.log
            .insert_frame_event_with_correlation(
                FrameKind::TaskComplete,
                pid,
                None,
                &reply_correlation(ReplyLabel::Task, task_id),
                "task.reply",
                payload.to_string().as_bytes(),
                maos_domain::invariants::i3::FrameOrigin::Kernel,
            )
            .into_inner();
        Ok(answer)
    }
}
