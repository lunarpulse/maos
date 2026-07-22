#![forbid(unsafe_code)]

//! Stories 13.1–13.5d — physical multi-tenant Loom wall and crossing gate.
//!
//! Hermetic legs are [`BindingClass::Blocking`] at development HEAD. Live
//! Postgres legs are [`BindingClass::AdvisorySubstrate`]: absence emits a
//! WOULD-HAVE-BLOCKED banner; presence makes any RED result blocking.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

use crate::gate_common::{dev_enforced_red_blocks, emit_command, read_disposition, BindingClass};

const GATE_NAME: &str = "check-multi-tenant-loom";
const ABSENT_SUCCESSORS: &[&str] = &[];

struct TestLeg {
    name: &'static str,
    class: BindingClass,
    args: &'static [&'static str],
}

#[derive(serde::Serialize)]
struct LegResult {
    name: &'static str,
    binding: &'static str,
    attempted: bool,
    substrate_present: bool,
    green: bool,
    detail: String,
}

impl LegResult {
    fn blocks(&self, class: BindingClass) -> bool {
        !self.green && dev_enforced_red_blocks(class, self.substrate_present)
    }
}

fn class_name(class: BindingClass) -> &'static str {
    match class {
        BindingClass::Blocking => "blocking",
        BindingClass::AdvisorySubstrate => "advisory-substrate",
    }
}

fn live_substrate_present() -> bool {
    ["MAOS_TEST_POSTGRES_TEAM_A", "MAOS_TEST_POSTGRES_TEAM_B"]
        .iter()
        .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
}

fn run_test_leg(leg: &TestLeg, substrate_present: bool) -> LegResult {
    if leg.class == BindingClass::AdvisorySubstrate && !substrate_present {
        return LegResult {
            name: leg.name,
            binding: class_name(leg.class),
            attempted: false,
            substrate_present: false,
            green: false,
            detail: "two-datname Postgres substrate absent".to_string(),
        };
    }

    let output = match Command::new("cargo").args(leg.args).output() {
        Ok(output) => output,
        Err(error) => {
            return LegResult {
                name: leg.name,
                binding: class_name(leg.class),
                attempted: true,
                substrate_present,
                green: false,
                detail: format!("could not start cargo: {error}"),
            };
        }
    };
    let transcript = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let non_vacuous = transcript.contains("running 1 test") && transcript.contains("1 passed");
    LegResult {
        name: leg.name,
        binding: class_name(leg.class),
        attempted: true,
        substrate_present,
        green: output.status.success() && non_vacuous,
        detail: if !output.status.success() {
            transcript
        } else if !non_vacuous {
            format!("vacuous: expected exactly one attempted passing test\n{transcript}")
        } else {
            "running 1 test; 1 passed".to_string()
        },
    }
}

