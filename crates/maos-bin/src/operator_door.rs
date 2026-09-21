#![forbid(unsafe_code)]

//! Story 16-1 (D-16-1-N) — the operator door's port implementation over the
//! daemon's REAL kernel objects.
//!
//! This is a LIB module, not a `mod` in `main.rs`, on purpose: the in-process
//! proofs (`crates/maos-bin/tests/operator_door_16_1.rs`) must drive the real
//! port — a private item of the binary crate cannot be named from `tests/`,
//! and an in-`src` test module is budget-charged and CI-invisible (the 14-2b
//! doctrine that moved `worker_spawn` out).
//!
//! Three measured invariants shape every handler:
//!
//! - **Kernel transition FIRST, rows only on success (D-16-1-L).** The HEAD
//!   one-shot arms wrote the Lifecycle Journal + approval-log rows FIRST and
//!   never called the scheduler — `maosctl pause butler` journaled a pause the
//!   running daemon never performed (§4 of the story). Every handler here
//!   runs the scheduler transition, and only on `Ok` writes the rows.
//! - **A command that STARTS runs to completion and writes its rows, never
//!   cancelled (D-16-1-E/F).** [`OperatorCommandPort::submit`] mints the
//!   `operation_id`, CASes the shared state `QUEUED → STARTED` only when the
//!   work is committed, and the spawned future delivers exactly one
//!   [`OperatorOutcome`]. A 503 `handler_still_running` id is therefore always
//!   findable: the completion TL row is written even when the outcome arrives
//!   after the server gave up.
//! - **Per-Spirit serialization (D-16-1-R).** Mutating commands on one Spirit
//!   are serialized behind a per-id async mutex so two operators cannot
//!   interleave journal and approval-log rows. The wait happens INSIDE the
//!   command's route budget: if the lock is not held by `deadline - 250 ms`,
//!   the future exits leaving the state at `QUEUED` — the server's CAS then
//!   wins and answers `spirit_busy` with no id, and the command never runs.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use maos_control::{
    OperatorCommand, OperatorCommandPort, OperatorOutcome, OperatorSubmission,
    UpgradePolicyRequest, COMMAND_QUEUED, COMMAND_STARTED,
};

/// Bin-private composition-root operations the LIB-side port cannot reach.
///
/// D-16-1-N: `run_uninstall_cascade`, `append_lifecycle_journal`,
/// `enforce_vetted_upgrade_precondition` and
/// `migration_plan::upgrade_with_plan_guard` drag bin-only wiring (the erasure
/// cascade, the daemon-held journal, the vetting preconditions, the successor
/// factory) that would widen the library surface if made `pub`. They enter
/// here instead, implemented in `main.rs`.
#[async_trait]
pub trait BinPrivateOps: Send + Sync + 'static {
    /// Append one Lifecycle Journal transition through the daemon-held
    /// journal, stamping `monotonic_now_ns`. The one-shot arms opened a FRESH
    /// adapter per write — the cross-process overwrite hazard routed to 16-5;
    /// the daemon path reuses the one open handle.
    async fn append_lifecycle_journal(
        &self,
        event: maos_domain::invariants::i10::LifecycleEvent,
        spirit_id: &str,
    ) -> Result<PathBuf, String>;

    /// The real erasure cascade (proof bundle, principal teardown, token
    /// revocation), after the door already unloaded the SCB. The terminal
    /// carries the preserved one-shot exit contract: 0/3/4/5.
    async fn run_uninstall_cascade(&self, spirit_id: &str) -> UninstallOutcome;

    /// Vetted-target upgrade precondition. `attestation`/`vetter_keyring` are
    /// PARAMETERS (D-16-1-W(4)) — never re-read from the daemon's
    /// environment, where `MAOS_UPGRADE_TO_ATTESTATION`/`MAOS_VETTER_KEYRING`
    /// are absent and one daemon serves many targets.
    async fn enforce_vetted_upgrade_precondition(
        &self,
        spirit_id: &str,
        target_manifest: &Path,
        attestation: Option<&str>,
        vetter_keyring: Option<&str>,
    ) -> Result<(), String>;

    /// `migration_plan::upgrade_with_plan_guard` over the daemon's real
    /// `UpgradeOrchestrator`.
    async fn upgrade_with_plan_guard(
        &self,
        spirit_id: &str,
        target_manifest: &Path,
        policy: maos_kernel_core::lifecycle::UpgradePolicy,
    ) -> Result<serde_json::Value, String>;

    /// `migration_plan::create_plan` — resolve, validate, hash and persist a
    /// multi-hop migration plan.
    ///
    /// The creation half of `maosctl spirit upgrade --plan`. It travels over
    /// the door because the EXECUTION half already does: the guard above
    /// reads whatever plan is persisted for this Spirit, so dropping creation
    /// would leave a guard nothing could arm and a shipped operator flag with
    /// no implementation.
    async fn create_migration_plan(
        &self,
        spirit_id: &str,
        from_version: &str,
        target_manifest: &Path,
        candidates: &[String],
    ) -> Result<serde_json::Value, String>;

    /// Story 16-6 — the `load → admit → start` triple over the daemon's real
    /// kernel composites.
    ///
    /// It enters through this seam for the reason every other method does:
    /// `DoorInner` holds neither `security` (the `SecurityManagerAdapter`
    /// that applies every gate that matters) nor `pid_by_spirit_id` (the map
    /// every id-resolving surface reads) nor the class-construction switch,
    /// and widening the library surface to reach them would put the
    /// composition root inside the door.
    ///
    /// ⚠ NOT `SpiritSchedulerAdapter::load`. The daemon constructs its
    /// scheduler with `security_manager: None`, so the kernel's own internal
    /// `admit_spirit` never runs here — calling it directly would admit the
    /// Spirit not at a lower sandbox tier but with no admission at all.
    ///
    /// ⚠ `#[cfg(feature = "network")]`: `crate::admission` is network-gated
    /// because Gate 8 (model provenance) calls
    /// `maos_registry::admission::validate_model_provenance`
    /// (`admission.rs:608`). That coupling is CORRECT — dropping the gate in an
    /// air-gap build would silently remove an admission check — so the door's
    /// load path is gated instead. Story 16-6 gated the module and not its
    /// consumers, which broke `cargo build -p maos-bin --no-default-features`
    /// and reddened the `check-exit-commands` JOB while its GATE stayed green
    /// (Epic-16 retrospective, 2026-09-21).
    #[cfg(feature = "network")]
    async fn load_spirit(
        &self,
        manifest: &Path,
        pread: Option<Arc<String>>,
    ) -> Result<crate::admission::Admitted, crate::admission::AdmissionRefusal>;
}

/// The uninstall terminal as the door renders it: the serialized
/// `UninstallCascadeTerminal` (serde tag `outcome`) plus the preserved
/// one-shot terminal code the client exits with.
#[derive(Debug, Clone)]
pub struct UninstallOutcome {
    pub body: serde_json::Value,
    pub terminal_code: i32,
}

/// Where the upgrade successor factory finds the target manifest path.
///
/// The kernel factory signature (`SuccessorSpiritFactory::create`) receives
/// only the PARSED bundle, which carries no source path — but the butler arm
/// must re-parse `[epistemic_policy]` from the target file to build a FAITHFUL
/// successor (a bare `Butler::new()` completes the swap and silently loses the
/// halt — the AC4 root-E falsifier). The door stages the path here just before
/// invoking the upgrade and clears it after.
#[derive(Clone, Default)]
pub struct SuccessorManifestSlot(Arc<Mutex<Option<PathBuf>>>);

impl SuccessorManifestSlot {
    pub fn stage(&self, path: PathBuf) {
        *self.0.lock().expect(
            "successor manifest slot: poisoned — a prior panic left upgrade staging inconsistent",
        ) = Some(path);
    }

    pub fn take(&self) -> Option<PathBuf> {
        self.0
            .lock()
            .expect(
                "successor manifest slot: poisoned — a prior panic left upgrade staging inconsistent",
            )
            .take()
    }

    /// Read WITHOUT clearing: the factory may be consulted per hop while the
    /// door owns the slot's lifecycle (it clears with [`Self::take`]).
    pub fn peek(&self) -> Option<PathBuf> {
        self.0
            .lock()
            .expect(
                "successor manifest slot: poisoned — a prior panic left upgrade staging inconsistent",
            )
            .clone()
    }
}

