//! Story 7.2 AC2 §2.1–§2.3 — publish happy-path against FixtureReplay client.

#![cfg(feature = "fixture_replay")]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use maos_domain::ports::registry::SpiritRegistryClient;
use maos_registry::fixture_replay::FixtureReplaySpiritRegistryClient;
use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
use maos_spirit_cli::publish::{
    build_signed_package, run_publish_with_client, PublishArgs, PublishOutcome,
};

fn write_temp(contents: &[u8], suffix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let stem = format!(
        "maos-spirit-cli-test-{}-{}",
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

fn minimal_manifest(tier: &str) -> Vec<u8> {
    format!(
        r#"name = "test-spirit"
version = "0.1.0"
trust_tier = "{tier}"
sandbox_tier = "t0"
capability_scope = ["log.write.local"]
provider_id = "anthropic"
endpoint_url = "https://api.anthropic.com"
crypto_provider = "ring"
"#
    )
    .into_bytes()
}

fn hex_key() -> Vec<u8> {
    b"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_vec()
}

#[test]
fn publishes_local_tier_against_fixture_replay() {
    let manifest = write_temp(&minimal_manifest("local"), ".toml");
    let artifact = write_temp(b"#!/bin/sh\necho hi\n", ".bin");
    let key = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest,
        artifact,
        signing_key: Some(key),
        signing_key_env: None,
        registry_uri: Some("stub".into()),
        compliance_claim: None,
        dry_run: false,
    };

    let receipt_value = serde_json::json!({
        "publish_id": "p-123",
        "spirit_id": "test-spirit",
        "version": "0.1.0",
    });
    let client = FixtureReplaySpiritRegistryClient::new(vec![Ok(receipt_value)]);

    let outcome = run_publish_with_client(&args, &client).expect("publish failed");
    match outcome {
        PublishOutcome::Receipt(r) => {
            assert_eq!(r.publish_id, "p-123");
            assert_eq!(r.spirit_id.as_str(), "test-spirit");
            assert_eq!(r.version, "0.1.0");
        }
        PublishOutcome::DryRun { .. } => panic!("unexpected dry-run outcome"),
    }
    let calls = client.take_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "registry.publish");
}

#[test]
fn dry_run_prints_signed_package_without_dispatch() {
    let manifest = write_temp(&minimal_manifest("local"), ".toml");
    let artifact = write_temp(b"bytes", ".bin");
    let key = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest,
        artifact,
        signing_key: Some(key),
        signing_key_env: None,
        registry_uri: Some("stub".into()),
        compliance_claim: None,
        dry_run: true,
    };
    // No responses queued — if dispatch happened, we'd get a Transport error.
    let client = FixtureReplaySpiritRegistryClient::new(vec![]);

    let outcome = run_publish_with_client(&args, &client).expect("dry-run failed");
    match outcome {
        PublishOutcome::DryRun {
            signed_package_summary,
        } => {
            assert_eq!(signed_package_summary.spirit_id, "test-spirit");
            assert_eq!(signed_package_summary.signature_hex.len(), 128);
            assert_eq!(signed_package_summary.publisher_pubkey_hex.len(), 64);
        }
        PublishOutcome::Receipt(_) => panic!("dry-run should not dispatch"),
    }
    let calls = client.take_calls();
    assert!(calls.is_empty(), "dry-run must not record any calls");
}

#[test]
fn publishes_public_untrusted_with_envelope() {
    let manifest = write_temp(&minimal_manifest("public_untrusted"), ".toml");
    let artifact = write_temp(b"artifact-data", ".bin");
    let key = write_temp(&hex_key(), ".key");

    // Create an external ComplianceClaim envelope (CBOR-encoded).
    let external_envelope = ComplianceClaimEnvelope {
        signature: [0xAAu8; 64],
        attester_pubkey: [0xCCu8; 32], // Different from publisher — third-party attested.
        claim_bytes: vec![0xA1, 0x01, 0x02],
        signing_alg: SigningAlg::Ed25519,
    };
    let envelope_cbor = serde_cbor::to_vec(&external_envelope).unwrap();
    let envelope_path = write_temp(&envelope_cbor, ".cbor");

    let args = PublishArgs {
        tier: "public_untrusted".into(),
        manifest: manifest.clone(),
        artifact: artifact.clone(),
        signing_key: Some(key),
        signing_key_env: None,
        registry_uri: Some("stub".into()),
        compliance_claim: Some(envelope_path),
        dry_run: false,
    };

    let receipt_value = serde_json::json!({
        "publish_id": "p-external-1",
        "spirit_id": "test-spirit",
        "version": "0.1.0",
    });
    let client = FixtureReplaySpiritRegistryClient::new(vec![Ok(receipt_value)]);

    let outcome = run_publish_with_client(&args, &client).expect("publish with envelope failed");
    match outcome {
        PublishOutcome::Receipt(r) => {
            assert_eq!(r.publish_id, "p-external-1");
        }
        PublishOutcome::DryRun { .. } => panic!("unexpected dry-run outcome"),
    }

    // Verify the package carried the EXTERNAL envelope, not an auto-populated one.
    let pkg = build_signed_package(&args).unwrap();
    assert_eq!(
        pkg.compliance_envelope.attester_pubkey, [0xCCu8; 32],
        "external envelope's attester_pubkey must be preserved"
    );
}

#[test]
fn build_signed_package_produces_valid_signature_envelope() {
    let manifest = write_temp(&minimal_manifest("local"), ".toml");
    let artifact = write_temp(b"abc", ".bin");
    let key = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest,
        artifact,
        signing_key: Some(key),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: false,
    };
    let pkg = build_signed_package(&args).expect("build failed");
    assert_eq!(pkg.spirit_id.as_str(), "test-spirit");
    assert_eq!(pkg.version, "0.1.0");
    // Auto-populated envelope is self-attested: attester_pubkey == publisher_pubkey.
    assert_eq!(
        pkg.compliance_envelope.attester_pubkey,
        pkg.publisher_pubkey
    );
    // Envelope signature must be exactly 64 bytes.
    assert_eq!(pkg.compliance_envelope.signature.len(), 64);
}
