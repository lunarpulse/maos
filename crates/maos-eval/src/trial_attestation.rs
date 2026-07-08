#![forbid(unsafe_code)]

//! Story 11.7 — v2.0 third-party trial attestation derivation.
//!
//! This module owns the machine-derived facts that graduate the Story 10.2
//! hand-authored trial-results file into a v2.0 attestation seam. Inputs are
//! candidate artifacts and execution reports; outputs are provenance-stamped
//! records. Self-reported values are accepted only as contradiction probes —
//! they never decide the result.

use maos_audit::release_verify::{
    sign_sha256sums, verify_release, verify_release_signature, RELEASE_PUBKEY,
};
use maos_audit::sealed_export::derive_pubkey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PROVENANCE_STAMP: &str = "maos-trial-attestation-v2";
pub const PROXY_COHORT_LABEL: &str = "in-house Chinese-wall proxy";
pub const FRAME_FLOOR: i64 = 1000;
pub const HALT_RECALL_FLOOR: f64 = 0.85;

/// Relative path to the producer-emitted, producer-signed per-participant derived
/// attestations consumed by `check-third-party-trial` at v2.0. The producer
/// (`check-trial-attestation`) writes this; the consumer verifies each signature.
pub const DERIVED_ATTESTATIONS_PATH: &str = "docs/third-party-trial/results/derived-attestations.json";

// ─── Producer signing keypair (D1 — binds the seam; mirrors release_verify's
//     dev_seed/RELEASE_PUBKEY split) ──────────────────────────────────────────
// The producer bot signs each per-participant derived attestation; the consumer
// verifies against the producer pubkey. In local/test builds the dev seed is used
// (its pubkey is the default). Production CI sets BOTH `MAOS_TRIAL_PRODUCER_SEED`
// (the bot's private seed, a secret) and `MAOS_TRIAL_PRODUCER_PUBKEY` (its pubkey)
// so a hand-authored attestation cannot be forged without the producer key.

fn parse_hex_32(hex: &str) -> Option<[u8; 32]> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = match bytes[i * 2] {
            b'0'..=b'9' => bytes[i * 2] - b'0',
            b'a'..=b'f' => bytes[i * 2] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2] - b'A' + 10,
            _ => return None,
        };
        let lo = match bytes[i * 2 + 1] {
            b'0'..=b'9' => bytes[i * 2 + 1] - b'0',
            b'a'..=b'f' => bytes[i * 2 + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2 + 1] - b'A' + 10,
            _ => return None,
        };
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

