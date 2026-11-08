//! Bounded blocking registry server: authenticated TLS, or explicit loopback development.
//! No kernel, hyper or asynchronous network stack. Response signatures bind the
//! operator-configured URI, fresh request nonce, exact bytes and complete yank head.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::handlers;
use crate::operations::{ArtifactArgs, DeprecateArgs, ManifestArgs, SearchArgs, YanksSinceArgs};
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::SignedPackage;

const MAX_CONNECTIONS: usize = 8;
const MAX_PER_SOURCE: usize = 2;
const MAX_REJECTIONS: usize = 2;
const METADATA_BODY_LIMIT: usize = 64 * 1024;
const UPLOAD_BODY_LIMIT: usize = 64 * 1024 * 1024;
const FIRST_BYTE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(1);
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
const REQUEST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

fn development_uri(addr: std::net::SocketAddr) -> Result<String, String> {
    url::Url::parse(&format!("http://{addr}/mcp"))
        .map(String::from)
        .map_err(|error| error.to_string())
}

/// JSON-RPC 2.0 request frame.
#[derive(Debug, serde::Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response frame.
#[derive(Debug, serde::Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Clone, Default)]
struct ServerSecurity {
    development_http: bool,
    uri: Option<String>,
    #[cfg(feature = "tls")]
    tls: Option<Arc<rustls::ServerConfig>>,
}

/// The Spirit Registry server.
pub struct SpiritRegistryServer {
    storage: Arc<dyn RegistryStorage>,
    listen_addr: String,
    org_pubkey: Option<[u8; 32]>,
    /// Story 7.2 — Ed25519 signing key for server-side tier attestation.
    server_signing_key: Option<[u8; 32]>,
    vetters: Option<Arc<(maos_compliance::VetterKeyring, [u8; 32])>>,
    security: ServerSecurity,
}

impl SpiritRegistryServer {
    /// Construct a new registry server.
    pub fn new(
        storage: Arc<dyn RegistryStorage>,
        listen_addr: String,
        org_pubkey: Option<[u8; 32]>,
    ) -> Self {
        Self {
            storage,
            listen_addr,
            org_pubkey,
            server_signing_key: None,
            vetters: None,
            security: Default::default(),
        }
    }

    /// Story 7.2 — attach an Ed25519 signing key for server-side tier attestation.
    /// Raw bytes have no file identity: direct library callers own provisioning
    /// custody. `RegistryServerArgs::build` checks file modes and store exclusion.
    pub fn with_server_signing_key(mut self, key: [u8; 32]) -> Self {
        self.server_signing_key = Some(key);
        self
    }

    pub fn with_development_http(mut self) -> Self {
        self.security.development_http = true;
        self
    }
    #[cfg(feature = "tls")]
    pub fn with_tls(
        mut self,
        certificate: &std::path::Path,
        key: &std::path::Path,
        uri: String,
    ) -> Result<Self, String> {
        if crate::security::canonical_uri(&uri)?.scheme() != "https" {
            return Err("TLS registry requires an https canonical URI".into());
        }
        self.security.tls = Some(crate::security::server_tls(certificate, key)?);
        self.security.uri = Some(uri);
        Ok(self)
    }

    /// Operator-root anchored lifecycle journal authorizing vetter withdrawals.
    pub fn with_vetter_keyring(
        mut self,
        keyring: maos_compliance::VetterKeyring,
        root: [u8; 32],
    ) -> Self {
        self.vetters = Some(Arc::new((keyring, root)));
        self
    }

