#![forbid(unsafe_code)]

//! Proven-red vectors for `check-j1-two-host-signed-run` (story `j1-crosshost-2c`).
//!
//! Every vector runs the actual xtask binary against a fixture tree rooted at its
//! tempdir. The gate reads `root.join(rel)` throughout, so the fixtures are
//! complete enough for every leg to pass BEFORE one fact is removed — and then the
//! gate must go RED under the leg that owns that fact.
//!
//! A gate with no proven red is a linter that has never been observed rejecting
//! anything. The most important vector here is
//! `an_extra_field_in_a_bundle_reds_the_schema_leg`: AC2.6's binding acceptance
//! test. Wiring a schema without it produces a validator that validates nothing.

use std::io::Write;
use std::path::Path;

const SUBCOMMANDS_RS: &str = "crates/maos-cli/src/subcommands.rs";
const CLI_RS: &str = "crates/maos-cli/src/cli.rs";
const SEALED_EXPORT_RS: &str = "crates/maos-audit/src/sealed_export.rs";
const AUDIT_MANIFEST: &str = "crates/maos-audit/Cargo.toml";
const ROUTER_RS: &str = "crates/maos-a2a-core/src/router.rs";
const COHORT_RS: &str = "crates/maos-a2a-core/src/cohort.rs";
const TRANSPORT_RS: &str = "crates/maos-a2a-tcp/src/transport.rs";
const A2A_TCP_MANIFEST: &str = "crates/maos-a2a-tcp/Cargo.toml";
const COHORT_STATE_RS: &str = "crates/maos-cohort/src/state.rs";
const REDACTION_RS: &str = "crates/maos-iac/src/adapter/redaction.rs";
const BUNDLE_SCHEMA: &str = "schemas/audit-bundle.schema.json";
const WORKFLOW: &str = ".github/workflows/discipline.yml";
const DEMO_J1_RS: &str = "xtask/src/demo_j1.rs";

const CAPTURE_DIR: &str = "_bmad-output/test-artifacts/j1-two-host-evidence";
const CAPTURE: &str = "_bmad-output/test-artifacts/j1-two-host-evidence/two-host-capture.json";
const BUNDLE_A: &str = "_bmad-output/test-artifacts/j1-two-host-evidence/host-a-bundle.json";
const BUNDLE_B: &str = "_bmad-output/test-artifacts/j1-two-host-evidence/host-b-bundle.json";

const CLAIM_SCOPE: &str =
    "two keyed identities signed; not two machines, two processes, or two operators";

/// Derived test targets the fixture tree carries. The gate DERIVES this set from
/// the directories, so the fixture must supply both naming conventions.
const TEST_FILES: &[(&str, &str)] = &[
    ("crates/maos-cli/tests", "signing_identity_2c.rs"),
    ("crates/maos-cli/tests", "two_host_reconcile_2c.rs"),
    ("crates/maos-cli/tests", "credential_posture_2c.rs"),
    ("crates/maos-audit/tests", "two_host_bundle_2c.rs"),
    ("crates/maos-a2a-core/tests", "fault_typing_2c.rs"),
    (
        "crates/maos-a2a-core/tests",
        "digest_reply_durability_2c.rs",
    ),
    ("crates/maos-a2a-tcp/tests", "t_2c_fault_windows.rs"),
    ("crates/maos-a2a-tcp/tests", "t_2c_pin_journal.rs"),
];

const GOOD_SUBCOMMANDS: &str = "\
fn audit_sealed_export() {
    let region_home = match resolve_region_home() { Ok(r) => r, Err(e) => return };
    let pubkey = match &region_home { Some(r) => derive_region_pubkey(&seed, r), None => derive_pubkey(&seed) };
    eprintln!(\"maosctl: sealed export written to {} ({} entries, pubkey {})\", p, n, h);
    eprintln!(\"maosctl: sealed export written to stdout ({} entries, pubkey {})\", n, h);
}
fn audit_trajectory_export() {
    let region_home = match resolve_region_home() { Ok(r) => r, Err(e) => return };
    let pubkey = match &region_home { Some(r) => derive_region_pubkey(&seed, r), None => derive_pubkey(&seed) };
}
";

const GOOD_CLI: &str = "\
pub enum AuditQuery {
    SealedExport { host: Option<String> },
    #[command(group(clap::ArgGroup::new(\"verify_key\").required(true).args([\"pubkey\", \"seed\"])))]
    VerifyBundle { pubkey: Option<String>, seed: Option<std::path::PathBuf> },
    ReconcileHosts {},
    ScanCredentials { spirit: Option<String> },
}
";

