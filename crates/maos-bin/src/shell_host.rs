#![forbid(unsafe_code)]

//! Story 16-2 / D-16-2-B — the production [`ShellHost`].
//!
//! Implements `maos_shell::ShellHost` over the daemon's REAL kernel objects:
//! the Transparency Log, the shared Lifecycle Journal, the halt registry, the
//! memory manager and the capability registry. Nothing here writes a kernel
//! byte — every API driven is `pub` in `maos-kernel-core` (the 16-0 pin stays
//! `changed == 0`).
//!
//! The halt is raised through the kernel's `invoke_halt` — the ONE owner of
//! the `epistemic.halt` TL row, the Lifecycle Journal `Halt` entry and the
//! registry insert — and the halt id is a ULID, the kernel's own halt-id
//! grammar (§15 R9: one registry, one id grammar; 26 Crockford-base32 chars,
//! inside the door's `[A-Za-z0-9._-]{1,128}` rule).

use std::sync::Arc;

use maos_control::{
    submit_and_wait, OperatorCommand, OperatorCommandPort, OperatorOutcome, SubmitOutcome,
};
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::capability::CapError;
use maos_domain::ports::{CapabilityRegistryPort, MemoryManagerPort};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::MemoryManagerAdapter;

/// The halt this shell raises, and the policy id the kernel journals it under.
pub const AMBIGUITY_TAG: &str = "task.acceptance_criterion.ambiguous";
const AMBIGUITY_POLICY_ID: &str = "hello-spirit.ambiguity";
const AMBIGUITY_DERIVED_FROM: &str = "shell.directive";

/// D-16-2-B — the shell's kernel-side port.
pub struct ShellHost {
    spirit_pid: u32,
    boot_nonce: u64,
    default_provider: String,
    capability: Arc<CapabilityRegistryAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    shared_journal: Arc<JournalAdapter>,
    halt_registry: Arc<HaltRegistry>,
    memory: Arc<MemoryManagerAdapter>,
    /// The SAME operator door `maosctl halt resolve` reaches (D-16-2-C):
    /// one mechanism, two surfaces.
    door: Arc<dyn OperatorCommandPort>,
}

impl ShellHost {
    pub fn new(
        spirit_pid: u32,
        boot_nonce: u64,
        default_provider: String,
        capability: Arc<CapabilityRegistryAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        shared_journal: Arc<JournalAdapter>,
        halt_registry: Arc<HaltRegistry>,
        memory: Arc<MemoryManagerAdapter>,
        door: Arc<dyn OperatorCommandPort>,
    ) -> Self {
        Self {
            spirit_pid,
            boot_nonce,
            default_provider,
            capability,
            transparency_log,
            shared_journal,
            halt_registry,
            memory,
            door,
        }
    }

    /// The context key the kernel's `KernelHaltResolver` writes the resolution
    /// text under (`resolver.rs` — the Story 4.3 delivery channel).
    fn context_key(halt_id: &str) -> String {
        format!("halt_context::{halt_id}")
    }
}

impl maos_shell::ShellHost for ShellHost {
    fn spirit_pid(&self) -> u32 {
        self.spirit_pid
    }

    fn issue_turn_token(&self) -> Result<CapabilityToken, CapError> {
        self.capability.issue_with_mediation(
            self.spirit_pid,
            Scope::ProviderInfer {
                provider: self.default_provider.clone(),
            },
            60,        // ttl_secs
            [0u8; 32], // posture_hash (deterministic fallback)
            IntentClass::Standard,
        )
    }

    fn record_turn(&self, token: &CapabilityToken, payload: &[u8]) -> Result<(), CapError> {
        // Passes `record_invocation`'s result through UNCHANGED — the single
        // swallow stays at the REPL call site (the site 16-5 AC1 cites).
        self.capability
            .record_invocation(token, "shell.turn".into(), payload)
    }

    fn raise_ambiguity_halt(
        &self,
        tag: &str,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let halt_id = ulid::Ulid::new().to_string();
        let payload = EpistemicHaltPayload::new(
            halt_id.clone(),
            tag.to_string(),
            1.0,
            None,
            AMBIGUITY_POLICY_ID.to_string(),
            AMBIGUITY_DERIVED_FROM.to_string(),
        )?;
        invoke_halt(
            &self.transparency_log,
            &self.shared_journal,
            &self.halt_registry,
            payload,
            self.spirit_pid,
            "hello-spirit",
            self.boot_nonce,
        )?;
        // `prompt` is rendered by the REPL (after this Ok) — the kernel
        // payload carries the tag and provenance, not the REPL line.
        let _ = prompt;
        Ok(halt_id)
    }

    fn halt_state(&self, halt_id: &str) -> Option<HaltState> {
        let id = HaltId::new(halt_id.to_string()).ok()?;
        self.halt_registry.lookup_state(&id)
    }

    fn take_context(&self, halt_id: &str) -> Option<String> {
        match self.memory.read(
            self.spirit_pid,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            &Self::context_key(halt_id),
        ) {
            Ok(Some(MemoryValue::Text(text))) => Some(text),
            _ => None,
        }
    }

