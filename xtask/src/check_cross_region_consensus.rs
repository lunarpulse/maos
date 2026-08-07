#![forbid(unsafe_code)]

//! Story 11.2a (AC5, D10) — `check-cross-region-consensus` gate.
//!
//! Cross-region convergent replication gate with five oracle legs. This is THE
//! authoritative row for the ADR-049 cross-region convergent replication
//! binding (sign-only Ed25519 re-attestation + Merkle convergence oracle +
//! region-identity reflex + AP-degrade partition + kernel-ABI baseline).
//!
//! # The five legs
//!
//! 1. **reattestation-mediated** — live Postgres: a `CrossRegionReadmit`
//!    write drives a real Ed25519 re-attestation through the mediator bundle.
//! 2. **convergence-oracle** — live Postgres: the KV-payload oracle + Merkle
//!    root converge across two regions.
//! 3. **region-identity** — live Postgres: the region-identity reflex rejects
//!    a foreign-region source log ref.
//! 4. **ap-degrade** — live Postgres: a severed transport forces the AP-degrade
//!    router to a deterministic degraded path.
//! 5. **kernel-abi-diff** — the `check-kernel-baseline` re-pin is GREEN (the
//!    `WriteEntryPoint::CrossRegionReadmit` addition did not drift the kernel).
//!
//! # Live-oracle posture (D5 anti-canned)
//!
//! Legs 1–4 run as `cargo test -p maos-loom-lite --test cross_region_live
//! -- --ignored --nocapture`, gated on the `MAOS_TEST_POSTGRES` connection
//! string. An environment WITHOUT Postgres reports those legs as `ABSENT` —
//! never a silent pass.
//!
//! # Story 13.6e — leg-level binding, and the ledger
//!
//! This gate used to key its whole verdict off a private `CURRENT_PHASE =
//! "v1_5"` const plus a registry `advisory` row, so a RED LIVE leg returned
//! `Ok(())` — D-2's Family-B vacuity. Those private phase copies are retired:
//! every leg now carries a [`BindingClass`] (Option C, E12-B1), so a RED live
//! leg with its substrate up hard-fails at HEAD, exactly as the two Family-A
//! gates already did. The GA ladder still lives in `gate-registry.toml` and
//! still governs ONLY ship disposition, never dev-time enforcement.
//!
//! Every leg also carries a projected [`crate::gate_common::EvidenceState`] and
//! the gate publishes a `product_claim` (Story 13.6e AC1/AC2/AC5).

use crate::evidence_ledger::{
    finish_ledger_gate, run_exact_test_leg, BuildBinding, EvidenceLeg, EvidenceVerifier,
    LegObservation, SignatureCheck, TestLeg,
};
use crate::gate_common::{read_disposition, BindingClass};

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
pub(crate) const GATE_NAME: &str = "check-cross-region-consensus";

/// The four consensus oracles. Each runs by exact identity: the shared
/// `cross_region_live` binary also contains three-region and multi-tenant tests
/// with different substrate contracts, so broadcasting one unfiltered result
/// would turn their absence or failure into four unrelated consensus REDs.
const LIVE_LEGS: &[TestLeg] = &[
    TestLeg {
        name: "reattestation-mediated",
        class: BindingClass::AdvisorySubstrate,
        args: &[
            "test",
            "--locked",
            "-p",
            "maos-loom-lite",
            "--test",
            "cross_region_live",
            "reattest_copy_fails_then_reattest_succeeds",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ],
    },
    TestLeg {
        name: "convergence-oracle",
        class: BindingClass::AdvisorySubstrate,
        args: &[
            "test",
            "--locked",
            "-p",
            "maos-loom-lite",
            "--test",
            "cross_region_live",
            "crdt_reorder_independence_oracle_converges",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ],
    },
    TestLeg {
        name: "region-identity",
        class: BindingClass::AdvisorySubstrate,
        args: &[
            "test",
            "--locked",
            "-p",
            "maos-loom-lite",
            "--test",
            "cross_region_live",
            "region_identity_forge_rejected_count_moves",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ],
    },
    TestLeg {
        name: "ap-degrade",
        class: BindingClass::AdvisorySubstrate,
        args: &[
            "test",
            "--locked",
            "-p",
            "maos-loom-lite",
            "--test",
            "cross_region_live",
            "ap_degrade_real_partition",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ],
    },
];

fn live_substrate_present() -> bool {
    [
        "MAOS_TEST_POSTGRES",
        "MAOS_TEST_POSTGRES_A",
        "MAOS_TEST_POSTGRES_B",
        "MAOS_TEST_POSTGRES_C",
        "MAOS_TEST_POSTGRES_TEAM_A",
        "MAOS_TEST_POSTGRES_TEAM_B",
        "MAOS_TEST_POSTGRES_TEAM_C",
    ]
    .iter()
    .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
}

