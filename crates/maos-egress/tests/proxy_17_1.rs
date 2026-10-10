#![cfg(all(unix, feature = "daemon"))]

use maos_domain::host_grant::EgressDestination;
use maos_egress::{
    Allowlist, BearerTable, Credential, Limits, ProxyConfig, ProxyGuard, Refusal, RefusalSink,
};
use parking_lot::Mutex;
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    sync::{mpsc, Arc},
    time::Duration,
};

struct ConfirmGate {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl RefusalSink for ConfirmGate {
    fn confirm(&self, _: &Refusal<'_>) -> io::Result<()> {
        self.entered.send(()).map_err(io::Error::other)?;
        self.release
            .lock()
            .recv_timeout(Duration::from_secs(3))
            .map_err(io::Error::other)
    }
}

/// The caller's private per-root directory, as `maos-bin` creates it.
fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

#[test]
fn forbidden_destination_waits_for_confirmed_audit_before_any_response_byte() {
    let directory = private_directory();
    let table = Arc::new(BearerTable::default());
    let allowed: EgressDestination = "api.example.com:443".to_owned().try_into().unwrap();
    let policy = Arc::new(Allowlist::intersect(
        std::slice::from_ref(&allowed),
        std::slice::from_ref(&allowed),
    ));
    let bearer = table
        .mint(17, "worker", policy, Duration::from_secs(60))
        .unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut proxy = ProxyGuard::bind(
        directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table,
            credential: Credential::Bearer("RAW-VENDOR-KEY-DO-NOT-ECHO".into()),
            sink: Arc::new(ConfirmGate {
                entered: entered_tx,
                release: Mutex::new(release_rx),
            }),
            limits: Limits::default(),
        },
    )
    .unwrap();
    let mut connection = UnixStream::connect(proxy.socket_path()).unwrap();
    connection.write_all(format!("GET /blocked.example.com:443/v1/test HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {bearer}\r\n\r\n").as_bytes()).unwrap();
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_millis(30)))
        .unwrap();
    let error = connection.read(&mut [0u8; 1]).unwrap_err();
    assert!(matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    ));
    release_tx.send(()).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut response = String::new();
    connection.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    assert!(!response.contains("RAW-VENDOR-KEY-DO-NOT-ECHO"));
    assert!(!response.contains(&bearer));
    proxy.stop_and_join().unwrap();
}

struct BrokenJournal;
impl RefusalSink for BrokenJournal {
    fn confirm(&self, _: &Refusal<'_>) -> io::Result<()> {
        Err(io::Error::other("journal unavailable"))
    }
}

#[test]
fn an_unconfirmed_refusal_cannot_emit_a_successfully_audited_response() {
    let directory = private_directory();
    let mut proxy = ProxyGuard::bind(
        directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table: Arc::new(BearerTable::default()),
            credential: Credential::None,
            sink: Arc::new(BrokenJournal),
            limits: Limits::default(),
        },
    )
    .unwrap();
    let mut connection = UnixStream::connect(proxy.socket_path()).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    connection.write_all(b"GET /blocked.example.com:443/test HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrong\r\n\r\n").unwrap();
    let mut response = Vec::new();
    let _ = connection.read_to_end(&mut response);
    assert!(
        response.is_empty(),
        "no response is truthful without a confirmed journal write"
    );
    assert!(
        proxy.stop_and_join().is_err(),
        "root teardown must observe the journal failure"
    );
}
