//! Admission-time blocking HTTP/TLS leaf; no kernel or asynchronous network stack.
//! Production authenticates both WebPKI transport and an operator-pinned registry.
use maos_domain::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct RegistryHttp {
    uri: String,
    url: url::Url,
    security: crate::security::RegistrySecurityConfig,
    #[cfg(feature = "tls")]
    tls: Option<std::sync::Arc<rustls::ClientConfig>>,
}
impl RegistryHttp {
    pub fn new(
        uri: &str,
        security: crate::security::RegistrySecurityConfig,
    ) -> Result<Self, IoError> {
        let url = crate::security::canonical_uri(uri).map_err(IoError::InvalidUrl)?;
        if url.scheme() == "http"
            && (!security.development_http || !crate::security::literal_loopback(&url))
        {
            return Err(IoError::InvalidUrl(
                "HTTP requires explicit development opt-in and literal loopback".into(),
            ));
        }
        if security.uri.as_deref().is_some_and(|bound| bound != uri)
            || (security.signing_pubkey.is_some() && security.uri.as_deref() != Some(uri))
            || (url.scheme() == "https" && security.signing_pubkey.is_none())
        {
            return Err(crate::security::authentication(
                "configured signing key and exact URI binding required",
            ));
        }
        #[cfg(not(feature = "tls"))]
        if url.scheme() == "https" {
            return Err(IoError::InvalidUrl(
                "registry TLS feature is disabled".into(),
            ));
        }
        #[cfg(feature = "tls")]
        let tls = (url.scheme() == "https")
            .then(|| crate::security::client_tls(security.ca_certificate.as_deref()))
            .transpose()
            .map_err(crate::security::authentication)?;
        Ok(Self {
            uri: uri.into(),
            url,
            security,
            #[cfg(feature = "tls")]
            tls,
        })
    }
}
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
        if url != self.uri {
            return Err(IoError::InvalidUrl("registry endpoint changed".into()));
        }
        let authority = &self.url[url::Position::BeforeHost..url::Position::AfterPort];
        let host = self
            .url
            .host_str()
            .ok_or_else(|| IoError::InvalidUrl("missing host".into()))?
            .trim_matches(['[', ']']);
        let port = self
            .url
            .port_or_known_default()
            .ok_or_else(|| IoError::InvalidUrl("missing port".into()))?;
        let path = self.url.path();
        let timeout = Duration::from_secs(30);
        let addresses = (host, port).to_socket_addrs().map_err(transport)?;
        let stream = connect_addresses(addresses, timeout).map_err(transport)?;
        stream.set_read_timeout(Some(timeout)).map_err(transport)?;
        stream.set_write_timeout(Some(timeout)).map_err(transport)?;
        #[cfg(feature = "tls")]
        let mut stream = if let Some(config) = &self.tls {
            let name = rustls::pki_types::ServerName::try_from(host.to_owned())
                .map_err(crate::security::authentication)?;
            let connection = rustls::ClientConnection::new(config.clone(), name)
                .map_err(crate::security::authentication)?;
            HttpStream::Tls(Box::new(rustls::StreamOwned::new(connection, stream)))
        } else {
            HttpStream::Plain(stream)
        };
        #[cfg(not(feature = "tls"))]
        let mut stream = HttpStream::Plain(stream);
        let nonce = crate::security::nonce()?;
        write!(stream, "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {}\r\nConnection: close\r\n", body.len()).map_err(transport)?;
        write!(
            stream,
            "{}: {}\r\n",
            crate::security::NONCE_HEADER,
            hex::encode(nonce)
        )
        .map_err(transport)?;
        if self.url.scheme() == "http" {
            write!(stream, "{}: true\r\n", crate::security::DEVELOPMENT_HEADER)
                .map_err(transport)?;
        }
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
        stream.flush().map_err(transport)?;
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
        let mut signed_response = None;
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
            if name.eq_ignore_ascii_case(crate::security::RESPONSE_HEADER)
                && signed_response.replace(value.to_owned()).is_some()
            {
                return Err(crate::security::authentication("duplicate signed envelope"));
            }
        }
        let length = length.ok_or_else(|| IoError::Decode("missing Content-Length".into()))?;
        if length > 64 * 1024 * 1024 {
            return Err(IoError::Decode("registry response exceeds 64 MiB".into()));
        }
        let mut response_body = vec![0; length];
        reader.read_exact(&mut response_body).map_err(transport)?;
        if let Some(key) = &self.security.signing_pubkey {
            crate::security::verify_response(
                url,
                nonce,
                body,
                &response_body,
                signed_response.as_deref(),
                key,
            )?;
        }
        Ok(HttpExchange {
            status,
            body: response_body,
            header,
        })
    }
}

enum HttpStream {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}
impl Read for HttpStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(bytes),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.read(bytes),
        }
    }
}
impl Write for HttpStream {
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
fn connect_addresses(
    addresses: impl IntoIterator<Item = std::net::SocketAddr>,
    timeout: Duration,
) -> std::io::Result<TcpStream> {
    let deadline = std::time::Instant::now() + timeout;
    let mut last = std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        "endpoint has no address",
    );
    for address in addresses {
        let Some(remaining) = deadline
            .checked_duration_since(std::time::Instant::now())
            .filter(|d| !d.is_zero())
        else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "registry connect deadline",
            ));
        };
        match TcpStream::connect_timeout(&address, remaining.min(Duration::from_secs(2))) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = error,
        }
    }
    Err(last)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn later_resolved_address_is_tried_after_connection_refusal() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let good = listener.local_addr().unwrap();
        let rejected = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
        let stream = connect_addresses([rejected, good], Duration::from_secs(2)).unwrap();
        assert_eq!(stream.peer_addr().unwrap(), good);
    }
}
