//! Private direct HTTP/1.1 transport. Never logs request bytes or upstream errors.

use crate::proxy::{token, Limits};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn timeout() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "upstream phase deadline exceeded")
}
fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete upstream message")
}

// A phase deadline is checked on every underlying socket operation, including
// rustls handshake reads; bytes dripped below a socket timeout cannot reset it.
struct TimedSocket {
    socket: TcpStream,
    io_timeout: Duration,
    deadline: Option<Instant>,
}

impl TimedSocket {
    fn duration(&self) -> io::Result<Duration> {
        match self.deadline {
            Some(deadline) => deadline
                .checked_duration_since(Instant::now())
                .filter(|remaining| !remaining.is_zero())
                .map(|remaining| remaining.min(self.io_timeout))
                .ok_or_else(timeout),
            None => Ok(self.io_timeout),
        }
    }
}

impl Read for TimedSocket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.socket.set_read_timeout(Some(self.duration()?))?;
        let count = self.socket.read(bytes)?;
        self.duration()?;
        Ok(count)
    }
}
impl Write for TimedSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket.set_write_timeout(Some(self.duration()?))?;
        let count = self.socket.write(bytes)?;
        self.duration()?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

enum Wire {
    Plain(TimedSocket),
    Tls(Box<StreamOwned<ClientConnection, TimedSocket>>),
}

pub(crate) struct Connection {
    wire: Wire,
    io_timeout: Duration,
}

impl Connection {
    /// The caller has already vetted every address. No DNS or proxy discovery
    /// occurs here, and all connect attempts share one absolute deadline.
    pub(crate) fn connect(
        host: &str,
        tls: bool,
        addresses: &[SocketAddr],
        io_timeout: Duration,
        register: impl FnOnce(&TcpStream) -> io::Result<()>,
    ) -> io::Result<Self> {
        let deadline = Instant::now().checked_add(io_timeout).ok_or_else(timeout)?;
        let mut connected = None;
        for address in addresses {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|duration| !duration.is_zero())
                .ok_or_else(timeout)?;
            if let Ok(socket) = TcpStream::connect_timeout(address, remaining) {
                connected = Some(socket);
                break;
            }
        }
        let socket = connected.ok_or_else(|| {
            io::Error::new(io::ErrorKind::ConnectionRefused, "upstream connect failed")
        })?;
        socket.set_nodelay(true)?;
        register(&socket)?;
        let mut socket = TimedSocket {
            socket,
            io_timeout,
            deadline: Some(deadline),
        };
        let wire = if tls {
            let name = rustls::pki_types::ServerName::try_from(host.to_owned())
                .map_err(|_| invalid("invalid TLS server name"))?;
            let mut connection = ClientConnection::new(tls_config()?, name)
                .map_err(|_| invalid("TLS connection creation failed"))?;
            connection.set_buffer_limit(Some(16 * 1024));
            while connection.is_handshaking() {
                connection
                    .complete_io(&mut socket)
                    .map_err(|_| invalid("TLS handshake failed"))?;
            }
            Wire::Tls(Box::new(StreamOwned::new(connection, socket)))
        } else {
            Wire::Plain(socket)
        };
        let mut connection = Self { wire, io_timeout };
        connection.set_deadline(None);
        Ok(connection)
    }

    fn set_deadline(&mut self, deadline: Option<Instant>) {
        match &mut self.wire {
            Wire::Plain(socket) => socket.deadline = deadline,
            Wire::Tls(stream) => stream.sock.deadline = deadline,
        }
    }

    fn begin_phase(&mut self) -> io::Result<Instant> {
        let deadline = Instant::now()
            .checked_add(self.io_timeout)
            .ok_or_else(timeout)?;
        self.set_deadline(Some(deadline));
        Ok(deadline)
    }

    pub(crate) fn send(&mut self, head: &[u8], body: &[u8]) -> io::Result<()> {
        self.begin_phase()?;
        self.write_all(head)?;
        self.write_all(body)?;
        self.flush()?;
        self.set_deadline(None);
        Ok(())
    }

    pub(crate) fn response(mut self, method: &str, limits: &Limits) -> io::Result<Response> {
        let deadline = self.begin_phase()?;
        let mut reader = BufReader::with_capacity(8192, self);
        let mut total = 0usize;
        for _ in 0..8 {
            let line = bounded_line(&mut reader, limits.header_bytes.saturating_sub(total))?;
            total = total
                .checked_add(line.len() + 2)
                .ok_or_else(|| invalid("upstream headers exceed limit"))?;
            let status = status_line(&line)?;
            let mut headers: Vec<(String, String)> = Vec::new();
            let mut length = None;
            let mut chunked = false;
            loop {
                if Instant::now() >= deadline {
                    return Err(timeout());
                }
                let line = bounded_line(&mut reader, limits.header_bytes.saturating_sub(total))?;
                total = total
                    .checked_add(line.len() + 2)
                    .ok_or_else(|| invalid("upstream headers exceed limit"))?;
                if line.is_empty() {
                    break;
                }
                let (name, value) = field(&line)?;
                if [
                    "content-length",
                    "transfer-encoding",
                    "connection",
                    "content-encoding",
                ]
                .contains(&name.as_str())
                    && headers.iter().any(|(seen, _)| seen == &name)
                {
                    return Err(invalid("duplicate upstream framing field"));
                }
                if name == "content-length" {
                    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                        return Err(invalid("invalid upstream body length"));
                    }
                    length = Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| invalid("invalid upstream body length"))?,
                    );
                } else if name == "transfer-encoding" {
                    if !value.eq_ignore_ascii_case("chunked") {
                        return Err(invalid("unsupported upstream transfer encoding"));
                    }
                    chunked = true;
                } else if name == "connection" {
                    if value
                        .split(',')
                        .map(str::trim)
                        .any(|option| !token(option.as_bytes()))
                    {
                        return Err(invalid("invalid upstream connection option"));
                    }
                }
                headers.push((name, value));
            }
            if chunked && length.is_some() {
                return Err(invalid("ambiguous upstream framing"));
            }
            if status < 200 {
                if status == 101 {
                    return Err(invalid("upstream protocol upgrade is forbidden"));
                }
                continue;
            }
            if Instant::now() >= deadline {
                return Err(timeout());
            }
            reader.get_mut().set_deadline(None);
            let body = if method == "HEAD" || matches!(status, 204 | 205 | 304) {
                Body::Length(0)
            } else if chunked {
                Body::Chunked {
                    remaining: 0,
                    delimiter: false,
                    finished: false,
                }
            } else if let Some(length) = length {
                Body::Length(length)
            } else {
                Body::Close
            };
            return Ok(Response {
                status,
                headers,
                reader,
                body,
                metadata_limit: limits.header_bytes,
            });
        }
        Err(invalid("too many informational upstream responses"))
    }
}

