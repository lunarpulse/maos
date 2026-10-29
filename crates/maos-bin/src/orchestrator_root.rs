//! Story 19-1 — the `maos run` root's half of the Orchestrator: the ONE
//! construction every Orchestrator of the root goes through (topology arm,
//! standalone arm, hot-swap successor; D-19-1-04 A), the `--epic` load and
//! its refusals (D-19-1-02/03 A), the stop check before each `task.assign`
//! (D-19-1-09 A) — preceded by Story 19-4's ambiguity judgement and followed
//! by its hold for the halt's resolution over the door (D-19-4-02/05 A) — and
//! the per-section distillate at the Worker's pid (D-19-1-07/08 A). In the
//! library, not `main.rs`, so `crates/maos-bin/tests/` drives the production
//! functions.

use std::ffi::OsStr;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{EpistemicHaltPayload, FramePayload, IacFrame, PriorDistillateRef};
use maos_domain::halt::{HaltId, HaltReceipt, HaltState};
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::notification::{NotificationEvent, NotificationLevel};
use maos_domain::orchestrator::OrchestratorInstruction;
use maos_domain::ports::{DistillationPort, EpistemicScalarPort, MemoryManagerPort};
use maos_domain::supervision::OnCrashAction;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::orchestrator::OrchestratorBufferRegistry;
use maos_kernel_core::scheduler::control_block::ScbLifecycleState;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use orchestrator::{
    fold_instruction, split_epic, EpicSection, Orchestrator, OrchestratorInferenceBinding,
    PidSource, SCALAR_TAG_DISPATCH_AMBIGUITY,
};
use sha2::{Digest, Sha256};

use crate::worker_spawn::WorkerRun;

/// The debug-build harness plant for the AC4.3(b) proven-red: before the
/// section whose 1-based ordinal it names, a real pending halt is raised for
/// the Orchestrator's pid through the kernel's `invoke_halt`. Release builds
/// never read it.
pub const HALT_PLANT_ENV: &str = "MAOS_TEST_ONLY_HALT_BEFORE_SECTION";

/// The debug-build harness plant for 19-6 AC3.2: on the FIRST attempt of the
/// section whose 1-based ordinal it names, the root kills that Worker once it
/// runs (see [`plant_test_crash`]). Release builds never read it.
pub const CRASH_PLANT_ENV: &str = "MAOS_TEST_ONLY_CRASH_SECTION_ONCE";

/// Story 19-2 (D-19-2-04 A) — the debug-build harness hold: once the named
/// 1-based section's `task.assign` is delivered (its delegation in flight),
/// the root waits for `$MAOS_HOME/release-section-<k>` (see
/// [`hold_test_section`]). Release builds never read it.
pub const HOLD_SECTION_ENV: &str = "MAOS_TEST_ONLY_HOLD_SECTION";

/// Story 19-4 (D-19-4-01 A) — the root's ONE Orchestrator scalar-port factory
/// (`main.rs`, beside `ButlerOrchestratorAdapter`): the production adapter
/// over the manifest's `[epistemic_policy]`, writing at the pid `PidSource`
/// resolves. A manifest without the policy is refused, never a bare port.
pub type ScalarPortFactory<'a> =
    &'a dyn Fn(PidSource) -> Result<Arc<dyn EpistemicScalarPort>, String>;

/// D-19-1-04 A — the ONE Orchestrator binding: the scalar port (19-4,
/// D-19-4-01 A) and the context port through the seam's binder (18-5) on
/// every path; under `--live` or an explicit mode the Inference Port from the
/// seam's builder (18-1) with a per-pass mint naming the first declared
/// provider the router registers. Unset mode returns it unbound (ADR-064).
/// The port serves only `judge_ambiguity`, never dispatch.
pub fn construct(
    orchestrator: Orchestrator,
    seam: &butler::ButlerSeam,
    declared: &[String],
    pid_of: PidSource,
    scalar: ScalarPortFactory<'_>,
) -> Result<Orchestrator, String> {
    let orchestrator = orchestrator.with_scalar_port(scalar(Arc::clone(&pid_of))?);
    let orchestrator = match &seam.context {
        Some(bind) => orchestrator.with_context_port(bind(Arc::clone(&pid_of))),
        None => orchestrator,
    };
    let Some((build_port, registered)) = seam.inference.as_ref() else {
        return Ok(orchestrator);
    };
    let provider = declared_provider(declared, registered)?;
    let port = build_port().ok_or("orchestrator inference: the host is shutting down")?;
    eprintln!("orchestrator inference seam wired (provider {provider})");
    let binding = OrchestratorInferenceBinding::per_pass(provider, Arc::clone(&seam.mint), pid_of);
    Ok(orchestrator.with_deferred_inference_port(port, Arc::new(Mutex::new(Some(binding)))))
}

