#![cfg(all(unix, feature = "daemon"))]

use maos_domain::host_grant::EgressDestination;
use maos_egress::{
    Allowlist, BearerTable, Credential, Limits, ProxyConfig, ProxyGuard, Refusal, RefusalSink,
};
use parking_lot::Mutex;
use std::{
    fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::Path,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Journal {
    rows: Mutex<Vec<(u32, String, Option<String>, &'static str)>>,
}
impl RefusalSink for Journal {
    fn confirm(&self, refusal: &Refusal<'_>) -> io::Result<()> {
        self.rows.lock().push((
            refusal.spirit_pid,
            refusal.spirit_id.to_owned(),
            refusal.destination.map(ToString::to_string),
            refusal.reason,
        ));
        Ok(())
    }
}

struct Running {
    _directory: tempfile::TempDir,
    proxy: ProxyGuard,
    table: Arc<BearerTable>,
    policy: Arc<Allowlist>,
    bearer: String,
    journal: Arc<Journal>,
}

fn destination(value: &str) -> EgressDestination {
    value.to_owned().try_into().unwrap()
}

/// The caller's private per-root directory, as `maos-bin` creates it.
fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

fn start(destinations: &[EgressDestination], credential: Credential, limits: Limits) -> Running {
    let directory = private_directory();
    let table = Arc::new(BearerTable::default());
    let policy = Arc::new(Allowlist::intersect(destinations, destinations));
    let bearer = table
        .mint(17, "worker", Arc::clone(&policy), Duration::from_secs(60))
        .unwrap();
    let journal = Arc::new(Journal::default());
    let proxy = ProxyGuard::bind(
        directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table: Arc::clone(&table),
            credential,
            sink: journal.clone(),
            limits,
        },
    )
    .unwrap();
    Running {
        _directory: directory,
        proxy,
        table,
        policy,
        bearer,
        journal,
    }
}

fn connect(path: &Path) -> UnixStream {
    let connection = UnixStream::connect(path).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    connection
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    connection
}

fn exchange(proxy: &ProxyGuard, request: &str) -> String {
    let mut socket = connect(proxy.socket_path());
    socket.write_all(request.as_bytes()).unwrap();
    let mut bytes = Vec::new();
    // Closing without draining hostile bytes can report reset after a complete
    // refusal; preserve those bytes, rather than mistaking reset for no response.
    loop {
        let mut buffer = [0u8; 4096];
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => bytes.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => break,
            Err(error) => panic!("proxy response read failed: {:?}", error.kind()),
        }
    }
    String::from_utf8(bytes).unwrap()
}

fn request(route: &str, bearer: &str, extra: &str) -> String {
    format!("GET /{route}/v1/test HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {bearer}\r\n{extra}\r\n")
}

fn accept(listener: &TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return socket;
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => panic!("upstream accept failed: {:?}", error.kind()),
        }
    }
}

fn upstream_head(socket: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8; 1];
        socket.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        assert!(
            bytes.len() <= 64 * 1024,
            "upstream head exceeded test bound"
        );
    }
    String::from_utf8(bytes).unwrap()
}

