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
    let config = operator_config()?;
    let client = McpSpiritRegistryClient::http(uri.to_owned(), config.clone())?;
    let id = SpiritId::from(id);
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
        t3_for_public_untrusted: config.t3_for_public_untrusted,
        allow_unsigned_local: config.allow_unsigned_local,
        org_signing_pubkey: config.org_signing_pubkey,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };
    // Signature verification is mandatory even when a remote manifest declares local.
    if !crate::admission::verify_publisher_sig(&pkg) {
        return Err(crate::admission::AdmissionError::PublisherSignatureInvalid.into());
    }
    // HTTPS verifies the signed response envelope and complete yank head before
    // returning this list. Explicit unsigned loopback development cannot do so.
    if let Some(entry) = client
        .yanks_since(0)?
        .entries
        .into_iter()
        .find(|entry| entry.spirit_id == pkg.spirit_id && entry.version == pkg.version)
    {
        let vetters: Option<maos_compliance::VetterKeyring> = keyring
            .map(|path| -> Result<_, Error> { Ok(serde_cbor::from_slice(&std::fs::read(path)?)?) })
            .transpose()?;
        let root = if vetters.is_some() {
            Some(maos_domain::audit_key::derive_audit_public_key(
                &maos_domain::audit_key::load_audit_key_seed(&None)?,
            ))
        } else {
            None
        };
        crate::yank_auth::verify_entry(
            &entry,
            &pkg.publisher_pubkey,
            vetters.as_ref().zip(root.as_ref()),
            uri,
            crate::yank_auth::now_unix_ms()?,
        )?;
        return Err(maos_domain::ports::registry::RegistryError::Yanked {
            spirit_id: pkg.spirit_id.as_str().into(),
            version: pkg.version.clone(),
            reason: entry.reason,
        }
        .into());
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

/// Launch-facing result: the one open, read-only, regular component file that
/// was compared byte-for-byte with the signed artifact, bound to the signed
/// package it came from. Deliberately not `Clone`: there is exactly one holder.
/// The path it was acquired through is never a recovery input.
#[derive(Debug)]
pub struct VerifiedLaunchComponent {
    file: std::fs::File,
    artifact_sha256: [u8; 32],
    spirit_id: SpiritId,
    version: String,
    manifest_sha256: [u8; 32],
    publisher: [u8; 32],
}

impl VerifiedLaunchComponent {
    /// SHA-256 of `SignedPackage.artifact_bytes`, the exact authorized bytes.
    pub fn artifact_sha256(&self) -> [u8; 32] {
        self.artifact_sha256
    }
    /// SHA-256 of the signed `manifest_toml` this component is bound to.
    pub fn manifest_sha256(&self) -> [u8; 32] {
        self.manifest_sha256
    }
    /// Signed package version this component is bound to.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Signed package identity this component is bound to.
    pub fn spirit_id(&self) -> &SpiritId {
        &self.spirit_id
    }
    /// The publisher key that signed the package this component was verified under.
    pub fn publisher(&self) -> [u8; 32] {
        self.publisher
    }
    /// Surrender the held file together with the signed-byte digest.
    pub fn into_parts(self) -> (std::fs::File, [u8; 32]) {
        (self.file, self.artifact_sha256)
    }
}

/// This platform has no defensive origin-relative component acquisition, so no
/// launch-facing held component can be produced. Distinct from verification
/// failure: downcast the boxed error to this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldComponentUnsupported;

impl std::fmt::Display for HeldComponentUnsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("held component launch is unsupported on this platform")
    }
}

impl std::error::Error for HeldComponentUnsupported {}

/// Installed boot result: verified manifest text, publisher, and the held
/// component when the manifest names one (native spirits have none).
#[derive(Debug)]
pub struct InstalledLaunch {
    pub manifest_toml: String,
    pub publisher: [u8; 32],
    pub component: Option<VerifiedLaunchComponent>,
}

/// A registry subject is the signed package AND its deployed bytes, not a
/// neighboring metadata file. Local manifests without sidecars remain local.
pub fn signed_package_at(name: &str, manifest_path: &Path) -> Result<Option<SignedPackage>, Error> {
    signed_package_at_with(name, manifest_path, None).map(|found| found.map(|(pkg, _)| pkg))
}