    /// Run the configured TLS/development server, draining workers on SIGTERM.
    pub async fn run(self) -> Result<(), String> {
        let listener = TcpListener::bind(&self.listen_addr).map_err(|e| format!("bind: {e}"))?;
        #[cfg(unix)]
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|e| format!("SIGTERM: {e}"))?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&shutdown);
        let mut worker = tokio::task::spawn_blocking(move || self.serve(listener, stop));
        tokio::select! {
            result = &mut worker => return result.map_err(|e| e.to_string())?,
            result = tokio::signal::ctrl_c() => { result.map_err(|e| e.to_string())?; },
            _ = async {
                #[cfg(unix)]
                terminate.recv().await;
                #[cfg(not(unix))]
                std::future::pending::<()>().await;
            } => {},
        }
        shutdown.store(true, Ordering::SeqCst);
        worker.await.map_err(|e| e.to_string())?
    }

    /// Serve an already-bound listener; caller owns its shutdown flag.
    pub fn serve(self, listener: TcpListener, shutdown: Arc<AtomicBool>) -> Result<(), String> {
        let addr = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {e}"))?;
        let mut security = self.security;
        #[cfg(feature = "tls")]
        let tls_enabled = security.tls.is_some();
        #[cfg(not(feature = "tls"))]
        let tls_enabled = false;
        if tls_enabled {
            if security.development_http || self.server_signing_key.is_none() {
                return Err(
                    "TLS requires a registry signing key and forbids development mode".into(),
                );
            }
        } else if !security.development_http || !addr.ip().is_loopback() {
            return Err(
                "registry requires TLS or explicit literal-loopback development mode".into(),
            );
        }
        if security.uri.is_none() {
            security.uri = Some(development_uri(addr)?);
        }
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        eprintln!("maos-registry: listening on {addr}");
        let storage = self.storage;
        let server_signing_key = self.server_signing_key;
        let vetters = self.vetters;
        let _ = &self.org_pubkey;
        // Two bounded refusal workers can finish TLS and return an encrypted 503.
        let mut handles: Vec<(TcpStream, std::net::IpAddr, bool, thread::JoinHandle<()>)> =
            Vec::with_capacity(MAX_CONNECTIONS + MAX_REJECTIONS);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            handles.retain(|(_, _, _, handle)| !handle.is_finished());
            match listener.accept() {
                Ok((stream, peer)) => {
                    // Accepted sockets inherit O_NONBLOCK on some supported hosts.
                    if let Err(error) = stream.set_nonblocking(false) {
                        eprintln!("maos-registry: accepted socket mode: {error}");
                        continue;
                    }
                    let source = peer.ip().to_canonical();
                    let (mut active, mut same_source, mut rejections) = (0, 0, 0);
                    for (_, ip, rejected, _) in &handles {
                        if *rejected {
                            rejections += 1;
                        } else {
                            active += 1;
                            same_source += usize::from(*ip == source);
                        }
                    }
                    let rejected = active >= MAX_CONNECTIONS || same_source >= MAX_PER_SOURCE;
                    if rejected && security.development_http {
                        // Loopback development needs no handshake/refusal worker.
                        // Return complete 503s without consuming TLS refusal slots;
                        // accept has one absolute 100 ms write/drain budget here.
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_millis(100);
                        let mut stream = DeadlineStream::new(stream, deadline);
                        let _ = stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        let _ = stream.stream.shutdown(std::net::Shutdown::Write);
                        let _ = std::io::copy(
                            &mut stream.take(UPLOAD_BODY_LIMIT as u64),
                            &mut std::io::sink(),
                        );
                        continue;
                    }
                    if rejected && rejections >= MAX_REJECTIONS {
                        continue;
                    }
                    let cancel = match stream.try_clone() {
                        Ok(cancel) => cancel,
                        Err(error) => {
                            eprintln!("maos-registry: accepted socket clone: {error}");
                            thread::sleep(std::time::Duration::from_millis(50));
                            continue;
                        }
                    };
                    let s = Arc::clone(&storage);
                    let vetters = vetters.clone();
                    let security = security.clone();
                    handles.push((
                        cancel,
                        source,
                        rejected,
                        thread::spawn(move || {
                            if let Err(error) = handle_connection(
                                s,
                                server_signing_key,
                                vetters,
                                stream,
                                security,
                                rejected,
                            ) {
                                eprintln!("maos-registry: connection error: {error}");
                            }
                        }),
                    ));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    eprintln!("maos-registry: accept error: {e}");
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }

        for (stream, _, _, _) in &handles {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        for (_, _, _, handle) in handles {
            let _ = handle.join();
        }

        Ok(())
    }
}

/// Shared standalone/daemon construction. No root means the operator's local store.
#[derive(Default)]
pub struct RegistryServerArgs {
    pub bind: Option<String>,
    pub root: Option<std::path::PathBuf>,
    pub signing_key: Option<std::path::PathBuf>,
    pub vetter_keyring: Option<std::path::PathBuf>,
    pub operator_root: Option<String>,
    pub release_pin: Option<String>,
    pub tls_certificate: Option<std::path::PathBuf>,
    pub tls_private_key: Option<std::path::PathBuf>,
    pub uri: Option<String>,
    pub development_http: bool,
}

impl RegistryServerArgs {
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args;
        while let Some(flag) = args.next() {
            if flag == "--dev-http" {
                result.development_http = true;
                continue;
            }
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--bind" => result.bind = Some(value),
                "--root" => result.root = Some(value.into()),
                "--signing-key" => result.signing_key = Some(value.into()),
                "--tls-cert" => result.tls_certificate = Some(value.into()),
                "--tls-key" => result.tls_private_key = Some(value.into()),
                "--uri" => result.uri = Some(value),
                "--vetter-keyring" => result.vetter_keyring = Some(value.into()),
                "--operator-root" => result.operator_root = Some(value),
                "release-pin" => result.release_pin = Some(value),
                _ => return Err(format!("unknown registry flag {flag}")),
            }
        }
        Ok(result)
    }

    pub fn build(self) -> Result<SpiritRegistryServer, String> {
        if self.release_pin.is_some() {
            return Err(
                "release-pin is a standalone local operator verb, not a server mode".into(),
            );
        }
        let storage = match self.root {
            Some(root) => crate::storage::LocalFsRegistryStorage::at_path(root),
            None => crate::storage::LocalFsRegistryStorage::new(),
        }
        .map_err(|e| format!("registry root: {e}"))?;
        for path in self.signing_key.iter().chain(self.tls_private_key.iter()) {
            crate::security::key_outside_store(path, storage.root_path())?;
        }
        let mut server = SpiritRegistryServer::new(
            Arc::new(storage),
            self.bind.unwrap_or_else(|| "127.0.0.1:6789".into()),
            None,
        );
        if let Some(path) = self.signing_key {
            let mut text = String::new();
            crate::security::private_key_file(&path)?
                .read_to_string(&mut text)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let seed = hex::decode(text.trim())
                .map_err(|_| "invalid signing key hex")?
                .try_into()
                .map_err(|_| "signing key must be a 32-byte hex seed")?;
            server = server.with_server_signing_key(seed);
        }
        if self.development_http {
            server = server.with_development_http();
        }
        match (self.tls_certificate, self.tls_private_key, self.uri) {
            (None, None, None) if self.development_http => {}
            (Some(cert), Some(key), Some(uri)) if !self.development_http => {
                #[cfg(feature = "tls")]
                {
                    server = server.with_tls(&cert, &key, uri)?;
                }
                #[cfg(not(feature = "tls"))]
                {
                    let _ = (cert, key, uri);
                    return Err("registry TLS feature is disabled".into());
                }
            }
            _ => {
                return Err(
                    "supply --tls-cert, --tls-key, --uri and --signing-key; or explicit --dev-http"
                        .into(),
                )
            }
        }
        match (self.vetter_keyring, self.operator_root) {
            (Some(path), Some(root)) => {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                let keyring = serde_cbor::from_slice(&bytes).map_err(|e| e.to_string())?;
                let root = hex::decode(root)
                    .map_err(|_| "invalid operator root hex")?
                    .try_into()
                    .map_err(|_| "operator root must be 32 bytes")?;
                server = server.with_vetter_keyring(keyring, root);
            }
            (None, None) => {}
            _ => {
                return Err("--vetter-keyring and --operator-root must be supplied together".into())
            }
        }
        Ok(server)
    }
}

