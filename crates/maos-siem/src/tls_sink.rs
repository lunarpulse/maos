//! Story 23-4 (D-23-4-01, -02, -10, -13) — the syslog-over-TLS sink (RFC 5425).
//!
//! TLS only: there is no plaintext path, and the server is verified against the operator's CA
//! bundle alone (no system or bundled roots) under the configured host name (SNI). Each
//! batch uses one fresh connection, resolved and vetted by the outbound policy on its own
//! (a DNS change between ticks is vetted again) and made only to the vetted addresses.
//! RFC 5425 has no application acknowledgement, so a batch is acknowledged exactly when the
//! collector answers our `close_notify` with its own (§4.4); a peer that resets, closes
//! without one, or stays silent is not an acknowledgement and the batch is re-sent.

use std::io::Write;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_domain::outbound::{OutboundPolicy, ResolveHost};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore};

use crate::export::{BatchSink, Frame, SinkFailure};

/// Bounded waits of one batch connection (AC5: 5 s / 10 s / 10 s / 10 s by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub handshake: Duration,
    pub write: Duration,
    /// How long to wait for the collector's `close_notify` after ours.
    pub close: Duration,
}

/// The TLS client configuration (it holds the client private key: never printed).
pub type ClientConfigHandle = Arc<ClientConfig>;
impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(5),
            handshake: Duration::from_secs(10),
            write: Duration::from_secs(10),
            close: Duration::from_secs(10),
        }
    }
}

/// Why the TLS material cannot be used. Carries nothing from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupError {
    /// The CA bundle holds no usable certificate.
    CaInvalid,
    ClientCertInvalid,
    /// The key is not a PEM private key, or does not match the certificate.
    ClientKeyInvalid,
}

impl SetupError {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::CaInvalid => "siem_ca_invalid",
            Self::ClientCertInvalid => "siem_client_cert_invalid",
            Self::ClientKeyInvalid => "siem_client_key_invalid",
        }
    }
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for SetupError {}

/// The TLS client configuration: trust anchors from `ca_pem` only, TLS 1.3 and 1.2 over
/// `ring`, and the optional client certificate (RFC 9662 §4: rustls offers
/// `TLS_ECDHE_*_WITH_AES_128_GCM_SHA256` and TLS 1.3; it does not offer static-RSA CBC).
///
/// # Errors
/// [`SetupError`] for a bundle with no certificate, or an unusable client certificate or key.
pub fn client_config(
    ca_pem: &[u8],
    client: Option<(&[u8], &[u8])>,
) -> Result<Arc<ClientConfig>, SetupError> {
    let mut roots = RootCertStore::empty();
    let mut anchors = 0_usize;
    for cert in CertificateDer::pem_slice_iter(ca_pem) {
        let cert = cert.map_err(|_| SetupError::CaInvalid)?;
        roots.add(cert).map_err(|_| SetupError::CaInvalid)?;
        anchors += 1;
    }
    if anchors == 0 {
        return Err(SetupError::CaInvalid);
    }
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| SetupError::CaInvalid)?
            .with_root_certificates(roots);
    let config = match client {
        None => builder.with_no_client_auth(),
        Some((cert_pem, key_pem)) => {
            let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem)
                .collect::<Result<_, _>>()
                .map_err(|_| SetupError::ClientCertInvalid)?;
            if chain.is_empty() {
                return Err(SetupError::ClientCertInvalid);
            }
            let key: PrivateKeyDer<'static> =
                PrivateKeyDer::from_pem_slice(key_pem).map_err(|_| SetupError::ClientKeyInvalid)?;
            builder
                .with_client_auth_cert(chain, key)
                .map_err(|_| SetupError::ClientKeyInvalid)?
        }
    };
    Ok(Arc::new(config))
}

/// Where in a delivery a test hook runs.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseStage {
    /// The batch is written and flushed; the pre-close look has not started.
    BeforeEarlyCheck,
    /// The pre-close look found nothing; our `close_notify` has not been sent.
    AfterEarlyCheck,
}

/// A test hook (see [`TlsSink::with_close_hook`]).
#[doc(hidden)]
pub type CloseHook = Arc<dyn Fn(&TcpStream, CloseStage) + Send + Sync>;

