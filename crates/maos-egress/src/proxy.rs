//! Per-spawn HTTP/1.1 reverse proxy. This is a transport, not Worker isolation.

use crate::{
    bearer::BearerTable,
    upstream::{Connection, Response},
};
use maos_domain::host_grant::EgressDestination;
use parking_lot::{Condvar, Mutex};
use std::{
    collections::HashMap,
    fs::{self, DirBuilder},
    io::{self, Read, Write},
    net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpStream, ToSocketAddrs},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// The daemon's vendor credential. Deliberately has no Debug or Display.
pub enum Credential {
    Bearer(String),
    ApiKey(String),
    None,
}

pub struct Limits {
    pub header_bytes: usize,
    pub body_bytes: usize,
    /// Absolute time from acceptance to the complete request head, not per byte.
    pub header_timeout: Duration,
    /// Absolute time from acceptance to the complete request body. Per-read
    /// `io_timeout` alone lets a client drip a bounded body forever.
    pub body_timeout: Duration,
    pub io_timeout: Duration,
    pub max_connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            header_bytes: 16 * 1024,
            body_bytes: 8 * 1024 * 1024,
            header_timeout: Duration::from_secs(5),
            body_timeout: Duration::from_secs(300),
            io_timeout: Duration::from_secs(30),
            max_connections: 16,
        }
    }
}

pub struct ProxyConfig {
    pub spirit_pid: u32,
    pub spirit_id: String,
    pub table: Arc<BearerTable>,
    pub credential: Credential,
    pub sink: Arc<dyn RefusalSink>,
    pub limits: Limits,
}

pub struct Refusal<'a> {
    pub spirit_pid: u32,
    pub spirit_id: &'a str,
    pub destination: Option<&'a EgressDestination>,
    pub reason: &'static str,
}

/// The composition root must supply a durable, acknowledged write here.
pub trait RefusalSink: Send + Sync {
    fn confirm(&self, refusal: &Refusal<'_>) -> io::Result<()>;
}

struct Active {
    sockets: HashMap<u64, UnixStream>,
    upstream: HashMap<u64, TcpStream>,
    failure: Option<&'static str>,
}

struct Shared {
    stop: AtomicBool,
    active: Mutex<Active>,
    wake: Condvar,
}

impl Shared {
    fn fail(&self, reason: &'static str) {
        let mut active = self.active.lock();
        active.failure.get_or_insert(reason);
    }

    fn register_upstream(&self, id: u64, socket: &TcpStream) -> io::Result<()> {
        let registered = socket.try_clone()?;
        let mut active = self.active.lock();
        if self.stop.load(Ordering::Acquire) {
            let _ = socket.shutdown(Shutdown::Both);
            return Err(io::Error::new(io::ErrorKind::Interrupted, "proxy stopped"));
        }
        active.upstream.insert(id, registered);
        Ok(())
    }

