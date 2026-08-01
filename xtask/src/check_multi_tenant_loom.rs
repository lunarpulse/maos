#![forbid(unsafe_code)]

//! Stories 13.1–13.6d — physical multi-tenant Loom wall, authenticated write
//! crossing, and consent-governed production read crossing.
//!
//! Hermetic legs are [`BindingClass::Blocking`] at development HEAD. Live
//! Postgres legs are [`BindingClass::AdvisorySubstrate`]: absence emits a
//! WOULD-HAVE-BLOCKED banner; presence makes any RED result blocking.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

use crate::gate_common::{dev_enforced_red_blocks, emit_command, read_disposition, BindingClass};

const GATE_NAME: &str = "check-multi-tenant-loom";
/// Successors this gate declares ABSENT: a control this ADR names but that no
/// leg here can yet bind.
///
/// **Empty is a claim, not a default.** Story 13.6b inverted the write-side
/// dead-wire clause and Story 13.6d inverted the read-side clause; both now
/// bind at HEAD. Remaining controls owned elsewhere are deliberately not
/// laundered through this list:
///
///   * the kernel's collective-cause erasure (`Transport(_)` collapses all five
///     causes at `kernel-core/src/memory/mod.rs`) — a kernel-core edit plus a
///     FLAG-Winston conversation, owned by Story 13.6 to judge;
///   * the three-team journey closer — Story 13.6.
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
            name: "four-site-chokepoint",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "read_path_chokepoint",
                "team_guard_is_exactly_the_four_guarded_entry_points",
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
        // ── Story 13.6b — the crossing crosses, and the team that crossed it
        // is the team that signed it.
        //
        // `replication-crossing-has-no-production-initiator` lived here and was
        // INVERTED in 13.6b's commit. Per AC5 it is not merely deleted: the
        // clauses below replace it with a positive naming BOTH endpoints, the
        // D-6b hole closure, the D-5 one-store control, and the AC3 weld — each
        // a separately-named leg with its own inverter, never one composite
        // assertion (epic-13:175). One `#[test]` per `--exact` leg, because the
        // gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`.
        TestLeg {
            name: "crossing-has-production-initiator-both-endpoints",
            class: BindingClass::Blocking,
            // Inverter: delete the emitter arm in `run_cohort_a2a_daemon` or the
            // `apply_replication_bundle` call in `cross_team_crossing.rs` — the
            // 13.5g composition-root test.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_has_a_production_initiator_at_both_endpoints",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-scan-closes-originate-team-row-hole",
            class: BindingClass::Blocking,
            // Inverter: drop `originate_team_row(` from `CROSSING_NEEDLES` and
            // the fixture stops being caught — the exact D-6b blindness.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_scan_closes_the_originate_team_row_hole",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "exactly-one-production-loom-lite-store",
            class: BindingClass::Blocking,
            // Inverter: add a second production `LoomLiteStore::new` anywhere.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "exactly_one_production_loom_lite_store_construction",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-payload-team-must-equal-authenticated-envelope",
            class: BindingClass::Blocking,
            // AC3 / D-13 — its OWN leg and its OWN inverter, never folded into a
            // composite. Inverter: neuter the `payload_team != authenticated_team`
            // comparison in `cross_team_crossing.rs`.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_weld_refuses_a_forged_payload_team_before_apply",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-weld-is-a-binding-not-a-refuse-all-stub",
            class: BindingClass::Blocking,
            // Inverter: make the weld refuse unconditionally — the leg above
            // would still pass, this one reds.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_weld_admits_the_authenticated_team_and_proceeds_to_apply",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-applier-ignores-non-crossing-frames",
            class: BindingClass::Blocking,
            // Inverter: make the applier claim every frame — every other cohort
            // intent would start failing, and this leg names why.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_applier_ignores_frames_that_are_not_crossings",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-control-rides-the-telemetry-idiom",
            class: BindingClass::Blocking,
            // Inverter: add a `FramePayload`/`FrameKind` variant instead — this
            // leg is what keeps the null `abi-diff` control from mattering.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_control_round_trips_through_the_telemetry_idiom",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "boot-refuses-home-team-manifest-disagreement",
            class: BindingClass::Blocking,
            // AC4 — the correctness control against misconfiguration. Inverter:
            // downgrade clause (d) of `reconcile_transport_identity_with_manifest`
            // to a warning.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "boot_refuses_a_home_team_that_disagrees_with_the_signed_manifest",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "boot-refuses-uncorroborated-home-team",
            class: BindingClass::Blocking,
            // AC4 — absence never permits. Inverter: treat `team_of_host == None`
            // as a pass.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "boot_refuses_a_home_team_the_manifest_cannot_corroborate",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-weld-refusal-has-its-own-wire-code",
            class: BindingClass::Blocking,
            // AC3 — the refusal must not collapse into -32010 or -32012.
            // Inverter: reuse `CODE_TEAM_IDENTITY_MISMATCH` for the weld.
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--test",
                "crossing_wire_13_6b",
                "crossing_source_team_unbound_survives_the_wire_under_its_own_code",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-consent-denial-reaches-the-emitter",
            class: BindingClass::Blocking,
            // AC2 — the ordered pair + intent survive the socket. Inverter: drop
            // the `data` object from `crossing_refusal_nack`.
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--test",
                "crossing_wire_13_6b",
                "crossing_consent_denial_reaches_the_emitter_with_the_ordered_pair",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-causes-stay-distinguishable-on-the-wire",
            class: BindingClass::Blocking,
            // AC2 — denied / stale / unavailable must not collapse the way the
            // kernel collapses them (Residual 6). Inverter: map two refusal
            // variants onto one `reason` token.
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--test",
                "crossing_wire_13_6b",
                "crossing_denial_staleness_and_unavailability_stay_distinguishable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-applier-rejects-mismatched-frame-kind",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_applier_rejects_a_mismatched_frame_kind",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-binds-requested-destination-team",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_applier_binds_the_requested_destination_team",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-unconsented-applier-refusal",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "unconsented_crossing_is_refused_at_the_destination_applier",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-seedless-relabel-refusal",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "seedless_source_team_relabel_is_refused_at_the_destination_applier",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-rejects-unreadable-peer-config",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_request_rejects_non_utf8_peer_configuration",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-rejects-unreadable-namespace-config",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_request_rejects_non_utf8_namespace_configuration",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-rejects-empty-key",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "crossing_request_rejects_an_empty_key",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-unavailable-applier-fails-closed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--lib",
                "router::tests::verified_crossing_without_an_applier_fails_closed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-stale-gate-keeps-typed-outcome",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--lib",
                "router::tests::stale_crossing_gate_keeps_the_typed_stale_outcome",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "live-crossing-runs-through-two-daemons",
            class: BindingClass::AdvisorySubstrate,
            // The two-datname witness starts the real team-A and team-B daemon
            // processes, sends through route_outbound/prepare_outbound, and
            // observes the destination row after handle_intake_verified applies it.
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "cross_team_crossing_13_6b",
                "live_crossing_runs_through_two_daemon_processes",
                "--",
                "--ignored",
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
            name: "cross-wall-recall-no-consent-provider",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_without_injected_consent_fails_closed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-no-grant",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_no_grant_is_observable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-wrong-direction",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_wrong_direction_is_observable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-stale-state",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_stale_state_is_observable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-unavailable-state",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_unavailable_state_is_observable",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-read-port-unavailable",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_granted_without_read_port_fails_closed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-local-emitter-scope",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::recall_emitter_scope_only_returns_own_frames",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-ranged-recall-compile-pinned",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "researcher_recall_surface_cannot_import_unscoped_ranged_recall",
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
            name: "cross-wall-recall-filter-no-team-dimension",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "log_recall_filter_has_no_team_dimension",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-remote-artifact-read",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_wall_log_read_13_6d",
                "cross_wall_reader_returns_rows_from_the_named_bound_artifact",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-foreign-binding-refused",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_wall_log_read_13_6d",
                "cross_wall_reader_refuses_a_path_whose_artifact_binding_names_another_team",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-read-only-open",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_wall_log_read_13_6d",
                "cross_wall_reader_open_is_read_only_nofollow_and_non_migrating",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-remote-not-local",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_wall_log_read_13_6d",
                "consented_cross_wall_page_contains_remote_frames_and_no_local_frames",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-ordinary-foreign-open-refused",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_wall_log_read_13_6d",
                "ordinary_boot_path_still_refuses_a_foreign_bound_artifact",
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
            name: "cross-wall-recall-production-caller-live",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_has_production_caller_and_live_preconditions",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-signed-consent-live-path",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_live_path_uses_verified_state_and_home_team",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-request-parser",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--bin",
                "maos",
                "tests::story_13_6d_parses_validated_cross_wall_traceback_request",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-request-parser-refuses-invalid",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--bin",
                "maos",
                "tests::story_13_6d_rejects_invalid_or_incomplete_traceback_request",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-refusal-and-disclosure-journaled",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "cross_team_consent_13_3",
                "cross_wall_recall_refusals_and_disclosures_are_journaled",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-refusal-journal",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_journals_refusal",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cross-wall-recall-disclosure-before-read",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-iac",
                "--lib",
                "adapter::log_recall::tests::cross_wall_recall_journals_disclosure_before_remote_read",
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
        // ── Story 13.5a — enterprise governance at the cohort-a2a-daemon seam.
        //
        // A REAL control, not a null one. At HEAD the `EnterpriseRuntime` was
        // constructed at `main.rs` and never threaded into the daemon, so every
        // collective read the daemon served ran with no SSO principal, no PDP
        // mediation, no at-rest seal and no SIEM forward. Proven-red contract:
        // dropping the enterprise argument at the `cohort-a2a-daemon` dispatch
        // reds the source leg; dropping the governed decorator in
        // `build_cohort_a2a_daemon_runtime` reds both runtime legs (every
        // recording port falls to zero).
        //
        // All three are hermetic — an in-process daemon boot on 127.0.0.1:0 and
        // a `main.rs` source read. No Postgres, no live SSO/SIEM substrate, so
        // none of them is `AdvisorySubstrate`.
        TestLeg {
            name: "enterprise-governance-reaches-cohort-daemon",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--bin",
                "maos",
                "story_13_5a_enterprise_daemon_seam::story_13_5a_enterprise_governance_reaches_the_booted_cohort_daemon",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "enterprise-governance-daemon-dead-wire-negative",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--bin",
                "maos",
                "story_13_5a_enterprise_daemon_seam::story_13_5a_daemon_governance_is_dead_wired_unwired_and_fails_closed_when_denied",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "enterprise-governance-daemon-dispatch-threaded",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "enterprise_daemon_seam_13_5a",
                "story_13_5a_cohort_daemon_dispatch_threads_the_enterprise_runtime",
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
        // ── Story 13.6a — authenticated team identity (COHORT_SCHEMA_V4).
        //
        // Every leg is hermetic and therefore `Blocking`: the property under
        // test is answerable against synthetic frames, so nothing here needs a
        // Postgres substrate and nothing here is advisory. All five limbs were
        // proven-red by deletion (accept team block, send team stamp, both
        // `Defer if crossing` arms) with a byte-identical restore.
        TestLeg {
            name: "manifest-v4-additive-over-three-predecessors",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::v4_is_additive_over_all_three_frozen_predecessors",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "manifest-v4-golden",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::v4_canonical_body_matches_frozen_golden",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "manifest-v4-member-team-signature-bound",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::v4_member_team_is_signature_bound_and_fail_closed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "manifest-v4-member-team-negatives-typed",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::v4_member_team_rejections_are_typed",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "cohort-v4-to-v3-downgrade-refused-and-audited",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "state::tests::signed_v4_cache_refuses_higher_version_v3_reissue_and_audits_it",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "team-identity-impersonation-refused",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "impersonation_is_refused_at_the_accept_seam",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "team-identity-absence-refuses",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "crossing_without_a_verified_team_claim_is_refused",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "team-identity-emitter-self-check",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "emitter_refuses_a_crossing_it_cannot_speak_for",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-intent-not-reserved-both-seams",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "crossing_intent_is_not_reserved_so_both_seams_consult_the_gate",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-gate-wired-at-composition-root",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "production_composition_root_wires_a_real_cohort_gate",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "eviction-enforced-on-both-endpoints",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "eviction_is_enforced_on_both_endpoints_and_restoring_membership_recovers",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "stale-cache-refuses-crossing",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "stale_cache_refuses_the_crossing_on_both_seams",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "crossing-defer-refused-on-both-seams",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "derostered_crossing_is_refused_on_both_seams",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "bilateral-fallback-preserved-for-non-crossing-intents",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--features",
                "network",
                "--test",
                "team_identity_13_6a",
                "bilateral_fallback_survives_for_every_non_crossing_intent",
                "--",
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
