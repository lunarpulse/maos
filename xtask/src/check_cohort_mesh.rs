#![forbid(unsafe_code)]

//! Story 12.1 — cohort manifest and full-pairwise mesh tripwire.

use crate::gate_common::{dev_enforced_red_blocks, emit_command, BindingClass, CURRENT_PHASE};
use std::process::Command;

const GATE_NAME: &str = "check-cohort-mesh";

/// Dev-time enforcement class for this gate's legs.
///
/// Story 12.1 hand-rolled a phase-independent hard-fail carve-out here because
/// keying enforcement off the ship ladder made a RED leg advisory;
/// `gate_common`'s [`BindingClass`] IS that carve-out, promoted to a shared
/// home. Adopting it closes the `check-cohort-mesh` half of **D20** — the
/// ship ladder now governs GA disposition only, and `CURRENT_PHASE` is read
/// from exactly one place (`gate_common`).
const BINDING: BindingClass = BindingClass::Blocking;

struct Leg {
    name: &'static str,
    args: &'static [&'static str],
}

fn run_leg(leg: &Leg) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(leg.args)
        .output()
        .map_err(|error| format!("{GATE_NAME}: could not start {}: {error}", leg.name))?;
    let transcript = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        return Err(format!("{GATE_NAME}: {} RED\n{transcript}", leg.name));
    }
    if !transcript.contains("running 1 test") || !transcript.contains("1 passed") {
        return Err(format!(
            "{GATE_NAME}: {} vacuous — expected exactly one attempted passing test\n{transcript}",
            leg.name
        ));
    }
    Ok(())
}

