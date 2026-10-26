//! Story 18-1 (D-18-1-04 A; review BH-2/EC-2) — the `--once` verdict on a
//! Butler pass. In the library, not `main.rs`, so `crates/maos-bin/tests/`
//! drives the production function (an in-`src` test module is budget-charged).
//!
//! Story 18-4 — the pid-fused principal port and the root's acceptance
//! binder (D-18-4-02 A; review TIA-1/BH-2: the root, not the Spirit, fixes
//! the pid). Story 18-5 — the root's context binder (CR-3). The scene's per-day step is Spirit-side (`butler::Scene`,
//! `Butler::scene_step`; operator ruling 2026-10-25, kloc C).

use std::path::Path;
use std::sync::{Arc, Weak};

use maos_domain::ports::ContextBudgetPort;
use maos_iac::adapter::context_signal::{AdmittedSpirit, ContextSignalAdapter};
use maos_kernel_core::capability::context_budget::ContextWindow;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::IacBusAdapter;
use maos_kernel_core::scheduler::control_block::SpiritControlBlock;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::HookOutcome;
use maos_kernel_core::security::OutputShapePredicate;
use maos_spirit_sdk::principal::PrincipalMemoryPort;

/// A refused pass, a bound pass the dispatcher did not run to completion
/// (budget exceeded, panicked — its blocking thread may still be running) and
/// an `output_shape` violation are all refusals the root exits non-zero on.
pub fn pass_verdict(
    butler: Option<&butler::Butler>,
    outcome: &HookOutcome,
    output: Option<&serde_json::Value>,
    predicate: &OutputShapePredicate,
) -> Result<(), String> {
    if let Some(error) = butler.and_then(butler::Butler::take_pass_error) {
        return Err(format!("butler on_idle pass refused: {error}"));
    }
    let completed = matches!(
        outcome,
        HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { fired: true, .. }
    );
    if butler.is_some_and(butler::Butler::is_inference_bound) && !completed {
        return Err(format!("butler on_idle pass did not complete: {outcome:?}"));
    }
    match output.map(|output| predicate.check(output)) {
        Some(Err(violation)) => Err(format!("output_shape violation: {violation}")),
        _ => Ok(()),
    }
}

/// Story 18-4 (D-18-4-02 A, AC3.1) — the principal port the root fuses to a
/// Spirit's resolved pid: every call writes as THAT pid through the kernel's
/// Principal-write enforcement (an unauthorized pid is refused
/// `PrincipalWriteUnauthorized`), in the Private tier, as Markdown (always
/// spilled, so it survives the process). Holds the adapter weakly: the
/// root's teardown drops it before the audit drain.
pub struct PidBoundPrincipalPort {
    memory: Weak<MemoryManagerAdapter>,
    pid: u32,
}

impl PidBoundPrincipalPort {
    pub fn new(memory: Weak<MemoryManagerAdapter>, pid: u32) -> Self {
        Self { memory, pid }
    }

    fn namespace(principal_id: &str, schema: &str) -> Result<MemoryNamespace, String> {
        MemoryNamespace::principal(principal_id, schema).map_err(|e| format!("principal port: {e}"))
    }

    fn memory(&self) -> Result<Arc<MemoryManagerAdapter>, String> {
        self.memory
            .upgrade()
            .ok_or_else(|| "principal port: the host shut down".to_owned())
    }
}

impl PrincipalMemoryPort for PidBoundPrincipalPort {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn write(
        &self,
        principal_id: &str,
        schema: &str,
        key: &str,
        markdown: &str,
    ) -> Result<(), String> {
        let namespace = Self::namespace(principal_id, schema)?;
        self.memory()?
            .for_spirit(self.pid)
            .write(
                MemoryTier::Private,
                &namespace,
                key,
                MemoryValue::Markdown(markdown.into()),
            )
            .map_err(|e| format!("principal port write (pid {}): {e}", self.pid))
    }

    fn read(&self, principal_id: &str, schema: &str, key: &str) -> Result<Option<String>, String> {
        let namespace = Self::namespace(principal_id, schema)?;
        match self
            .memory()?
            .for_spirit(self.pid)
            .read(MemoryTier::Private, &namespace, key)
            .map_err(|e| format!("principal port read (pid {}): {e}", self.pid))?
        {
            None => Ok(None),
            Some(MemoryValue::Markdown(markdown)) => Ok(Some(markdown)),
            Some(other) => Err(format!(
                "principal port read (pid {}): `{key}` holds {:?}, not Markdown",
                self.pid,
                other.kind()
            )),
        }
    }
}