/// D-19-1-04 A — the boot arms' Orchestrator: [`construct`] over a pid cell
/// the admission fixup fills (0 until then, read as unbound), with the seam
/// taken from the root's late-filled slot. Returns the cell beside the
/// Orchestrator so the caller stores the admitted pid into it.
pub fn construct_at_admitted_pid(
    spirit_id: &str,
    seam: &Mutex<Option<butler::ButlerSeam>>,
    declared: &[String],
    scalar: ScalarPortFactory<'_>,
) -> Result<(Orchestrator, Arc<AtomicU32>), String> {
    let cell = Arc::new(AtomicU32::new(0));
    let pid_of: PidSource = {
        let cell = Arc::clone(&cell);
        Arc::new(move || Some(cell.load(Ordering::Acquire)).filter(|pid| *pid != 0))
    };
    let seam = seam
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .ok_or("maos run: butler seam not ready")?;
    let orchestrator = construct(
        Orchestrator::new(spirit_id),
        &seam,
        declared,
        pid_of,
        scalar,
    )
    .map_err(|e| format!("maos run: {e}"))?;
    Ok((orchestrator, cell))
}

type RuntimeObject = dyn maos_kernel_core::scheduler::control_block::AnySpiritObj;
type LiveJudgement = (std::sync::Weak<RuntimeObject>, Orchestrator);
static LIVE_JUDGEMENTS: std::sync::LazyLock<Mutex<Vec<LiveJudgement>>> =
    std::sync::LazyLock::new(|| Mutex::new(Vec::new()));

/// Associate the complete judgement binding with the exact committed runtime.
/// Weak runtime keys do not retain discarded candidates; stale bindings are
/// pruned on registration and lookup. Staging never publishes a new binding.
pub fn judgement_runtime(
    emitter: Orchestrator,
    scalar: Arc<dyn EpistemicScalarPort>,
) -> Arc<RuntimeObject> {
    let emitter = emitter.with_scalar_port(scalar);
    let object = maos_kernel_core::scheduler::make_spirit_obj(emitter.clone());
    let mut live = LIVE_JUDGEMENTS.lock().unwrap_or_else(|e| e.into_inner());
    live.retain(|(object, _)| object.strong_count() != 0);
    live.push((Arc::downgrade(&object), emitter));
    object
}

/// Resolve both inference and scalar policy through the committed SCB. An
/// unload racing the pre-judgement stop check is left to `dispatch_stop`.
pub fn judge_live_section(
    scheduler: &SpiritSchedulerAdapter,
    pid: u32,
    emitter: &Orchestrator,
    section: &EpicSection,
) -> Result<Option<HaltReceipt>, String> {
    let object = {
        let scbs = scheduler.scbs();
        let scbs = scbs.read().unwrap_or_else(|e| e.into_inner());
        let Some(scb) = scbs
            .get(&pid)
            .filter(|scb| scb.spirit_id == emitter.orchestrator_id())
        else {
            return Ok(None);
        };
        scb.runtime_snapshot().spirit_obj
    };
    let runtime_key = Arc::downgrade(&object);
    let mut live = LIVE_JUDGEMENTS.lock().unwrap_or_else(|e| e.into_inner());
    live.retain(|(runtime, _)| runtime.strong_count() != 0);
    let emitter = live
        .iter()
        .find(|(runtime, _)| runtime.ptr_eq(&runtime_key))
        .map(|(_, emitter)| emitter.clone())
        .ok_or("maos run: live Orchestrator judgement binding is not wired")?;
    drop(live);
    judge_section(&emitter, section)
}

/// D-19-1-04 A — the pid the Orchestrator's admission stored in `cell`. 0
/// means it was never admitted, and no frame is dispatched from it.
pub fn admitted_pid(cell: &AtomicU32) -> Result<u32, String> {
    match cell.load(Ordering::Acquire) {
        0 => Err(format!(
            "maos run: the `{}` Orchestrator has no admitted pid; no delegation frame is \
             dispatched from an unadmitted Orchestrator",
            crate::delegation::FROM_SPIRIT
        )),
        pid => Ok(pid),
    }
}

/// The provider an Orchestrator's token and requests name: its first declared
/// `ProviderInfer` the host router registers; none is a typed refusal naming
/// every declared provider (the Researcher's rule, D-18-3-02 B).
fn declared_provider(declared: &[String], registered: &[String]) -> Result<String, String> {
    if declared.is_empty() {
        return Err("orchestrator inference: the Spirit declares no provider".into());
    }
    declared
        .iter()
        .find(|provider| registered.contains(provider))
        .cloned()
        .ok_or_else(|| {
            let names: Vec<String> = declared.iter().map(|p| format!("`{p}`")).collect();
            format!(
                "orchestrator inference: no inference provider is configured for its declared \
                 providers {}",
                names.join(", ")
            )
        })
}

/// D-19-1-02 A — `--epic` dispatches on the loopback arm only. The root calls
/// this the moment the arm is decided, before the cross-host boot-nonce row,
/// banner and pairing hold; [`load_epic`] repeats it as defence.
pub fn refuse_epic_on_cross_host(epic: bool, cross_host: bool) -> Result<(), String> {
    if epic && cross_host {
        return Err(
            "maos run: --epic dispatches on the loopback arm only; on the cross-host arm the far \
             Host runs each Worker and nothing returns to distil, so section k+1 could never \
             carry section k's distillate. Drop --epic and set MAOS_DELEGATED_GOAL"
                .into(),
        );
    }
    Ok(())
}

