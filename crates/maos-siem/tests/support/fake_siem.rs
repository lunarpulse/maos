//! Story 23-4 — the in-process fake SIEM: a rustls server on `127.0.0.1:0` with an `rcgen` CA.
//!
//! No sleeps: every wait is a channel receive or a gate the test opens. A connection is read
//! on the server's own thread; each one ends in exactly one [`Event`]. Shared by
//! `maos-siem/tests/tls_sink_23_4.rs` and `maos-bin/tests/siem_network_23_4.rs` through
//! `#[path]`, so it takes only `rustls`, `rcgen` and `maos-domain`.
#![allow(dead_code)]

use std::collections::HashSet;
use std::io::Read;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use maos_domain::outbound::{ResolveFailed, ResolveHost};
use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig, ServerConnection};

/// The outer deadline every wait on the fake is bounded by (never an assertion on speed).
pub const OUTER: Duration = Duration::from_secs(30);

// ── PKI ──────────────────────────────────────────────────────────────────────

pub struct Ca {
    pub pem: String,
    der: CertificateDer<'static>,
    cert: rcgen::Certificate,
    key: KeyPair,
}

pub struct Leaf {
    pub cert_pem: String,
    pub key_pem: String,
    der: CertificateDer<'static>,
    key_der: Vec<u8>,
}

impl Ca {
    pub fn new(name: &str) -> Self {
        let key = KeyPair::generate().expect("ca key");
        let mut params = CertificateParams::new(vec![name.to_string()]).expect("ca params");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key).expect("ca cert");
        Self {
            pem: cert.pem(),
            der: cert.der().clone(),
            cert,
            key,
        }
    }

    pub fn der(&self) -> CertificateDer<'static> {
        self.der.clone()
    }

    pub fn server(&self, name: &str) -> Leaf {
        self.leaf(name, ExtendedKeyUsagePurpose::ServerAuth)
    }

    pub fn client(&self, name: &str) -> Leaf {
        self.leaf(name, ExtendedKeyUsagePurpose::ClientAuth)
    }

    fn leaf(&self, name: &str, usage: ExtendedKeyUsagePurpose) -> Leaf {
        let key = KeyPair::generate().expect("leaf key");
        let mut params = CertificateParams::new(vec![name.to_string()]).expect("leaf params");
        params.extended_key_usages = vec![usage];
        let cert = params
            .signed_by(&key, &self.cert, &self.key)
            .expect("leaf cert");
        Leaf {
            cert_pem: cert.pem(),
            key_pem: key.serialize_pem(),
            der: cert.der().clone(),
            key_der: key.serialize_der(),
        }
    }
}

// ── gates ────────────────────────────────────────────────────────────────────

/// A latch the test opens; waiters block on it (bounded by [`OUTER`]).
#[derive(Clone)]
pub struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    pub fn new(open: bool) -> Self {
        Self(Arc::new((Mutex::new(open), Condvar::new())))
    }

    pub fn open(&self) {
        *self.0 .0.lock().unwrap() = true;
        self.0 .1.notify_all();
    }

    fn wait(&self) {
        let guard = self.0 .0.lock().unwrap();
        let _ = self
            .0
             .1
            .wait_timeout_while(guard, OUTER, |open| !*open)
            .unwrap();
    }
}

// ── the server ───────────────────────────────────────────────────────────────

/// How the next connection is treated.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Read to the client's `close_notify`, then (after the ack gate) answer with ours.
    Ack,
    /// Read the batch, then drop the TCP stream without a `close_notify`.
    CloseWithoutNotify,
    /// Accept the TCP connection and do nothing until the handshake gate opens.
    HoldHandshake,
    /// Complete the handshake, send `close_notify` and close the TCP stream at once.
    CloseFirst,
}

/// What one connection ended in.
#[derive(Debug)]
pub enum Event {
    /// The batch was read through the client's `close_notify` and answered with ours.
    Acked {
        bytes: Vec<u8>,
        version: String,
        suite: String,
    },
    /// The batch was read but never answered (reset, or the mode said so).
    Unacked { bytes: Vec<u8> },
    /// The TLS handshake or a record was refused (the server's reason, for diagnosis only).
    Refused(String),
    /// A connection was accepted in [`Mode::HoldHandshake`] and is being held.
    Held,
}

