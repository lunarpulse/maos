//! Story 7.2 AC3 — FR60 air-gapped artifact import (`maosctl import --offline`).
//!
//! Reads a tar bundle carrying `signed-package.json` (authoritative) +
//! `manifest.toml` (operator-inspection extract) + `artifact.bin` (operator-
//! inspection extract) and, optionally, `vetter-attestations/` and
//! `compliance-claims/` directories carrying 0+ supplementary artifacts.
//!
//! v1.0 supports UNCOMPRESSED `.tar` only — gzip/zstd is deferred to v0.7
//! per the spec's "air-gapped operators often run on systems without zstd"
//! rationale.

use std::path::{Path, PathBuf};

use maos_domain::ports::registry::SignedPackage;
use maos_spirit_abi::compliance::ComplianceClaimEnvelope;
use sha2::Digest;

/// Maximum total tar file size (1 GiB).
const MAX_TAR_SIZE: u64 = 1_073_741_824;

/// Maximum size for any individual entry in the tar (100 MiB).
const MAX_ENTRY_SIZE: u64 = 104_857_600;

/// Maximum number of entries in the tar.
const MAX_ENTRY_COUNT: usize = 10_000;

/// A parsed offline-import bundle.
#[derive(Debug)]
pub struct ImportedBundle {
    /// SHA-256 of the tar file itself, hex-encoded.
    pub bundle_sha256: String,
    /// Authoritative SignedPackage (parsed from `signed-package.json`).
    pub signed_package: SignedPackage,
    /// Raw extracted manifest.toml — used for consistency checking against
    /// `signed_package.manifest_toml`.
    pub extracted_manifest_toml: Vec<u8>,
    /// Raw extracted artifact.bin — used for consistency checking.
    pub extracted_artifact_bytes: Vec<u8>,
    /// 0+ optional vetter attestations (each a raw JSON-encoded VetterAttestation
    /// per the FR37 v2.5-deferred shape).
    pub vetter_attestations: Vec<Vec<u8>>,
    /// 0+ optional supplementary ComplianceClaim envelopes (each a CBOR-encoded
    /// auxiliary envelope).
    pub supplementary_claims: Vec<ComplianceClaimEnvelope>,
}

/// Typed import-path errors. Map to FR63 typed-error catalog entries
/// (`EImportBundleInconsistent`, `EImportSignedPackageParse`, etc.).
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ImportError {
    #[error("tar archive parse failure: {0}")]
    TarParse(String),

    #[error("signed-package.json parse failure: {0}")]
    SignedPackageParseFailure(String),

    #[error(
        "bundle file '{file}' diverges from signed-package.json — corrupted or tampered"
    )]
    InconsistentExtract { file: String },

    #[error("vetter attestation parse failure: {0}")]
    VetterAttestationParse(String),

    #[error("supplementary ComplianceClaim parse failure: {0}")]
    SupplementaryClaimParse(String),

    #[error("io error: {0}")]
    Io(String),
}