/// D-19-1-02 / 03 A — what a topology's `host_entries` host-bearing Workers
/// are dispatched with, decided before ANY entry loads so every refusal
/// precedes every emit: `--epic`'s sections (section k → the k-th entry), or
/// `None` (one loopback Worker gets the rehearsal string; the cross-host arm
/// reads `MAOS_DELEGATED_GOAL` per frame). `delegated_goal` is that variable's
/// raw value. Refused typed on loopback: a set goal that is non-blank or not
/// UTF-8 (the goal comes from `--epic`), and more than one host-bearing entry
/// without `--epic` (Worker 2's follow-up would carry no distillate, FR21).
pub fn dispatch_plan(
    epic: Option<&Path>,
    host_entries: usize,
    cross_host: bool,
    delegated_goal: Option<&OsStr>,
) -> Result<Option<Vec<EpicSection>>, String> {
    if !cross_host && host_entries > 0 {
        let goal_set = delegated_goal
            .is_some_and(|goal| goal.to_str().is_none_or(|goal| !goal.trim().is_empty()));
        if goal_set {
            return Err(
                "maos run: MAOS_DELEGATED_GOAL names a cross-host goal and this run delegates on \
                 the loopback arm. On loopback the goal comes from `--epic <file>` (one \
                 `### Story` section per host-bearing Worker); unset MAOS_DELEGATED_GOAL and \
                 pass --epic instead."
                    .into(),
            );
        }
        if epic.is_none() && host_entries > 1 {
            return Err(format!(
                "maos run: this topology declares {host_entries} host-bearing Worker entries and \
                 no --epic. On the loopback arm each Worker's goal is one `### Story` section of \
                 `--epic <file>`, and without it Worker 2's follow-up could carry no \
                 distillate (FR21). Pass --epic with {host_entries} `### Story` sections."
            ));
        }
    }
    epic.map(|path| load_epic(path, host_entries, cross_host))
        .transpose()
}

/// D-19-1-02 / 03 A — read and split `--epic <path>` for a topology with
/// `host_entries` host-bearing entries. Refused typed, before any emit: on the
/// cross-host arm ([`refuse_epic_on_cross_host`]), with no host-bearing
/// entry, an unreadable file, or a split the Orchestrator refuses (a
/// non-`### Story` heading, a count mismatch).
pub fn load_epic(
    path: &Path,
    host_entries: usize,
    cross_host: bool,
) -> Result<Vec<EpicSection>, String> {
    refuse_epic_on_cross_host(true, cross_host)?;
    if host_entries == 0 {
        return Err(
            "maos run: --epic needs a topology with host-bearing Worker entries (one per \
             `### Story` section); this topology declares none"
                .into(),
        );
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("maos run: --epic {}: {e}", path.display()))?;
    split_epic(&text, host_entries).map_err(|e| format!("maos run: --epic {}: {e}", path.display()))
}

/// D-19-1-09 A — whether dispatch stops before section `ordinal` of `total`:
/// the `orchestrator` no longer resolves to its admitted `pid` (`resolved`;
/// unloaded through the door mid-epic), or `pid` holds a halt pending
/// resolution (read without draining). `Some` is the `dispatch_halted` event
/// to print; no further Worker is admitted and the run drains normally.
pub fn dispatch_stop(
    registry: &HaltRegistry,
    pid: u32,
    resolved: Option<u32>,
    ordinal: usize,
    total: usize,
) -> Option<serde_json::Value> {
    let unloaded = resolved != Some(pid);
    let halt_ids: Vec<String> = registry
        .drain_for_spirit_dry_run(pid)
        .into_iter()
        .filter(|(_, state)| *state == HaltState::PendingResolution)
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    (unloaded || !halt_ids.is_empty()).then(|| {
        serde_json::json!({
            "event": "dispatch_halted",
            "orchestrator_pid": pid,
            "orchestrator_unloaded": unloaded,
            "halt_ids": halt_ids,
            "dispatched_sections": ordinal.saturating_sub(1),
            "undispatched_sections": (ordinal..=total).collect::<Vec<_>>(),
        })
    })
}

/// Include the judgement's identity even after the resolver removed its pending
/// metadata. A terminal resolution still belongs to this section, not the next.
pub fn judged_dispatch_stop(
    registry: &HaltRegistry,
    pid: u32,
    resolved: Option<u32>,
    ordinal: usize,
    total: usize,
    judged: Option<&HaltReceipt>,
) -> Option<serde_json::Value> {
    let stop = dispatch_stop(registry, pid, resolved, ordinal, total);
    let Some(receipt) = judged else {
        return stop;
    };
    let mut stop = stop.unwrap_or_else(|| {
        serde_json::json!({
            "event": "dispatch_halted",
            "orchestrator_pid": pid,
            "orchestrator_unloaded": resolved != Some(pid),
            "halt_ids": [],
            "dispatched_sections": ordinal.saturating_sub(1),
            "undispatched_sections": (ordinal..=total).collect::<Vec<_>>(),
        })
    });
    if let Some(ids) = stop["halt_ids"].as_array_mut() {
        if !ids
            .iter()
            .any(|id| id.as_str() == Some(receipt.halt_id.as_str()))
        {
            ids.push(serde_json::Value::String(
                receipt.halt_id.as_str().to_owned(),
            ));
        }
    }
    Some(stop)
}