/// The syslog-over-TLS [`BatchSink`].
pub struct TlsSink {
    host: String,
    port: u16,
    config: Arc<ClientConfig>,
    policy: OutboundPolicy,
    resolver: Arc<dyn ResolveHost>,
    timeouts: Timeouts,
    cancel: Option<Arc<AtomicBool>>,
    hook: Option<CloseHook>,
}

impl std::fmt::Debug for TlsSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the TLS configuration: it holds the client private key.
        f.debug_struct("TlsSink")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("timeouts", &self.timeouts)
            .finish_non_exhaustive()
    }
}

impl TlsSink {
    /// `host` is a name, an IPv4 literal, or an IPv6 literal with or without brackets.
    #[must_use]
    pub fn new(
        host: &str,
        port: u16,
        config: Arc<ClientConfig>,
        policy: OutboundPolicy,
        resolver: Arc<dyn ResolveHost>,
        timeouts: Timeouts,
    ) -> Self {
        Self {
            host: host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned(),
            port,
            config,
            policy,
            resolver,
            timeouts,
            cancel: None,
            hook: None,
        }
    }
    /// Install a test hook that runs on each side of the pre-close look (ordering fixtures).
    #[doc(hidden)]
    #[must_use]
    pub fn with_close_hook(mut self, hook: CloseHook) -> Self {
        self.hook = Some(hook);
        self
    }

    fn check_cancel(&self) -> Result<(), SinkFailure> {
        match &self.cancel {
            Some(flag) if flag.load(Ordering::SeqCst) => {
                Err(SinkFailure::new("siem_export_cancelled"))
            }
            _ => Ok(()),
        }
    }
    fn connect(&self, addresses: &[IpAddr]) -> Result<TcpStream, SinkFailure> {
        let deadline = Instant::now() + self.timeouts.connect;
        let mut refused = SinkFailure::new("siem_sink_connect_failed");
        for ip in addresses {
            let Some(remaining) = remaining(deadline) else {
                return Err(SinkFailure::new("siem_sink_timeout"));
            };
            match TcpStream::connect_timeout(&SocketAddr::new(*ip, self.port), remaining) {
                Ok(stream) => return Ok(stream),
                Err(error) if is_timeout(&error) => refused = SinkFailure::new("siem_sink_timeout"),
                Err(_) => refused = SinkFailure::new("siem_sink_connect_failed"),
            }
        }
        Err(refused)
    }
}

impl BatchSink for TlsSink {
    fn identity(&self) -> String {
        if self.host.contains(':') {
            format!("tls://[{}]:{}", self.host, self.port)
        } else {
            format!("tls://{}:{}", self.host, self.port)
        }
    }

    fn set_cancel(&mut self, cancel: Arc<AtomicBool>) {
        self.cancel = Some(cancel);
    }

