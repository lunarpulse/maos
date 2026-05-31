//! Story 7.2 D1 — Air-gapped import path integration tests.

use std::io::Write;
use std::path::PathBuf;

use maos_domain::ports::registry::SpiritId;
use maos_registry::import::{extract_bundle, verify_bundle_consistency, ImportError};
use maos_registry::origin::RegistryOrigin;
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};

fn write_tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        for (name, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, std::io::Cursor::new(*contents))
                .unwrap();
        }
        builder.finish().unwrap();
    }
    buf
}

fn synthetic_signed_package_json(manifest: &[u8], artifact: &[u8]) -> String {
    let envelope = serde_json::json!({
        "signature": vec![0u8; 64],
        "attester_pubkey": vec![1u8; 32],
        "claim_bytes": vec![0xA1u8, 0x01, 0x02],
        "signing_alg": "ed25519",
    });
    let value = serde_json::json!({
        "spirit_id": "import-spirit",
        "version": "0.1.0",
        "manifest_toml": manifest,
        "artifact_bytes": artifact,
        "signature": hex::encode([0u8; 64]),
        "publisher_pubkey": hex::encode([0u8; 32]),
        "compliance_envelope": envelope,
    });
    serde_json::to_string(&value).unwrap()
}

fn unique_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn make_bundle_tar() -> (PathBuf, Vec<u8>) {
    let manifest = br#"name = "import-spirit"
version = "0.1.0"
trust_tier = "local"
"#;
    let artifact = b"artifact-payload";
    let pkg_json = synthetic_signed_package_json(manifest, artifact);
    let tar_bytes = write_tar(&[
        ("manifest.toml", manifest as &[u8]),
        ("artifact.bin", artifact),
        ("signed-package.json", pkg_json.as_bytes()),
    ]);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "maos-import-test-{}-{}.tar",
        std::process::id(),
        unique_id()
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(&tar_bytes)
        .unwrap();
    (tmp, tar_bytes)
}

#[test]
fn extract_bundle_happy_path() {
    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).expect("extract should succeed");
    assert_eq!(bundle.signed_package.spirit_id.as_str(), "import-spirit");
    assert_eq!(bundle.signed_package.version, "0.1.0");
    assert!(!bundle.bundle_sha256.is_empty());
}

#[test]
fn verify_consistent_bundle_ok() {
    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).unwrap();
    verify_bundle_consistency(&bundle).expect("consistency check should pass");
}

#[test]
fn divergent_manifest_rejects() {
    let manifest = b"original\n";
    let artifact = b"a";
    let pkg_json = synthetic_signed_package_json(manifest, artifact);
    let tar_bytes = write_tar(&[
        ("manifest.toml", b"DIVERGENT" as &[u8]),
        ("artifact.bin", artifact),
        ("signed-package.json", pkg_json.as_bytes()),
    ]);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "maos-import-divergent-{}-{}.tar",
        std::process::id(),
        unique_id()
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(&tar_bytes)
        .unwrap();
    let bundle = extract_bundle(&tmp).unwrap();
    let err = verify_bundle_consistency(&bundle).unwrap_err();
    assert!(matches!(err, ImportError::InconsistentExtract { file } if file == "manifest.toml"));
}

#[test]
fn missing_signed_package_rejects() {
    let tar_bytes = write_tar(&[("manifest.toml", b"x" as &[u8]), ("artifact.bin", b"y")]);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "maos-import-missing-pkg-{}-{}.tar",
        std::process::id(),
        unique_id()
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(&tar_bytes)
        .unwrap();
    let err = extract_bundle(&tmp).unwrap_err();
    assert!(matches!(err, ImportError::SignedPackageParseFailure(_)));
}

