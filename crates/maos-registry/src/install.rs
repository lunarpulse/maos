//! Signature-verified offline installation: transport, existing admission, persistence.
use crate::admission::{admit_spirit, admit_spirit_with_attestation, AdmissionConfig};
use crate::client::{McpSpiritRegistryClient, RegistryClientConfig};
use crate::origin::RegistryOrigin;
use crate::storage::LocalFsRegistryStorage;
use maos_domain::ports::registry::{SignedPackage, SpiritId, SpiritRegistryClient, TrustTier};
use std::path::Path;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Promotion evidence and the operator's live keyring, retained atomically with installation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InstalledVetting {
    pub attestation: maos_compliance::VettingAttestation,
    pub keyring_path: std::path::PathBuf,
}

impl InstalledVetting {
    pub fn verify(
        &self,
        pkg: &SignedPackage,
        root: &[u8; 32],
        now: u64,
    ) -> Result<
        maos_compliance::vetting::VerifiedAttestation,
        maos_compliance::vetting::VettingRejection,
    > {
        let bytes = std::fs::read(&self.keyring_path).map_err(|e| {
            maos_compliance::vetting::VettingRejection::MalformedClaim(e.to_string())
        })?;
        let keyring = serde_cbor::from_slice(&bytes).map_err(|e| {
            maos_compliance::vetting::VettingRejection::MalformedClaim(e.to_string())
        })?;
        maos_compliance::verify_attestation(
            &self.attestation,
            pkg.into(),
            &keyring,
            root,
            pkg.spirit_id.as_str(),
            &pkg.version,
            now,
        )
    }
}