    fn close_connections(&self) {
        let active = self.active.lock();
        for socket in active.sockets.values() {
            let _ = socket.shutdown(Shutdown::Both);
        }
        for socket in active.upstream.values() {
            let _ = socket.shutdown(Shutdown::Both);
        }
        self.wake.notify_all();
    }
}

pub struct ProxyGuard {
    socket_path: PathBuf,
    directory: PathBuf,
    socket_identity: (u64, u64),
    config: Arc<ProxyConfig>,
    shared: Arc<Shared>,
    listener: Option<JoinHandle<()>>,
    cleaned: bool,
}

impl ProxyGuard {
    pub fn bind(run_dir: &Path, config: ProxyConfig) -> io::Result<Self> {
        if config.limits.header_bytes < 4
            || config.limits.max_connections == 0
            || config.limits.header_timeout.is_zero()
            || config.limits.io_timeout.is_zero()
            || Instant::now()
                .checked_add(config.limits.header_timeout)
                .is_none()
            || Instant::now()
                .checked_add(config.limits.io_timeout)
                .is_none()
        {
            return Err(invalid("invalid proxy limits"));
        }
        // `run_dir` is one root's private directory. Never delete an existing
        // socket: a colliding pid within that root is a bind failure.
        let metadata = fs::symlink_metadata(run_dir)?;
        if !metadata.file_type().is_dir() || metadata.permissions().mode() & 0o777 != 0o700 {
            return Err(invalid("egress directory must be a private directory"));
        }
        let directory = run_dir.join(config.spirit_pid.to_string());
        DirBuilder::new().mode(0o700).create(&directory)?;
        let socket_path = directory.join("egress.sock");
        let setup = (|| {
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
            let listener = UnixListener::bind(&socket_path)?;
            fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o666))?;
            listener.set_nonblocking(true)?;
            let metadata = fs::symlink_metadata(&socket_path)?;
            Ok::<_, io::Error>((listener, (metadata.dev(), metadata.ino())))
        })();
        let (listener, socket_identity) = match setup {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(&socket_path);
                let _ = fs::remove_dir(&directory);
                return Err(error);
            }
        };
        let config = Arc::new(config);
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            active: Mutex::new(Active {
                sockets: HashMap::new(),
                upstream: HashMap::new(),
                failure: None,
            }),
            wake: Condvar::new(),
        });
        let thread_config = Arc::clone(&config);
        let thread_shared = Arc::clone(&shared);
        let handle = match thread::Builder::new()
            .name("maos-egress-listener".into())
            .spawn(move || {
                serve(listener, thread_config, thread_shared);
            }) {
            Ok(handle) => handle,
            Err(error) => {
                let _ = fs::remove_file(&socket_path);
                let _ = fs::remove_dir(&directory);
                return Err(error);
            }
        };
        Ok(Self {
            socket_path,
            directory,
            socket_identity,
            config,
            shared,
            listener: Some(handle),
            cleaned: false,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Revokes first, interrupts clients, joins every worker, then removes custody files.
    /// Upstream calls retain their configured connect/read/write timeouts.
    pub fn stop_and_join(&mut self) -> io::Result<()> {
        if self.listener.is_some() {
            // A stopped guard must not revoke a later spawn that reuses its pid.
            self.config.table.revoke(self.config.spirit_pid);
        }
        self.shared.stop.store(true, Ordering::Release);
        self.shared.close_connections();
        if let Some(handle) = self.listener.take() {
            if handle.join().is_err() {
                self.shared.fail("proxy listener panicked");
            }
        }
        if !self.cleaned {
            // Do not unlink a socket somebody replaced while we were running.
            match fs::symlink_metadata(&self.socket_path) {
                Ok(metadata) if (metadata.dev(), metadata.ino()) == self.socket_identity => {
                    if fs::remove_file(&self.socket_path).is_err() {
                        self.shared.fail("proxy socket cleanup failed");
                    }
                }
                Ok(_) => self.shared.fail("proxy socket was replaced"),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => self.shared.fail("proxy socket cleanup failed"),
            }
            match fs::remove_dir(&self.directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => self.shared.fail("proxy directory cleanup failed"),
            }
            self.cleaned = true;
        }
        match self.shared.active.lock().failure {
            Some(reason) => Err(io::Error::other(reason)),
            None => Ok(()),
        }
    }
}

impl Drop for ProxyGuard {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
    }
}

struct ConnectionRegistration {
    shared: Arc<Shared>,
    id: u64,
}
impl Drop for ConnectionRegistration {
    fn drop(&mut self) {
        let mut active = self.shared.active.lock();
        active.sockets.remove(&self.id);
        active.upstream.remove(&self.id);
        self.shared.wake.notify_all();
    }
}