/// Production per-Spirit lock registry used by [`OperatorDoor`].
///
/// Public only so the integration proof can exercise the exact lock registry
/// without constructing unrelated kernel adapters.
#[doc(hidden)]
#[derive(Default)]
pub struct SpiritCommandLocks {
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl SpiritCommandLocks {
    pub fn spirit_lock(&self, spirit_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self
            .locks
            .lock()
            .expect("spirit lock map: poisoned — a prior panic left serialization inconsistent");
        Arc::clone(locks.entry(spirit_id.to_string()).or_default())
    }
}

/// The door's kernel handles. One `Arc` around all of them so `submit` moves a
/// single clone into the spawned kernel future.
struct DoorInner {
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    orchestrator_registry: Arc<maos_kernel_core::orchestrator::OrchestratorBufferRegistry>,
    revocation_applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
    hot_swap_coordinator: Arc<maos_kernel_core::hot_swap::HotSwapCoordinator>,
    capability: Arc<maos_kernel_core::api::CapabilityRegistryAdapter>,
    memory: Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    orchestrator:
        Arc<maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator>,
    mailbox: Arc<maos_kernel_core::iac::Mailbox>,
    notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
    transparency_log: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    crypto_provider: Arc<dyn maos_domain::ports::crypto::CryptoProvider>,
    /// Cloned at boot BEFORE the anchor moves into `LocalFileRegistryClient`
    /// (D-16-1-X): re-reading `MAOS_CRL_TRUST_ANCHOR_PUB_HEX` per request would
    /// let the anchor change under a running daemon.
    crl_trust_anchor: Option<Vec<u8>>,
    /// Per-CRL (matched, revoked) counts of CRLs applied THROUGH this door.
    /// `RevocationApplier::list_applied` retains only ids, but the operator
    /// read route must report real occupancy, not zeros.
    crl_reports: Mutex<BTreeMap<[u8; 32], (usize, usize)>>,
    daemon_pid: u32,
    boot_nonce: u64,
    runtime: tokio::runtime::Handle,
    private_ops: Arc<dyn BinPrivateOps>,
    successor_manifest_slot: SuccessorManifestSlot,
    upgrade_lock: tokio::sync::Mutex<()>,
    output_markers: Arc<maos_kernel_core::halt::OutputMarkerRegistry>,
    spirit_locks: SpiritCommandLocks,
    command_tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

/// `maosctl`-visible director identity on every approval-log row (FR42 keeps
/// the `"director"` actor label the one-shot arms wrote).
const DIRECTOR: &str = "director";

/// The `maos_control::OperatorCommandPort` over the daemon's live kernel.
///
/// Always constructed behind an `Arc` (the server takes
/// `Arc<dyn OperatorCommandPort>`); the inner `Arc` makes `submit`'s hand-off
/// to the spawned future a single clone.
pub struct OperatorDoor(Arc<DoorInner>);

impl OperatorDoor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
        policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
        halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
        output_markers: Arc<maos_kernel_core::halt::OutputMarkerRegistry>,
        orchestrator_registry: Arc<maos_kernel_core::orchestrator::OrchestratorBufferRegistry>,
        revocation_applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
        hot_swap_coordinator: Arc<maos_kernel_core::hot_swap::HotSwapCoordinator>,
        capability: Arc<maos_kernel_core::api::CapabilityRegistryAdapter>,
        memory: Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
        orchestrator: Arc<
            maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator,
        >,
        mailbox: Arc<maos_kernel_core::iac::Mailbox>,
        notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
        transparency_log: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
        crypto_provider: Arc<dyn maos_domain::ports::crypto::CryptoProvider>,
        crl_trust_anchor: Option<Vec<u8>>,
        boot_nonce: u64,
        runtime: tokio::runtime::Handle,
        private_ops: Arc<dyn BinPrivateOps>,
        successor_manifest_slot: SuccessorManifestSlot,
    ) -> Self {
        Self(Arc::new(DoorInner {
            scheduler,
            policy,
            halt_registry,
            orchestrator_registry,
            revocation_applier,
            hot_swap_coordinator,
            capability,
            memory,
            orchestrator,
            mailbox,
            notification_dispatcher,
            transparency_log,
            crypto_provider,
            crl_trust_anchor,
            crl_reports: Mutex::new(BTreeMap::new()),
            daemon_pid: std::process::id(),
            boot_nonce,
            runtime,
            private_ops,
            successor_manifest_slot,
            upgrade_lock: tokio::sync::Mutex::new(()),
            output_markers,
            spirit_locks: SpiritCommandLocks::default(),
            command_tasks: Mutex::new(Vec::new()),
        }))
    }

    /// The per-Spirit serialization lock (D-16-1-R). Exposed so the in-process
    /// proof can hold one Spirit busy past a second command's deadline and
    /// observe the never-runs contract against the REAL port.
    pub fn spirit_lock(&self, spirit_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.0.spirit_lock(spirit_id)
    }

    /// Await every command that reached the runtime. Call only after the HTTP
    /// server has stopped accepting and joined its connection workers.
    pub async fn drain_started_tasks(&self) {
        let tasks = {
            let mut tasks = self
                .0
                .command_tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::mem::take(&mut *tasks)
        };
        for task in tasks {
            if let Err(error) = task.await {
                eprintln!("maos: operator command task failed during drain: {error}");
            }
        }
    }

    /// Daemon-lifetime output markers emitted by authorized halt overrides.
    pub fn output_markers(&self) -> Arc<maos_kernel_core::halt::OutputMarkerRegistry> {
        Arc::clone(&self.0.output_markers)
    }
}

impl DoorInner {
    fn spirit_lock(&self, spirit_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.spirit_locks.spirit_lock(spirit_id)
    }

    fn not_found(code: &str, detail: String) -> OperatorOutcome {
        OperatorOutcome::NotFound {
            code: code.to_string(),
            detail,
        }
    }

    fn spirit_not_loaded(spirit_id: &str) -> OperatorOutcome {
        Self::not_found(
            "spirit_not_loaded",
            format!("spirit '{spirit_id}' is not loaded in this daemon"),
        )
    }

    /// Dispatch one STARTED command to its handler. Called only after the
    /// per-Spirit lock is held and the `QUEUED → STARTED` CAS won.
    async fn dispatch(
        &self,
        command: OperatorCommand,
        pread: Option<Arc<String>>,
    ) -> OperatorOutcome {
        match command {
            OperatorCommand::Start { spirit_id } => {
                self.lifecycle_command(
                    "start",
                    maos_domain::invariants::i10::LifecycleEvent::Start,
                    &spirit_id,
                )
                .await
            }
            OperatorCommand::Pause { spirit_id } => {
                self.lifecycle_command(
                    "pause",
                    maos_domain::invariants::i10::LifecycleEvent::Pause,
                    &spirit_id,
                )
                .await
            }
            OperatorCommand::Resume { spirit_id } => {
                self.lifecycle_command(
                    "resume",
                    maos_domain::invariants::i10::LifecycleEvent::Resume,
                    &spirit_id,
                )
                .await
            }
            OperatorCommand::Unload { spirit_id } => {
                self.lifecycle_command(
                    "unload",
                    maos_domain::invariants::i10::LifecycleEvent::Unload,
                    &spirit_id,
                )
                .await
            }
            OperatorCommand::Posture { spirit_id, posture } => {
                self.posture_command(&spirit_id, &posture).await
            }
            OperatorCommand::Upgrade {
                spirit_id,
                target_manifest,
                policy,
                attestation,
                vetter_keyring,
                from_version,
                candidates,
                create_plan,
            } => {
                self.upgrade_command(
                    &spirit_id,
                    &target_manifest,
                    policy,
                    attestation.as_deref(),
                    vetter_keyring.as_deref(),
                    from_version.as_deref(),
                    &candidates,
                    create_plan,
                )
                .await
            }
            OperatorCommand::HotSwapPrecheck {
                spirit_id,
                target_manifest,
            } => {
                self.precheck_command(&spirit_id, target_manifest.as_deref())
                    .await
            }
            OperatorCommand::Uninstall { spirit_id } => self.uninstall_command(&spirit_id).await,
            OperatorCommand::ResolveHalt {
                spirit_id,
                halt_id,
                resolution,
                rationale,
            } => {
                self.resolve_halt_command(&spirit_id, &halt_id, &resolution, rationale.as_deref())
                    .await
            }
            OperatorCommand::OrchestratorEnqueue { spirit_id, text } => {
                self.orchestrator_enqueue_command(&spirit_id, &text).await
            }
            OperatorCommand::RevokeToken { token_id } => self.revoke_token_command(&token_id).await,
            OperatorCommand::ImportRevocations { crl } => {
                self.import_revocations_command(&crl).await
            }
            OperatorCommand::ForgetMemory { principal, reason } => {
                self.forget_command(&principal, reason.as_deref()).await
            }
            OperatorCommand::ReleaseLegalHold { principal } => {
                self.release_legal_hold_command(&principal).await
            }
            OperatorCommand::AdmitGovernanceSchema { schema } => {
                self.admit_governance_schema_command(schema).await
            }
            #[cfg(feature = "network")]
            OperatorCommand::Load { manifest } => self.load_command(&manifest, pread).await,
            // Air-gap build: the load verb needs the provenance gate, and the
            // gate needs maos-registry. Refuse LOUDLY rather than admit a
            // Spirit with one fewer check than the operator was promised.
            #[cfg(not(feature = "network"))]
            OperatorCommand::Load { .. } => OperatorOutcome::Conflict {
                code: "load_unavailable".into(),
                detail: "this binary was built without the `network` feature, so the model-\
                         provenance admission gate (maos-registry) is absent; loading a Spirit \
                         here would skip a check the operator was promised"
                    .into(),
            },
        }
    }

    // ── handlers ─────────────────────────────────────────────────────────

