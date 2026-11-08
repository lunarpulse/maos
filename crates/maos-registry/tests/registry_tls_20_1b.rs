//! AC15: authenticated first contact, transport and explicit development boundaries.
use maos_registry::{client::RegistryClientConfig, McpSpiritRegistryClient, SpiritRegistryServer};
use std::sync::{atomic::AtomicBool, Arc};
#[cfg(feature = "tls")]
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[cfg(feature = "tls")]
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;

#[test]
fn http_install_requires_explicit_development_opt_in_ac15() {
    assert!(
        McpSpiritRegistryClient::http(
            "http://127.0.0.1:6789/mcp".into(),
            RegistryClientConfig::default(),
        )
        .is_err(),
        "HTTP install without explicit development opt-in was accepted"
    );
}

#[test]
fn server_requires_tls_or_explicit_loopback_development_ac15() {
    let dir = tempfile::tempdir().unwrap();
    let storage =
        maos_registry::storage::LocalFsRegistryStorage::at_path(dir.path().to_path_buf()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let server = SpiritRegistryServer::new(Arc::new(storage), "127.0.0.1:0".into(), None);
    assert!(
        server
            .serve(listener, Arc::new(AtomicBool::new(true)))
            .is_err(),
        "server silently enabled unauthenticated development transport"
    );
}

#[cfg(feature = "tls")]
mod tls {
    use super::*;
    use maos_domain::ports::registry::{SpiritId, SpiritRegistryClient, YankOrigin};
    use maos_registry::{
        security::RegistrySecurityConfig,
        storage::{LocalFsRegistryStorage, RegistryStorage},
    };
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    const REGISTRY: [u8; 32] = [0x91; 32];
    struct Certificates {
        _dir: tempfile::TempDir,
        ca: PathBuf,
        cert: PathBuf,
        key: PathBuf,
    }
    impl Certificates {
        fn new(host: &str, expired: bool) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let ca_key = rcgen::KeyPair::generate().unwrap();
            let mut ca = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
            ca.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
            let ca = ca.self_signed(&ca_key).unwrap();
            let key = rcgen::KeyPair::generate().unwrap();
            let mut leaf = rcgen::CertificateParams::new(vec![host.to_owned()]).unwrap();
            if expired {
                leaf.not_before = rcgen::date_time_ymd(2000, 1, 1);
                leaf.not_after = rcgen::date_time_ymd(2001, 1, 1);
            }
            let leaf = leaf.signed_by(&key, &ca, &ca_key).unwrap();
            let ca_path = dir.path().join("test-ca.pem");
            let cert_path = dir.path().join("test-leaf.pem");
            let key_path = dir.path().join("test-only-key.pem");
            std::fs::write(&ca_path, ca.pem()).unwrap();
            std::fs::write(&cert_path, leaf.pem()).unwrap();
            std::fs::write(&key_path, key.serialize_pem()).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                    .unwrap();
            }
            Self {
                _dir: dir,
                ca: ca_path,
                cert: cert_path,
                key: key_path,
            }
        }
        fn client_config(&self, uri: &str) -> RegistryClientConfig {
            RegistryClientConfig {
                security: RegistrySecurityConfig {
                    uri: Some(uri.into()),
                    signing_pubkey: Some(trust::public(&REGISTRY)),
                    ca_certificate: Some(self.ca.clone()),
                    development_http: false,
                },
                ..Default::default()
            }
        }
        fn server_config(&self) -> Arc<rustls::ServerConfig> {
            let cert = rustls_pemfile::certs(&mut std::io::BufReader::new(
                std::fs::File::open(&self.cert).unwrap(),
            ))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
            let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(
                std::fs::File::open(&self.key).unwrap(),
            ))
            .unwrap()
            .unwrap();
            Arc::new(
                rustls::ServerConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(cert, key)
                .unwrap(),
            )
        }
    }
    #[test]
    #[cfg(unix)]
    fn private_key_custody_refuses_group_or_world_access_ac15() {
        use std::os::unix::fs::PermissionsExt;
        let certs = Certificates::new("localhost", false);
        std::fs::set_permissions(&certs.key, std::fs::Permissions::from_mode(0o644)).unwrap();
        let root = tempfile::tempdir().unwrap();
        let store = LocalFsRegistryStorage::at_path(root.path().to_owned()).unwrap();
        let server = SpiritRegistryServer::new(Arc::new(store), "127.0.0.1:0".into(), None);
        assert!(server
            .with_tls(&certs.cert, &certs.key, "https://localhost/mcp".into())
            .is_err());
    }
    #[test]
    fn private_key_custody_refuses_seed_inside_package_store_ac15() {
        let root = tempfile::tempdir().unwrap();
        let seed = root.path().join("test-only.seed");
        std::fs::write(&seed, hex::encode(REGISTRY)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&seed, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let args = maos_registry::server::RegistryServerArgs {
            root: Some(root.path().to_owned()),
            signing_key: Some(seed),
            development_http: true,
            ..Default::default()
        };
        assert!(
            args.build().is_err(),
            "private signing seed inside the package store was accepted"
        );
    }
    #[test]
    #[cfg(unix)]
    fn private_seed_custody_refuses_0644_and_0440_p12() {
        use std::os::unix::fs::PermissionsExt;
        for mode in [0o644, 0o440] {
            let root = tempfile::tempdir().unwrap();
            let seed = root.path().join("registry.seed");
            std::fs::write(&seed, hex::encode(REGISTRY)).unwrap();
            std::fs::set_permissions(&seed, std::fs::Permissions::from_mode(mode)).unwrap();
            let error = maos_registry::server::RegistryServerArgs {
                root: Some(root.path().join("packages")),
                signing_key: Some(seed),
                development_http: true,
                ..Default::default()
            }
            .build()
            .err()
            .expect("seed mode must be refused");
            assert!(error.contains("0600"), "{error}");
        }
    }
    #[test]
    fn private_tls_key_inside_package_store_is_refused_p12() {
        let certs = Certificates::new("localhost", false);
        let root = tempfile::tempdir().unwrap();
        let packages = root.path().join("packages");
        std::fs::create_dir(&packages).unwrap();
        let key = packages.join("registry.key");
        std::fs::copy(&certs.key, &key).unwrap();
        let seed = root.path().join("registry.seed");
        std::fs::write(&seed, hex::encode(REGISTRY)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&key, &seed] {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
        let error = maos_registry::server::RegistryServerArgs {
            root: Some(packages),
            signing_key: Some(seed),
            tls_certificate: Some(certs.cert),
            tls_private_key: Some(key),
            uri: Some("https://localhost/mcp".into()),
            ..Default::default()
        }
        .build()
        .err()
        .expect("TLS key must stay outside served storage");
        assert!(error.contains("outside"), "{error}");
    }
    #[test]
    fn malformed_private_seed_diagnostic_never_echoes_key_bytes_p8() {
        let root = tempfile::tempdir().unwrap();
        let seed = root.path().join("registry.seed");
        std::fs::write(&seed, "@").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&seed, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let error = maos_registry::server::RegistryServerArgs {
            root: Some(root.path().join("packages")),
            signing_key: Some(seed),
            development_http: true,
            ..Default::default()
        }
        .build()
        .err()
        .expect("malformed private seed");
        assert_eq!(error, "invalid signing key hex");
        assert!(!error.contains('@'));
    }
    struct Server {
        uri: String,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
        _store: tempfile::TempDir,
    }
    impl Server {
        fn start(certs: &Certificates, signing_key: [u8; 32]) -> Self {
            Self::start_mode(Some(certs), signing_key)
        }
        fn start_mode(certs: Option<&Certificates>, signing_key: [u8; 32]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let storage = LocalFsRegistryStorage::at_path(dir.path().to_owned()).unwrap();
            let package = trust::package("tls-widget", "1.0.0", &trust::PUBLISHER);
            storage
                .put(&package.spirit_id, &package.version, &package)
                .unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let uri = if certs.is_some() {
                format!("https://localhost:{port}/mcp")
            } else {
                format!("http://127.0.0.1:{port}/mcp")
            };
            let server = SpiritRegistryServer::new(Arc::new(storage), "127.0.0.1:0".into(), None)
                .with_server_signing_key(signing_key);
            let server = if let Some(certs) = certs {
                server
                    .with_tls(&certs.cert, &certs.key, uri.clone())
                    .unwrap()
            } else {
                server.with_development_http()
            };
            let stop = Arc::new(AtomicBool::new(false));
            let flag = stop.clone();
            let worker = std::thread::spawn(move || server.serve(listener, flag).unwrap());
            Self {
                uri,
                stop,
                worker: Some(worker),
                _store: dir,
            }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            self.worker.take().unwrap().join().unwrap();
        }
    }
    #[test]
    fn configured_root_and_ca_authenticate_real_registry_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let published = trust::package("tls-published", "1.0.0", &trust::PUBLISHER);
        assert_eq!(
            client.publish(&published).unwrap().spirit_id,
            published.spirit_id
        );
        assert_eq!(
            client
                .manifest(&SpiritId::from("tls-widget"), "1.0.0")
                .unwrap()
                .spirit_id
                .as_str(),
            "tls-widget"
        );
        assert!(!client
            .artifact(&SpiritId::from("tls-widget"), "1.0.0")
            .unwrap()
            .artifact_bytes
            .is_empty());
        assert!(client.yanks_since(0).unwrap().entries.is_empty());
        let mut reason = maos_domain::ports::registry::YankReason::new("withdrawn".into());
        reason.authorization = Some(
            maos_registry::yank_auth::YankAuthorization::sign(
                &trust::PUBLISHER,
                maos_registry::yank_auth::YankScope {
                    name: "tls-widget",
                    version: "1.0.0",
                    reason: &reason.summary,
                    registry_uri: &server.uri,
                },
                YankOrigin::Publisher,
                String::new(),
                maos_registry::yank_auth::now_unix_ms().unwrap(),
            )
            .unwrap(),
        );
        client
            .deprecate(&SpiritId::from("tls-widget"), "1.0.0", &reason)
            .unwrap();
        assert_eq!(client.yanks_since(0).unwrap().entries.len(), 1);
        assert!(client.yanks_since(1).unwrap().entries.is_empty());
    }
    #[test]
    fn hostile_registry_wrong_configured_root_is_refused_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, [0x92; 32]);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let error = client.yanks_since(0).unwrap_err().to_string();
        assert!(error.contains("authentication"), "{error}");
    }
    #[test]
    fn configured_root_is_bound_to_canonical_uri_before_contact_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let mut config = certs.client_config(&server.uri);
        config.security.uri = Some("https://other.invalid/mcp".into());
        assert!(McpSpiritRegistryClient::http(server.uri.clone(), config).is_err());
        for uri in [
            "http://localhost:6789/mcp",
            "http://192.0.2.1:6789/mcp",
            "http://127.0.0.1:6789/mcp?x=1",
            "http://127.0.0.1:6789/mcp#x",
            "http://user@127.0.0.1:6789/mcp",
        ] {
            assert!(
                McpSpiritRegistryClient::http(
                    uri.into(),
                    RegistryClientConfig {
                        security: RegistrySecurityConfig::development_loopback(),
                        ..Default::default()
                    }
                )
                .is_err(),
                "{uri}"
            );
        }
    }
    #[test]
    fn interception_wrong_host_certificate_is_refused_ac15() {
        let certs = Certificates::new("different.invalid", false);
        let server = Server::start(&certs, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let error = client.yanks_since(0).unwrap_err().to_string();
        assert!(error.contains("certificate"), "{error}");
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("certificate"));
        assert_not_persisted(home.path());
    }
    #[test]
    fn interception_expired_certificate_is_refused_ac15() {
        let certs = Certificates::new("localhost", true);
        let server = Server::start(&certs, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let error = client.yanks_since(0).unwrap_err().to_string();
        assert!(error.contains("certificate"), "{error}");
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("certificate"));
        assert_not_persisted(home.path());
    }
    #[test]
    fn interception_untrusted_ca_is_refused_ac15() {
        let certs = Certificates::new("localhost", false);
        let stranger = Certificates::new("localhost", false);
        let server = Server::start(&stranger, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let error = client.yanks_since(0).unwrap_err().to_string();
        assert!(error.contains("certificate"), "{error}");
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("certificate"));
        assert_not_persisted(home.path());
    }

    fn cli_install(
        uri: &str,
        bound_uri: &str,
        root: [u8; 32],
        ca: &std::path::Path,
        development: bool,
    ) -> (tempfile::TempDir, std::process::Output) {
        let home = tempfile::tempdir().unwrap();
        let config = home.path().join(".config/maos");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("operator.toml"), format!(
            "[registry]\nuri = {bound_uri:?}\nsigning_pubkey = {:?}\nca_certificate = {:?}\ndevelopment_http = {development}\n",
            hex::encode(root), ca.to_str().unwrap())).unwrap();
        let binary = support::sibling("maosctl");
        let output = std::process::Command::new(binary)
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .current_dir(home.path())
            .args([
                "spirit",
                "install",
                "tls-widget@1.0.0",
                "--registry-uri",
                uri,
            ])
            .output()
            .unwrap();
        (home, output)
    }
    fn assert_not_persisted(home: &std::path::Path) {
        let root = home.join(".local/share/maos/registry");
        assert!(
            !root.exists(),
            "refused installation created registry persistence: {}",
            root.display()
        );
    }
    #[test]
    fn cli_tls_install_authenticates_before_persistence_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let root = home.path().join(".local/share/maos/registry");
        assert!(root.join("installed-index.json").is_file());
        let inventory = LocalFsRegistryStorage::at_path(root)
            .unwrap()
            .installed_manifests()
            .unwrap();
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].0, "tls-widget");
        assert!(inventory[0]
            .1
            .parent()
            .unwrap()
            .join("signed_package.json")
            .is_file());
    }
    #[test]
    fn cli_wrong_configured_key_refuses_hostile_first_contact_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&[0x92; 32]),
            &certs.ca,
            false,
        );
        assert!(
            !output.status.success(),
            "hostile metadata accepted under the wrong configured key"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("authentication"));
        assert_not_persisted(home.path());
    }
    #[test]
    fn cli_wrong_configured_uri_refuses_hostile_first_contact_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let (home, output) = cli_install(
            &server.uri,
            "https://other.invalid/mcp",
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(!output.status.success(), "configured URI binding ignored");
        assert!(String::from_utf8_lossy(&output.stderr).contains("authentication"));
        assert_not_persisted(home.path());
    }

    #[derive(Clone, Copy)]
    enum Attack {
        Control,
        MissingEnvelope,
        WrongUri,
        WrongNonce,
        MissingHead,
        ShortHead,
        StripEntry,
        Redirect,
        InstallStrip,
        InstallMissingEnvelope,
        InstallMissingHead,
        SignedGap,
        SignedDuplicate,
        SignedExtra,
        SignedEmpty,
        PayloadHeadMismatch,
        CursorAhead,
        WrongRequestHash,
        WrongVersion,
        BadSignature,
    }
    struct Responder {
        uri: String,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Responder {
        fn start(certs: &Certificates, attack: Attack, redirect: Option<String>) -> Self {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let uri = format!(
                "https://localhost:{}/mcp",
                listener.local_addr().unwrap().port()
            );
            let canonical = uri.clone();
            listener.set_nonblocking(true).unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let signal = stop.clone();
            let config = certs.server_config();
            let worker = std::thread::spawn(move || {
                let installation = matches!(
                    attack,
                    Attack::InstallStrip
                        | Attack::InstallMissingEnvelope
                        | Attack::InstallMissingHead
                );
                let store_dir = tempfile::tempdir().unwrap();
                let store = LocalFsRegistryStorage::at_path(store_dir.path().to_owned()).unwrap();
                let package = trust::package("tls-widget", "1.0.0", &trust::PUBLISHER);
                store
                    .put(&package.spirit_id, &package.version, &package)
                    .unwrap();
                let mut withdrawal =
                    maos_domain::ports::registry::YankReason::new("withdrawn".into());
                withdrawal.authorization = Some(
                    maos_registry::yank_auth::YankAuthorization::sign(
                        &trust::PUBLISHER,
                        maos_registry::yank_auth::YankScope {
                            name: "tls-widget",
                            version: "1.0.0",
                            reason: &withdrawal.summary,
                            registry_uri: &canonical,
                        },
                        YankOrigin::Publisher,
                        String::new(),
                        maos_registry::yank_auth::now_unix_ms().unwrap(),
                    )
                    .unwrap(),
                );
                if installation || matches!(attack, Attack::StripEntry) {
                    store
                        .yank(&package.spirit_id, &package.version, &withdrawal)
                        .unwrap();
                }
                for _ in 0..if installation { 3 } else { 1 } {
                    let socket = loop {
                        if signal.load(Ordering::SeqCst) {
                            return;
                        }
                        match listener.accept() {
                            Ok((socket, _)) => break socket,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(std::time::Duration::from_millis(5))
                            }
                            Err(error) => panic!("{error}"),
                        }
                    };
                    socket
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    socket
                        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    let mut stream = rustls::StreamOwned::new(
                        rustls::ServerConnection::new(config.clone()).unwrap(),
                        socket,
                    );
                    let mut reader = std::io::BufReader::new(&mut stream);
                    let mut line = String::new();
                    let mut length = None;
                    let mut nonce = None;
                    use std::io::BufRead;
                    reader.read_line(&mut line).unwrap();
                    loop {
                        line.clear();
                        reader.read_line(&mut line).unwrap();
                        if line == "\r\n" {
                            break;
                        }
                        let Some((name, value)) = line.split_once(':') else {
                            panic!("bad fixture request");
                        };
                        if name.eq_ignore_ascii_case("content-length") {
                            length = Some(value.trim().parse::<usize>().unwrap());
                        }
                        if name.eq_ignore_ascii_case(maos_registry::security::NONCE_HEADER) {
                            nonce = Some(
                                <[u8; 32]>::try_from(hex::decode(value.trim()).unwrap()).unwrap(),
                            );
                        }
                    }
                    let mut request = vec![0; length.unwrap()];
                    reader.read_exact(&mut request).unwrap();
                    drop(reader);
                    let parsed: serde_json::Value = match serde_json::from_slice(&request) {
                        Ok(value) => value,
                        Err(e) => panic!("{e}"),
                    };
                    let Some(id) = parsed.get("id") else {
                        panic!("missing request id");
                    };
                    let method = parsed
                        .pointer("/params/name")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| parsed.get("method").and_then(serde_json::Value::as_str))
                        .unwrap_or("");
                    let is_yanks = method == "registry.yanks_since";
                    let result = if installation {
                        match method {
                            "registry.manifest" => fixture_json(
                                &store
                                    .get_manifest(&package.spirit_id, &package.version)
                                    .unwrap(),
                            ),
                            "registry.artifact" => fixture_json(
                                &store
                                    .get_artifact(&package.spirit_id, &package.version)
                                    .unwrap(),
                            ),
                            "registry.yanks_since" => {
                                let mut value = fixture_json(&store.yanks_since(0).unwrap());
                                value["head"] = 1.into();
                                value
                            }
                            _ => panic!("unexpected fixture method {method}"),
                        }
                    } else {
                        let mut list = fixture_json(&store.yanks_since(0).unwrap());
                        list["head"] = if matches!(attack, Attack::StripEntry) {
                            1.into()
                        } else {
                            0.into()
                        };
                        list
                    };
                    let mut body = serde_json::json!({"jsonrpc":"2.0","id":id,"result":result});
                    let mut sent = match serde_json::to_vec(&body) {
                        Ok(value) => value,
                        Err(e) => panic!("{e}"),
                    };
                    let mut signed = sent.clone();
                    let mut head = if is_yanks {
                        Some(if installation || matches!(attack, Attack::StripEntry) {
                            1
                        } else {
                            0
                        })
                    } else {
                        None
                    };
                    let mut claimed_uri = canonical.as_str();
                    let mut nonce = nonce.unwrap();
                    match attack {
                        Attack::WrongUri => claimed_uri = "https://wrong.invalid/mcp",
                        Attack::WrongNonce => nonce[0] ^= 1,
                        Attack::MissingHead => head = None,
                        Attack::ShortHead => head = Some(1),
                        Attack::Redirect => {
                            sent.clear();
                            signed.clear();
                        }
                        Attack::InstallMissingHead if is_yanks => head = None,
                        Attack::InstallStrip | Attack::StripEntry if is_yanks => {
                            body["result"]["entries"] = serde_json::json!([]);
                            sent = fixture_bytes(&body);
                        }
                        Attack::SignedGap
                        | Attack::SignedDuplicate
                        | Attack::SignedExtra
                        | Attack::SignedEmpty
                        | Attack::PayloadHeadMismatch
                        | Attack::CursorAhead => {
                            let (sequences, claimed, payload): (&[u64], u64, u64) = match attack {
                                Attack::SignedGap => (&[2], 1, 1),
                                Attack::SignedDuplicate => (&[1, 1], 2, 2),
                                Attack::SignedExtra => (&[1, 2], 1, 1),
                                Attack::SignedEmpty => (&[], 1, 1),
                                Attack::PayloadHeadMismatch => (&[], 0, 1),
                                Attack::CursorAhead => (&[], 2, 2),
                                _ => unreachable!(),
                            };
                            let entries: Vec<_> = sequences
                                .iter()
                                .map(|sequence| {
                                    let mut entry = maos_domain::ports::registry::YankEntry::new(
                                        "tls-widget".into(),
                                        "1.0.0".into(),
                                        0,
                                        "test withdrawal".into(),
                                    );
                                    entry.sequence = *sequence;
                                    entry
                                })
                                .collect();
                            body["result"] = serde_json::json!({"entries":entries,"head":payload});
                            sent = fixture_bytes(&body);
                            signed = sent.clone();
                            head = Some(claimed);
                        }
                        _ => {}
                    }
                    let envelope =
                        response_envelope(claimed_uri, nonce, &request, &signed, head, attack);
                    let status = if matches!(attack, Attack::Redirect) {
                        "302 Found"
                    } else {
                        "200 OK"
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
                        sent.len()
                    )
                    .unwrap();
                    if !matches!(attack, Attack::MissingEnvelope | Attack::Redirect)
                        && !(matches!(attack, Attack::InstallMissingEnvelope) && is_yanks)
                    {
                        write!(
                            stream,
                            "{}: {envelope}\r\n",
                            maos_registry::security::RESPONSE_HEADER
                        )
                        .unwrap();
                    }
                    if let Some(redirect) = &redirect {
                        write!(stream, "Location: {redirect}\r\n").unwrap();
                    }
                    stream.write_all(b"\r\n").unwrap();
                    stream.write_all(&sent).unwrap();
                    stream.flush().unwrap();
                    stream.conn.send_close_notify();
                    let _ = stream.flush();
                }
            });
            Self {
                uri,
                stop,
                worker: Some(worker),
            }
        }
    }
    impl Drop for Responder {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let result = self.worker.take().unwrap().join();
            if !std::thread::panicking() {
                assert!(result.is_ok(), "TLS fixture worker panicked");
            }
        }
    }
    fn fixture_json(value: &impl serde::Serialize) -> serde_json::Value {
        match serde_json::to_value(value) {
            Ok(value) => value,
            Err(e) => panic!("{e}"),
        }
    }
    fn fixture_bytes(value: &impl serde::Serialize) -> Vec<u8> {
        match serde_json::to_vec(value) {
            Ok(value) => value,
            Err(e) => panic!("{e}"),
        }
    }
    /// Independent fixture implementation of the documented signed-envelope preimage.
    fn response_envelope(
        uri: &str,
        nonce: [u8; 32],
        request: &[u8],
        response: &[u8],
        yank_head: Option<u64>,
        attack: Attack,
    ) -> String {
        use sha2::{Digest, Sha256};
        #[derive(serde::Serialize)]
        struct Claim<'a> {
            version: u8,
            uri: &'a str,
            nonce: [u8; 32],
            request_sha256: [u8; 32],
            response_sha256: [u8; 32],
            yank_head: Option<u64>,
        }
        #[derive(serde::Serialize)]
        struct Envelope<'a> {
            claim: Claim<'a>,
            signature: Vec<u8>,
        }
        let mut claim = Claim {
            version: 1,
            uri,
            nonce,
            request_sha256: Sha256::digest(request).into(),
            response_sha256: Sha256::digest(response).into(),
            yank_head,
        };
        if matches!(attack, Attack::WrongRequestHash) {
            claim.request_sha256[0] ^= 1;
        }
        if matches!(attack, Attack::WrongVersion) {
            claim.version = 2;
        }
        let encoded = match serde_cbor::to_vec(&claim) {
            Ok(value) => value,
            Err(e) => panic!("{e}"),
        };
        let mut hash = Sha256::new();
        hash.update(b"maos.registry.response.v1\0");
        hash.update(encoded);
        let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&REGISTRY).unwrap();
        let mut signature = key.sign(&hash.finalize()).as_ref().to_vec();
        if matches!(attack, Attack::BadSignature) {
            signature[0] ^= 1;
        }
        match serde_cbor::to_vec(&Envelope { claim, signature }) {
            Ok(value) => hex::encode(value),
            Err(e) => panic!("{e}"),
        }
    }
    fn check_attack(attack: Attack) {
        let certs = Certificates::new("localhost", false);
        for attack in [Attack::Control, attack] {
            let peer = Responder::start(&certs, attack, None);
            let client =
                McpSpiritRegistryClient::http(peer.uri.clone(), certs.client_config(&peer.uri))
                    .unwrap();
            let result = client.yanks_since(0);
            if matches!(attack, Attack::Control) {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(
                    result.is_err(),
                    "altered/omitted authenticated yank view was accepted"
                );
                assert!(result.unwrap_err().to_string().contains("authentication"));
            }
        }
    }
    #[test]
    fn signed_valid_gap_body_is_refused_p3() {
        check_attack(Attack::SignedGap);
    }
    #[test]
    fn signed_valid_duplicate_body_is_refused_p3() {
        check_attack(Attack::SignedDuplicate);
    }
    #[test]
    fn signed_valid_extra_entries_are_refused_p3() {
        check_attack(Attack::SignedExtra);
    }
    #[test]
    fn signed_valid_empty_view_with_nonzero_head_is_refused_p3() {
        check_attack(Attack::SignedEmpty);
    }
    #[test]
    fn signed_payload_head_must_equal_envelope_head_p3() {
        check_attack(Attack::PayloadHeadMismatch);
    }
    #[test]
    fn signed_wrong_request_hash_is_refused_p3() {
        check_attack(Attack::WrongRequestHash);
    }
    #[test]
    fn signed_wrong_envelope_version_is_refused_p3() {
        check_attack(Attack::WrongVersion);
    }
    #[test]
    fn bad_signature_byte_is_refused_p3() {
        check_attack(Attack::BadSignature);
    }
    #[test]
    fn signed_head_behind_requested_cursor_is_typed_d3_p5() {
        let certs = Certificates::new("localhost", false);
        let peer = Responder::start(&certs, Attack::CursorAhead, None);
        let client =
            McpSpiritRegistryClient::http(peer.uri.clone(), certs.client_config(&peer.uri))
                .unwrap();
        let result = client.yanks_since(5);
        assert!(
            matches!(
                result,
                Err(
                    maos_domain::ports::registry::RegistryError::HeadBehindCursor {
                        head: 2,
                        cursor: 5
                    }
                )
            ),
            "{result:?}"
        );
    }
    #[test]
    fn envelope_missing_signature_is_refused_ac15() {
        check_attack(Attack::MissingEnvelope);
    }
    #[test]
    fn envelope_wrong_uri_is_refused_ac15() {
        check_attack(Attack::WrongUri);
    }
    #[test]
    fn envelope_wrong_nonce_is_refused_ac15() {
        check_attack(Attack::WrongNonce);
    }
    #[test]
    fn envelope_missing_head_is_refused_ac15() {
        check_attack(Attack::MissingHead);
    }
    #[test]
    fn envelope_incomplete_head_is_refused_ac15() {
        check_attack(Attack::ShortHead);
    }
    #[test]
    fn envelope_stripped_yank_entry_is_refused_ac15() {
        check_attack(Attack::StripEntry);
    }
    #[test]
    fn https_redirect_never_downgrades_to_http_ac15() {
        let certs = Certificates::new("localhost", false);
        let trap = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        trap.set_nonblocking(true).unwrap();
        let peer = Responder::start(
            &certs,
            Attack::Redirect,
            Some(format!("http://{}/mcp", trap.local_addr().unwrap())),
        );
        let client =
            McpSpiritRegistryClient::http(peer.uri.clone(), certs.client_config(&peer.uri))
                .unwrap();
        assert!(client.yanks_since(0).is_err());
        assert_eq!(
            trap.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "HTTPS redirected to plaintext"
        );
    }
    #[test]
    fn loopback_development_requires_both_sides_and_keeps_yank_signatures_ac15() {
        let server = Server::start_mode(None, REGISTRY);
        assert!(McpSpiritRegistryClient::http(server.uri.clone(), Default::default()).is_err());
        let config = RegistryClientConfig {
            security: RegistrySecurityConfig::development_loopback(),
            ..Default::default()
        };
        let client = McpSpiritRegistryClient::http(server.uri.clone(), config).unwrap();
        assert!(client.yanks_since(0).unwrap().entries.is_empty());
        let reason = maos_domain::ports::registry::YankReason::new("unsigned".into());
        assert!(client
            .deprecate(&SpiritId::from("tls-widget"), "1.0.0", &reason)
            .is_err());
    }
    #[test]
    fn server_rejects_client_without_dev_handshake_ac15() {
        let server = Server::start_mode(None, REGISTRY);
        let endpoint = maos_registry::security::canonical_uri(&server.uri).unwrap();
        let mut raw =
            std::net::TcpStream::connect(("127.0.0.1", endpoint.port().unwrap())).unwrap();
        let body =
            br#"{"jsonrpc":"2.0","id":1,"method":"registry.yanks_since","params":{"since_ns":0}}"#;
        write!(raw, "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            endpoint.port().unwrap(), body.len()).unwrap();
        raw.write_all(body).unwrap();
        let mut response = String::new();
        raw.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    }
    #[test]
    fn cli_tls_yanked_version_is_refused_before_persistence_ac15() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let mut reason = maos_domain::ports::registry::YankReason::new("withdrawn".into());
        reason.authorization = Some(
            maos_registry::yank_auth::YankAuthorization::sign(
                &trust::PUBLISHER,
                maos_registry::yank_auth::YankScope {
                    name: "tls-widget",
                    version: "1.0.0",
                    reason: &reason.summary,
                    registry_uri: &server.uri,
                },
                YankOrigin::Publisher,
                String::new(),
                maos_registry::yank_auth::now_unix_ms().unwrap(),
            )
            .unwrap(),
        );
        client
            .deprecate(&SpiritId::from("tls-widget"), "1.0.0", &reason)
            .unwrap();
        let (home, output) = cli_install(
            &server.uri,
            &server.uri,
            trust::public(&REGISTRY),
            &certs.ca,
            false,
        );
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("yanked"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_not_persisted(home.path());
    }
    #[test]
    fn cli_omitted_yank_or_unauthenticated_head_refuses_before_persistence_ac15() {
        let certs = Certificates::new("localhost", false);
        for attack in [
            Attack::InstallStrip,
            Attack::InstallMissingEnvelope,
            Attack::InstallMissingHead,
        ] {
            let peer = Responder::start(&certs, attack, None);
            let (home, output) = cli_install(
                &peer.uri,
                &peer.uri,
                trust::public(&REGISTRY),
                &certs.ca,
                false,
            );
            assert!(
                !output.status.success(),
                "incomplete/unauthenticated yank view installed"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("authentication"),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_not_persisted(home.path());
        }
    }
    #[test]
    fn https_requires_configured_root_not_response_supplied_key_ac15() {
        assert!(McpSpiritRegistryClient::http(
            "https://127.0.0.1:9/mcp".into(),
            Default::default()
        )
        .is_err());
    }
    #[test]
    fn handshake_failure_never_attempts_plaintext_ac15() {
        let certs = Certificates::new("localhost", false);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let uri = format!(
            "https://localhost:{}/mcp",
            listener.local_addr().unwrap().port()
        );
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut record = [0; 3];
            socket.read_exact(&mut record).unwrap();
            assert_eq!(record[0], 0x16, "first contact was not TLS");
            let _ =
                socket.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");
            drop(socket);
            listener
        });
        let client = McpSpiritRegistryClient::http(uri.clone(), certs.client_config(&uri)).unwrap();
        assert!(client.yanks_since(0).is_err());
        let listener = worker.join().unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "TLS failure triggered a second/plaintext connection"
        );
    }
    #[test]
    fn tls_admission_refusal_is_encrypted_http_503_f1() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let client =
            McpSpiritRegistryClient::http(server.uri.clone(), certs.client_config(&server.uri))
                .unwrap();
        let port = url::Url::parse(&server.uri).unwrap().port().unwrap();
        let _first = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let _second = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let error = client.yanks_since(0).unwrap_err().to_string();
        assert!(
            error.contains("503"),
            "TLS refusal lost its HTTP status: {error}"
        );
    }
    #[test]
    fn incomplete_tls_handshake_cannot_enter_http_error_write_timeout_f1() {
        let certs = Certificates::new("localhost", false);
        let server = Server::start(&certs, REGISTRY);
        let port = url::Url::parse(&server.uri).unwrap().port().unwrap();
        let mut idle = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        idle.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let read = idle.read(&mut [0u8; 1]);
        assert!(
            matches!(read, Ok(0)),
            "unfinished handshake was retained or answered as plaintext: {read:?}"
        );
    }
    #[test]
    #[ignore = "20-1b hosted-only public WebPKI route; explicitly enrolled in the verification branch"]
    fn private_ca_replaces_public_webpki_for_real_public_peer_d4_f7() {
        use maos_domain::ports::io_subsystem::IoSubsystemPort;
        let certs = Certificates::new("localhost", false);
        let uri = "https://github.com/mcp";
        let mut security = certs.client_config(uri).security;
        security.ca_certificate = None;
        let request =
            br#"{"jsonrpc":"2.0","id":1,"method":"registry.yanks_since","params":{"since_ns":0}}"#;
        let control = maos_registry::http::RegistryHttp::new(uri, security.clone())
            .unwrap()
            .http_post_exchange(uri, request, &[("Content-Type", "application/json")], "");
        if let Err(error) = control {
            let message = error.to_string();
            assert!(
                message.contains("HTTP")
                    || message.contains("Content-Length")
                    || message.contains("registry response authentication"),
                "public-WebPKI control did not reach HTTP: {message}"
            );
        }
        security.ca_certificate = Some(certs.ca);
        let error = maos_registry::http::RegistryHttp::new(uri, security)
            .unwrap()
            .http_post_exchange(uri, request, &[("Content-Type", "application/json")], "")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("UnknownIssuer"),
            "configured private CA did not exclude public roots: {error}"
        );
    }
}