fn write_step_summary(text: &str) {
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| writeln!(file, "{text}"));
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition(GATE_NAME)?;
    if !matches!(
        disposition.get("v2_2").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_2 disposition must be blocking"
        ));
    }

    let live_present = live_substrate_present();
    let specs = [
        TestLeg {
            name: "three-site-chokepoint",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "read_path_chokepoint",
                "team_guard_is_exactly_the_three_spirit_entry_points",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "tenant-map-hermetic-matrix",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "tenant_map_13_1",
                "tenant_map_13_1_gate_matrix",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "manifest-option-a-plus-matrix",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::tenant_manifest_option_a_plus_gate_matrix",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "two-datname-physical-absence",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "tenant_wall_two_datname_physical_absence_and_assignment_matrix",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "d1-forged-stamp-served-boundary",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "tenant_wall_d1_forged_stamp_is_still_served_boundary",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "forged-team-stamp-refused-at-verify",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::bundle::tests::test_forged_team_stamp_refused_at_verify_same_region",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "apply-refuses-forged-bundle",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::bundle::tests::test_apply_refuses_forged_bundle_writes_zero_rows",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "team-identity-source-reflex",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::bundle::tests::test_team_identity_source_reflex",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "per-team-merkle-independence",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "tenant_wall_per_team_merkle_independence_mixed_v1_v2",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // ── Story 13.3 — asymmetric cross-team consent + row attestation.
        TestLeg {
            name: "cross-team-crossing-lands-with-bound-source-team",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_region_live",
                "cross_team_crossing_lands_with_bound_source_team",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "asymmetric-consent-reverse-share-refused",
            class: BindingClass::AdvisorySubstrate,
            // Composition-level observer (13.3 review): the headline negative
            // is driven by the PRODUCTION manifest-backed consent adapter
            // over a signed V3 manifest, on two physical databases — never a
            // hard-coded consent stub.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "asymmetric_consent_reverse_share_refused",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-team-clobber-refused",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_region_live",
                "cross_team_clobber_refused",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "per-row-inclusion-verified-at-read-time",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_region_live",
                "per_row_inclusion_verified_at_read_time",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "foreign-team-row-without-attestation-refused-at-read",
            class: BindingClass::AdvisorySubstrate,
            // AC5(d) — registered at the 13.3 review: previously the test
            // existed but no leg invoked it, so the refusal could regress
            // silently (the test is #[ignore]-gated and runs nowhere else).
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_region_live",
                "unattested_cross_team_row_is_refused_at_read",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-team-apply-requires-claimed-pair-verifying-key",
            class: BindingClass::Blocking,
            // Party-mode D1 (13.3 review): apply must refuse a crossing whose
            // claimed (region, team) the destination could never serve.
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_team_apply_13_3",
                "apply_refuses_crossing_without_claimed_pair_verifying_key",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "tenant-consent-cause-taxonomy",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "adapter::tests::five_tenant_consent_causes_remain_distinguishable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "replication-crossing-has-no-production-initiator",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_consent_13_3",
                "replication_crossing_has_no_production_initiator",
                "--",
                "--exact",
            ],
        },
        // ── Story 13.3b — signed origin provenance + consented recall.
        TestLeg {
            name: "leaf-v3-preserves-v1-and-v2-goldens",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::leaf::tests::test_v3_provenance_is_additive_and_predecessors_stay_frozen",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "leaf-v3-golden",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::leaf::tests::test_v3_canonical_hash_golden",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "provenance-carries-across-two-stores",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "cross_region_live",
                "v3_provenance_crosses_team_wall_and_survives_rebundle",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // 13.3b rework — the cross-team provenance-laundering negative.
        // Hermetic and therefore Blocking: a hop team re-signing a
        // foreign-origin leaf under its own envelope must be REFUSED AT
        // BUILD, on both the team and the region axis. The refusal cannot
        // live at verify — once the origin is erased the bundle is
        // byte-indistinguishable from a genuine first-party one. The test
        // asserts the ORIGIN team and region by name, so a
        // refuse-everything stub cannot satisfy it.
        TestLeg {
            name: "leaf-origin-relabel-refused-at-build",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::bundle::tests::v2_builder_refuses_to_relabel_a_foreign_origin_leaf",
                "--",
                "--exact",
            ],
        },
        // The paired positive control: without it the leg above could be
        // satisfied by a builder that refuses everything.
        TestLeg {
            name: "first-party-promotion-still-permitted",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::bundle::tests::v2_builder_promotes_a_first_party_leaf_and_is_idempotent",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "leaf-v3-boundary-shift",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--lib",
                "replication::leaf::tests::test_source_team_v3_tail_boundary_shift_no_collision",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-refusal-distinguishable",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_has_five_distinguishable_outcomes",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-manifest-direction",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_manifest_direction_and_staleness_are_typed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "diamond-provenance-flattens",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::distillate::tests::diamond_dependency_is_not_a_cycle",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "diamond-true-cycle-still-rejected",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "distillation_i11_audit_chain",
                "cycle_detection",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-no-production-caller",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_has_no_production_caller",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-refusals-not-journaled",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_refusals_not_journaled",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "digest-frame-ref-codec",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-digest",
                "--lib",
                "tests::frame_ref_codec_accepts_compact_and_colon_grouped_formats",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "digest-clause-source-redaction",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-digest",
                "--lib",
                "tests::real_writer_accepts_owned_evidence_and_rejects_peer_private_evidence",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "digest-clause-source-secret-scrub",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::redaction::tests::clause_source_frame_refs_survive_without_exempting_other_hex_or_secrets",
                "--",
                "--exact",
            ],
        },
        // ── Story 13.5c — single composition root + bootable tenant mode.
        TestLeg {
            name: "cohort-daemon-boots-and-serves",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "cohort_daemon_boots_and_serves",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cohort-daemon-per-boot-nonce-single-sourced",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "daemon_boot_rows_prove_per_boot_nonce_variance",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "non-daemon-does-not-enable-tenant-map",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "non_daemon_process_with_config_refuses_unrefreshable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "tenant-mode-boots-live",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "tenant_mode_boots_on_live_substrate",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "collective-store-tenant-wall-live",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "spirit_collective_route_registered_pid_serves_only_own_team",
                "--",
                "--ignored",
                "--exact",
            ],
        },
    ];
    let mut legs: Vec<(BindingClass, LegResult)> = specs
        .iter()
        .map(|spec| {
            let substrate = spec.class == BindingClass::Blocking || live_present;
            (spec.class, run_test_leg(spec, substrate))
        })
        .collect();

    let kernel_report = crate::check_kernel_baseline::check()?;
    legs.push((
        BindingClass::Blocking,
        LegResult {
            name: "kernel-baseline-pinned",
            binding: class_name(BindingClass::Blocking),
            attempted: true,
            substrate_present: true,
            green: kernel_report.passed,
            detail: if kernel_report.passed {
                format!(
                    "kernel baseline actual=pinned={}",
                    kernel_report.actual_lines
                )
            } else {
                format!(
                    "kernel baseline mismatch: actual={}, pinned={}",
                    kernel_report.actual_lines, kernel_report.pinned_lines
                )
            },
        },
    ));

    let blockers: Vec<&LegResult> = legs
        .iter()
        .filter_map(|(class, leg)| leg.blocks(*class).then_some(leg))
        .collect();
    let skipped_live: Vec<&LegResult> = legs
        .iter()
        .map(|(_, leg)| leg)
        .filter(|leg| !leg.attempted)
        .collect();
    let oracle_green = legs.iter().all(|(_, leg)| leg.green);

    if !skipped_live.is_empty() {
        let banner = format!(
            "## ⚠️ Multi-Tenant Loom Gate: WOULD HAVE BLOCKED SHIP (v2.2)\n\
             Live two-datname Postgres substrate was absent; skipped: {}.\n\
             Hermetic legs still bind at HEAD. ABSENT successors: {}.",
            skipped_live
                .iter()
                .map(|leg| leg.name)
                .collect::<Vec<_>>()
                .join(", "),
            ABSENT_SUCCESSORS.join("; ")
        );
        emit_command(json, "warning", &banner.replace('\n', " "));
        write_step_summary(&banner);
    }

    if !blockers.is_empty() {
        let detail = blockers
            .iter()
            .map(|leg| format!("{}: {}", leg.name, leg.detail))
            .collect::<Vec<_>>()
            .join("\n");
        emit_command(json, "error", &format!("{GATE_NAME} RED: {detail}"));
        write_step_summary(&format!("## ❌ Multi-Tenant Loom Gate: RED\n{detail}"));
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": blockers.is_empty(),
                "oracle_green": oracle_green,
                "advisory": blockers.is_empty() && !oracle_green,
                "disposition": disposition,
                "legs": legs.iter().map(|(_, leg)| leg).collect::<Vec<_>>(),
                "absent_successors": ABSENT_SUCCESSORS,
            })
        );
    } else if blockers.is_empty() {
        println!(
            "{GATE_NAME}: PASSED ({}; {} absent successors declared)",
            if oracle_green {
                "oracle green"
            } else {
                "live substrate advisory"
            },
            ABSENT_SUCCESSORS.len()
        );
    }

    if blockers.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{GATE_NAME}: {} blocking leg(s) RED",
            blockers.len()
        ))
    }
}