    /// `start`/`pause`/`resume`/`unload` (D-16-1-L): scheduler transition
    /// FIRST; journal + approval rows only on success. `pause` on a Paused
    /// SCB is `InvalidStateTransition` — mapped honestly to `Conflict`,
    /// never retried (Trap 4: the state is committed before a failing
    /// `on_pause` hook returns, so a retry cannot un-see it).
    async fn lifecycle_command(
        &self,
        verb: &str,
        event: maos_domain::invariants::i10::LifecycleEvent,
        spirit_id: &str,
    ) -> OperatorOutcome {
        use maos_domain::lifecycle::LifecycleError;

        let Some(pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        // ⚠ D-16-1-J — `start` is `Loaded → Running` and NOTHING else, and the
        // kernel's table alone does not give that: `is_transition_allowed`
        // keys on the TARGET state, so it also permits `Paused → Running`.
        // Measured against a live butler: `maosctl start butler` on a PAUSED
        // Spirit returned 200 `Running` and journaled a `Start` row — the same
        // class of lie as the retired `stop` verb journaling a `Halt`. An
        // operator who paused a Spirit and then "started" it must be told
        // which verb actually resumes it.
        if verb == "start" {
            let current = self.scb_state(spirit_id);
            if current
                != Some(maos_kernel_core::scheduler::control_block::ScbLifecycleState::Loaded)
            {
                return OperatorOutcome::Conflict {
                    code: "invalid_state_transition".into(),
                    detail: match current {
                        Some(
                            maos_kernel_core::scheduler::control_block::ScbLifecycleState::Paused,
                        ) => format!(
                            "spirit '{spirit_id}' is Paused, not Loaded — use `maosctl resume` \
                             to return it to Running"
                        ),
                        Some(state) => format!(
                            "spirit '{spirit_id}' is in state {state:?}; start only moves a \
                             Loaded Spirit to Running"
                        ),
                        None => format!("spirit '{spirit_id}' has no control block"),
                    },
                };
            }
        }
        let transitioned = match verb {
            "start" => self.scheduler.start(pid).await,
            "pause" => self.scheduler.pause(pid).await,
            "resume" => self.scheduler.resume(pid).await,
            "unload" => self.scheduler.unload(pid).await,
            _ => unreachable!("lifecycle_command is only called with the four scheduler verbs"),
        };
        if let Err(error) = transitioned {
            return match &error {
                LifecycleError::InvalidStateTransition { current, verb, .. } => {
                    OperatorOutcome::Conflict {
                        code: "invalid_state_transition".into(),
                        detail: format!(
                            "spirit '{spirit_id}' is in state {current:?}, cannot execute verb {verb:?}"
                        ),
                    }
                }
                LifecycleError::NotLoaded { .. } => Self::spirit_not_loaded(spirit_id),
                LifecycleError::HookBudgetExceeded { hook_name, .. } => OperatorOutcome::Failed {
                    code: "hook_budget_exceeded".into(),
                    detail: format!("spirit '{spirit_id}': hook {hook_name} exceeded its budget"),
                },
                other => OperatorOutcome::Failed {
                    code: "lifecycle_failed".into(),
                    detail: other.to_string(),
                },
            };
        }

        if let Err(error) = self
            .private_ops
            .append_lifecycle_journal(event, spirit_id)
            .await
        {
            return OperatorOutcome::Failed {
                code: "journal_failed".into(),
                detail: error,
            };
        }
        if let Err(error) = maos_kernel_core::orchestrator::journal_director_lifecycle_action(
            &self.transparency_log,
            DIRECTOR,
            spirit_id,
            verb,
        ) {
            return OperatorOutcome::Failed {
                code: "approval_log_failed".into(),
                detail: error.to_string(),
            };
        }

        let lifecycle_state = self
            .scb_state(spirit_id)
            .map(|state| format!("{state:?}"))
            .unwrap_or_else(|| "Unloaded".into());
        OperatorOutcome::Completed(serde_json::json!({
            "spirit_id": spirit_id,
            "verb": verb,
            "lifecycle_state": lifecycle_state,
        }))
    }
    /// Story 16-6 — `POST /v1/spirits`: admit a Spirit into the running
    /// daemon and leave it in `Loaded`.
    ///
    /// It does NOT start the Spirit. `load` and `start` are two of FR9's five
    /// verbs, and collapsing them would leave `maosctl start` with no
    /// operator-creatable subject — which is the state it has been in since
    /// 16-1 shipped it, because every root loads, admits and starts in one
    /// unbroken run before the serving loop this door answers from.
    ///
    /// Serialization: the per-Spirit lock for this id was taken by
    /// `run_command` BEFORE the `QUEUED → STARTED` CAS, using the id peeked
    /// out of the manifest — so a contended pair behaves exactly like every
    /// other verb (the loser is withdrawn as `spirit_busy` within
    /// `lock_wait`), and the kernel's own non-atomic `AlreadyLoaded` guard
    /// (a `resolve_pid` read and an `insert` in separate acquisitions, with
    /// the whole of `admit_spirit` between them) cannot be raced through this
    /// door.
    #[cfg(feature = "network")]
    async fn load_command(&self, manifest: &str, pread: Option<Arc<String>>) -> OperatorOutcome {
        use crate::admission::AdmissionRefusal;

        let path = Path::new(manifest);
        // D-16-1-X — the client canonicalises; the daemon refuses to guess.
        // A relative path here would resolve in the DAEMON's working
        // directory, which is not the operator's.
        if !path.is_absolute() {
            return OperatorOutcome::Invalid {
                code: "manifest_not_canonical".into(),
                detail: format!(
                    "manifest path '{manifest}' is not absolute; the client canonicalises \
                     before sending because the daemon's working directory is not yours"
                ),
            };
        }

        let admitted = match self.private_ops.load_spirit(path, pread).await {
            Ok(admitted) => admitted,
            Err(refusal) => {
                let code = refusal.code().to_owned();
                let detail = refusal.to_string();
                // An id already in the scheduler is a STATE conflict (409);
                // every other refusal is a bad request (400). A load that is
                // refused has left nothing behind either way — the triple
                // rolls back its own partial admission.
                return if refusal.is_already_loaded() {
                    OperatorOutcome::Conflict { code, detail }
                } else {
                    match refusal {
                        AdmissionRefusal::Start { .. } => OperatorOutcome::Failed { code, detail },
                        _ => OperatorOutcome::Invalid { code, detail },
                    }
                };
            }
        };

        let mut payload = serde_json::json!({
            "spirit_id": admitted.spirit_id,
            "verb": "load",
            "pid": admitted.pid,
            "lifecycle_state": self
                .scb_state(&admitted.spirit_id)
                .map(|state| format!("{state:?}"))
                .unwrap_or_else(|| "Unloaded".into()),
            "effective_sandbox_tier": format!("{:?}", admitted.effective_sandbox_tier),
            "requested_posture_ceiling": format!("{:?}", admitted.requested_posture_ceiling),
            "effective_posture_ceiling": format!("{:?}", admitted.effective_posture_ceiling),
        });
        if let Err(error) = self
            .private_ops
            .append_lifecycle_journal(
                maos_domain::invariants::i10::LifecycleEvent::Load,
                &admitted.spirit_id,
            )
            .await
        {
            eprintln!(
                "maos: WARNING — load for '{}' admitted but lifecycle journal failed: {error}",
                admitted.spirit_id
            );
            payload["journal_error"] = serde_json::Value::String(error);
        }
        if let Err(error) = maos_kernel_core::orchestrator::journal_director_lifecycle_action(
            &self.transparency_log,
            DIRECTOR,
            &admitted.spirit_id,
            "load",
        ) {
            eprintln!(
                "maos: WARNING — load for '{}' admitted but approval log failed: {error}",
                admitted.spirit_id
            );
            payload["approval_log_error"] = serde_json::Value::String(error.to_string());
        }
        OperatorOutcome::Completed(payload)
    }

    /// Posture shift (D-16-1-L): resolve → `policy.shift_posture` → journal
    /// `PostureShift` → approval row. The `from` value is read from the SAME
    /// `PolicyTable` snapshot `evaluate_with_posture` reads — never a journal
    /// row and never a manifest re-parse (the HEAD arm re-admitted pid 0 from
    /// a CWD-relative manifest, which touched nothing the daemon runs).
    async fn posture_command(&self, spirit_id: &str, posture: &str) -> OperatorOutcome {
        let Some(pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        let new_posture = match posture {
            "cautious" => maos_kernel_core::security::manifest::Posture::Cautious,
            "assistive" => maos_kernel_core::security::manifest::Posture::Assistive,
            "autonomous-with-halt" => {
                maos_kernel_core::security::manifest::Posture::AutonomousWithHalt
            }
            other => {
                return OperatorOutcome::Invalid {
                    code: "unknown_posture".into(),
                    detail: format!(
                    "unknown posture '{other}' — expected cautious|assistive|autonomous-with-halt"
                ),
                }
            }
        };
        let previous = self
            .policy
            .inner()
            .load_full()
            .spirit_postures
            .get(&pid)
            .map(|state| state.current);
        match self.policy.shift_posture(pid, new_posture) {
            Err(maos_kernel_core::security::posture::PostureError::UnknownSpirit(_)) => {
                Self::spirit_not_loaded(spirit_id)
            }
            Err(maos_kernel_core::security::posture::PostureError::AboveCeiling {
                allowed,
                ..
            }) => OperatorOutcome::Conflict {
                code: "above_posture_ceiling".into(),
                detail: format!(
                    "spirit '{spirit_id}' ceiling is {allowed:?}; '{posture}' exceeds it"
                ),
            },
            Err(error) => OperatorOutcome::Conflict {
                code: "posture_shift_failed".into(),
                detail: error.to_string(),
            },
            Ok(hash) => {
                if let Err(error) = self
                    .private_ops
                    .append_lifecycle_journal(
                        maos_domain::invariants::i10::LifecycleEvent::PostureShift,
                        spirit_id,
                    )
                    .await
                {
                    return OperatorOutcome::Failed {
                        code: "journal_failed".into(),
                        detail: error,
                    };
                }
                if let Err(error) = maos_kernel_core::security::posture::journal_posture_shift(
                    &self.transparency_log,
                    DIRECTOR,
                    spirit_id,
                    previous.unwrap_or(maos_kernel_core::security::manifest::Posture::Assistive),
                    new_posture,
                ) {
                    return OperatorOutcome::Failed {
                        code: "approval_log_failed".into(),
                        detail: error.to_string(),
                    };
                }
                OperatorOutcome::Completed(serde_json::json!({
                    "spirit_id": spirit_id,
                    "previous_posture": previous.map(|p| format!("{p:?}")),
                    "posture": format!("{new_posture:?}"),
                    "posture_hash": hex::encode(hash),
                }))
            }
        }
    }

    /// Upgrade over the door (D-16-1-W): hot-swap only, forward only,
    /// faithful successor. Attestation/keyring arrive as command PARAMETERS.
    #[allow(clippy::too_many_arguments)]
    async fn upgrade_command(
        &self,
        spirit_id: &str,
        target_manifest: &str,
        policy: UpgradePolicyRequest,
        attestation: Option<&str>,
        vetter_keyring: Option<&str>,
        from_version: Option<&str>,
        candidates: &[String],
        create_plan: bool,
    ) -> OperatorOutcome {
        if policy == UpgradePolicyRequest::ColdSwap {
            // The kernel's ColdSwap arm starts the successor under a NEW pid
            // without `admit_spirit` — a kernel byte routed to the Epic-16
            // retrospective; the door refuses it typed instead of leaving an
            // unadmitted Spirit running.
            return OperatorOutcome::Conflict {
                code: "cold_swap_unsupported".into(),
                detail: "cold-swap over the door is unsupported: it would start an \
                         unadmitted successor under a new pid"
                    .into(),
            };
        }
        let Some(_pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        let target_path = PathBuf::from(target_manifest);
        let target = match std::fs::read(&target_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "target_manifest_unreadable".into(),
                    detail: format!(
                        "read upgrade target manifest {}: {error}",
                        target_path.display()
                    ),
                }
            }
        };
        let target_identity = match manifest_class_identity(&target) {
            Ok(identity) => identity,
            Err(detail) => {
                return OperatorOutcome::Invalid {
                    code: "target_manifest_unparseable".into(),
                    detail,
                }
            }
        };
        let (target_name, target_version) = target_identity;
        if target_name != spirit_id {
            return OperatorOutcome::Invalid {
                code: "target_manifest_mismatch".into(),
                detail: format!(
                    "upgrade target manifest names '{target_name}', expected '{spirit_id}'"
                ),
            };
        }
        // Forward-only comparison uses SemVer, including prerelease ordering.
        let target_semver = match semver::Version::parse(&target_version) {
            Ok(version) => version,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "target_version_invalid".into(),
                    detail: format!("successor version {target_version:?} is not SemVer: {error}"),
                }
            }
        };
        if let Some(loaded) = self.loaded_version(spirit_id) {
            let loaded_semver = match semver::Version::parse(&loaded) {
                Ok(version) => version,
                Err(error) => {
                    return OperatorOutcome::Failed {
                        code: "loaded_version_invalid".into(),
                        detail: format!("loaded version {loaded:?} is not SemVer: {error}"),
                    }
                }
            };
            if target_semver <= loaded_semver {
                return OperatorOutcome::Conflict {
                    code: "version_not_increasing".into(),
                    detail: format!(
                        "successor version {target_version} does not increase the loaded {loaded}"
                    ),
                };
            }
        }
        if let Err(rejection) = self
            .private_ops
            .enforce_vetted_upgrade_precondition(
                spirit_id,
                &target_path,
                attestation,
                vetter_keyring,
            )
            .await
        {
            return OperatorOutcome::Invalid {
                code: "vetting_precondition_failed".into(),
                detail: rejection,
            };
        }
        // Plan creation is a plan-only operation. The persisted plan is
        // consumed by a later upgrade command's guard.
        if create_plan {
            let from = from_version
                .map(str::to_owned)
                .or_else(|| self.loaded_version(spirit_id));
            let Some(from) = from else {
                return OperatorOutcome::Invalid {
                    code: "from_version_required".into(),
                    detail: "the loaded manifest declares no [class] version, so a \
                             migration plan needs an explicit from_version"
                        .into(),
                };
            };
            return match self
                .private_ops
                .create_migration_plan(spirit_id, &from, &target_path, candidates)
                .await
            {
                Ok(plan) => OperatorOutcome::Completed(serde_json::json!({
                    "spirit_id": spirit_id,
                    "outcome": "planned",
                    "migration_plan": plan,
                })),
                Err(detail) => OperatorOutcome::Invalid {
                    code: "migration_plan_rejected".into(),
                    detail,
                },
            };
        }
        // The successor factory reads one process-wide slot, so upgrades of
        // different Spirits must not overlap this stage/use/clear sequence.
        let _upgrade_guard = self.upgrade_lock.lock().await;
        self.successor_manifest_slot.stage(target_path.clone());
        let guarded = self
            .private_ops
            .upgrade_with_plan_guard(
                spirit_id,
                &target_path,
                maos_kernel_core::lifecycle::UpgradePolicy::HotSwap,
            )
            .await;
        self.successor_manifest_slot.take();
        match guarded {
            Err(error) => OperatorOutcome::Failed {
                code: "upgrade_failed".into(),
                detail: error,
            },
            Ok(reports) => OperatorOutcome::Completed(serde_json::json!({
                "spirit_id": spirit_id,
                "policy": "hot-swap",
                "reports": reports,
            })),
        }
    }

    /// Precheck the LOADED SCB through the daemon's real coordinator. Writes
    /// NO `lifecycle.load`/`lifecycle.admit` row — the HEAD arm loaded a
    /// placeholder Spirit into a fresh scheduler, appending phantom rows to
    /// the SHARED TL.
    async fn precheck_command(
        &self,
        spirit_id: &str,
        target_manifest: Option<&str>,
    ) -> OperatorOutcome {
        let Some(_pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        let Some(target_manifest) = target_manifest else {
            return OperatorOutcome::Invalid {
                code: "target_manifest_required".into(),
                detail: "hot-swap precheck needs a candidate successor manifest".into(),
            };
        };
        let target_path = PathBuf::from(target_manifest);
        if let Err(rejection) = self
            .private_ops
            .enforce_vetted_upgrade_precondition(spirit_id, &target_path, None, None)
            .await
        {
            // The one-shot printed the verdict and exited 2; over the door the
            // verdict is the 200 body and the client maps the terminal code.
            return OperatorOutcome::Completed(serde_json::json!({
                "spirit_id": spirit_id,
                "target_manifest": target_manifest,
                "verdict": { "verdict": "VettingPreconditionFailed", "cause": rejection },
                "safe": false,
                "terminal_code": 2,
            }));
        }
        let from_version = self
            .loaded_version(spirit_id)
            .unwrap_or_else(|| "0.0.0".into());
        match self
            .hot_swap_coordinator
            .precheck(spirit_id, target_manifest, &from_version)
        {
            Err(error) => OperatorOutcome::Invalid {
                code: "precheck_failed".into(),
                detail: error.to_string(),
            },
            Ok(verdict) => {
                let safe = matches!(
                    verdict.verdict,
                    maos_domain::hot_swap::PrecheckOutcome::SafeDrained
                        | maos_domain::hot_swap::PrecheckOutcome::SafeMigrated
                );
                OperatorOutcome::Completed(serde_json::json!({
                    "spirit_id": spirit_id,
                    "target_manifest": target_manifest,
                    "verdict": verdict,
                    "safe": safe,
                    "terminal_code": if safe { 0 } else { 2 },
                }))
            }
        }
    }

    /// Uninstall over the door (D-16-1-L): unload the SCB FIRST (revoking
    /// live tokens, draining halts, planned-unload receipts), then the real
    /// erasure cascade. The 0/3/4/5 terminal contract is preserved in the
    /// body's `terminal_code`.
    async fn uninstall_command(&self, spirit_id: &str) -> OperatorOutcome {
        let Some(pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        if let Err(error) = self.scheduler.unload(pid).await {
            return OperatorOutcome::Failed {
                code: "unload_failed".into(),
                detail: error.to_string(),
            };
        }
        let outcome = self.private_ops.run_uninstall_cascade(spirit_id).await;
        OperatorOutcome::Completed(outcome.body)
    }

    /// Halt resolution through the daemon's real `HaltRegistry` +
    /// `KernelHaltResolver` + `HaltFlow::submit_resolution`. The HEAD arm's
    /// fabricated pending halt — tag `"test"`, value `0.5`, policy
    /// `"test-policy"`, seeded so the arm could resolve it — is DELETED, not
    /// moved: a halt id that is not pending is a typed 404 and NO resolution
    /// row (`submit_resolution` is fail-closed: resolver before journal).
    async fn resolve_halt_command(
        &self,
        spirit_id: &str,
        halt_id: &str,
        resolution: &str,
        rationale: Option<&str>,
    ) -> OperatorOutcome {
        use maos_director_surface::halt_ui::{HaltFlow, HaltUiError};
        use maos_domain::halt::ResolveError;
        use maos_kernel_core::halt::KernelHaltResolver;

        let Some(_pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        let Ok(halt_id) = maos_domain::halt::HaltId::new(halt_id.to_string()) else {
            return OperatorOutcome::Invalid {
                code: "invalid_halt_id".into(),
                detail: format!("'{halt_id}' is not a valid halt id"),
            };
        };
        if let Some(pending) = self.halt_registry.lookup_pending_metadata(&halt_id) {
            if pending.spirit_id != spirit_id {
                return OperatorOutcome::Conflict {
                    code: "halt_spirit_mismatch".into(),
                    detail: format!(
                        "halt '{}' belongs to spirit '{}', not '{}'",
                        halt_id.as_str(),
                        pending.spirit_id,
                        spirit_id
                    ),
                };
            }
        }
        let parsed = match resolution {
            "provided_context" => {
                let Some(text) = rationale else {
                    return OperatorOutcome::Invalid {
                        code: "rationale_required".into(),
                        detail: "provided_context requires a rationale carrying the context text"
                            .into(),
                    };
                };
                maos_domain::halt::Resolution::provided_context(text)
            }
            "accepted_halt" => Ok(maos_domain::halt::Resolution::AcceptedHalt),
            "authorized_override" => {
                let Some(policy_ref) = rationale else {
                    return OperatorOutcome::Invalid {
                        code: "rationale_required".into(),
                        detail:
                            "authorized_override requires a rationale naming the operator policy"
                                .into(),
                    };
                };
                maos_domain::halt::Resolution::authorized_override(policy_ref)
            }
            other => {
                return OperatorOutcome::Invalid {
                    code: "unknown_resolution".into(),
                    detail: format!(
                        "unknown halt resolution '{other}' — expected \
                         provided_context|accepted_halt|authorized_override"
                    ),
                }
            }
        };
        let resolution = match parsed {
            Ok(value) => value,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "invalid_resolution".into(),
                    detail: error.to_string(),
                }
            }
        };

        let kernel_resolver = Arc::new(KernelHaltResolver::new(
            Arc::clone(&self.halt_registry),
            Arc::clone(&self.transparency_log),
            Arc::clone(&self.output_markers),
            Arc::clone(&self.mailbox),
            self.boot_nonce,
            Arc::clone(&self.memory),
            Arc::clone(&self.orchestrator),
        ));
        let rollback_metadata = self.halt_registry.lookup_pending_metadata(&halt_id);
        let halt_flow = HaltFlow::new(
            Arc::clone(&kernel_resolver),
            Arc::clone(&self.notification_dispatcher),
            Arc::clone(&self.transparency_log) as Arc<dyn maos_domain::halt::HaltJournal>,
        );
        let resolution_result =
            halt_flow.submit_resolution(halt_id.clone(), resolution.clone(), spirit_id);
        if matches!(&resolution_result, Err(HaltUiError::Audit(_))) {
            self.halt_registry
                .rollback_resolution(&halt_id, rollback_metadata);
        }
        match resolution_result {
            Err(HaltUiError::Resolver(ResolveError::UnknownHalt(error))) => {
                Self::not_found("halt_not_pending", error)
            }
            Err(HaltUiError::Resolver(ResolveError::AlreadyResolved(error))) => {
                OperatorOutcome::Conflict {
                    code: "halt_already_resolved".into(),
                    detail: error,
                }
            }
            Err(HaltUiError::Resolver(ResolveError::InvalidResolution(error))) => {
                OperatorOutcome::Invalid {
                    code: "invalid_resolution".into(),
                    detail: error.to_string(),
                }
            }
            Err(HaltUiError::InvalidResolution(error)) => OperatorOutcome::Invalid {
                code: "invalid_resolution".into(),
                detail: error.to_string(),
            },
            Err(HaltUiError::Resolver(ResolveError::Internal(error))) => OperatorOutcome::Failed {
                code: "halt_resolution_internal".into(),
                detail: error,
            },
            Err(error) => OperatorOutcome::Failed {
                code: "halt_resolution_failed".into(),
                detail: error.to_string(),
            },
            Ok(()) => OperatorOutcome::Completed(serde_json::json!({
                "spirit_id": spirit_id,
                "halt_id": halt_id.as_str(),
                "resolution": resolution.kind_label(),
            })),
        }
    }

    /// Enqueue onto the daemon's REAL per-Spirit Orchestrator buffer — the
    /// same registry the scheduler's resume arm recalls from, so status
    /// reports real occupancy and not a fresh `0/32`.
    async fn orchestrator_enqueue_command(&self, spirit_id: &str, text: &str) -> OperatorOutcome {
        use maos_spirit_abi::identity::SpiritId;

        let Some(_pid) = self.scheduler.resolve_pid(spirit_id) else {
            return Self::spirit_not_loaded(spirit_id);
        };
        static INSTRUCTION_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let id = maos_domain::orchestrator::OrchestratorInstructionId(
            INSTRUCTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let instruction = match maos_domain::orchestrator::OrchestratorInstruction::new(
            id,
            text,
            maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
        ) {
            Ok(value) => value,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "invalid_instruction".into(),
                    detail: error.to_string(),
                }
            }
        };
        let buffer = self
            .orchestrator_registry
            .get_or_create(&SpiritId::from(spirit_id));
        if let Err(error) = buffer.enqueue(instruction.clone()) {
            return OperatorOutcome::Conflict {
                code: "orchestrator_buffer_full".into(),
                detail: error.to_string(),
            };
        }
        if let Err(error) = maos_kernel_core::orchestrator::journal_orchestrator_queue(
            &self.transparency_log,
            DIRECTOR,
            spirit_id,
            &instruction,
        ) {
            return OperatorOutcome::Failed {
                code: "approval_log_failed".into(),
                detail: error.to_string(),
            };
        }
        OperatorOutcome::Completed(serde_json::json!({
            "spirit_id": spirit_id,
            "instruction_id": instruction.id.0,
            "pending": buffer.pending_count(),
            "capacity": buffer.capacity(),
        }))
    }

    /// Token revocation through the canonical adapter path. D-16-1-V: an
    /// already-revoked token with `RevokeReason::Operator` is
    /// `Err(CapError::Revoked)` with NO second audit event — mapped to
    /// `Conflict{code:"revoked"}`.
    async fn revoke_token_command(&self, token_id: &str) -> OperatorOutcome {
        use maos_domain::ports::CapabilityRegistryPort;

        let token_bytes = match parse_token_id_hex(token_id) {
            Ok(bytes) => bytes,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "invalid_token_id".into(),
                    detail: format!("invalid token_id '{token_id}': {error}"),
                }
            }
        };
        match self
            .capability
            .revoke(maos_domain::invariants::i1::TokenId(token_bytes))
        {
            Err(maos_domain::ports::capability::CapError::UnknownToken) => Self::not_found(
                "unknown_token",
                format!("token {token_id} was never issued in this daemon"),
            ),
            Err(maos_domain::ports::capability::CapError::Revoked) => OperatorOutcome::Conflict {
                code: "revoked".into(),
                detail: format!("token {token_id} is already revoked"),
            },
            Err(error) => OperatorOutcome::Failed {
                code: "revoke_failed".into(),
                detail: error.to_string(),
            },
            Ok(()) => {
                if let Err(error) = maos_kernel_core::orchestrator::journal_token_revocation(
                    &self.transparency_log,
                    DIRECTOR,
                    token_id,
                    None,
                ) {
                    return OperatorOutcome::Failed {
                        code: "approval_log_failed".into(),
                        detail: error.to_string(),
                    };
                }
                OperatorOutcome::Completed(serde_json::json!({
                    "token_id": token_id,
                    "revoked": true,
                }))
            }
        }
    }

    /// CRL import (D-16-1-X): the BYTES arrive in the command; the trust
    /// anchor was cloned at boot, never re-read from the environment.
    async fn import_revocations_command(&self, crl: &[u8]) -> OperatorOutcome {
        // A daemon booted without `MAOS_CRL_TRUST_ANCHOR_PUB_HEX` has no way
        // to tell a signed CRL from an attacker's. Refusing typed is the only
        // honest answer: verifying against an empty anchor would accept
        // anything, and accepting unverified revocations would let one POST
        // unload every Spirit on the host.
        let Some(anchor) = self.crl_trust_anchor.as_deref() else {
            return OperatorOutcome::Conflict {
                code: "crl_trust_anchor_unconfigured".into(),
                detail: "this daemon booted without MAOS_CRL_TRUST_ANCHOR_PUB_HEX, so it \
                         cannot verify a CRL signature"
                    .into(),
            };
        };
        let crl = match maos_kernel_core::revocation::parser::parse_signed_crl(
            crl,
            anchor,
            &*self.crypto_provider,
        ) {
            Ok(value) => value,
            Err(error) => {
                return OperatorOutcome::Invalid {
                    code: "crl_rejected".into(),
                    detail: format!("CRL parse/verify failed: {error}"),
                }
            }
        };
        let crl_id = crl.id;
        match self.revocation_applier.apply_crl(crl).await {
            Err(maos_domain::revocation::RevocationError::AlreadyApplied { .. }) => {
                OperatorOutcome::Conflict {
                    code: "crl_already_applied".into(),
                    detail: format!("CRL {} is already applied", hex::encode(crl_id.0)),
                }
            }
            Err(error) => OperatorOutcome::Failed {
                code: "crl_apply_failed".into(),
                detail: error.to_string(),
            },
            Ok(report) => {
                if let Ok(mut reports) = self.crl_reports.lock() {
                    reports.insert(crl_id.0, (report.matched_count, report.revoked_count));
                }
                OperatorOutcome::Completed(serde_json::json!({
                    "crl_id": hex::encode(crl_id.0),
                    "matched_count": report.matched_count,
                    "revoked_count": report.revoked_count,
                    "halt_receipts_produced": report.halt_receipts_produced,
                    "tokens_revoked_total": report.tokens_revoked_total,
                }))
            }
        }
    }

    /// The offline-forget cascade, reached over the door while the daemon
    /// holds the stores: the in-memory private values and live tokens the
    /// one-shot child could not see are the reason this route exists.
    async fn forget_command(&self, principal: &str, reason: Option<&str>) -> OperatorOutcome {
        if principal.trim().is_empty() {
            return OperatorOutcome::Invalid {
                code: "invalid_principal".into(),
                detail: "principal must not be empty".into(),
            };
        }
        match self.memory.forget_with_reason(principal, reason) {
            Err(error) => OperatorOutcome::Failed {
                code: "forget_failed".into(),
                detail: error.to_string(),
            },
            Ok(outcome @ maos_domain::memory::ForgetOutcome::Erased { .. }) => {
                OperatorOutcome::Completed(serde_json::json!({
                    "principal_id": principal,
                    "outcome": "erased",
                    "forget": outcome,
                    "terminal_code": 0,
                }))
            }
            Ok(outcome @ maos_domain::memory::ForgetOutcome::Suspended { .. }) => {
                // P29-cli: a legal-hold suspension is NOT a success — the
                // preserved terminal code 3 rides in the body.
                OperatorOutcome::Completed(serde_json::json!({
                    "principal_id": principal,
                    "outcome": "held",
                    "forget": outcome,
                    "terminal_code": 3,
                }))
            }
        }
    }

    async fn release_legal_hold_command(&self, principal: &str) -> OperatorOutcome {
        if principal.trim().is_empty() {
            return OperatorOutcome::Invalid {
                code: "invalid_principal".into(),
                detail: "principal must not be empty".into(),
            };
        }
        match self.transparency_log.release_legal_hold(principal.trim()) {
            Err(error) => OperatorOutcome::Failed {
                code: "legal_hold_release_failed".into(),
                detail: error.to_string(),
            },
            Ok(released) => OperatorOutcome::Completed(serde_json::json!({
                "principal_id": principal.trim(),
                "released": released,
                "auto_erased": false,
            })),
        }
    }

    async fn admit_governance_schema_command(&self, schema: serde_json::Value) -> OperatorOutcome {
        let missing = |name: &str| OperatorOutcome::Invalid {
            code: "invalid_schema_admission".into(),
            detail: format!("schema admission is missing '{name}'"),
        };
        let schema_id = match schema.get("schema_id").and_then(|v| v.as_str()) {
            Some(value) => value.to_string(),
            None => return missing("schema_id"),
        };
        let version = match schema.get("version").and_then(|v| v.as_u64()) {
            Some(value) => match u32::try_from(value) {
                Ok(version) => version,
                Err(_) => {
                    return OperatorOutcome::Invalid {
                        code: "invalid_schema_admission".into(),
                        detail: format!("schema version {value} exceeds u32::MAX"),
                    }
                }
            },
            None => return missing("version"),
        };
        let content_hash = match schema.get("content_hash").and_then(|v| v.as_str()) {
            Some(value) => value.to_string(),
            None => return missing("content_hash"),
        };
        let ratified_by = match schema.get("ratified_by").and_then(|v| v.as_str()) {
            Some(value) => value.to_string(),
            None => return missing("ratified_by"),
        };
        let effective_at_ns = match schema.get("effective_at_ns").and_then(|v| v.as_u64()) {
            Some(value) => value,
            None => return missing("effective_at_ns"),
        };
        let supersedes_hash = schema
            .get("supersedes")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let recorded_at_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let entry = maos_domain::governance::SchemaRegistryEntry {
            schema_id: schema_id.clone(),
            version,
            effective_at_ns,
            supersedes_hash: supersedes_hash.clone(),
            ratified_by: ratified_by.clone(),
            recorded_at_ns,
            schema_content_hash: content_hash,
        };
        if let Err(error) = self.transparency_log.register_schema_lifecycle(&entry) {
            return OperatorOutcome::Failed {
                code: "governance_admit_failed".into(),
                detail: error.to_string(),
            };
        }
        OperatorOutcome::Completed(serde_json::json!({
            "schema_id": schema_id,
            "version": version,
            "ratified_by": ratified_by,
            "effective_at_ns": effective_at_ns,
            "supersedes_hash": supersedes_hash,
            "admitted": true,
        }))
    }

    // ── shared helpers ───────────────────────────────────────────────────

    fn scb_state(
        &self,
        spirit_id: &str,
    ) -> Option<maos_kernel_core::scheduler::control_block::ScbLifecycleState> {
        let pid = self.scheduler.resolve_pid(spirit_id)?;
        let scbs = self.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|error| error.into_inner());
        guard.get(&pid).map(|scb| scb.current_state())
    }

    /// The loaded `[class]` version from the SCB's runtime snapshot — the
    /// version the daemon actually RUNS, not a manifest on disk.
    fn loaded_version(&self, spirit_id: &str) -> Option<String> {
        let pid = self.scheduler.resolve_pid(spirit_id)?;
        let scbs = self.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|error| error.into_inner());
        let scb = guard.get(&pid)?;
        scb.runtime_snapshot()
            .manifest
            .class
            .as_ref()
            .map(|class| class.version.clone())
    }

    /// The ONE body of a submitted command: wait for the Spirit's lock inside
    /// the route budget, CAS to STARTED, run, write the completion row,
    /// deliver the outcome.
    async fn run_command(
        self: Arc<Self>,
        operation_id: String,
        command: OperatorCommand,
        deadline: Duration,
        state: Arc<AtomicU8>,
        completion_tx: std::sync::mpsc::Sender<OperatorOutcome>,
    ) {
        let withdrawal_deadline = tokio::time::Instant::now() + deadline;
        // A command which finds its Spirit lock held has not started. Leave
        // the shared submit-and-withdraw protocol to classify it as
        // `spirit_busy`, rather than waiting until the first command finishes
        // and misreporting a duplicate-load conflict.
        // ⚠ The `Arc` is bound BEFORE the guard so it outlives it. Taking it
        // inside the `if let` made the guard borrow a temporary that dropped
        // at the end of the block, one line before the guard itself.
        //
        // Story 16-6 (D-16-6-D, §17 R12) — `Load` has no direct
        // `spirit_key`, but it still needs the per-Spirit critical section.
        // Read its bounded regular manifest on a blocking worker, then use
        // that same text both for the serialization key and admission.
        let pread = match &command {
            #[cfg(feature = "network")]
            OperatorCommand::Load { manifest } => {
                let path = PathBuf::from(manifest);
                tokio::task::spawn_blocking(move || crate::admission::read_manifest(&path))
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .map(Arc::new)
            }
            _ => None,
        };
        let lock_key = match (&command, &pread) {
            #[cfg(feature = "network")]
            (OperatorCommand::Load { .. }, Some(text)) => {
                crate::admission::peek_spirit_id_from_str(text)
            }
            (OperatorCommand::Load { .. }, None) => None,
            (other, _) => other.spirit_key().map(str::to_owned),
        };
        // D-16-6-D as ratified (§17 R12): the wait for the Spirit's lock is
        // the route budget minus a 250 ms margin, so a loser still inside
        // `lock_wait` keeps the sender alive and answers `spirit_busy`, and
        // a winner at the edge can still finish inside the budget.
        let lock_wait = deadline.saturating_sub(Duration::from_millis(250));
        let spirit_lock = lock_key.as_deref().map(|key| self.spirit_lock(key));
        let _permit = match spirit_lock.as_ref() {
            Some(lock) => match tokio::time::timeout(lock_wait, lock.lock()).await {
                Ok(guard) => Some(guard),
                Err(_elapsed) => {
                    // Keep the sender alive until the server's route-budget
                    // CAS withdraws this still-QUEUED command as spirit_busy.
                    tokio::time::sleep_until(withdrawal_deadline + Duration::from_millis(50)).await;
                    return;
                }
            },
            None => None,
        };
        // Exactly one of the port (here) and the server (at budget expiry)
        // wins this CAS. Losing means the server withdrew BEFORE the command
        // committed — the command must never run, so exit silently.
        if state
            .compare_exchange(
                COMMAND_QUEUED,
                COMMAND_STARTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return;
        }
        let verb = command_verb(&command);
        let spirit_id = command
            .spirit_key()
            .map(str::to_string)
            .or_else(|| lock_key.clone())
            .unwrap_or_default();
        let spirit_pid = self.scheduler.resolve_pid(&spirit_id).unwrap_or(0);
        let outcome = self.dispatch(command, pread).await;
        // ONE completion TL row per started mutating command, `intent`
        // carrying the `operation_id` so
        // `maosctl audit query --intent-contains <id>` finds it. Written
        // BEFORE the outcome is delivered, so a client that saw the answer
        // can always find the row.
        //
        // Story 16-2 / D-16-1-E as ratified (§15 R5): kind
        // `telemetry.event` — a tokenless `capability.invocation` row at a
        // Spirit's pid is precisely what FR4's view exists to refuse — and
        // the intent carries the OUTCOME (`operator.<verb>.<op>:<outcome>`),
        // prefix-compatible with 16-1's lookups (`starts_with
        // ("operator.pause.")`, the length check, `--intent-contains
        // <operation_id>`), so the row SAYS whether it resolved. `Completed`
        // maps to the literal `completed` (it carries no code); every other
        // variant maps to its typed code (all `[a-z_]`, none contain `:`).
        let outcome_label = match &outcome {
            OperatorOutcome::Completed(_) => "completed".to_string(),
            OperatorOutcome::NotFound { code, .. }
            | OperatorOutcome::Conflict { code, .. }
            | OperatorOutcome::Invalid { code, .. }
            | OperatorOutcome::Failed { code, .. } => code.clone(),
        };
        let payload = serde_json::json!({
            "operation_id": operation_id,
            "verb": verb,
            "spirit_id": spirit_id,
            "outcome": outcome_label,
        });
        let _ = self.transparency_log.insert_frame_event(
            maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent,
            spirit_pid,
            None,
            &format!("operator.{verb}.{operation_id}:{outcome_label}"),
            payload.to_string().as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
        // A receiver that already timed out may be gone; the rows are
        // written either way.
        let _ = completion_tx.send(outcome);
    }
}