fn serve(listener: UnixListener, config: Arc<ProxyConfig>, shared: Arc<Shared>) {
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut next_id = 0u64;
    while !shared.stop.load(Ordering::Acquire) {
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                if workers.swap_remove(index).join().is_err() {
                    shared.fail("proxy worker panicked");
                }
            } else {
                index += 1;
            }
        }
        // Backpressure at the listener, rather than an unbounded set of denial threads.
        // Unaccepted sockets live only in the OS's bounded listen backlog.
        if shared.active.lock().sockets.len() >= config.limits.max_connections {
            let mut active = shared.active.lock();
            if active.sockets.len() >= config.limits.max_connections
                && !shared.stop.load(Ordering::Acquire)
            {
                shared.wake.wait_for(&mut active, Duration::from_millis(10));
            }
            continue;
        }
        match listener.accept() {
            Ok((mut socket, _)) => {
                let accepted = Instant::now();
                if shared.stop.load(Ordering::Acquire) {
                    break;
                }
                let registration_socket = match socket.try_clone() {
                    Ok(value) => value,
                    Err(_) => {
                        shared.fail("proxy connection registration failed");
                        continue;
                    }
                };
                let id = next_id;
                next_id = next_id.wrapping_add(1);
                {
                    let mut active = shared.active.lock();
                    if shared.stop.load(Ordering::Acquire) {
                        break;
                    }
                    active.sockets.insert(id, registration_socket);
                }
                let registration = ConnectionRegistration {
                    shared: Arc::clone(&shared),
                    id,
                };
                let worker_config = Arc::clone(&config);
                let worker_shared = Arc::clone(&shared);
                match thread::Builder::new()
                    .name("maos-egress-request".into())
                    .spawn(move || {
                        let _registration = registration;
                        handle_connection(
                            &mut socket,
                            accepted,
                            &worker_config,
                            &worker_shared,
                            id,
                        );
                        let _ = socket.shutdown(Shutdown::Both);
                    }) {
                    Ok(handle) => workers.push(handle),
                    Err(_) => shared.fail("proxy worker start failed"),
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let mut active = shared.active.lock();
                if !shared.stop.load(Ordering::Acquire) {
                    shared.wake.wait_for(&mut active, Duration::from_millis(10));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                shared.fail("proxy listener failed");
                break;
            }
        }
    }
    drop(listener);
    shared.stop.store(true, Ordering::Release);
    shared.close_connections();
    for worker in workers {
        if worker.join().is_err() {
            shared.fail("proxy worker panicked");
        }
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

struct Head<'a> {
    method: &'a str,
    destination: EgressDestination,
    path: &'a str,
    headers: Vec<(&'a str, &'a str)>,
    bearer: Option<&'a str>,
    body_length: usize,
    connection_tokens: Vec<&'a str>,
}

pub(crate) fn token(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(b))
}

fn parse_head<'a>(bytes: &'a [u8], limits: &Limits) -> io::Result<Head<'a>> {
    if bytes.len() > limits.header_bytes {
        return Err(invalid("request headers exceed limit"));
    }
    if !bytes.ends_with(b"\r\n\r\n") {
        return Err(invalid("incomplete request headers"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("non-UTF8 request headers"))?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let line = lines
        .next()
        .ok_or_else(|| invalid("missing request line"))?;
    let mut words = line.split(' ');
    let method = words.next().ok_or_else(|| invalid("missing method"))?;
    let target = words
        .next()
        .ok_or_else(|| invalid("missing request target"))?;
    if words.next() != Some("HTTP/1.1") || words.next().is_some() {
        return Err(invalid("invalid request line"));
    }
    if !matches!(
        method,
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    ) {
        return Err(invalid("unsupported request method"));
    }
    let routed = target
        .strip_prefix('/')
        .ok_or_else(|| invalid("invalid reverse route"))?;
    let (authority, suffix) = routed
        .split_once('/')
        .ok_or_else(|| invalid("missing reverse route path"))?;
    let destination = EgressDestination::try_from(authority.to_owned())
        .map_err(|_| invalid("invalid destination"))?;
    let path = &target[authority.len() + 1..];
    validate_path(path)?;
    if suffix.starts_with('/') {
        return Err(invalid("ambiguous reverse route path"));
    }
    let mut headers: Vec<(&str, &str)> = Vec::new();
    let mut host = false;
    let mut authorization = None;
    let mut api_key = None;
    let mut body_length = 0;
    let mut connection_tokens = Vec::new();
    for line in lines {
        let (name, raw_value) = line
            .split_once(':')
            .ok_or_else(|| invalid("invalid header field"))?;
        if !token(name.as_bytes())
            || raw_value
                .bytes()
                .any(|b| b < 32 || b == 127 || !b.is_ascii())
        {
            return Err(invalid("invalid header field"));
        }
        if headers
            .iter()
            .any(|(seen, _)| name.eq_ignore_ascii_case(seen))
        {
            return Err(invalid("duplicate header field"));
        }
        let value = raw_value.trim_matches(' ');
        if name.eq_ignore_ascii_case("host") {
            validate_host(value)?;
            host = true;
        } else if name.eq_ignore_ascii_case("authorization") {
            // RFC auth schemes are case-insensitive; the token itself is opaque.
            let (scheme, value) = value
                .split_once(' ')
                .ok_or_else(|| invalid("invalid scoped authorization"))?;
            if !scheme.eq_ignore_ascii_case("bearer")
                || value.is_empty()
                || value.bytes().any(|b| b.is_ascii_whitespace())
            {
                return Err(invalid("invalid scoped authorization"));
            }
            authorization = Some(value);
        } else if name.eq_ignore_ascii_case("x-api-key") {
            if value.is_empty() || value.bytes().any(|b| b.is_ascii_whitespace()) {
                return Err(invalid("invalid scoped authorization"));
            }
            api_key = Some(value);
        } else if name.eq_ignore_ascii_case("content-length") {
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid("invalid request body length"));
            }
            body_length = value
                .parse()
                .map_err(|_| invalid("invalid request body length"))?;
            if body_length > limits.body_bytes {
                return Err(invalid("request body exceeds limit"));
            }
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            // The local protocol uses only bounded, length-delimited request bodies.
            return Err(invalid("request transfer encoding is unsupported"));
        } else if name.eq_ignore_ascii_case("expect")
            || name.eq_ignore_ascii_case("upgrade")
            || name.eq_ignore_ascii_case("trailer")
            || name.eq_ignore_ascii_case("proxy-authorization")
        {
            return Err(invalid("unsupported request header"));
        } else if name.eq_ignore_ascii_case("connection") {
            for value in value.split(',').map(str::trim) {
                if !token(value.as_bytes()) || matches_critical(value) {
                    return Err(invalid("invalid connection option"));
                }
                connection_tokens.push(value);
            }
        }
        headers.push((name, value));
    }
    if !host {
        return Err(invalid("missing Host header"));
    }
    if authorization.is_some() && api_key.is_some() {
        return Err(invalid("ambiguous scoped authorization"));
    }
    Ok(Head {
        method,
        destination,
        path,
        headers,
        bearer: authorization.or(api_key),
        body_length,
        connection_tokens,
    })
}

