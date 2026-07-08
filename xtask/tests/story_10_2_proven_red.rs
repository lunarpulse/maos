//! Story 10.2 — proven-red tests for third-party trial gate (AC1),
//! cross-form equivalence gate (AC2), adversarial red-team gate (AC3),
//! and ship-gate-completeness update (AC4).
//!
//! Per Epic 9 §A1: proven-red as dev-pass gate.

use std::io::Write;

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn run_in_tempdir(
    subcommand: &str,
    fixture_setup: impl FnOnce(&std::path::Path),
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    fixture_setup(dir.path());
    std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([subcommand, "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask")
}

fn write_file(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// #11: the cross-form gate reads ADR-040 frontmatter at runtime to determine scope.
/// Tests that exercise the cross-form gate must supply a valid ADR-040 fixture.
fn write_adr_040_fixture(root: &std::path::Path) {
    write_file(root, "docs/adr/ADR-040-rust-inproc-measurement-gate-v05-decision.md",
        "---\nStatus: accepted\n---\n\n# ADR-040: Rust In-Proc Measurement Gate v0.5 Decision\n\nAccepted.\n");
}

// ═══════════════════════════════════════════════════════════════════
// Task 1.7: Third-Party Trial Gate — 5 proven-red vectors
// ═══════════════════════════════════════════════════════════════════

fn make_trial(successes: i64, strata: [i64; 5], participants: &str) -> String {
    format!(
        r#"[trial]
participants_total = 12
successes = {successes}
trial_start = "2026-06-01"
trial_end = "2026-06-14"
methodology_version = "1.0"
no_prior_contribution = {}
no_rust_spirit = {}
no_rust = {}
non_english = {}
offline_only = {}

{participants}"#,
        strata[0], strata[1], strata[2], strata[3], strata[4]
    )
}

use maos_audit::sealed_export::derive_pubkey;
use maos_eval::trial_attestation::{
    sign_attestation, DerivedParticipantAttestation, PROVENANCE_STAMP,
};

/// A FOREIGN producer keypair for the v2.0 tests (R2-D2): the consumer refuses
/// the public dev key at v2.0, so the tests sign with this test seed and set its
/// pubkey via MAOS_TRIAL_PRODUCER_PUBKEY on the consumer subprocess — modeling the
/// production config (CI sets a real, non-dev keypair) without env-mutating races.
const TEST_PRODUCER_SEED: [u8; 32] = [0x42; 32];
fn test_producer_pubkey_hex() -> String {
    let pk = derive_pubkey(&TEST_PRODUCER_SEED);
    pk.iter().map(|b| format!("{b:02x}")).collect()
}

/// The 12-participant cohort declared in `VALID_PARTICIPANTS` (10 green + 2
/// non-producers P011/P012 → `successes=10`).
const COHORT_IDS: &[&str] = &[
    "P001", "P002", "P003", "P004", "P005", "P006", "P007", "P008", "P009", "P010", "P011", "P012",
];

fn green_attestation(participant_id: &str) -> DerivedParticipantAttestation {
    DerivedParticipantAttestation {
        participant_id: participant_id.to_string(),
        produced_binary: true,
        binary_loads: true,
        frames_run: 1500,
        halt_recall: 0.92,
        sbom_verified: true,
        signing_chain_verified: true,
        provenance_stamp: PROVENANCE_STAMP.to_string(),
        environment_clean: true,
        corpus_sha256: "class-appropriate-corpus-sha".to_string(),
        artifact_sha256: "candidate-artifact-sha".to_string(),
        ignored_self_report: false,
        success: true,
    }
}

/// Build producer-SIGNED per-participant derived attestations for the cohort,
/// matching `successes=10` (10 green + P011/P012 non-producers). When
/// `red_sbom_id` is set, that participant's derived `sbom_verified=false`
/// (a SBOM the producer genuinely failed to reconcile) — the P5 negative
/// control: a coordinator's self-reported `sbom_verified=true` for that
/// participant in the TOML must be IGNORED (the signed derivation wins).
fn cohort_signed_attestations_json(red_sbom_id: Option<&str>, seed: &[u8; 32]) -> String {
    let signed: Vec<_> = COHORT_IDS
        .iter()
        .map(|id| {
            let mut att = if *id == "P011" || *id == "P012" {
                let mut a = green_attestation(id);
                a.produced_binary = false;
                a.binary_loads = false;
                a.frames_run = 0;
                a.halt_recall = 0.0;
                a.sbom_verified = false;
                a.signing_chain_verified = false;
                a.success = false;
                a
            } else {
                green_attestation(id)
            };
            if Some(*id) == red_sbom_id {
                att.sbom_verified = false;
                att.success = false;
            }
            sign_attestation(&att, &seed)
        })
        .collect();
    serde_json::to_string(&signed).unwrap()
}

const VALID_PARTICIPANTS: &str = r#"
[[participant]]
id = "P001"
stratum = ["no_prior_contribution"]
produced_binary = true
binary_loads = true
frames_run = 1500
halt_recall = 0.92
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P002"
stratum = ["no_prior_contribution"]
produced_binary = true
binary_loads = true
frames_run = 1200
halt_recall = 0.88
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P003"
stratum = ["no_prior_contribution"]
produced_binary = true
binary_loads = true
frames_run = 2000
halt_recall = 0.95
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P004"
stratum = ["no_prior_contribution"]
produced_binary = true
binary_loads = true
frames_run = 1100
halt_recall = 0.90
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P005"
stratum = ["no_rust_spirit"]
produced_binary = true
binary_loads = true
frames_run = 1800
halt_recall = 0.91
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P006"
stratum = ["no_rust_spirit"]
produced_binary = true
binary_loads = true
frames_run = 1300
halt_recall = 0.87
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P007"
stratum = ["no_rust_spirit"]
produced_binary = true
binary_loads = true
frames_run = 1600
halt_recall = 0.93
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P008"
stratum = ["no_rust"]
produced_binary = true
binary_loads = true
frames_run = 1400
halt_recall = 0.89
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P009"
stratum = ["no_rust"]
produced_binary = true
binary_loads = true
frames_run = 1700
halt_recall = 0.94
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P010"
stratum = ["non_english"]
produced_binary = true
binary_loads = true
frames_run = 1250
halt_recall = 0.86
sbom_verified = true
signing_chain_verified = true

[[participant]]
id = "P011"
stratum = ["non_english"]
produced_binary = false
binary_loads = false
frames_run = 0
halt_recall = 0.0
sbom_verified = false
signing_chain_verified = false

[[participant]]
id = "P012"
stratum = ["offline_only"]
produced_binary = false
binary_loads = false
frames_run = 0
halt_recall = 0.0
sbom_verified = false
signing_chain_verified = false
"#;

/// Vector (a): successes = 8 (below floor of 10) → fail.
#[test]
fn trial_gate_fails_on_low_successes() {
    let out = run_in_tempdir("check-third-party-trial", |root| {
        write_file(
            root,
            "docs/third-party-trial/results/trial-results.toml",
            &make_trial(8, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
        );
    });
    assert!(!out.status.success(), "gate should fail with successes=8");
}

/// Vector (b): no_prior_contribution = 3 (below ≥4) → fail.
#[test]
fn trial_gate_fails_on_low_stratification() {
    let out = run_in_tempdir("check-third-party-trial", |root| {
        write_file(
            root,
            "docs/third-party-trial/results/trial-results.toml",
            &make_trial(10, [3, 3, 2, 2, 1], VALID_PARTICIPANTS),
        );
    });
    assert!(
        !out.status.success(),
        "gate should fail with no_prior_contribution=3"
    );
}

/// Vector (c): all valid, successes = 10, all strata met → pass.
#[test]
fn trial_gate_passes_on_valid_results() {
    let out = run_in_tempdir("check-third-party-trial", |root| {
        write_file(
            root,
            "docs/third-party-trial/results/trial-results.toml",
            &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
        );
    });
    assert!(
        out.status.success(),
        "gate should pass with valid results: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Story 11.7 consumer graduation (D1): a perfect hand-authored file with NO
/// producer-signed derived attestations is rejected at v2.0 — a coordinator
/// cannot `vim` their way to green (the bare provenance stamp no longer suffices).
#[test]
fn trial_gate_rejects_missing_producer_signed_attestation_at_v2_0() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    // NOTE: deliberately NO derived-attestations.json — the honor-system stamp
    // ([derivation_provenance] stamp=...) is no longer accepted at v2.0.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"]).env("MAOS_SHIP_PHASE", "v2_0").env("MAOS_TRIAL_PRODUCER_PUBKEY", &test_producer_pubkey_hex())
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        !out.status.success(),
        "v2.0 consumer must reject records lacking producer-signed attestations"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("missing producer-signed derived attestation"),
        "stderr should name the producer-signature provenance failure: {stderr}"
    );
}
/// Story 11.7 consumer graduation (D1, green-half): the cohort carries
/// producer-SIGNED per-participant derived attestations (10 green + 2
/// non-producers, matching `successes=10`) → the consumer verifies every
/// signature against the producer pubkey and ACCEPTS at v2.0.
#[test]
fn trial_gate_accepts_valid_record_with_producer_signed_attestation_at_v2_0() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    write_file(
        dir.path(),
        "docs/third-party-trial/results/derived-attestations.json",
        &cohort_signed_attestations_json(None, &TEST_PRODUCER_SEED),
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"]).env("MAOS_SHIP_PHASE", "v2_0").env("MAOS_TRIAL_PRODUCER_PUBKEY", &test_producer_pubkey_hex())
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        out.status.success(),
        "a producer-signed valid cohort must pass at v2.0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// P5 / D7-fold negative control: a participant (P005) ships a producer-signed
/// attestation whose derived `sbom_verified=false`, while the hand-authored TOML
/// self-reports `sbom_verified=true` for that same participant. The consumer
/// must IGNORE the self-report, count P005 a non-success (derived_successes=9 ≠
/// reported 10), and FAIL. This proves the D7 fold actually gates and the
/// canned-trap stays closed: the self-reported boolean cannot override the
/// signed derivation.
#[test]
fn trial_gate_rejects_when_signed_sbom_false_overrides_self_reported_true_at_v2_0() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    write_file(
        dir.path(),
        "docs/third-party-trial/results/derived-attestations.json",
        &cohort_signed_attestations_json(Some("P005"), &TEST_PRODUCER_SEED),
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"]).env("MAOS_SHIP_PHASE", "v2_0").env("MAOS_TRIAL_PRODUCER_PUBKEY", &test_producer_pubkey_hex())
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        !out.status.success(),
        "v2.0 must reject when the signed sbom_verified=false overrides the self-reported true"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("sbom_verified=false") || stderr.contains("does not match"),
        "stderr should name the derived-SBOM failure or the count mismatch: {stderr}"
    );
}

/// D1 tamper control: a derived-attestations.json whose signature does NOT verify
/// against the producer pubkey (tampered after signing) is rejected as if unsigned.
#[test]
fn trial_gate_rejects_tampered_producer_signature_at_v2_0() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    // Tamper one signature nibble after serialization (safe byte round-trip).
    let json = cohort_signed_attestations_json(None, &TEST_PRODUCER_SEED);
    let sig_marker = "\"producer_signature_hex\":\"";
    let mut bytes = json.into_bytes();
    if let Some(pos) = (0..bytes.len().saturating_sub(sig_marker.len()))
        .find(|&i| &bytes[i..i + sig_marker.len()] == sig_marker.as_bytes())
    {
        let sig_start = pos + sig_marker.len();
        bytes[sig_start] = if bytes[sig_start] == b'0' { b'1' } else { b'0' };
    }
    let json = String::from_utf8(bytes).expect("toggling a hex nibble keeps valid UTF-8");
    write_file(dir.path(), "docs/third-party-trial/results/derived-attestations.json", &json);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"]).env("MAOS_SHIP_PHASE", "v2_0").env("MAOS_TRIAL_PRODUCER_PUBKEY", &test_producer_pubkey_hex())
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        !out.status.success(),
        "v2.0 must reject a tampered producer signature (P001 unverified → count mismatch)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("P001"),
        "the dropped participant should be P001 (its signature was tampered): {stderr}"
    );
}

/// R2-D1: `emit_signed_attestations` is NOT dead — the producer WRITES the file
/// the consumer reads, and the consumer accepts the producer-signed cohort. This
/// proves the producer→consumer handoff is wired (the producer half of the seam,
/// not just the consumer half the other tests exercise).
#[test]
fn producer_emit_writes_file_the_consumer_accepts_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    // Producer emit: derive the cohort records + sign + write derived-attestations.json.
    let cohort: Vec<DerivedParticipantAttestation> = COHORT_IDS
        .iter()
        .map(|id| {
            if *id == "P011" || *id == "P012" {
                let mut a = green_attestation(id);
                a.produced_binary = false;
                a.binary_loads = false;
                a.frames_run = 0;
                a.halt_recall = 0.0;
                a.sbom_verified = false;
                a.signing_chain_verified = false;
                a.success = false;
                a
            } else {
                green_attestation(id)
            }
        })
        .collect();
    let emit_path = dir
        .path()
        .join("docs/third-party-trial/results/derived-attestations.json");
    maos_eval::trial_attestation::emit_signed_attestations(&cohort, &TEST_PRODUCER_SEED, &emit_path)
        .expect("producer emit must succeed");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"])
        .env("MAOS_SHIP_PHASE", "v2_0")
        .env("MAOS_TRIAL_PRODUCER_PUBKEY", test_producer_pubkey_hex())
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        out.status.success(),
        "the producer-emitted signed cohort must pass the consumer at v2.0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// R2-D2 guard: at v2.0 the consumer MUST refuse the public dev producer key —
/// otherwise a coordinator with repo read could forge attestations. Only an
/// explicitly-set production key (MAOS_TRIAL_PRODUCER_PUBKEY) is trusted.
#[test]
fn trial_gate_refuses_public_dev_producer_key_at_v2_0() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "docs/third-party-trial/results/trial-results.toml",
        &make_trial(10, [4, 3, 2, 2, 1], VALID_PARTICIPANTS),
    );
    // Deliberately do NOT set MAOS_TRIAL_PRODUCER_PUBKEY → consumer would trust
    // the public dev key. It must refuse instead.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-third-party-trial", "--json"])
        .env("MAOS_SHIP_PHASE", "v2_0")
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        !out.status.success(),
        "v2.0 must refuse the public dev producer key (forgeable)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("MAOS_TRIAL_PRODUCER_PUBKEY"),
        "stderr should demand a production producer pubkey: {stderr}"
    );
}