#[test]
fn hostile_framing_and_header_smuggling_are_confirmed_refusals() {
    let allowed = destination("127.0.0.1:9");
    let mut running = start(&[allowed], Credential::None, Limits::default());
    let route = "127.0.0.1:9";
    let malformed = [
        "Content-Length: 0\r\nContent-Length: 0\r\n",
        "Content-Length: 0\r\ncontent-length: 1\r\n",
        "Host: another.example\r\n",
        "Authorization: Bearer duplicated\r\n",
        "x-api-key: second-credential\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Content-Length: 0\r\nTransfer-Encoding: chunked\r\n",
        "Transfer-Encoding: identity\r\n",
        "Content-Length: +1\r\n",
        "Content-Length: 1, 1\r\n",
        "Content-Length: 99999999999999999999999999\r\n",
        " X-Folded: value\r\n",
        "X-Test: one\r\n two\r\n",
        "X-Test: value\tother\r\n",
        "X-Test: value\u{7f}\r\n",
        "Bad Name: value\r\n",
        "Connection: authorization\r\n",
        "Expect: 100-continue\r\n",
        "Upgrade: websocket\r\n",
        "Proxy-Authorization: Bearer bypass\r\n",
    ];
    for extra in malformed {
        let response = exchange(&running.proxy, &request(route, &running.bearer, extra));
        assert!(
            response.starts_with("HTTP/1.1 403"),
            "malformed input was not refused"
        );
        assert!(!response.contains(&running.bearer));
    }
    let x_api_duplicate = format!("GET /{route}/test HTTP/1.1\r\nHost: localhost\r\nx-api-key: {}\r\nX-Api-Key: duplicate\r\n\r\n", running.bearer);
    assert!(exchange(&running.proxy, &x_api_duplicate).starts_with("HTTP/1.1 403"));
    assert_eq!(running.journal.rows.lock().len(), malformed.len() + 1);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn traversal_connect_and_noncanonical_routes_never_reach_upstream() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let mut running = start(&[destination(&route)], Credential::None, Limits::default());
    let paths = [
        "/../secret",
        "/./secret",
        "/%2e%2e/secret",
        "/%252e%252e/secret",
        "/%2fblocked",
        "/%5cblocked",
        "/bad\\path",
        "//other.example/secret",
        "/bad%",
        "/bad%0d%0a",
        "/test#fragment",
    ];
    for path in paths {
        let wire = format!(
            "GET /{route}{path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            running.bearer
        );
        assert!(exchange(&running.proxy, &wire).starts_with("HTTP/1.1 403"));
    }
    let wire = format!(
        "CONNECT {route} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
        running.bearer
    );
    assert!(exchange(&running.proxy, &wire).starts_with("HTTP/1.1 403"));
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(running.journal.rows.lock().len(), paths.len() + 1);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn request_header_and_body_bounds_are_enforced() {
    let limits = Limits {
        header_bytes: 256,
        body_bytes: 4,
        ..Limits::default()
    };
    let mut running = start(&[destination("127.0.0.1:9")], Credential::None, limits);
    let huge = request(
        "127.0.0.1:9",
        &running.bearer,
        &format!("X-Padding: {}\r\n", "z".repeat(257)),
    );
    assert!(exchange(&running.proxy, &huge).starts_with("HTTP/1.1 403"));
    let oversized_body = request("127.0.0.1:9", &running.bearer, "Content-Length: 5\r\n");
    assert!(exchange(&running.proxy, &oversized_body).starts_with("HTTP/1.1 403"));
    let unexpected_body = format!("{}extra", request("127.0.0.1:9", &running.bearer, ""));
    assert!(exchange(&running.proxy, &unexpected_body).starts_with("HTTP/1.1 403"));
    assert_eq!(running.journal.rows.lock().len(), 3);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn header_deadline_is_absolute_despite_regular_slowloris_bytes() {
    let limits = Limits {
        header_timeout: Duration::from_millis(150),
        io_timeout: Duration::from_secs(1),
        ..Limits::default()
    };
    let mut running = start(&[], Credential::None, limits);
    let mut socket = connect(running.proxy.socket_path());
    let began = Instant::now();
    socket
        .write_all(b"GET /127.0.0.1:9/test HTTP/1.1\r\n")
        .unwrap();
    let mut drip_socket = socket.try_clone().unwrap();
    let drip = thread::spawn(move || {
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(35));
            if drip_socket.write_all(b"a").is_err() {
                break;
            }
        }
    });
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"));
    assert!(
        began.elapsed() < Duration::from_millis(700),
        "a fresh byte must not reset the deadline"
    );
    drip.join().unwrap();
    assert_eq!(running.journal.rows.lock().len(), 1);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn request_body_deadline_is_absolute_despite_regular_slowloris_bytes() {
    let limits = Limits {
        body_timeout: Duration::from_millis(150),
        io_timeout: Duration::from_secs(1),
        ..Limits::default()
    };
    let mut running = start(&[], Credential::None, limits);
    let mut socket = connect(running.proxy.socket_path());
    let began = Instant::now();
    let head = format!(
        "POST /127.0.0.1:9/v1/test HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Length: 64\r\n\r\n",
        running.bearer
    );
    socket.write_all(head.as_bytes()).unwrap();
    socket.write_all(b"0123456789abcdef").unwrap();
    let mut drip_socket = socket.try_clone().unwrap();
    let drip = thread::spawn(move || {
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(35));
            if drip_socket.write_all(b"a").is_err() {
                break;
            }
        }
    });
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"));
    assert!(
        began.elapsed() < Duration::from_millis(700),
        "a fresh body byte must not reset the deadline"
    );
    drip.join().unwrap();
    assert_eq!(running.journal.rows.lock().len(), 1);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn wrong_expired_foreign_pid_and_foreign_spirit_tokens_do_not_connect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let allowed = destination(&route);
    let mut running = start(
        std::slice::from_ref(&allowed),
        Credential::Bearer("private-vendor-key".into()),
        Limits::default(),
    );
    let foreign_pid = running
        .table
        .mint(
            18,
            "worker",
            Arc::clone(&running.policy),
            Duration::from_secs(60),
        )
        .unwrap();
    for bearer in ["wrong", foreign_pid.as_str()] {
        let response = exchange(&running.proxy, &request(&route, bearer, ""));
        assert!(response.starts_with("HTTP/1.1 403"));
        assert!(!response.contains(bearer));
        assert!(!response.contains("private-vendor-key"));
    }
    // Exercise expiry while this is still the current pid entry: reminting must
    // not be the reason this token fails.
    let expired = running
        .table
        .mint(17, "worker", Arc::clone(&running.policy), Duration::ZERO)
        .unwrap();
    assert!(exchange(&running.proxy, &request(&route, &expired, "")).starts_with("HTTP/1.1 403"));
    let foreign_spirit = running
        .table
        .mint(
            17,
            "different-worker",
            Arc::clone(&running.policy),
            Duration::from_secs(60),
        )
        .unwrap();
    for bearer in [foreign_spirit.as_str(), running.bearer.as_str()] {
        let response = exchange(&running.proxy, &request(&route, bearer, ""));
        assert!(response.starts_with("HTTP/1.1 403"));
        assert!(!response.contains(bearer));
        assert!(!response.contains("private-vendor-key"));
    }
    let missing = format!("GET /{route}/test HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(exchange(&running.proxy, &missing).starts_with("HTTP/1.1 403"));
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(running.journal.rows.lock().len(), 6);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn a_granted_localhost_hostname_is_not_the_plaintext_loopback_exception() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("localhost:{}", listener.local_addr().unwrap().port());
    let mut running = start(&[destination(&route)], Credential::None, Limits::default());
    assert!(
        exchange(&running.proxy, &request(&route, &running.bearer, "")).starts_with("HTTP/1.1 403")
    );
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    let rows = running.journal.rows.lock();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].3, "unsafe-upstream-resolution");
    drop(rows);
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn permitted_loopback_reaches_an_authenticated_virtual_host() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let expected_route = route.clone();
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let head = upstream_head(&mut socket);
        let lower = head.to_ascii_lowercase();
        let mut body = [0u8; 4];
        socket.read_exact(&mut body).unwrap();
        let response = if !lower
            .lines()
            .filter(|line| line.starts_with("authorization:"))
            .eq(["authorization: bearer vendor-secret-bearer"])
        {
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else if !lower.contains(&format!("host: {expected_route}\r\n")) {
            "HTTP/1.1 421 Misdirected Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else if !head.starts_with("POST /v1/test?name=value HTTP/1.1\r\n")
            || &body != b"data"
            || lower.contains("x-drop:")
            || lower.contains("keep-alive:")
            || lower.contains("transfer-encoding:")
            || lower.contains("x-api-key:")
        {
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else {
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
        };
        socket.write_all(response.as_bytes()).unwrap();
    });
    let mut running = start(
        &[destination(&route)],
        Credential::Bearer("vendor-secret-bearer".into()),
        Limits::default(),
    );
    let wire = format!("POST /{route}/v1/test?name=value HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nConnection: X-Drop, keep-alive\r\nX-Drop: never-forward\r\nKeep-Alive: timeout=999\r\nContent-Length: 4\r\nContent-Type: application/json\r\n\r\ndata", running.bearer);
    let response = exchange(&running.proxy, &wire);
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("ok"));
    assert!(running.journal.rows.lock().is_empty());
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn a_scoped_api_key_reaches_the_vendor_authenticated_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let lower = upstream_head(&mut socket).to_ascii_lowercase();
        let authenticated = lower
            .lines()
            .filter(|line| line.starts_with("x-api-key:"))
            .eq(["x-api-key: vendor-secret-api-key"])
            && !lower.contains("authorization:");
        let response = if authenticated {
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
        } else {
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        };
        socket.write_all(response.as_bytes()).unwrap();
    });
    let mut running = start(
        &[destination(&route)],
        Credential::ApiKey("vendor-secret-api-key".into()),
        Limits::default(),
    );
    let wire = format!(
        "GET /{route}/v1/test HTTP/1.1\r\nHost: localhost\r\nx-api-key: {}\r\n\r\n",
        running.bearer
    );
    assert!(exchange(&running.proxy, &wire).starts_with("HTTP/1.1 200"));
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn upstream_refusals_preserve_status_but_never_echo_secret_error_data() {
    for status in [401, 403, 429, 500] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        let upstream = thread::spawn(move || {
            let mut socket = accept(&listener);
            let _ = upstream_head(&mut socket);
            let body = "vendor-secret-api-key";
            let response = format!("HTTP/1.1 {status} vendor-secret-api-key\r\nX-Error: vendor-secret-api-key\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).unwrap();
        });
        let mut running = start(
            &[destination(&route)],
            Credential::ApiKey("vendor-secret-api-key".into()),
            Limits::default(),
        );
        let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
        assert!(response.starts_with(&format!("HTTP/1.1 {status}")));
        assert!(!response.contains("vendor-secret-api-key"));
        assert!(!response.contains(&running.bearer));
        let rows = running.journal.rows.lock();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].3.contains("vendor-secret-api-key"));
        drop(rows);
        upstream.join().unwrap();
        running.proxy.stop_and_join().unwrap();
    }
}