#[derive(Default)]
pub struct Options {
    /// Require a client certificate issued by this CA (its DER certificate).
    pub require_client_ca: Option<CertificateDer<'static>>,
    /// Offer only TLS 1.2 with `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256`.
    pub tls12_ecdhe_gcm_only: bool,
    /// Start with the ack gate closed: the server reads a batch but does not answer its
    /// `close_notify` until the test opens `ack_gate`.
    pub hold_ack: bool,
}

pub struct FakeSiem {
    pub addr: SocketAddr,
    pub events: Receiver<Event>,
    /// Held closed, the server does not answer a `close_notify`.
    pub ack_gate: Gate,
    pub handshake_gate: Gate,
    mode: Arc<Mutex<Mode>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FakeSiem {
    pub fn start(leaf: &Leaf, options: Options) -> Self {
        let provider = {
            let mut provider = rustls::crypto::ring::default_provider();
            if options.tls12_ecdhe_gcm_only {
                provider.cipher_suites = vec![
                    rustls::crypto::ring::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                ];
            }
            Arc::new(provider)
        };
        let builder = ServerConfig::builder_with_provider(Arc::clone(&provider));
        let builder = if options.tls12_ecdhe_gcm_only {
            builder.with_protocol_versions(&[&rustls::version::TLS12])
        } else {
            builder.with_safe_default_protocol_versions()
        }
        .expect("server versions");
        let builder = match &options.require_client_ca {
            Some(ca_der) => {
                let mut roots = RootCertStore::empty();
                roots.add(ca_der.clone()).expect("client ca");
                let verifier =
                    WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider)
                        .build()
                        .expect("client verifier");
                builder.with_client_cert_verifier(verifier)
            }
            None => builder.with_no_client_auth(),
        };
        let config = Arc::new(
            builder
                .with_single_cert(
                    vec![leaf.der.clone()],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf.key_der.clone())),
                )
                .expect("server cert"),
        );

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake siem");
        let addr = listener.local_addr().expect("fake siem addr");
        let (sender, events) = channel();
        let ack_gate = Gate::new(!options.hold_ack);
        let handshake_gate = Gate::new(false);
        let mode = Arc::new(Mutex::new(Mode::Ack));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let (ack, hold, mode, stop) = (
                ack_gate.clone(),
                handshake_gate.clone(),
                Arc::clone(&mode),
                Arc::clone(&stop),
            );
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    let Ok(stream) = stream else { continue };
                    let mode = *mode.lock().unwrap();
                    serve(stream, Arc::clone(&config), mode, &ack, &hold, &sender);
                }
            })
        };
        Self {
            addr,
            events,
            ack_gate,
            handshake_gate,
            mode,
            stop,
            handle: Some(handle),
        }
    }

    pub fn set_mode(&self, mode: Mode) {
        *self.mode.lock().unwrap() = mode;
    }

    /// The next connection's outcome, bounded by [`OUTER`].
    pub fn next_event(&self) -> Event {
        self.events
            .recv_timeout(OUTER)
            .expect("the fake SIEM saw no connection end")
    }

    /// Every event already queued (never blocks).
    pub fn drain(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Stop the server thread and join it.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.ack_gate.open();
        self.handshake_gate.open();
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("fake SIEM thread");
        }
    }
}

impl Drop for FakeSiem {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.shutdown();
        }
    }
}

