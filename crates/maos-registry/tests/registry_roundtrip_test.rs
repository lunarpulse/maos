//! Roundtrip corpus driver for registry fixtures (Story 5-5d, AC6).
//!
//! Loads JSON fixture files from `tests/fixtures/registry-roundtrip-v05/`,
//! reconstructs `SignedPackage` + `AdmissionConfig`, and asserts the
//! expected admission outcome.
//!
//! Run with: `cargo test -p maos-registry --features fixture_replay`

#![cfg(feature = "fixture_replay")]

use std::path::{Path, PathBuf};

use maos_domain::ports::registry::{SearchQuery, SignedPackage, SpiritId, SpiritRegistryClient};
use maos_registry::fixture_replay::FixtureReplaySpiritRegistryClient;
use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    description: String,
    spirit_id: String,
    version: String,
    manifest_toml_b64: String,
    artifact_bytes_b64: String,
    signature_hex: String,
    publisher_pubkey_hex: String,
    compliance_envelope: FixtureEnvelope,
    expected_admission: Option<ExpectedAdmission>,
    expected_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureEnvelope {
    signature_hex: String,
    attester_pubkey_hex: String,
    claim_bytes_b64: String,
    signing_alg: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedAdmission {
    admit: bool,
    effective_tier: Option<String>,
    error_variant: Option<String>,
}

fn hex_to_64bytes(hex: &str) -> [u8; 64] {
    let bytes = hex::decode(hex).expect("invalid hex in signature");
    let arr: [u8; 64] = bytes.try_into().expect("expected 64 bytes for signature");
    arr
}

fn hex_to_32bytes(hex: &str) -> [u8; 32] {
    let bytes = hex::decode(hex).expect("invalid hex in pubkey");
    let arr: [u8; 32] = bytes.try_into().expect("expected 32 bytes for pubkey");
    arr
}

fn b64_to_vec(b64: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("invalid base64")
}

fn load_fixture(path: &Path) -> Fixture {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

fn build_signed_package(f: &Fixture) -> Result<SignedPackage, String> {
    let sig_bytes = hex::decode(&f.signature_hex).map_err(|e| format!("signature hex: {e}"))?;
    if sig_bytes.len() != 64 {
        return Err(format!(
            "signature is {} bytes, expected 64",
            sig_bytes.len()
        ));
    }
    let signature: [u8; 64] = sig_bytes.try_into().unwrap();

    let pk_bytes = hex::decode(&f.publisher_pubkey_hex).map_err(|e| format!("pubkey hex: {e}"))?;
    if pk_bytes.len() != 32 {
        return Err(format!("pubkey is {} bytes, expected 32", pk_bytes.len()));
    }
    let publisher_pubkey: [u8; 32] = pk_bytes.try_into().unwrap();

    let env_sig_bytes = hex::decode(&f.compliance_envelope.signature_hex)
        .map_err(|e| format!("envelope sig hex: {e}"))?;
    if env_sig_bytes.len() != 64 {
        return Err(format!(
            "envelope signature is {} bytes, expected 64",
            env_sig_bytes.len()
        ));
    }
    let envelope_signature: [u8; 64] = env_sig_bytes.try_into().unwrap();

    let env_pk_bytes = hex::decode(&f.compliance_envelope.attester_pubkey_hex)
        .map_err(|e| format!("envelope pubkey hex: {e}"))?;
    if env_pk_bytes.len() != 32 {
        return Err(format!(
            "envelope pubkey is {} bytes, expected 32",
            env_pk_bytes.len()
        ));
    }
    let attester_pubkey: [u8; 32] = env_pk_bytes.try_into().unwrap();

    let signing_alg = match f.compliance_envelope.signing_alg.as_str() {
        "Ed25519" => SigningAlg::Ed25519,
        other => return Err(format!("unknown signing_alg: {other}")),
    };

    let envelope = ComplianceClaimEnvelope {
        signature: envelope_signature,
        attester_pubkey,
        claim_bytes: b64_to_vec(&f.compliance_envelope.claim_bytes_b64),
        signing_alg,
    };

    Ok(SignedPackage::new(
        SpiritId::from(f.spirit_id.as_str()),
        f.version.clone(),
        b64_to_vec(&f.manifest_toml_b64),
        b64_to_vec(&f.artifact_bytes_b64),
        signature,
        publisher_pubkey,
        envelope,
    ))
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry-roundtrip-v05")
}

fn collect_fixtures(subdir: &str) -> Vec<(String, Fixture)> {
    let dir = fixture_dir().join(subdir);
    if !dir.exists() {
        return Vec::new();
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries
        .into_iter()
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let fixture = load_fixture(&e.path());
            (name, fixture)
        })
        .collect()
}

#[test]
fn well_formed_fixtures_admit_or_error_as_expected() {
    use maos_registry::admission::{admit_spirit, AdmissionConfig};
    use maos_spirit_abi::compliance::TrustTier;

    let fixtures = collect_fixtures("well-formed");
    assert!(
        fixtures.len() >= 10,
        "expected >= 10 well-formed fixtures, got {}",
        fixtures.len()
    );

    for (name, f) in &fixtures {
        let pkg = match build_signed_package(f) {
            Ok(p) => p,
            Err(e) => panic!("[{name}] failed to build package: {e}"),
        };
        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
        };

        let result = admit_spirit(&pkg, &cfg);

        if let Some(expected) = &f.expected_admission {
            match result {
                Ok(decision) => {
                    assert_eq!(
                        decision.admit, expected.admit,
                        "[{name}] admit mismatch: got {}, expected {}",
                        decision.admit, expected.admit
                    );
                    if let Some(tier) = &expected.effective_tier {
                        let expected_tier = match tier.as_str() {
                            "local" => TrustTier::Local,
                            "org_internal" => TrustTier::OrgInternal,
                            "public_untrusted" => TrustTier::PublicUntrusted,
                            "public_vetted" => TrustTier::PublicVetted,
                            other => panic!("[{name}] unknown expected tier: {other}"),
                        };
                        assert_eq!(
                            decision.effective_tier, expected_tier,
                            "[{name}] effective_tier mismatch",
                        );
                    }
                }
                Err(e) => {
                    if let Some(err_var) = &expected.error_variant {
                        let err_str = format!("{e:?}");
                        assert!(
                            err_str.contains(err_var),
                            "[{name}] error '{err_str}' does not contain expected variant '{err_var}'"
                        );
                    } else if expected.admit {
                        panic!("[{name}] expected admit=true but got error: {e:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn malformed_fixtures_rejected_with_typed_error() {
    use maos_registry::admission::{admit_spirit, AdmissionConfig};
    use maos_spirit_abi::compliance::TrustTier;

    let fixtures = collect_fixtures("malformed-rejected");
    assert!(
        fixtures.len() >= 8,
        "expected >= 8 malformed fixtures, got {}",
        fixtures.len()
    );

    for (name, f) in &fixtures {
        let pkg = match build_signed_package(f) {
            Ok(p) => p,
            Err(_construction_err) => {
                continue;
            }
        };
        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
        };

        let result = admit_spirit(&pkg, &cfg);

        if let Some(expected) = &f.expected_admission {
            match result {
                Ok(decision) => {
                    assert_eq!(decision.admit, expected.admit, "[{name}] admit mismatch");
                }
                Err(e) => {
                    if let Some(err_var) = &expected.error_variant {
                        let err_str = format!("{e:?}");
                        assert!(
                            err_str.contains(err_var),
                            "[{name}] error does not contain '{err_var}': {err_str}"
                        );
                    }
                }
            }
        } else {
            assert!(
                result.is_err(),
                "[{name}] expected error but got Ok: {:?}",
                result.unwrap()
            );

            if let Some(expected_err) = &f.expected_error {
                let err_str = format!("{:?}", result.unwrap_err());
                assert!(
                    err_str.contains(expected_err),
                    "[{name}] error does not contain expected '{expected_err}': {err_str}"
                );
            }
        }
    }
}

#[test]
fn fixture_replay_search_roundtrip() {
    let client = FixtureReplaySpiritRegistryClient::new(vec![Ok(serde_json::json!({
        "items": [{
            "spirit_id": "spirit-local-unsigned-001",
            "version": "0.1.0",
            "summary": "test spirit"
        }]
    }))]);

    let q = SearchQuery::new("spirit-local".into(), false, 10);
    let results = client.search(&q).unwrap();
    assert_eq!(results.items.len(), 1);
}

#[test]
fn fixture_replay_publish_roundtrip() {
    let client = FixtureReplaySpiritRegistryClient::new(vec![Ok(serde_json::json!({
        "publish_id": "pub-001",
        "spirit_id": "test-spirit",
        "version": "0.1.0"
    }))]);

    let fixtures = collect_fixtures("well-formed");
    let (_name, f) = fixtures.first().expect("at least one well-formed fixture");
    let pkg = build_signed_package(f).expect("first fixture should be valid");

    let receipt = client.publish(&pkg).unwrap();
    assert_eq!(receipt.spirit_id.as_str(), "test-spirit");

    let calls = client.take_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "registry.publish");
}

#[test]
fn all_fixtures_have_valid_structure() {
    let wf = collect_fixtures("well-formed");
    let mf = collect_fixtures("malformed-rejected");

    for (name, f) in wf.iter().chain(mf.iter()) {
        assert!(!f.description.is_empty(), "[{name}] missing description");
        assert!(!f.spirit_id.is_empty(), "[{name}] missing spirit_id");
        assert!(!f.version.is_empty(), "[{name}] missing version");

        assert!(
            !f.signature_hex.is_empty(),
            "[{name}] missing signature_hex"
        );
        assert!(
            !f.publisher_pubkey_hex.is_empty(),
            "[{name}] missing publisher_pubkey_hex"
        );
    }
}