#[test]
fn redirects_are_refused_without_contacting_even_a_granted_second_destination() {
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", first.local_addr().unwrap().port());
    let second_route = format!("127.0.0.1:{}", second.local_addr().unwrap().port());
    let redirect_route = second_route.clone();
    let upstream = thread::spawn(move || {
        let mut socket = accept(&first);
        let _ = upstream_head(&mut socket);
        let response = format!("HTTP/1.1 302 Found\r\nLocation: http://{redirect_route}/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        socket.write_all(response.as_bytes()).unwrap();
    });
    let mut running = start(
        &[destination(&route), destination(&second_route)],
        Credential::None,
        Limits::default(),
    );
    assert!(
        exchange(&running.proxy, &request(&route, &running.bearer, "")).starts_with("HTTP/1.1 403")
    );
    second.set_nonblocking(true).unwrap();
    assert!(matches!(second.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(
        running.journal.rows.lock()[0].3,
        "upstream-redirect-forbidden"
    );
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn sse_bytes_arrive_before_upstream_eof_and_without_upstream_chunk_framing() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let (release_tx, release_rx) = mpsc::channel();
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let _ = upstream_head(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nD\r\ndata: first\n\n\r\n").unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        socket
            .write_all(b"E\r\ndata: second\n\n\r\n0\r\n\r\n")
            .unwrap();
    });
    let mut running = start(&[destination(&route)], Credential::None, Limits::default());
    let mut socket = connect(running.proxy.socket_path());
    socket
        .write_all(request(&route, &running.bearer, "").as_bytes())
        .unwrap();
    let mut received = Vec::new();
    let first = b"data: first\n\n";
    while !received.windows(first.len()).any(|window| window == first) {
        let mut buffer = [0u8; 256];
        let count = socket.read(&mut buffer).unwrap();
        assert!(count > 0, "stream closed before the first event");
        received.extend_from_slice(&buffer[..count]);
        assert!(received.len() < 4096);
    }
    let text = String::from_utf8(received.clone())
        .unwrap()
        .to_ascii_lowercase();
    assert!(text.contains("content-type: text/event-stream"));
    assert!(!text.contains("transfer-encoding:"));
    assert!(!text.contains("content-length:"));
    release_tx.send(()).unwrap();
    socket.read_to_end(&mut received).unwrap();
    assert!(received.ends_with(b"data: second\n\n"));
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn listener_backpressure_bounds_active_connections_and_stop_interrupts_partial_heads() {
    let limits = Limits {
        max_connections: 1,
        header_timeout: Duration::from_secs(3),
        ..Limits::default()
    };
    let mut running = start(&[], Credential::None, limits);
    let mut first = connect(running.proxy.socket_path());
    first.write_all(b"GET /").unwrap();
    // The first request remains active; the second can enter only the bounded OS backlog.
    let mut second = connect(running.proxy.socket_path());
    second
        .write_all(b"GET /127.0.0.1:9/test HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    second
        .set_read_timeout(Some(Duration::from_millis(60)))
        .unwrap();
    assert!(
        matches!(second.read(&mut [0u8; 1]), Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut))
    );
    assert!(running.journal.rows.lock().is_empty());
    let path = running.proxy.socket_path().to_owned();
    let began = Instant::now();
    running.proxy.stop_and_join().unwrap();
    assert!(began.elapsed() < Duration::from_secs(1));
    assert!(!path.exists());
    running.proxy.stop_and_join().unwrap();
    assert!(!running
        .table
        .verify(17, "worker", &destination("127.0.0.1:9"), &running.bearer));
    match first.read(&mut [0u8; 1]) {
        Ok(0) => {}
        Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
        _ => panic!("shutdown did not close the partial-head client"),
    }
}

#[test]
fn drop_revokes_token_and_removes_private_directory_and_socket() {
    let running = start(
        &[destination("127.0.0.1:9")],
        Credential::None,
        Limits::default(),
    );
    let path = running.proxy.socket_path().to_owned();
    let directory = path.parent().unwrap().to_owned();
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o666
    );
    assert!(running
        .table
        .verify(17, "worker", &destination("127.0.0.1:9"), &running.bearer));
    // Moving the guard drops it without calling explicit stop.
    drop(running.proxy);
    assert!(!path.exists());
    assert!(!directory.exists());
    assert!(!running
        .table
        .verify(17, "worker", &destination("127.0.0.1:9"), &running.bearer));
}

#[test]
fn a_colliding_bind_does_not_unlink_the_existing_listener() {
    let mut running = start(
        &[destination("127.0.0.1:9")],
        Credential::None,
        Limits::default(),
    );
    let result = ProxyGuard::bind(
        running._directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "other".into(),
            table: Arc::clone(&running.table),
            credential: Credential::None,
            sink: running.journal.clone(),
            limits: Limits::default(),
        },
    );
    assert!(result.is_err());
    assert!(running.proxy.socket_path().exists());
    assert!(
        exchange(&running.proxy, &request("127.0.0.1:9", "wrong", "")).starts_with("HTTP/1.1 403")
    );
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn a_root_directory_others_can_enter_is_refused_before_binding() {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let table = Arc::new(BearerTable::default());
    let result = ProxyGuard::bind(
        directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table,
            credential: Credential::None,
            sink: Arc::new(Journal::default()),
            limits: Limits::default(),
        },
    );
    assert_eq!(
        result.err().map(|error| error.to_string()).as_deref(),
        Some("egress directory must be a private directory")
    );
    assert!(!directory.path().join("17").exists());
}

#[test]
fn stopping_an_active_upstream_stream_closes_both_ends_and_joins() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let (closed_tx, closed_rx) = mpsc::channel();
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let _ = upstream_head(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: ready\n\n").unwrap();
        let mut byte = [0u8; 1];
        let closed = matches!(socket.read(&mut byte), Ok(0));
        closed_tx.send(closed).unwrap();
    });
    let limits = Limits {
        io_timeout: Duration::from_secs(5),
        ..Limits::default()
    };
    let mut running = start(&[destination(&route)], Credential::None, limits);
    let mut socket = connect(running.proxy.socket_path());
    socket
        .write_all(request(&route, &running.bearer, "").as_bytes())
        .unwrap();
    let mut received = Vec::new();
    while !received.ends_with(b"data: ready\n\n") {
        let mut buffer = [0u8; 128];
        let count = socket.read(&mut buffer).unwrap();
        assert!(count > 0);
        received.extend_from_slice(&buffer[..count]);
        assert!(received.len() < 4096);
    }
    let began = Instant::now();
    running.proxy.stop_and_join().unwrap();
    assert!(
        began.elapsed() < Duration::from_secs(1),
        "shutdown must close upstream sockets, not await their I/O timeout"
    );
    assert!(closed_rx.recv_timeout(Duration::from_secs(3)).unwrap());
    assert!(
        running.journal.rows.lock().is_empty(),
        "normal shutdown is not an upstream refusal"
    );
    assert!(socket.read(&mut [0u8; 1]).is_ok_and(|count| count == 0));
    upstream.join().unwrap();
}