fn matches_critical(name: &str) -> bool {
    [
        "host",
        "authorization",
        "x-api-key",
        "content-length",
        "transfer-encoding",
    ]
    .iter()
    .any(|field| name.eq_ignore_ascii_case(field))
}

fn validate_host(host: &str) -> io::Result<()> {
    let canonical = if host.contains(':') {
        host.to_owned()
    } else {
        format!("{host}:443")
    };
    EgressDestination::try_from(canonical).map_err(|_| invalid("invalid Host header"))?;
    Ok(())
}

fn validate_path(path: &str) -> io::Result<()> {
    if !path.starts_with('/')
        || path
            .bytes()
            .any(|b| !(33..=126).contains(&b) || matches!(b, b'\\' | b'#'))
    {
        return Err(invalid("invalid request path"));
    }
    let pathname = path.split('?').next().unwrap_or(path);
    if pathname
        .split('/')
        .any(|segment| segment == "." || segment == "..")
        || pathname.contains("//")
    {
        return Err(invalid("route traversal is forbidden"));
    }
    let bytes = path.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes
                .get(index + 1..index + 3)
                .ok_or_else(|| invalid("invalid path escape"))?;
            let high = (hex[0] as char)
                .to_digit(16)
                .ok_or_else(|| invalid("invalid path escape"))?;
            let low = (hex[1] as char)
                .to_digit(16)
                .ok_or_else(|| invalid("invalid path escape"))?;
            let decoded = (high * 16 + low) as u8;
            if decoded < 32 || decoded == 127 || matches!(decoded, b'.' | b'/' | b'\\' | b'%') {
                return Err(invalid("ambiguous path escape"));
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn read_head(
    socket: &mut UnixStream,
    accepted: Instant,
    limits: &Limits,
) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let deadline = accepted
        .checked_add(limits.header_timeout)
        .ok_or_else(|| invalid("invalid header deadline"))?;
    let mut bytes = Vec::with_capacity(limits.header_bytes.min(4096));
    let mut buffer = [0u8; 1024];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::TimedOut, "request header deadline exceeded")
            })?;
        socket.set_read_timeout(Some(remaining.min(limits.io_timeout)))?;
        let count = socket.read(&mut buffer)?;
        if count == 0 {
            return Err(invalid("incomplete request headers"));
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "request header deadline exceeded",
            ));
        }
        let scan_from = bytes.len().saturating_sub(3);
        // Accumulate at most header_bytes: copy only the bytes still within
        // the limit. A terminator completed inside the allowed region is
        // still found below; one that would end past the limit is not, and
        // the trailing size check rejects the request.
        let allowed = limits.header_bytes.saturating_sub(bytes.len()).min(count);
        bytes.extend_from_slice(&buffer[..allowed]);
        if let Some(relative) = bytes[scan_from..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            let end = scan_from + relative + 4;
            if end > limits.header_bytes {
                return Err(invalid("request headers exceed limit"));
            }
            let prefix = bytes.split_off(end);
            return Ok((bytes, prefix));
        }
        if bytes.len() >= limits.header_bytes {
            return Err(invalid("request headers exceed limit"));
        }
    }
}

