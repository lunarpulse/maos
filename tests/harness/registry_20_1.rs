//! Process-isolated registry exit witnesses; never invoke cargo from a test.
#![allow(dead_code)]
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
#[path = "fixtures.rs"]
mod fixtures;

pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "registry-20-1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(path.join("home")).unwrap();
        std::fs::create_dir_all(path.join("home/.config/maos")).unwrap();
        std::fs::write(
            path.join("home/.config/maos/operator.toml"),
            "[registry]\ndevelopment_http = true\n",
        )
        .unwrap();
        Self(path)
    }
    pub fn command(&self, binary: &Path) -> Command {
        let mut command = Command::new(binary);
        command
            .env("HOME", self.0.join("home"))
            .env("MAOS_HOME", self.0.join("home/.maos"))
            .env("XDG_DATA_HOME", self.0.join("home/.local/share"))
            .env("XDG_CONFIG_HOME", self.0.join("home/.config"))
            .env_remove("MAOS_REGISTRY_ORG_SIGNING_PUBKEY")
            .env_remove("MAOS_REGISTRY_REQUIRE_SERVER_TIER_SIGNATURE")
            .env_remove("MAOS_REGISTRY_TIER_FLOOR");
        command
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn sibling(name: &str) -> PathBuf {
    fixtures::fixture(name)
}
pub fn rpc(addr: &str, method: &str, params: serde_json::Value) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let body = serde_json::to_vec(
        &serde_json::json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}),
    )
    .unwrap();
    write!(
        stream,
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nMcp-Name: {method}\r\nX-Maos-Registry-Development: true\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let start = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    serde_json::from_slice(&response[start..]).unwrap()
}
pub struct Server {
    pub child: Child,
    pub addr: String,
}
impl Server {
    pub fn start(scratch: &Scratch, binary: &Path) -> Self {
        let mut child = scratch
            .command(binary)
            .args(["--dev-http", "--bind", "127.0.0.1:0", "--root"])
            .arg(scratch.0.join("registry"))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                if let Some(addr) = line.strip_prefix("maos-registry: listening on ") {
                    let _ = tx.send(addr.to_owned());
                }
            }
        });
        let addr = match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(addr) => addr,
            Err(error) => {
                let _ = child.kill();
                let status = child.wait().unwrap();
                panic!("registry readiness failed: {error}; {status}");
            }
        };
        Self { child, addr }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct InProcessServer {
    pub addr: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl InProcessServer {
    pub fn start(scratch: &Scratch) -> Self {
        Self::start_signed(scratch, None)
    }
    pub fn start_signed(scratch: &Scratch, key: Option<[u8; 32]>) -> Self {
        Self::start_config(scratch, key, None)
    }
    pub fn start_config(
        scratch: &Scratch,
        key: Option<[u8; 32]>,
        vetters: Option<(maos_compliance::VetterKeyring, [u8; 32])>,
    ) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let storage =
            maos_registry::storage::LocalFsRegistryStorage::at_path(scratch.0.join("registry"))
                .unwrap();
        let server = maos_registry::SpiritRegistryServer::new(
            std::sync::Arc::new(storage),
            addr.clone(),
            None,
        )
        .with_development_http();
        let server = if let Some(key) = key {
            server.with_server_signing_key(key)
        } else {
            server
        };
        let server = if let Some((keyring, root)) = vetters {
            server.with_vetter_keyring(keyring, root)
        } else {
            server
        };
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = stop.clone();
        let worker = std::thread::spawn(move || server.serve(listener, signal).unwrap());
        Self {
            addr,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for InProcessServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}

pub fn publish(
    scratch: &Scratch,
    publisher: &Path,
    addr: &str,
    manifest: &Path,
) -> std::process::Output {
    use std::os::unix::fs::PermissionsExt;
    let key = scratch.0.join("publisher.key");
    std::fs::write(&key, "53".repeat(32)).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    let artifact = scratch.0.join("spirit.wasm");
    std::fs::write(&artifact, b"\0asm\x0d\0\x01\0").unwrap();
    publish_artifact(scratch, publisher, addr, manifest, &artifact, &key)
}

pub fn publish_artifact(
    scratch: &Scratch,
    publisher: &Path,
    addr: &str,
    manifest: &Path,
    artifact: &Path,
    key: &Path,
) -> std::process::Output {
    scratch
        .command(publisher)
        .args([
            "publish",
            "--tier=public_untrusted",
            "--registry-uri",
            &format!("http://{addr}/mcp"),
            "--manifest",
        ])
        .arg(manifest)
        .arg("--artifact")
        .arg(artifact)
        .arg("--signing-key")
        .arg(key)
        .output()
        .unwrap()
}
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
pub fn example(scratch: &Scratch) -> PathBuf {
    let text = std::fs::read_to_string(root().join("examples/example-spirit-ts/manifest.toml"))
        .unwrap()
        .replace(
            "trust_tier = \"local\"",
            "trust_tier = \"public-untrusted\"",
        );
    let path = scratch.0.join("manifest.toml");
    std::fs::write(&path, text).unwrap();
    path
}

pub fn installed_version(scratch: &Scratch) -> PathBuf {
    maos_registry::storage::LocalFsRegistryStorage::at_path(
        scratch.0.join("home/.local/share/maos/registry"),
    )
    .unwrap()
    .installed_manifests()
    .unwrap()
    .into_iter()
    .find(|(name, _)| name == "example-spirit-ts")
    .unwrap()
    .1
    .parent()
    .unwrap()
    .to_owned()
}