const GOOD_SEALED_EXPORT: &str = "\
pub struct AuditBundle {
    #[serde(default, skip_serializing_if = \"Option::is_none\")]
    pub host: Option<String>,
}
pub struct BundleForSigning {
    #[serde(default, skip_serializing_if = \"Option::is_none\")]
    pub host: Option<String>,
}
impl BundleForSigning {
    pub fn with_host(mut self, host: &str) -> Self { self.host = Some(host.to_string()); self }
}
pub const TWO_HOST_CLAIM_SCOPE: &str =
    \"two keyed identities signed; not two machines, two processes, or two operators\";
pub fn sign_bundle(b: BundleForSigning) -> AuditBundle {
    AuditBundle { host: bundle_for_signing.host }
}
pub fn verify_bundle(bundle: &AuditBundle) {
    let unsigned = BundleForSigning { host: bundle.host.clone() };
}
pub fn reconcile_two_host_bundles(a: &AuditBundle, key_a: &[u8; 32], b: &AuditBundle, key_b: &[u8; 32]) -> Result<TwoHostJoin, SealedExportError> {
    if key_a == key_b {
        return Err(SealedExportError::SharedAttesterRoot);
    }
    let a_host = a.host.as_deref().ok_or(SealedExportError::MissingHostClaim)?;
    if a_host == b_host { return Err(SealedExportError::DuplicateHostClaim(a_host.to_string())); }
    let ids = a.entries.iter().map(|e| e.frame_id_hex.as_str());
    if shared.is_empty() { return Err(SealedExportError::NoSharedFrames); }
    Ok(join)
}
pub fn build_two_host_receipt() -> TwoHostRunReceipt { todo!() }
pub fn verify_two_host_receipt() -> Result<(), SealedExportError> { todo!() }
";

const GOOD_AUDIT_MANIFEST: &str = "[package]\nname = \"maos-audit\"\n\n[dependencies]\nmaos-domain = { path = \"../maos-domain\" }\n";

const GOOD_ROUTER: &str = "\
impl A2ARouterCore {
    const DIGEST_REPLY_NOT_DELIVERED: &'static str = \"intake sink full or receiver dropped — digest reply NOT delivered\";
    async fn push_to_intake_sink(&self) {}
    pub fn interpret_response(&self) {
        match code {
            CODE_INTERNAL => Err(A2AError::PeerInternalFailure { peer, message }),
            CODE_TIMEOUT => Err(A2AError::PeerIntakeTimeout { peer, message }),
            _ => Err(A2AError::TransportFailed(m)),
        }
    }
    async fn handle_intake_inner(&self) {
        if matches!(class, DigestFrameClass::Reply { .. }) {
            let observed = self.digest_read_port.observe_reply_guarded(&peer_host, frame, &mut push);
        }
        // (3) ADR-012 accept-allowlist check.
    }
}
";

const GOOD_COHORT: &str = "\
pub trait DigestReadPort: Send + Sync {
    fn observe_reply_guarded(&self, peer: &HostId, frame: &IacFrame, before_commit: &mut dyn FnMut() -> Result<(), String>) -> Result<DigestReplyObservation, String>;
}
pub enum PeerRefusalDirection { Dial, Listen }
";

const GOOD_COHORT_STATE: &str = "\
impl DigestReadPort for CohortManifestState {
    fn observe_reply_guarded(&self, peer: &HostId, frame: &IacFrame, before_commit: &mut dyn FnMut() -> Result<(), String>) -> Result<DigestReplyObservation, String> {
        before_commit()?;
        self.audit.append(&event)?;
        received.insert(key, summary);
        Ok(DigestReplyObservation::Accepted)
    }
}
";

const GOOD_TRANSPORT: &str = "\
async fn dial_once(&self) {
    let tcp = match tokio::time::timeout(partition, TcpStream::connect(addr)).await { Ok(Ok(s)) => s, _ => return };
    match tokio::time::timeout(partition, framed.send(Bytes::from(body))).await { _ => {} }
}
async fn serve_connection() {
    let tls = match acceptor.accept(tcp).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let classified = TcpTransportError::classify_handshake(&e.to_string());
            let _ = core.journal_peer_identity_refusal(PeerRefusalDirection::Listen, hint, &m).await;
            return;
        }
        Err(_) => return,
    };
    let verified = match resolve_verified_peer(&tls, &pins) {
        Some(r) => r,
        None => {
            let _ = core.journal_peer_identity_refusal(PeerRefusalDirection::Listen, hint, \"no pin\").await;
            return;
        }
    };
}
async fn route_outbound(&self) {
    let partition = Duration::from_secs(peer_cfg.partition_timeout_secs).min(self.timeouts.idle);
    return Err(A2AError::PartitionTimeout { peer, frame_id, timeout_secs });
    let _ = self.core.journal_peer_identity_refusal(PeerRefusalDirection::Dial, peer.as_str(), &m).await;
}
";

const GOOD_A2A_TCP_MANIFEST: &str = "\
[package]
name = \"maos-a2a-tcp\"

