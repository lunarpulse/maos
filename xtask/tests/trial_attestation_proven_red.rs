#![forbid(unsafe_code)]

//! Story 11.7 AC2/AC4 — proven-red tests for the v2.0 trial-attestation producer
//! gate and the shared substrates it reuses.
//!
//! These cover the contracts that REQUIRE the `maos-audit` signing helpers and
//! the `maos-fkcs` Chinese-wall proxy cohort — both available to the `xtask`
//! test crate — plus the producer gate's `--json` contract, the ship-gate
//! completeness enrollment, and the L11 release-graph-absence tripwire.
//!
//! The pure-logic derivation contracts (reload, SBOM reconciliation, halt-recall
//! math, self-report ignore) live in `crates/maos-eval/tests/trial_attestation.rs`.

use maos_audit::release_verify::{generate_sha256sums, sign_sha256sums};
use maos_eval::trial_attestation::{
    derive_halt_recall, derive_participant_attestation, derive_reload_facts,
    derive_sbom_from_sources, derive_signing_chain, recompute_cargo_tree_locked, sha256_hex,
    DerivationInputs, HaltRecallCounts, HermeticityReport, ReloadExecutionReport,
    ReportedParticipantFacts, PROXY_COHORT_LABEL,
};
use maos_fkcs::{AdmissionHarness, KernelFreezeProvenance, ProxyCohort, ProxySpirit};