/// [`signed_package_at`] for launch: also returns the held component, from the
/// same single verification (no second open).
pub fn signed_package_at_launch(
    name: &str,
    manifest_path: &Path,
) -> Result<Option<(SignedPackage, Option<VerifiedLaunchComponent>)>, Error> {
    match signed_package_at_with(name, manifest_path, None)? {
        None => Ok(None),
        Some((pkg, file)) => {
            let component = launch_component(&pkg, file)?;
            Ok(Some((pkg, component)))
        }
    }
}

fn signed_package_at_with(
    name: &str,
    manifest_path: &Path,
    crypto: Option<&dyn maos_domain::ports::CryptoProvider>,
) -> Result<Option<(SignedPackage, Option<std::fs::File>)>, Error> {
    let directory = manifest_path
        .parent()
        .ok_or("manifest has no package directory")?;
    let bytes = match std::fs::read(directory.join("signed_package.json")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let pkg: SignedPackage = serde_json::from_slice(&bytes)?;
    if pkg.spirit_id.as_str() != name {
        return Err("installed package name differs".into());
    }
    let file = verify_deployed_package_with(&pkg, manifest_path, crypto)?;
    Ok(Some((pkg, file)))
}

/// Validate an explicit immutable subject against its original deployed files.
/// A temporary manifest snapshot is never a package-identity lookup location.
pub fn verify_deployed_package(pkg: &SignedPackage, manifest_path: &Path) -> Result<(), Error> {
    verify_deployed_package_with(pkg, manifest_path, None).map(drop)
}

#[cfg(unix)]
fn launch_component(
    pkg: &SignedPackage,
    file: Option<std::fs::File>,
) -> Result<Option<VerifiedLaunchComponent>, Error> {
    use sha2::Digest;
    let Some(file) = file else { return Ok(None) };
    Ok(Some(VerifiedLaunchComponent {
        file,
        artifact_sha256: sha2::Sha256::digest(&pkg.artifact_bytes).into(),
        spirit_id: pkg.spirit_id.clone(),
        version: pkg.version.clone(),
        manifest_sha256: sha2::Sha256::digest(&pkg.manifest_toml).into(),
        publisher: pkg.publisher_pubkey,
    }))
}

#[cfg(not(unix))]
fn launch_component(
    _pkg: &SignedPackage,
    file: Option<std::fs::File>,
) -> Result<Option<VerifiedLaunchComponent>, Error> {
    match file {
        None => Ok(None),
        Some(_) => Err(Box::new(HeldComponentUnsupported)),
    }
}

fn matches_reader(mut file: impl std::io::Read, expected: &[u8]) -> std::io::Result<bool> {
    let mut buffer = [0; 8192];
    for chunk in expected.chunks(buffer.len()) {
        file.read_exact(&mut buffer[..chunk.len()])?;
        if &buffer[..chunk.len()] != chunk {
            return Ok(false);
        }
    }
    Ok(file.read(&mut buffer[..1])? == 0)
}

/// Open the manifest-named component relative to the trusted package directory,
/// one segment at a time with no symlink following, so neither a symlink nor a
/// racing rename can move the opened object outside the origin. The result is a
/// read-only regular file. Segments split on `/` exactly as materialization
/// joins them; `..` is refused again here.
#[cfg(unix)]
fn open_component(directory: &Path, artifact: &str) -> Result<std::fs::File, Error> {
    use rustix::fs::{fcntl_setfl, openat, Mode, OFlags};
    let segments: Vec<&str> = artifact
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    if segments.is_empty() || segments.contains(&"..") || artifact.contains('\0') {
        return Err("component path escapes the package directory".into());
    }
    let mut current: std::os::fd::OwnedFd = rustix::fs::open(
        directory,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let last = segments.len() - 1;
    for (index, segment) in segments.iter().enumerate() {
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        flags |= if index == last {
            OFlags::NONBLOCK
        } else {
            OFlags::DIRECTORY
        };
        current = openat(&current, *segment, flags, Mode::empty())?;
    }
    let file = std::fs::File::from(current);
    if !file.metadata()?.is_file() {
        return Err("component is not a regular file".into());
    }
    // NONBLOCK only guarded the open against special files; the held handle is shared.
    fcntl_setfl(&file, OFlags::empty())?;
    Ok(file)
}

#[cfg(not(unix))]
fn open_component(directory: &Path, artifact: &str) -> Result<std::fs::File, Error> {
    Ok(std::fs::File::open(directory.join(artifact))?)
}

fn verify_deployed_package_with(
    pkg: &SignedPackage,
    manifest_path: &Path,
    crypto: Option<&dyn maos_domain::ports::CryptoProvider>,
) -> Result<Option<std::fs::File>, Error> {
    fn matches(path: &Path, expected: &[u8]) -> std::io::Result<bool> {
        matches_reader(std::fs::File::open(path)?, expected)
    }
    let directory = manifest_path.parent().ok_or("package has no directory")?;
    crate::storage::path_segment(&pkg.version)?;
    if !crypto.map_or_else(
        || crate::admission::verify_publisher_sig(pkg),
        |provider| crate::admission::verify_publisher_sig_with(pkg, provider),
    ) {
        return Err("installed publisher signature is invalid".into());
    }
    if !matches(manifest_path, &pkg.manifest_toml)?
        || !matches(&directory.join("artifact.bin"), &pkg.artifact_bytes)?
    {
        return Err("installed manifest or artifact differs from signed bytes".into());
    }
    let document: toml::Value = toml::from_str(std::str::from_utf8(&pkg.manifest_toml)?)?;
    check_identity(&document, pkg.spirit_id.as_str(), &pkg.version)?;
    let manifest = maos_manifest::ManifestDocument::parse(&pkg.manifest_toml)?;
    if let Some(artifact) = manifest.class_section()?.and_then(|class| class.artifact) {
        use std::io::Seek;
        let mut file = open_component(directory, &artifact)?;
        if !matches_reader(&mut file, &pkg.artifact_bytes)? {
            return Err("materialized component differs from signed bytes".into());
        }
        file.rewind()?;
        return Ok(Some(file));
    }
    Ok(None)
}

/// Only an inventory-owned target supplies a vetted upgrade subject.
pub fn installed_upgrade_target(name: &str, manifest: &Path) -> Result<SignedPackage, Error> {
    let canonical = std::fs::canonicalize(manifest)?;
    for (id, installed) in LocalFsRegistryStorage::default_installed_manifests()? {
        if id == name && std::fs::canonicalize(&installed)? == canonical {
            return signed_package_at(name, &installed)?
                .ok_or("installed upgrade target lacks signed package".into());
        }
    }
    Err("vetted upgrade target is not an installed registry package".into())
}

/// One identity format shared by admission and uninstall's erasure scope.
pub fn publisher_storage_identity(name: &str, publisher: &[u8; 32]) -> String {
    use sha2::Digest;
    fn append_hex(out: &mut String, bytes: &[u8]) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 15) as usize] as char);
        }
    }
    let mut identity = String::with_capacity(if name.len() <= 93 {
        name.len() + 33
    } else {
        65
    });
    if name.len() <= 93 {
        identity.push_str(name);
    } else {
        append_hex(&mut identity, &sha2::Sha256::digest(name.as_bytes())[..16]);
    }
    identity.push(':');
    append_hex(&mut identity, &sha2::Sha256::digest(publisher)[..16]);
    identity
}