/// Story 19-4 (AC2.3, D-19-4-02 A) — judge a NEW section before its frame
/// (never a 19-6 re-dispatch): the Orchestrator's completion becomes the
/// `dispatch_ambiguity` scalar through its port and the kernel's policy
/// decides the halt; prints `dispatch_judged`. Unset mode (`NotWired`) is no
/// judgement and no event (ADR-064); every other failure refuses the run
/// typed, never a default (D-19-1-12 A).
pub fn judge_section(
    emitter: &Orchestrator,
    section: &EpicSection,
) -> Result<Option<HaltReceipt>, String> {
    let (value, receipt) = match emitter.assess_ambiguity(section) {
        Ok(judged) => judged,
        Err(orchestrator::OrchestratorInferenceError::NotWired) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "maos run: section {} ambiguity judgement refused the run: {error}",
                section.ordinal
            ))
        }
    };
    println!(
        "{}",
        serde_json::json!({
            "event": "dispatch_judged",
            "section": section.ordinal,
            "tag": SCALAR_TAG_DISPATCH_AMBIGUITY,
            "value": value,
            "halt_id": receipt.as_ref().map(|r| r.halt_id.as_str().to_owned()),
            "halt_frame_id": receipt.as_ref().map(|r| hex::encode(r.frame_id)),
        })
    );
    Ok(receipt)
}

/// Story 19-4 (AC5.2, D-19-4-05 A) — `MAOS_EPIC_HALT_HOLD_SECS`, read once
/// per `--epic` run before any entry loads: unset ⇒ 0 (the 19-1 stop);
/// anything but whole seconds is refused typed.
pub fn epic_halt_hold() -> Result<std::time::Duration, String> {
    let Some(raw) = std::env::var_os("MAOS_EPIC_HALT_HOLD_SECS") else {
        return Ok(std::time::Duration::ZERO);
    };
    let secs = raw
        .to_str()
        .and_then(|text| text.trim().parse::<u64>().ok());
    secs.map(std::time::Duration::from_secs).ok_or_else(|| {
        format!("maos run: MAOS_EPIC_HALT_HOLD_SECS must be whole seconds, got {raw:?}")
    })
}

/// Story 19-4 (D-19-4-05 A) — what joins a resumed section's goal before the
/// operator's `provided-context` clarification.
pub const HALT_CLARIFICATION_DELIMITER: &str = "\n\nOperator clarification (halt resolved): ";

/// Story 19-4 (AC5.1, D-19-4-05 A) — after `dispatch_stop`'s `stop` event,
/// hold section `ordinal` (no frame, no Worker) while the Orchestrator at
/// `pid` has a halt pending, polled every 100 ms, bounded by `bound` and the
/// root's shutdown; a halt raised during the hold is held on too. `Some(goal)`
/// = every held halt was resolved over the door without `accepted-halt`:
/// `dispatch_resumed` is printed and each `provided-context` text (the
/// resolver's `halt_context::<id>`) is appended to `goal`; `authorized-override`
/// leaves it unchanged. `None` = the 19-1 stop: a zero bound (the default),
/// an unloaded Orchestrator, `accepted-halt`, the bound or shutdown.
#[allow(clippy::too_many_arguments)]
pub async fn hold_for_resolution(
    tl: &TransparencyLogAdapter,
    registry: &HaltRegistry,
    memory: &MemoryManagerAdapter,
    resolved: impl Fn() -> Option<u32>,
    stop: &serde_json::Value,
    ordinal: usize,
    goal: String,
    bound: std::time::Duration,
    shutdown: &tokio_util::sync::CancellationToken,
) -> Option<String> {
    if shutdown.is_cancelled()
        || bound.is_zero()
        || stop["orchestrator_unloaded"].as_bool() != Some(false)
    {
        return None;
    }
    let pid = u32::try_from(stop["orchestrator_pid"].as_u64()?).ok()?;
    let mut held: Vec<HaltId> = stop["halt_ids"]
        .as_array()?
        .iter()
        .filter_map(|id| HaltId::new(id.as_str()?.to_owned()).ok())
        .collect();
    // The registry drops terminal metadata. The TL retains ownership and IDs,
    // including a halt both raised and resolved between two registry polls.
    let started = tokio::time::Instant::now();
    let boot_nonce = tl.boot_nonce();
    let before = halt_ids_at_pid(tl, pid, boot_nonce)?;
    // Include intervening rows already terminal at entry too: the first held
    // halt marks this hold interval, not the first polling wake.
    if let Some(first) = before.iter().position(|id| held.contains(id)) {
        for id in &before[first..] {
            if !held.contains(id) {
                held.push(id.clone());
            }
        }
    }
    loop {
        if shutdown.is_cancelled() || started.elapsed() >= bound || resolved() != Some(pid) {
            return None;
        }
        for id in halt_ids_at_pid(tl, pid, boot_nonce)? {
            if !before.contains(&id) && !held.contains(&id) {
                held.push(id);
            }
        }
        for (id, _) in registry.drain_for_spirit_dry_run(pid) {
            if !held.contains(&id) {
                held.push(id);
            }
        }
        // Re-check after the registry/identity reads, before accepting even an
        // already-empty pending set.
        if shutdown.is_cancelled() || started.elapsed() >= bound {
            return None;
        }
        let terminal = held.iter().all(|id| {
            registry
                .lookup_state(id)
                .is_some_and(|state| state != HaltState::PendingResolution)
        });
        let approvals = if terminal {
            tl.query_approvals(None).ok()?
        } else {
            Vec::new()
        };
        if terminal
            && held.iter().all(|id| {
                let intent = match registry.lookup_state(id) {
                    Some(HaltState::Resumed) => "provided_context",
                    Some(HaltState::Overridden) => "authorized_override",
                    Some(HaltState::Terminated) => "accepted_halt",
                    _ => return false,
                };
                // Resolver publication precedes this durable door approval.
                let prefix = format!("halt={}: {intent}", id.as_str());
                approvals.iter().any(|row| {
                    row.decision
                        && row.actor == "director"
                        && row.target == crate::delegation::FROM_SPIRIT
                        && row.capability == "halt.resolve"
                        && row.intent == intent
                        && row.reasoning.as_deref().is_some_and(|text| {
                            text == prefix
                                || text
                                    .strip_prefix(&prefix)
                                    .is_some_and(|rest| rest.starts_with(": "))
                        })
                })
            })
        {
            // Close the interval with a second log snapshot: a halt may have
            // been raised AND resolved while the registry was being inspected.
            let mut discovered = false;
            for id in halt_ids_at_pid(tl, pid, boot_nonce)? {
                if !before.contains(&id) && !held.contains(&id) {
                    held.push(id);
                    discovered = true;
                }
            }
            if discovered {
                continue;
            }
            break;
        }
        let left = bound.saturating_sub(started.elapsed());
        tokio::select! {
            biased;
            () = shutdown.cancelled() => return None,
            () = tokio::time::sleep(left.min(std::time::Duration::from_millis(100))) => {}
        }
    }
    let mut goal = goal;
    let mut provided_context = false;
    for id in &held {
        match registry.lookup_state(id)? {
            // The resolver writes the text BEFORE the state flips (resolver.rs);
            // a clarification that cannot be read is never dropped silently.
            HaltState::Resumed => {
                let key = format!("halt_context::{}", id.as_str());
                match memory.read(pid, MemoryTier::Private, &MemoryNamespace::Default, &key) {
                    Ok(Some(MemoryValue::Text(text))) => {
                        goal = format!("{goal}{HALT_CLARIFICATION_DELIMITER}{text}");
                        provided_context = true;
                    }
                    _ => return None,
                }
            }
            HaltState::Overridden => {}
            // `accepted-halt`, or a resolution rolled back to pending.
            HaltState::Terminated | HaltState::PendingResolution => return None,
        }
    }
    if resolved() != Some(pid) || shutdown.is_cancelled() || started.elapsed() >= bound {
        return None;
    }
    let halt_ids: Vec<&str> = held.iter().map(HaltId::as_str).collect();
    println!(
        "{}",
        serde_json::json!({
            "event": "dispatch_resumed",
            "section": ordinal,
            "halt_ids": halt_ids,
            "provided_context": provided_context,
        })
    );
    Some(goal)
}