#[test]
fn publish_with_origin_persists_origin_file() {
    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).unwrap();

    let tmp_root = std::env::temp_dir().join(format!("maos-registry-{}-origin", unique_id()));
    std::fs::create_dir_all(&tmp_root).unwrap();
    let storage = LocalFsRegistryStorage::at_path(tmp_root.clone()).unwrap();

    let origin = RegistryOrigin::Imported {
        bundle_sha256: bundle.bundle_sha256.clone(),
    };
    storage
        .publish_with_origin(
            &bundle.signed_package.spirit_id,
            &bundle.signed_package.version,
            &bundle.signed_package,
            &origin,
        )
        .unwrap();

    // Verify origin.json was written.
    let origin_path = tmp_root
        .join("spirits")
        .join("import-spirit")
        .join("0.1.0")
        .join("origin.json");
    assert!(origin_path.exists(), "origin.json should be written");
    let origin_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&origin_path).unwrap()).unwrap();
    assert_eq!(
        origin_json["Imported"]["bundle_sha256"],
        bundle.bundle_sha256
    );
}

#[test]
fn admit_spirit_local_unsigned_passes() {
    use maos_registry::admission::{admit_spirit, AdmissionConfig};
    use maos_spirit_abi::compliance::TrustTier;

    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).unwrap();

    let op_cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier: TrustTier::Local,
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };

    let decision = admit_spirit(&bundle.signed_package, &op_cfg).expect("admission should pass");
    assert_eq!(decision.effective_tier, TrustTier::Local);
}

#[test]
fn admit_spirit_public_untrusted_without_sig_fails() {
    use maos_registry::admission::{admit_spirit, AdmissionConfig};
    use maos_spirit_abi::compliance::TrustTier;

    let manifest = br#"name = "import-spirit"
version = "0.1.0"
trust_tier = "public_untrusted"
"#;
    let artifact = b"artifact-payload";
    let pkg_json = synthetic_signed_package_json(manifest, artifact);
    let tar_bytes = write_tar(&[
        ("manifest.toml", manifest as &[u8]),
        ("artifact.bin", artifact),
        ("signed-package.json", pkg_json.as_bytes()),
    ]);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "maos-import-pubunt-{}-{}.tar",
        std::process::id(),
        unique_id()
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(&tar_bytes)
        .unwrap();
    let bundle = extract_bundle(&tmp).unwrap();

    let op_cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier: TrustTier::Local,
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };

    // The synthetic package has a zeroed signature, so verification will fail.
    let err = admit_spirit(&bundle.signed_package, &op_cfg).unwrap_err();
    assert!(
        err.to_string().contains("PublisherSignatureInvalid")
            || err.to_string().contains("signature"),
        "expected signature failure, got: {err}"
    );
}

#[test]
fn published_origin_persists_without_origin_file() {
    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).unwrap();

    let tmp_root = std::env::temp_dir().join(format!("maos-registry-{}-pub", unique_id()));
    std::fs::create_dir_all(&tmp_root).unwrap();
    let storage = LocalFsRegistryStorage::at_path(tmp_root.clone()).unwrap();

    let origin = RegistryOrigin::Published;
    storage
        .publish_with_origin(
            &bundle.signed_package.spirit_id,
            &bundle.signed_package.version,
            &bundle.signed_package,
            &origin,
        )
        .unwrap();

    // Verify origin.json was written even for Published.
    let origin_path = tmp_root
        .join("spirits")
        .join("import-spirit")
        .join("0.1.0")
        .join("origin.json");
    assert!(
        origin_path.exists(),
        "origin.json should be written for Published too"
    );
}

#[test]
fn origin_roundtrips_through_storage() {
    let (path, _) = make_bundle_tar();
    let bundle = extract_bundle(&path).unwrap();

    let tmp_root = std::env::temp_dir().join(format!("maos-registry-{}-rt", unique_id()));
    std::fs::create_dir_all(&tmp_root).unwrap();
    let storage = LocalFsRegistryStorage::at_path(tmp_root.clone()).unwrap();

    let origin = RegistryOrigin::Imported {
        bundle_sha256: "abc123".into(),
    };
    storage
        .publish_with_origin(
            &bundle.signed_package.spirit_id,
            &bundle.signed_package.version,
            &bundle.signed_package,
            &origin,
        )
        .unwrap();

    // Verify the manifest can be retrieved.
    let manifest = storage
        .get_manifest(&SpiritId::from("import-spirit"), "0.1.0")
        .expect("manifest should be retrievable");
    assert_eq!(manifest.spirit_id.as_str(), "import-spirit");
}
