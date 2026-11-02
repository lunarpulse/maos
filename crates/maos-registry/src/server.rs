//! SpiritRegistryServer — MCP-Streamable-HTTP server on std::net::TcpListener.
//!
//! v0.5-α: HTTP-only on 127.0.0.1.  Each connection is handled by a dedicated
//! `std::thread::spawn` worker.  The worker reads one JSON-RPC 2.0 frame,
//! dispatches to the matching handler based on `method`, and returns the result.
//!
//! Per the kernel-stays-small invariant: NO `tokio::net`, NO `axum`, NO `hyper`.
//! Transport dependency is `std::net` + `std::io::BufRead` + `serde_json`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::handlers;
use crate::operations::{ArtifactArgs, DeprecateArgs, ManifestArgs, SearchArgs, YanksSinceArgs};
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::SignedPackage;

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
        }
    }

    /// Story 7.2 — attach an Ed25519 signing key for server-side tier attestation.
    pub fn with_server_signing_key(mut self, key: [u8; 32]) -> Self {
        self.server_signing_key = Some(key);
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
            return Err("unauthenticated development registry requires a loopback bind (20-1b owns mutation authentication)".into());
        }
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        eprintln!("maos-registry: listening on {addr}");
        let storage = self.storage;
        let server_signing_key = self.server_signing_key;
        let _ = &self.org_pubkey;
        let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            handles.retain(|handle| !handle.is_finished());
            match listener.accept() {
                Ok((stream, _)) => {
                    let s = Arc::clone(&storage);
                    handles.push(thread::spawn(move || {
                        let _ = handle_connection(s, server_signing_key, stream);
                    }));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    eprintln!("maos-registry: accept error: {e}");
                }
            }
        }

        for h in handles {
            let _ = h.join();
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
                _ => return Err(format!("unknown registry flag {flag}")),
            }
        }
        Ok(result)
    }

    pub fn build(self) -> Result<SpiritRegistryServer, String> {
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
        Ok(server)
    }
}

/// Handle a single TCP connection — read one JSON-RPC request, dispatch, respond.
fn handle_connection(
    storage: Arc<dyn RegistryStorage>,
    server_signing_key: Option<[u8; 32]>,
    mut stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();

    // Read the HTTP request
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|e| format!("TcpStream::try_clone failed: {e}"))?,
    );

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return Ok(());
    }

    // Only handle POST /mcp
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 || parts[0] != "POST" {
        let _ = write!(stream, "HTTP/1.1 405 Method Not Allowed\r\n\r\n");
        return Ok(());
    }

    // Skip headers until empty line
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return Ok(());
        }
        let line = line.trim().to_string();
        if line.is_empty() {
            break;
        }
        if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    const MAX_BODY_SIZE: usize = 64 * 1024 * 1024;
    if content_length > MAX_BODY_SIZE {
        let _ = write!(
            stream,
            "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n"
        );
        return Ok(());
    }

    // Read body
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        use std::io::Read;
        if reader.read_exact(&mut body).is_err() {
            let _ = write!(stream, "HTTP/1.1 400 Bad Request\r\n\r\n");
            return Ok(());
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
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                resp_json.len(),
                String::from_utf8_lossy(&resp_json)
            );
            let _ = stream.write_all(response.as_bytes());
            return Ok(());
        }
    };

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
                handlers::deprecate::handle_deprecate(&storage, &args)
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
        let method = request
            .params
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or("tools/call requires name")?;
        let args = request
            .params
            .get("arguments")
            .ok_or("tools/call requires arguments")?;
        handle_result(method, args)
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
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        resp_json.len(),
        String::from_utf8_lossy(&resp_json)
    );
    let _ = stream.write_all(http_response.as_bytes());
    let _ = stream.flush();
    Ok(())
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