#[async_trait]
impl OperatorCommandPort for OperatorDoor {
    fn submit(&self, command: OperatorCommand, deadline: Duration) -> OperatorSubmission {
        // 96 bits from the OS CSPRNG, hex — the id is only a lookup key, but
        // guessing one must not let an operator forge another's trail.
        let mut bytes = [0u8; 12];
        getrandom::fill(&mut bytes).expect("operation_id entropy unavailable");
        let operation_id = format!("op-{}", hex::encode(bytes));
        let state = Arc::new(AtomicU8::new(COMMAND_QUEUED));
        let (completion_tx, completion_rx) = std::sync::mpsc::channel();
        let door = Arc::clone(&self.0);
        let submitted_state = Arc::clone(&state);
        let submitted_id = operation_id.clone();
        // Spawned on the captured runtime and NEVER cancelled (D-16-1-E): a
        // `handler_still_running` id must find its completion row later.
        let task = self.0.runtime.spawn(async move {
            door.run_command(
                submitted_id,
                command,
                deadline,
                submitted_state,
                completion_tx,
            )
            .await;
        });
        self.0
            .command_tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(task);
        OperatorSubmission {
            operation_id,
            state,
            completion: completion_rx,
        }
    }

    fn spirit_status(&self, spirit_id: &str) -> Option<maos_control::SpiritStatusRow> {
        let pid = self.0.scheduler.resolve_pid(spirit_id)?;
        let scbs = self.0.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|error| error.into_inner());
        let scb = guard.get(&pid)?;
        let postures = self.0.policy.inner().load_full();
        let state = postures.spirit_postures.get(&pid);
        let posture = state
            .map(|state| format!("{:?}", state.current))
            .unwrap_or_else(|| "Unknown".into());
        // Story 16-6 (R23(i)) — the EFFECTIVE ceiling, read from the same
        // `PolicyTable` snapshot as `posture`, so a clamped admission is
        // distinguishable from an unclamped one on a real surface.
        let posture_ceiling = state
            .map(|state| format!("{:?}", state.allowed_max))
            .unwrap_or_else(|| "Unknown".into());
        Some(maos_control::SpiritStatusRow {
            spirit_id: spirit_id.to_string(),
            pid,
            boot_nonce: scb.boot_nonce.to_string(),
            lifecycle_state: format!("{:?}", scb.current_state()),
            posture,
            posture_ceiling,
        })
    }

    fn daemon_status(&self) -> maos_control::DaemonStatusRow {
        let scbs = self.0.scheduler.scbs();
        let guard = scbs.read().unwrap_or_else(|error| error.into_inner());
        // Story 16-6 (R2/R18) — ids and states are built from ONE sorted
        // pass so the two arrays are positionally aligned. Sorting the ids
        // separately and reading states afterwards would silently mismatch
        // them the moment two Spirits share a prefix ordering.
        let mut rows: Vec<(String, String)> = guard
            .values()
            .map(|scb| (scb.spirit_id.clone(), format!("{:?}", scb.current_state())))
            .collect();
        rows.sort();
        let (spirit_ids, lifecycle_states): (Vec<String>, Vec<String>) = rows.into_iter().unzip();
        let audit_health = maos_kernel_core::capability::cap_audit::audit_health_snapshot();
        maos_control::DaemonStatusRow {
            pid: self.0.daemon_pid,
            boot_nonce: self.0.boot_nonce.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            spirit_ids,
            lifecycle_states,
            audit_degraded: audit_health.degraded,
            audit_drop_count: audit_health.total_drops,
        }
    }

    fn orchestrator_status(&self, spirit_id: &str) -> Option<maos_control::OrchestratorStatusRow> {
        use maos_spirit_abi::identity::SpiritId;

        // Unknown Spirit ⇒ None (404), never a fake `0/32` that reads as an
        // empty buffer for a Spirit this daemon does not run (D-16-1-K). A
        // loaded Spirit with no buffer yet genuinely has `0/32`.
        self.0.scheduler.resolve_pid(spirit_id)?;
        let (pending, capacity) = match self.0.orchestrator_registry.get(&SpiritId::from(spirit_id))
        {
            Some(buffer) => (buffer.pending_count(), buffer.capacity()),
            None => (0, 32),
        };
        Some(maos_control::OrchestratorStatusRow {
            spirit_id: spirit_id.to_string(),
            pending,
            capacity,
        })
    }

    fn applied_revocations(&self) -> Vec<maos_control::RevocationRow> {
        let reports =
            self.0.crl_reports.lock().expect(
                "CRL report map: poisoned — a prior panic left import accounting inconsistent",
            );
        self.0
            .revocation_applier
            .list_applied()
            .into_iter()
            .map(|id| {
                let (matched_count, revoked_count) = reports.get(&id.0).copied().unwrap_or((0, 0));
                maos_control::RevocationRow {
                    // Hex, never the 32-integer array serde yields for a
                    // `CrlId` (D-16-1-X).
                    crl_id: hex::encode(id.0),
                    matched_count,
                    revoked_count,
                }
            })
            .collect()
    }
}

