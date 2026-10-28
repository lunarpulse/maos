//! Story 19-1 — the `maos run` root's half of the Orchestrator: the ONE
//! construction every Orchestrator of the root goes through (topology arm,
//! standalone arm, hot-swap successor; D-19-1-04 A), the `--epic` load and
//! its refusals (D-19-1-02/03 A), the pending-halt check before each
//! `task.assign` (D-19-1-09 A) and the per-section distillate at the Worker's
//! pid (D-19-1-07/08 A). In the library, not `main.rs`, so
//! `crates/maos-bin/tests/` drives the production functions.

use std::path::Path;
use std::sync::{Arc, Mutex};

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{EpistemicHaltPayload, PriorDistillateRef};
use maos_domain::halt::HaltState;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
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

/// D-19-1-02 / 03 A — read and split `--epic <path>` for a topology with
/// `host_entries` host-bearing entries. Refused typed, before any emit: on the
/// cross-host arm (the far Host runs the Worker and nothing returns to
/// distil), with no host-bearing entry, an unreadable file, or a split the
/// Orchestrator refuses (a non-`### Story` heading, a count mismatch).
pub fn load_epic(
    path: &Path,
    host_entries: usize,
    cross_host: bool,
) -> Result<Vec<EpicSection>, String> {
    if cross_host {
        return Err(
            "maos run: --epic dispatches on the loopback arm only; on the cross-host arm the far \
             Host runs each Worker and nothing returns to distil, so section k+1 could never \
             carry section k's distillate. Drop --epic and set MAOS_DELEGATED_GOAL"
                .into(),
        );
    }
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

/// D-19-1-09 A — the ids of every halt pending resolution for `pid`, read
/// without draining (`HaltRegistry::drain_for_spirit_dry_run`).
pub fn pending_halts(registry: &HaltRegistry, pid: u32) -> Vec<String> {
    registry
        .drain_for_spirit_dry_run(pid)
        .into_iter()
        .filter(|(_, state)| *state == HaltState::PendingResolution)
        .map(|(id, _)| id.as_str().to_owned())
        .collect()
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
/// carries (FR21).
pub fn write_section_distillate(
    writer: &dyn DistillationPort,
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
    Ok(PriorDistillateRef {
        digest_frame_id: receipt.digest_frame_id,
        distillation_depth: receipt.effective_distillation_depth,
        intent_lineage: receipt.intent_lineage,
    })
}