[dependencies]
maos-a2a-core = { path = \"../maos-a2a-core\" }

[dev-dependencies]
maos-kernel-core = { path = \"../maos-kernel-core\" }
";

const GOOD_REDACTION: &str = "\
static RULES: &[RedactionRule] = &[];
pub enum CredentialShape { Prefix(&'static str), HexRun { len: usize } }
pub fn scan_stored_payload(bytes: &[u8]) -> Vec<CredentialShape> {
    let mut findings = Vec::new();
    for rule in RULES { findings.push(CredentialShape::Prefix(rule.class)); }
    findings.push(CredentialShape::HexRun { len: 0 });
    findings
}
";

const GOOD_DEMO_J1: &str =
    "fn beats() { let owner = Some(\"j1-crosshost-2d-paid-two-host-run\"); }\n";

fn good_workflow() -> String {
    let mut s = String::from(
        "jobs:\n  check-j1-two-host-signed-run:\n    runs-on: ubuntu-latest\n    steps:\n",
    );
    for (dir, file) in TEST_FILES {
        let crate_name = dir.split('/').nth(1).unwrap();
        let test = file.trim_end_matches(".rs");
        s.push_str(&format!(
            "      - run: cargo test -p {crate_name} --test {test} -- --test-threads=1\n"
        ));
    }
    s.push_str("\n  check-other:\n    runs-on: ubuntu-latest\n");
    s
}

/// The corrected schema: every field the struct can emit is declared.
fn good_schema() -> String {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["schema_version", "entries", "i12_digest_refs", "i11_distilled_content", "freshness", "signature_block"],
        "properties": {
            "schema_version": { "type": "string" },
            "entries": { "type": "array", "items": { "type": "object", "properties": {
                "frame_id": { "type": "string" },
                "timestamp_ns": { "type": "integer" },
                "spirit_pid": { "type": "integer" },
                "boot_nonce": { "type": "integer" },
                "capability_token": { "type": ["string", "null"] },
                "kind": { "type": "string" },
                "intent": { "type": "string" },
                "payload": { "type": "string" },
                "redaction": { "type": "object", "properties": {}, "additionalProperties": false }
            }, "additionalProperties": false } },
            "i12_digest_refs": { "type": "array", "items": { "type": "string" } },
            "i11_distilled_content": { "type": "array", "items": { "type": "object", "properties": {
                "source_log_ref": { "type": "array", "items": { "type": "string" } },
                "distillation_depth": { "type": "integer" }
            }, "additionalProperties": false } },
            "freshness": { "type": "object", "required": ["export_timestamp_ns", "covered_window", "export_seq"], "properties": {
                "export_timestamp_ns": { "type": "integer" },
                "covered_window": { "type": "object", "properties": {
                    "since_ns": { "type": "integer" }, "until_ns": { "type": "integer" }
                }, "additionalProperties": false },
                "export_seq": { "type": "integer" }
            }, "additionalProperties": false },
            "applied_redaction": { "type": "boolean" },
            "redaction_policy": { "type": "string" },
            "region": { "type": "string" },
            "host": { "type": "string" },
            "signature_block": { "type": "object", "required": ["algorithm", "attester_pubkey", "signature"], "properties": {
                "algorithm": { "type": "string" },
                "attester_pubkey": { "type": "string" },
                "signature": { "type": "string" }
            }, "additionalProperties": false }
        },
        "additionalProperties": false
    })
    .to_string()
}

/// A schema-conformant bundle half.
fn good_bundle(host: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": "maos.audit-bundle.v1",
        "entries": [{
            "frame_id": "2c".repeat(16),
            "timestamp_ns": 1000,
            "spirit_pid": 1,
            "boot_nonce": 42,
            "kind": "task.assign",
            "intent": "dev.delegate"
        }],
        "i12_digest_refs": [],
        "i11_distilled_content": [],
        "freshness": {
            "export_timestamp_ns": 2000,
            "covered_window": { "since_ns": 0, "until_ns": 2000 },
            "export_seq": 1
        },
        "host": host,
        "signature_block": {
            "algorithm": "Ed25519",
            "attester_pubkey": "aa".repeat(32),
            "signature": "bb".repeat(64)
        }
    })
}