fn refuse(
    socket: &mut UnixStream,
    config: &ProxyConfig,
    shared: &Shared,
    destination: Option<&EgressDestination>,
    reason: &'static str,
    status: u16,
) {
    if shared.stop.load(Ordering::Acquire) {
        return;
    }
    if config
        .sink
        .confirm(&Refusal {
            spirit_pid: config.spirit_pid,
            spirit_id: &config.spirit_id,
            destination,
            reason,
        })
        .is_err()
    {
        // Never retain/format an injected sink's error: it may contain secrets.
        shared.fail("egress refusal journal confirmation failed");
        return;
    }
    if shared.stop.load(Ordering::Acquire) {
        return;
    }
    let _ = socket.set_write_timeout(Some(config.limits.io_timeout));
    // Empty error bodies are also correct for HEAD, and cannot echo an upstream
    // error, a URL, a Worker token or the vendor credential.
    let head = format!(
        "HTTP/1.1 {status} Egress Refused\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = socket.write_all(head.as_bytes());
}

fn handle_connection(
    socket: &mut UnixStream,
    accepted: Instant,
    config: &ProxyConfig,
    shared: &Shared,
    id: u64,
) {
    let (bytes, prefix) = match read_head(socket, accepted, &config.limits) {
        Ok(value) => value,
        Err(_) => {
            refuse(socket, config, shared, None, "invalid-request-head", 403);
            return;
        }
    };
    let head = match parse_head(&bytes, &config.limits) {
        Ok(head) => head,
        Err(_) => {
            refuse(socket, config, shared, None, "invalid-request-head", 403);
            return;
        }
    };
    let destination = Some(&head.destination);
    if !head.bearer.is_some_and(|bearer| {
        config.table.verify(
            config.spirit_pid,
            &config.spirit_id,
            &head.destination,
            bearer,
        )
    }) {
        refuse(
            socket,
            config,
            shared,
            destination,
            "unauthorized-destination",
            403,
        );
        return;
    }
    if prefix.len() > head.body_length {
        refuse(
            socket,
            config,
            shared,
            destination,
            "ambiguous-request-body",
            403,
        );
        return;
    }
    if socket
        .set_read_timeout(Some(config.limits.io_timeout))
        .is_err()
        || socket
            .set_write_timeout(Some(config.limits.io_timeout))
            .is_err()
    {
        return;
    }
    // Buffer only the bounded request body, never the upstream response.
    // Auth precedes DNS, connecting and all other upstream I/O. The body
    // phase has one absolute deadline from acceptance; the per-read
    // io_timeout alone would let a client drip a bounded body forever and
    // occupy a connection slot for the life of the proxy.
    let body_deadline = match accepted.checked_add(config.limits.body_timeout) {
        Some(deadline) => deadline,
        None => {
            refuse(
                socket,
                config,
                shared,
                destination,
                "invalid-request-body-deadline",
                403,
            );
            return;
        }
    };
    let mut body = prefix;
    let mut buffer = [0u8; 8192];
    while body.len() < head.body_length {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let phase = body_deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero());
        match phase {
            None => {
                refuse(
                    socket,
                    config,
                    shared,
                    destination,
                    "request-body-deadline-exceeded",
                    403,
                );
                return;
            }
            Some(phase) => {
                if socket
                    .set_read_timeout(Some(phase.min(config.limits.io_timeout)))
                    .is_err()
                {
                    return;
                }
            }
        }
        let remaining = (head.body_length - body.len()).min(buffer.len());
        match socket.read(&mut buffer[..remaining]) {
            Ok(0) | Err(_) => {
                refuse(
                    socket,
                    config,
                    shared,
                    destination,
                    "incomplete-request-body",
                    403,
                );
                return;
            }
            Ok(count) => body.extend_from_slice(&buffer[..count]),
        }
    }
    if shared.stop.load(Ordering::Acquire) {
        return;
    }
    let (scheme, addresses) = match vetted_addresses(&head.destination) {
        Ok(value) => value,
        Err(_) => {
            refuse(
                socket,
                config,
                shared,
                destination,
                "unsafe-upstream-resolution",
                403,
            );
            return;
        }
    };
    if shared.stop.load(Ordering::Acquire) {
        return;
    }
    let authority = head.destination.to_string();
    let outgoing = match request_head(&head, &authority, &config.credential, &config.limits) {
        Ok(value) => value,
        Err(_) => {
            refuse(
                socket,
                config,
                shared,
                destination,
                "invalid-proxy-credential",
                502,
            );
            return;
        }
    };
    let response = Connection::connect(
        &head.destination.host,
        scheme == "https",
        &addresses,
        config.limits.io_timeout,
        |socket| shared.register_upstream(id, socket),
    )
    .and_then(|mut connection| {
        connection.send(&outgoing, &body)?;
        connection.response(head.method, &config.limits)
    });
    let mut response = match response {
        Ok(response) => response,
        Err(error) => {
            let reason = if error.kind() == io::ErrorKind::InvalidData {
                "invalid-upstream-head"
            } else {
                "upstream-transport-failed"
            };
            refuse(socket, config, shared, destination, reason, 502);
            return;
        }
    };
    let status = response.status;
    if !(200..300).contains(&status) {
        let client_status = if (400..600).contains(&status) {
            status
        } else {
            403
        };
        let reason = if (300..400).contains(&status) {
            "upstream-redirect-forbidden"
        } else {
            "upstream-request-refused"
        };
        refuse(socket, config, shared, destination, reason, client_status);
        return;
    }
    let response_head = match success_head(&response, &config.limits) {
        Ok(value) => value,
        Err(_) => {
            refuse(
                socket,
                config,
                shared,
                destination,
                "invalid-upstream-head",
                502,
            );
            return;
        }
    };
    if shared.stop.load(Ordering::Acquire) || socket.write_all(&response_head).is_err() {
        return;
    }
    if head.method == "HEAD" || status == 204 || status == 205 {
        return;
    }
    loop {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        match response.read(&mut buffer) {
            Ok(0) => return,
            Ok(count) => {
                if shared.stop.load(Ordering::Acquire)
                    || socket.write_all(&buffer[..count]).is_err()
                {
                    return;
                }
            }
            Err(_) => {
                if shared.stop.load(Ordering::Acquire) {
                    return;
                }
                // Success headers are already on the wire; only close, never append a
                // fake HTTP error to SSE. Still confirm the refusal before returning.
                if config
                    .sink
                    .confirm(&Refusal {
                        spirit_pid: config.spirit_pid,
                        spirit_id: &config.spirit_id,
                        destination,
                        reason: "upstream-stream-failed",
                    })
                    .is_err()
                {
                    shared.fail("egress refusal journal confirmation failed");
                }
                return;
            }
        }
    }
}

