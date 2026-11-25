//! Story 23-0b — a policed host-name request connects only to the addresses
//! the outbound policy vetted (D-23-0b-01 A, 02 a, 03 a, 04 a, 05 a).
//!
//! Loopback fixtures only: no system DNS, no network. The listener threads
//! stop on a flag and are joined before any count is read, so no assertion
//! depends on timing. `.test` is never resolvable (RFC 6761): the only way a
//! connection can reach a fixture is through the vetted address set.

use std::io::ErrorKind;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use maos_domain::outbound::{OutboundPolicy, PolicedIo, ResolveFailed, ResolveHost};
use maos_domain::ports::io_subsystem::{IoError, IoSubsystemPort};
use maos_kernel_core::api::IoSubsystemAdapter;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

const NAME: &str = "rebind.maos.test";
const CERT_DER: &[u8] = include_bytes!("fixtures/io_binding_23_0b/cert.der");
const KEY_DER: &[u8] = include_bytes!("fixtures/io_binding_23_0b/key.der");

/// Answers `answers[n]` on call `n` and the last answer on every later call.
struct Scripted {
    calls: AtomicUsize,
    answers: Vec<Vec<IpAddr>>,
}

impl Scripted {
    fn new(answers: &[&[&str]]) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            answers: answers
                .iter()
                .map(|set| set.iter().map(|a| a.parse().unwrap()).collect())
                .collect(),
        })
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

struct Shared(Arc<Scripted>);

impl ResolveHost for Shared {
    fn resolve(&self, host: &str, _port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        assert_eq!(host, NAME);
        let n = self.0.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.0.answers[n.min(self.0.answers.len() - 1)].clone())
    }
}

#[derive(Clone, Copy)]
enum Serve {
    /// Read the ClientHello, record the SNI, close.
    HelloThenClose,
    /// Read the ClientHello, record the SNI, answer with the self-signed certificate.
    SelfSignedCertificate,
}

struct Fixture {
    port: u16,
    accepted: Arc<AtomicUsize>,
    snis: Arc<Mutex<Vec<Option<String>>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Listener B on 127.0.0.2 with the same port (Linux only: macOS has no 127.0.0.2).
    other: Option<TcpListener>,
}

impl Fixture {
    fn start(serve: Serve) -> Self {
        let (listener, other) = (0..64)
            .find_map(|_| {
                let a = TcpListener::bind("127.0.0.1:0").ok()?;
                let port = a.local_addr().ok()?.port();
                if cfg!(target_os = "linux") {
                    let b = TcpListener::bind(("127.0.0.2", port)).ok()?;
                    b.set_nonblocking(true).ok()?;
                    Some((a, Some(b)))
                } else {
                    Some((a, None))
                }
            })
            .expect("a loopback port free on 127.0.0.1 and 127.0.0.2");
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let snis = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (accepted, snis, stop) = (accepted.clone(), snis.clone(), stop.clone());
            thread::spawn(move || {
                // The deadline only ends a runaway fixture; no assertion reads it.
                let deadline = Instant::now() + Duration::from_secs(60);
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            accepted.fetch_add(1, Ordering::SeqCst);
                            handle(stream, serve, &snis);
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            if stop.load(Ordering::SeqCst) || Instant::now() > deadline {
                                break;
                            }
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Self {
            port,
            accepted,
            snis,
            stop,
            thread: Some(thread),
            other,
        }
    }

    /// Stop the listener (after it drained its backlog) and return how many
    /// connections it accepted.
    fn finish(&mut self) -> usize {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().expect("finished once").join().unwrap();
        self.accepted.load(Ordering::SeqCst)
    }

    /// Connections that reached listener B (always 0 off Linux).
    fn other_connections(&self) -> usize {
        let Some(b) = &self.other else { return 0 };
        let mut n = 0;
        while b.accept().is_ok() {
            n += 1;
        }
        n
    }

    fn url(&self, path: &str) -> String {
        format!("https://{NAME}:{}{path}", self.port)
    }
}

fn handle(mut stream: TcpStream, serve: Serve, snis: &Mutex<Vec<Option<String>>>) {
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut acceptor = rustls::server::Acceptor::default();
    let accepted = loop {
        match acceptor.read_tls(&mut stream) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        match acceptor.accept() {
            Ok(Some(accepted)) => break accepted,
            Ok(None) => {}
            Err(_) => return,
        }
    };
    snis.lock()
        .unwrap()
        .push(accepted.client_hello().server_name().map(str::to_owned));
    if let Serve::HelloThenClose = serve {
        return;
    }
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(CERT_DER.to_vec())],
        PrivateKeyDer::try_from(KEY_DER.to_vec()).unwrap(),
    )
    .unwrap();
    if let Ok(mut connection) = accepted.into_connection(Arc::new(config)) {
        // The client aborts the handshake when it refuses the certificate.
        let _ = connection.complete_io(&mut stream);
    }
}

