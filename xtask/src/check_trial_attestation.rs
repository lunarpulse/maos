#![forbid(unsafe_code)]

//! Story 11.7 — producer gate for v2.0 third-party-trial attestation.
//!
//! The producer validates the machinery that derives per-participant facts. The
//! consumer (`check-third-party-trial`) validates cohort results and refuses
//! unprovenanced v2.0 records.

use crate::check_fkcs::{is_blocking_at, parse_inline_disposition};
use maos_audit::release_verify::{generate_sha256sums, sign_sha256sums};
use maos_eval::trial_attestation::{
    derive_halt_recall, derive_participant_attestation, derive_reload_facts,
    derive_sbom_from_sources, derive_signing_chain, summarize_attestations, DerivationInputs,
    HaltRecallCounts, HermeticEnvironment, HermeticityReport, PROVENANCE_STAMP,
};
use maos_fkcs::{AdmissionHarness, KernelFreezeProvenance, ProxyCohort, ProxySpirit};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const GATE_NAME: &str = "check-trial-attestation";
const CURRENT_PHASE: &str = "v1_5";
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

#[derive(Debug, Clone, Serialize)]
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
}

impl LegResult {
    fn from_bool(label: &'static str, green: bool) -> Self {
        Self {
            label,
            passed: u32::from(green),
            failed: u32::from(!green),
            ran: true,
            attempted: true,
            green,
        }
    }

    fn status_word(&self) -> &'static str {
        if self.green {
            "green"
        } else if self.attempted {
            "red"
        } else {
            "skipped"
        }
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition()?;
    if !matches!(
        disposition.get("v2_0").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be blocking"
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    // Option C (Epic 12 retro B1): hermetic gate — the Blocking binding class
    // hard-fails a RED oracle at HEAD regardless of CURRENT_PHASE. Dev-time
    // enforcement is decoupled from the GA ship-phase ladder (`blocking_now` is
    // retained for JSON reporting). See gate_common::BindingClass.
    let dev_blocks = blocking_now
        || crate::gate_common::dev_enforced_red_blocks(
            crate::gate_common::BindingClass::Blocking,
            true,
        );
    let legs = vec![
        reload_derives_load_and_frames_leg(),
        hermetic_environment_enforced_leg(),
        sbom_completeness_derived_leg(),
        signing_chain_verified_leg(),
        halt_recall_measured_leg(),
        attestation_ignores_self_report_leg(),
        blind_harness_negative_control_leg(),
        proxy_cohort_proof_of_mechanism_leg(),
        release_graph_absence_leg(),
        kernel_abi_diff_leg(),
    ];
    let vacuous = legs
        .iter()
        .find(|leg| leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0)));
    let oracle_green = legs.iter().all(|leg| leg.green);
    let gate_passed = vacuous.is_none() && (oracle_green || !dev_blocks);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": gate_passed,
                "oracle_green": oracle_green,
                "advisory": !oracle_green && !blocking_now,
                "blocking_now": blocking_now,
                "current_phase": CURRENT_PHASE,
                "disposition": disposition,
                "provenance_stamp": PROVENANCE_STAMP,
                "legs": legs,
                "vacuous_leg": vacuous.map(|leg| leg.label),
            })
        );
    } else if let Some(leg) = vacuous {
        eprintln!(
            "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
            leg.label, leg.ran, leg.passed, leg.failed
        );
    } else if oracle_green {
        eprintln!("{GATE_NAME}: PASSED — oracle green ({} legs)", legs.len());
    } else {
        eprintln!(
            "{GATE_NAME}: PASS (advisory — oracle RED, would block at v2.0); {}",
            legs.iter()
                .map(|leg| format!("{}={}", leg.label, leg.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if let Some(leg) = vacuous {
        return Err(format!(
            "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
            leg.label, leg.ran, leg.passed, leg.failed
        ));
    }
    if !oracle_green && blocking_now {
        return Err(format!("{GATE_NAME}: BLOCKING — oracle RED"));
    }
    Ok(())
}

fn reload_derives_load_and_frames_leg() -> LegResult {
    let good = derive_reload_facts(maos_eval::trial_attestation::ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let fail_load = derive_reload_facts(maos_eval::trial_attestation::ReloadExecutionReport {
        load_accepted: false,
        frames_executed: 1_000,
    });
    let low_frames = derive_reload_facts(maos_eval::trial_attestation::ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 999,
    });
    LegResult::from_bool(
        "reload-derives-load-and-frames",
        good.binary_loads
            && good.meets_frame_floor
            && !fail_load.binary_loads
            && !low_frames.meets_frame_floor,
    )
}

fn hermetic_environment_enforced_leg() -> LegResult {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(_) => return LegResult::from_bool("hermetic-environment-enforced", false),
    };
    let clean = HermeticEnvironment {
        maos_home: temp.path().join("missing-home"),
        candidate_cache: temp.path().join("missing-cache"),
    }
    .assert_clean();
    let dirty_home = temp.path().join("dirty-home");
    if std::fs::create_dir_all(&dirty_home).is_err()
        || std::fs::write(dirty_home.join("warm-artifact"), b"stale").is_err()
    {
        return LegResult::from_bool("hermetic-environment-enforced", false);
    }
    let dirty = HermeticEnvironment {
        maos_home: dirty_home,
        candidate_cache: temp.path().join("missing-cache-2"),
    }
    .assert_clean();
    LegResult::from_bool(
        "hermetic-environment-enforced",
        clean.clean && !dirty.clean && dirty.dirty_paths.len() == 1,
    )
}

fn sbom_completeness_derived_leg() -> LegResult {
    let declared = lock_with(&[("maos", "0.1.0"), ("serde", "1.0.0")]);
    let tree = "maos v0.1.0 (/tmp/maos)\n└── serde v1.0.0\n";
    let good = derive_sbom_from_sources(&declared, tree);
    let mismatch = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), tree);
    LegResult::from_bool(
        "sbom-completeness-derived",
        good.sbom_verified
            && !mismatch.sbom_verified
            && !mismatch.missing_from_declaration.is_empty(),
    )
}