/// Open a tar archive, extract every file into memory, and parse the
/// authoritative `signed-package.json` plus the convenience extracts.
pub fn extract_bundle(tar_path: &Path) -> Result<ImportedBundle, ImportError> {
    // 1. Compute sha256 of the tar file itself.
    let metadata = std::fs::metadata(tar_path)
        .map_err(|e| ImportError::Io(format!("read tar metadata: {e}")))?;
    let tar_size = metadata.len();
    if tar_size > MAX_TAR_SIZE {
        return Err(ImportError::TarParse(format!(
            "tar file too large: {} bytes (max {})",
            tar_size, MAX_TAR_SIZE
        )));
    }
    let bundle_bytes =
        std::fs::read(tar_path).map_err(|e| ImportError::Io(format!("read tar: {e}")))?;
    let bundle_sha256 = hex::encode(sha256_hash(&bundle_bytes));

    // 2. Walk the tar.
    let mut archive = tar::Archive::new(std::io::Cursor::new(&bundle_bytes[..]));
    let mut signed_package_bytes: Option<Vec<u8>> = None;
    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut artifact_bytes: Option<Vec<u8>> = None;
    let mut vetter_attestations: Vec<Vec<u8>> = Vec::new();
    let mut supplementary_claims: Vec<ComplianceClaimEnvelope> = Vec::new();
    let mut entry_count: usize = 0;

    let entries = archive
        .entries()
        .map_err(|e| ImportError::TarParse(format!("entries: {e}")))?;
    for entry_res in entries {
        entry_count += 1;
        if entry_count > MAX_ENTRY_COUNT {
            return Err(ImportError::TarParse(format!(
                "too many entries in tar: {} (max {})",
                entry_count, MAX_ENTRY_COUNT
            )));
        }
        let mut entry = entry_res.map_err(|e| ImportError::TarParse(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| ImportError::TarParse(e.to_string()))?
            .into_owned();
        let path_str = path.to_string_lossy().to_string();
        
        // Check entry size before reading.
        let entry_size = entry.size();
        if entry_size > MAX_ENTRY_SIZE {
            return Err(ImportError::TarParse(format!(
                "entry '{}' too large: {} bytes (max {})",
                path_str, entry_size, MAX_ENTRY_SIZE
            )));
        }
        
        let mut buf = Vec::new();
        use std::io::Read;
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ImportError::Io(format!("read entry {path_str}: {e}")))?;
        match path_str.as_str() {
            "signed-package.json" | "./signed-package.json" => {
                if signed_package_bytes.is_some() {
                    return Err(ImportError::TarParse(
                        "duplicate signed-package.json entry in bundle".into(),
                    ));
                }
                signed_package_bytes = Some(buf);
            }
            "manifest.toml" | "./manifest.toml" => {
                if manifest_bytes.is_some() {
                    return Err(ImportError::TarParse(
                        "duplicate manifest.toml entry in bundle".into(),
                    ));
                }
                manifest_bytes = Some(buf);
            }
            "artifact.bin" | "./artifact.bin" => {
                if artifact_bytes.is_some() {
                    return Err(ImportError::TarParse(
                        "duplicate artifact.bin entry in bundle".into(),
                    ));
                }
                artifact_bytes = Some(buf);
            }
            other if other.starts_with("vetter-attestations/")
                || other.starts_with("./vetter-attestations/") =>
            {
                if !buf.is_empty() {
                    vetter_attestations.push(buf);
                }
            }
            other if other.starts_with("compliance-claims/")
                || other.starts_with("./compliance-claims/") =>
            {
                if buf.is_empty() {
                    continue;
                }
                let env = serde_cbor::from_slice::<ComplianceClaimEnvelope>(&buf)
                    .or_else(|_| serde_json::from_slice::<ComplianceClaimEnvelope>(&buf))
                    .map_err(|e| {
                        ImportError::SupplementaryClaimParse(format!("{other}: {e}"))
                    })?;
                supplementary_claims.push(env);
            }
            // Directory entries and tar-house-keeping files are tolerated.
            other if other.ends_with('/') => continue,
            _ => continue,
        }
    }

    // 3. Parse signed-package.json AS AUTHORITATIVE.
    let signed_package_raw = signed_package_bytes.ok_or_else(|| {
        ImportError::SignedPackageParseFailure("signed-package.json missing from bundle".into())
    })?;
    let signed_package: SignedPackage =
        serde_json::from_slice(&signed_package_raw).map_err(|e| {
            ImportError::SignedPackageParseFailure(format!("JSON decode: {e}"))
        })?;

    Ok(ImportedBundle {
        bundle_sha256,
        signed_package,
        extracted_manifest_toml: manifest_bytes.unwrap_or_default(),
        extracted_artifact_bytes: artifact_bytes.unwrap_or_default(),
        vetter_attestations,
        supplementary_claims,
    })
}