/// Recheck the actual installed package at each boot, not merely its inventory
/// entry. Only return manifest bytes whose publisher, claim and component agree.
pub fn verify_installed<'a>(
    name: &str,
    manifest_path: &Path,
    crypto: &dyn maos_domain::ports::CryptoProvider,
    crypto_name: &str,
    endpoint: &dyn Fn(&str) -> Option<&'a str>,
) -> Result<(String, [u8; 32]), Error> {
    let (pkg, _file) = verify_installed_inner(name, manifest_path, crypto, crypto_name, endpoint)?;
    Ok((String::from_utf8(pkg.manifest_toml)?, pkg.publisher_pubkey))
}

/// [`verify_installed`] for launch: identical checks (same implementation), and
/// the component compared against the signed bytes comes back held. A manifest
/// that names a component on a platform without defensive acquisition fails
/// with a downcastable [`HeldComponentUnsupported`].
pub fn verify_installed_launch<'a>(
    name: &str,
    manifest_path: &Path,
    crypto: &dyn maos_domain::ports::CryptoProvider,
    crypto_name: &str,
    endpoint: &dyn Fn(&str) -> Option<&'a str>,
) -> Result<InstalledLaunch, Error> {
    let (pkg, file) = verify_installed_inner(name, manifest_path, crypto, crypto_name, endpoint)?;
    let component = launch_component(&pkg, file)?;
    Ok(InstalledLaunch {
        publisher: pkg.publisher_pubkey,
        manifest_toml: String::from_utf8(pkg.manifest_toml)?,
        component,
    })
}