#[test]
fn transport_errors_are_confirmed_but_never_include_vendor_or_scoped_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let _ = upstream_head(&mut socket);
        // EOF before any upstream response head is a real transport failure.
    });
    let mut running = start(
        &[destination(&route)],
        Credential::Bearer("vendor-transport-secret".into()),
        Limits::default(),
    );
    let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
    assert!(response.starts_with("HTTP/1.1 502"));
    assert!(!response.contains("vendor-transport-secret"));
    assert!(!response.contains(&running.bearer));
    assert_eq!(
        running.journal.rows.lock()[0].3,
        "upstream-transport-failed"
    );
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

struct SecretBearingJournalError;
impl RefusalSink for SecretBearingJournalError {
    fn confirm(&self, _: &Refusal<'_>) -> io::Result<()> {
        Err(io::Error::other(
            "vendor-journal-secret scoped-journal-secret",
        ))
    }
}

#[test]
fn journal_failure_is_visible_idempotently_without_retaining_its_secret_diagnostic() {
    let directory = private_directory();
    let mut proxy = ProxyGuard::bind(
        directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table: Arc::new(BearerTable::default()),
            credential: Credential::Bearer("vendor-journal-secret".into()),
            sink: Arc::new(SecretBearingJournalError),
            limits: Limits::default(),
        },
    )
    .unwrap();
    let response = exchange(&proxy, &request("127.0.0.1:9", "scoped-journal-secret", ""));
    assert!(response.is_empty());
    let path = proxy.socket_path().to_owned();
    for _ in 0..2 {
        let error = proxy.stop_and_join().unwrap_err().to_string();
        assert!(!error.contains("vendor-journal-secret"));
        assert!(!error.contains("scoped-journal-secret"));
    }
    assert!(!path.exists());
}