    fn deliver(&mut self, frames: &[Frame]) -> Result<(), SinkFailure> {
        self.check_cancel()?;
        // One resolution per connection; the connection goes only to what it vetted.
        let addresses = self
            .policy
            .vet_host_port(&self.host, self.port, self.resolver.as_ref())
            .map_err(|refusal| SinkFailure::new(refusal.code()))?;
        let mut tcp = self.connect(&addresses)?;
        let _ = tcp.set_nodelay(true);
        let name = ServerName::try_from(self.host.clone())
            .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?;
        let mut tls = ClientConnection::new(Arc::clone(&self.config), name)
            .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?;

        // The handshake is driven one record at a time (see `drive_handshake`).
        drive_handshake(
            &mut tls,
            &mut tcp,
            self.timeouts.handshake,
            &Instant::now,
            &|| self.check_cancel().is_err(),
        )?;

        let write = Instant::now() + self.timeouts.write;
        let payload = octet_counted(frames);
        let mut sent = 0;
        while sent < payload.len() {
            self.check_cancel()?;
            let accepted = tls
                .writer()
                .write(&payload[sent..])
                .map_err(|_| SinkFailure::new("siem_sink_io"))?;
            sent += accepted;
            flush(&mut tls, &mut tcp, write)?;
        }

        // A collector that closed first (its close_notify is already here) did not read this
        // batch through OUR close: counting that close_notify as the acknowledgement would
        // advance the cursor past rows it never saw. Look before we send ours. Residual
        // (RFC 5425 gives no way around it): a close_notify that crosses ours on the wire is
        // indistinguishable from the acknowledgement.
        if let Some(hook) = &self.hook {
            hook(&tcp, CloseStage::BeforeEarlyCheck);
        }
        early_close_check(&mut tls, &mut tcp, write, &|| self.check_cancel().is_err())?;
        if let Some(hook) = &self.hook {
            hook(&tcp, CloseStage::AfterEarlyCheck);
        }
        tls.send_close_notify();
        flush(&mut tls, &mut tcp, write)?;

        // RFC 5425 §4.4: the receiver answers our close_notify with its own.
        let close = Instant::now() + self.timeouts.close;
        loop {
            if tls
                .process_new_packets()
                .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?
                .peer_has_closed()
            {
                return Ok(());
            }
            self.check_cancel()?;
            if arm(&tcp, close).is_err() {
                return Err(SinkFailure::new("siem_sink_no_close_notify"));
            }
            match tls.read_tls(&mut tcp) {
                // The peer closed the TCP stream without a close_notify.
                Ok(0) => return Err(SinkFailure::new("siem_sink_no_close_notify")),
                Ok(_) => {}
                Err(error) if is_timeout(&error) => {}
                Err(_) => return Err(SinkFailure::new("siem_sink_no_close_notify")),
            }
        }
    }
}

/// Most reads the pre-close look takes before it stops looking.
#[doc(hidden)]
pub const MAX_DRAIN_READS: usize = 64;

/// A stream whose next blocking operation can be bounded (the handshake seam).
#[doc(hidden)]
pub trait StepIo: std::io::Read + std::io::Write {
    /// Bound the next read by `left` (at most one [`SLICE`]) and the next write by `left`.
    ///
    /// # Errors
    /// The transport refused the bound.
    fn bound(&mut self, left: Duration) -> std::io::Result<()>;
}

impl StepIo for TcpStream {
    fn bound(&mut self, left: Duration) -> std::io::Result<()> {
        self.set_read_timeout(Some(left.min(SLICE)))?;
        self.set_write_timeout(Some(left))
    }
}

/// The handshake, one record at a time: the cancel flag and the deadline (read from `now`) are
/// checked on EVERY step, so a peer that keeps trickling forever cannot stretch it.
///
/// # Errors
/// A typed [`SinkFailure`] (timeout, cancel, refusal, transport).
#[doc(hidden)]
pub fn drive_handshake<S: StepIo>(
    tls: &mut ClientConnection,
    io: &mut S,
    budget: Duration,
    now: &dyn Fn() -> Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), SinkFailure> {
    let deadline = now() + budget;
    while tls.is_handshaking() {
        if cancelled() {
            return Err(SinkFailure::new("siem_export_cancelled"));
        }
        let left = deadline
            .checked_duration_since(now())
            .filter(|left| !left.is_zero())
            .ok_or(SinkFailure::new("siem_sink_timeout"))?;
        io.bound(left)
            .map_err(|_| SinkFailure::new("siem_sink_io"))?;
        if tls.wants_write() {
            tls.write_tls(io)
                .map_err(|error| classify(&error, "siem_sink_io"))?;
            continue;
        }
        match tls.read_tls(io) {
            Ok(0) => return Err(SinkFailure::new("siem_sink_io")),
            Ok(_) => {
                tls.process_new_packets()
                    .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?;
            }
            Err(error) if is_timeout(&error) => {}
            Err(error) => return Err(classify(&error, "siem_sink_io")),
        }
    }
    Ok(())
}

/// Look at what the peer already sent, without blocking: a collector that closed first did not
/// read this batch through OUR close, and counting its `close_notify` as the acknowledgement
/// would move the cursor past rows it never saw.
fn early_close_check(
    tls: &mut ClientConnection,
    tcp: &mut TcpStream,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), SinkFailure> {
    tcp.set_nonblocking(true)
        .map_err(|_| SinkFailure::new("siem_sink_io"))?;
    let outcome = drain_pending(tls, tcp, deadline, &Instant::now, cancelled);
    let _ = tcp.set_nonblocking(false);
    outcome
}