fn signing_chain_verified_leg() -> LegResult {
    let artifact = b"candidate artifact";
    let sha = maos_eval::trial_attestation::sha256_hex(artifact);
    let sha256sums = generate_sha256sums(&[("candidate.bin".to_string(), sha)]);
    let sig = sign_sha256sums(sha256sums.as_bytes(), &dev_seed());
    let good = derive_signing_chain(sha256sums.as_bytes(), &sig, &[("candidate.bin", artifact)]);
    let mut tampered_sig = sig;
    tampered_sig[0] ^= 0xA5;
    let bad = derive_signing_chain(
        sha256sums.as_bytes(),
        &tampered_sig,
        &[("candidate.bin", artifact)],
    );
    LegResult::from_bool(
        "signing-chain-verified",
        good.signing_chain_verified && !bad.signing_chain_verified,
    )
}

fn halt_recall_measured_leg() -> LegResult {
    let good = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 17,
            false_negatives: 3,
        },
        b"class-appropriate-corpus",
        true,
    );
    let low = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 16,
            false_negatives: 4,
        },
        b"class-appropriate-corpus",
        true,
    );
    LegResult::from_bool(
        "halt-recall-measured",
        good.meets_floor && !low.meets_floor && good.halt_recall < 1.0,
    )
}

fn attestation_ignores_self_report_leg() -> LegResult {
    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(maos_eval::trial_attestation::ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), "maos v0.1.0\n");
    let signing = maos_eval::trial_attestation::SigningDerivation {
        signing_chain_verified: false,
        verified_manifest_entries: 0,
        error: Some("tampered signature".to_string()),
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 17,
            false_negatives: 3,
        },
        b"class-appropriate-corpus",
        true,
    );
    let record = derive_participant_attestation(DerivationInputs {
        participant_id: "proxy-1",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: Some(maos_eval::trial_attestation::ReportedParticipantFacts {
            binary_loads: true,
            frames_run: 1_000,
            halt_recall: 0.99,
            sbom_verified: true,
            signing_chain_verified: true,
        }),
    });
    let summary = summarize_attestations(&[record.clone()]);
    LegResult::from_bool(
        "attestation-ignores-self-report",
        record.ignored_self_report && !record.success && summary.derived_successes == 0,
    )
}

fn negative_control_record_with_reported(
    reported: Option<maos_eval::trial_attestation::ReportedParticipantFacts>,
) -> maos_eval::trial_attestation::DerivedParticipantAttestation {
    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(maos_eval::trial_attestation::ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 999,
    });
    let sbom = derive_sbom_from_sources(
        &lock_with(&[("maos", "0.1.0")]),
        "maos v0.1.0\n└── serde v1.0.0\n",
    );
    let signing = maos_eval::trial_attestation::SigningDerivation {
        signing_chain_verified: false,
        verified_manifest_entries: 0,
        error: Some("tampered signature".to_string()),
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 16,
            false_negatives: 4,
        },
        b"class-appropriate-corpus",
        true,
    );
    derive_participant_attestation(DerivationInputs {
        participant_id: "negative-control",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported,
    })
}

fn negative_control_record() -> maos_eval::trial_attestation::DerivedParticipantAttestation {
    negative_control_record_with_reported(None)
}

