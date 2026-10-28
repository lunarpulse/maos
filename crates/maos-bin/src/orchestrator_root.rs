//! Story 19-1 — the `maos run` root's half of the Orchestrator: the ONE
//! construction every Orchestrator of the root goes through (topology arm,
//! standalone arm, hot-swap successor; D-19-1-04 A), the `--epic` load and
//! its refusals (D-19-1-02/03 A), the stop check before each `task.assign`
//! (D-19-1-09 A) and the per-section distillate at the Worker's pid
//! (D-19-1-07/08 A). In the library, not `main.rs`, so
//! `crates/maos-bin/tests/` drives the production functions.

use std::ffi::OsStr;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{EpistemicHaltPayload, PriorDistillateRef};
use maos_domain::halt::HaltState;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::transparency_log::FrameKind;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use orchestrator::{
    split_epic, EpicSection, Orchestrator, OrchestratorInferenceBinding, PidSource,
    SCALAR_TAG_DISPATCH_AMBIGUITY,
};
use sha2::{Digest, Sha256};

use crate::worker_spawn::WorkerRun;

/// The debug-build harness plant for the AC4.3(b) proven-red: before the
/// section whose 1-based ordinal it names, a real pending halt is raised for
/// the Orchestrator's pid through the kernel's `invoke_halt`. Release builds
/// never read it.
pub const HALT_PLANT_ENV: &str = "MAOS_TEST_ONLY_HALT_BEFORE_SECTION";

/// D-19-1-04 A — the ONE Orchestrator binding: the context port through the
/// seam's binder (18-5) on every path; under `--live` or an explicit mode the
/// Inference Port from the seam's builder (18-1) with a per-pass mint naming
/// the first declared provider the router registers. Unset mode returns it
/// unbound (ADR-064). The port serves only `judge_ambiguity`, never dispatch.
pub fn construct(
    orchestrator: Orchestrator,
    seam: &butler::ButlerSeam,
    declared: &[String],
    pid_of: PidSource,
) -> Result<Orchestrator, String> {
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
    let orchestrator = construct(Orchestrator::new(spirit_id), &seam, declared, pid_of)
        .map_err(|e| format!("maos run: {e}"))?;
    Ok((orchestrator, cell))
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
