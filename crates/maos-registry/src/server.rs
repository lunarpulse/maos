//! SpiritRegistryServer — MCP-Streamable-HTTP server on std::net::TcpListener.
//!
//! v0.5-α: HTTP-only on 127.0.0.1.  Each connection is handled by a dedicated
//! `std::thread::spawn` worker.  The worker reads one JSON-RPC 2.0 frame,
//! dispatches to the matching handler based on `method`, and returns the result.
//!
//! Per the kernel-stays-small invariant: NO `tokio::net`, NO `axum`, NO `hyper`.
//! Transport dependency is `std::net` + `std::io::BufRead` + `serde_json`.

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
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
const REQUEST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

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

/// The Spirit Registry server.
pub struct SpiritRegistryServer {
    storage: Arc<dyn RegistryStorage>,
    listen_addr: String,
    org_pubkey: Option<[u8; 32]>,
    /// Story 7.2 — Ed25519 signing key for server-side tier attestation.
    server_signing_key: Option<[u8; 32]>,
    vetters: Option<Arc<(maos_compliance::VetterKeyring, [u8; 32])>>,
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
        }
    }

    /// Story 7.2 — attach an Ed25519 signing key for server-side tier attestation.
    pub fn with_server_signing_key(mut self, key: [u8; 32]) -> Self {
        self.server_signing_key = Some(key);
        self
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

    /// Run the loopback development server, draining workers on SIGTERM.
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

    /// Serve an already-bound loopback listener; caller owns its shutdown flag.
    pub fn serve(self, listener: TcpListener, shutdown: Arc<AtomicBool>) -> Result<(), String> {
        let addr = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {e}"))?;
        if !addr.ip().is_loopback() {
            return Err("development registry requires a loopback bind; authenticated first-contact/TLS implementation is held for AC15".into());
        }
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        eprintln!("maos-registry: listening on {addr}");
        let storage = self.storage;
        let server_signing_key = self.server_signing_key;
        let vetters = self.vetters;
        let _ = &self.org_pubkey;
        let mut handles: Vec<(TcpStream, thread::JoinHandle<()>)> =
            Vec::with_capacity(MAX_CONNECTIONS);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            handles.retain(|(_, handle)| !handle.is_finished());
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if handles.len() >= MAX_CONNECTIONS {
                        let _ =
                            stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));
                        let _ = reject_http(&mut stream, "503 Service Unavailable");
                        continue;
                    }
                    let cancel = stream.try_clone().map_err(|e| e.to_string())?;
                    let s = Arc::clone(&storage);
                    let vetters = vetters.clone();
                    handles.push((
                        cancel,
                        thread::spawn(move || {
                            if let Err(error) =
                                handle_connection(s, server_signing_key, vetters, stream)
                            {
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
                }
            }
        }

        for (stream, _) in &handles {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        for (_, handle) in handles {
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
}

impl RegistryServerArgs {
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args;
        while let Some(flag) = args.next() {
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--bind" => result.bind = Some(value),
                "--root" => result.root = Some(value.into()),
                "--signing-key" => result.signing_key = Some(value.into()),
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
        let mut server = SpiritRegistryServer::new(
            Arc::new(storage),
            self.bind.unwrap_or_else(|| "127.0.0.1:6789".into()),
            None,
        );
        if let Some(path) = self.signing_key {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let seed = hex::decode(text.trim())
                .map_err(|e| format!("signing key hex: {e}"))?
                .try_into()
                .map_err(|_| "signing key must be a 32-byte hex seed")?;
            server = server.with_server_signing_key(seed);
        }
        match (self.vetter_keyring, self.operator_root) {
            (Some(path), Some(root)) => {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                let keyring = serde_cbor::from_slice(&bytes).map_err(|e| e.to_string())?;
                let root = hex::decode(root)
                    .map_err(|e| e.to_string())?
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
    mut stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = std::time::Instant::now() + REQUEST_DEADLINE;
    let mut reader = BufReader::new(DeadlineStream {
        stream: stream.try_clone()?,
        deadline,
    });
    let content_length = match read_request_headers(&mut reader) {
        Ok(length) => length,
        Err(status) => {
            reject_http(&mut stream, status)?;
            return Ok(());
        }
    };

    const MAX_BODY_SIZE: usize = 64 * 1024 * 1024;
    if content_length > MAX_BODY_SIZE {
        reject_http(&mut stream, "413 Payload Too Large")?;
        return Ok(());
    }

    // The request deadline bounds reads, not dispatch or the response write.
    let mut body = vec![0u8; content_length];
    if let Err(error) = reader.read_exact(&mut body) {
        let status = if matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) {
            "408 Request Timeout"
        } else {
            "400 Bad Request"
        };
        reject_http(&mut stream, status)?;
        return Ok(());
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
            respond_http(&mut stream, "200 OK", &resp_json)?;
            return Ok(());
        }
    };

    let registry_uri = format!("http://{}/mcp", stream.local_addr()?);
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
                    &registry_uri,
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
    respond_http(&mut stream, "200 OK", &resp_json)?;
    Ok(())
}

fn reject_http(stream: &mut TcpStream, status: &str) -> std::io::Result<()> {
    respond_http(stream, status, &[])
}

fn respond_http(stream: &mut TcpStream, status: &str, body: &[u8]) -> std::io::Result<()> {
    // A consumed request deadline must never suppress an HTTP error (or a
    // successful publish receipt after durable storage). Writes have their own
    // absolute bound; do not extend the five-second request-read deadline.
    let mut writer = DeadlineStream {
        stream: stream.try_clone()?,
        deadline: std::time::Instant::now() + REQUEST_DEADLINE,
    };
    write!(
        writer,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    writer.write_all(body)?;
    writer.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;
    // FIN delivers a complete response even when rejecting before the body.
    // Drain unread input briefly to avoid a reset on close, without allowing a
    // trickling peer to hold a worker beyond an absolute close deadline.
    let mut drain = DeadlineStream {
        stream: stream.try_clone()?,
        deadline: std::time::Instant::now() + std::time::Duration::from_millis(100),
    };
    let _ = std::io::copy(
        &mut (&mut drain).take(64 * 1024 * 1024),
        &mut std::io::sink(),
    );
    Ok(())
}

fn loopback_authority(value: &str) -> bool {
    if let Ok(socket) = value.parse::<std::net::SocketAddr>() {
        return socket.ip().is_loopback();
    }
    let host = if let Some(host) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        host
    } else if let Some((host, port)) = value.rsplit_once(':') {
        if value
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
        {
            return true;
        }
        if port.parse::<u16>().is_err() {
            return false;
        }
        host
    } else {
        value
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn read_request_headers(reader: &mut impl BufRead) -> Result<usize, &'static str> {
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
    for _ in 0..=MAX_HEADERS {
        next_line(&mut line)?;
        if line == "\r\n" {
            if !host {
                return Err("400 Bad Request");
            }
            if !json {
                return Err("415 Unsupported Media Type");
            }
            return length.ok_or("411 Length Required");
        }
        let (name, value) = line.trim_end().split_once(':').ok_or("400 Bad Request")?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("host") {
            if host {
                return Err("400 Bad Request");
            }
            if !loopback_authority(value) {
                return Err("403 Forbidden");
            }
            host = true;
        } else if name.eq_ignore_ascii_case("origin") {
            if origin {
                return Err("400 Bad Request");
            }
            if !value
                .strip_prefix("http://")
                .or_else(|| value.strip_prefix("https://"))
                .is_some_and(loopback_authority)
            {
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
        }
    }
    Err("431 Request Header Fields Too Large")
}

struct DeadlineStream {
    stream: TcpStream,
    deadline: std::time::Instant,
}
impl DeadlineStream {
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
    use crate::storage::LocalFsRegistryStorage;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    fn temp_storage() -> Arc<dyn RegistryStorage> {
        let dir =
            std::env::temp_dir().join(format!("maos-registry-server-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = LocalFsRegistryStorage::at_path(dir).unwrap();
        Arc::new(s)
    }

    #[test]
    #[ignore = "requires server to be running on a port — tested via end-to-end test"]
    fn server_handles_search() {
        // The real-wire test is at tests/end_to_end_test.rs (Task 16)
    }

    #[test]
    #[ignore = "requires server to be running on a port — tested via end-to-end test"]
    fn server_handles_unknown_method() {
        // The real-wire test is at tests/end_to_end_test.rs (Task 16)
    }
}