fn good_capture() -> serde_json::Value {
    serde_json::json!({
        "host_a": "host-a",
        "host_b": "host-b",
        "shape": "two real OS processes on one box",
        "claim_scope": CLAIM_SCOPE,
        "trust_anchor_established_out_of_band": true,
        "host_b_audit_key_provisioned_separately": true,
        "stranger_verification": "tools/verify-audit-bundle/verify.py OK"
    })
}

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn lay_green(root: &Path) {
    write_file(root, SUBCOMMANDS_RS, GOOD_SUBCOMMANDS);
    write_file(root, CLI_RS, GOOD_CLI);
    write_file(root, SEALED_EXPORT_RS, GOOD_SEALED_EXPORT);
    write_file(root, AUDIT_MANIFEST, GOOD_AUDIT_MANIFEST);
    write_file(root, ROUTER_RS, GOOD_ROUTER);
    write_file(root, COHORT_RS, GOOD_COHORT);
    write_file(root, COHORT_STATE_RS, GOOD_COHORT_STATE);
    write_file(root, TRANSPORT_RS, GOOD_TRANSPORT);
    write_file(root, A2A_TCP_MANIFEST, GOOD_A2A_TCP_MANIFEST);
    write_file(root, REDACTION_RS, GOOD_REDACTION);
    write_file(root, DEMO_J1_RS, GOOD_DEMO_J1);
    write_file(root, BUNDLE_SCHEMA, &good_schema());
    write_file(root, WORKFLOW, &good_workflow());
    for (dir, file) in TEST_FILES {
        write_file(
            root,
            &format!("{dir}/{file}"),
            "#[test]\nfn placeholder() {}\n",
        );
    }
    // No capture: ABSENT is the honest baseline, and the gate must be GREEN then.
    std::fs::create_dir_all(root.join(CAPTURE_DIR)).unwrap();
}

struct Verdict {
    passed: bool,
    stdout: String,
    success: bool,
}

fn run_gate(plant: impl FnOnce(&Path)) -> Verdict {
    let dir = tempfile::tempdir().unwrap();
    lay_green(dir.path());
    plant(dir.path());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-j1-two-host-signed-run", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("xtask must run");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Verdict {
        passed: stdout.contains("\"passed\":true") || stdout.contains("\"passed\": true"),
        stdout,
        success: output.status.success(),
    }
}

fn assert_red(verdict: &Verdict, expected_detail: &str, vector: &str) {
    assert!(
        !verdict.passed && !verdict.success,
        "the Blocking gate stayed green for `{vector}`\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict.stdout.contains(expected_detail),
        "the finding must name `{expected_detail}` for `{vector}`\nstdout:\n{}",
        verdict.stdout
    );
}

/// RF-1 — the PUBLISHED capture template must be admissible by the REAL validator.
///
/// The capture contract used to exist only inside the gate source and `good_capture()`
/// below, while the runbook told an operator to `cp two-host-capture.json` without ever
/// saying what was in it — so a paid run could be rejected AFTER the agent was billed.
/// The template is now committed at `two-host-capture.example.json`. This vector proves
/// it is not decorative: strip the template's comment key, place it where the gate looks,
/// and the real Blocking gate must accept it. If leg 9's required fields or the ratified
/// `claim_scope` ever change without the template following, this reds.
#[test]
fn published_capture_template_is_admissible_by_the_real_gate() {
    let template = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("_bmad-output/test-artifacts/j1-two-host-evidence/two-host-capture.example.json");
    let raw = std::fs::read_to_string(&template)
        .unwrap_or_else(|e| panic!("the published template must exist at {template:?}: {e}"));
    let mut doc: serde_json::Value =
        serde_json::from_str(&raw).expect("the published template must be valid JSON");
    // `_comment` explains why the file is not named `two-host-capture.json`; it is
    // documentation for a human, not part of the contract.
    doc.as_object_mut().unwrap().remove("_comment");
    // The placeholders are the two fields an operator fills in; everything the gate
    // compares byte-for-byte (claim_scope) ships already correct.
    doc["host_a"] = serde_json::json!("host-a");
    doc["host_b"] = serde_json::json!("host-b");

    let verdict = run_gate(|root| {
        write_file(root, CAPTURE, &doc.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert!(
        verdict.passed && verdict.success,
        "the PUBLISHED capture template was rejected by the real gate — an operator \
         following the runbook would be refused after the agent is billed\nstdout:\n{}",
        verdict.stdout
    );
}

#[test]
fn baseline_fixture_tree_is_green() {
    let verdict = run_gate(|_| {});
    assert!(
        verdict.passed && verdict.success,
        "the fixture baseline must be green, or every vector below is vacuous\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict
            .stdout
            .contains("\"paid_run_capture_present\":false"),
        "an absent capture must be honest and GREEN — it is the absence of a claim, \
         not a substrate gate\nstdout:\n{}",
        verdict.stdout
    );
}

// ── AC2.6's binding acceptance test ────────────────────────────────────────

/// **The vector AC2.6 named.** Plant a bundle carrying a field the schema does not
/// declare, and the gate must RED. Without this, wiring the schema produces a
/// validator that validates nothing.
#[test]
fn an_extra_field_in_a_bundle_reds_the_schema_leg() {
    let verdict = run_gate(|root| {
        let mut bundle = good_bundle("host-a");
        bundle["surprise"] = serde_json::json!("smuggled");
        write_file(root, BUNDLE_A, &bundle.to_string());
    });
    assert_red(&verdict, "not declared by the schema", "extra bundle field");
}

/// The nested case: an undeclared field inside `entries[]`. A validator that only
/// checks the top level is half a validator.
#[test]
fn an_extra_entry_field_reds_the_schema_leg() {
    let verdict = run_gate(|root| {
        let mut bundle = good_bundle("host-a");
        bundle["entries"][0]["correlation_id"] = serde_json::json!("nope");
        write_file(root, BUNDLE_A, &bundle.to_string());
    });
    assert_red(
        &verdict,
        "correlation_id: not declared by the schema",
        "extra entry field",
    );
}

/// A bundle missing a required field must RED too — `required` is the other half
/// of the schema's contract.
#[test]
fn a_bundle_missing_a_required_field_reds_the_schema_leg() {
    let verdict = run_gate(|root| {
        let mut bundle = good_bundle("host-b");
        bundle.as_object_mut().unwrap().remove("freshness");
        write_file(root, BUNDLE_B, &bundle.to_string());
    });
    assert_red(
        &verdict,
        "freshness: required but absent",
        "missing required",
    );
}

/// A schema that omits a field the struct emits is a FALSE SPECIFICATION, and the
/// gate must say so — this is the drift F14 measured.
#[test]
fn a_schema_omitting_an_emitted_field_reds() {
    for field in ["region", "applied_redaction", "redaction_policy", "host"] {
        let verdict = run_gate(|root| {
            let mut schema: serde_json::Value = serde_json::from_str(&good_schema()).unwrap();
            schema["properties"].as_object_mut().unwrap().remove(field);
            write_file(root, BUNDLE_SCHEMA, &schema.to_string());
        });
        assert_red(
            &verdict,
            "false specification",
            &format!("schema omits {field}"),
        );
    }
}

// ── AC1 ────────────────────────────────────────────────────────────────────

#[test]
fn fixing_only_one_signing_site_reds() {
    let verdict = run_gate(|root| {
        let half = GOOD_SUBCOMMANDS.replacen(
            "let region_home = match resolve_region_home()",
            "let unsigned = match resolve_region_home()",
            1,
        );
        write_file(root, SUBCOMMANDS_RS, &half);
    });
    assert_red(
        &verdict,
        "expected BOTH sealed-export sites",
        "one site only",
    );
}

#[test]
fn a_surviving_unconditional_base_key_print_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            SUBCOMMANDS_RS,
            &format!(
                "{GOOD_SUBCOMMANDS}\nfn other() {{ let pubkey = maos_audit::sealed_export::derive_pubkey(&seed); }}\n"
            ),
        );
    });
    assert_red(&verdict, "that is the P12 bug", "base key print survives");
}