/// Offline command entry point. Authentication is signatures, not a daemon bearer.
pub fn install(
    spec: &str,
    uri: &str,
    attestation: Option<&Path>,
    keyring: Option<&Path>,
) -> Result<serde_json::Value, Error> {
    let (id, version) = spec.rsplit_once('@').ok_or("expected <name>@<version>")?;
    crate::storage::path_segment(id)?;
    crate::storage::path_segment(version)?;
    if matches!(id, "hello-spirit" | "butler") {
        return Err("registry install name is reserved by the shell".into());
    }
    let config = resolve_config()?;
    let client = McpSpiritRegistryClient::http(uri.to_owned(), config.clone())?;
    let id = SpiritId::from(id);
    if let Some(entry) = client
        .yanks_since(0)?
        .entries
        .into_iter()
        .find(|entry| entry.spirit_id == id && entry.version == version)
    {
        return Err(maos_domain::ports::registry::RegistryError::Yanked {
            spirit_id: id.as_str().into(),
            version: version.into(),
            reason: entry.reason,
        }
        .into());
    }
    let manifest = client.manifest(&id, version)?;
    let artifact = client.artifact(&id, version)?;
    if manifest.spirit_id != id
        || artifact.spirit_id != id
        || manifest.version != version
        || artifact.version != version
        || manifest.signature != artifact.signature
        || manifest.signer_pubkey != artifact.signer_pubkey
    {
        return Err("registry manifest/artifact identity or signature mismatch".into());
    }
    let document: toml::Value = toml::from_str(std::str::from_utf8(&manifest.manifest_toml)?)?;
    check_identity(&document, id.as_str(), version)?;
    let pkg = SignedPackage::new(
        id,
        version.to_owned(),
        manifest.manifest_toml,
        artifact.artifact_bytes,
        manifest.signature,
        manifest.signer_pubkey,
        artifact
            .compliance_envelope
            .ok_or("registry artifact lacks signed ComplianceClaim")?,
    );
    let op_cfg = AdmissionConfig {
        tier_floor: config.tier_floor,
        registry_origin_tier: manifest.server_reported_tier.unwrap_or(TrustTier::Local),
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: config.org_signing_pubkey,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };
    // Signature verification is mandatory even when a remote manifest declares local.
    if !crate::admission::verify_publisher_sig(&pkg) {
        return Err(crate::admission::AdmissionError::PublisherSignatureInvalid.into());
    }
    let mut promotion = None;
    let decision = match (attestation, keyring) {
        (None, None) => admit_spirit(&pkg, &op_cfg)?,
        (Some(attestation), Some(keyring)) => {
            let attestation: maos_compliance::VettingAttestation =
                serde_cbor::from_slice(&std::fs::read(attestation)?)?;
            let keyring_path = std::fs::canonicalize(keyring)?;
            let keyring: maos_compliance::VetterKeyring =
                serde_cbor::from_slice(&std::fs::read(&keyring_path)?)?;
            let seed = maos_domain::audit_key::load_audit_key_seed(&None)?;
            let root = maos_domain::audit_key::derive_audit_public_key(&seed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64;
            promotion = Some(InstalledVetting {
                attestation: attestation.clone(),
                keyring_path,
            });
            admit_spirit_with_attestation(&pkg, &op_cfg, Some(&attestation), &keyring, &root, now)?
        }
        _ => {
            return Err(
                "--vetting-attestation and --vetter-keyring must be supplied together".into(),
            )
        }
    };
    let storage = LocalFsRegistryStorage::new()?;
    let origin = RegistryOrigin::Remote {
        uri: uri.to_owned(),
    };
    storage.install_package(&pkg, &origin, promotion.as_ref())?;
    Ok(serde_json::json!({
        "outcome": "installed", "spirit_id": pkg.spirit_id.as_str(), "version": pkg.version,
        "manifest_bytes": pkg.manifest_toml.len(), "artifact_bytes": pkg.artifact_bytes.len(),
        "effective_tier": format!("{:?}", decision.effective_tier),
        "sandbox_tier_floor": format!("{:?}", decision.sandbox_tier_floor), "origin": origin,
    }))
}
pub(crate) fn check_identity(document: &toml::Value, id: &str, version: &str) -> Result<(), Error> {
    let identity = document
        .get("class")
        .or_else(|| document.get("spirit"))
        .unwrap_or(document);
    if identity.get("name").and_then(toml::Value::as_str) != Some(id)
        || identity.get("version").and_then(toml::Value::as_str) != Some(version)
    {
        return Err("signed manifest identity differs from requested name/version".into());
    }
    Ok(())
}

/// Recheck the actual installed package at each boot, not merely its inventory
/// entry. Only return manifest bytes whose publisher, claim and component agree.
pub fn verify_installed(name: &str, manifest_path: &Path) -> Result<String, Error> {
    let directory = manifest_path.parent().ok_or("install has no directory")?;
    let pkg: SignedPackage =
        serde_json::from_slice(&std::fs::read(directory.join("signed_package.json"))?)?;
    crate::storage::path_segment(&pkg.version)?;
    if pkg.spirit_id.as_str() != name || !crate::admission::verify_publisher_sig(&pkg) {
        return Err("installed publisher signature or identity is invalid".into());
    }
    if std::fs::read(manifest_path)? != pkg.manifest_toml
        || std::fs::read(directory.join("artifact.bin"))? != pkg.artifact_bytes
    {
        return Err("installed manifest or artifact differs from signed bytes".into());
    }
    let text = std::str::from_utf8(&pkg.manifest_toml)?;
    let document: toml::Value = toml::from_str(text)?;
    check_identity(&document, name, &pkg.version)?;
    let manifest = maos_manifest::ManifestDocument::parse(&pkg.manifest_toml)?;
    if let Some(class) = manifest.class_section()? {
        if let Some(artifact) = class.artifact {
            if std::fs::read(directory.join(artifact))? != pkg.artifact_bytes {
                return Err("materialized component differs from signed bytes".into());
            }
        }
    }
    let mut fields =
        maos_compliance::runtime_context::extract_manifest_fingerprint_fields(&pkg.manifest_toml);
    let promotion_path = directory.join("vetting.json");
    if fields.trust_tier == TrustTier::PublicVetted && !promotion_path.exists() {
        return Err("installed public-vetted package lacks promotion evidence".into());
    }
    if promotion_path.exists() {
        let promotion: InstalledVetting = serde_json::from_slice(&std::fs::read(promotion_path)?)?;
        let seed = maos_domain::audit_key::load_audit_key_seed(&None)?;
        let root = maos_domain::audit_key::derive_audit_public_key(&seed);
        promotion.verify(&pkg, &root, crate::yank_auth::now_unix_ms()?)?;
        fields.trust_tier = TrustTier::PublicUntrusted;
    }
    let context = maos_compliance::RuntimeExecutionContext::from_admission(
        fields.trust_tier,
        fields.sandbox_tier,
        &pkg,
        &fields.provider_endpoint,
        &fields.crypto_provider,
    );
    if let maos_compliance::ComplianceVerdict::Reject(reason) =
        maos_compliance::evaluate_envelope(&pkg.compliance_envelope, &context)
    {
        return Err(format!("installed ComplianceClaim refused: {reason:?}").into());
    }
    Ok(String::from_utf8(pkg.manifest_toml)?)
}

/// Same [registry] keys and env precedence as the daemon, with fail-closed parsing.
fn resolve_config() -> Result<RegistryClientConfig, Error> {
    let path = std::path::PathBuf::from(std::env::var("HOME")?).join(".config/maos/operator.toml");
    let disk: toml::Value = match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            toml::Value::Table(Default::default())
        }
        Err(e) => return Err(e.into()),
    };
    let value = |env: &str, key: &str| -> Result<Option<String>, Error> {
        if let Some(value) = std::env::var_os(env) {
            return Ok(Some(
                value
                    .into_string()
                    .map_err(|_| "registry policy is not UTF-8")?,
            ));
        }
        Ok(disk.get("registry").and_then(|r| r.get(key)).map(|v| {
            v.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string())
        }))
    };
    let tier_floor = match value("MAOS_REGISTRY_TIER_FLOOR", "tier_floor")?
        .as_deref()
        .unwrap_or("local")
    {
        "local" => TrustTier::Local,
        "org-internal" | "org_internal" => TrustTier::OrgInternal,
        "public-untrusted" | "public_untrusted" => TrustTier::PublicUntrusted,
        "public-vetted" | "public_vetted" => TrustTier::PublicVetted,
        _ => return Err("invalid registry tier floor".into()),
    };
    let org_signing_pubkey = value("MAOS_REGISTRY_ORG_SIGNING_PUBKEY", "org_signing_pubkey")?
        .map(|v| -> Result<[u8; 32], Error> {
            Ok(hex::decode(v)?
                .try_into()
                .map_err(|_| "org signing key must be 32 bytes")?)
        })
        .transpose()?;
    let required = match value(
        "MAOS_REGISTRY_REQUIRE_SERVER_TIER_SIGNATURE",
        "require_server_tier_signature",
    )?
    .as_deref()
    .unwrap_or("false")
    {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => return Err("invalid require_server_tier_signature boolean".into()),
    };
    Ok(RegistryClientConfig {
        tier_floor,
        require_server_tier_signature: required || org_signing_pubkey.is_some(),
        org_signing_pubkey,
    })
}
