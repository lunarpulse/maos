#![forbid(unsafe_code)]

//! Stories 13.5d/13.5e — mediated production collective route plus the
//! physical per-team Transparency Log boundary.
//!
//! Hermetic route/audit legs are [`BindingClass::Blocking`] at development
//! HEAD. Live Postgres legs are [`BindingClass::AdvisorySubstrate`]: absence
//! emits a WOULD-HAVE-BLOCKED banner; presence makes any RED result blocking.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

use crate::gate_common::{dev_enforced_red_blocks, emit_command, read_disposition, BindingClass};

const GATE_NAME: &str = "check-reza-production-path";
const ABSENT_SUCCESSORS: &[&str] = &[
    "11.4b audit escape-anomaly detector wiring",
    "13.6 three-team product journey",
];

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
            name: "loom-scope-reaches-policy-table",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "xtask",
                "--test",
                "story_10_4a_ac1_proven_red",
                "story_13_5d_loom_scope_reaches_policy_table",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "route-not-spirit-reachable",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "researcher",
                "--lib",
                "unit_tests::collective_route_is_fail_closed_until_wired_then_reaches_port",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "production-collective-single-source",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "production_collective_calls_share_one_atomic_pid_binding",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "composition-root-does-not-seed-manifest-scopes",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cohort_daemon_smoke_13_5c",
                "composition_root_does_not_seed_manifest_scopes",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "mediated-operation-correlation",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "xtask",
                "--test",
                "story_10_4a_ac1_proven_red",
                "story_13_5d_request_route_row_audit_correlation",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (a): physical team addressing is deterministic and
        // cannot collapse two canonical teams onto one artifact.
        TestLeg {
            name: "tenant-audit-physical-team-paths",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "tests::team_transparency_log_paths_are_physically_distinct",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (b): backups carry team provenance, restore without
        // cross-team rows, and refuse a wrong-team destination.
        TestLeg {
            name: "tenant-audit-backup-restore-boundary",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cli",
                "backup::tests::tenant_backup_round_trip_rejects_wrong_team",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (c): a correlation requires both explicitly named
        // physical logs; one path only returns its local half.
        TestLeg {
            name: "tenant-audit-two-log-correlation",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "adapter::transparency_log::tests::cross_team_correlation_requires_both_physical_logs",
                "--",
                "--exact",
            ],
        },
        // Story 13-5f (13.5d correct-course): a real cross-team digest action
        // carries its request_id as the correlation_id on BOTH team logs, and a
        // NULL-correlation row cannot reconcile (AC3 end-to-end producer).
        TestLeg {
            name: "tenant-audit-correlation-producer-wired",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "audit::tests::digest_cross_team_action_correlates_via_request_id",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (e): the landed 13.3b directional consent seam must
        // return target-pid history without turning a refusal into empty data.
        TestLeg {
            name: "tenant-audit-cross-wall-historical-recall",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "adapter::log_recall::tests::cross_wall_recall_has_five_distinguishable_outcomes",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (f): traversal, Unicode/case collisions, symlink
        // aliases, partial env state, and renamed foreign artifacts refuse.
        TestLeg {
            name: "tenant-audit-adversarial-paths",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "tests::tenant_transparency_log_adversarial_paths_fail_closed",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e leg (g): SQLite tamper remains an explicit residual.
        // Escape anomaly wiring belongs to 11.4b; collective erase belongs to
        // 13.5b. This gate must not transmute absence into an integrity claim.
        TestLeg {
            name: "tenant-audit-integrity-residual",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "tests::tenant_audit_tamper_residual_is_not_misreported_as_integrity",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e (review): the ranged_recall Spirit filter must honour the
        // (boot_nonce, spirit_pid) identity pair — a pid reused by another
        // Spirit in a later boot must NOT leak into the named recall.
        TestLeg {
            name: "tenant-audit-ranged-recall-spirit-filter",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "log_composition::tests::spirit_filter_scopes_transparency_log_across_boots",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e (review): the kind-30 raw-SQL identity writer must not
        // mint a fresh unvalidated team shard (SQLITE_OPEN_CREATE removed).
        TestLeg {
            name: "tenant-audit-identity-writer-no-shard-mint",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "enterprise_identity::available_arm_tests::identity_asserted_does_not_mint_an_unvalidated_audit_shard",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e (review): the kind-31 run-capture raw-SQL writer must
        // fail closed against an absent TL rather than create one.
        TestLeg {
            name: "tenant-audit-run-capture-writer-fails-closed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cli",
                "subcommands::tests::journal_run_capture_row_fails_closed_without_a_tl",
                "--",
                "--exact",
            ],
        },
        // Story 13.5e (review, D3): a tenanted backup restored onto another
        // team's shard path is refused BEFORE any bytes are copied.
        TestLeg {
            name: "tenant-audit-restore-refuses-cross-team",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cli",
                "backup::tests::validate_restore_target_team_refuses_cross_team_planting",
                "--",
                "--exact",
            ],
        },
        // Story 13.5b: backend partition must account for every registered
        // erasure backend exactly once.
        TestLeg {
            name: "gdpr-backend-partition",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "--test",
                "multi_backend_erasure_test",
                "multi_backend_erasure_partition_invariant",
                "--",
                "--exact",
            ],
        },
        // 13.5b review: this leg replays the KERNEL forget outcomes from the
        // deterministic corpus. It calls `memory.forget_with_reason` directly
        // and never constructs an `UninstallCascadeTerminal`, so it is NOT the
        // four-terminal control its old name claimed. Named for what it proves;
        // the operator-facing terminals are bound by the two legs below.
        TestLeg {
            name: "gdpr-kernel-forget-outcome-corpus",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-audit",
                "--test",
                "gdpr_cascade_corpus_test",
                "gdpr_cascade_v0_corpus_replay",
                "--",
                "--exact",
            ],
        },
        // AC5's real operator vocabulary: the daemon terminal JSON and its exit
        // codes. Previously bound by nothing (13.5b review, null control).
        TestLeg {
            name: "gdpr-uninstall-terminal-erased",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "erased_uninstall_is_success_with_machine_receipt",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-uninstall-terminal-held",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "held_uninstall_is_non_success_and_writes_no_complete_proof",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-uninstall-terminal-not-found",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "not_found_uninstall_has_distinct_terminal_code",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-uninstall-terminal-failed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "proof_write_failure_has_failed_terminal_code",
                "--",
                "--exact",
            ],
        },
        // D1 (review consensus): a region-pinned Host erases and says `erased`,
        // with the Shared tier carried as an explicit CoverageGap. Restoring
        // `"shared"` to REQUIRED_STORES reds this leg.
        TestLeg {
            name: "gdpr-regional-shared-coverage-gap",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "regional_uninstall_emits_erased_terminal_with_shared_coverage_gap",
                "--",
                "--exact",
            ],
        },
        // D2 (review consensus): a mixed erased+held run must attest what it
        // destroyed. Reverting to a bare `held` terminal reds this leg.
        TestLeg {
            name: "gdpr-mixed-held-partial-proof",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "mixed_held_uninstall_writes_partial_proof",
                "--",
                "--exact",
            ],
        },
        // The private tier's Markdown residue survives the cascade (13.5b
        // review defect pin). This leg keeps the known hole visible; it goes
        // RED when a successor fixes it, forcing the proof category to follow.
        TestLeg {
            name: "gdpr-private-markdown-residue-pinned",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "private_tier_markdown_survives_the_forget_cascade",
                "--",
                "--exact",
            ],
        },
        // The operator wrapper must forward every terminal exit code verbatim.
        TestLeg {
            name: "gdpr-maosctl-forwards-terminal-exit-codes",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cli",
                "--test",
                "uninstall_exit_codes_13_5b",
                "uninstall_forwards_erased_held_not_found_and_failed_codes",
                "--",
                "--exact",
            ],
        },
        // An independently opened team shard must be indeterminate, never
        // answer "no hold" from a local empty table.
        TestLeg {
            name: "gdpr-legal-hold-fail-closed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--test",
                "legal_hold_authority_13_5b",
                "independently_opened_team_shard_cannot_answer_hold_absent",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-operator-erase-zero-spirit-reach",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "collective_erase_has_one_operator_route_and_zero_spirit_reach",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-one-sided-erase-reconciliation",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "xtask",
                "--test",
                "story_10_4a_ac1_proven_red",
                "story_13_5b_one_sided_collective_erase_is_red",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-collective-partition-live",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "collective_principal_partition_refuses_write_and_replication_apply",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-collective-erase-live",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "collective_erase_moves_merkle_triple_and_blocks_stale_replication",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "spirit-route-and-tenant-audit-stage2-refusal-live",
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
    ];
    let legs: Vec<(BindingClass, LegResult)> = specs
        .iter()
        .map(|spec| {
            let substrate = spec.class == BindingClass::Blocking || live_present;
            (spec.class, run_test_leg(spec, substrate))
        })
        .collect();

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
            "## ⚠️ Reza Production Path Gate: WOULD HAVE BLOCKED SHIP (v2.2)\n\
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
        write_step_summary(&format!("## ❌ Reza Production Path Gate: RED\n{detail}"));
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
