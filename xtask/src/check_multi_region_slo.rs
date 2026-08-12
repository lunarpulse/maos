#![forbid(unsafe_code)]

//! Story 11.2b (AC5, D5) — `check-multi-region-slo` gate.
//!
//! ONE standalone gate with **per-leg verdict independence** — each leg reads
//! its OWN oracle, so one break reds exactly one leg (Murat's one-break-one-red,
//! L4). This gate is DISTINCT from `check-cross-region-consensus` and MUST NOT
//! append legs there: the consensus gate's single-binary broadcast makes its
//! legs non-independent (a convergence break would mask an SLO/read-path break).
//!
//! # The five legs (each its own oracle invocation)
//!
//! 1. **three-region-convergence** (live, 3 PG) — `cross_region_live`'s
//!    `three_region_*` tests: ≥10-agent convergence across three sovereign
//!    regions + distinct-datname + physical-absence controls.
//! 2. **roundtrip-slo** (live, 2 PG, +`slo-fault-inject`) — the single-clock
//!    A→B→A round-trip SLO: the clean budget-met GREEN path AND the
//!    `slo-fault-inject` mutation that REDs the budget (both must pass).
//! 3. **halt-presence** (no PG) — per-region halt-receipt PRESENCE observability
//!    + the suppress-emission falsifier.
//! 4. **live-read-region-identity** (live, 3 PG + structural chokepoint) — the
//!    `live_read_region_identity_*` tests (fail-closed read path) AND the
//!    `read_path_chokepoint` static architecture gate (guard wired + port
//!    isolation). The chokepoint ALWAYS runs (structural); the live read tests
//!    run only with Postgres.
//! 5. **kernel-abi-diff** (no PG) — `check-kernel-baseline` re-pin GREEN at
//!    23023 (ZERO kernel-Δ).
//!
//! # Live-oracle posture (D5 anti-canned)
//!
//! The live legs (1, 2, 4) are gated on `MAOS_TEST_POSTGRES_{A,B,C}`. An
//! environment WITHOUT Postgres reports those legs as `ABSENT` — never a silent
//! pass. A vacuous leg (attempted but ZERO tests) hard-fails at every phase
//! (the J4 anti-canned guard).
//!
//! # Story 13.6e — leg-level binding, and the ledger
//!
//! This gate used to key its whole verdict off a private `CURRENT_PHASE =
//! "v1_5"` const plus a registry `advisory` row, so a RED LIVE leg returned
//! `Ok(())` — D-2's Family-B vacuity, and the `roundtrip-slo` floor breach sat
//! behind it for a month. Those private phase copies are retired: every leg now
//! carries a [`BindingClass`] (Option C, E12-B1), so a RED live leg with its
//! substrate up hard-fails at HEAD. The GA ladder still lives in
//! `gate-registry.toml` and still governs ONLY ship disposition.
//!
//! Every leg also carries a projected [`crate::gate_common::EvidenceState`] and
//! the gate publishes a `product_claim` (Story 13.6e AC1/AC2/AC5).

use crate::evidence_ledger::{
    finish_ledger_gate, harness_env, leg_signature, leg_signature_many, BuildBinding, EvidenceLeg,
    EvidenceVerifier, LegObservation, SignatureCheck,
};
use crate::gate_common::{emit_command, read_disposition, BindingClass};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
pub(crate) const GATE_NAME: &str = "check-multi-region-slo";

/// Raw labels are the single source for both gate construction and ledger
/// membership validation.
const RAW_LEG_LABELS: [&str; 5] = [
    "three-region-convergence",
    "roundtrip-slo",
    "halt-presence",
    "live-read-region-identity",
    "kernel-abi-diff",
];

/// One oracle leg's raw observation, before projection.
struct RawLeg {
    label: &'static str,
    class: BindingClass,
    substrate_present: bool,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
    signature: SignatureCheck,
}

impl RawLeg {
    /// A live leg that was not attempted (Postgres unavailable) — unmeasured,
    /// and therefore `ABSENT` once projected.
    fn skipped(label: &'static str) -> Self {
        RawLeg {
            label,
            class: BindingClass::AdvisorySubstrate,
            substrate_present: false,
            passed: 0,
            failed: 0,
            ran: false,
            attempted: false,
            green: false,
            signature: SignatureCheck::default(),
        }
    }

