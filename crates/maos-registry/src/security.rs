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
        if let Some(table) = value.and_then(toml::Value::as_table) {
            const KEYS: &[&str] = &[
                "uri",
                "tier_floor",
                "t3_for_public_untrusted",
                "allow_unsigned_local",
                "org_signing_pubkey",
                "require_server_tier_signature",
                "allow_force_tier_at_import",
                "signing_pubkey",
                "ca_certificate",
                "development_http",
            ];
            if let Some(key) = table.keys().find(|key| !KEYS.contains(&key.as_str())) {
                return Err(format!("unknown registry key {key:?}"));
            }
        }
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
                        .map_err(|_| "registry signing_pubkey must be 32-byte hex")?
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
        || url
            .host_str()
            .is_some_and(|host| host.ends_with('.') || host.contains(".."))
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
pub(crate) struct RequestRoute<'a> {
    pub method: &'a str,
    pub since: Option<u64>,
}
/// Borrow routing fields, skipping publication payloads without a second JSON Value.
pub(crate) fn request_route(request: &[u8]) -> Result<RequestRoute<'_>, IoError> {
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
        #[serde(default, borrow)]
        params: Option<Params<'a>>,
    }
    let request: Request<'_> = serde_json::from_slice(request).map_err(authentication)?;
    if request.method == "tools/call" {
        let params = request
            .params
            .ok_or_else(|| authentication("missing tool routing"))?;
        Ok(RequestRoute {
            method: params
                .name
                .ok_or_else(|| authentication("missing tool name"))?,
            since: params.arguments.and_then(|arguments| arguments.since_ns),
        })
    } else {
        Ok(RequestRoute {
            method: request.method,
            since: request.params.and_then(|params| params.since_ns),
        })
    }
}
fn verify_head(request: &[u8], response: &[u8], head: Option<u64>) -> Result<(), IoError> {
    let route = request_route(request)?;
    if route.method != "registry.yanks_since" {
        return Ok(());
    }
    #[derive(Deserialize)]
    struct Sequence {
        sequence: u64,
    }
    #[derive(Deserialize)]
    struct List {
        entries: Vec<Sequence>,
        head: u64,
    }
    #[derive(Deserialize)]
    struct Response {
        result: Option<List>,
    }
    let response: Response = serde_json::from_slice(response).map_err(authentication)?;
    let Some(list) = response.result else {
        return Ok(());
    };
    let head = head.ok_or_else(|| authentication("missing authenticated yank head"))?;
    if list.head != head {
        return Err(authentication(
            "payload head differs from authenticated head",
        ));
    }
    let mut next = route
        .since
        .ok_or_else(|| authentication("missing yank cursor"))?;
    // A valid empty historical view is authenticated, but semantically a rollback.
    // Preserve that distinction for the registry client's typed boundary error.
    if next > head && list.entries.is_empty() {
        return Ok(());
    }
    for entry in list.entries {
        next = next
            .checked_add(1)
            .ok_or_else(|| authentication("yank sequence overflow"))?;
        if entry.sequence != next {
            return Err(authentication("yank list gap or duplicate"));
        }
    }
    if head != next {
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
    let roots = client_roots(ca)?;
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
fn client_roots(ca: Option<&Path>) -> Result<rustls::RootCertStore, String> {
    let mut roots = rustls::RootCertStore::empty();
    if let Some(path) = ca {
        for certificate in certificates(path)? {
            roots.add(certificate).map_err(|e| e.to_string())?;
        }
    } else {
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    Ok(roots)
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

#[cfg(test)]
mod review_tests {
    use super::*;

    #[test]
    fn registry_uri_matrix_and_trailing_dns_labels_p3_p9() {
        for uri in [
            "https://REGISTRY.example/mcp",
            "https://registry.example:443/mcp",
            "https://registry.example./mcp",
            "https://registry..example/mcp",
            "https://registry.example/mcp/",
            "https://registry.example/%6dcp",
            "https://user:password@registry.example/mcp",
        ] {
            assert!(
                canonical_uri(uri).is_err(),
                "non-canonical URI accepted: {uri}"
            );
        }
    }

    #[test]
    fn explicit_development_literal_loopback_matrix_p3() {
        for (host, allowed) in [
            ("127.0.0.1", true),
            ("127.0.0.2", true),
            ("[::1]", true),
            ("0.0.0.0", false),
            ("[::ffff:127.0.0.1]", false),
            ("localhost", false),
        ] {
            let accepted = canonical_uri(&format!("http://{host}/mcp"))
                .is_ok_and(|uri| literal_loopback(&uri));
            assert_eq!(accepted, allowed, "{host}");
        }
    }

    #[test]
    fn registry_typo_is_refused_p11() {
        let value = toml::Value::Table(toml::map::Map::from_iter([(
            "signing_pubky".into(),
            toml::Value::String("typo".into()),
        )]));
        assert!(
            RegistrySecurityConfig::from_operator(Some(&value)).is_err(),
            "unknown registry security keys must not silently disappear"
        );
    }

    #[test]
    fn registry_hex_diagnostics_do_not_echo_input_p8() {
        let value = toml::Value::Table(toml::map::Map::from_iter([(
            "signing_pubkey".into(),
            toml::Value::String("!".repeat(64)),
        )]));
        let error = RegistrySecurityConfig::from_operator(Some(&value)).unwrap_err();
        assert!(
            !error.contains('!'),
            "hex diagnostic leaked offending input: {error}"
        );
    }

    #[test]
    fn signed_params_less_notification_verifies_p7() {
        let request = br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let response = br#"{"jsonrpc":"2.0","id":null,"result":{}}"#;
        let seed = [0x91; 32];
        let pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        use ring::signature::KeyPair;
        let key: [u8; 32] = pair.public_key().as_ref().try_into().unwrap();
        let header = sign_response(
            "https://registry.example/mcp",
            [7; 32],
            request,
            response,
            None,
            &seed,
        )
        .unwrap();
        assert!(verify_response(
            "https://registry.example/mcp",
            [7; 32],
            request,
            response,
            Some(&header),
            &key
        )
        .is_ok());
    }

    #[cfg(feature = "tls")]
    #[test]
    fn configured_ca_replaces_bundled_roots_d4() {
        let directory = tempfile::tempdir().unwrap();
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let ca = params.self_signed(&key).unwrap();
        let path = directory.path().join("operator-ca.pem");
        std::fs::write(&path, ca.pem()).unwrap();
        assert_eq!(
            client_roots(Some(&path)).unwrap().len(),
            1,
            "a configured private CA must replace, not extend, bundled Mozilla roots"
        );
        assert_eq!(
            client_roots(None).unwrap().len(),
            webpki_roots::TLS_SERVER_ROOTS.len()
        );
    }
}
