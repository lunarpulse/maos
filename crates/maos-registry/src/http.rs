//! Admission-time blocking HTTP leaf; no kernel or asynchronous network stack.
//! The development registry speaks HTTP/1.1 with Content-Length, not TLS/chunks.
use maos_domain::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct RegistryHttp;
impl IoSubsystemPort for RegistryHttp {
    fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
        Err(IoError::InvalidUrl(
            "registry transport supports POST only".into(),
        ))
    }
    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        let exchange = self.http_post_exchange(url, body, headers, "")?;
        if !(200..300).contains(&exchange.status) {
            return Err(IoError::Transport(format!("HTTP {}", exchange.status)));
        }
        Ok(exchange.body)
    }
    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        let endpoint = url.strip_prefix("http://").ok_or_else(|| {
            IoError::InvalidUrl(
                "development registry requires http:// (TLS is not implemented)".into(),
            )
        })?;
        let (authority, path) = endpoint
            .split_once('/')
            .map_or((endpoint, "/".to_owned()), |(host, path)| {
                (host, format!("/{path}"))
            });
        if authority.is_empty()
            || authority.contains('@')
            || url.bytes().any(|b| b.is_ascii_control() || b == b' ')
            || path.contains('#')
        {
            return Err(IoError::InvalidUrl("invalid registry HTTP endpoint".into()));
        }
        let socket = if authority.starts_with('[') {
            if authority.contains("]:") {
                authority.to_owned()
            } else {
                format!("{authority}:80")
            }
        } else if authority.contains(':') {
            authority.to_owned()
        } else {
            format!("{authority}:80")
        };
        let timeout = Duration::from_secs(30);
        let address = socket
            .to_socket_addrs()
            .map_err(transport)?
            .next()
            .ok_or_else(|| IoError::InvalidUrl("endpoint has no address".into()))?;
        let mut stream = TcpStream::connect_timeout(&address, timeout).map_err(transport)?;
        stream.set_read_timeout(Some(timeout)).map_err(transport)?;
        stream.set_write_timeout(Some(timeout)).map_err(transport)?;
        write!(stream, "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {}\r\nConnection: close\r\n", body.len()).map_err(transport)?;
        for (name, value) in headers {
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || value.bytes().any(|b| b.is_ascii_control())
            {
                return Err(IoError::InvalidUrl("invalid HTTP header".into()));
            }
            write!(stream, "{name}: {value}\r\n").map_err(transport)?;
        }
        stream.write_all(b"\r\n").map_err(transport)?;
        stream.write_all(body).map_err(transport)?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        read_header_line(&mut reader, &mut line)?;
        let status = line
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| IoError::Decode("invalid HTTP status".into()))?;
        let mut length = None;
        let mut header = None;
        let mut header_bytes = line.len();
        loop {
            read_header_line(&mut reader, &mut line)?;
            header_bytes += line.len();
            if header_bytes > 65536 {
                return Err(IoError::Decode("HTTP headers exceed 64 KiB".into()));
            }
            if line == "\r\n" {
                break;
            }
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| IoError::Decode("invalid HTTP header".into()))?;
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| IoError::Decode("invalid Content-Length".into()))?;
                if length.replace(parsed).is_some() {
                    return Err(IoError::Decode("duplicate Content-Length".into()));
                }
            }
            if name.eq_ignore_ascii_case("transfer-encoding") {
                return Err(IoError::Decode(
                    "registry requires Content-Length, not transfer encoding".into(),
                ));
            }
            if name.eq_ignore_ascii_case(response_header) {
                header = Some(value.to_owned());
            }
        }
        let length = length.ok_or_else(|| IoError::Decode("missing Content-Length".into()))?;
        if length > 64 * 1024 * 1024 {
            return Err(IoError::Decode("registry response exceeds 64 MiB".into()));
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).map_err(transport)?;
        Ok(HttpExchange {
            status,
            body,
            header,
        })
    }
}
fn transport(error: std::io::Error) -> IoError {
    IoError::Transport(error.to_string())
}
fn read_header_line(reader: &mut impl BufRead, line: &mut String) -> Result<(), IoError> {
    line.clear();
    let size = reader.take(65537).read_line(line).map_err(transport)?;
    if size == 0 || size > 65536 || !line.ends_with("\r\n") {
        return Err(IoError::Decode("truncated or oversized HTTP header".into()));
    }
    Ok(())
}