    fn into_leg(self, verifier: &EvidenceVerifier) -> EvidenceLeg {
        let detail = format!(
            "{} passed, {} failed (ran={})",
            self.passed, self.failed, self.ran
        );
        EvidenceLeg::observe(
            LegObservation {
                name: self.label,
                class: self.class,
                attempted: self.attempted,
                substrate_present: self.substrate_present,
                green: self.green,
                detail,
                signature: self.signature,
                passed: Some(self.passed),
                failed: Some(self.failed),
            },
            verifier.binding(),
            GATE_NAME,
        )
    }
}

fn connection_present(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| !value.trim().is_empty())
}

/// Two-region roundtrip needs A/B only; three-region convergence and identity
/// need A/B/C. Keep the predicates separate so a missing C cannot suppress a
/// valid A↔B measurement.
fn two_region_postgres_available() -> bool {
    connection_present("MAOS_TEST_POSTGRES_A") && connection_present("MAOS_TEST_POSTGRES_B")
}

fn three_region_postgres_available() -> bool {
    two_region_postgres_available() && connection_present("MAOS_TEST_POSTGRES_C")
}

fn sink_path(leg: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "maos-evidence-{GATE_NAME}-{leg}-{}-{nanos:x}.jsonl",
        std::process::id()
    ))
}

/// One `cargo test` invocation plus the transcript it produced.
struct CargoRun {
    passed: u32,
    failed: u32,
    ran: bool,
    green: bool,
    transcript: String,
    sink: PathBuf,
}

/// Invoke a `cargo test` filtered invocation and parse its `test result:`
/// summary. PER-LEG INDEPENDENCE: each leg calls this with its OWN
/// package/test/filter, so one break reds exactly one leg (no shared broadcast).
fn invoke_cargo_test(
    leg: &str,
    package: &str,
    test_file: &str,
    name_filter: Option<&str>,
    features: Option<&str>,
    ignored: bool,
    verifier: &EvidenceVerifier,
) -> Result<CargoRun, String> {
    let sink = sink_path(leg);
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", package, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    // Test-binary args after `--`.
    let mut dashdash = Vec::<&str>::new();
    dashdash.push("--");
    if let Some(flt) = name_filter {
        dashdash.push(flt);
    }
    if ignored {
        dashdash.push("--ignored");
    }
    dashdash.push("--nocapture");
    cmd.args(&dashdash);
    cmd.envs(harness_env(GATE_NAME, verifier.binding(), &sink));
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({package}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        let tail: String = combined
            .lines()
            .rev()
            .take(30)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {package}/{test_file} NOT green \
             (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok(CargoRun {
        passed,
        failed,
        ran,
        green,
        transcript: combined,
        sink,
    })
}

/// A leg whose oracle could not even be started.
fn errored(label: &'static str, class: BindingClass, substrate_present: bool) -> RawLeg {
    RawLeg {
        label,
        class,
        substrate_present,
        passed: 0,
        failed: 1,
        ran: true,
        attempted: true,
        green: false,
        signature: SignatureCheck::default(),
    }
}

/// Leg 1: three-region convergence (live, 3 PG).
fn run_three_region_convergence_leg(pg: bool, verifier: &EvidenceVerifier) -> RawLeg {
    let label = RAW_LEG_LABELS[0];
    if !pg {
        return RawLeg::skipped(label);
    }
    match invoke_cargo_test(
        label,
        "maos-loom-lite",
        "cross_region_live",
        Some("three_region"),
        None,
        true,
        verifier,
    ) {
        Ok(run) => {
            let signature = leg_signature(
                verifier,
                GATE_NAME,
                &[
                    "three_region_convergence_all_three_equal",
                    "three_region_reorder_independence",
                    "three_region_empty_set_is_na",
                ],
                &run.transcript,
                &run.sink,
                BindingClass::AdvisorySubstrate,
                run.green,
            );
            RawLeg {
                label,
                class: BindingClass::AdvisorySubstrate,
                substrate_present: true,
                passed: run.passed,
                failed: run.failed,
                ran: run.ran,
                attempted: true,
                green: run.green,
                signature,
            }
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} leg error: {e}");
            errored(label, BindingClass::AdvisorySubstrate, true)
        }
    }
}

/// Leg 2: roundtrip-slo (live, 2 PG) + the `slo-fault-inject` mutation. GREEN
/// requires BOTH the clean budget-met path AND the mutation that REDs the
/// budget (the falsifier must move the number). Both are "pass" outcomes.
fn run_roundtrip_slo_leg(pg: bool, verifier: &EvidenceVerifier) -> RawLeg {
    let label = RAW_LEG_LABELS[1];
    if !pg {
        return RawLeg::skipped(label);
    }
    let live = match invoke_cargo_test(
        label,
        "maos-bench",
        "t_11_2b_cross_region_slo",
        Some("cross_region_roundtrip_live"),
        None,
        true,
        verifier,
    ) {
        Ok(run) => run,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} live error: {e}");
            return errored(label, BindingClass::AdvisorySubstrate, true);
        }
    };
    let mutation = match invoke_cargo_test(
        label,
        "maos-bench",
        "t_11_2b_cross_region_slo",
        Some("cross_region_roundtrip_mutation"),
        Some("slo-fault-inject"),
        true,
        verifier,
    ) {
        Ok(run) => run,
        Err(e) => {
            let _ = std::fs::remove_file(&live.sink);
            eprintln!("{GATE_NAME}: {label} mutation error: {e}");
            return errored(label, BindingClass::AdvisorySubstrate, true);
        }
    };
    let green = live.green && mutation.green;
    // Both invocations determine this composite leg. A clean-run signature
    // cannot prove the mutation falsifier, so verify and publish both harness
    // records as one evidence block.
    let signature = leg_signature_many(
        verifier,
        GATE_NAME,
        &[
            "cross_region_roundtrip_live",
            "cross_region_roundtrip_mutation",
        ],
        &[
            (live.transcript.as_str(), live.sink.as_path()),
            (mutation.transcript.as_str(), mutation.sink.as_path()),
        ],
        BindingClass::AdvisorySubstrate,
        green,
    );
    RawLeg {
        label,
        class: BindingClass::AdvisorySubstrate,
        substrate_present: true,
        passed: live.passed + mutation.passed,
        failed: live.failed + mutation.failed,
        ran: live.ran && mutation.ran,
        attempted: true,
        green,
        signature,
    }
}