fn verify_installed_inner<'a>(
    name: &str,
    manifest_path: &Path,
    crypto: &dyn maos_domain::ports::CryptoProvider,
    crypto_name: &str,
    endpoint: &dyn Fn(&str) -> Option<&'a str>,
) -> Result<(SignedPackage, Option<std::fs::File>), Error> {
    let directory = manifest_path.parent().ok_or("install has no directory")?;
    let (pkg, file) = signed_package_at_with(name, manifest_path, Some(crypto))?
        .ok_or("installed signed package is absent")?;
    let mut fields =
        maos_compliance::runtime_context::extract_manifest_fingerprint_fields(&pkg.manifest_toml);
    let promotion = match std::fs::read(directory.join("vetting.json")) {
        Ok(bytes) => Some(serde_json::from_slice::<InstalledVetting>(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    if fields.trust_tier == TrustTier::PublicVetted && promotion.is_none() {
        return Err("installed public-vetted package lacks promotion evidence".into());
    }
    if let Some(promotion) = promotion {
        let seed = maos_domain::audit_key::load_audit_key_seed(&None)?;
        let root = maos_domain::audit_key::derive_audit_public_key(&seed);
        promotion.verify(&pkg, &root, crate::yank_auth::now_unix_ms()?)?;
        fields.trust_tier = TrustTier::PublicUntrusted;
    }
    let mut runtime_endpoint = fields.provider_endpoint.clone();
    runtime_endpoint.endpoint_url = endpoint(&runtime_endpoint.provider_id)
        .unwrap_or("")
        .to_owned();
    let runtime_crypto = maos_spirit_abi::compliance::CryptoProviderId(crypto_name.to_owned());
    let context = maos_compliance::RuntimeExecutionContext::from_admission(
        fields.trust_tier,
        fields.sandbox_tier,
        &pkg,
        &runtime_endpoint,
        &runtime_crypto,
    );
    if let maos_compliance::ComplianceVerdict::Reject(reason) =
        maos_compliance::evaluate_envelope_with(&pkg.compliance_envelope, &context, crypto)
    {
        return Err(format!("installed ComplianceClaim refused: {reason:?}").into());
    }
    Ok((pkg, file))
}

/// Same [registry] keys and env precedence as the daemon, with fail-closed parsing.
pub fn operator_config() -> Result<RegistryClientConfig, Error> {
    let path = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| std::path::PathBuf::from(home).join(".config/maos/operator.toml"));
    let disk: toml::Value = match path.map(std::fs::read_to_string).transpose() {
        Ok(Some(text)) => toml::from_str(&text)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            toml::Value::Table(Default::default())
        }
        Err(e) => return Err(e.into()),
        Ok(None) => toml::Value::Table(Default::default()),
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
            Ok(hex::decode(v)
                .map_err(|_| "org signing key must be 32-byte hex")?
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
    let boolean = |env: &str, key: &str, default: bool| -> Result<bool, Error> {
        match value(env, key)?.as_deref() {
            None => Ok(default),
            Some("true" | "1") => Ok(true),
            Some("false" | "0") => Ok(false),
            Some(_) => Err(format!("invalid registry {key} boolean").into()),
        }
    };
    Ok(RegistryClientConfig {
        uri: value("MAOS_REGISTRY_URI", "uri")?.unwrap_or_default(),
        t3_for_public_untrusted: boolean(
            "MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED",
            "t3_for_public_untrusted",
            false,
        )?,
        allow_unsigned_local: boolean(
            "MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL",
            "allow_unsigned_local",
            true,
        )?,
        tier_floor,
        require_server_tier_signature: required || org_signing_pubkey.is_some(),
        org_signing_pubkey,
        security: crate::security::RegistrySecurityConfig::from_operator(disk.get("registry"))?,
    })
}

#[cfg(all(test, unix))]
mod held_component_tests {
    use super::open_component;
    use std::io::Read;

    #[test]
    fn component_is_opened_inside_origin_without_following_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("wasm")).unwrap();
        std::fs::write(root.path().join("wasm/a.wasm"), b"inside").unwrap();
        std::fs::write(outside.path().join("b.wasm"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("b.wasm"), root.path().join("leaf.wasm"))
            .unwrap();

        let mut held = open_component(root.path(), "wasm/a.wasm").unwrap();
        let mut text = String::new();
        held.read_to_string(&mut text).unwrap();
        assert_eq!(text, "inside");
        assert!(open_component(root.path(), "link/b.wasm").is_err());
        assert!(open_component(root.path(), "leaf.wasm").is_err());
        assert!(open_component(root.path(), "../x.wasm").is_err());
        assert!(open_component(root.path(), "wasm").is_err());
    }
}