/// Vector (d): absent → pass with advisory.
#[test]
fn trial_gate_passes_advisory_when_absent() {
    let out = run_in_tempdir("check-third-party-trial", |_root| {
        // No fixture file created.
    });
    assert!(out.status.success(), "gate should pass when absent");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["advisory"], true, "should be advisory");
}

/// Vector (e): malformed TOML → hard-fail.
#[test]
fn trial_gate_fails_on_malformed_toml() {
    let out = run_in_tempdir("check-third-party-trial", |root| {
        write_file(
            root,
            "docs/third-party-trial/results/trial-results.toml",
            "this is not valid TOML {{{{",
        );
    });
    assert!(!out.status.success(), "gate should fail on malformed TOML");
}

// ═══════════════════════════════════════════════════════════════════
// Task 2.4: Cross-Form Equivalence Gate — 3 proven-red vectors
// ═══════════════════════════════════════════════════════════════════

fn make_cross_form(p_value: f64) -> String {
    // Per-run hashes are MANDATORY (§A7 derive-and-reconcile; review finding
    // #6 default-deny): 30+30 interleaved values — cli={0,2,..58},
    // sub={1,3,..59} → U1=465 (see `cross_form_recompute_path_with_hashes`).
    // The reported u_statistic matches the recomputed value.
    let hashes_cli: Vec<String> = (0..30).map(|i| format!("{:064x}", i * 2)).collect();
    let hashes_sub: Vec<String> = (0..30).map(|i| format!("{:064x}", i * 2 + 1)).collect();
    format!(
        r#"{{
  "test_metadata": {{
    "spirit_name": "hello",
    "spirit_version": "0.1.0",
    "run_date": "2026-06-01",
    "environment": "ubuntu-24.04-x86_64",
    "cli_wrapper_runs": 30,
    "subprocess_runs": 30
  }},
  "results": {{
    "u_statistic": 465.0,
    "p_value": {p_value},
    "sample_size_cli": 30,
    "sample_size_sub": 30,
    "per_run_hashes_cli": {hashes_cli:?},
    "per_run_hashes_sub": {hashes_sub:?}
  }}
}}"#
    )
}

