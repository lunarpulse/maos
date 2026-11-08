use maos_registry::{storage::LocalFsRegistryStorage, SpiritRegistryServer};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
struct Server {
    addr: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    _store: tempfile::TempDir,
}
impl Server {
    fn start() -> Self {
        let store = tempfile::tempdir().unwrap();
        let storage = LocalFsRegistryStorage::at_path(store.path().to_owned()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = SpiritRegistryServer::new(Arc::new(storage), addr.to_string(), None)
            .with_development_http();
        let stop = Arc::new(AtomicBool::new(false));
        let shutdown = stop.clone();
        let worker = std::thread::spawn(move || server.serve(listener, shutdown).unwrap());
        Self {
            addr,
            stop,
            worker: Some(worker),
            _store: store,
        }
    }
    fn socket(&self) -> TcpStream {
        let stream = TcpStream::connect(self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
    }
    fn request(&self, tool: &str, length: usize, body: &[u8]) -> String {
        let mut stream = self.socket();
        write!(stream,"POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nMcp-Name: {tool}\r\nX-Maos-Registry-Development: true\r\nContent-Length: {length}\r\n\r\n",self.addr).unwrap();
        stream.write_all(body).unwrap();
        let mut status = String::new();
        let read = BufReader::new(stream).read_line(&mut status);
        assert!(read.is_ok(), "no bounded HTTP refusal: {read:?}");
        status
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let result = self.worker.take().unwrap().join();
        if !std::thread::panicking() {
            assert!(result.is_ok());
        }
    }
}
#[test]
fn one_source_cannot_occupy_every_registry_slot_f1() {
    let server = Server::start();
    let _first = server.socket();
    let _second = server.socket();
    let body =
        br#"{"jsonrpc":"2.0","id":1,"method":"registry.yanks_since","params":{"since_ns":0}}"#;
    let status = server.request("registry.yanks_since", body.len(), body);
    assert!(
        status.contains("503"),
        "source exceeded its admission cap: {status}"
    );
}
#[test]
fn idle_first_byte_is_bounded_more_tightly_than_a_request_f1() {
    let server = Server::start();
    let mut stream = BufReader::new(server.socket());
    let mut status = String::new();
    let read = stream.read_line(&mut status);
    assert!(
        matches!(read, Ok(size) if size > 0),
        "idle socket kept its slot beyond the first-byte deadline: {read:?}"
    );
    assert!(status.contains("408"), "{status}");
}
#[test]
fn metadata_body_limit_is_checked_before_body_allocation_f2() {
    let server = Server::start();
    let status = server.request("registry.yanks_since", 64 * 1024 + 1, b"");
    assert!(
        status.contains("413"),
        "oversized metadata request was not refused from headers: {status}"
    );
}
#[test]
fn publication_hint_cannot_widen_another_rpc_body_cap_f2() {
    let server = Server::start();
    let body = serde_json::to_vec(&serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"registry.yanks_since",
        "params":{"since_ns":0},"padding":"x".repeat(96 * 1024)
    }))
    .unwrap();
    let status = server.request("registry.publish", body.len(), &body);
    assert!(
        status.contains("413"),
        "forged upload hint widened a metadata operation: {status}"
    );
}