#[test]
fn removing_the_stdout_pubkey_line_reds() {
    let verdict = run_gate(|root| {
        let no_stdout = GOOD_SUBCOMMANDS
            .lines()
            .filter(|l| !l.contains("written to stdout"))
            .collect::<Vec<_>>()
            .join("\n");
        write_file(root, SUBCOMMANDS_RS, &no_stdout);
    });
    assert_red(&verdict, "unverifiable artifact", "no stdout pubkey");
}

#[test]
fn removing_the_verify_derivation_path_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            CLI_RS,
            &GOOD_CLI.replace("seed: Option<std::path::PathBuf>", "unused: ()"),
        );
    });
    assert_red(
        &verdict,
        "verify-bundle needs a derivation path",
        "no --seed",
    );
}

// ── AC2.1 / AC2.4 ──────────────────────────────────────────────────────────

/// A host field only on `AuditBundle` is a LABEL, not a control: `verify_bundle`
/// would never re-canonicalize it, so a forger could rewrite it freely.
#[test]
fn a_host_field_outside_the_signature_reds() {
    let verdict = run_gate(|root| {
        let one_only = GOOD_SEALED_EXPORT.replacen(
            "pub struct BundleForSigning {\n    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub host: Option<String>,\n}",
            "pub struct BundleForSigning {}",
            1,
        );
        write_file(root, SEALED_EXPORT_RS, &one_only);
    });
    assert_red(
        &verdict,
        "is a LABEL a forger can rewrite",
        "host not signed",
    );
}

#[test]
fn dropping_the_skip_serializing_if_reds_byte_identity() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            SEALED_EXPORT_RS,
            &GOOD_SEALED_EXPORT.replace(
                "#[serde(default, skip_serializing_if = \"Option::is_none\")]",
                "#[serde(default)]",
            ),
        );
    });
    assert_red(&verdict, "byte-identically", "host always serialized");
}

/// **The AC2.4 control.** Remove the shared-root refusal and one seed holder can
/// legitimately sign both halves of a "two-host" bundle.
#[test]
fn removing_the_shared_root_refusal_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            SEALED_EXPORT_RS,
            &GOOD_SEALED_EXPORT.replace("if key_a == key_b {", "if false {"),
        );
    });
    assert_red(
        &verdict,
        "must be checked BEFORE anything else succeeds",
        "no shared-root refusal",
    );
}

