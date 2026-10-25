//! Story 18-1 (D-18-1-04 A; review BH-2/EC-2) — the `--once` verdict on a
//! Butler pass. In the library, not `main.rs`, so `crates/maos-bin/tests/`
//! drives the production function (an in-`src` test module is budget-charged).
//!
//! Story 18-4 — the pid-fused principal port (D-18-4-02 A). The scene's
//! per-day step is Spirit-side (`butler::Scene`, `Butler::scene_step`;
//! operator ruling 2026-10-25, kloc C).

use std::sync::{Arc, Weak};

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