/// Vector (a): p_value = 0.03 (below 0.05) → advisory warns (gate still passes).
#[test]
fn cross_form_gate_warns_on_low_p_value() {
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
        write_file(
            root,
            "docs/cross-form/results/cross-form-results.json",
            &make_cross_form(0.03),
        );
    });
    // Gate always passes (advisory), but should log warning
    assert!(out.status.success(), "advisory gate should always pass");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning") || stderr.contains("WARNING") || stderr.contains("p_value"),
        "should log a warning for low p-value: {stderr}"
    );
}

/// Vector (b): valid results with p > 0.05 → advisory passes clean.
#[test]
fn cross_form_gate_passes_clean() {
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
        write_file(
            root,
            "docs/cross-form/results/cross-form-results.json",
            &make_cross_form(0.95),
        );
    });
    assert!(out.status.success(), "gate should pass with valid results");
}

/// Vector (c): absent → advisory pass with "cross-form results pending".
#[test]
fn cross_form_gate_passes_advisory_when_absent() {
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
    });
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["advisory"], true, "should be advisory");
}

/// Review finding #6 / D15b default-deny: a PRESENT artifact WITHOUT per-run
/// hashes is an unrecognized measurement — it cannot be derive-and-reconciled,
/// and a deterministic fixture routed here must not slip through the advisory
/// path. Must hard-ERROR (never advisory-green).
#[test]
fn cross_form_gate_errors_when_hashes_absent() {
    let artifact = r#"{
  "test_metadata": {
    "spirit_name": "hello", "spirit_version": "0.1.0",
    "run_date": "2026-06-01", "environment": "test",
    "cli_wrapper_runs": 30, "subprocess_runs": 30
  },
  "results": {
    "u_statistic": 450.0, "p_value": 0.95,
    "sample_size_cli": 30, "sample_size_sub": 30
  }
}"#;
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
        write_file(
            root,
            "docs/cross-form/results/cross-form-results.json",
            artifact,
        );
    });
    assert!(
        !out.status.success(),
        "a present hashless artifact must default-deny ERROR, not advisory-pass"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("per-run hashes") || stderr.contains("default-deny"),
        "error should cite the missing per-run hashes / default-deny: {stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Task 3.5: Red-Team Gate — 5 proven-red vectors
// ═══════════════════════════════════════════════════════════════════

const RED_TEAM_SHA: &str = "783d064d4bdea810785393036f90111fb734222c96fd2c221caea69753091358";

fn make_manifest() -> String {
    format!(
        r#"[corpus."red-team-640"]
sha256 = "{RED_TEAM_SHA}"
schema_version = 1
item_count = 640
valid_until = "2027-05-12"
description = "test"
"#
    )
}

fn make_red_team(
    class_scores: &[(&str, i64)],
    aggregate_detected: i64,
    unmitigated_cats: i64,
) -> String {
    let mut s = format!(
        r#"[gate]
corpus_sha256 = "{RED_TEAM_SHA}"
engagement_start = "2026-06-01"
engagement_end = "2026-06-14"
methodology_version = "1.0"

"#
    );
    for (class, detected) in class_scores {
        s.push_str(&format!(
            r#"[[class_result]]
class = "{class}"
scenarios_total = 10
detected_blocked = {detected}
unmitigated = {}
notes = "test"

"#,
            10 - detected
        ));
    }
    s.push_str(&format!(
        r#"[aggregate]
total_scenarios = 80
total_detected = {aggregate_detected}
total_unmitigated_categories = {unmitigated_cats}
"#
    ));
    s
}

fn all_classes() -> Vec<(&'static str, i64)> {
    vec![
        ("capability_confusion", 10),
        ("iac_frame_injection", 10),
        ("distillation_poisoning", 10),
        ("ledger_tampering", 10),
        ("cross_spirit_privilege_escalation", 10),
        ("resource_exhaustion", 10),
        ("side_channel_timing", 10),
        ("kernel_syscall_abuse", 10),
    ]
}

/// Vector (a): one class detected_blocked = 7 (below ≥9/10 floor) → advisory "WOULD HAVE BLOCKED SHIP".
#[test]
fn red_team_gate_logs_would_block_on_low_class() {
    let mut classes = all_classes();
    classes[0].1 = 7; // capability_confusion below floor
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &make_red_team(&classes, 77, 0),
        );
    });
    // Gate is advisory — should still pass
    assert!(
        out.status.success(),
        "advisory gate should pass even with threshold failure"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["threshold_met"], false, "threshold should not be met");
}