#[test]
fn repeated_stop_and_late_drop_cannot_revoke_or_unlink_a_later_spawn_reusing_the_pid() {
    let allowed = destination("127.0.0.1:9");
    let mut running = start(
        std::slice::from_ref(&allowed),
        Credential::None,
        Limits::default(),
    );
    running.proxy.stop_and_join().unwrap();
    let mut replacement = ProxyGuard::bind(
        running._directory.path(),
        ProxyConfig {
            spirit_pid: 17,
            spirit_id: "worker".into(),
            table: Arc::clone(&running.table),
            credential: Credential::None,
            sink: running.journal.clone(),
            limits: Limits::default(),
        },
    )
    .unwrap();
    let new_bearer = running
        .table
        .mint(
            17,
            "worker",
            Arc::clone(&running.policy),
            Duration::from_secs(60),
        )
        .unwrap();
    running.proxy.stop_and_join().unwrap();
    drop(running.proxy);
    assert!(replacement.socket_path().exists());
    assert!(running.table.verify(17, "worker", &allowed, &new_bearer));
    assert!(
        exchange(&replacement, &request("127.0.0.1:9", "wrong", "")).starts_with("HTTP/1.1 403")
    );
    replacement.stop_and_join().unwrap();
}

struct CaptureLog(Mutex<Vec<String>>);
impl log::Log for CaptureLog {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        self.0.lock().push(record.args().to_string());
    }
    fn flush(&self) {}
}
static CAPTURE_LOG: CaptureLog = CaptureLog(Mutex::new(Vec::new()));