fn serve(
    mut tcp: TcpStream,
    config: Arc<ServerConfig>,
    mode: Mode,
    ack: &Gate,
    hold: &Gate,
    events: &Sender<Event>,
) {
    if mode == Mode::HoldHandshake {
        // The TCP connection is accepted; the client is now committed to a handshake that
        // will not complete until the test opens the gate.
        let _ = events.send(Event::Held);
        hold.wait();
    }
    let _ = tcp.set_read_timeout(Some(OUTER));
    let mut conn = ServerConnection::new(config).expect("server connection");
    let mut bytes = Vec::new();
    loop {
        match conn.read_tls(&mut tcp) {
            Ok(0) | Err(_) => {
                let _ = events.send(Event::Unacked { bytes });
                return;
            }
            Ok(_) => {}
        }
        let state = match conn.process_new_packets() {
            Ok(state) => state,
            Err(error) => {
                let _ = conn.write_tls(&mut tcp);
                let _ = events.send(Event::Refused(error.to_string()));
                return;
            }
        };
        let readable = state.plaintext_bytes_to_read();
        if readable > 0 {
            let mut chunk = vec![0_u8; readable];
            conn.reader().read_exact(&mut chunk).expect("plaintext");
            bytes.extend_from_slice(&chunk);
        }
        if mode == Mode::CloseFirst && !conn.is_handshaking() {
            conn.send_close_notify();
        }
        while conn.wants_write() {
            if conn.write_tls(&mut tcp).is_err() {
                let _ = events.send(Event::Unacked { bytes });
                return;
            }
        }
        if mode == Mode::CloseFirst && !conn.is_handshaking() {
            // The collector closes first: its close_notify and the FIN leave with the last
            // handshake flight, before it has read (or been sent a close for) the batch. It
            // then keeps draining the client's bytes so the close is not a reset.
            let _ = tcp.shutdown(std::net::Shutdown::Write);
            let mut sink = [0_u8; 4096];
            while matches!(std::io::Read::read(&mut tcp, &mut sink), Ok(n) if n > 0) {}
            let _ = events.send(Event::Unacked { bytes });
            return;
        }
        if state.peer_has_closed() {
            break;
        }
    }
    if mode == Mode::CloseWithoutNotify {
        let _ = events.send(Event::Unacked { bytes });
        return;
    }
    ack.wait();
    let version = conn
        .protocol_version()
        .map_or_else(String::new, |v| format!("{v:?}"));
    let suite = conn
        .negotiated_cipher_suite()
        .map_or_else(String::new, |s| format!("{:?}", s.suite()));
    conn.send_close_notify();
    while conn.wants_write() {
        if conn.write_tls(&mut tcp).is_err() {
            break;
        }
    }
    let _ = events.send(Event::Acked {
        bytes,
        version,
        suite,
    });
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Split RFC 5425 octet-counted frames (`MSG-LEN SP SYSLOG-MSG`); panics on a malformed stream.
pub fn octet_frames(bytes: &[u8]) -> Vec<String> {
    let (mut frames, mut at) = (Vec::new(), 0);
    while at < bytes.len() {
        let space = bytes[at..]
            .iter()
            .position(|b| *b == b' ')
            .expect("MSG-LEN is followed by SP");
        let len: usize = std::str::from_utf8(&bytes[at..at + space])
            .expect("MSG-LEN is ASCII")
            .parse()
            .expect("MSG-LEN is a number");
        let start = at + space + 1;
        frames.push(String::from_utf8(bytes[start..start + len].to_vec()).expect("UTF-8 frame"));
        at = start + len;
    }
    frames
}

/// The `externalId=` value of every frame.
pub fn external_ids(frames: &[String]) -> Vec<String> {
    frames
        .iter()
        .map(|frame| {
            let rest = frame
                .split(" externalId=")
                .nth(1)
                .expect("every frame carries externalId");
            rest.split(' ').next().expect("externalId value").to_owned()
        })
        .collect()
}

pub fn id_set(ids: &[String]) -> HashSet<String> {
    ids.iter().cloned().collect()
}

/// A resolver that answers `host` from a script (the last answer repeats) and counts calls.
pub struct StubResolver {
    host: String,
    answers: Vec<Vec<IpAddr>>,
    pub calls: AtomicUsize,
}

impl StubResolver {
    pub fn new(host: &str, answers: Vec<Vec<IpAddr>>) -> Arc<Self> {
        Arc::new(Self {
            host: host.to_owned(),
            answers,
            calls: AtomicUsize::new(0),
        })
    }

    pub fn loopback(host: &str) -> Arc<Self> {
        Self::new(host, vec![vec![IpAddr::from([127, 0, 0, 1])]])
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ResolveHost for StubResolver {
    fn resolve(&self, host: &str, _port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if host != self.host {
            return Err(ResolveFailed);
        }
        let index = call.min(self.answers.len() - 1);
        Ok(self.answers[index].clone())
    }
}