fn lock_with(packages: &[(&str, &str)]) -> String {
    packages
        .iter()
        .map(|(name, version)| format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─── helpers ───────────────────────────────────────────────────────────────

/// The development signing seed whose derived public key is the bundled
/// `RELEASE_PUBKEY` in non-production builds (matches `release_verify::tests`).
/// Lets the proven-reds sign a valid artifact offline.
fn dev_seed() -> [u8; 32] {
    let hex = "794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d";
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid dev seed hex"))
        .collect::<Vec<_>>();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    seed
}

/// Run an xtask subcommand `--json` against the workspace root (the only CWD
/// from which the gates can resolve `xtask/gate-registry.toml` and
/// `.github/workflows/discipline.yml`).
fn run_gate(subcommand: &str) -> std::process::Output {
    let workspace_root = std::env::current_dir()
        .expect("test cwd")
        .parent()
        .expect("workspace root is the parent of the xtask package dir")
        .to_path_buf();
    std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([subcommand, "--json"])
        .current_dir(&workspace_root)
        .output()
        .expect("failed to run xtask subcommand")
}

/// The pinned kernel src-line count (L9 tripwire). Reused for the proxy-cohort
/// freeze provenance so the cohort score reflects a real, stable kernel.
const KERNEL_BASELINE_LINES: usize = 23_081;

// ═══════════════════════════════════════════════════════════════════════════
// AC2b — signing chain derived at the REAL Ed25519 verify path
// ═══════════════════════════════════════════════════════════════════════════

/// AC2b (green): a properly-signed artifact verifies against `RELEASE_PUBKEY`
/// via the real `maos_audit::release_verify` path (Ed25519 over the SHA256SUMS
/// manifest). This proves the green path is a real cryptographic verify, not a
/// harness echo.
#[test]
fn valid_signature_derives_signing_chain_verified_true_at_real_ed25519_path() {
    let artifact = b"candidate artifact";
    let hash = sha256_hex(artifact);
    let sha256sums = generate_sha256sums(&[("candidate.bin".to_string(), hash)]);
    let sig = sign_sha256sums(sha256sums.as_bytes(), &dev_seed());

    let derivation =
        derive_signing_chain(sha256sums.as_bytes(), &sig, &[("candidate.bin", artifact)]);

    assert!(
        derivation.signing_chain_verified,
        "a valid signature must verify against RELEASE_PUBKEY"
    );
    assert_eq!(derivation.verified_manifest_entries, 1);
    assert!(derivation.error.is_none());
}

/// AC2b (red): a tampered signature, a signature over tampered SHA256SUMS
/// content, and a signature from the wrong key each fail the real Ed25519
/// verify → `signing_chain_verified = false`. L3: the negative controls fail at
/// the REAL verify path, not a harness-internal assert.
#[test]
fn tampered_and_wrong_key_signatures_derive_false_at_real_verify_path() {
    let artifact = b"candidate artifact";
    let hash = sha256_hex(artifact);
    let sha256sums = generate_sha256sums(&[("candidate.bin".to_string(), hash)]);
    let sig = sign_sha256sums(sha256sums.as_bytes(), &dev_seed());

    // (a) tampered signature bytes.
    let mut tampered_sig = sig;
    tampered_sig[0] ^= 0xA5;
    let tampered = derive_signing_chain(
        sha256sums.as_bytes(),
        &tampered_sig,
        &[("candidate.bin", artifact)],
    );
    assert!(
        !tampered.signing_chain_verified,
        "a tampered signature must derive false"
    );

    // (b) valid signature over tampered SHA256SUMS content (hash changed).
    let tampered_sums =
        generate_sha256sums(&[("candidate.bin".to_string(), sha256_hex(b"different bytes"))]);
    let content_tampered = derive_signing_chain(
        tampered_sums.as_bytes(),
        &sig,
        &[("candidate.bin", artifact)],
    );
    assert!(
        !content_tampered.signing_chain_verified,
        "a signature over tampered content must derive false"
    );

    // (c) signature made with a different key, verified against RELEASE_PUBKEY.
    let wrong_seed = [0x11u8; 32];
    let wrong_sig = sign_sha256sums(sha256sums.as_bytes(), &wrong_seed);
    let wrong_key = derive_signing_chain(
        sha256sums.as_bytes(),
        &wrong_sig,
        &[("candidate.bin", artifact)],
    );
    assert!(
        !wrong_key.signing_chain_verified,
        "a signature from the wrong key must derive false"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC4 — empty Chinese-wall proxy cohort is N/A (never a vacuous pass)
// ═══════════════════════════════════════════════════════════════════════════

/// AC4 / Murat's two-things guard: an empty proxy cohort is N/A — it can never
/// pose as a vacuous pass. A non-empty cohort carries the honest
/// "in-house Chinese-wall proxy" label and an ADVISORY floor (the proxy score
/// never blocks at v2.0; the genuine-external floor is v2.5).
#[test]
fn empty_chinese_wall_proxy_cohort_is_na_not_a_vacuous_pass() {
    let empty = ProxyCohort::new(Vec::new()).evaluate(
        &AdmissionHarness::default(),
        &KernelFreezeProvenance::stable_at(KERNEL_BASELINE_LINES),
    );
    assert!(
        empty.is_na,
        "an empty cohort must be is_na (never a vacuous pass)"
    );
    assert_eq!(empty.cohort_label, PROXY_COHORT_LABEL);

    let cohort = ProxyCohort::new(vec![ProxySpirit::conformance("proxy-1")]).evaluate(
        &AdmissionHarness::default(),
        &KernelFreezeProvenance::stable_at(KERNEL_BASELINE_LINES),
    );
    assert!(!cohort.is_na, "a non-empty cohort must not be is_na");
    assert_eq!(cohort.cohort_label, PROXY_COHORT_LABEL);
    assert!(
        cohort.floor_is_advisory_for_proxy_cohort,
        "the proxy-cohort floor is advisory at v2.0 (we wrote the proxy)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC4 — producer gate JSON: non-vacuous legs + v2.0 blocking disposition
// ═══════════════════════════════════════════════════════════════════════════

/// AC4 / F2: the producer gate `check-trial-attestation --json` emits a report
/// whose falsifier legs are each NON-VACUOUS (actually ran, with a real
/// pass/fail signal) and whose disposition is `v2_0 = "blocking"` (absent
/// result → BLOCK at the v2.0 ship gate). A vacuous leg is a defect.
#[test]
fn producer_gate_json_reports_non_vacuous_legs_and_v2_0_blocking_disposition() {
    let out = run_gate("check-trial-attestation");
    assert!(
        out.status.success(),
        "check-trial-attestation should pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
        .expect("gate emits one JSON object");

    assert_eq!(json["gate"], "check-trial-attestation");
    assert_eq!(
        json["disposition"]["v2_0"], "blocking",
        "the producer gate must record v2_0=blocking (absent→BLOCK@v2.0)"
    );

    let legs = json["legs"]
        .as_array()
        .expect("legs is a JSON array of falsifier results");
    assert!(
        !legs.is_empty(),
        "the gate must carry minimum falsifier legs"
    );
    for leg in legs {
        let label = leg["label"].as_str().unwrap_or("<unnamed>");
        let ran = leg["ran"].as_bool().unwrap_or(false);
        let attempted = leg["attempted"].as_bool().unwrap_or(false);
        let passed = leg["passed"].as_u64().unwrap_or(0);
        let failed = leg["failed"].as_u64().unwrap_or(0);
        assert!(
            attempted && ran,
            "leg `{label}` must have actually run (attempted={attempted}, ran={ran})"
        );
        assert!(
            passed > 0 || failed > 0,
            "leg `{label}` is vacuous (passed={passed}, failed={failed}) — a vacuous leg is a defect"
        );
    }
    assert!(
        json["vacuous_leg"].is_null(),
        "no leg may be vacuous: {:?}",
        json["vacuous_leg"]
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// L11 — release-graph absence (ship-blocker tripwire)
// ═══════════════════════════════════════════════════════════════════════════

/// L11 / AC4: the `trial-attestation` feature and the `maos-fkcs` test-harness
/// crate must NEVER enter the `maos-bin` release dependency graph. If a
/// `*-fault-inject` path or the eval harness ever leaks into the shipped
/// binary, this leg reds as a ship-blocker.
#[test]
fn release_graph_excludes_trial_attestation_feature_and_maos_fkcs() {
    let workspace_root = std::env::current_dir()
        .expect("test cwd")
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let out = std::process::Command::new("cargo")
        .args(["tree", "-p", "maos-bin", "-e", "features"])
        .current_dir(&workspace_root)
        .output()
        .expect("cargo tree invocation failed");
    assert!(
        out.status.success(),
        "cargo tree -p maos-bin failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8_lossy(&out.stdout);

    assert!(
        !tree.contains("maos-eval feature \"trial-attestation\""),
        "the trial-attestation feature must not leak into maos-bin's release graph:\n{tree}"
    );

    let fkcs_leaked = tree.lines().any(|line| {
        let stripped = line.trim_start_matches(|c: char| c.is_whitespace() || "│├└─".contains(c));
        stripped
            .split_whitespace()
            .next()
            .map(|name| name == "maos-fkcs")
            .unwrap_or(false)
    });
    assert!(
        !fkcs_leaked,
        "maos-fkcs must not appear in maos-bin's release dependency graph:\n{tree}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC4d — ship-gate completeness enrolls the new producer gate
// ═══════════════════════════════════════════════════════════════════════════

/// AC4d / L8: the new producer gate is enrolled in the ship-gate aggregate —
/// both `check_ship_gate_completeness::EXPECTED_GATES` and the
/// `v1.0-ship-gate` `needs:` array in `discipline.yml`. Stripping either reds
/// the completeness check. Until enrolled, `check-trial-attestation` is absent
/// from the aggregate `needs`, so this leg reds.
#[test]
fn ship_gate_completeness_enrolls_check_trial_attestation() {
    let out = run_gate("check-ship-gate-completeness");
    assert!(
        out.status.success(),
        "check-ship-gate-completeness should pass once the producer gate is enrolled: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
        .expect("completeness emits one JSON object");
    let found = json["found"]
        .as_array()
        .expect("found is the v1.0-ship-gate needs array");
    let enrolled = found
        .iter()
        .any(|v| v.as_str() == Some("check-trial-attestation"));
    assert!(
        enrolled,
        "check-trial-attestation must be enrolled in the v1.0-ship-gate needs array (found: {:?})",
        found
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2 / P2 — the "independently recomputed cargo tree" is a REAL subprocess
// ═══════════════════════════════════════════════════════════════════════════

/// P2: `recompute_cargo_tree_locked` is the "independently recomputed
/// `cargo tree --locked`" ground truth the §A7 `sbom-completeness-derived` leg
/// names. It must be a REAL subprocess (not dead code, not a hand-written
/// literal) so the F4 two-source tautology guard holds end-to-end. Exercise it
/// against a real workspace crate and confirm it returns a non-empty closure.
#[test]
fn recompute_cargo_tree_locked_runs_the_real_cargo_tree_subprocess() {
    let tree = recompute_cargo_tree_locked("maos-eval")
        .expect("cargo tree --locked -p maos-eval must succeed in the workspace");
    assert!(
        tree.contains("maos-eval"),
        "the recomputed closure must name the queried crate: {tree}"
    );
    assert!(!tree.trim().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2 / P4 — self-report-ignored derives signing from a TAMPERED artifact
// ═══════════════════════════════════════════════════════════════════════════

/// P4: the "self-report ignored" discipline must derive `signing_chain_verified`
/// from a TAMPERED signature via the real `derive_signing_chain` (not a
/// hand-constructed `SigningDerivation`), then feed a forged
/// `signing_chain_verified = true` self-report. The derived `false` must win and
/// the forgery must be flagged ignored — so a `derive_signing_chain` that always
/// returned `true` could not slip past.
#[test]
fn self_report_ignored_when_real_signing_derivation_contradicts_forged_self_report() {
    let artifact = b"candidate";
    let manifest = generate_sha256sums(&[("candidate.bin".to_string(), sha256_hex(artifact))]);
    let good_sig = sign_sha256sums(manifest.as_bytes(), &dev_seed());
    let mut tampered = good_sig;
    tampered[0] ^= 0x55;
    let signing = derive_signing_chain(
        manifest.as_bytes(),
        &tampered,
        &[("candidate.bin", artifact)],
    );
    assert!(
        !signing.signing_chain_verified,
        "tampered sig must derive false at the real Ed25519 verify path"
    );

    let hermeticity = HermeticityReport {
        clean: true,
        checked_paths: Vec::new(),
        dirty_paths: Vec::new(),
    };
    let reload = derive_reload_facts(ReloadExecutionReport {
        load_accepted: true,
        frames_executed: 1_000,
    });
    let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), "maos v0.1.0\n");
    let halt = derive_halt_recall(
        HaltRecallCounts {
            true_positives: 17,
            false_negatives: 3,
        },
        b"class-appropriate-corpus",
        true,
    );
    let record = derive_participant_attestation(DerivationInputs {
        participant_id: "p1",
        produced_binary: true,
        artifact_bytes: artifact,
        hermeticity: &hermeticity,
        reload,
        sbom: &sbom,
        signing: &signing,
        halt: &halt,
        reported: Some(ReportedParticipantFacts {
            binary_loads: true,
            frames_run: 1_000,
            halt_recall: 0.9,
            sbom_verified: true,
            signing_chain_verified: true,
        }),
    });
    assert!(
        !record.signing_chain_verified,
        "derived signing_chain_verified=false must win over the forged self-report"
    );
    assert!(!record.success);
    assert!(
        record.ignored_self_report,
        "the forged self-report must be flagged ignored"
    );
}