/// Vector (b): one class detected_blocked = 0 (unmitigated category) → advisory "WOULD HAVE BLOCKED SHIP".
#[test]
fn red_team_gate_logs_would_block_on_unmitigated() {
    let mut classes = all_classes();
    classes[0].1 = 0; // capability_confusion completely unmitigated
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &make_red_team(&classes, 70, 1),
        );
    });
    assert!(out.status.success(), "advisory gate should pass");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["threshold_met"], false, "threshold should not be met");
}

/// Vector (c): 5 classes below per-class floor (8/10) → advisory "WOULD HAVE BLOCKED SHIP".
/// Note: aggregate floor (72) = 8 × per-class floor (9), so the two are mathematically
/// coupled — they cannot fail independently. This vector exercises per-class threshold
/// failure + aggregate cross-validation (sum(detected) must match aggregate.total_detected).
#[test]
fn red_team_gate_logs_would_block_on_low_classes() {
    let mut classes = all_classes();
    classes[0].1 = 8;
    classes[1].1 = 8;
    classes[2].1 = 8;
    classes[3].1 = 8;
    classes[4].1 = 8;
    // sum(detected) = 8+8+8+8+8+10+10+10 = 70; aggregate.total_detected = 70 (cross-validates)
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &make_red_team(&classes, 70, 0),
        );
    });
    assert!(out.status.success(), "advisory gate should pass");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["threshold_met"], false, "threshold should not be met");
}

