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
        // ── Story 13.5g — in-artifact tenant binding (defense-in-depth Stage-2).
        // The pure AC3 verdict table + AC4 datname cases are hermetic Blocking
        // legs (one #[test] per limb — the anti-vacuity check greps
        // `running 1 test`/`1 passed`); the Phase A wiring legs prove the
        // read-only orchestration (D-3/D-4), and the Phase B wiring leg carries
        // the live Postgres `current_database()` substrate (AdvisorySubstrate).
        TestLeg {
            name: "tl-tenant-binding-round-trip",
            class: BindingClass::Blocking,
            args: &["test", "-p", "maos-audit", "tests::tl_tenant_binding_round_trip", "--", "--exact"],
        },
        TestLeg {
            name: "tl-tenant-binding-read-only-missing-none",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_tenant_binding_read_is_read_only_and_missing_reads_none", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-bound-match-proceeds",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_a_verdict_bound_match_proceeds",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-bound-foreign-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_a_verdict_bound_foreign_refuses",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-corrupt-binding-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_a_verdict_corrupt_binding_refuses",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-fresh-needs-write",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_a_verdict_fresh_artifact_needs_write",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-legacy-sidecar-migrates",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_a_verdict_legacy_sidecar_migrates",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-verdict-foreign-history-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_phase_a_verdict_foreign_history_without_sidecar_refuses", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-b-datname-none-records",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_b_datname_none_records", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-b-datname-match-proceeds",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_b_datname_match_proceeds", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-b-datname-drift-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit", "tests::tl_phase_b_datname_drift_refuses", "--", "--exact",
            ],
        },
        // Phase A wiring (D-3/D-4): a foreign shard with history and no sidecar
        // is refused by a read-only preflight that does NOT mutate the artifact.
        TestLeg {
            name: "tl-phase-a-refuses-foreign-shard-before-append",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "phase_a_refuses_foreign_shard_with_history_before_append", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-refuses-artifact-bound-to-foreign-team",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "phase_a_refuses_artifact_bound_to_foreign_team", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-legacy-sidecar-migrates-wiring",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "phase_a_legacy_artifact_with_matching_sidecar_migrates", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-phase-a-needs-write-then-proceeds-wiring",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "phase_a_needs_write_then_proceeds_after_binding_written", "--", "--exact",
            ],
        },
        // Phase B live substrate: persisted datname vs live current_database().
        TestLeg {
            name: "tl-phase-b-persisted-datname-vs-live-current-database",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "phase_b_persisted_datname_vs_live_current_database", "--", "--ignored", "--exact",
            ],
        },
        // ── Story 13.5g code-review repairs (2026-07-27). Each leg below is a
        // control that did not exist when the story was first gated, and each
        // reds on its own mutation.
        TestLeg {
            name: "tl-phase-a-verdict-whitespace-binding-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_phase_a_verdict_whitespace_binding_refuses", "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-tenant-binding-read-fails-closed",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_tenant_binding_read_fails_closed_on_unreadable_artifact",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-tenant-binding-write-refuses-foreign-overwrite",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_tenant_binding_write_refuses_foreign_binding_overwrite",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-tenant-binding-refuses-symlinked-artifact",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-audit",
                "tests::tl_tenant_binding_refuses_symlinked_artifact", "--", "--exact",
            ],
        },
        // Composition-root ordering: the legs above call `phase_a_preflight`
        // directly and stay green if the Phase A block is deleted from
        // `main.rs`. These two boot the shipped binary.
        TestLeg {
            name: "tl-boot-refuses-foreign-shard-before-open",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "boot_refuses_foreign_shard_before_opening_the_transparency_log",
                "--", "--exact",
            ],
        },
        TestLeg {
            name: "tl-boot-writes-binding-after-open",
            class: BindingClass::Blocking,
            args: &[
                "test", "-p", "maos-bin", "--test", "tenant_audit_phase_a_13_5g",
                "boot_writes_binding_after_open_for_a_legacy_shard", "--", "--exact",
            ],
        },
        // Story 13.5b: backend partition must account for every registered
        // erasure backend exactly once.
        // Story 13.5h: this leg now also carries the Shared-tier refusal proof.
        // The discharge used to plant its canary under `Coordination`, so it
        // passed identically with and without principal PII in the tier; it now
        // asserts the typed `NamespaceViolation` at the Shared write/read/scan
        // entry points and scans for zero principal-namespaced rows. Deleting
        // any one of the three hoisted `reject_principal_outside_private` calls
        // reds this leg at that specific arm.
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
        // D1 (review consensus), re-pointed by Story 13.5h: a region-pinned Host
        // erases and says `erased`, with the Shared tier now attested
        // `VerifiedEmpty` — earned by counting principal-namespaced rows, not
        // asserted. Removing `"shared"` from REQUIRED_STORES reds this leg.
        TestLeg {
            name: "gdpr-regional-shared-verified-empty",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "regional_uninstall_attests_shared_tier_verified_empty",
                "--",
                "--exact",
            ],
        },
        // Story 13.5h Trap 4: the partition makes pre-existing Shared principal
        // rows unreachable, NOT erased. This is the leg that makes the sibling
        // above non-vacuous — hard-code `VerifiedEmpty` instead of counting and
        // this leg reds while the sibling stays green. Hermetic (TempDir +
        // SQLite), hence Blocking.
        TestLeg {
            name: "gdpr-shared-pre-partition-residue-fail-closed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "regional_uninstall_refuses_to_attest_pre_partition_shared_residue",
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
        // Story 13.5i: the real one-shot uninstall must delete durable private
        // bytes, carry the effect into the signed proof, and leave no residue.
        TestLeg {
            name: "gdpr-private-filesystem-erasure-subprocess",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "erasure_uninstall_13_5b",
                "private_tier_markdown_is_erased_by_the_forget_cascade",
                "--",
                "--exact",
            ],
        },
        // Restart-backed anti-vacuity siblings. Each exact leg owns one
        // assertion surface so `run_test_leg` observes one attempted test.
        TestLeg {
            name: "gdpr-private-restart-markdown-erasure",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "restart_forget_erases_markdown_content",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-restart-spill-erasure",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "restart_forget_erases_non_markdown_spill_content",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-proof-count",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "restart_forget_reports_distinct_persisted_entry_count",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-exact-once-count",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_counts_cached_and_spilled_value_exactly_once",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-bystander-retention",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_preserves_bystander_and_default_namespace_content",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-symlink-containment",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_does_not_follow_pid_directory_symlink",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-fail-closed-io",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_fails_closed_when_pid_directory_is_unreadable",
                "--",
                "--exact",
            ],
        },
        // 13.5i code review. M8 as specified (per-file counting) had no
        // CI-executed detector; namespace-level symlinks were followed and
        // counted while `remove_dir_all` unlinked only the link; and
        // `file_stem()` attested sub-directories and editor backups as keys.
        TestLeg {
            name: "gdpr-private-inline-only-count",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_counts_inline_only_entries_that_never_spill",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-namespace-symlink-containment",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_does_not_follow_namespace_directory_symlink",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "gdpr-private-logical-key-identity",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_forget_restart_13_5i",
                "forget_counts_logical_keys_not_filesystem_nodes",
                "--",
                "--exact",
            ],
        },
        // Story 13.5j: 13.5i made `forget_principal` authoritative about the
        // filesystem; these legs hold `write`, `read` and `scan` to the same
        // account. Each exact leg owns one assertion surface so `run_test_leg`
        // observes one attempted test.
        TestLeg {
            name: "private-spill-supersession-kind-change",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "write_unlinks_the_superseded_spill_when_the_kind_changes",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-spill-supersession-shrink",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "write_unlinks_the_spill_when_the_value_shrinks_below_the_threshold",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-scan-logical-key-merge",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "scan_returns_one_entry_for_a_key_held_in_cache_and_on_disk",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-scan-read-cardinality",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "a_read_does_not_change_scan_cardinality",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-scan-empty-key-recovery",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "scan_recovers_the_empty_key_from_its_spill_name",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-scan-junk-node-skip",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "scan_skips_a_directory_that_looks_like_a_spill",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-scan-namespace-symlink-containment",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "scan_does_not_follow_a_namespace_directory_symlink",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-read-spill-symlink-containment",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "read_does_not_follow_a_spill_symlink",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-forget-legacy-residue-count",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "forget_counts_a_pre_existing_superseded_spill_once",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "private-signed-frame-digest-dedup",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "private_spill_supersession_13_5j",
                "digest_refs_are_not_duplicated_by_a_spilled_working_memory_entry",
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