fn build_journey_daemon() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["build", "-p", "maos-bin"])
        .output()
        .map_err(|error| format!("{GATE_NAME}: could not build J3 daemon: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{GATE_NAME}: J3 daemon build RED\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn run(json: bool) -> Result<(), String> {
    // Every designated leg is phase-independent and hard-fails under
    // [`BINDING`], before any ship-phase disposition can classify the aggregate
    // as advisory. Story 15-3 F4 deleted the two asserts that stood here: they
    // compared a LOCAL const to its own literal (a tautology that proved
    // nothing), and repointing them at the shared `CURRENT_PHASE` would have
    // turned a sanctioned phase advance into a runtime panic — a tripwire
    // against the operation it is meant to permit, in the very crate that
    // consolidates the phase source.
    build_journey_daemon()?;
    let legs = [
        Leg {
            name: "unpinned-authority",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "state::tests::wrong_signer_is_journaled_with_actual_versions",
                "--",
                "--exact",
            ],
        },
        Leg {
            name: "concurrent-fork",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "state::tests::same_verified_body_confirms_but_divergent_body_forks",
                "--",
                "--exact",
            ],
        },
        Leg {
            name: "version-regression",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "state::tests::lower_verified_version_is_a_version_regression",
                "--",
                "--exact",
            ],
        },
        Leg {
            name: "n3-real-mesh",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_1_cohort_mesh",
                "t_12_1_n3_mesh_smoke",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        Leg {
            name: "n8-full-pairwise",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_1_cohort_mesh",
                "t_12_1_n8_full_pairwise_mesh_measurement",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        Leg {
            name: "stale-pull-push-resubmit",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_1_cohort_mesh",
                "t_12_1_stale_pull_push_resubmit_real_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 14-2b convergence-observability legs. Enrolled HERE and not in
        // a new gate because 14-2a's `check_cert_rotation_trigger.rs` cost 945
        // raw lines against this story's +35 xtask headroom, and NOT in
        // `check-cert-rotation-trigger` because its `derive_rotation_tests_in`
        // hard-fails on any derived test its fixed `TEST_FILES` list does not
        // name as a leg.
        //
        // Both legs are real three-host mTLS sweeps whose assertions an EMPTY
        // table cannot satisfy: one tells two peers at different versions
        // apart, the other invalidates exactly one of two live records.
        Leg {
            name: "convergence-versions-told-apart",
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "t_14_2b_cohort_convergence",
                "t_14_2b_peers_at_different_versions_are_told_apart",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        Leg {
            name: "convergence-record-invalidation",
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "t_14_2b_cohort_convergence",
                "t_14_2b_a_restarted_peer_is_not_a_convergence_claim",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 14-2c: a local-leaf reissue must remain diagnosable through the
        // authenticated operator HTTP body after the mesh path is severed.
        Leg {
            name: "local-leaf-declaration-diagnosis",
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "t_14_2c_local_leaf_declaration",
                "t_14_2c_local_leaf_reissue_is_diagnosable",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 14-2c review: the agree verdict and the pull-health carrier
        // must be proven on live pulls through the operator HTTP body.
        Leg {
            name: "self-identity-agreement-pull-health",
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "t_14_2c_local_leaf_declaration",
                "t_14_2c_self_identity_agreement_and_pull_health",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 12.2 consent legs — each hard-fails independently (loop below).
        // §A7 role-identity reflex: the admitted acting role is the manifest-
        // bound-to-peer AND frame-carried role; a relabel reds via the real NACK.
        Leg {
            name: "role-mismatch-on-allowed-peer",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_role_mismatch_on_allowed_peer_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 role-identity reflex: positive + relabeled-negative prove no any-role OR.
        Leg {
            name: "acting-role-exact-match",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_acting_role_exact_match_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 entitlement reflex (parse arm): the tightened per-peer validator
        // rejects a role a different member declares.
        Leg {
            name: "entitlement-parse",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::reject_consent_role_declared_only_by_different_peer",
                "--",
                "--exact",
            ],
        },
        // §A7 entitlement reflex (accept arm): an unheld acting role is refused
        // distinctly from "no grant".
        Leg {
            name: "entitlement-accept",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_entitlement_accept_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 skew-cause reflex: asserts ECohortManifestSkew specifically, paired
        // with a |Δv| ≤ 1 admit.
        Leg {
            name: "manifest-skew-cause",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_manifest_skew_cause_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 fail-closed reflex: None acting role AND None version each refused.
        Leg {
            name: "fail-closed-none",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_fail_closed_none_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 placement reflex: a reserved-intent frame with no role/version still
        // ACKs — the role gate sits after the reserved short-circuit (P4).
        Leg {
            name: "reserved-intent-passes",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_2_cohort_consent",
                "t_12_2_reserved_intent_without_role_or_version_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 12.3 halt-receipt legs — each hard-fails independently (loop below).
        // §A7 absence-first-class reflex (P2a/P3): a dropped member probes to Io →
        // ABSENT(MemberLoss), paired with a PRESENT positive so up/down is proven
        // distinguished — NOT PartitionTimeout (dead on the wire).
        Leg {
            name: "halt-receipt-absence-member-loss",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_3_cohort_halt_receipt",
                "t_12_3_absence_member_loss_over_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 absence-first-class reflex (P2a/P3): a partitioned member times out →
        // ABSENT(ConnectivityLoss), a DISTINCT variant, paired with a PRESENT
        // positive — NOT a bare TransportFailed (an up peer's unknown NACK).
        Leg {
            name: "halt-receipt-absence-connectivity-loss",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_3_cohort_halt_receipt",
                "t_12_3_absence_connectivity_loss_over_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 replay-dedup reflex (P4): the SAME receipt shipped twice keeps the
        // presence count at 1 — keyed on HaltReceipt.halt_id, NOT the per-ship
        // envelope frame_id; a distinct receipt raises it (non-vacuous).
        Leg {
            name: "halt-receipt-replay-dedup",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_3_cohort_halt_receipt",
                "t_12_3_replay_dedup_over_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 source-identity reflex (P5r): a receipt via the unverified
        // handle_intake / loopback path, or from ≠ TLS peer, is NOT counted —
        // only the TLS-anchored, matching-`from` receipt increments presence.
        Leg {
            name: "halt-source-identity",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_3_cohort_halt_receipt",
                "t_12_3_source_identity_over_core",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 wiring reflex (P7c, anti-silent-green): an injected RECORDING
        // observer records a shipped cohort:halt-receipt through the real
        // bind → handle_intake_verified → observer path; a mis-wired/inert
        // observer reds it.
        Leg {
            name: "halt-receipt-observer-wired",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_3_cohort_halt_receipt",
                "t_12_3_observer_wired_over_live_tcp",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 observability-not-arbitration reflex (P6/P5r): the AC3 static
        // chokepoint — a single observer call site, no arbitration sink named
        // (graph-guaranteed by cohort ↛ kernel-core).
        Leg {
            name: "observer-no-arbitration-chokepoint",
            args: &[
                "test",
                "-p",
                "maos-a2a-core",
                "--test",
                "cohort_halt_receipt_chokepoint_12_3",
                "halt_receipt_observed_at_one_site_with_no_arbitration_sink",
                "--",
                "--exact",
            ],
        },
        // ── Story 12.4a — no-surveillance MECHANISM legs (live N=8 TCP) ──────
        // §A7 no-surveillance reflex: a consent-matrix cohort:digest-read is
        // admitted through the target's accept-gate and the correlated reply
        // lands, tagged with the request's request_id (a data-return without
        // an admit, or a lost reply, reds).
        Leg {
            name: "digest-read-consented-admitted",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_4a_digest_read",
                "t_12_4a_consented_read_admitted_reply",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 no-surveillance reflex: an out-of-matrix read is refused at the
        // COHORT accept-overlay (a real Deny, not the coarse allowlist), no data
        // returned, and the refusal is a genuine ConsentRupture bound to the
        // denier=target (an invisible refusal or a data-return reds).
        Leg {
            name: "digest-read-surveillance-negative",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_4a_digest_read",
                "t_12_4a_surveillance_negative_denied_and_visible",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 sink-is-live reflex (P7c, anti-silent-green): the F4 fix — a
        // refusal's rupture lands in a RECORDING target journal AND is returned
        // by a --frame-kind ConsentRupture query; a stubbed/no-op sink journals
        // nothing and reds it ("wired" ≠ "the member can query the row").
        Leg {
            name: "digest-read-rupture-sink-wired",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_4a_digest_read",
                "t_12_4a_rupture_sink_wired_live_journal",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 anti-canned reflex: a re-signed manifest with the digest-read
        // accept grant REMOVED flips admit→deny through the gate's own
        // comparator (a static verdict reds).
        Leg {
            name: "digest-read-anti-canned-resign",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_4a_digest_read",
                "t_12_4a_anti_canned_resign_flips_verdict",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 replay-dedup reflex: the SAME request_id reply shipped twice is
        // counted once (a distinct request_id counts again); dedup on the
        // resetting envelope frame_id is the 12.3 P4 silent no-op and reds.
        Leg {
            name: "digest-read-replay-dedup",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_4a_digest_read",
                "t_12_4a_replay_dedup_reply_idempotent",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // Story 12.4b §A7 day-30 journey reflex: the production daemon loads
        // the captured raw dataset, journals Digest-owned ingestion frames,
        // derives and persists the I11 distillate, and moves the real vt100
        // screen. A missing render/persist seam reds this exact Grade-A test.
        Leg {
            name: "j3-day-30-scene",
            args: &[
                "test",
                "-p",
                "maos-journey-test",
                "--test",
                "journey_j3",
                "j3_day_30_digest_renders_and_persists_via_real_daemon",
                "--",
                "--exact",
            ],
        },
        // Story 12.4b §A7 anti-canned reflex: blind exactly one byte in a
        // captured summary count, derive through the one shared function, and
        // require the vt100 line to move. Static prose or stale pre-render reds.
        Leg {
            name: "j3-anti-canned-raw-byte",
            args: &[
                "test",
                "-p",
                "maos-journey-test",
                "--test",
                "journey_j3",
                "j3_anti_canned_blinded_raw_input_moves_rendered_line",
                "--",
                "--exact",
            ],
        },
        // ── Story 12.5 — cohort hot-swap + linear-chain migration legs ──
        // §A7 wildcard-fork reflex: declared-string uniqueness is insufficient;
        // exact 1.0 plus wildcard 1.x must reject concrete source 1.0.
        Leg {
            name: "migration-chain-wildcard-fork",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--test",
                "migration_chain",
                "rejects_wildcard_overlap_as_a_fork_at_the_concrete_source",
                "--",
                "--exact",
            ],
        },
        // §A7 cycle reflex: one outgoing candidate per source is still invalid
        // when the chain cannot terminate.
        Leg {
            name: "migration-chain-cycle",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--test",
                "migration_chain",
                "rejects_a_two_cycle_even_when_every_source_has_one_outgoing_candidate",
                "--",
                "--exact",
            ],
        },
        // §A7 self-loop reflex: the length-one cycle has its own precise error.
        Leg {
            name: "migration-chain-self-loop",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--test",
                "migration_chain",
                "rejects_a_self_loop_before_attempting_a_walk",
                "--",
                "--exact",
            ],
        },
        // §A7 fan-in negative control: multiple predecessor patterns on one
        // successor stay legal; only a disconnected requested target is no-path.
        Leg {
            name: "migration-chain-fanin-no-path",
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--test",
                "migration_chain",
                "permits_fan_in_and_reports_no_path_only_for_a_well_formed_set",
                "--",
                "--exact",
            ],
        },
        // §A7 missing-hop reflex: distinct from the plan-time no-path miss, the
        // kernel refuses a RESOLVED hop whose migrator is absent at run time,
        // naming the specific predecessor -> successor hop (AC4(b)).
        Leg {
            name: "migration-chain-missing-hop-at-runtime",
            args: &[
                "test",
                "-p",
                "maos-kernel-core",
                "--test",
                "hot_swap_cross_major_migration",
                "run_migrator_names_the_specific_absent_hop_at_run_time",
                "--",
                "--exact",
            ],
        },
        // §A7 plan-drift reflex: the refusal comes from a REAL hash comparison of
        // the persisted plan FILE vs the chain re-derived from the live candidate
        // manifests on disk (not an in-memory-only comparison) — a candidate
        // mutated after --plan reds EMigrationPlanDrift.
        Leg {
            name: "migration-plan-drift",
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--bin",
                "maos",
                "migration_plan::tests::persisted_plan_file_drift_is_refused_after_a_candidate_mutates_on_disk",
                "--",
                "--exact",
            ],
        },
        // §A7 cert-rotation reflex: the rebuilt N=8 transport rejects an old
        // credential, admits the new one through TOFU plus the cohort gate, and
        // reconciles the signed reissue fingerprint with the active pin.
        Leg {
            name: "n8-cert-rotation-repin-coherence",
            args: &[
                "test",
                "-p",
                "maos-a2a-tcp",
                "--test",
                "t_12_5_cohort_hot_swap",
                "t_12_5_n8_rotation_refuses_old_cert_and_admits_reissued_new_cert",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        // §A7 halt-continuity reflex: blinding a durable receipt changes the
        // derived surviving count; the drained pending registry is not used.
        Leg {
            name: "halt-receipt-derived-continuity",
            args: &[
                "test",
                "-p",
                "maos-journey-test",
                "--test",
                "journey_j3",
                "j3_blinded_halt_receipt_moves_persistent_agents_halted_count",
                "--",
                "--exact",
            ],
        },
    ];
    for leg in &legs {
        if let Err(red) = run_leg(leg) {
            if dev_enforced_red_blocks(BINDING, true) {
                return Err(red);
            }
            // Unreachable while [`BINDING`] is `Blocking`, and deliberately not
            // an `unreachable!()`: if the class is ever downgraded, a RED leg
            // must still surface as a banner rather than a silent green.
            emit_command(
                json,
                "warning",
                &format!("{GATE_NAME}: WOULD-HAVE-BLOCKED — {red}"),
            );
        }
    }
    // Story 16-0 / AC5: emit this gate's OWN report BEFORE propagating the red.
    // `Ok(passed: false)` does not abort at the `?` — the bare `return Err` on
    // the next line did, and a policy red that produces zero bytes of `--json`
    // looks like a crash (`deferred-work.md:637`). The kernel report's detail is
    // forwarded too: it NAMES the files that moved, and "kernel-abi-diff RED"
    // alone does not.
    let kernel = match crate::check_kernel_baseline::check() {
        Ok(report) => report,
        Err(error) => {
            // The Err class (missing `[kernel_src]`, hand-edited pin, walk
            // failure) must not abort this gate before its own JSON exists
            // either — zero-byte `--json` is the `deferred-work.md:637`
            // crash-shape (Story 16-0 review).
            if json {
                println!(
                    "{{\"gate\":\"{GATE_NAME}\",\"oracle_green\":false,\"ship_phase\":\"{CURRENT_PHASE}\",\"legs\":{},\"error\":true}}",
                    legs.len() + 1
                );
            }
            return Err(format!("{GATE_NAME}: kernel-abi-diff ERRORED — {error}"));
        }
    };
    if !kernel.passed {
        if json {
            println!(
                "{{\"gate\":\"{GATE_NAME}\",\"oracle_green\":false,\"ship_phase\":\"{CURRENT_PHASE}\",\"legs\":{}}}",
                legs.len() + 1
            );
        }
        return Err(format!(
            "{GATE_NAME}: kernel-abi-diff RED — {}",
            crate::check_kernel_baseline::failure_detail(&kernel)
        ));
    }
    if json {
        println!(
            "{{\"gate\":\"{GATE_NAME}\",\"oracle_green\":true,\"ship_phase\":\"{CURRENT_PHASE}\",\"legs\":{}}}",
            legs.len() + 1
        );
    } else {
        println!(
            "{GATE_NAME}: PASSED ({} independent hard-fail legs)",
            legs.len() + 1
        );
    }
    Ok(())
}