/// Vector (d): all thresholds met → pass clean.
#[test]
fn red_team_gate_passes_clean() {
    let classes = all_classes();
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &make_red_team(&classes, 80, 0),
        );
    });
    assert!(
        out.status.success(),
        "gate should pass with all thresholds met: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["threshold_met"], true, "threshold should be met");
}

/// Vector (e): absent → pass with advisory.
#[test]
fn red_team_gate_passes_advisory_when_absent() {
    let out = run_in_tempdir("check-red-team-gate", |root| {
        // Need MANIFEST for the gate even when results absent
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
    });
    assert!(out.status.success(), "gate should pass when absent");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["advisory"], true, "should be advisory");
}

// ═══════════════════════════════════════════════════════════════════
// Task 4.4: Ship-Gate Completeness — proven-red
// ═══════════════════════════════════════════════════════════════════

/// Remove one new gate from discipline.yml copy → completeness check must fail;
/// verified by checking that the REAL run passes (done inline in the gate run above).
#[test]
fn ship_gate_completeness_passes_with_all_gates() {
    // Run against the real repo — all 8 gates should be present.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-ship-gate-completeness", "--json"])
        .current_dir(workspace_root)
        .output()
        .expect("failed to run xtask");
    assert!(
        out.status.success(),
        "completeness check should pass with all gates present: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a discipline.yml missing one gate → completeness must fail.
#[test]
fn ship_gate_completeness_fails_with_missing_gate() {
    let dir = tempfile::tempdir().unwrap();
    let workflows_dir = dir.path().join(".github/workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();

    // Write a discipline.yml with v1-0-ship-gate that's MISSING check-red-team-gate.
    let yml = r#"
jobs:
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
      - check-breaking-md
      - check-pentest-gate
      - check-third-party-trial
      - check-cross-form-equiv
    if: always()
    steps:
      - name: Check
        run: echo done
"#;
    write_file(dir.path(), ".github/workflows/discipline.yml", yml);

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-ship-gate-completeness", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");
    assert!(
        !out.status.success(),
        "completeness check should fail when check-red-team-gate is missing"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Story 10.2 re-review: additional proven-red vectors for new enforcement
// ═══════════════════════════════════════════════════════════════════

/// #9: cross-form U-test recomputation path — supply 30+30 hashes, verify consistency_ok.
#[test]
fn cross_form_recompute_path_with_hashes() {
    // Interleaved hash pattern: cli gets even-indexed, sub gets odd-indexed.
    // This produces a mid-range U (not 0 or n1*n2) so the recompute exercises
    // real ranking, and U ≈ 450 for n=30 with near-balanced interleaving.
    let hashes_cli: Vec<String> = (0..30).map(|i| format!("{:064x}", i * 2)).collect();
    let hashes_sub: Vec<String> = (0..30).map(|i| format!("{:064x}", i * 2 + 1)).collect();
    // With fully interleaved values (0,2,4..58 vs 1,3,5..59), every cli value
    // ranks just below its sub neighbor. R1 ≈ sum of cli ranks. For n=30:
    // cli ranks at positions 1,3,5..59 → R1 = sum of odd numbers 1..59 = 900.
    // U1 = 30*30 + 30*31/2 - 900 = 900 + 465 - 900 = 465.
    let expected_u = 465.0_f64;
    let artifact = format!(
        r#"{{
  "test_metadata": {{
    "spirit_name": "hello", "spirit_version": "0.1.0",
    "run_date": "2026-06-01", "environment": "test",
    "cli_wrapper_runs": 30, "subprocess_runs": 30
  }},
  "results": {{
    "u_statistic": {expected_u}, "p_value": 0.95,
    "sample_size_cli": 30, "sample_size_sub": 30,
    "per_run_hashes_cli": {hashes_cli:?},
    "per_run_hashes_sub": {hashes_sub:?}
  }}
}}"#
    );
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
        write_file(
            root,
            "docs/cross-form/results/cross-form-results.json",
            &artifact,
        );
    });
    assert!(
        out.status.success(),
        "gate should pass with consistent hashes: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        json["u_statistic_recomputed"].is_number(),
        "recompute should produce a U value"
    );
    assert_eq!(
        json["consistency_ok"], true,
        "recomputed U should match reported within tolerance"
    );
}

