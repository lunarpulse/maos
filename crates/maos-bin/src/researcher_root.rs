//! Story 18-3 — the `maos run` root's half of the Researcher's Inference
//! Port seam (CR-1: the Butler seam's port builder, registered ids and mint;
//! no second seam) and the `--once` verdict on a bound pass. In the library,
//! not `main.rs`, so `crates/maos-bin/tests/` drives the production functions.

use std::sync::{Arc, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::{DistillationPort, InferencePort};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::HookOutcome;
use maos_kernel_core::security::OutputShapePredicate;
use researcher::{PidSource, Researcher, ResearcherInferenceBinding, SEED_CLAIM_INTENT};
use sha2::{Digest, Sha256};

/// What a bound Researcher writes into its host: the seed journal and the
/// distillate writer.
pub struct ResearcherHost {
    pub seed_sink: researcher::SeedSink,
    pub distill: Arc<dyn DistillationPort + Send + Sync>,
}

impl ResearcherHost {
    /// D-18-3-01 A — each seed claim is a TL frame at the pass's pid whose id
    /// is derived from `(pid, payload)`, so a rejournal is a no-op duplicate.
    pub fn new(
        tl: Arc<TransparencyLogAdapter>,
        distill: Arc<dyn DistillationPort + Send + Sync>,
    ) -> Self {
        let seed_sink: researcher::SeedSink = Arc::new(move |pid, payload| {
            let digest = Sha256::new()
                .chain_update(b"maos.researcher.seed/v1")
                .chain_update(pid.to_be_bytes())
                .chain_update(payload)
                .finalize();
            let mut id = [0u8; 16];
            id.copy_from_slice(&digest[..16]);
            let _ = tl.insert_frame_event_with_id(
                Some(id),
                FrameKind::TelemetryEvent,
                pid,
                "",
                "",
                None,
                SEED_CLAIM_INTENT,
                payload,
                FrameOrigin::SpiritAuto,
            );
            Ok(id)
        });
        Self { seed_sink, distill }
    }
}

/// D-18-3-02 B / D-18-3-04 A — under `--live` or an explicit mode (the seam
/// carries a port builder), bind `researcher` to its first declared provider
/// the router registers; none registered refuses here, at boot or load.
/// Deterministic mode returns it unbound (ADR-064).
pub fn attach(
    researcher: Researcher,
    seam: &butler::ButlerSeam,
    declared: &[String],
    pid_of: PidSource,
    host: &ResearcherHost,
) -> Result<Researcher, String> {
    let Some((build_port, registered)) = seam.inference.as_ref() else {
        return Ok(researcher);
    };
    let provider = researcher::declared_provider(declared, registered)?;
    let port = build_port().ok_or("researcher inference: the host is shutting down")?;
    Ok(bind(researcher, port, provider, seam, pid_of, host))
}

/// Bind `port` with a per-pass mint (D-18-3-03 A) naming `provider`.
pub fn bind(
    researcher: Researcher,
    port: Arc<dyn InferencePort + Send + Sync>,
    provider: String,
    seam: &butler::ButlerSeam,
    pid_of: PidSource,
    host: &ResearcherHost,
) -> Researcher {
    eprintln!("researcher inference seam wired (provider `{provider}`)");
    let binding = ResearcherInferenceBinding::per_pass(provider, Arc::clone(&seam.mint), pid_of);
    researcher
        .with_deferred_inference_port(port, Arc::new(Mutex::new(Some(binding))))
        .with_seed_sink(Arc::clone(&host.seed_sink))
        .with_distillation_port(Arc::clone(&host.distill))
}

/// D-18-3-05 A — the `--once` verdict on a bound pass: a refusal, an
/// incomplete pass, consumed completions ≠ surveyed claims, or an
/// `output_shape` violation is an error; otherwise the `researcher_survey`
/// event and the pass's primary scalar tag. Unbound passes yield `None`.
pub fn pass_verdict(
    researcher: Option<&Researcher>,
    outcome: &HookOutcome,
    predicate: &OutputShapePredicate,
) -> Result<Option<(serde_json::Value, &'static str)>, String> {
    let Some(researcher) = researcher.filter(|r| r.is_live()) else {
        return Ok(None);
    };
    if let Some(error) = researcher.take_pass_error() {
        return Err(format!("researcher on_idle pass refused: {error}"));
    }
    if !matches!(
        outcome,
        HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { fired: true, .. }
    ) {
        return Err(format!(
            "researcher on_idle pass did not complete: {outcome:?}"
        ));
    }
    let output = researcher
        .last_output()
        .ok_or("researcher on_idle pass surveyed nothing")?;
    let (consumed, claims) = (researcher.completions_consumed(), output.findings.len());
    if consumed != claims {
        return Err(format!(
            "researcher consumed {consumed} completions for {claims} surveyed claims"
        ));
    }
    let mut event =
        serde_json::to_value(&output).map_err(|e| format!("researcher survey encode: {e}"))?;
    predicate
        .check(&event)
        .map_err(|violation| format!("output_shape violation: {violation}"))?;
    event["event"] = "researcher_survey".into();
    event["completions_consumed"] = consumed.into();
    Ok(Some((event, output.primary_scalar().0)))
}