impl Read for Connection {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match &mut self.wire {
            Wire::Plain(socket) => socket.read(bytes),
            Wire::Tls(stream) => stream.read(bytes),
        }
    }
}
impl Write for Connection {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match &mut self.wire {
            Wire::Plain(socket) => socket.write(bytes),
            Wire::Tls(stream) => stream.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match &mut self.wire {
            Wire::Plain(socket) => socket.flush(),
            Wire::Tls(stream) => stream.flush(),
        }
    }
}

fn tls_config() -> io::Result<Arc<ClientConfig>> {
    static CONFIG: LazyLock<Result<Arc<ClientConfig>, ()>> = LazyLock::new(|| {
        ClientConfig::builder_with_provider(rustls::crypto::ring::default_provider().into())
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
            .map(|builder| {
                Arc::new(
                    builder
                        .with_root_certificates(RootCertStore {
                            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
                        })
                        .with_no_client_auth(),
                )
            })
            .map_err(|_| ())
    });
    CONFIG
        .as_ref()
        .map(Arc::clone)
        .map_err(|_| invalid("TLS configuration failed"))
}

fn status_line(line: &[u8]) -> io::Result<u16> {
    if line.len() < 12
        || !matches!(&line[..8], b"HTTP/1.1" | b"HTTP/1.0")
        || line[8] != b' '
        || !line[9..12].iter().all(u8::is_ascii_digit)
        || (line.len() > 12 && line[12] != b' ')
        || line.iter().any(|byte| *byte < 32 || *byte == 127)
    {
        return Err(invalid("invalid upstream status line"));
    }
    let status = ((line[9] - b'0') as u16) * 100
        + ((line[10] - b'0') as u16) * 10
        + (line[11] - b'0') as u16;
    if !(100..600).contains(&status) {
        return Err(invalid("invalid upstream status"));
    }
    Ok(status)
}