/// Leg 3: halt-presence observability (no PG — in-process termination corpus).
/// Hermetic, therefore [`BindingClass::Blocking`]: a RED here has always been a
/// real defect and now hard-fails at HEAD instead of hiding behind the phase.
fn run_halt_presence_leg(verifier: &EvidenceVerifier) -> RawLeg {
    let label = RAW_LEG_LABELS[2];
    match invoke_cargo_test(
        label,
        "maos-bench",
        "t_11_2b_halt_presence",
        None,
        None,
        false,
        verifier,
    ) {
        Ok(run) => {
            let _ = std::fs::remove_file(&run.sink);
            RawLeg {
                label,
                class: BindingClass::Blocking,
                substrate_present: true,
                passed: run.passed,
                failed: run.failed,
                ran: run.ran,
                attempted: true,
                green: run.green,
                signature: SignatureCheck::unverified(
                    "hermetic leg — reproducible from source, no signature required".to_string(),
                ),
            }
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} leg error: {e}");
            errored(label, BindingClass::Blocking, true)
        }
    }
}

fn blocking_chokepoint_failure(run: CargoRun) -> RawLeg {
    RawLeg {
        label: RAW_LEG_LABELS[3],
        class: BindingClass::Blocking,
        substrate_present: true,
        passed: run.passed,
        failed: run.failed,
        ran: run.ran,
        attempted: true,
        green: false,
        signature: SignatureCheck::unverified(
            "the always-on read-path chokepoint failed before the live half".to_string(),
        ),
    }
}