/// Verify per-file consistency: extracted `manifest.toml` and `artifact.bin`
/// MUST byte-match the equivalent fields inside `signed-package.json`.
///
/// On divergence the operator sees WHICH file differs, surfacing tampering
/// vs. corruption without ambiguity.
///
/// Empty extracts are only tolerated when the authoritative package also has
/// empty data for that field. If the package has non-empty data but the
/// extract is empty, that's treated as inconsistent (possible tampering).
pub fn verify_bundle_consistency(bundle: &ImportedBundle) -> Result<(), ImportError> {
    // manifest.toml consistency check
    let manifest_empty_in_pkg = bundle.signed_package.manifest_toml.is_empty();
    let manifest_empty_in_extract = bundle.extracted_manifest_toml.is_empty();
    if manifest_empty_in_pkg && manifest_empty_in_extract {
        // Both empty — consistent
    } else if bundle.extracted_manifest_toml != bundle.signed_package.manifest_toml {
        return Err(ImportError::InconsistentExtract {
            file: "manifest.toml".into(),
        });
    }

    // artifact.bin consistency check
    let artifact_empty_in_pkg = bundle.signed_package.artifact_bytes.is_empty();
    let artifact_empty_in_extract = bundle.extracted_artifact_bytes.is_empty();
    if artifact_empty_in_pkg && artifact_empty_in_extract {
        // Both empty — consistent
    } else if bundle.extracted_artifact_bytes != bundle.signed_package.artifact_bytes {
        return Err(ImportError::InconsistentExtract {
            file: "artifact.bin".into(),
        });
    }

    Ok(())
}

/// Compute the default scratch directory for a bundle SHA.
///
/// Returns `~/.cache/maos/import/<sha256>/` per the spec. Test code may
/// override via `MAOS_IMPORT_SCRATCH_ROOT` env var.
pub fn scratch_dir_for(bundle_sha256: &str) -> PathBuf {
    if let Ok(root) = std::env::var("MAOS_IMPORT_SCRATCH_ROOT") {
        return PathBuf::from(root).join(bundle_sha256);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache/maos/import").join(bundle_sha256)
}

fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let r = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        // Construct a minimal signed-package.json shape mirroring SignedPackage's
        // hand-rolled serde impl: `signature` + `publisher_pubkey` are hex
        // strings, manifest/artifact bytes are byte arrays, and the embedded
        // envelope uses its own derived shape (byte arrays for sig/pubkey).
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

    #[test]
    fn extract_and_verify_consistent_bundle() {
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
        tmp.push(format!("maos-import-test-{}.tar", std::process::id()));
        std::fs::File::create(&tmp)
            .unwrap()
            .write_all(&tar_bytes)
            .unwrap();

        let bundle = extract_bundle(&tmp).expect("extract");
        assert_eq!(bundle.signed_package.spirit_id.as_str(), "import-spirit");
        assert_eq!(bundle.signed_package.version, "0.1.0");
        verify_bundle_consistency(&bundle).expect("consistency");
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
            "maos-import-test-divergent-{}.tar",
            std::process::id()
        ));
        std::fs::File::create(&tmp)
            .unwrap()
            .write_all(&tar_bytes)
            .unwrap();
        let bundle = extract_bundle(&tmp).expect("extract");
        let err = verify_bundle_consistency(&bundle).unwrap_err();
        match err {
            ImportError::InconsistentExtract { file } => assert_eq!(file, "manifest.toml"),
            other => panic!("expected InconsistentExtract, got: {other:?}"),
        }
    }

    #[test]
    fn missing_signed_package_json_rejects() {
        let tar_bytes = write_tar(&[
            ("manifest.toml", b"x" as &[u8]),
            ("artifact.bin", b"y"),
        ]);
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "maos-import-test-missing-pkg-{}.tar",
            std::process::id()
        ));
        std::fs::File::create(&tmp)
            .unwrap()
            .write_all(&tar_bytes)
            .unwrap();
        let err = extract_bundle(&tmp).unwrap_err();
        assert!(matches!(err, ImportError::SignedPackageParseFailure(_)));
    }
}