/// Documented development producer seed (local/test builds only). Distinct from
/// the release `dev_seed()` so trial-attestation signing is a separate concern.
pub fn producer_dev_seed() -> [u8; 32] {
    parse_hex_32("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        .expect("hardcoded producer dev seed is valid 64-char hex")
}
/// Producer private seed. Production CI overrides via `MAOS_TRIAL_PRODUCER_SEED`.
pub fn producer_signing_seed() -> [u8; 32] {
    std::env::var("MAOS_TRIAL_PRODUCER_SEED")
        .ok()
        .and_then(|hex| parse_hex_32(&hex))
        .unwrap_or_else(producer_dev_seed)
}

/// Trusted producer public key. Production CI overrides via
/// `MAOS_TRIAL_PRODUCER_PUBKEY` (the pubkey of its secret seed); local/test
/// builds derive the dev seed's pubkey.
pub fn producer_pubkey() -> [u8; 32] {
    std::env::var("MAOS_TRIAL_PRODUCER_PUBKEY")
        .ok()
        .and_then(|hex| parse_hex_32(&hex))
        .unwrap_or_else(|| derive_pubkey(&producer_dev_seed()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermeticEnvironment {
    pub maos_home: PathBuf,
    pub candidate_cache: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermeticityReport {
    pub clean: bool,
    pub checked_paths: Vec<PathBuf>,
    pub dirty_paths: Vec<PathBuf>,
}

impl HermeticEnvironment {
    pub fn assert_clean(&self) -> HermeticityReport {
        let checked_paths = vec![self.maos_home.clone(), self.candidate_cache.clone()];
        let dirty_paths = checked_paths
            .iter()
            .filter(|path| path_exists_with_state(path))
            .cloned()
            .collect::<Vec<_>>();
        HermeticityReport {
            clean: dirty_paths.is_empty(),
            checked_paths,
            dirty_paths,
        }
    }
}

fn path_exists_with_state(path: &Path) -> bool {
    // L4: "no prior MAOS state" is the load-bearing property. ANY existing file
    // (including a zero-byte planted marker) or symlink counts as dirty — a
    // zero-byte stale artifact must not launder a "clean VM" claim, and a
    // dangling symlink (which `metadata` would error on) must read dirty.
    // `symlink_metadata` does not follow links, so symlinks are detected as
    // themselves rather than their (possibly absent) target.
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => true,
        Ok(meta) if meta.is_file() => true,
        Ok(meta) if meta.is_dir() => true, // an existing ~/.maos dir IS prior state (even if empty)
        Ok(_) => true,
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReloadExecutionReport {
    pub load_accepted: bool,
    pub frames_executed: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReloadDerivedFacts {
    pub binary_loads: bool,
    pub frames_run: i64,
    pub meets_frame_floor: bool,
}

pub fn derive_reload_facts(report: ReloadExecutionReport) -> ReloadDerivedFacts {
    let frames_run = report.frames_executed.max(0);
    ReloadDerivedFacts {
        binary_loads: report.load_accepted,
        frames_run,
        meets_frame_floor: report.load_accepted && frames_run >= FRAME_FLOOR,
    }
}

pub fn derive_reload_facts_from_local_runner<S: maos_spirit_sdk::Spirit>(
    spirit: &S,
    vtable: &maos_spirit_sdk::SpiritVtable<S>,
    fixture: &maos_spirit_sdk::local_runner::LocalRunnerFixture,
) -> ReloadDerivedFacts {
    let report = maos_spirit_sdk::local_runner::LocalRunner::run(spirit, vtable, fixture);
    let frames_run = report
        .hooks_fired
        .get("on_frame")
        .copied()
        .unwrap_or(0)
        .into();
    derive_reload_facts(ReloadExecutionReport {
        load_accepted: report.hooks_fired.values().any(|count| *count > 0),
        frames_executed: frames_run,
    })
}

pub const DEFAULT_FORBIDDEN_CLOSURE_CRATES: &[&str] = &[
    "sqlx",
    "tokio-postgres",
    "postgres",
    "pgvector",
    "deadpool-postgres",
    "wasmtime",
    "wasmtime-wasi",
    "wit-bindgen",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomDerivation {
    pub sbom_verified: bool,
    pub declared_packages: BTreeSet<PackageId>,
    pub recomputed_packages: BTreeSet<PackageId>,
    pub missing_from_declaration: BTreeSet<PackageId>,
    pub extra_in_declaration: BTreeSet<PackageId>,
    pub closure_policy_passed: bool,
    pub forbidden_in_closure: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageId {
    pub name: String,
    pub version: String,
}

impl PackageId {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

pub fn derive_sbom_from_sources(declared_cargo_lock: &str, recomputed_cargo_tree: &str) -> SbomDerivation {
    derive_sbom_with_policy(
        declared_cargo_lock,
        recomputed_cargo_tree,
        DEFAULT_FORBIDDEN_CLOSURE_CRATES,
    )
}

pub fn derive_sbom_with_policy(
    declared_cargo_lock: &str,
    recomputed_cargo_tree: &str,
    forbidden_crates: &[&str],
) -> SbomDerivation {
    let declared_packages = parse_cargo_lock_packages(declared_cargo_lock);
    let recomputed_packages = parse_cargo_tree_packages(recomputed_cargo_tree);
    let missing_from_declaration = recomputed_packages
        .difference(&declared_packages)
        .cloned()
        .collect::<BTreeSet<_>>();
    let extra_in_declaration = declared_packages
        .difference(&recomputed_packages)
        .cloned()
        .collect::<BTreeSet<_>>();
    let forbidden_in_closure = recomputed_packages
        .iter()
        .filter(|pkg| forbidden_crates.iter().any(|forbidden| *forbidden == pkg.name))
        .map(|pkg| pkg.name.clone())
        .collect::<BTreeSet<_>>();
    let closure_policy_passed = forbidden_in_closure.is_empty();
    let sbom_verified = !declared_packages.is_empty()
        && !recomputed_packages.is_empty()
        && missing_from_declaration.is_empty()
        && extra_in_declaration.is_empty()
        && closure_policy_passed;
    SbomDerivation {
        sbom_verified,
        declared_packages,
        recomputed_packages,
        missing_from_declaration,
        extra_in_declaration,
        closure_policy_passed,
        forbidden_in_closure,
    }
}

pub fn recompute_cargo_tree_locked(package: &str) -> Result<String, String> {
    run_cargo_tree_locked(package, Duration::from_secs(120))
}

fn run_cargo_tree_locked(package: &str, timeout: Duration) -> Result<String, String> {
    let mut child = Command::new("cargo")
        .args(["tree", "--locked", "-p", package])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn cargo tree --locked -p {package}: {e}"))?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "cargo tree --locked -p {package} exceeded {timeout:?} timeout"
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("failed to wait on cargo tree: {e}")),
        }
    };
    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to read cargo tree output: {e}"))?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn parse_cargo_lock_packages(lock: &str) -> BTreeSet<PackageId> {
    // Tolerate a leading UTF-8 BOM (some editors / candidate bundles ship one);
    // a BOM on the first line would otherwise mismatch the `[[package]]` header
    // and silently drop the leading package.
    let lock = lock.strip_prefix('\u{feff}').unwrap_or(lock);
    let mut packages = BTreeSet::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    for line in lock.lines().map(str::trim) {
        if line == "[[package]]" {
            if let (Some(n), Some(v)) = (name.take(), version.take()) {
                packages.insert(PackageId::new(n, v));
            }
            name = None;
            version = None;
            continue;
        }
        if let Some(value) = line.strip_prefix("name =") {
            name = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("version =") {
            version = Some(value.trim().trim_matches('"').to_string());
        }
    }
    if let (Some(n), Some(v)) = (name, version) {
        packages.insert(PackageId::new(n, v));
    }
    packages
}

pub fn parse_cargo_tree_packages(tree: &str) -> BTreeSet<PackageId> {
    tree.lines()
        .filter_map(parse_cargo_tree_line)
        .collect::<BTreeSet<_>>()
}

fn parse_cargo_tree_line(line: &str) -> Option<PackageId> {
    // Strip the full set of cargo-tree drawing chars — both unicode (│├└─) and
    // the ASCII fallback (|--, +-, \, /) — plus whitespace. Drop feature-edge
    // lines ("maos feature \"trial-attestation\"") and lines with no version.
    // Tolerate `(*)` (path/git/extra-source) and build metadata (`+build`) so
    // those deps are still captured by name+version.
    let cleaned: String = line
        .trim_matches(|c: char| c.is_whitespace() || "│├└─|+-\\//`".contains(c))
        .to_string();
    if cleaned.is_empty() || cleaned.starts_with("feature ") {
        return None;
    }
    let mut tokens = cleaned.split_whitespace();
    let name = tokens.next()?;
    // The version is the first `vX.Y.Z`-shaped token; strip a trailing `(*)`
    // marker and ignore build metadata for reconciliation (name+version only).
    let version_token = tokens.find(|token| {
        token.starts_with('v')
            && token.len() > 1
            && token.as_bytes()[1].is_ascii_digit()
    })?;
    let version = version_token
        .trim_start_matches('v')
        .trim_end_matches("(*)")
        .split('+')
        .next()
        .unwrap_or("");
    if version.is_empty() {
        return None;
    }
    Some(PackageId::new(name, version))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningDerivation {
    pub signing_chain_verified: bool,
    pub verified_manifest_entries: usize,
    pub error: Option<String>,
}

pub fn derive_signing_chain(
    sha256sums_content: &[u8],
    sig_bytes: &[u8; 64],
    files: &[(&str, &[u8])],
) -> SigningDerivation {
    match verify_release(sha256sums_content, sig_bytes, &RELEASE_PUBKEY, files, false) {
        Ok(entries) => SigningDerivation {
            signing_chain_verified: true,
            verified_manifest_entries: entries.len(),
            error: None,
        },
        Err(err) => SigningDerivation {
            signing_chain_verified: false,
            verified_manifest_entries: 0,
            error: Some(err.to_string()),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaltRecallCounts {
    pub true_positives: usize,
    pub false_negatives: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaltRecallDerivation {
    pub halt_recall: f64,
    pub meets_floor: bool,
    /// `false` when there was nothing to measure (zero denominator / overflow).
    /// A "not measured" result must never pose as a clean pass: `meets_floor`
    /// is forced `false` and `halt_recall` is `0.0` when `measured == false`.
    pub measured: bool,
    pub corpus_sha256: String,
    pub provisional: bool,
}

pub fn derive_halt_recall(
    counts: HaltRecallCounts,
    corpus_bytes: &[u8],
    provisional: bool,
) -> HaltRecallDerivation {
    // Checked arithmetic (L10): a count surface must never wrap. A zero
    // denominator is "not measured", not a clean 0% — masked as a below-floor
    // failure via `measured=false`.
    let den = counts
        .true_positives
        .checked_add(counts.false_negatives)
        .unwrap_or(usize::MAX);
    let (halt_recall, measured) = if den == 0 {
        (0.0, false)
    } else {
        (counts.true_positives as f64 / den as f64, true)
    };
    HaltRecallDerivation {
        halt_recall,
        meets_floor: measured && halt_recall >= HALT_RECALL_FLOOR,
        measured,
        corpus_sha256: sha256_hex(corpus_bytes),
        provisional,
    }
}

pub fn derive_halt_recall_from_onboarding_score(
    corpus: &crate::onboarding_gate_corpus::OnboardingCorpus,
    resolved: &crate::onboarding_gate_corpus::ResolvedCorpus,
    input: &crate::onboarding_gate_corpus::CandidateInput,
    observations: Option<&BTreeMap<String, bool>>,
) -> HaltRecallDerivation {
    let outcome = crate::onboarding_gate_corpus::score_candidate(
        corpus,
        resolved,
        input,
        observations,
    );
    // If the class-appropriate corpus subset has zero expected-halt scenarios,
    // there is nothing to measure: score_candidate's ratio-or-one would yield a
    // vacuous 1.0 (the L5 trap). Treat that as not-measured (below floor), not a
    // clean pass.
    let cc_expected = corpus
        .scenarios
        .iter()
        .filter(|s| s.calendar_conflict && s.expected_halt)
        .count();
    let measured = cc_expected > 0;
    let halt_recall = if measured {
        outcome.halt_recall_calendar_conflict
    } else {
        0.0
    };
    HaltRecallDerivation {
        halt_recall,
        meets_floor: measured && halt_recall >= HALT_RECALL_FLOOR,
        measured,
        corpus_sha256: outcome.corpus_sha256,
        provisional: outcome.provisional,
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportedParticipantFacts {
    pub binary_loads: bool,
    pub frames_run: i64,
    pub halt_recall: f64,
    pub sbom_verified: bool,
    pub signing_chain_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedParticipantAttestation {
    pub participant_id: String,
    pub produced_binary: bool,
    pub binary_loads: bool,
    pub frames_run: i64,
    pub halt_recall: f64,
    pub sbom_verified: bool,
    pub signing_chain_verified: bool,
    pub provenance_stamp: String,
    pub environment_clean: bool,
    pub corpus_sha256: String,
    pub artifact_sha256: String,
    pub ignored_self_report: bool,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DerivationInputs<'a> {
    pub participant_id: &'a str,
    pub produced_binary: bool,
    pub artifact_bytes: &'a [u8],
    pub hermeticity: &'a HermeticityReport,
    pub reload: ReloadDerivedFacts,
    pub sbom: &'a SbomDerivation,
    pub signing: &'a SigningDerivation,
    pub halt: &'a HaltRecallDerivation,
    pub reported: Option<ReportedParticipantFacts>,
}

pub fn derive_participant_attestation(inputs: DerivationInputs<'_>) -> DerivedParticipantAttestation {
    let halt_ok = inputs.halt.measured
        && inputs.halt.halt_recall.is_finite()
        && inputs.halt.halt_recall >= HALT_RECALL_FLOOR
        && inputs.halt.halt_recall <= 1.0;
    let success = inputs.produced_binary
        && inputs.reload.binary_loads
        && inputs.reload.frames_run >= FRAME_FLOOR
        && halt_ok
        && inputs.sbom.sbom_verified
        && inputs.signing.signing_chain_verified
        && inputs.hermeticity.clean;
    let derived_facts = ReportedParticipantFacts {
        binary_loads: inputs.reload.binary_loads,
        frames_run: inputs.reload.frames_run,
        halt_recall: inputs.halt.halt_recall,
        sbom_verified: inputs.sbom.sbom_verified,
        signing_chain_verified: inputs.signing.signing_chain_verified,
    };
    let ignored_self_report = inputs
        .reported
        .as_ref()
        .map(|reported| reported != &derived_facts)
        .unwrap_or(false);
    DerivedParticipantAttestation {
        participant_id: inputs.participant_id.to_string(),
        produced_binary: inputs.produced_binary,
        binary_loads: inputs.reload.binary_loads,
        frames_run: inputs.reload.frames_run,
        halt_recall: inputs.halt.halt_recall,
        sbom_verified: inputs.sbom.sbom_verified,
        signing_chain_verified: inputs.signing.signing_chain_verified,
        provenance_stamp: PROVENANCE_STAMP.to_string(),
        environment_clean: inputs.hermeticity.clean,
        corpus_sha256: inputs.halt.corpus_sha256.clone(),
        artifact_sha256: sha256_hex(inputs.artifact_bytes),
        ignored_self_report,
        success,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationReconciliation {
    pub participant_id: String,
    pub field_mismatches: Vec<String>,
    pub ignored_self_report: bool,
}

pub fn reconcile_attestation_against_report(
    record: &DerivedParticipantAttestation,
    reported: &ReportedParticipantFacts,
) -> AttestationReconciliation {
    let mut field_mismatches = Vec::new();
    if record.binary_loads != reported.binary_loads {
        field_mismatches.push("binary_loads".to_string());
    }
    if record.frames_run != reported.frames_run {
        field_mismatches.push("frames_run".to_string());
    }
    let halt_mismatch = record.halt_recall.is_nan()
        || reported.halt_recall.is_nan()
        || (record.halt_recall - reported.halt_recall).abs() > f64::EPSILON;
    if halt_mismatch {
        field_mismatches.push("halt_recall".to_string());
    }
    if record.sbom_verified != reported.sbom_verified {
        field_mismatches.push("sbom_verified".to_string());
    }
    if record.signing_chain_verified != reported.signing_chain_verified {
        field_mismatches.push("signing_chain_verified".to_string());
    }
    AttestationReconciliation {
        participant_id: record.participant_id.clone(),
        ignored_self_report: !field_mismatches.is_empty(),
        field_mismatches,
    }
}

pub fn reconcile_reported_successes(
    records: &[DerivedParticipantAttestation],
    reported_successes: i64,
) -> Result<usize, String> {
    if reported_successes < 0 {
        return Err(format!(
            "reported successes is negative ({reported_successes}) — invalid input"
        ));
    }
    let derived = records.iter().filter(|record| record.success).count();
    // L10: i64 → usize via try_from (never an `as` truncation on a count surface).
    let reported = usize::try_from(reported_successes)
        .map_err(|_| format!("reported successes={reported_successes} overflows usize"))?;
    if derived != reported {
        return Err(format!(
            "reported successes={reported_successes} does not match derived={derived}"
        ));
    }
    Ok(derived)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialAttestationSummary {
    pub participants_total: usize,
    pub derived_successes: usize,
    pub provenance_stamp: String,
}

pub fn summarize_attestations(records: &[DerivedParticipantAttestation]) -> TrialAttestationSummary {
    TrialAttestationSummary {
        participants_total: records.len(),
        derived_successes: records.iter().filter(|record| record.success).count(),
        provenance_stamp: PROVENANCE_STAMP.to_string(),
    }
}

// ─── D1: producer-signed per-participant derived attestation ─────────────────
// The seam that graduates the v2.0 consumer off hand-authored booleans. The
// producer signs the canonical bytes of a DerivedParticipantAttestation; the
// consumer verifies against `producer_pubkey()`. A coordinator without the
// producer private seed cannot forge a record the consumer accepts — closing
// the "10.2 canned-trap" (a planted lie now turns the gate RED).

/// A per-participant derived attestation plus the producer's Ed25519 signature
/// over its canonical serialization (hex, 64 bytes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedAttestation {
    pub attestation: DerivedParticipantAttestation,
    pub producer_signature_hex: String,
}

/// Deterministic canonical bytes of an attestation for signing. serde_json
/// serializes structs in declaration order; f64 round-trips through Ryu, so the
/// producer and consumer serialize the same value to identical bytes.
pub fn canonical_attestation_bytes(
    attestation: &DerivedParticipantAttestation,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(attestation)
        .map_err(|e| format!("attestation does not serialize canonically: {e}"))
}

fn signature_hex(sig: &[u8; 64]) -> String {
    sig.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_signature_hex(hex: &str) -> Option<[u8; 64]> {
    let bytes = hex.as_bytes();
    if bytes.len() != 128 {
        return None;
    }
    let mut out = [0u8; 64];
    for i in 0..64 {
        let hi = match bytes[i * 2] {
            b'0'..=b'9' => bytes[i * 2] - b'0',
            b'a'..=b'f' => bytes[i * 2] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2] - b'A' + 10,
            _ => return None,
        };
        let lo = match bytes[i * 2 + 1] {
            b'0'..=b'9' => bytes[i * 2 + 1] - b'0',
            b'a'..=b'f' => bytes[i * 2 + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2 + 1] - b'A' + 10,
            _ => return None,
        };
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

pub fn sign_attestation(attestation: &DerivedParticipantAttestation, seed: &[u8; 32]) -> SignedAttestation {
    // Serialization of a DerivedParticipantAttestation is infallible in practice
    // (all fields are finite JSON types); surface the error rather than panic.
    let bytes = canonical_attestation_bytes(attestation).expect("infallible canonical serialization");
    let sig = sign_sha256sums(&bytes, seed);
    SignedAttestation {
        attestation: attestation.clone(),
        producer_signature_hex: signature_hex(&sig),
    }
}

/// Verifies a producer-signed attestation against `pubkey`. Returns the
/// attestation only if the signature is valid over its current canonical bytes
/// (any field tampering invalidates the signature).
pub fn verify_signed_attestation(
    signed: &SignedAttestation,
    pubkey: &[u8; 32],
) -> Result<DerivedParticipantAttestation, String> {
    let sig = parse_signature_hex(&signed.producer_signature_hex)
        .ok_or_else(|| "producer signature is not 128 hex chars".to_string())?;
    let bytes = canonical_attestation_bytes(&signed.attestation)?;
    verify_release_signature(&bytes, &sig, pubkey)
        .map_err(|err| format!("producer signature verification failed: {err}"))?;
    Ok(signed.attestation.clone())
}

/// Producer-side emit: sign each derived attestation with the producer seed and
/// write the `derived-attestations.json` artifact the v2.0 consumer reads. This
/// is the real producer→consumer handoff (closes the "producer never writes the
/// file the consumer reads" gap): the consumer verifies every signature against
/// `producer_pubkey()`, so only the producer (holding the seed) can mint a record
/// the consumer accepts.
pub fn emit_signed_attestations(
    records: &[DerivedParticipantAttestation],
    seed: &[u8; 32],
    path: &Path,
) -> Result<(), String> {
    let signed: Vec<SignedAttestation> = records
        .iter()
        .map(|r| sign_attestation(r, &seed))
        .collect();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(&signed)
        .map_err(|e| format!("cannot serialize signed attestations: {e}"))?;
    std::fs::write(path, &json)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_sdk::Spirit;

    fn lock_with(packages: &[(&str, &str)]) -> String {
        packages
            .iter()
            .map(|(name, version)| format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub struct TrialReloadSpirit;

    #[maos_spirit_sdk::spirit]
    impl TrialReloadSpirit {
        fn on_frame(&self, _ctx: &mut maos_spirit_sdk::Ctx, _payload: &maos_spirit_sdk::FramePayload<'_>) {}
    }

    fn frame_fixture(frames: usize) -> maos_spirit_sdk::local_runner::LocalRunnerFixture {
        maos_spirit_sdk::local_runner::LocalRunnerFixture {
            frames: (0..frames).map(|idx| idx.to_le_bytes().to_vec()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn local_runner_reload_derives_frame_floor_from_real_hook_fires() {
        let spirit = TrialReloadSpirit;
        let green = derive_reload_facts_from_local_runner(
            &spirit,
            &__maos_spirit_vtable_TrialReloadSpirit(),
            &frame_fixture(1_000),
        );
        let red = derive_reload_facts_from_local_runner(
            &spirit,
            &__maos_spirit_vtable_TrialReloadSpirit(),
            &frame_fixture(999),
        );
        assert!(green.binary_loads);
        assert_eq!(green.frames_run, 1_000);
        assert!(green.meets_frame_floor);
        assert!(red.binary_loads);
        assert_eq!(red.frames_run, 999);
        assert!(!red.meets_frame_floor);
    }

    #[test]
    fn sbom_reconciles_candidate_lock_against_independent_tree() {
        let lock = lock_with(&[("maos", "0.1.0"), ("serde", "1.0.0")]);
        let tree = "maos v0.1.0 (/tmp/maos)\n└── serde v1.0.0\n";
        let derived = derive_sbom_from_sources(&lock, tree);
        assert!(derived.sbom_verified);
        assert!(derived.missing_from_declaration.is_empty());
    }

    #[test]
    fn sbom_mismatch_derives_false() {
        let lock = lock_with(&[("maos", "0.1.0")]);
        let tree = "maos v0.1.0 (/tmp/maos)\n└── serde v1.0.0\n";
        let derived = derive_sbom_from_sources(&lock, tree);
        assert!(!derived.sbom_verified);
        assert!(derived
            .missing_from_declaration
            .contains(&PackageId::new("serde", "1.0.0")));
    }

    #[test]
    fn sbom_closure_policy_forbidden_crate_derives_false() {
        let lock = lock_with(&[("maos", "0.1.0"), ("wasmtime", "1.0.0")]);
        let tree = "maos v0.1.0 (/tmp/maos)\n└── wasmtime v1.0.0\n";
        let derived = derive_sbom_from_sources(&lock, tree);
        assert!(!derived.sbom_verified);
        assert!(!derived.closure_policy_passed);
        assert!(derived.forbidden_in_closure.contains("wasmtime"));
    }

    fn dev_seed() -> [u8; 32] {
        let hex_str = "794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d";
        let bytes = (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("valid dev seed hex"))
            .collect::<Vec<_>>();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        seed
    }

    #[test]
    fn signing_chain_derives_from_release_verify_and_rejects_wrong_key_or_tamper() {
        let artifact = b"candidate artifact";
        let manifest = maos_audit::release_verify::generate_sha256sums(&[(
            "candidate.bin".to_string(),
            sha256_hex(artifact),
        )]);
        let good_sig = maos_audit::release_verify::sign_sha256sums(manifest.as_bytes(), &dev_seed());
        let good = derive_signing_chain(manifest.as_bytes(), &good_sig, &[("candidate.bin", artifact)]);
        assert!(good.signing_chain_verified);
        assert_eq!(good.verified_manifest_entries, 1);

        let wrong_seed = [7u8; 32];
        let wrong_key_sig =
            maos_audit::release_verify::sign_sha256sums(manifest.as_bytes(), &wrong_seed);
        let wrong = derive_signing_chain(manifest.as_bytes(), &wrong_key_sig, &[("candidate.bin", artifact)]);
        assert!(!wrong.signing_chain_verified);

        let mut tampered = good_sig;
        tampered[0] ^= 0x55;
        let bad = derive_signing_chain(manifest.as_bytes(), &tampered, &[("candidate.bin", artifact)]);
        assert!(!bad.signing_chain_verified);
    }

    fn class_appropriate_corpus(expected_halts: usize) -> crate::onboarding_gate_corpus::OnboardingCorpus {
        crate::onboarding_gate_corpus::OnboardingCorpus {
            meta: None,
            scenarios: (0..expected_halts)
                .map(|idx| crate::onboarding_gate_corpus::OnbScenario {
                    scenario_id: format!("cc-{idx}"),
                    calendar_conflict: true,
                    expected_halt: true,
                    observed_halt: true,
                })
                .collect(),
        }
    }

    #[test]
    fn halt_recall_is_measured_by_onboarding_scorer_not_hard_coded_one() {
        let corpus = class_appropriate_corpus(20);
        let resolved = crate::onboarding_gate_corpus::ResolvedCorpus {
            source: crate::onboarding_gate_corpus::CorpusSource::Fixture,
            path: "fixture.jsonl".into(),
            sha256: "corpus-sha".to_string(),
        };
        let input = crate::onboarding_gate_corpus::CandidateInput {
            participant_id: "p-halt".to_string(),
            compiles_against_abi: true,
            time_to_success_min: 12.0,
            within_window: true,
        };
        let green_obs = (0..20)
            .map(|idx| (format!("cc-{idx}"), idx < 17))
            .collect::<BTreeMap<_, _>>();
        let low_obs = (0..20)
            .map(|idx| (format!("cc-{idx}"), idx < 16))
            .collect::<BTreeMap<_, _>>();
        let green = derive_halt_recall_from_onboarding_score(
            &corpus,
            &resolved,
            &input,
            Some(&green_obs),
        );
        let low = derive_halt_recall_from_onboarding_score(
            &corpus,
            &resolved,
            &input,
            Some(&low_obs),
        );
        assert_eq!(green.halt_recall, 0.85);
        assert!(green.meets_floor);
        assert_eq!(green.corpus_sha256, "corpus-sha");
        assert!(green.provisional);
        assert_eq!(low.halt_recall, 0.8);
        assert!(!low.meets_floor);
    }

    #[test]
    fn self_report_contradiction_is_ignored_by_success_count() {
        let hermeticity = HermeticityReport {
            clean: true,
            checked_paths: vec![],
            dirty_paths: vec![],
        };
        let reload = derive_reload_facts(ReloadExecutionReport {
            load_accepted: true,
            frames_executed: 1000,
        });
        let sbom = derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), "maos v0.1.0\n");
        let signing = SigningDerivation {
            signing_chain_verified: false,
            verified_manifest_entries: 0,
            error: Some("bad signature".to_string()),
        };
        let halt = derive_halt_recall(
            HaltRecallCounts {
                true_positives: 9,
                false_negatives: 1,
            },
            b"corpus",
            true,
        );
        let record = derive_participant_attestation(DerivationInputs {
            participant_id: "p1",
            produced_binary: true,
            artifact_bytes: b"artifact",
            hermeticity: &hermeticity,
            reload,
            sbom: &sbom,
            signing: &signing,
            halt: &halt,
            reported: Some(ReportedParticipantFacts {
                binary_loads: true,
                frames_run: 1000,
                halt_recall: 0.99,
                sbom_verified: true,
                signing_chain_verified: true,
            }),
        });
        let reported = ReportedParticipantFacts {
            binary_loads: true,
            frames_run: 1000,
            halt_recall: 0.99,
            sbom_verified: true,
            signing_chain_verified: true,
        };
        let reconciliation = reconcile_attestation_against_report(&record, &reported);
        assert!(reconciliation.ignored_self_report);
        assert_eq!(
            reconciliation.field_mismatches,
            vec!["halt_recall".to_string(), "signing_chain_verified".to_string()]
        );
        assert!(reconcile_reported_successes(&[record.clone()], 1).is_err());
        assert_eq!(reconcile_reported_successes(&[record.clone()], 0).unwrap(), 0);
        assert!(record.ignored_self_report);
        assert!(!record.signing_chain_verified);
        assert!(!record.success);
    }
    #[test]
    fn fail_to_load_derives_binary_loads_false_from_real_local_runner_with_zero_hook_fires() {
        // P3 (L3): fail-to-load must DERIVE from a real LocalRunner run that fires
        // zero hooks (a candidate that produces no frame hooks cannot have loaded),
        // not from hand-setting `load_accepted: false` on the report.
        let spirit = TrialReloadSpirit;
        let vtable = __maos_spirit_vtable_TrialReloadSpirit();
        let zero_frames = derive_reload_facts_from_local_runner(&spirit, &vtable, &frame_fixture(0));
        assert!(!zero_frames.binary_loads, "zero hook fires must derive binary_loads=false");
        assert_eq!(zero_frames.frames_run, 0);
        assert!(!zero_frames.meets_frame_floor);
        // And a real 1000-frame load still derives green — the machinery is wired.
        let green = derive_reload_facts_from_local_runner(&spirit, &vtable, &frame_fixture(1_000));
        assert!(green.binary_loads && green.meets_frame_floor);
    }

    #[test]
    fn halt_recall_corpus_sha_is_computed_from_corpus_bytes_not_an_echo() {
        // P8 (L5): the corpus SHA must be a real SHA-256 of the corpus bytes, not a
        // constant string copied from an input field.
        let corpus = b"class-appropriate-corpus";
        let derived = derive_halt_recall(
            HaltRecallCounts { true_positives: 17, false_negatives: 3 },
            corpus,
            true,
        );
        assert_eq!(derived.corpus_sha256, sha256_hex(corpus));
        // A different corpus yields a different SHA (not a fixed constant).
        let other = derive_halt_recall(
            HaltRecallCounts { true_positives: 17, false_negatives: 3 },
            b"different-corpus",
            true,
        );
        assert_ne!(derived.corpus_sha256, other.corpus_sha256);
    }

    #[test]
    fn halt_recall_zero_denominator_is_not_measured_and_never_a_clean_pass() {
        // P10: zero denominator = "not measured" (measured=false, below floor), never a vacuous pass.
        let unmeasured = derive_halt_recall(
            HaltRecallCounts { true_positives: 0, false_negatives: 0 },
            b"corpus",
            true,
        );
        assert!(!unmeasured.measured);
        assert!(!unmeasured.meets_floor);
    }

    #[test]
    fn hermeticity_reds_on_zero_byte_stale_artifact_and_dangling_symlink() {
        // P7 (L4): a zero-byte planted stale file AND a dangling symlink must read dirty.
        let temp = tempfile::tempdir().unwrap();
        let zero_byte = temp.path().join("warm-marker");
        std::fs::write(&zero_byte, b"").unwrap();
        let env_zero = HermeticEnvironment {
            maos_home: zero_byte,
            candidate_cache: temp.path().join("missing-cache-1"),
        };
        let report = env_zero.assert_clean();
        assert!(!report.clean, "zero-byte stale artifact must red hermeticity");

        let dangling = temp.path().join("dangling-link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(temp.path().join("nope-target"), &dangling).unwrap();
        let env_link = HermeticEnvironment {
            maos_home: temp.path().join("missing-home-2"),
            candidate_cache: dangling,
        };
        let report_link = env_link.assert_clean();
        assert!(!report_link.clean, "dangling symlink must red hermeticity");
    }

    #[test]
    fn sbom_parser_tolerates_ascii_tree_drawing_and_build_metadata_and_lock_bom() {
        // P11: ASCII tree drawing (|-- / +-), `(*)` extra-source markers, and a
        // leading UTF-8 BOM on the candidate Cargo.lock must not drop packages.
        let lock = format!(
            "\u{feff}{}",
            lock_with(&[("maos", "0.1.0"), ("serde", "1.0.0"), ("tokio", "1.0.0")])
        );
        let tree = "maos v0.1.0 (/tmp/maos)\n|-- serde v1.0.0\n`-- tokio v1.0.0 (*)\n";
        let derived = derive_sbom_from_sources(&lock, tree);
        assert!(derived.sbom_verified, "ASCII tree + BOM lock must reconcile cleanly");
    }

    #[test]
    fn producer_signed_attestation_round_trips_and_rejects_tamper() {
        // D1: the producer signature binds the derived facts. A valid signature
        // verifies; a tampered field invalidates it; a wrong-pubkey rejects it.
        let hermeticity = HermeticityReport {
            clean: true,
            checked_paths: vec![],
            dirty_paths: vec![],
        };
        let record = derive_participant_attestation(DerivationInputs {
            participant_id: "proxy-1",
            produced_binary: true,
            artifact_bytes: b"candidate",
            hermeticity: &hermeticity,
            reload: derive_reload_facts(ReloadExecutionReport { load_accepted: true, frames_executed: 1_000 }),
            sbom: &derive_sbom_from_sources(&lock_with(&[("maos", "0.1.0")]), "maos v0.1.0\n"),
            signing: &SigningDerivation { signing_chain_verified: true, verified_manifest_entries: 1, error: None },
            halt: &derive_halt_recall(HaltRecallCounts { true_positives: 17, false_negatives: 3 }, b"corpus", true),
            reported: None,
        });
        let signed = sign_attestation(&record, &producer_signing_seed());
        // Valid signature verifies against the producer pubkey.
        assert!(verify_signed_attestation(&signed, &producer_pubkey()).is_ok());
        // Tampering any field invalidates the signature.
        let mut tampered = signed.clone();
        tampered.attestation.sbom_verified = false;
        assert!(verify_signed_attestation(&tampered, &producer_pubkey()).is_err());
        // A wrong (non-producer) pubkey rejects even a valid signature.
        let wrong_pubkey = derive_pubkey(&[9u8; 32]);
        assert!(verify_signed_attestation(&signed, &wrong_pubkey).is_err());
        // The dev seed's pubkey equals the default producer pubkey (env unset).
        assert_eq!(derive_pubkey(&producer_dev_seed()), producer_pubkey());
    }

    #[test]
    fn cargo_tree_version_parser_handles_feature_lines_and_build_metadata() {
        // P11: feature-edge lines are dropped; build metadata is stripped to name+version.
        assert!(parse_cargo_tree_line("maos feature \"trial-attestation\"").is_none());
        // Isolate the `feature ` filter: after drawing-char strip this line is
        // `feature serde v1.0.0` — it CARRIES a valid v1.0.0 token, so without the
        // filter it would parse Some(name="feature"). Only the filter forces None.
        assert!(parse_cargo_tree_line("└── feature serde v1.0.0").is_none());
        let pkg = parse_cargo_tree_line("└── serde v1.0.0+build").unwrap();
        assert_eq!(pkg, PackageId::new("serde", "1.0.0"));
    }
}