#[test]
fn direct_transport_debug_logging_never_records_a_vendor_api_key() {
    const CHILD: &str = "EGRESS_LOGGING_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "direct_transport_debug_logging_never_records_a_vendor_api_key",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "isolated credential logging regression failed"
        );
        return;
    }
    log::set_logger(&CAPTURE_LOG).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let key = "isolated-real-wire-vendor-key";
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let head = upstream_head(&mut socket);
        assert!(
            head.to_ascii_lowercase()
                .contains(&format!("x-api-key: {key}\r\n")),
            "the real vendor credential must reach the permitted upstream"
        );
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
    });
    let mut running = start(
        &[destination(&route)],
        Credential::ApiKey(key.into()),
        Limits::default(),
    );
    let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "credential replacement did not complete"
    );
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
    let records = CAPTURE_LOG.0.lock();
    assert!(
        records
            .iter()
            .all(|record| !record.contains(key) && !record.contains(&running.bearer)),
        "a dependency logged credential bytes"
    );
}

#[test]
fn ambiguous_malformed_and_oversized_upstream_heads_are_confirmed_before_a_502() {
    let cases = [
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nContent-Length: 1\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: +1\r\n\r\n",
        "HTTP/1.1 200 OK\r\nX-Test: value\r\n folded\r\n\r\n",
        "HTTP/1.1 200 OK\nContent-Length: 0\n\n",
        "HTTP/1.1 101 Switching Protocols\r\nConnection: upgrade\r\nUpgrade: websocket\r\n\r\n",
    ];
    for wire in cases
        .into_iter()
        .map(str::to_owned)
        .chain(std::iter::once(format!(
            "HTTP/1.1 200 OK\r\nX-Padding: {}\r\n\r\n",
            "z".repeat(513)
        )))
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        let upstream = thread::spawn(move || {
            let mut socket = accept(&listener);
            let _ = upstream_head(&mut socket);
            let _ = socket.write_all(wire.as_bytes());
        });
        let limits = Limits {
            header_bytes: 512,
            ..Limits::default()
        };
        let mut running = start(&[destination(&route)], Credential::None, limits);
        let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
        assert!(response.starts_with("HTTP/1.1 502"));
        assert!(!response.contains(&running.bearer));
        assert_eq!(running.journal.rows.lock().len(), 1);
        upstream.join().unwrap();
        running.proxy.stop_and_join().unwrap();
    }
}