fn field(line: &[u8]) -> io::Result<(String, String)> {
    let separator = line
        .iter()
        .position(|byte| *byte == b':')
        .ok_or_else(|| invalid("invalid upstream header"))?;
    if !token(&line[..separator])
        || line[separator + 1..]
            .iter()
            .any(|byte| *byte < 32 || *byte == 127)
    {
        return Err(invalid("invalid upstream header"));
    }
    let name = std::str::from_utf8(&line[..separator])
        .map_err(|_| invalid("invalid upstream header"))?
        .to_ascii_lowercase();
    let value = std::str::from_utf8(&line[separator + 1..])
        .map_err(|_| invalid("invalid upstream header"))?
        .trim_matches(' ')
        .to_owned();
    Ok((name, value))
}

/// Reads at most the configured bytes including CRLF, without consuming body
/// bytes already buffered after the line. Bare LF, folding and controls fail.
fn bounded_line(reader: &mut BufReader<Connection>, limit: usize) -> io::Result<Vec<u8>> {
    let mut line = Vec::with_capacity(limit.min(256));
    loop {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return Err(eof());
        }
        let newline = bytes.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(bytes.len(), |index| index + 1);
        if line.len().saturating_add(count) > limit {
            return Err(invalid("upstream metadata exceeds limit"));
        }
        line.extend_from_slice(&bytes[..count]);
        reader.consume(count);
        if newline.is_some() {
            if !line.ends_with(b"\r\n") {
                return Err(invalid("invalid upstream line ending"));
            }
            line.truncate(line.len() - 2);
            return Ok(line);
        }
    }
}

enum Body {
    Length(u64),
    Close,
    Chunked {
        remaining: u64,
        delimiter: bool,
        finished: bool,
    },
}

pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    reader: BufReader<Connection>,
    body: Body,
    metadata_limit: usize,
}

impl Read for Response {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        match &mut self.body {
            Body::Length(remaining) => {
                if *remaining == 0 {
                    return Ok(0);
                }
                let size = (*remaining).min(output.len() as u64) as usize;
                let count = self.reader.read(&mut output[..size])?;
                if count == 0 {
                    return Err(eof());
                }
                *remaining -= count as u64;
                Ok(count)
            }
            Body::Close => self.reader.read(output),
            Body::Chunked {
                remaining,
                delimiter,
                finished,
            } => {
                if *finished {
                    return Ok(0);
                }
                if *remaining == 0 {
                    self.reader.get_mut().begin_phase()?;
                    if *delimiter {
                        let mut ending = [0u8; 2];
                        self.reader.read_exact(&mut ending)?;
                        if ending != *b"\r\n" {
                            return Err(invalid("invalid upstream chunk delimiter"));
                        }
                    }
                    let line = bounded_line(&mut self.reader, self.metadata_limit.min(4096))?;
                    let size = chunk_length(&line)?;
                    if size == 0 {
                        let mut total = 0usize;
                        loop {
                            let line = bounded_line(
                                &mut self.reader,
                                self.metadata_limit.saturating_sub(total),
                            )?;
                            total = total
                                .checked_add(line.len() + 2)
                                .ok_or_else(|| invalid("upstream trailers exceed limit"))?;
                            if line.is_empty() {
                                break;
                            }
                            let (name, _) = field(&line)?;
                            if [
                                "content-length",
                                "transfer-encoding",
                                "host",
                                "authorization",
                                "x-api-key",
                                "connection",
                            ]
                            .contains(&name.as_str())
                            {
                                return Err(invalid("forbidden upstream trailer"));
                            }
                        }
                        *finished = true;
                        return Ok(0);
                    }
                    *remaining = size;
                    *delimiter = true;
                    self.reader.get_mut().set_deadline(None);
                }
                let size = (*remaining).min(output.len() as u64) as usize;
                let count = self.reader.read(&mut output[..size])?;
                if count == 0 {
                    return Err(eof());
                }
                *remaining -= count as u64;
                Ok(count)
            }
        }
    }
}

fn chunk_length(line: &[u8]) -> io::Result<u64> {
    let end = line
        .iter()
        .position(|byte| *byte == b';')
        .unwrap_or(line.len());
    let digits = &line[..end];
    if digits.is_empty()
        || line
            .iter()
            .any(|byte| !byte.is_ascii() || *byte < 32 || *byte == 127)
    {
        return Err(invalid("invalid upstream chunk length"));
    }
    let mut size = 0u64;
    for byte in digits {
        let digit = (*byte as char)
            .to_digit(16)
            .ok_or_else(|| invalid("invalid upstream chunk length"))?;
        size = size
            .checked_mul(16)
            .and_then(|size| size.checked_add(digit as u64))
            .ok_or_else(|| invalid("upstream chunk length overflow"))?;
    }
    Ok(size)
}