/// #19: corpus_sha256 mismatch → hard-fail (provenance enforcement).
#[test]
fn red_team_gate_fails_on_corpus_sha_mismatch() {
    let classes = all_classes();
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        // Results SHA differs from manifest by one byte
        let mut bad_sha = RED_TEAM_SHA.to_string();
        bad_sha.replace_range(0..1, "0");
        let results = format!(
            r#"[gate]
corpus_sha256 = "{bad_sha}"
engagement_start = "2026-06-01"
engagement_end = "2026-06-14"
methodology_version = "1.0"

{}"#,
            make_red_team(&classes, 80, 0)
                .splitn(2, "\n\n")
                .nth(1)
                .unwrap_or("")
        );
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &results,
        );
    });
    assert!(
        !out.status.success(),
        "gate should hard-fail on corpus SHA mismatch"
    );
}

/// #22: malformed JSON → hard-fail (cross-form).
#[test]
fn cross_form_gate_fails_on_malformed_json() {
    let out = run_in_tempdir("check-cross-form-equiv", |root| {
        write_adr_040_fixture(root);
        write_file(
            root,
            "docs/cross-form/results/cross-form-results.json",
            "{{{not valid JSON",
        );
    });
    assert!(!out.status.success(), "gate should fail on malformed JSON");
}

