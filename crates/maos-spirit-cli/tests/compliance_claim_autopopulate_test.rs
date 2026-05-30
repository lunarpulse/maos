//! Story 7.2 AC2 §2.10–§2.12 — ComplianceClaim envelope auto-population.

#![cfg(feature = "fixture_replay")]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use maos_registry::compliance_verify::{verify_envelope_structural, VerificationResult};
use maos_spirit_cli::publish::{build_signed_package, PublishArgs};

fn write_temp(contents: &[u8], suffix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let stem = format!(
        "maos-spirit-cli-cc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    path.push(format!("{stem}{suffix}"));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(contents).unwrap();
    // Key files must have restrictive permissions (0600 or tighter).
    if suffix.ends_with(".key") {
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms).unwrap();
    }
    path
}

fn manifest_public_untrusted() -> Vec<u8> {
    br#"name = "cc-spirit"
version = "0.1.0"
trust_tier = "public_untrusted"
sandbox_tier = "t0"
capability_scope = ["log.write.local", "inference.invoke.scoped"]
provider_id = "anthropic"
endpoint_url = "https://api.anthropic.com"
crypto_provider = "ring"
"#
    .to_vec()
}

fn hex_key() -> Vec<u8> {
    b"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_vec()
}

#[test]
fn auto_populated_envelope_passes_structural_verify() {
    let m = write_temp(&manifest_public_untrusted(), ".toml");
    let a = write_temp(b"artifact", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "public_untrusted".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let pkg = build_signed_package(&args).unwrap();

    // The auto-populated envelope MUST pass Story 5.5d's structural verifier.
    let result = verify_envelope_structural(&pkg.compliance_envelope, &pkg);
    assert!(
        matches!(result, VerificationResult::Ok),
        "auto-populated envelope must verify structurally; got: {:?}",
        result
    );
}

#[test]
fn auto_populated_envelope_is_self_attested() {
    let m = write_temp(&manifest_public_untrusted(), ".toml");
    let a = write_temp(b"artifact", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "public_untrusted".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let pkg = build_signed_package(&args).unwrap();
    assert_eq!(
        pkg.compliance_envelope.attester_pubkey, pkg.publisher_pubkey,
        "auto-populated envelope MUST be self-attested per §8.5 v0.5 binding"
    );
}

#[test]
fn external_envelope_overrides_auto_population() {
    let m = write_temp(&manifest_public_untrusted(), ".toml");
    let a = write_temp(b"artifact", ".bin");
    let k = write_temp(&hex_key(), ".key");

    // Create an external envelope with a deliberately different attester_pubkey
    // so we can distinguish it from the auto-populated (self-attested) case.
    let external_envelope = maos_spirit_abi::compliance::ComplianceClaimEnvelope {
        signature: [0xDDu8; 64],
        attester_pubkey: [0xEEu8; 32], // Not the publisher's key — third-party attested.
        claim_bytes: vec![0xA2, 0x01, 0x02, 0x03, 0x04],
        signing_alg: maos_spirit_abi::compliance::SigningAlg::Ed25519,
    };
    let envelope_cbor = serde_cbor::to_vec(&external_envelope).unwrap();
    let envelope_path = write_temp(&envelope_cbor, ".cbor");

    let args = PublishArgs {
        tier: "public_untrusted".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: Some(envelope_path),
        dry_run: true,
    };
    let pkg = build_signed_package(&args).unwrap();

    // The external envelope MUST override auto-population.
    assert_eq!(
        pkg.compliance_envelope.attester_pubkey, [0xEEu8; 32],
        "--compliance-claim must bypass auto-population and use the provided envelope"
    );
    assert_eq!(
        pkg.compliance_envelope.signature, [0xDDu8; 64],
        "external envelope signature must be preserved"
    );
}
