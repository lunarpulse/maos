#![forbid(unsafe_code)]

//! Story 10.2 AC1 — xtask CI gate: parse `docs/third-party-trial/results/trial-results.toml`
//! via typed serde deserialization. Integrity failures are fatal at every N.
//! Story 22-7 (D-22-7-10 A): the human-author denominator is removed. There is no sampling floor
//! (no N = 12, no >= 10 successes, no strata) and no Wilson interval: a cohort of N >= 1 that
//! passes every per-participant check reads Green and says its N; any per-participant finding
//! reads Red; N = 0 reads Insufficient. A green gate does not claim an N = 12 human cohort.
//! Registry disposition decides whether a verdict blocks, never whether it passed.

use serde::Deserialize;
use std::path::Path;

/// Root of `docs/third-party-trial/results/trial-results.toml`.
#[derive(Debug, Deserialize)]
pub struct TrialResults {
    pub trial: TrialSection,
    pub participant: Vec<Participant>,
}

#[derive(Debug, Deserialize)]
pub struct TrialSection {
    pub participants_total: i64,
    pub successes: i64,
    pub trial_start: String,
    pub trial_end: String,
    pub methodology_version: String,
}

#[derive(Debug, Deserialize)]
pub struct Participant {
    pub id: String,
    pub produced_binary: bool,
    pub binary_loads: bool,
    pub frames_run: i64,
    pub halt_recall: f64,
    pub sbom_verified: bool,
    pub signing_chain_verified: bool,
}

/// Reject negative counts — these indicate malformed input, not a failed assertion.
fn validate_non_negative(name: &str, value: i64) -> Result<(), String> {
    if value < 0 {
        return Err(format!("{name} is negative ({value}) — invalid input"));
    }
    Ok(())
}

/// #32: delegate to shared chrono-based validator (was cosmetic `contains('-')` check).
fn validate_dates(start: &str, end: &str) -> Result<(), String> {
    validate_dates_shared("trial_start", start, "trial_end", end)
}

// #33: emit_command + validate_dates extracted to gate_common (shared across all gate modules).
use crate::gate_common::{
    validate_dates as validate_dates_shared, Observation, OutOfLedgerVerdict,
};

const RESULTS_PATH: &str = "docs/third-party-trial/results/trial-results.toml";

/// True at the v2.0 ship phase, where the consumer additionally requires a
/// producer-SIGNED derived attestation per participant.
///
/// Tolerates the dotted `v2.0` spelling alongside the ladder's `v2_0`. Story
/// 15-3 AC2(b) retired the ship-phase ENV source this used to read (the name is
/// recorded in ADR-065, deliberately not repeated here — AC1's oracle counts
/// FILES carrying the token, so naming it in a comment would defeat the check
/// that it is gone). A GitHub *repository variable* could move this one gate's
/// phase with no code change, and `check-env-contract` can never see it — it
/// scans only the maos-bin src tree — which is how a third phase source
/// survived eight epics. The phase is now `gate_common::CURRENT_PHASE`.
fn is_v2_blocking_phase(phase: &str) -> Result<bool, String> {
    let requested = phase.trim();
    let canonical = if requested.eq_ignore_ascii_case("v2.0") {
        "v2_0"
    } else {
        crate::gate_common::PHASE_ORDER
            .iter()
            .copied()
            .find(|candidate| candidate.eq_ignore_ascii_case(requested))
            .ok_or_else(|| {
                format!(
                    "check-third-party-trial: invalid ship phase `{requested}`; expected one of {}",
                    crate::gate_common::PHASE_ORDER.join(", ")
                )
            })?
    };
    let current = crate::gate_common::PHASE_ORDER
        .iter()
        .position(|candidate| *candidate == canonical)
        .expect("canonical phase came from PHASE_ORDER");
    let v2 = crate::gate_common::PHASE_ORDER
        .iter()
        .position(|candidate| *candidate == "v2_0")
        .expect("PHASE_ORDER contains v2_0");
    Ok(current >= v2)
}

