#![forbid(unsafe_code)]

//! Story 12.1 — cohort manifest and full-pairwise mesh tripwire.

use std::process::Command;

const GATE_NAME: &str = "check-cohort-mesh";
const CURRENT_PHASE: &str = "v1_5";
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0", "v2_2"];

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

pub fn run(json: bool) -> Result<(), String> {
    assert_eq!(
        CURRENT_PHASE, "v1_5",
        "Story 12.1 must not advance global phase"
    );
    assert!(PHASE_ORDER.contains(&"v2_2"));
    // Every designated leg is phase-independent and hard-fails here, before
    // any ship-phase disposition can classify the aggregate as advisory.
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
    ];
    for leg in &legs {
        run_leg(leg)?;
    }
    if !crate::check_kernel_baseline::check()?.passed {
        return Err(format!("{GATE_NAME}: kernel-abi-diff RED"));
    }
    if json {
        println!(
            "{{\"gate\":\"{GATE_NAME}\",\"oracle_green\":true,\"current_phase\":\"{CURRENT_PHASE}\",\"legs\":{}}}",
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