/// Drain what `reader` (a NON-BLOCKING source) holds. `Ok` only when the source ran dry
/// (`WouldBlock`) with no close seen. A close, a refusal, the deadline and the cancel flag are
/// errors; so is EXHAUSTION: after [`MAX_DRAIN_READS`] successful reads the source may still
/// hold a close queued behind them, so the batch is NOT acknowledgeable
/// (`siem_sink_drain_exhausted`; the cursor stays and the batch is re-sent).
///
/// # Errors
/// A typed [`SinkFailure`].
#[doc(hidden)]
pub fn drain_pending<R: std::io::Read>(
    tls: &mut ClientConnection,
    reader: &mut R,
    deadline: Instant,
    now: &dyn Fn() -> Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), SinkFailure> {
    // A close_notify read together with the last handshake flight is already processed: ask
    // the connection before reading more.
    let pending = tls
        .process_new_packets()
        .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?;
    if pending.peer_has_closed() {
        return Err(SinkFailure::new("siem_sink_no_close_notify"));
    }
    for _ in 0..MAX_DRAIN_READS {
        if cancelled() {
            return Err(SinkFailure::new("siem_export_cancelled"));
        }
        if deadline
            .checked_duration_since(now())
            .is_none_or(|left| left.is_zero())
        {
            return Err(SinkFailure::new("siem_sink_timeout"));
        }
        match tls.read_tls(reader) {
            Ok(0) => return Err(SinkFailure::new("siem_sink_no_close_notify")),
            Ok(_) => {
                let state = tls
                    .process_new_packets()
                    .map_err(|_| SinkFailure::new("siem_sink_tls_refused"))?;
                if state.peer_has_closed() {
                    return Err(SinkFailure::new("siem_sink_no_close_notify"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(_) => return Err(SinkFailure::new("siem_sink_no_close_notify")),
        }
    }
    Err(SinkFailure::new("siem_sink_drain_exhausted"))
}

/// RFC 5425 §4.3: `MSG-LEN SP SYSLOG-MSG`, MSG-LEN in octets.
fn octet_counted(frames: &[Frame]) -> Vec<u8> {
    let mut out = Vec::new();
    for frame in frames {
        out.extend_from_slice(frame.syslog.len().to_string().as_bytes());
        out.push(b' ');
        out.extend_from_slice(frame.syslog.as_bytes());
    }
    out
}

fn flush(
    tls: &mut ClientConnection,
    tcp: &mut TcpStream,
    deadline: Instant,
) -> Result<(), SinkFailure> {
    while tls.wants_write() {
        arm(tcp, deadline)?;
        tls.write_tls(tcp)
            .map_err(|error| classify(&error, "siem_sink_io"))?;
    }
    Ok(())
}

fn remaining(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

/// Longest a single socket read blocks. Reads wake at least this often, so every phase
/// re-checks its deadline and the shutdown flag on EVERY step, however slowly a peer trickles.
const SLICE: Duration = Duration::from_millis(250);

/// Bound the next socket operation by what is left of the phase (reads by one [`SLICE`]).
fn arm(tcp: &TcpStream, deadline: Instant) -> Result<(), SinkFailure> {
    let left = remaining(deadline).ok_or(SinkFailure::new("siem_sink_timeout"))?;
    tcp.set_read_timeout(Some(left.min(SLICE)))
        .and_then(|()| tcp.set_write_timeout(Some(left)))
        .map_err(|_| SinkFailure::new("siem_sink_io"))
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// A timeout is a timeout; a TLS alert or certificate failure (`InvalidData`) is a refusal;
/// anything else is the transport.
fn classify(error: &std::io::Error, fallback: &'static str) -> SinkFailure {
    SinkFailure::new(if is_timeout(error) {
        "siem_sink_timeout"
    } else if error.kind() == std::io::ErrorKind::InvalidData {
        "siem_sink_tls_refused"
    } else {
        fallback
    })
}