/// A stable machine label per command for the completion row's intent.
fn command_verb(command: &OperatorCommand) -> &'static str {
    match command {
        OperatorCommand::Start { .. } => "start",
        OperatorCommand::Pause { .. } => "pause",
        OperatorCommand::Resume { .. } => "resume",
        OperatorCommand::Unload { .. } => "unload",
        OperatorCommand::Posture { .. } => "posture",
        OperatorCommand::Upgrade { .. } => "upgrade",
        OperatorCommand::HotSwapPrecheck { .. } => "hot-swap-precheck",
        OperatorCommand::Uninstall { .. } => "uninstall",
        OperatorCommand::ResolveHalt { .. } => "halt-resolve",
        OperatorCommand::OrchestratorEnqueue { .. } => "orchestrator",
        OperatorCommand::RevokeToken { .. } => "revoke-token",
        OperatorCommand::ImportRevocations { .. } => "revocations-import",
        OperatorCommand::ForgetMemory { .. } => "forget",
        OperatorCommand::ReleaseLegalHold { .. } => "legal-hold-release",
        OperatorCommand::AdmitGovernanceSchema { .. } => "governance-admit",
        // ⚠ Not cosmetic. This is the machine label written into the
        // completion TL row's intent as
        // `operator.{verb}.{operation_id}:{outcome}` — it is what the audit
        // record says the operator did. The compiler proves the arm exists;
        // nothing proves the label is right, so AC4 asserts the row reads
        // `operator.load.<operation_id>:completed`.
        OperatorCommand::Load { .. } => "load",
    }
}