/// Leg 4: live-read-region-identity (live, 3 PG) + the structural chokepoint
/// (no PG, ALWAYS runs). The chokepoint is a structural precondition — if it
/// fails, the leg is RED regardless of Postgres. The live read tests run only
/// with Postgres; without it the live half is unmeasured (leg not green), and
/// `substrate_present` stays FALSE so the dev lane is not blocked by an
/// unmeasurable half.
fn run_live_read_region_identity_leg(pg: bool, verifier: &EvidenceVerifier) -> RawLeg {
    let label = RAW_LEG_LABELS[3];
    // The chokepoint ALWAYS runs (structural — no Postgres).
    let chokepoint = match invoke_cargo_test(
        label,
        "maos-loom-lite",
        "read_path_chokepoint",
        None,
        None,
        false,
        verifier,
    ) {
        Ok(run) => run,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} chokepoint error: {e}");
            return errored(label, BindingClass::Blocking, true);
        }
    };
    let _ = std::fs::remove_file(&chokepoint.sink);
    if !chokepoint.green {
        return blocking_chokepoint_failure(chokepoint);
    }
    if !pg {
        return RawLeg {
            label,
            class: BindingClass::AdvisorySubstrate,
            substrate_present: false,
            passed: chokepoint.passed,
            failed: chokepoint.failed,
            ran: chokepoint.ran,
            attempted: true,
            green: false,
            signature: SignatureCheck::unverified(
                "live half unmeasured — non-empty MAOS_TEST_POSTGRES_{A,B,C} required".to_string(),
            ),
        };
    }
    let live_read = match invoke_cargo_test(
        label,
        "maos-loom-lite",
        "cross_region_live",
        Some("live_read_region_identity"),
        None,
        true,
        verifier,
    ) {
        Ok(run) => run,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} live-read error: {e}");
            return errored(label, BindingClass::AdvisorySubstrate, true);
        }
    };
    // `live_scan_region_identity_foreign_refused` does not match the
    // `live_read_region_identity` filter. Run it explicitly; otherwise the leg
    // can never verify the five records it publishes as its trusted set.
    let live_scan = match invoke_cargo_test(
        label,
        "maos-loom-lite",
        "cross_region_live",
        Some("live_scan_region_identity_foreign_refused"),
        None,
        true,
        verifier,
    ) {
        Ok(run) => run,
        Err(e) => {
            let _ = std::fs::remove_file(&live_read.sink);
            eprintln!("{GATE_NAME}: {label} live-scan error: {e}");
            return errored(label, BindingClass::AdvisorySubstrate, true);
        }
    };
    let green = chokepoint.green && live_read.green && live_scan.green;
    let signature = leg_signature_many(
        verifier,
        GATE_NAME,
        &[
            "live_read_region_identity_foreign_refused",
            "live_read_region_identity_reattested_served",
            "live_read_region_identity_home_served",
            "live_scan_region_identity_foreign_refused",
            "live_read_region_identity_forged_stamp_served",
        ],
        &[
            (live_read.transcript.as_str(), live_read.sink.as_path()),
            (live_scan.transcript.as_str(), live_scan.sink.as_path()),
        ],
        BindingClass::AdvisorySubstrate,
        green,
    );
    RawLeg {
        label,
        class: BindingClass::AdvisorySubstrate,
        substrate_present: true,
        passed: chokepoint.passed + live_read.passed + live_scan.passed,
        failed: chokepoint.failed + live_read.failed + live_scan.failed,
        ran: chokepoint.ran && live_read.ran && live_scan.ran,
        attempted: true,
        green,
        signature,
    }
}

/// Leg 5: kernel-ABI baseline (no PG) — ZERO kernel-Δ.
fn run_kernel_abi_leg() -> RawLeg {
    let green = crate::check_kernel_baseline::run(false).is_ok();
    RawLeg {
        label: RAW_LEG_LABELS[4],
        class: BindingClass::Blocking,
        substrate_present: true,
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        signature: SignatureCheck::default(),
    }
}

/// Sum `passed`/`failed` counts across every `test result:` line in `output`.
fn parse_test_summary(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("test result:") {
            passed += parse_count(rest, "passed");
            failed += parse_count(rest, "failed");
        }
    }
    (passed, failed)
}

