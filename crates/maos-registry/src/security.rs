//! AC15 trust roots are operator configuration, never data supplied by a registry.
use maos_domain::ports::io_subsystem::IoError;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const NONCE_HEADER: &str = "X-Maos-Registry-Nonce";
pub const RESPONSE_HEADER: &str = "X-Maos-Registry-Response";
pub const DEVELOPMENT_HEADER: &str = "X-Maos-Registry-Development";

#[derive(Debug, Clone, Default)]
pub struct RegistrySecurityConfig {
    pub uri: Option<String>,
    pub signing_pubkey: Option<[u8; 32]>,
    pub ca_certificate: Option<PathBuf>,
    pub development_http: bool,
}
impl RegistrySecurityConfig {
    /// Explicit opt-in for local development; not an authenticated trust root.
    pub fn development_loopback() -> Self {
        Self {
            development_http: true,
            ..Self::default()
        }
    }
    pub(crate) fn from_operator(value: Option<&toml::Value>) -> Result<Self, String> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Disk {
            uri: Option<String>,
            signing_pubkey: Option<String>,
            ca_certificate: Option<PathBuf>,
            development_http: bool,
        }
        let disk: Disk = value
            .cloned()
            .map(toml::Value::try_into)
            .transpose()
            .map_err(|e| format!("registry security configuration: {e}"))?
            .unwrap_or_default();
        Ok(Self {
            uri: disk.uri,
            signing_pubkey: disk
                .signing_pubkey
                .map(|key| {
                    hex::decode(key)
                        .map_err(|e| e.to_string())?
                        .try_into()
                        .map_err(|_| "registry signing_pubkey must be 32 bytes".to_owned())
                })
                .transpose()?,
            ca_certificate: disk.ca_certificate,
            development_http: disk.development_http,
        })
    }
}

/// Deliberately one spelling per origin; no credentials, query, fragment or aliases.
pub fn canonical_uri(uri: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(uri).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/mcp"
        || url.as_str() != uri
    {
        return Err("registry URI must be canonical http(s)://host[:port]/mcp".into());
    }
    Ok(url)
}
pub(crate) fn literal_loopback(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}
pub(crate) fn authentication(reason: impl std::fmt::Display) -> IoError {
    IoError::Transport(format!("registry response authentication: {reason}"))
}
pub(crate) fn nonce() -> Result<[u8; 32], IoError> {
    let mut nonce = [0; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(authentication)?;
    Ok(nonce)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    version: u8,
    uri: String,
    nonce: [u8; 32],
    request_sha256: [u8; 32],
    response_sha256: [u8; 32],
    yank_head: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    claim: Claim,
    signature: Vec<u8>,
}
fn message(claim: &Claim) -> Result<[u8; 32], String> {
    let wire = serde_cbor::to_vec(claim).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(b"maos.registry.response.v1\0");
    hash.update(wire);
    Ok(hash.finalize().into())
}
pub(crate) fn sign_response(
    uri: &str,
    nonce: [u8; 32],
    request: &[u8],
    response: &[u8],
    yank_head: Option<u64>,
    seed: &[u8; 32],
) -> Result<String, String> {
    let claim = Claim {
        version: 1,
        uri: uri.into(),
        nonce,
        request_sha256: Sha256::digest(request).into(),
        response_sha256: Sha256::digest(response).into(),
        yank_head,
    };
    let key =
        ring::signature::Ed25519KeyPair::from_seed_unchecked(seed).map_err(|e| e.to_string())?;
    let signature = key.sign(&message(&claim)?).as_ref().to_vec();
    Ok(hex::encode(
        serde_cbor::to_vec(&Envelope { claim, signature }).map_err(|e| e.to_string())?,
    ))
}
pub(crate) fn verify_response(
    uri: &str,
    nonce: [u8; 32],
    request: &[u8],
    response: &[u8],
    header: Option<&str>,
    key: &[u8; 32],
) -> Result<(), IoError> {
    let encoded =
        hex::decode(header.ok_or_else(|| authentication("missing signed envelope/head"))?)
            .map_err(authentication)?;
    let envelope: Envelope = serde_cbor::from_slice(&encoded).map_err(authentication)?;
    let claim = &envelope.claim;
    if claim.version != 1
        || claim.uri != uri
        || claim.nonce != nonce
        || claim.request_sha256 != <[u8; 32]>::from(Sha256::digest(request))
        || claim.response_sha256 != <[u8; 32]>::from(Sha256::digest(response))
    {
        return Err(authentication(
            "URI, nonce, request or response binding differs",
        ));
    }
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
        .verify(
            &message(claim).map_err(authentication)?,
            &envelope.signature,
        )
        .map_err(authentication)?;
    verify_head(request, response, claim.yank_head)
}
fn verify_head(request: &[u8], response: &[u8], head: Option<u64>) -> Result<(), IoError> {
    // Inspect only routing/cursor fields. In particular, never materialize a
    // second JSON Value containing a publication's potentially large artifact.
    #[derive(Deserialize)]
    struct Cursor {
        since_ns: Option<u64>,
    }
    #[derive(Deserialize)]
    struct Params<'a> {
        #[serde(borrow)]
        name: Option<&'a str>,
        arguments: Option<Cursor>,
        since_ns: Option<u64>,
    }
    #[derive(Deserialize)]
    struct Request<'a> {
        method: &'a str,
        #[serde(borrow)]
        params: Params<'a>,
    }
    let request: Request<'_> = serde_json::from_slice(request).map_err(authentication)?;
    let since = if request.method == "registry.yanks_since" {
        request.params.since_ns
    } else if request.method == "tools/call" && request.params.name == Some("registry.yanks_since")
    {
        request.params.arguments.and_then(|args| args.since_ns)
    } else {
        return Ok(());
    };
    #[derive(Deserialize)]
    struct Sequence {
        sequence: u64,
    }
    #[derive(Deserialize)]
    struct List {
        entries: Vec<Sequence>,
    }
    #[derive(Deserialize)]
    struct Response {
        result: Option<List>,
    }
    let response: Response = serde_json::from_slice(response).map_err(authentication)?;
    let Some(list) = response.result else {
        return Ok(());
    };
    let mut next = since.ok_or_else(|| authentication("missing yank cursor"))?;
    for entry in list.entries {
        next = next
            .checked_add(1)
            .ok_or_else(|| authentication("yank sequence overflow"))?;
        if entry.sequence != next {
            return Err(authentication("yank list gap or duplicate"));
        }
    }
    if head != Some(next) {
        return Err(authentication(
            "yank list does not reach authenticated head",
        ));
    }
    Ok(())
}