/// Handle a single TCP connection — read one JSON-RPC request, dispatch, respond.
fn handle_connection(
    storage: Arc<dyn RegistryStorage>,
    server_signing_key: Option<[u8; 32]>,
    vetters: Option<Arc<(maos_compliance::VetterKeyring, [u8; 32])>>,
    stream: TcpStream,
    security: ServerSecurity,
    rejected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry_uri = security
        .uri
        .as_deref()
        .ok_or("missing registry canonical URI")?;
    let endpoint = crate::security::canonical_uri(registry_uri)?;
    let deadline = std::time::Instant::now() + FIRST_BYTE_DEADLINE;
    let stream = DeadlineStream { stream, deadline };
    #[cfg(feature = "tls")]
    let stream = if let Some(config) = security.tls {
        ServerStream::Tls(Box::new(rustls::StreamOwned::new(
            rustls::ServerConnection::new(config)?,
            stream,
        )))
    } else {
        ServerStream::Plain(stream)
    };
    #[cfg(not(feature = "tls"))]
    let stream = ServerStream::Plain(stream);
    let mut reader = BufReader::new(stream);
    if let Err(error) = reader.fill_buf() {
        // An HTTP error write must not restart an incomplete TLS handshake
        // under the longer response-write timeout.
        if !reader.get_ref().handshake_complete() {
            return Err(error.into());
        }
        reject_http(reader.get_mut(), "408 Request Timeout")?;
        return Ok(());
    }
    if rejected {
        reject_http(reader.get_mut(), "503 Service Unavailable")?;
        return Ok(());
    }
    reader
        .get_mut()
        .set_deadline(std::time::Instant::now() + REQUEST_DEADLINE);
    let headers = match read_request_headers(&mut reader, &endpoint, security.development_http) {
        Ok(headers) => headers,
        Err(status) => {
            reject_http(reader.get_mut(), status)?;
            return Ok(());
        }
    };
    let content_length = headers.content_length;

    if content_length > headers.body_limit {
        reject_http(reader.get_mut(), "413 Payload Too Large")?;
        return Ok(());
    }

    // Grow from received bytes, never reserve an untrusted declared body length.
    let mut body = Vec::new();
    while body.len() < content_length {
        match reader.fill_buf() {
            Ok([]) => {
                reject_http(reader.get_mut(), "400 Bad Request")?;
                return Ok(());
            }
            Ok(available) => {
                let size = available.len().min(content_length - body.len());
                body.extend_from_slice(&available[..size]);
                reader.consume(size);
            }
            Err(error) => {
                let status = if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) {
                    "408 Request Timeout"
                } else {
                    "400 Bad Request"
                };
                reject_http(reader.get_mut(), status)?;
                return Ok(());
            }
        }
    }

    // Parse JSON-RPC request
    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: serde_json::Value::Null,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {e}"),
                }),
            };
            let resp_json = serde_json::to_vec(&resp)?;
            respond_http(reader.get_mut(), "200 OK", &resp_json, None)?;
            return Ok(());
        }
    };
    let publish = request.method == "registry.publish"
        || (request.method == "tools/call"
            && request
                .params
                .get("name")
                .and_then(serde_json::Value::as_str)
                == Some("registry.publish"));
    if !publish && body.len() > METADATA_BODY_LIMIT {
        reject_http(reader.get_mut(), "413 Payload Too Large")?;
        return Ok(());
    }

    let handle_result = |method: &str,
                         params: &serde_json::Value|
     -> Result<serde_json::Value, String> {
        match method {
            "registry.search" => {
                let args: SearchArgs = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::search::handle_search(&storage, &args)
            }
            "registry.manifest" => {
                let args: ManifestArgs = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::manifest::handle_manifest(&storage, &args, server_signing_key.as_ref())
            }
            "registry.artifact" => {
                let args: ArtifactArgs = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::artifact::handle_artifact(&storage, &args)
            }
            "registry.publish" => {
                let pkg: SignedPackage = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::publish::handle_publish(&storage, &pkg)
            }
            "registry.deprecate" => {
                let args: DeprecateArgs = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::deprecate::handle_deprecate(
                    &storage,
                    &args,
                    registry_uri,
                    vetters.as_deref().map(|(keyring, root)| (keyring, root)),
                )
            }
            "registry.yanks_since" => {
                let args: YanksSinceArgs = serde_json::from_value(params.clone())
                    .map_err(|e| format!("invalid params: {e}"))?;
                handlers::yanks_since::handle_yanks_since(&storage, &args)
            }
            unknown => Err(format!("unknown method: {unknown}")),
        }
    };

    let result = if request.method == "tools/call" {
        (|| {
            let method = request
                .params
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("tools/call requires name")?;
            let args = request
                .params
                .get("arguments")
                .filter(|args| args.is_object())
                .ok_or("tools/call requires arguments object")?;
            handle_result(method, args)
        })()
    } else {
        handle_result(&request.method, &request.params)
    };

    let head = result
        .as_ref()
        .ok()
        .and_then(|value| value.get("head"))
        .and_then(|value| value.as_u64());
    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(msg) => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: msg,
            }),
        },
    };

    let resp_json = serde_json::to_vec(&response)?;
    let signed = headers
        .nonce
        .zip(server_signing_key)
        .map(|(nonce, seed)| {
            crate::security::sign_response(registry_uri, nonce, &body, &resp_json, head, &seed)
        })
        .transpose()?;
    respond_http(reader.get_mut(), "200 OK", &resp_json, signed.as_deref())?;
    Ok(())
}