/// Resolve a repo-relative path against the workspace root (not the CWD), so the
/// gate still finds `trial-results.toml` / `derived-attestations.json` when invoked
/// from a subdirectory (P15). Walks up from CWD to the dir containing both a
/// workspace `Cargo.toml` and an `xtask/` directory.
fn resolve_workspace_path(relative: &str) -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir: Option<&std::path::Path> = Some(&cwd);
    while let Some(d) = dir {
        if d.join("Cargo.toml").exists() && d.join("xtask").exists() {
            return Some(d.join(relative));
        }
        dir = d.parent();
    }
    None
}

/// Load the producer-emitted, producer-signed per-participant derived attestations
/// (D1). Each signature is verified against the producer pubkey; only validly-
/// signed attestations are returned. Returns an empty map when the file is absent
/// (v1.0/v1.5 path — provenance is not required there).
fn load_signed_attestations(
) -> std::collections::HashMap<String, maos_eval::trial_attestation::DerivedParticipantAttestation>
{
    let path = match resolve_workspace_path(maos_eval::trial_attestation::DERIVED_ATTESTATIONS_PATH)
    {
        Some(p) => p,
        None => std::path::PathBuf::from(maos_eval::trial_attestation::DERIVED_ATTESTATIONS_PATH),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        // Absent file is the v1.0/v1.5 path (provenance not required) — silent.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return std::collections::HashMap::new()
        }
        // Any OTHER read failure (permissions, truncated) is a real defect — surface
        // it so an operator is not misled by 12× "missing producer-signed" messages.
        Err(e) => {
            eprintln!(
                "check-third-party-trial: WARNING — derived-attestations.json unreadable: {e}"
            );
            return std::collections::HashMap::new();
        }
    };
    let signed: Vec<maos_eval::trial_attestation::SignedAttestation> =
        match serde_json::from_str(&content) {
            Ok(v) => v,
            // A malformed/truncated JSON is a producer bug, not an absent file — surface it.
            Err(e) => {
                eprintln!(
                    "check-third-party-trial: WARNING — derived-attestations.json malformed: {e}"
                );
                return std::collections::HashMap::new();
            }
        };
    let pubkey = maos_eval::trial_attestation::producer_pubkey();
    let mut map = std::collections::HashMap::new();
    for att in signed
        .into_iter()
        .filter_map(|s| maos_eval::trial_attestation::verify_signed_attestation(&s, &pubkey).ok())
    {
        let id = att.participant_id.clone();
        if map.insert(id.clone(), att).is_some() {
            eprintln!(
                "check-third-party-trial: WARNING — duplicate signed attestation for participant {id} (last wins)"
            );
        }
    }
    map
}

/// `ship_phase` is supplied by the caller and defaults to the single shared
/// `gate_common::CURRENT_PHASE` (see the `--ship-phase` argument in `main.rs`).
/// It is an explicit ARGUMENT rather than an env read on purpose: story 15-3
/// AC2(b) retired the ambient env source, because a GitHub repository variable
/// could move this one gate's phase with no code change and no instrument
/// could see it. An argument cannot be set without editing a reviewed file.
pub fn run(json: bool, ship_phase: &str) -> Result<(), String> {
    let require_derivation_provenance = is_v2_blocking_phase(ship_phase)?;
    let ship_phase = if ship_phase.trim().eq_ignore_ascii_case("v2.0") {
        "v2_0"
    } else {
        crate::gate_common::PHASE_ORDER
            .iter()
            .copied()
            .find(|phase| phase.eq_ignore_ascii_case(ship_phase.trim()))
            .expect("ship phase validated above")
    };
    let (verdict, details) = match check(ship_phase, require_derivation_provenance) {
        Ok(report) => report,
        Err(error) => (
            OutOfLedgerVerdict::judge_registered(
                "check-third-party-trial",
                Observation::Red,
                true,
                ship_phase,
                &error,
            )?,
            serde_json::json!({"error": error}),
        ),
    };
    verdict.finish(json, details)
}