/// #22: malformed TOML → hard-fail (red-team).
#[test]
fn red_team_gate_fails_on_malformed_toml() {
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            "this is not valid TOML {{{{",
        );
    });
    assert!(!out.status.success(), "gate should fail on malformed TOML");
}

/// #1 pin: trial gate with empty participant array + successes=12 → hard-fail (derive-from-detail).
#[test]
fn trial_gate_fails_on_empty_participants_with_fabricated_successes() {
    let out = run_in_tempdir("check-third-party-trial", |root| {
        write_file(
            root,
            "docs/third-party-trial/results/trial-results.toml",
            &make_trial(12, [4, 3, 2, 2, 1], ""),
        ); // empty participants, successes=12
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail: fabricated successes with no participant records"
    );
}

/// #3 pin: red-team gate with 8 identical class names → hard-fail (canonical enforcement).
#[test]
fn red_team_gate_fails_on_duplicate_classes() {
    let fake_classes: Vec<(&str, i64)> = (0..8).map(|_| ("resource_exhaustion", 10)).collect();
    let out = run_in_tempdir("check-red-team-gate", |root| {
        write_file(root, "tests/corpora/MANIFEST.toml", &make_manifest());
        write_file(
            root,
            "docs/red-team/results/red-team-results.toml",
            &make_red_team(&fake_classes, 80, 0),
        );
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail: 8 identical class names (not distinct canonical)"
    );
}