/// Parse the total of `"<n> <key>"` occurrences in `s` (e.g. `19 passed`).
fn parse_count(s: &str, key: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut total = 0u32;
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(key) {
        let abs = from + rel;
        let mut end = abs;
        while end > 0 && bytes[end - 1] == b' ' {
            end -= 1;
        }
        let mut start = end;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start < end {
            if let Ok(n) = s[start..end].parse::<u32>() {
                total += n;
            }
        }
        from = abs + key.len();
    }
    total
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + validate the phase disposition from the registry. The GA ladder
    //    is still the registry's job; it no longer decides dev-time enforcement
    //    (Story 13.6e T5 — leg-level `BindingClass`).
    let disposition = read_disposition(GATE_NAME)?;
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

    // 2. Per-leg oracles (each its OWN invocation — per-leg independence).
    // Roundtrip needs non-empty A/B; convergence and live identity need
    // non-empty A/B/C. Hermetic halt + kernel legs always attempt.
    let pg_ab = two_region_postgres_available();
    let pg_abc = three_region_postgres_available();
    let raw: Vec<RawLeg> = vec![
        run_three_region_convergence_leg(pg_abc, &verifier),
        run_roundtrip_slo_leg(pg_ab, &verifier),
        run_halt_presence_leg(&verifier),
        run_live_read_region_identity_leg(pg_abc, &verifier),
        run_kernel_abi_leg(),
    ];

    // 3. Vacuous-green guard (J4 anti-canned): a leg that was ATTEMPTED but
    //    compiled to ZERO tests / never reported results is a re-stubbed
    //    harness — hard-fail at EVERY phase. ABSENT legs are NOT vacuous
    //    (unmeasured). The kernel-abi-diff leg is exempt (baseline, not a count).
    for leg in &raw {
        if leg.label != "kernel-abi-diff"
            && leg.attempted
            && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The oracle produced no tests — a re-stubbed harness cannot pass this gate \
                 (J4 anti-canned guard).",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let legs: Vec<EvidenceLeg> = raw.into_iter().map(|leg| leg.into_leg(&verifier)).collect();

    finish_ledger_gate(
        GATE_NAME,
        "Multi-Region SLO Gate",
        json,
        &disposition,
        legs,
        &verifier,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_sums_passed_and_failed() {
        let s = "test result: ok. 19 passed; 0 failed; 0 ignored";
        assert_eq!(parse_count(s, "passed"), 19);
        assert_eq!(parse_count(s, "failed"), 0);
        let s2 = "test result: FAILED. 3 passed; 2 failed; 1 ignored";
        assert_eq!(parse_count(s2, "passed"), 3);
        assert_eq!(parse_count(s2, "failed"), 2);
    }

    /// T5/D-2 (Family B): `roundtrip-slo` is the leg whose real floor breach sat
    /// behind `CURRENT_PHASE = "v1_5"` and exited 0 anyway. With leg-level
    /// binding it blocks — and the ledger records why.
    #[test]
    fn a_red_roundtrip_slo_leg_blocks_and_is_indeterminate() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let leg = RawLeg {
            label: "roundtrip-slo",
            class: BindingClass::AdvisorySubstrate,
            substrate_present: true,
            passed: 1,
            failed: 1,
            ran: true,
            attempted: true,
            green: false,
            signature: SignatureCheck::default(),
        }
        .into_leg(&verifier);
        assert_eq!(
            leg.state(),
            crate::gate_common::EvidenceState::Indeterminate
        );
        assert!(leg.blocks_dev_lane(), "a RED live leg must block at HEAD");
        assert!(leg.blocks_product_claim(false));
    }

    /// The hermetic legs bind unconditionally — `halt-presence` is `Blocking`,
    /// so a RED there hard-fails whether or not any substrate is present.
    #[test]
    fn halt_presence_is_hermetic_and_binds_without_substrate() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let leg = RawLeg {
            label: "halt-presence",
            class: BindingClass::Blocking,
            substrate_present: false,
            passed: 0,
            failed: 1,
            ran: true,
            attempted: true,
            green: false,
            signature: SignatureCheck::default(),
        }
        .into_leg(&verifier);
        assert!(leg.blocks_dev_lane());
    }

    #[test]
    fn structural_chokepoint_failure_blocks_without_postgres() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let leg = blocking_chokepoint_failure(CargoRun {
            passed: 5,
            failed: 1,
            ran: true,
            green: false,
            transcript: String::new(),
            sink: PathBuf::new(),
        })
        .into_leg(&verifier);
        assert_eq!(leg.binding, "blocking");
        assert!(leg.blocks_dev_lane());
    }

    #[test]
    fn skipped_live_leg_is_absent_and_unmeasured() {
        let verifier = EvidenceVerifier::with_pubkey(
            BuildBinding {
                commit: "c0ffee".to_string(),
                nonce: "n".to_string(),
            },
            None,
        );
        let leg = RawLeg::skipped("three-region-convergence").into_leg(&verifier);
        assert_eq!(leg.state(), crate::gate_common::EvidenceState::Absent);
        assert!(!leg.green);
        assert!(!leg.blocks_dev_lane());
        assert!(!leg.blocks_product_claim(false));
    }

    #[test]
    fn ledger_leg_names_are_derived_from_raw_labels() {
        assert_eq!(ledger_leg_names(), RAW_LEG_LABELS.to_vec());
    }
}

/// Complete ledger leg set, derived from the raw labels used to construct this
/// gate's legs.
pub fn ledger_leg_names() -> Vec<&'static str> {
    RAW_LEG_LABELS.to_vec()
}