pub(crate) fn private_key_file(path: &Path) -> Result<std::fs::File, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("registry private key is not a regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 || metadata.uid() != rustix::process::geteuid().as_raw() {
            return Err("registry private key must be operator-owned, mode0600 or tighter".into());
        }
    }
    Ok(file)
}
pub(crate) fn key_outside_store(path: &Path, root: &Path) -> Result<(), String> {
    let key = path.canonicalize().map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if key.starts_with(root) {
        return Err("registry private key must live outside the package/inventory store".into());
    }
    Ok(())
}

#[cfg(feature = "tls")]
pub(crate) fn certificates(
    path: &Path,
) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>, String> {
    let mut input = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    let certificates = rustls_pemfile::certs(&mut input)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if certificates.is_empty() {
        return Err("empty certificate chain".into());
    }
    Ok(certificates)
}
#[cfg(feature = "tls")]
pub(crate) fn client_tls(
    ca: Option<&Path>,
) -> Result<std::sync::Arc<rustls::ClientConfig>, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Some(path) = ca {
        for certificate in certificates(path)? {
            roots.add(certificate).map_err(|e| e.to_string())?;
        }
    }
    let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(std::sync::Arc::new(config))
}
#[cfg(feature = "tls")]
pub(crate) fn server_tls(
    cert: &Path,
    key: &Path,
) -> Result<std::sync::Arc<rustls::ServerConfig>, String> {
    let mut input = std::io::BufReader::new(private_key_file(key)?);
    let key = rustls_pemfile::private_key(&mut input)
        .map_err(|e| e.to_string())?
        .ok_or("missing TLS private key")?;
    let config = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_no_client_auth()
    .with_single_cert(certificates(cert)?, key)
    .map_err(|e| e.to_string())?;
    Ok(std::sync::Arc::new(config))
}