    fn resolve_with_context(
        &self,
        halt_id: &str,
        text: &str,
    ) -> Result<(), maos_shell::ResolveRefusal> {
        // D-16-2-C — the SAME `OperatorDoor` submission `maosctl halt resolve`
        // performs, in-process: same validation, same `KernelHaltResolver`,
        // same approval-log row, same completion TL row. ONE submit-and-
        // withdraw implementation (§15 R4) — neither surface copies the CAS.
        let command = OperatorCommand::ResolveHalt {
            spirit_id: "hello-spirit".into(),
            halt_id: halt_id.to_string(),
            resolution: "provided_context".into(),
            rationale: Some(text.to_string()),
        };
        match submit_and_wait(self.door.as_ref(), command) {
            SubmitOutcome::Completed(OperatorOutcome::Completed(_)) => Ok(()),
            // `HandlerStillRunning` means the port already CASed to STARTED:
            // the command WILL run and write its row, so the REPL's tick
            // observes the registry and renders (D-16-2-C) — nothing to say.
            SubmitOutcome::HandlerStillRunning { .. } => Ok(()),
            // `SpiritBusy` is the OPPOSITE: the withdraw CAS won, so the
            // command NEVER RAN (`maos_control::SubmitOutcome`) and the
            // registry will never flip — the tick has nothing to observe and
            // would render nothing, forever. Story 16-2 §A6 review: saying
            // nothing here silently consumes the operator's clarification.
            // §15 R8's rule ("say what happened to the text") applies to
            // this arm exactly as it does to `halt_already_resolved`.
            SubmitOutcome::SpiritBusy => Err(maos_shell::ResolveRefusal {
                code: "spirit_busy".into(),
                detail: "another operator command held hello-spirit past the route budget, \
                         so the clarification was withdrawn unrun — the halt is still \
                         pending; type it again"
                    .into(),
            }),
            SubmitOutcome::Completed(OperatorOutcome::NotFound { code, detail })
            | SubmitOutcome::Completed(OperatorOutcome::Conflict { code, detail })
            | SubmitOutcome::Completed(OperatorOutcome::Invalid { code, detail })
            | SubmitOutcome::Completed(OperatorOutcome::Failed { code, detail }) => {
                Err(maos_shell::ResolveRefusal { code, detail })
            }
            SubmitOutcome::Internal => Err(maos_shell::ResolveRefusal {
                code: "internal".into(),
                detail: "the operator-door handler closed without delivering an outcome".into(),
            }),
        }
    }
}

/// Story 16-2 / §15 R6 (D-16-2-D) — the shell session's closing act.
///
/// Called by `main` on BOTH arms of the shell body, in D-16-2-C's order —
/// AFTER the door is closed (`server.shutdown` + `drain_started_tasks`, so no
/// `maosctl halt resolve` races the drain) and immediately BEFORE the port
/// drop and the audit-writer drain. Takes `shell_result` and returns the
/// result `main` exits with, so the Err-arm exit contract is provable
/// in-process (`shell_host_16_2.rs`).
///
/// Story 16-3 (D-16-3-M) — the loop that used to live here is now
/// [`crate::supervision::unload_all_loaded`], the ONE unload function every
/// `maos run` root leaves through. The generalisation is a delegation, not a
/// rewrite: this function keeps its whole contract.
///
/// 1. Dry-run the Spirit's STILL-PENDING halts from the registry
///    (`drain_for_spirit_dry_run` — non-draining; the registry is the
///    evidence, no id has to survive a failed REPL).
/// 2. `scheduler.unload(pid)` — `terminate_spirit(PlannedUnload)` drains
///    every pending halt into a receipt row carrying its `halt_id` (NFR-Rel-11).
/// 3. Only for each dry-run id the registry no longer knows: print
///    `halt <id> closed by planned unload (no resolution)` — never before
///    the act.
pub async fn finish_shell_session(
    shell_result: Result<(), Box<dyn std::error::Error>>,
    scheduler: &maos_kernel_core::scheduler::SpiritSchedulerAdapter,
    halt_registry: &HaltRegistry,
    out: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    // If the shell never loaded the Spirit (pre-load error), there is nothing
    // to unload — the caller's result stands.
    if scheduler.resolve_pid("hello-spirit").is_none() {
        return shell_result;
    }
    let report = crate::supervision::unload_all_loaded(scheduler, halt_registry).await;
    // Story 16-2 §A6 review — the unload failure is REPORTED, never returned:
    // `shell_result` decides the exit, exactly as `main`'s own P13 invariant
    // demands ("the causal shell failure wins … never allowed to mask the
    // real cause"). This is not hypothetical: `is_transition_allowed` has no
    // `(Loaded, Unloaded)` arm (`scheduler/control_block.rs`), so when `load`
    // succeeded but `admit_spirit` did not, the SCB is still `Loaded` and
    // this unload ALWAYS fails — returning its error would replace the real
    // admission rejection with "invalid state transition" on precisely the
    // path where the operator needs the cause.
    for (pid, error) in &report.failed {
        eprintln!("maos shell: planned unload failed for pid {pid}: {error}");
    }
    if report.had_failures() {
        return shell_result;
    }
    for halt_id in &report.closed_halts {
        // Best-effort, like every other write on this seam: a closed
        // stdout (`maos shell | head`) must not panic out of the caller
        // and skip the audit-writer drain that follows.
        let _ = writeln!(
            out,
            "halt {halt_id} closed by planned unload (no resolution)"
        );
    }
    shell_result
}