fn reject_http(stream: &mut ServerStream, status: &str) -> std::io::Result<()> {
    respond_http(stream, status, &[], None)
}

fn respond_http(
    stream: &mut ServerStream,
    status: &str,
    body: &[u8],
    signature: Option<&str>,
) -> std::io::Result<()> {
    // A consumed request deadline must never suppress an HTTP error (or a
    // successful publish receipt after durable storage). Writes have their own
    // absolute bound; do not extend the five-second request-read deadline.
    stream.set_deadline(std::time::Instant::now() + REQUEST_DEADLINE);
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    )?;
    if let Some(signature) = signature {
        write!(
            stream,
            "{}: {signature}\r\n",
            crate::security::RESPONSE_HEADER
        )?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    stream.flush()?;
    stream.shutdown_write()?;
    stream.set_deadline(std::time::Instant::now() + std::time::Duration::from_millis(100));
    let _ = std::io::copy(&mut stream.take(64 * 1024 * 1024), &mut std::io::sink());
    Ok(())
}

struct RequestHeaders {
    content_length: usize,
    body_limit: usize,
    nonce: Option<[u8; 32]>,
}
fn read_request_headers(
    reader: &mut impl BufRead,
    endpoint: &url::Url,
    development: bool,
) -> Result<RequestHeaders, &'static str> {
    let mut line = String::new();
    let mut bytes = 0;
    let mut next_line = |line: &mut String| -> Result<(), &'static str> {
        line.clear();
        let remaining = MAX_HEADER_BYTES - bytes;
        let read = (&mut *reader)
            .take((remaining + 1) as u64)
            .read_line(line)
            .map_err(|e| {
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) {
                    "408 Request Timeout"
                } else {
                    "400 Bad Request"
                }
            })?;
        if read > remaining {
            return Err("431 Request Header Fields Too Large");
        }
        bytes += read;
        if read == 0 || !line.ends_with("\r\n") {
            return Err("400 Bad Request");
        }
        Ok(())
    };
    next_line(&mut line)?;
    let mut parts = line.split_whitespace();
    if parts.next() != Some("POST") {
        return Err("405 Method Not Allowed");
    }
    if parts.next() != Some("/mcp") {
        return Err("404 Not Found");
    }
    if parts.next() != Some("HTTP/1.1") || parts.next().is_some() {
        return Err("400 Bad Request");
    }
    let (mut host, mut json, mut origin) = (false, false, false);
    let mut length = None;
    let mut nonce = None;
    let mut development_opt_in = false;
    let mut publish_hint = None;
    for _ in 0..=MAX_HEADERS {
        next_line(&mut line)?;
        if line == "\r\n" {
            if !host {
                return Err("400 Bad Request");
            }
            if !json {
                return Err("415 Unsupported Media Type");
            }
            if development != development_opt_in || (!development && nonce.is_none()) {
                return Err("403 Forbidden");
            }
            return Ok(RequestHeaders {
                content_length: length.ok_or("411 Length Required")?,
                nonce,
                body_limit: if publish_hint == Some(true) {
                    UPLOAD_BODY_LIMIT
                } else {
                    METADATA_BODY_LIMIT
                },
            });
        }
        let (name, value) = line.trim_end().split_once(':').ok_or("400 Bad Request")?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("host") {
            if host {
                return Err("400 Bad Request");
            }
            if value != &endpoint[url::Position::BeforeHost..url::Position::AfterPort] {
                return Err("403 Forbidden");
            }
            host = true;
        } else if name.eq_ignore_ascii_case("origin") {
            if origin {
                return Err("400 Bad Request");
            }
            if value != endpoint.origin().ascii_serialization() {
                return Err("403 Forbidden");
            }
            origin = true;
        } else if name.eq_ignore_ascii_case("content-type") {
            if json {
                return Err("400 Bad Request");
            }
            if !value
                .split(';')
                .next()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
            {
                return Err("415 Unsupported Media Type");
            }
            json = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            if length
                .replace(value.parse().map_err(|_| "400 Bad Request")?)
                .is_some()
            {
                return Err("400 Bad Request");
            }
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("400 Bad Request");
        } else if name.eq_ignore_ascii_case("mcp-name") {
            if publish_hint.replace(value == "registry.publish").is_some() {
                return Err("400 Bad Request");
            }
        } else if name.eq_ignore_ascii_case(crate::security::NONCE_HEADER) {
            let decoded = hex::decode(value).map_err(|_| "400 Bad Request")?;
            if nonce
                .replace(decoded.try_into().map_err(|_| "400 Bad Request")?)
                .is_some()
            {
                return Err("400 Bad Request");
            }
        } else if name.eq_ignore_ascii_case(crate::security::DEVELOPMENT_HEADER) {
            if development_opt_in || value != "true" {
                return Err("400 Bad Request");
            }
            development_opt_in = true;
        }
    }
    Err("431 Request Header Fields Too Large")
}