/// Run the kernel-ABI baseline leg by reusing the existing `check-kernel-baseline`
/// logic directly (no Postgres dependency). The re-pin must be GREEN — the
/// `WriteEntryPoint::CrossRegionReadmit` addition must not drift the kernel.
fn run_kernel_abi_leg(verifier: &EvidenceVerifier) -> EvidenceLeg {
    // Reuse the real baseline check rather than duplicating the line counter.
    // `run(false)` keeps its output on stderr (diagnostic) and returns Ok/Err;
    // it never emits JSON to stdout, so this gate's JSON output stays clean.
    let green = crate::check_kernel_baseline::run(false).is_ok();
    EvidenceLeg::observe(
        LegObservation {
            name: "kernel-abi-diff",
            class: BindingClass::Blocking,
            attempted: true,
            substrate_present: true,
            green,
            detail: if green {
                "kernel baseline re-pin GREEN".to_string()
            } else {
                "kernel baseline re-pin FAILED".to_string()
            },
            signature: SignatureCheck::default(),
            passed: Some(u32::from(green)),
            failed: Some(u32::from(!green)),
        },
        verifier.binding(),
        GATE_NAME,
    )
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + validate the phase disposition from the registry. The GA ladder
    //    is still the registry's job; it no longer decides dev-time
    //    enforcement, which is now leg-level `BindingClass` (Story 13.6e T5).
    let disposition = read_disposition(GATE_NAME)?;
    // The v2.0 binding promise MUST be present — its absence is a registry
    // defect (the gate would silently stay advisory forever).
    if !matches!(
        disposition.get("v2_0").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }

    let verifier = EvidenceVerifier::load(BuildBinding::for_run(GATE_NAME)?)?;

    // 2. Each conceptual leg runs only its exact oracle. The shared file holds
    // tests with unrelated A/B/C and TEAM_A/B/C contracts; those cannot affect
    // this gate. An absent or whitespace-only base connection makes all four
    // legs ABSENT and remains advisory on the local lane.
    let live_present = live_substrate_present();
    let mut legs: Vec<EvidenceLeg> = LIVE_LEGS
        .iter()
        .map(|spec| run_exact_test_leg(spec, live_present, GATE_NAME, &verifier))
        .collect();

    // 3. Kernel-ABI baseline leg (always attempted; no Postgres dependency).
    legs.push(run_kernel_abi_leg(&verifier));

    finish_ledger_gate(
        GATE_NAME,
        "Cross-Region Consensus Gate",
        json,
        &disposition,
        legs,
        &verifier,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T5/D-2 (Family B): a live leg that was ATTEMPTED and came back RED with
    /// its substrate up now hard-fails at HEAD. Before Story 13.6e this gate's
    /// private `CURRENT_PHASE = "v1_5"` turned it into `Ok(())`.
    #[test]
    fn a_red_live_leg_blocks_now_that_binding_is_leg_level() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let red = EvidenceLeg::observe(
            LegObservation {
                name: "convergence-oracle",
                class: BindingClass::AdvisorySubstrate,
                attempted: true,
                substrate_present: true,
                green: false,
                detail: "0 passed, 1 failed".to_string(),
                signature: SignatureCheck::default(),
                passed: Some(0),
                failed: Some(1),
            },
            verifier.binding(),
            GATE_NAME,
        );
        assert!(red.blocks_dev_lane(), "a RED live leg must block at HEAD");
        assert!(red.blocks_product_claim(false));
    }

    /// An unmeasured live leg is `ABSENT`, never green, and — with its
    /// substrate genuinely absent — advisory to the dev lane while still
    /// making the product claim NOT_PROVEN.
    #[test]
    fn skipped_leg_is_absent_not_green() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let leg = EvidenceLeg::observe(
            LegObservation {
                name: "convergence-oracle",
                class: BindingClass::AdvisorySubstrate,
                attempted: false,
                substrate_present: false,
                green: false,
                detail: "MAOS_TEST_POSTGRES unset — live oracle unmeasured".to_string(),
                signature: SignatureCheck::default(),
                passed: None,
                failed: None,
            },
            verifier.binding(),
            GATE_NAME,
        );
        assert!(!leg.green);
        assert!(!leg.attempted);
        assert_eq!(
            leg.state(),
            crate::gate_common::EvidenceState::Absent,
            "an unmeasured live leg is ABSENT"
        );
        assert!(!leg.blocks_dev_lane());
        assert!(!leg.blocks_product_claim(false));
        assert!(crate::evidence_ledger::product_claim(&[leg]).starts_with("NOT_PROVEN("));
    }
    #[test]
    fn every_consensus_leg_runs_one_exact_trusted_test() {
        for spec in LIVE_LEGS {
            let separator = spec
                .args
                .iter()
                .position(|arg| *arg == "--")
                .expect("libtest separator");
            let expected_test = spec.args[separator - 1];
            assert!(
                spec.args[separator + 1..].contains(&"--exact"),
                "{} must use libtest --exact",
                spec.name
            );
            assert_eq!(
                crate::evidence_ledger::trusted_evidence_tests(GATE_NAME, spec.name)
                    .expect("trusted mapping"),
                &[expected_test],
                "{} gate runner and published consumer mapping drifted",
                spec.name
            );
        }
    }
}
