#![cfg(feature = "trial-attestation")]
#![forbid(unsafe_code)]

//! Story 11.7 AC1-AC3 — black-box proven-red tests for the v2.0 trial-attestation
//! derivation harness (`maos_eval::trial_attestation`).
//!
//! These tests defend the machine-DERIVED contract: every per-participant fact
//! (`binary_loads`, `frames_run`, `halt_recall`, `sbom_verified`,
//! `signing_chain_verified`) is produced from real inputs and a forged
//! self-report that contradicts the derived value is IGNORED. They are the
//! de-canning guard for the "10.2 canned-trap" — a value typed into a file and
//! echoed as a pass. Each test names the concrete contract it defends.
//!
//! Signing-chain round-trips (which require the `maos-audit` signing helpers)
//! and the Chinese-wall proxy-cohort contract are covered in
//! `xtask/tests/trial_attestation_proven_red.rs`, where those crates are
//! available.

use maos_eval::trial_attestation::{
    derive_halt_recall, derive_participant_attestation, derive_reload_facts,
    derive_sbom_from_sources, summarize_attestations, DerivationInputs,
    DerivedParticipantAttestation, HaltRecallCounts, HermeticEnvironment, HermeticityReport,
    PackageId, ReloadExecutionReport, ReportedParticipantFacts, SigningDerivation, HALT_RECALL_FLOOR,
    PROVENANCE_STAMP,
};
use std::path::PathBuf;

// ─── fixtures ─────────────────────────────────────────────────────────────