/// Story 18-4 (D-18-4-02 A; review TIA-1/BH-2) — the root's binder: each
/// Butler's port and TL attestation resolve the pid through the resolver
/// the ROOT handed `ButlerSeam::attach` (unresolved ⇒ refused, typed).
pub fn acceptance_binder(
    principal: String,
    memory: Weak<MemoryManagerAdapter>,
    tl: Weak<TransparencyLogAdapter>,
) -> butler::AcceptanceBinder {
    Arc::new(move |pid_of: butler::PidSource| {
        let unresolved = || "the Spirit has no admitted pid yet".to_owned();
        let (memory, tl, port_pid) = (memory.clone(), tl.clone(), Arc::clone(&pid_of));
        butler::AcceptanceSeam {
            principal: principal.clone(),
            port_for: Arc::new(move || {
                let pid = port_pid().ok_or_else(unresolved)?;
                Ok(Arc::new(PidBoundPrincipalPort::new(memory.clone(), pid))
                    as Arc<dyn PrincipalMemoryPort>)
            }),
            attest: Arc::new(move |payload: &[u8]| {
                let pid = pid_of().ok_or_else(unresolved)?;
                let tl = tl.upgrade().ok_or("attestation: the host shut down")?;
                let _ = tl.insert_frame_event(
                    FrameKind::TelemetryEvent,
                    pid,
                    None,
                    butler::ACCEPTANCE_INTENT,
                    payload,
                    maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
                );
                Ok(())
            }),
        }
    })
}

/// The scheduler's live control blocks, keyed by pid.
pub type Scbs = std::sync::RwLock<std::collections::BTreeMap<u32, Arc<SpiritControlBlock>>>;

/// Story 18-5 (CR-3; the 18-4 TIA-1 lesson) — the root's context binder: each
/// reporter resolves the pid through the resolver the ROOT handed `attach` /
/// `construct`, and the window and thresholds from that pid's admitted bundle
/// (no `[budget]` ⇒ window 0, accounting off). Never from the Spirit.
pub fn context_binder(
    capability: &CapabilityRegistryAdapter,
    bus: &Arc<IacBusAdapter>,
    scbs: &Arc<Scbs>,
) -> butler::ContextBinder {
    let ledger = Arc::clone(capability.context_budget());
    let (bus, scbs) = (Arc::downgrade(bus), Arc::downgrade(scbs));
    Arc::new(move |pid_of: butler::PidSource| {
        let scbs = scbs.clone();
        Arc::new(ContextSignalAdapter {
            ledger: Arc::clone(&ledger),
            bus: bus.clone(),
            admitted: Arc::new(move || {
                let pid = pid_of()?;
                let scb = scbs.upgrade()?.read().ok()?.get(&pid).cloned()?;
                let window = context_window(scb.runtime_snapshot().manifest.budget.as_ref());
                let spirit_id = scb.spirit_id.as_str().into();
                Some(AdmittedSpirit {
                    pid,
                    spirit_id,
                    window,
                })
            }),
        }) as Arc<dyn ContextBudgetPort>
    })
}

/// Story 18-5 — an admitted bundle's window and thresholds (no `[budget]` ⇒
/// window 0: accounting off).
pub fn context_window(
    budget: Option<&maos_kernel_core::security::manifest::Budget>,
) -> ContextWindow {
    budget.map_or(ContextWindow::default(), |b| ContextWindow {
        size: b.context_window_size,
        pressure_permille: b.context_pressure_permille,
        limit_permille: b.context_limit_permille,
    })
}

/// Story 18-4 (review ruling D-18-4-R3) — harness-only: before each scene
/// day after the first (`later`), wait while `MAOS_SCENE_HOLD_FILE` names a
/// file, so a process witness can drive the door mid-scene.
pub async fn scene_hold(later: bool) {
    let held = || std::env::var_os("MAOS_SCENE_HOLD_FILE").is_some_and(|f| Path::new(&f).exists());
    while later && held() {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