enum ServerStream {
    Plain(DeadlineStream),
    #[cfg(feature = "tls")]
    Tls(Box<rustls::StreamOwned<rustls::ServerConnection, DeadlineStream>>),
}
impl ServerStream {
    fn handshake_complete(&self) -> bool {
        match self {
            Self::Plain(_) => true,
            #[cfg(feature = "tls")]
            Self::Tls(stream) => !stream.conn.is_handshaking(),
        }
    }
    fn socket(&mut self) -> &mut DeadlineStream {
        match self {
            Self::Plain(stream) => stream,
            #[cfg(feature = "tls")]
            Self::Tls(stream) => &mut stream.sock,
        }
    }
    fn set_deadline(&mut self, deadline: std::time::Instant) {
        self.socket().deadline = deadline;
    }
    fn shutdown_write(&mut self) -> std::io::Result<()> {
        #[cfg(feature = "tls")]
        if let Self::Tls(stream) = self {
            stream.conn.send_close_notify();
            stream.flush()?;
        }
        self.socket().stream.shutdown(std::net::Shutdown::Write)
    }
}
impl Read for ServerStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(bytes),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.read(bytes),
        }
    }
}
impl Write for ServerStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(bytes),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.flush(),
        }
    }
}

pub(crate) struct DeadlineStream {
    stream: TcpStream,
    deadline: std::time::Instant,
}
impl DeadlineStream {
    pub(crate) fn new(stream: TcpStream, deadline: std::time::Instant) -> Self {
        Self { stream, deadline }
    }
    fn remaining(&self) -> std::io::Result<std::time::Duration> {
        self.deadline
            .checked_duration_since(std::time::Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "registry request deadline")
            })
    }
}
impl Read for DeadlineStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(bytes)
    }
}
impl Write for DeadlineStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_default_ports_use_the_same_canonical_uri_as_clients() {
        assert_eq!(
            development_uri(([127, 0, 0, 1], 80).into()).unwrap(),
            "http://127.0.0.1/mcp"
        );
        assert_eq!(
            development_uri((std::net::Ipv6Addr::LOCALHOST, 80).into()).unwrap(),
            "http://[::1]/mcp"
        );
    }
}