/// Render a candidate-declared `Cargo.lock` over the given `(name, version)` set.
fn lock_with(packages: &[(&str, &str)]) -> String {
    packages
        .iter()
        .map(|(name, version)| {
            format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render an independently-recomputed `cargo tree --locked` closure.
fn tree_with(packages: &[(&str, &str)]) -> String {
    packages
        .iter()
        .map(|(name, version)| format!("{name} v{version}\n"))
        .collect()
}

/// Build a derived participant attestation whose only variable is whether the
/// signing chain verified. Used to exercise the success conjunction + summary.
fn attestation(signing_ok: bool) -> DerivedParticipantAttestation {
    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), &tree_with(&[("maos", "0.1.0")]));
    let signing = SigningDerivation {
        signing_chain_verified: signing_ok,
        verified_manifest_entries: if signing_ok { 1 } else { 0 },
        error: if signing_ok {
            None
        } else {
            Some("tampered signature".to_string())
        },
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 20,
            false_negatives: 0,
        },
        b"class-appropriate-corpus",
        true,
    );
    derive_participant_attestation(DerivationInputs {
        participant_id: "proxy",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// AC1 — clean-environment re-load derives binary_loads + frames_run
// ═══════════════════════════════════════════════════════════════════════════

/// AC1a: a binary that fails to load derives `binary_loads = false` (a derived
/// boolean), never an error or panic that masks the verdict.
#[test]
fn fail_to_load_derives_binary_loads_false_not_an_error() {
    let facts = derive_reload_facts(ReloadExecutionReport {
        load_accepted: false,
        frames_executed: 1_000,
    });
    assert!(
        !facts.binary_loads,
        "a fail-to-load must derive binary_loads=false (not an error that hides it)"
    );
    assert!(
        !facts.meets_frame_floor,
        "a non-loading binary cannot satisfy the frame floor"
    );
    assert_eq!(facts.frames_run, 1_000, "frames_run is still derived from the run report");
}

/// AC1b: the ≥1000-frame floor is a real boundary — 1000 meets it, 999 does not.
#[test]
fn frame_floor_1000_meets_and_999_derives_below_floor() {
    let at_floor = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let below = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 999,
    });
    assert_eq!(at_floor.frames_run, 1_000);
    assert!(at_floor.meets_frame_floor, "exactly 1000 frames must meet the floor");
    assert_eq!(below.frames_run, 999);
    assert!(
        !below.meets_frame_floor,
        "999 frames must derive below the 1000 floor (NFR-Test-8)"
    );
}

/// AC1c / L4: planting a stale MAOS artifact (a warm `~/.maos` cache) makes the
/// hermeticity assertion RED — proving "clean VM" is enforced, not assumed.
#[test]
fn dirty_environment_with_prior_maos_state_reds_hermeticity() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // Clean: neither prior-state path exists → clean.
    let clean = HermeticEnvironment {
        maos_home: tmp.path().join("no-home"),
        candidate_cache: tmp.path().join("no-cache"),
    }
    .assert_clean();
    assert!(clean.clean, "absent prior state must derive clean=true");
    assert!(clean.dirty_paths.is_empty());

    // Dirty: plant a non-empty ~/.maos cache → the leg reds.
    let dirty_home = tmp.path().join("dirty-home");
    std::fs::create_dir_all(&dirty_home).expect("mkdir");
    std::fs::write(dirty_home.join("warm-artifact"), b"stale candidate").expect("write");
    let dirty = HermeticEnvironment {
        maos_home: dirty_home,
        candidate_cache: tmp.path().join("no-cache-2"),
    }
    .assert_clean();
    assert!(
        !dirty.clean,
        "a planted stale MAOS artifact must red the hermeticity assertion"
    );
    assert!(
        !dirty.dirty_paths.is_empty(),
        "the offending path must be named so the violation is legible"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2 — SBOM completeness derived from TWO independent sources (F4 tautology guard)
// ═══════════════════════════════════════════════════════════════════════════

/// AC2a (drop): the candidate's declared `Cargo.lock` omits a transitive
/// dependency that the independently-recomputed `cargo tree` closure contains →
/// `sbom_verified = false`. This is the planted-mismatch negative control.
#[test]
fn sbom_drop_derives_false_when_candidate_lock_omits_a_dependency() {
    let declared = lock_with(&[("maos", "0.1.0")]); // omits serde
    let recomputed = tree_with(&[("maos", "0.1.0"), ("serde", "1.0.0")]);
    let derivation = derive_sbom_from_sources(&declared, &recomputed);
    assert!(
        !derivation.sbom_verified,
        "a dropped dependency must derive sbom_verified=false"
    );
    assert!(
        derivation
            .missing_from_declaration
            .contains(&PackageId::new("serde", "1.0.0")),
        "the dropped dependency must be named in missing_from_declaration"
    );
}

/// AC2a (pad): the candidate's declared `Cargo.lock` declares a spurious
/// dependency that the recomputed closure does not contain → `sbom_verified =
/// false`. A declaration that over-states its closure is also a mismatch.
#[test]
fn sbom_pad_derives_false_when_candidate_lock_declares_a_spurious_dependency() {
    let declared = lock_with(&[("maos", "0.1.0"), ("evil-crate", "9.9.9")]);
    let recomputed = tree_with(&[("maos", "0.1.0")]);
    let derivation = derive_sbom_from_sources(&declared, &recomputed);
    assert!(
        !derivation.sbom_verified,
        "a padded dependency must derive sbom_verified=false"
    );
    assert!(
        derivation
            .extra_in_declaration
            .contains(&PackageId::new("evil-crate", "9.9.9")),
        "the spurious dependency must be named in extra_in_declaration"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC3 — halt-recall MEASURED against the pinned corpus (never hard-coded)
// ═══════════════════════════════════════════════════════════════════════════

/// AC3a: a candidate that fails to halt derives `halt_recall` below the 0.85
/// floor. The score is the measured TP/(TP+FN) ratio, never a self-reported
/// float.
#[test]
fn low_halt_recall_derives_below_floor() {
    let low = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 5,
            false_negatives: 15,
        },
        b"class-appropriate-corpus",
        true,
    );
    assert!(
        low.halt_recall < HALT_RECALL_FLOOR,
        "5/20 recall (0.25) must derive below the 0.85 floor"
    );
    assert!(!low.meets_floor);
    assert!(
        low.provisional,
        "fixture-sourced runs must be stamped provisional (L5 anti-vacuous guard)"
    );
    assert!(
        !low.corpus_sha256.is_empty(),
        "the corpus SHA-256 provenance must be stamped on the derivation"
    );
}

/// AC3c / L5: an imperfect candidate derives a real MEASURED fraction, not a
/// canned `1.0`. 17 TP / 20 = 0.85 exercises the scoring boundary; a perfect
/// candidate legitimately reaches 1.0 — proving the value is computed, not
/// hard-coded.
#[test]
fn halt_recall_is_measured_not_hard_coded_to_one() {
    let imperfect = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 17,
            false_negatives: 3,
        },
        b"class-appropriate-corpus",
        true,
    );
    assert_ne!(
        imperfect.halt_recall, 1.0,
        "halt_recall must be measured (17/20), never a hard-coded 1.0"
    );
    assert!(
        imperfect.halt_recall > 0.84 && imperfect.halt_recall < 0.86,
        "measured recall should be 0.85 (17/20), got {}",
        imperfect.halt_recall
    );
    assert!(imperfect.meets_floor, "0.85 meets the >=0.85 floor");

    let perfect = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 20,
            false_negatives: 0,
        },
        b"class-appropriate-corpus",
        true,
    );
    assert_eq!(perfect.halt_recall, 1.0, "a perfect candidate legitimately hits 1.0");
    assert!(perfect.meets_floor);
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2c / AC4a — a forged self-report that contradicts the derivation is IGNORED
// ═══════════════════════════════════════════════════════════════════════════