fn hop_header(name: &str) -> bool {
    [
        "connection",
        "keep-alive",
        "proxy-connection",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ]
    .iter()
    .any(|field| name.eq_ignore_ascii_case(field))
}

fn success_head(response: &Response, limits: &Limits) -> io::Result<Vec<u8>> {
    let connection_options: Vec<_> = response
        .headers
        .iter()
        .filter(|(name, _)| name == "connection")
        .flat_map(|(_, value)| value.split(',').map(str::trim))
        .collect();
    let mut bytes =
        format!("HTTP/1.1 {} OK\r\nConnection: close\r\n", response.status).into_bytes();
    // Decode chunk framing only. Content-Encoding remains intact for the client.
    for (name, value) in &response.headers {
        if hop_header(name)
            || matches_critical(name)
            || connection_options
                .iter()
                .any(|option| name.eq_ignore_ascii_case(option))
        {
            continue;
        }
        append_parts(
            &mut bytes,
            &[name.as_bytes(), b": ", value.as_bytes(), b"\r\n"],
            limits.header_bytes,
        )?;
    }
    append_parts(&mut bytes, &[b"\r\n"], limits.header_bytes)?;
    Ok(bytes)
}

fn request_head(
    head: &Head<'_>,
    authority: &str,
    credential: &Credential,
    limits: &Limits,
) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limits.header_bytes.min(4096));
    append_parts(
        &mut bytes,
        &[
            head.method.as_bytes(),
            b" ",
            head.path.as_bytes(),
            b" HTTP/1.1\r\nHost: ",
            authority.as_bytes(),
            b"\r\nConnection: close\r\nAccept-Encoding: identity\r\nContent-Length: ",
            head.body_length.to_string().as_bytes(),
            b"\r\n",
        ],
        limits.header_bytes,
    )?;
    for (name, value) in &head.headers {
        if !matches_critical(name)
            && !hop_header(name)
            && !name.eq_ignore_ascii_case("accept-encoding")
            && !head
                .connection_tokens
                .iter()
                .any(|option| name.eq_ignore_ascii_case(option))
        {
            append_parts(
                &mut bytes,
                &[name.as_bytes(), b": ", value.as_bytes(), b"\r\n"],
                limits.header_bytes,
            )?;
        }
    }
    match credential {
        Credential::Bearer(key) | Credential::ApiKey(key) => {
            if key
                .bytes()
                .any(|byte| byte < 32 || byte == 127 || !byte.is_ascii())
            {
                return Err(invalid("invalid vendor credential"));
            }
            let prefix: &[u8] = if matches!(credential, Credential::Bearer(_)) {
                b"Authorization: Bearer "
            } else {
                b"x-api-key: "
            };
            append_parts(
                &mut bytes,
                &[prefix, key.as_bytes(), b"\r\n"],
                limits.header_bytes,
            )?;
        }
        Credential::None => {}
    }
    append_parts(&mut bytes, &[b"\r\n"], limits.header_bytes)?;
    Ok(bytes)
}