#[test]
fn projecting_correlation_id_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            SEALED_EXPORT_RS,
            &format!("{GOOD_SEALED_EXPORT}\nfn join() {{ let x = e.correlation_id; }}\n"),
        );
    });
    assert_red(&verdict, "not the join key", "correlation_id projected");
}

/// `maos-audit -> maos-loom-lite` closes a CYCLE. The gate must catch the edge,
/// not just the intent.
#[test]
fn adding_the_loom_lite_edge_to_maos_audit_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            AUDIT_MANIFEST,
            &format!("{GOOD_AUDIT_MANIFEST}maos-loom-lite = {{ path = \"../maos-loom-lite\" }}\n"),
        );
    });
    assert_red(&verdict, "closes a CYCLE", "loom-lite edge");
}

/// R-RG1 — reading `attester_pubkey` inside the reconciliation lets the artifact
/// nominate the key that checks it.
#[test]
fn reading_attester_pubkey_inside_reconciliation_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            SEALED_EXPORT_RS,
            &GOOD_SEALED_EXPORT.replace(
                "let a_host = a.host.as_deref()",
                "let k = &a.signature_block.attester_pubkey;\n    let a_host = a.host.as_deref()",
            ),
        );
    });
    assert_red(&verdict, "R-RG1", "attester_pubkey trusted");
}

// ── AC3 ────────────────────────────────────────────────────────────────────

#[test]
fn untyping_either_nack_code_reds() {
    for needle in [
        "CODE_INTERNAL => Err(A2AError::PeerInternalFailure { peer, message }),",
        "CODE_TIMEOUT => Err(A2AError::PeerIntakeTimeout { peer, message }),",
    ] {
        let verdict = run_gate(|root| {
            write_file(root, ROUTER_RS, &GOOD_ROUTER.replace(needle, ""));
        });
        assert_red(
            &verdict,
            "is absent from live code",
            &format!("untyped {needle}"),
        );
    }
}

#[test]
fn unbounding_either_operation_reds() {
    for needle in [
        "tokio::time::timeout(partition, TcpStream::connect(addr))",
        "tokio::time::timeout(partition, framed.send(",
    ] {
        let verdict = run_gate(|root| {
            write_file(
                root,
                TRANSPORT_RS,
                &GOOD_TRANSPORT.replace(needle, "unbounded("),
            );
        });
        assert_red(
            &verdict,
            "is absent from live code",
            &format!("unbounded {needle}"),
        );
    }
}

#[test]
fn dropping_the_partition_config_read_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            TRANSPORT_RS,
            &GOOD_TRANSPORT.replace("peer_cfg.partition_timeout_secs", "30"),
        );
    });
    assert_red(
        &verdict,
        "§7.2 claim is not true of the wire",
        "config unread",
    );
}

/// **The AC3.5 vector.** Publishing the dedup before the commit guard is the
/// `Duplicate`-before-durable lie, one layer down from the one `2b` fixed.
#[test]
fn publishing_the_dedup_before_the_commit_guard_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            COHORT_STATE_RS,
            &GOOD_COHORT_STATE
                .replace("before_commit()?;\n        self.audit.append(&event)?;\n        received.insert(key, summary);",
                         "received.insert(key, summary);\n        before_commit()?;"),
        );
    });
    assert_red(&verdict, "publishing the dedup first", "dedup before guard");
}

#[test]
fn pushing_after_observing_in_the_reply_path_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            ROUTER_RS,
            &GOOD_ROUTER.replace(
                "let observed = self.digest_read_port.observe_reply_guarded(&peer_host, frame, &mut push);",
                "let observed = self.digest_read_port.observe_reply_guarded(&peer_host, frame, &mut push);\n            self.push_to_intake_sink(frame).await;",
            ),
        );
    });
    assert_red(
        &verdict,
        "Duplicate`-before-durable lie",
        "push after observe",
    );
}

#[test]
fn dropping_either_side_of_the_pin_journal_reds() {
    let verdict = run_gate(|root| {
        // Remove the dial-side journaling only — "both sides" must be checked.
        write_file(
            root,
            TRANSPORT_RS,
            &GOOD_TRANSPORT.replace(
                "let _ = self.core.journal_peer_identity_refusal(PeerRefusalDirection::Dial, peer.as_str(), &m).await;",
                "",
            ),
        );
    });
    assert_red(&verdict, "found 2 call(s)", "dial side not journaled");
}

#[test]
fn adding_kernel_core_to_a2a_tcp_production_deps_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            A2A_TCP_MANIFEST,
            &GOOD_A2A_TCP_MANIFEST.replace(
                "maos-a2a-core = { path = \"../maos-a2a-core\" }",
                "maos-a2a-core = { path = \"../maos-a2a-core\" }\nmaos-kernel-core = { path = \"../maos-kernel-core\" }",
            ),
        );
    });
    assert_red(&verdict, "PRODUCTION dependency", "kernel-core in a2a-tcp");
}