fn policed(
    resolver: &Arc<Scripted>,
    policy: OutboundPolicy,
) -> PolicedIo<IoSubsystemAdapter, Shared> {
    PolicedIo::new(
        IoSubsystemAdapter::new(),
        Shared(Arc::clone(resolver)),
        policy,
    )
    .expect("the kernel adapter declares the no-follow contract")
}

#[test]
fn a_changed_second_answer_does_not_move_the_connection() {
    let mut fixture = Fixture::start(Serve::HelloThenClose);
    // First answer 127.0.0.1 (listener A), every later answer 127.0.0.2 (listener B).
    let resolver = Scripted::new(&[&["127.0.0.1"], &["127.0.0.2"]]);
    let io = policed(&resolver, OutboundPolicy::strict().allow_private());
    let result = io.http_get(&fixture.url("/x"));
    assert!(result.is_err(), "the fixture closes after the ClientHello");
    assert_eq!(resolver.calls(), 1, "one resolution per request");
    assert_eq!(fixture.finish(), 1, "listener A saw exactly one connection");
    assert_eq!(fixture.other_connections(), 0, "listener B saw none");
}

#[test]
fn a_mixed_answer_is_refused_before_any_connection() {
    let mut fixture = Fixture::start(Serve::HelloThenClose);
    let resolver = Scripted::new(&[&["127.0.0.1", "169.254.169.254"]]);
    let io = policed(&resolver, OutboundPolicy::strict().allow_private());
    assert_eq!(
        io.http_get(&fixture.url("/x")),
        Err(IoError::InvalidUrl(
            "outbound_link_local_refused".to_owned()
        ))
    );
    assert_eq!(resolver.calls(), 1);
    assert_eq!(fixture.finish(), 0);
    assert_eq!(fixture.other_connections(), 0);
}

#[test]
fn a_bound_port_refuses_a_different_host_or_port() {
    let mut fixture = Fixture::start(Serve::HelloThenClose);
    let resolver = Scripted::new(&[&["127.0.0.1"]]);
    let vetted = OutboundPolicy::strict()
        .allow_private()
        .vet_url(&fixture.url("/x"), &Shared(Arc::clone(&resolver)))
        .unwrap();
    let bound = IoSubsystemAdapter::new().bind(&vetted).unwrap();
    assert!(bound.never_follows_redirects());
    for other in [
        format!("https://other.maos.test:{}/", fixture.port),
        format!("https://{NAME}:{}/", fixture.port.wrapping_add(1)),
        format!("https://127.0.0.1:{}/", fixture.port),
    ] {
        match bound.http_get(&other) {
            Err(IoError::Transport(text)) => {
                assert!(text.contains("outbound_unbound_refused"), "{other}: {text}");
            }
            result => panic!("{other}: expected a transport refusal, got {result:?}"),
        }
        assert!(matches!(
            bound.http_post_exchange(&other, b"", &[], ""),
            Err(IoError::Transport(_))
        ));
    }
    assert_eq!(fixture.finish(), 0, "no connection left the bound port");
    assert_eq!(fixture.other_connections(), 0);
}

#[test]
fn the_tls_name_is_the_original_host() {
    let mut fixture = Fixture::start(Serve::HelloThenClose);
    let resolver = Scripted::new(&[&["127.0.0.1"]]);
    let io = policed(&resolver, OutboundPolicy::strict().allow_private());
    assert!(io.http_get(&fixture.url("/x")).is_err());
    assert_eq!(fixture.finish(), 1);
    // SNI is the host name, never the vetted IP (RFC 6066 section 3).
    assert_eq!(*fixture.snis.lock().unwrap(), vec![Some(NAME.to_owned())]);
}

#[test]
fn an_untrusted_certificate_for_the_name_is_refused() {
    let mut fixture = Fixture::start(Serve::SelfSignedCertificate);
    let resolver = Scripted::new(&[&["127.0.0.1"]]);
    let io = policed(&resolver, OutboundPolicy::strict().allow_private());
    let result = io.http_get(&fixture.url("/x"));
    let text = format!("{result:?}").to_ascii_lowercase();
    assert!(
        matches!(result, Err(IoError::Transport(_))) && text.contains("certificate"),
        "expected a certificate refusal, got {text}"
    );
    assert_eq!(fixture.finish(), 1);
    assert_eq!(*fixture.snis.lock().unwrap(), vec![Some(NAME.to_owned())]);
}

#[test]
fn a_literal_ip_never_calls_bind_or_the_resolver() {
    let mut fixture = Fixture::start(Serve::HelloThenClose);
    let resolver = Scripted::new(&[&["127.0.0.2"]]);
    let io = policed(&resolver, OutboundPolicy::strict().allow_loopback_http());
    let url = format!("https://127.0.0.1:{}/", fixture.port);
    assert!(io.http_get(&url).is_err());
    assert_eq!(resolver.calls(), 0);
    assert_eq!(fixture.finish(), 1, "the literal reached listener A");
    // The raw-string path carries no pinned resolver: SNI is absent for an IP.
    assert_eq!(*fixture.snis.lock().unwrap(), vec![None]);
}