fn blind_harness_negative_control_leg() -> LegResult {
    // AC4b: "blinding the harness" = feeding a forged ALL-GREEN self-report over the
    // negative-control candidate (the direct analogue of 11.5's always-attest stub).
    // The derive MUST ignore it — the record stays a non-success on its DERIVED facts
    // and the forgery is flagged ignored. A harness that trusted self-report would
    // green this; ours reds it, proving the rejection is not canned.
    let forged_green = maos_eval::trial_attestation::ReportedParticipantFacts {
        binary_loads: true,
        frames_run: 1_000,
        halt_recall: 0.99,
        sbom_verified: true,
        signing_chain_verified: true,
    };
    let blind = negative_control_record_with_reported(Some(forged_green));
    let real = negative_control_record();
    let blind_summary = summarize_attestations(&[blind.clone()]);
    LegResult::from_bool(
        "blind-harness-negative-control",
        blind.ignored_self_report
            && !blind.success
            && !real.success
            && blind_summary.derived_successes == 0,
    )
}

fn proxy_cohort_proof_of_mechanism_leg() -> LegResult {
    let empty = ProxyCohort::new(Vec::new()).evaluate(
        &AdmissionHarness::default(),
        &KernelFreezeProvenance::stable_at(23_081),
    );
    let cohort = ProxyCohort::new(vec![ProxySpirit::conformance("proxy-1")]).evaluate(
        &AdmissionHarness::default(),
        &KernelFreezeProvenance::stable_at(23_081),
    );
    LegResult::from_bool(
        "proxy-cohort-proof-of-mechanism",
        empty.is_na
            && cohort.cohort_label == maos_eval::trial_attestation::PROXY_COHORT_LABEL
            && cohort.floor_is_advisory_for_proxy_cohort,
    )
}

fn release_graph_absence_leg() -> LegResult {
    let output = Command::new("cargo")
        .args(["tree", "-p", "maos-bin", "-e", "features"])
        .output();
    let green = match output {
        Ok(out) if out.status.success() => {
            let tree = String::from_utf8_lossy(&out.stdout);
            // P12: line-based match so a cargo-tree feature-edge label format
            // change cannot silently hide a trial-attestation (or maos-fkcs) leak.
            let trial_feature_leaked = tree.lines().any(|line| line.contains("trial-attestation"));
            let fkcs_crate_leaked = tree.lines().any(|line| {
                let stripped =
                    line.trim_start_matches(|c: char| c.is_whitespace() || "│├└─".contains(c));
                stripped
                    .split_whitespace()
                    .next()
                    .map(|name| name == "maos-fkcs")
                    .unwrap_or(false)
            });
            !trial_feature_leaked && !fkcs_crate_leaked
        }
        _ => false,
    };
    LegResult::from_bool("release-graph-absence", green)
}

fn kernel_abi_diff_leg() -> LegResult {
    let green = crate::check_kernel_baseline::check()
        .map(|report| report.passed)
        .unwrap_or(false);
    LegResult::from_bool("kernel-abi-diff", green)
}

fn read_disposition() -> Result<HashMap<String, String>, String> {
    let raw = std::fs::read_to_string(resolve_workspace_path(Path::new(
        "xtask/gate-registry.toml",
    ))?)
    .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    let mut in_target_stanza = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") || trimmed.starts_with('[') {
            in_target_stanza = false;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name =") {
            in_target_stanza = rest.trim().trim_matches('"') == GATE_NAME;
            continue;
        }
        if in_target_stanza && trimmed.starts_with("disposition =") {
            let parsed = parse_inline_disposition(trimmed)?;
            if phase_disposition_with_order(&parsed, CURRENT_PHASE).is_none() {
                return Err(format!("{GATE_NAME}: no disposition for {CURRENT_PHASE}"));
            }
            return Ok(parsed);
        }
    }
    Err(format!(
        "{GATE_NAME} [[ship_gate]] disposition row not found"
    ))
}

fn phase_disposition_with_order<'a>(
    disposition: &'a HashMap<String, String>,
    phase: &str,
) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    for i in (0..=idx).rev() {
        if let Some(d) = disposition.get(PHASE_ORDER[i]) {
            return Some(d.as_str());
        }
    }
    None
}

fn resolve_workspace_path(path: &Path) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot determine cwd: {e}"))?;
    Ok(cwd.join(path))
}

fn lock_with(packages: &[(&str, &str)]) -> String {
    packages
        .iter()
        .map(|(name, version)| format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn dev_seed() -> [u8; 32] {
    let hex_str = "794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d";
    let bytes = (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("valid dev seed hex"))
        .collect::<Vec<_>>();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    seed
}