// ── AC4 ────────────────────────────────────────────────────────────────────

#[test]
fn removing_the_hex_run_class_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            REDACTION_RS,
            &GOOD_REDACTION.replace("findings.push(CredentialShape::HexRun { len: 0 });", ""),
        );
    });
    assert_red(&verdict, "blind to exactly the class", "prefix-only scan");
}

#[test]
fn a_scan_that_rederives_the_rules_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            REDACTION_RS,
            &GOOD_REDACTION.replace(
                "for rule in RULES { findings.push(CredentialShape::Prefix(rule.class)); }",
                "",
            ),
        );
    });
    assert_red(&verdict, "second source of truth", "scan ignores RULES");
}

// ── AC5 ────────────────────────────────────────────────────────────────────

/// A capture that overclaims must RED. The artifact may say "two identities"; it
/// may not say "two machines" unless something proved it.
#[test]
fn an_overclaiming_capture_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["shape"] = serde_json::json!("two machines in two datacentres");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert_red(
        &verdict,
        "which no control in this story proves",
        "overclaiming capture",
    );
}

/// A capture present but missing a bounded-claim field must RED: the properties
/// belong in the artifact, not in a story file nobody reads.
#[test]
fn a_capture_missing_a_human_step_property_reds() {
    for field in [
        "trust_anchor_established_out_of_band",
        "host_b_audit_key_provisioned_separately",
        "stranger_verification",
    ] {
        let verdict = run_gate(|root| {
            let mut capture = good_capture();
            capture.as_object_mut().unwrap().remove(field);
            write_file(root, CAPTURE, &capture.to_string());
            write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
            write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
        });
        assert_red(
            &verdict,
            "the capture must state the SYSTEM properties",
            field,
        );
    }
}

/// A capture claiming a two-host run whose halves are missing is a claim without
/// its artifact.
#[test]
fn a_capture_without_both_bundle_halves_reds() {
    let verdict = run_gate(|root| {
        write_file(root, CAPTURE, &good_capture().to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        // host B's half deliberately absent
    });
    assert_red(&verdict, "a claim without its artifact", "one half only");
}

/// A capture rewriting the claim scope must RED — the wording IS the bound.
#[test]
fn a_capture_with_a_rewritten_claim_scope_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["claim_scope"] = serde_json::json!("two hosts did this");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert_red(
        &verdict,
        "ratified claim scope verbatim",
        "rewritten claim scope",
    );
}

/// §A6 review 2026-08-18 (P2): a sworn property stated as FALSE is not an
/// attestation — the gate used to check only field PRESENCE, so the honest
/// default passed as the attestation itself.
#[test]
fn a_false_sworn_property_reds() {
    for field in [
        "trust_anchor_established_out_of_band",
        "host_b_audit_key_provisioned_separately",
    ] {
        let verdict = run_gate(|root| {
            let mut capture = good_capture();
            capture[field] = serde_json::json!(false);
            write_file(root, CAPTURE, &capture.to_string());
            write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
            write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
        });
        assert_red(&verdict, "must be present and TRUE", field);
    }
}

/// An empty attestation string is an omission with a key in front of it.
#[test]
fn an_empty_stranger_verification_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["stranger_verification"] = serde_json::json!("   ");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert_red(
        &verdict,
        "omits or leaves `stranger_verification` empty",
        "empty stranger verification",
    );
}

/// A capture naming one host twice is one host, whatever the keys say.
#[test]
fn a_capture_naming_one_host_twice_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["host_b"] = serde_json::json!("host-a");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-a").to_string());
    });
    assert_red(&verdict, "names one host twice", "host_a == host_b");
}

/// The capture must attest the halves it NAMES: a capture claiming alice/bob
/// over host-a/host-b bundles used to pass.
#[test]
fn a_capture_disagreeing_with_its_bundle_host_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["host_a"] = serde_json::json!("alice");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert_red(
        &verdict,
        "the capture must attest the halves it names",
        "capture/bundle host mismatch",
    );
}

/// A capture present but unparseable is worse than absent.
#[test]
fn an_invalid_json_capture_reds() {
    let verdict = run_gate(|root| {
        write_file(root, CAPTURE, "{not json");
    });
    assert_red(
        &verdict,
        "present but not valid JSON",
        "invalid capture JSON",
    );
}

/// A bundle half present but unparseable must RED the schema leg — `Err(e)`
/// paths are guards too.
#[test]
fn an_invalid_json_bundle_half_reds() {
    let verdict = run_gate(|root| {
        write_file(root, BUNDLE_B, "{not json");
    });
    assert_red(&verdict, "is not valid JSON", "invalid bundle JSON");
}