fn check(
    ship_phase: &str,
    require_derivation_provenance: bool,
) -> Result<(OutOfLedgerVerdict, serde_json::Value), String> {
    let path = resolve_workspace_path(RESULTS_PATH)
        .unwrap_or_else(|| Path::new(RESULTS_PATH).to_path_buf());

    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                OutOfLedgerVerdict::judge_registered(
                    "check-third-party-trial",
                    Observation::NotRun,
                    false,
                    ship_phase,
                    "trial-results.toml absent — third-party trial pending",
                )?,
                serde_json::json!({"trial_status": "absent"}),
            ));
        }
        Err(error) => return Err(format!("cannot read {RESULTS_PATH}: {error}")),
    };
    let results: TrialResults =
        toml::from_str(&content).map_err(|e| format!("malformed trial-results.toml: {e}"))?;

    let t = &results.trial;
    // D1: at v2.0 provenance is a producer-SIGNED per-participant derived
    // attestation (verified against the producer pubkey), never a pasteable
    // bare-string stamp and never a file-level stamp that launders all
    // participants. The producer (`check-trial-attestation`) emits these.
    let signed_attestations = load_signed_attestations();
    // R2-D2 guard: the dev producer keypair is PUBLIC (committed for local/test
    // builds). At the v2.0 ship gate the consumer MUST NOT trust it — otherwise a
    // coordinator with repo read could mint accepted attestations with the public
    // dev seed. Production CI sets MAOS_TRIAL_PRODUCER_PUBKEY to a real key; if it
    // is unset at v2.0, fail loud rather than silently trusting the forgeable dev key.
    if require_derivation_provenance {
        let trusted = maos_eval::trial_attestation::producer_pubkey();
        let dev_pubkey = maos_audit::sealed_export::derive_pubkey(
            &maos_eval::trial_attestation::producer_dev_seed(),
        );
        if trusted == dev_pubkey {
            return Err(format!(
                "check-third-party-trial: FAIL — v2.0 requires MAOS_TRIAL_PRODUCER_PUBKEY set to a \
                 production key (the committed dev key is not trusted at the v2.0 ship gate)"
            ));
        }
    }

    // Reject negative counts up front (input validity, not assertion).
    let counts = [
        ("participants_total", t.participants_total),
        ("successes", t.successes),
    ];
    for (name, val) in counts {
        validate_non_negative(name, val)?;
        if val > t.participants_total {
            return Err(format!(
                "{name}={val} > participants_total={}",
                t.participants_total
            ));
        }
    }
    validate_dates(&t.trial_start, &t.trial_end)?;

    // ── Integrity checks (axis-1 precondition: always fatal, not advisory) ─────────
    // #20: successes/participants consistency — impossible counts are malformed input.
    if t.successes > t.participants_total {
        return Err(format!(
            "check-third-party-trial: FAIL — successes={} > participants_total={} (impossible)",
            t.successes, t.participants_total
        ));
    }
    // #1/#21: participant records must match the declared cohort exactly and be unique.
    let participants_total = usize::try_from(t.participants_total).map_err(|_| {
        format!(
            "check-third-party-trial: FAIL — participants_total={} overflows usize",
            t.participants_total
        )
    })?;
    if results.participant.len() != participants_total {
        return Err(format!(
            "check-third-party-trial: FAIL — participant records={} != participants_total={} \
             (cohort must be fully enumerated; self-reported count is not trusted)",
            results.participant.len(),
            t.participants_total
        ));
    }
    {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for p in &results.participant {
            if !seen.insert(p.id.as_str()) {
                return Err(format!(
                    "check-third-party-trial: FAIL — duplicate participant id '{}' (inflates cohort)",
                    p.id
                ));
            }
        }
    }

    // ── Per-participant validation (F6→C): validate EVERY participant unconditionally ─
    // #2: the success conjunction is derived per participant; non-producers are NOT
    // skipped — they simply fail the conjunction and count as non-successes.
    let mut failures: Vec<String> = Vec::new();
    let mut derived_successes: i64 = 0;
    for p in &results.participant {
        if p.frames_run < 0 {
            return Err(format!(
                "participant {} frames_run is negative ({}) — invalid input",
                p.id, p.frames_run
            ));
        }
        // D1: at v2.0 the EFFECTIVE fields come from the producer-SIGNED derived
        // attestation (a coordinator cannot forge it without the producer private
        // key). The self-reported TOML fields are probes the derived value
        // overrides — this closes the "10.2 canned-trap" (a planted `sbom_verified
        // = true` over a tampered artifact no longer turns the gate green).
        let signed = signed_attestations.get(&p.id);
        let (
            produced_binary,
            binary_loads,
            frames_run,
            halt_recall,
            sbom_verified,
            signing_chain_verified,
        ) = if require_derivation_provenance {
            match signed {
                Some(att) => (
                    att.produced_binary,
                    att.binary_loads,
                    att.frames_run,
                    att.halt_recall,
                    att.sbom_verified,
                    att.signing_chain_verified,
                ),
                None => {
                    failures.push(format!(
                            "participant {} missing producer-signed derived attestation (v2.0 default-deny)",
                            p.id
                        ));
                    (p.produced_binary, false, 0_i64, 0.0_f64, false, false)
                }
            }
        } else {
            (
                p.produced_binary,
                p.binary_loads,
                p.frames_run,
                p.halt_recall,
                p.sbom_verified,
                p.signing_chain_verified,
            )
        };
        if frames_run < 0 {
            return Err(format!(
                "participant {} effective frames_run is negative ({frames_run}) — invalid input",
                p.id
            ));
        }

        // halt_recall must be a finite probability in [0,1] (on the EFFECTIVE value).
        if !halt_recall.is_finite() {
            return Err(format!(
                "participant {} halt_recall={:.3} (not finite)",
                p.id, halt_recall
            ));
        } else if !(0.0..=1.0).contains(&halt_recall) {
            return Err(format!(
                "participant {} halt_recall={:.3} (out of [0,1])",
                p.id, halt_recall
            ));
        }

        // Success conjunction over the EFFECTIVE (derived at v2.0) fields.
        let v2_attestation_ok = !require_derivation_provenance
            || (signed.is_some() && sbom_verified && signing_chain_verified);
        let is_success = produced_binary
            && binary_loads
            && frames_run >= 1000
            && halt_recall.is_finite()
            && halt_recall >= 0.85
            && halt_recall <= 1.0
            && v2_attestation_ok;
        if is_success {
            derived_successes += 1;
        } else if produced_binary && binary_loads {
            // Producer that didn't clear the quality bar — flag the specific shortfall.
            if frames_run < 1000 {
                failures.push(format!(
                    "participant {} ran {} frames (< 1000)",
                    p.id, frames_run
                ));
            }
            if halt_recall.is_finite() && halt_recall < 0.85 {
                failures.push(format!(
                    "participant {} halt_recall={:.3} (< 0.85)",
                    p.id, halt_recall
                ));
            }
            if require_derivation_provenance && !sbom_verified {
                failures.push(format!(
                    "participant {} derived sbom_verified=false (v2.0)",
                    p.id
                ));
            }
            if require_derivation_provenance && !signing_chain_verified {
                failures.push(format!(
                    "participant {} derived signing_chain_verified=false (v2.0)",
                    p.id
                ));
            }
        }
    }

    // #1: reconcile derived vs reported successes — self-reported count is NOT trusted.
    if derived_successes != t.successes {
        return Err(format!(
            "check-third-party-trial: FAIL — successes={} does not match participant records (derived={derived_successes}); {}",
            t.successes, failures.join("; ")
        ));
    }
    // Story 22-7 (D-22-7-10 A): no sampling floor. The human-author denominator (the N = 12 and
    // >= 10 floors, the strata and the Wilson interval) is removed; what a cohort of any size
    // must still do is pass every per-participant check above.
    let n = t.participants_total;
    let observation = if !failures.is_empty() {
        Observation::Red
    } else if n == 0 {
        Observation::Insufficient
    } else {
        Observation::Green
    };
    let reason = if !failures.is_empty() {
        failures.join("; ")
    } else if n == 0 {
        "N=0; no participants recorded".to_owned()
    } else {
        format!("N={n}; no sampling floor (human-author denominator removed, 22-7)")
    };
    Ok((
        OutOfLedgerVerdict::judge_registered(
            "check-third-party-trial",
            observation,
            true,
            ship_phase,
            reason,
        )?,
        serde_json::json!({
            "trial_status": if n == 0 { "empty" } else { "recorded" },
            "participants_total": t.participants_total,
            "successes_reported": t.successes,
            "successes_derived": derived_successes,
            "trial_start": t.trial_start,
            "trial_end": t.trial_end,
            "methodology_version": t.methodology_version,
            "provenance_required": require_derivation_provenance,
            "failures": failures,
        }),
    ))
}