fn halt_ids_at_pid(tl: &TransparencyLogAdapter, pid: u32, boot_nonce: u64) -> Option<Vec<HaltId>> {
    Some(
        tl.query_frames(FrameFilter {
            spirit_pid: Some(pid),
            boot_nonce: Some(boot_nonce),
            kind: Some(FrameKind::EpistemicHalt),
            ..FrameFilter::default()
        })
        .ok()?
        .into_iter()
        // Termination markers and receipts share this kind, but are not raises.
        .filter_map(|row| {
            let payload: EpistemicHaltPayload =
                serde_json::from_slice(&row.payload_redacted).ok()?;
            HaltId::new(payload.halt_id).ok()
        })
        .collect(),
    )
}

/// The AC4.3(b) harness plant ([`HALT_PLANT_ENV`]): debug builds only, and
/// only before the named section. The halt is a real one — the kernel's
/// `invoke_halt` writes its TL row, Journal entry and registry insert.
pub fn plant_test_halt(
    tl: &TransparencyLogAdapter,
    journal: &JournalAdapter,
    registry: &HaltRegistry,
    pid: u32,
    spirit_id: &str,
    boot_nonce: u64,
    ordinal: usize,
) -> Result<(), String> {
    let planted = cfg!(debug_assertions)
        && std::env::var(HALT_PLANT_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            == Some(ordinal);
    if !planted {
        return Ok(());
    }
    let payload = EpistemicHaltPayload::new(
        ulid::Ulid::new().to_string(),
        SCALAR_TAG_DISPATCH_AMBIGUITY.to_owned(),
        1.0,
        Some(0.5),
        "dispatch_ambiguity".to_owned(),
        HALT_PLANT_ENV.to_owned(),
    )
    .map_err(|e| format!("maos run: {HALT_PLANT_ENV}: {e}"))?;
    invoke_halt(tl, journal, registry, payload, pid, spirit_id, boot_nonce)
        .map(drop)
        .map_err(|e| format!("maos run: {HALT_PLANT_ENV}: {e}"))
}

/// The [`CRASH_PLANT_ENV`] plant: debug builds only, never on a re-dispatch.
/// A root task polls until the Worker bound to `task_id` has a child, then
/// kills it through the supervisor's signal/teardown without a planned stop,
/// so the kernel handles a real crash. The caller aborts it after the run.
pub fn plant_test_crash(
    supervisor: &Arc<crate::supervision::WorkerSupervisor>,
    task_id: &str,
    ordinal: Option<usize>,
    redispatched: bool,
) -> Option<tokio::task::JoinHandle<()>> {
    let planted = cfg!(debug_assertions)
        && !redispatched
        && ordinal.is_some()
        && std::env::var(CRASH_PLANT_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            == ordinal;
    planted.then(|| {
        let (supervisor, task_id) = (Arc::clone(supervisor), task_id.to_owned());
        tokio::spawn(async move {
            while !supervisor.test_only_crash_task(&task_id) {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
    })
}

/// Story 19-2 (D-19-2-02 B / 03 A) — the director instruction section's
/// `frame` carries, folded into its `task.assign` goal. The caller builds
/// the frame and re-checks the Orchestrator's pause BEFORE this runs, so an
/// instruction leaves the buffer only once its frame exists and dispatch is
/// not paused: nothing is ever handed back (the kernel buffer has no
/// push-front, so a hand-back would reorder the FIFO).
///
/// `carried` is a 19-6 re-dispatch's instruction: its crashed first attempt
/// already consumed it, so it is re-folded into the NEW frame without
/// draining. Otherwise, at a NEW `--epic` section's safe point
/// (`safe_point`: never a re-dispatch, never off `--epic`), ONE instruction
/// is taken through the Orchestrator's FR20 gate from the buffer the door
/// enqueues on. Anything else stays queued (D-03 A).
pub fn fold_at_safe_point(
    emitter: &Orchestrator,
    registry: &OrchestratorBufferRegistry,
    frame: &mut IacFrame,
    carried: Option<OrchestratorInstruction>,
    safe_point: bool,
) -> Result<Option<OrchestratorInstruction>, String> {
    let FramePayload::TaskAssign(payload) = &mut frame.payload else {
        return Err(format!(
            "maos run: frame {} is not a task.assign; no director instruction was folded",
            hex::encode(frame.frame_id)
        ));
    };
    let Some(instruction) = carried.or_else(|| {
        registry
            .get(&crate::delegation::FROM_SPIRIT.into())
            .filter(|_| safe_point)
            .and_then(|buffer| emitter.drain_next(|| buffer.dequeue_at_safe_point()))
    }) else {
        return Ok(None);
    };
    payload.goal = fold_instruction(std::mem::take(&mut payload.goal), &instruction);
    Ok(Some(instruction))
}

/// The goal back out of a section frame that was built but is not delivered
/// (the Orchestrator was paused after the build): the loop holds, then
/// rebuilds the frame from it.
pub fn reclaim_goal(frame: IacFrame) -> Result<String, String> {
    match frame.payload {
        FramePayload::TaskAssign(payload) => Ok(payload.goal),
        _ => Err(format!(
            "maos run: frame {} is not a task.assign; its goal cannot be re-dispatched",
            hex::encode(frame.frame_id)
        )),
    }
}

/// Story 19-2 (AC1.2, CR-2) — journal the instruction folded into section
/// `ordinal`'s frame `frame_id` BEFORE the frame is delivered, then print
/// `orchestrator_instruction_dispatched`. A failed row fails the run: the
/// frame is never delivered. A 19-6 re-dispatch (`redispatch`) writes a
/// second row naming the NEW frame, so the log never claims only the frame
/// whose Worker crashed.
pub fn record_safe_point_drain(
    tl: &TransparencyLogAdapter,
    instruction: &OrchestratorInstruction,
    ordinal: usize,
    frame_id: &[u8; 16],
    redispatch: bool,
) -> Result<(), String> {
    let frame = hex::encode(frame_id);
    tl.insert_approval_decision(ApprovalDecision {
        actor: crate::delegation::FROM_SPIRIT.into(),
        target: crate::delegation::FROM_SPIRIT.into(),
        capability: "orchestrator.dispatch".into(),
        intent: "safe-point".into(),
        decision: true,
        reasoning: Some(format!(
            "id={} section={ordinal} frame={frame}{}",
            instruction.id.0,
            if redispatch { " re-dispatch" } else { "" }
        )),
    })
    .map_err(|e| {
        format!(
            "maos run: safe-point row for instruction {}: {e}",
            instruction.id.0
        )
    })?;
    println!(
        "{}",
        serde_json::json!({
            "event": "orchestrator_instruction_dispatched",
            "instruction_id": instruction.id.0,
            "section": ordinal,
            "frame_id": frame,
            "redispatch": redispatch,
        })
    );
    Ok(())
}

/// Story 19-2 (D-19-2-17 A) — whether the Orchestrator at `pid` is Paused.
pub fn orchestrator_paused(scheduler: &SpiritSchedulerAdapter, pid: u32) -> bool {
    let scbs = scheduler.scbs();
    let guard = scbs.read().unwrap_or_else(|e| e.into_inner());
    guard
        .get(&pid)
        .is_some_and(|scb| scb.current_state() == ScbLifecycleState::Paused)
}

/// Story 19-2 (D-19-2-17 A) — before section `ordinal`'s frame, wait while
/// the Orchestrator at `pid` is Paused (polled every 100 ms), printing
/// `dispatch_paused` once. `false` when the root's shutdown ended the wait.
pub async fn hold_while_paused(
    scheduler: &SpiritSchedulerAdapter,
    pid: u32,
    ordinal: usize,
    shutdown: &tokio_util::sync::CancellationToken,
) -> bool {
    if orchestrator_paused(scheduler, pid) {
        println!(
            "{}",
            serde_json::json!({"event": "dispatch_paused", "section": ordinal})
        );
    }
    while orchestrator_paused(scheduler, pid) {
        tokio::select! {
            () = shutdown.cancelled() => return false,
            () = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
    }
    !shutdown.is_cancelled()
}

/// Judgement is a one-shot operation behind the pause/cancellation gate.
/// Keeping the closure here also makes "no inference while paused" observable
/// without a real provider or a Worker process.
pub async fn judge_after_pause<T>(
    scheduler: &SpiritSchedulerAdapter,
    pid: u32,
    ordinal: usize,
    shutdown: &tokio_util::sync::CancellationToken,
    judge: impl FnOnce() -> Result<T, String>,
) -> Result<Option<T>, String> {
    if !hold_while_paused(scheduler, pid, ordinal, shutdown).await {
        return Ok(None);
    }
    judge().map(Some)
}

/// The [`HOLD_SECTION_ENV`] hold: debug builds only, only at the named
/// section. Prints `dispatch_held_test_only` before waiting; bounded at 30 s,
/// then a typed error.
pub async fn hold_test_section(ordinal: Option<usize>) -> Result<(), String> {
    let held = cfg!(debug_assertions)
        && ordinal.is_some()
        && std::env::var(HOLD_SECTION_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            == ordinal;
    let Some(ordinal) = ordinal.filter(|_| held) else {
        return Ok(());
    };
    let release = maos_domain::operator_door::maos_home()
        .map_err(|e| format!("maos run: {HOLD_SECTION_ENV}: {e}"))?
        .ok_or_else(|| format!("maos run: {HOLD_SECTION_ENV} needs MAOS_HOME"))?
        .join(format!("release-section-{ordinal}"));
    println!(
        "{}",
        serde_json::json!({"event": "dispatch_held_test_only", "section": ordinal})
    );
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while !release.exists() {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "maos run: {HOLD_SECTION_ENV}: section {ordinal} not released within 30 s ({})",
                release.display()
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Ok(())
}

/// D-19-1-08 A — the distillate's digest: non-raw facts only, so 18-6's forget
/// cascade reaches it through its sources, never its body.
pub fn section_digest(section: &EpicSection, run: &WorkerRun) -> serde_json::Value {
    serde_json::json!({
        "section": section.ordinal,
        "heading_sha256": hex::encode(Sha256::digest(section.heading.as_bytes())),
        "verdict": run.completion.label(),
        "stdout_lines": run.stdout_frame_ids.len(),
        "stderr_lines": run.stderr_lines,
        "last_stdout_tl_ref": run.last_stdout_tl_ref.map(hex::encode),
    })
}

/// D-19-1-07 A — distil a completed Worker's stdout rows AT THE WORKER'S PID
/// through the root's `DistillateWriter` (the only sanctioned inserter; I11
/// refuses any other citer) and return the ref the next section's follow-up
/// carries (FR21). The receipt's id is read back from `tl` before it is used:
/// the writer recovers it from the log's last insert, so a concurrent row
/// could stand in for it — anything but a `Distillate` row at the Worker's
/// pid is refused typed.
pub fn write_section_distillate(
    writer: &dyn DistillationPort,
    tl: &TransparencyLogAdapter,
    section: &EpicSection,
    run: &WorkerRun,
) -> Result<PriorDistillateRef, String> {
    let receipt = DistillationRequest::new(
        run.stdout_frame_ids.clone(),
        1,
        DigestPayload::Json(section_digest(section, run)),
        None,
    )
    .and_then(|request| writer.write_distillate(run.spirit_pid, request))
    .map_err(|e| {
        format!(
            "maos run: epic section {} distillate refused at Worker pid {}: {e}",
            section.ordinal, run.spirit_pid
        )
    })?;
    let row = tl
        .query_frame_metadata_by_id(receipt.digest_frame_id)
        .map_err(|e| {
            format!(
                "maos run: epic section {} distillate receipt unreadable: {e}",
                section.ordinal
            )
        })?;
    if !row.is_some_and(|row| {
        row.raw_kind == FrameKind::Distillate as i64 && row.spirit_pid == run.spirit_pid
    }) {
        return Err(format!(
            "maos run: epic section {} distillate receipt {} is not a Distillate row at Worker \
             pid {}; its ref is not dispatched",
            section.ordinal,
            hex::encode(receipt.digest_frame_id),
            run.spirit_pid
        ));
    }
    Ok(PriorDistillateRef {
        digest_frame_id: receipt.digest_frame_id,
        distillation_depth: receipt.effective_distillation_depth,
        intent_lineage: receipt.intent_lineage,
    })
}

/// D-19-6-04/05 A — whether a Worker entry's manifest declares
/// `[on_crash] action = "reassign-to-replica"`: only then does the loop arm
/// the root's replica resolver around its one delegated run.
pub fn declares_reassign(manifest: &toml::Value) -> bool {
    manifest
        .get("on_crash")
        .and_then(|section| section.get("action"))
        .and_then(toml::Value::as_str)
        == Some("reassign-to-replica")
}

/// D-19-6-03 A — the `TaskComplete` rows at `pid` whose payload names
/// `task_id`, as `(intent, payload)`: the Transparency Log's join key for a
/// crashed Worker's `task.orphaned` (at its pid) and disposition (at pid 0).
fn task_rows(
    tl: &TransparencyLogAdapter,
    task_id: &str,
    pid: u32,
) -> Result<Vec<(String, serde_json::Value)>, String> {
    Ok(tl
        .query_frames(FrameFilter {
            spirit_pid: Some(pid),
            kind: Some(FrameKind::TaskComplete),
            ..FrameFilter::default()
        })
        .map_err(|e| format!("maos run: crash rows for {task_id} unreadable: {e}"))?
        .into_iter()
        .filter_map(|row| {
            let payload = serde_json::from_slice::<serde_json::Value>(&row.payload_redacted);
            Some((row.intent, payload.ok()?))
        })
        .filter(|(_, payload)| payload["task_id"] == task_id)
        .collect())
}

/// D-19-6-03 A — whether the kernel's crash handler journaled
/// `task.orphaned` for `task_id` at the Worker's pid, i.e. an orphan frame was
/// owed to the Orchestrator. A pid-0 disposition row alone is no evidence: a
/// planned stop (door unload, session timeout) writes one too. With no orphan
/// frame and no such row (a planned stop, no exit code, a handler past its
/// join budget) the loop reports the Worker's non-completion, not a delivery
/// fault.
pub fn orphan_recorded(
    tl: &TransparencyLogAdapter,
    task_id: &str,
    worker_pid: u32,
) -> Result<bool, String> {
    Ok(task_rows(tl, task_id, worker_pid)?
        .iter()
        .any(|(intent, _)| intent == "task.orphaned"))
}

/// D-19-6-03/04/06 A — the loop's disposition of a delegated Worker's crash,
/// called once its orphan frame was drained and the Orchestrator's in-flight
/// delegation released. Both facts are the Transparency Log's, joined on
/// `task_id`: the cause is the kernel's `task.orphaned` row at the Worker's
/// pid — absent on the crash-phase `NotLoaded` path (a concurrent unload
/// removed the SCB first), where `fallback_cause`, the Worker's own verdict,
/// stands in; the APPLIED disposition is the pid-0 row the kernel or the
/// supervisor's executor wrote (an unarmed `reassign-to-replica` is already
/// `task.escalated` there). Prints `delegation_orphaned`. `Ok(cause)` means
/// re-dispatch the section once; otherwise the typed stop, after one
/// `AnomalyFlagged` on escalation. `reassign_without_prior` (r19_6_d04): the
/// entry declared `reassign-to-replica` but the loop left the resolver
/// unarmed for want of a prior distillate (FR21) — the escalation's event
/// carries `"reason":"no_prior_distillate"` and its stop names that reason.
#[allow(clippy::too_many_arguments)]
pub fn dispose_orphan(
    tl: &TransparencyLogAdapter,
    notifications: &NotificationDispatcher,
    section: Option<usize>,
    task_id: &str,
    worker_pid: u32,
    fallback_cause: &str,
    safe_point: bool,
    reassign_without_prior: bool,
) -> Result<String, String> {
    let cause = task_rows(tl, task_id, worker_pid)?
        .into_iter()
        .filter(|(intent, _)| intent == "task.orphaned")
        .find_map(|(_, payload)| payload["cause"].as_str().map(str::to_owned))
        .unwrap_or_else(|| fallback_cause.to_owned());
    let disposition = task_rows(tl, task_id, 0)?
        .into_iter()
        .find_map(|(intent, _)| match intent.as_str() {
            "task.nacked" => Some(OnCrashAction::Nack),
            "task.reassigned" => Some(OnCrashAction::ReassignToReplica),
            "task.escalated" => Some(OnCrashAction::EscalateToOperator),
            _ => None,
        })
        .ok_or_else(|| format!("maos run: no disposition row for crashed {task_id}"))?;
    let no_prior = reassign_without_prior && disposition == OnCrashAction::EscalateToOperator;
    let mut event = serde_json::json!({
        "event": "delegation_orphaned",
        "section": section,
        "task_id": task_id,
        "cause": cause,
        "disposition": disposition.to_string(),
        "orchestrator_safe_point": safe_point,
    });
    if no_prior {
        event["reason"] = "no_prior_distillate".into();
    }
    println!("{event}");
    if disposition == OnCrashAction::ReassignToReplica {
        return Ok(cause);
    }
    let section = section.map_or_else(|| "-".to_owned(), |ordinal| ordinal.to_string());
    if disposition == OnCrashAction::EscalateToOperator {
        let event = NotificationEvent::anomaly_flagged(
            "kernel.crash-detector",
            format!("pid:{worker_pid}"),
            format!("section {section}: {cause}, task {task_id}"),
            1.0,
        )
        .map_err(|e| format!("maos run: escalation notification refused: {e}"))?;
        notifications
            .dispatch(event, NotificationLevel::Immediate)
            .map_err(|e| format!("maos run: escalation notification failed: {e}"))?;
    }
    let reason = if no_prior {
        " (reassign-to-replica needs a prior distillate)"
    } else {
        ""
    };
    Err(format!(
        "maos run: section {section} Worker crashed ({cause}); disposition {disposition}{reason}; \
         dispatch stopped"
    ))
}