fn append_parts(bytes: &mut Vec<u8>, parts: &[&[u8]], limit: usize) -> io::Result<()> {
    let mut length = bytes.len();
    for part in parts {
        length = length
            .checked_add(part.len())
            .filter(|length| *length <= limit)
            .ok_or_else(|| invalid("proxy headers exceed limit"))?;
    }
    for part in parts {
        bytes.extend_from_slice(part);
    }
    Ok(())
}

fn vetted_addresses(
    destination: &EgressDestination,
) -> io::Result<(&'static str, Vec<SocketAddr>)> {
    if let Ok(ip) = destination.host.parse::<Ipv4Addr>() {
        if ip.is_loopback() {
            return Ok((
                "http",
                vec![SocketAddr::new(IpAddr::V4(ip), destination.port)],
            ));
        }
        if !public_ip(IpAddr::V4(ip)) {
            return Err(invalid("unsafe upstream address"));
        }
        return Ok((
            "https",
            vec![SocketAddr::new(IpAddr::V4(ip), destination.port)],
        ));
    }
    // Resolve once and pass the vetted addresses directly to the TCP connector.
    // TLS still verifies the original host; mixed public/private answers fail closed.
    // getaddrinfo has no deadline of its own, so the call runs on a helper
    // thread under an absolute timeout; a stalled resolver refuses the
    // request instead of pinning this worker and stop_and_join. A timed-out
    // helper thread is abandoned (bounded by the distinct granted hostnames).
    let addresses = resolve_bounded(&destination.host, destination.port)?;
    if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
        return Err(invalid("unsafe upstream resolution"));
    }
    Ok(("https", addresses))
}

const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

fn resolve_bounded(host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let host = host.to_owned();
    thread::spawn(move || {
        let resolved: Vec<_> = (host.as_str(), port)
            .to_socket_addrs()
            .map_or_else(|_| Vec::new(), |addresses| addresses.collect());
        let _ = sender.send(resolved);
    });
    receiver
        .recv_timeout(RESOLVE_TIMEOUT)
        .map_err(|_| invalid("dns resolution deadline exceeded"))
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_unspecified()
                && a != 0
                && a < 240
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && (b == 0 || b == 2))
                && !(a == 198 && (b == 18 || b == 19 || b == 51))
                && !(a == 203 && b == 0)
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(mapped));
            }
            let segments = ip.segments();
            // Only global unicast; refuse unique-local, link/site-local, multicast,
            // IPv4 translations, documentation and other special-use blocks.
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && segments[1] < 0x0200)
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && !(segments[0] == 0x2002)
        }
    }
}