/// AC2c / AC4a: a self-report claiming `signing_chain_verified = true` over a
/// tampered artifact (real derivation = false) is IGNORED — the derived value
/// wins and the success conjunction stays false. This is the
/// attestation-source reflex: a planted lie must turn the count red.
#[test]
fn forged_self_report_is_ignored_and_derived_success_stays_false() {
    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), &tree_with(&[("maos", "0.1.0")]));
    // The REAL signing-chain derivation failed (tampered artifact).
    let signing = SigningDerivation {
        signing_chain_verified: false,
        verified_manifest_entries: 0,
        error: Some("tampered signature".to_string()),
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 20,
            false_negatives: 0,
        },
        b"class-appropriate-corpus",
        true,
    );
    let record = derive_participant_attestation(DerivationInputs {
        participant_id: "forged-self-report-proxy",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: Some(ReportedParticipantFacts {
            binary_loads: true,
            frames_run: 1_000,
            halt_recall: 1.0,
            sbom_verified: true,
            signing_chain_verified: true, // the FORGERY
        }),
    });
    assert!(
        !record.signing_chain_verified,
        "the derived value must win over the forged self-report"
    );
    assert!(
        !record.success,
        "a tampered signature must fail the conjunction despite the forged self-report"
    );
    assert!(
        record.ignored_self_report,
        "a self-report contradicting the derivation must be flagged as ignored"
    );
}

/// AC3b: a self-reported `halt_recall` that contradicts the measured value is
/// ignored — the measured value (0.4) decides `meets_floor`, and the
/// fabrication cannot rescue the conjunction.
#[test]
fn halt_recall_self_report_contradiction_is_ignored() {
    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), &tree_with(&[("maos", "0.1.0")]));
    let signing = SigningDerivation {
        signing_chain_verified: true,
        verified_manifest_entries: 1,
        error: None,
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 8,
            false_negatives: 12,
        }, // 0.4 — below floor
        b"class-appropriate-corpus",
        true,
    );
    let record = derive_participant_attestation(DerivationInputs {
        participant_id: "halt-fabricator",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: Some(ReportedParticipantFacts {
            binary_loads: true,
            frames_run: 1_000,
            halt_recall: 0.99, // the fabrication
            sbom_verified: true,
            signing_chain_verified: true,
        }),
    });
    assert!(
        record.halt_recall < HALT_RECALL_FLOOR,
        "measured 0.4 must win over the 0.99 self-report"
    );
    assert!(!record.success, "the measured shortfall must fail the conjunction");
    assert!(record.ignored_self_report);
}

/// The environment-hermeticity leg is part of the success conjunction: even
/// when every other fact is green, a dirty environment means the re-load was
/// NOT on a clean VM → `success = false` and the provenance records it.
#[test]
fn dirty_environment_invalidates_the_full_success_conjunction() {
    let dirty = HermeticityReport {
        clean: false,
        checked_paths: Vec::new(),
        dirty_paths: vec![PathBuf::from("/home/runner/.maos")],
    };
    let reload = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), &tree_with(&[("maos", "0.1.0")]));
    let signing = SigningDerivation {
        signing_chain_verified: true,
        verified_manifest_entries: 1,
        error: None,
    };
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 20,
            false_negatives: 0,
        },
        b"class-appropriate-corpus",
        true,
    );
    let record = derive_participant_attestation(DerivationInputs {
        participant_id: "dirty-env-proxy",
        produced_binary: true,
        artifact_bytes: b"candidate artifact",
        hermeticity: &dirty,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: None,
    });
    assert!(
        !record.environment_clean,
        "a dirty environment must be reflected in the provenance stamp"
    );
    assert!(
        !record.success,
        "a dirty environment must fail the conjunction (no clean-VM re-load)"
    );
    assert_eq!(record.provenance_stamp, PROVENANCE_STAMP);
}

/// AC3: `derived_successes` is a count of records satisfying the derived
/// conjunction — never a self-reported summary line. A green + red pair yields
/// exactly one derived success.
#[test]
fn summarize_derives_success_count_not_a_self_reported_line() {
    let green = attestation(true);
    let red = attestation(false);
    assert!(green.success);
    assert!(!red.success);
    let summary = summarize_attestations(&[green, red]);
    assert_eq!(summary.participants_total, 2);
    assert_eq!(
        summary.derived_successes, 1,
        "only the record satisfying the full derived conjunction counts as a success"
    );
    assert_eq!(summary.provenance_stamp, PROVENANCE_STAMP);
}