/// 32-char hex → `[u8; 16]` (`TokenId`). Lowercase hex only — SQLite `hex()`
/// is uppercase and the client accepts lowercase only, so the door rejects
/// mixed case rather than silently accepting what the client cannot send.
fn parse_token_id_hex(s: &str) -> Result<[u8; 16], String> {
    if s.len() != 32 {
        return Err(format!("expected 32 hex chars, got {}", s.len()));
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("expected lowercase hex".into());
    }
    let mut out = [0u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("invalid hex at byte {index}: {error}"))?;
    }
    Ok(out)
}

/// Extract `[class] name/version` (or legacy `[spirit]`) from a target
/// manifest, mirroring `enforce_vetted_upgrade_precondition`'s extraction.
fn manifest_class_identity(manifest: &[u8]) -> Result<(String, String), String> {
    let text = std::str::from_utf8(manifest)
        .map_err(|error| format!("upgrade target manifest is not UTF-8: {error}"))?;
    let root: toml::Value =
        toml::from_str(text).map_err(|error| format!("parse upgrade target TOML: {error}"))?;
    let section = root
        .get("class")
        .or_else(|| root.get("spirit"))
        .ok_or_else(|| "upgrade target manifest lacks [class] or [spirit]".to_string())?;
    let name = section
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "upgrade target manifest lacks string name".to_string())?;
    let version = section
        .get("version")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "upgrade target manifest lacks string version".to_string())?;
    Ok((name.to_string(), version.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// D-16-1-D — the store lock set.
//
// `flock` on the store DIRECTORIES' own handles. No lock file is ever created:
// a `.lock` file inside a store directory would change the file counts
// `erasure_uninstall_13_5b` asserts, and a shared parent (`/tmp` named through
// `MAOS_AUDIT_DB`) would yield a foreign-owned lock file. The set is taken
// before ANY store is opened, all-or-nothing, deduplicated by `(dev, ino)`
// (canonical paths miss bind-mount aliases, and a second `LOCK_EX` on the same
// inode from a second handle fails against the process's own first lock).
// ─────────────────────────────────────────────────────────────────────────────

/// Which flavor of boot is acquiring the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLockRole {
    /// A root that keeps the stores open (`maos run`, `maos shell`,
    /// `MAOS_ONE_SHOT=cohort-a2a-daemon` and every other composition-root
    /// boot): `LOCK_SH`.
    RootShared,
    /// An offline durable one-shot child (`forget`, `uninstall`,
    /// `legal-hold-release`, `governance-admit`): `LOCK_EX`. It must be the
    /// ONLY process touching the stores, because it erases.
    OfflineExclusive,
}

#[derive(Debug)]
pub enum StoreLockError {
    /// `flock` failed for any reason other than `EWOULDBLOCK` — NFS `EBADF`
    /// (a `LOCK_EX` needs a writable handle a directory cannot have),
    /// `EACCES`, a `mkdir` failure. Typed 78; network filesystems are
    /// unsupported for the offline arm and roots refuse to boot unlocked.
    LockUnavailable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The exclusive child found the set held after its 500 ms retry: typed
    /// 69 `StoreInUse`. A neutral name on purpose — `flock` cannot tell a
    /// door-less daemon from a second offline child.
    StoreInUse { path: PathBuf },
    /// A root found `LOCK_EX` held after its 500 ms retry: typed 69
    /// `OfflineOperationInProgress`.
    OfflineOperationInProgress { path: PathBuf },
}

/// The acquired set. `Drop` unlocks and closes, so the handles are released on
/// process exit AND on the SIGTERM teardown path, and `std::fs::File::open`
/// opened them with `O_CLOEXEC` so no spawned child ever inherits one.
pub struct StoreLockSet {
    entries: Vec<(PathBuf, std::fs::File)>,
}

impl StoreLockSet {
    /// The locked directories, sorted — the list the offline child prints.
    pub fn locked_paths(&self) -> Vec<PathBuf> {
        self.entries.iter().map(|(path, _)| path.clone()).collect()
    }
}

#[cfg(unix)]
impl Drop for StoreLockSet {
    fn drop(&mut self) {
        use rustix::fs::FlockOperation;
        for (_, file) in self.entries.iter() {
            let _ = rustix::fs::flock(file, FlockOperation::Unlock);
        }
    }
}

#[cfg(not(unix))]
impl Drop for StoreLockSet {
    fn drop(&mut self) {}
}

/// Resolve and acquire the store lock set, all-or-nothing.
///
/// `audit_db_path` is the TENANT-resolved Transparency Log path
/// (`resolved_transparency_log_path()` — it reads env and validates, it opens
/// no store). Every other member is resolved by its existing `maos_audit`
/// resolver; none are re-implemented here.
pub fn acquire_store_lock_set(
    audit_db_path: &Path,
    role: StoreLockRole,
) -> Result<StoreLockSet, StoreLockError> {
    #[cfg(not(unix))]
    {
        let _ = (audit_db_path, role);
        return Err(StoreLockError::LockUnavailable {
            path: PathBuf::from("/"),
            source: std::io::Error::other("store locking requires a unix platform"),
        });
    }

    #[cfg(unix)]
    {
        use rustix::fs::FlockOperation;
        use std::os::unix::fs::DirBuilderExt;

        let store_dir = |path: &Path| -> PathBuf {
            let resolved = if path.exists() {
                std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
            } else {
                path.to_path_buf()
            };
            match resolved.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                // A bare file name resolves against the cwd, so the store's
                // directory IS the cwd.
                _ => PathBuf::from("."),
            }
        };

        // 1. Resolve candidates by the EXISTING resolvers.
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(home) = maos_domain::operator_door::maos_home().map_err(|error| {
            StoreLockError::LockUnavailable {
                path: PathBuf::from("MAOS_HOME"),
                source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
            }
        })? {
            if home.exists() {
                candidates.push(home);
            }
        }
        candidates.push(store_dir(audit_db_path));
        candidates.push(store_dir(&maos_audit::default_transparency_log_path()));
        candidates.push(maos_audit::default_memory_root());
        candidates.push(store_dir(&maos_audit::default_journal_path()));
        candidates.push(maos_audit::default_erasure_proofs_dir());
        candidates.push(maos_audit::default_archive_dir());

        // 2. A path that exists as a NON-directory locks its parent
        //    (`erasure_uninstall_13_5b` plants such a file to force exit 5,
        //    which must survive), then create what is missing — mode 0700 for
        //    every component (boot's default `create_dir_all` is 0755, and
        //    any uid that can open a store directory can hold a lock on it).
        //    The proofs dir is created here ahead of its lazy first use.
        let mut resolved: Vec<PathBuf> = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            // `metadata` follows a directory symlink; canonicalization below
            // then locks the target inode rather than the alias's parent.
            let target = match std::fs::metadata(&candidate) {
                Ok(meta) if !meta.is_dir() => candidate
                    .parent()
                    .map(|parent| parent.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(".")),
                _ => candidate,
            };
            if let Err(error) = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&target)
            {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(StoreLockError::LockUnavailable {
                        path: target,
                        source: error,
                    });
                }
            }
            let canonical = std::fs::canonicalize(&target).map_err(|error| {
                StoreLockError::LockUnavailable {
                    path: target,
                    source: error,
                }
            })?;
            resolved.push(canonical);
        }
        // Lock up to two ancestor levels as stable guards. Purge removes store
        // directories while it runs; their flock handles then refer to
        // unlinked inodes. Every root boot takes these same shared guards, so
        // the exclusive purge remains authoritative until process exit even
        // after a store path has been removed. Never lock the process-wide
        // temporary directory itself: independent homes created beneath it
        // must not serialize or report false StoreInUse contention.
        let temp_root = std::fs::canonicalize(std::env::temp_dir()).ok();
        let mut guards = Vec::new();
        for path in &resolved {
            let mut ancestor = path.parent();
            for _ in 0..2 {
                let Some(path) = ancestor.filter(|path| {
                    path != &Path::new("/") && temp_root.as_deref().is_none_or(|temp| path != &temp)
                }) else {
                    break;
                };
                guards.push(path.to_path_buf());
                ancestor = path.parent();
            }
        }
        resolved.extend(guards);
        resolved.sort();
        resolved.dedup();

        // Retry window: absorbs maosctl's momentary home probe and the boot
        // window of a concurrently starting root.
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        loop {
            // 3. Open every handle read-only (`O_CLOEXEC`), dedup by
            //    `(st_dev, st_ino)` of the OPEN HANDLE — canonical paths miss
            //    bind-mount aliases, and flocking the same inode twice from
            //    one process fails against itself.
            let mut opened: Vec<(PathBuf, std::fs::File, u64, u64)> = Vec::new();
            let mut identity: std::collections::BTreeSet<(u64, u64)> =
                std::collections::BTreeSet::new();
            for path in &resolved {
                let file =
                    std::fs::File::open(path).map_err(|error| StoreLockError::LockUnavailable {
                        path: path.clone(),
                        source: error,
                    })?;
                let stat =
                    rustix::fs::fstat(&file).map_err(|errno| StoreLockError::LockUnavailable {
                        path: path.clone(),
                        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                    })?;
                let key = (stat.st_dev, stat.st_ino);
                if identity.insert(key) {
                    opened.push((path.clone(), file, stat.st_dev, stat.st_ino));
                }
            }

            // 4. Lock in sorted-path order, all-or-nothing.
            let mut locked: Vec<(PathBuf, std::fs::File)> = Vec::new();
            let mut stale = false;
            let mut busy_path: Option<PathBuf> = None;
            let mut failure: Option<StoreLockError> = None;
            for (path, file, dev, ino) in opened {
                let operation = match role {
                    StoreLockRole::RootShared => FlockOperation::NonBlockingLockShared,
                    StoreLockRole::OfflineExclusive => FlockOperation::NonBlockingLockExclusive,
                };
                match rustix::fs::flock(&file, operation) {
                    Ok(()) => {
                        // `maos purge` may have unlinked and re-created the
                        // directory between open and lock: a stale handle
                        // must not be trusted. fstat(handle) must equal
                        // stat(path).
                        match rustix::fs::stat(&path) {
                            Ok(stat) if stat.st_dev == dev && stat.st_ino == ino => {
                                locked.push((path, file));
                            }
                            _ => {
                                stale = true;
                                break;
                            }
                        }
                    }
                    Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => {
                        busy_path = Some(path);
                        break;
                    }
                    Err(errno) => {
                        failure = Some(StoreLockError::LockUnavailable {
                            path,
                            source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                        });
                        break;
                    }
                }
            }
            if failure.is_none() && busy_path.is_none() && !stale {
                return Ok(StoreLockSet { entries: locked });
            }
            for (_, file) in locked.iter() {
                let _ = rustix::fs::flock(file, FlockOperation::Unlock);
            }
            if let Some(error) = failure {
                return Err(error);
            }
            if std::time::Instant::now() >= deadline {
                let path =
                    busy_path.unwrap_or_else(|| resolved.first().cloned().unwrap_or_default());
                return Err(match role {
                    StoreLockRole::RootShared => {
                        StoreLockError::OfflineOperationInProgress { path }
                    }
                    StoreLockRole::OfflineExclusive => StoreLockError::StoreInUse { path },
                });
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