/// §A6 review 2026-08-18 (P8): the negation used to be document-global — one
/// `not two machines` anywhere disarmed an overclaim everywhere. The smuggle
/// shape: negate the phrase IN THE SAME BREATH as the overclaim.
#[test]
fn a_negation_smuggle_in_one_field_reds() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["shape"] =
            serde_json::json!("this is not two machines — two machines in two datacentres");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert_red(
        &verdict,
        "which no control in this story proves",
        "negation smuggle",
    );
}

/// The adjacent negation is still honored — a capture that says the honest
/// negative stays GREEN. Without this the smuggle fix could be "satisfied" by
/// refusing every negation outright.
#[test]
fn an_adjacent_negation_still_excuses() {
    let verdict = run_gate(|root| {
        let mut capture = good_capture();
        capture["shape"] = serde_json::json!("not two machines: two keyed identities on one box");
        write_file(root, CAPTURE, &capture.to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert!(
        verdict.passed && verdict.success,
        "an adjacent negation must not over-block\nstdout:\n{}",
        verdict.stdout
    );
}

/// **The enrollment falsifier.** A `_2c.rs` file that exists but is not named in
/// the job is dead in CI behind a green gate. The derived set makes it RED.
#[test]
fn an_unenrolled_story_test_target_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            "crates/maos-cli/tests/brand_new_thing_2c.rs",
            "#[test]\nfn placeholder() {}\n",
        );
    });
    assert_red(
        &verdict,
        "--test brand_new_thing_2c",
        "unenrolled _2c.rs target",
    );
}

/// Same property for the other naming convention this story's tests use — a
/// suffix-only derivation would silently miss the whole `maos-a2a-tcp` directory.
#[test]
fn an_unenrolled_t_prefixed_target_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            "crates/maos-a2a-tcp/tests/t_2c_brand_new.rs",
            "#[test]\nfn placeholder() {}\n",
        );
    });
    assert_red(&verdict, "--test t_2c_brand_new", "unenrolled t_2c_ target");
}

/// A `services:` block would trip the substrate-drift gate, which is itself
/// Blocking and in ship-gate needs.
#[test]
fn a_services_block_on_this_job_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            &good_workflow().replace("    runs-on: ubuntu-latest\n    steps:", "    runs-on: ubuntu-latest\n    services:\n      postgres:\n        image: postgres:16\n    steps:"),
        );
    });
    assert_red(
        &verdict,
        "must not declare a `services:` block",
        "services block",
    );
}

/// The job disappearing entirely must RED — otherwise every enrolled test becomes
/// a suggestion at once.
#[test]
fn deleting_the_gates_job_reds() {
    let verdict = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            "jobs:\n  check-other:\n    runs-on: ubuntu-latest\n",
        );
    });
    assert_red(&verdict, "is a suggestion, not a control", "no job");
}

/// A missing governed file is a FINDING, never a skip. A gate that passes when its
/// subject is absent is the null control this story exists to stop.
#[test]
fn a_missing_governed_file_is_a_finding_not_a_skip() {
    for rel in [
        SUBCOMMANDS_RS,
        CLI_RS,
        SEALED_EXPORT_RS,
        AUDIT_MANIFEST,
        ROUTER_RS,
        COHORT_RS,
        TRANSPORT_RS,
        A2A_TCP_MANIFEST,
        COHORT_STATE_RS,
        REDACTION_RS,
        BUNDLE_SCHEMA,
        WORKFLOW,
        DEMO_J1_RS,
    ] {
        let verdict = run_gate(|root| {
            std::fs::remove_file(root.join(rel)).unwrap();
        });
        assert_red(&verdict, &format!("cannot read {rel}"), rel);
    }
}

/// §A6 review P9 (AC3.5) — the missing PRESENT-capture green vector. The
/// baseline vector asserts `paid_run_capture_present: false`; nothing asserted
/// the gate's honesty in the state that actually lands once a paid run ships:
/// capture PRESENT, gate green, and the claim STILL refused —
/// `two_host_signed_run_claimed` must read `false` beside a `true` capture
/// presence, or the F2 re-scope has no committed pin at all.
#[test]
fn a_present_capture_still_claims_nothing() {
    let verdict = run_gate(|root| {
        write_file(root, CAPTURE, &good_capture().to_string());
        write_file(root, BUNDLE_A, &good_bundle("host-a").to_string());
        write_file(root, BUNDLE_B, &good_bundle("host-b").to_string());
    });
    assert!(
        verdict.passed && verdict.success,
        "a well-formed present capture is the operator lane's honest green state\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict.stdout.contains("\"paid_run_capture_present\":true"),
        "the capture IS present and the gate must say so\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict
            .stdout
            .contains("\"two_host_signed_run_claimed\":false"),
        "a present capture must NOT become a claimed run — verify.py and \
         reconcile-hosts are operator-performed, never gate-checked (F2/R1)\nstdout:\n{}",
        verdict.stdout
    );
}