#[test]
fn upstream_response_header_deadline_is_absolute_under_continuous_dripping() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let _ = upstream_head(&mut socket);
        if socket.write_all(b"HTTP/1.1 200 OK\r\nX-Drip: ").is_err() {
            return;
        }
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(35));
            if socket.write_all(b"a").is_err() {
                return;
            }
        }
    });
    let limits = Limits {
        io_timeout: Duration::from_millis(150),
        ..Limits::default()
    };
    let mut running = start(&[destination(&route)], Credential::None, limits);
    let began = Instant::now();
    let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
    assert!(response.starts_with("HTTP/1.1 502"));
    assert!(
        began.elapsed() < Duration::from_millis(700),
        "upstream drips reset an absolute phase deadline"
    );
    assert_eq!(running.journal.rows.lock().len(), 1);
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}

#[test]
fn malformed_chunk_metadata_closes_stream_without_appending_or_echoing_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let route = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
    let upstream = thread::spawn(move || {
        let mut socket = accept(&listener);
        let _ = upstream_head(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nsecret-invalid-chunk\r\n").unwrap();
    });
    let mut running = start(&[destination(&route)], Credential::None, Limits::default());
    let response = exchange(&running.proxy, &request(&route, &running.bearer, ""));
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("\r\n\r\n"));
    assert!(!response.contains("secret-invalid-chunk"));
    assert_eq!(running.journal.rows.lock()[0].3, "upstream-stream-failed");
    upstream.join().unwrap();
    running.proxy.stop_and_join().unwrap();
}
