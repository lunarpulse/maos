#![cfg(all(feature = "discovery", not(feature = "sso-fault-inject")))]

//! Story 23-2 AC1 and AC2 - OIDC discovery, key rotation and the trust rules.
//!
//! Hermetic: an injected `Fetcher` and an injected clock. No socket, no wall-clock
//! measurement, and no test asserts an elapsed time: the bounded-wait tests collect
//! the verdict while a gate is still held (and require the gate's safety timer not to
//! have fired). The only real-time quantities are the bounded wait of `verify` (tens
//! of milliseconds), a 1 ms poll for an event that must happen, and short negative
//! windows ("no third fetch started") that can only make a test stricter.
//! A gate that is never released fails CLOSED (the fetch errors).

#[path = "fixtures.rs"]
mod fixtures;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use maos_domain::outbound::OutboundRefusal;
use maos_domain::outbound::{OutboundPolicy, PolicedIo, ResolveFailed, ResolveHost};
use maos_domain::ports::io_subsystem::{IoError, IoSubsystemPort};
use maos_domain::ports::{IdentityAssertionPort, IdentityError};
use maos_sso::discovery::{
    Clock, DiscoveringVerifier, DiscoveryConfig, DiscoveryError, FetchError, Fetcher, IoFetcher,
};
use maos_sso::OidcAlgorithm;

const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";
/// Safety net for every gate: a fixture that waits this long fails closed.
const GATE_SAFETY: Duration = Duration::from_secs(30);

/// An ES256 key (P-256, public half only) and an assertion it signed, with the claims of the
/// RS256 fixtures (generated offline with `openssl`; no private key is kept).
const JWKS_KEY_ES256: &str = "{\"keys\":[{\"kty\":\"EC\",\"use\":\"sig\",\"alg\":\"ES256\",\"kid\":\"key-es\",\"crv\":\"P-256\",\"x\":\"5c4IKQSiIVygYoSsDihGEQiPd82EHle2tU1ERiBY0QE\",\"y\":\"8FVQQagaI23r7lhVb7udfDUMZbutEACebVLAejfr6OQ\"}]}";
const TOKEN_ES256: &str = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS1lcyJ9.eyJzdWIiOiJyZXphQG1hb3MuZXhhbXBsZSIsImlzcyI6Imh0dHBzOi8vaWRwLm1hb3MuZXhhbXBsZSIsImF1ZCI6Im1hb3MtZGVwbG95LWFscGhhIiwiZXhwIjo5OTk5OTk5OTk5LCJuYmYiOjAsImlhdCI6MTcwMDAwMDAwMCwibmFtZSI6IlJlemEgQ29ydGV4IiwiZW1haWwiOiJyZXphQG1hb3MuZXhhbXBsZSJ9.SlN3MshXGc-bskFOZFrbLwA24a13kIk2FPo3bcpU8ySNtOazXBwu3syefQ5VWYEXczZpMgd0rKwVL6lU2jikKw";
/// A clock the test sets by hand; it reads in milliseconds so sub-second limits are testable.
struct ManualClock(AtomicU64);

impl ManualClock {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicU64::new(0)))
    }
    fn set(&self, secs: u64) {
        self.0.store(secs * 1_000, Ordering::SeqCst);
    }
    fn set_millis(&self, millis: u64) {
        self.0.store(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst) / 1_000
    }
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// A gate that holds a fetch until released, and records that it was entered.
#[derive(Default)]
struct Gate {
    state: Mutex<(bool, bool, bool)>, // (entered, released, safety timer fired)
    changed: Condvar,
}

impl Gate {
    fn pass(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.changed.notify_all();
        let (mut state, timeout) = self
            .changed
            .wait_timeout_while(state, GATE_SAFETY, |state| !state.1)
            .unwrap();
        if timeout.timed_out() && !state.1 {
            state.2 = true;
        }
        !timeout.timed_out() && state.1
    }
    fn wait_entered(&self) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, GATE_SAFETY, |state| !state.0)
            .unwrap();
        assert!(!timeout.timed_out() && state.0, "the fetch never entered");
    }
    fn held(&self) -> bool {
        !self.state.lock().unwrap().1
    }
    fn released(&self) -> bool {
        self.state.lock().unwrap().1
    }
    /// The safety timer ran out before anyone released the gate.
    fn expired(&self) -> bool {
        self.state.lock().unwrap().2
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.changed.notify_all();
    }
}

/// Serves canned answers by URL; counts every request.
#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Result<Vec<u8>, FetchError>>>,
    hits: Mutex<Vec<String>>,
    gate_on_document: Mutex<Option<Arc<Gate>>>,
}

impl Script {
    fn serve(&self, url: &str, body: &str) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), Ok(body.as_bytes().to_vec()));
    }
    fn fail(&self, url: &str, error: FetchError) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), Err(error));
    }
    fn count(&self, url: &str) -> usize {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|hit| *hit == url)
            .count()
    }
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.hits.lock().unwrap().push(url.to_owned());
        let gate = self.gate_on_document.lock().unwrap().clone();
        if let (Some(gate), true) = (gate, url == DOC_URL) {
            if !gate.pass() {
                return Err(FetchError::Unavailable);
            }
        }
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .unwrap_or(Err(FetchError::Unavailable))
    }
}

fn document(issuer: &str, jwks_uri: &str) -> String {
    format!("{{\"issuer\":\"{issuer}\",\"jwks_uri\":\"{jwks_uri}\"}}")
}

/// keyA's material under another `kid` (the "old" key of a rotation).
fn old_keys() -> String {
    fixtures::JWKS_KEY_A.replace("key-a", "old-key")
}

/// keyA's material under both its own `kid` and the rotated-in `kid` that
/// `TOKEN_UNKNOWN_KID` names.
fn rotated_keys() -> String {
    let both = fixtures::JWKS_KEY_A.replacen(
        "\"keys\":[",
        &format!(
            "\"keys\":[{},",
            fixtures::JWKS_KEY_A
                .split_once("\"keys\":[")
                .unwrap()
                .1
                .trim_end_matches("]}")
                .replace("key-a", "missing-key")
        ),
        1,
    );
    both
}

fn start(script: &Arc<Script>, clock: &Arc<ManualClock>, wait: Duration) -> DiscoveringVerifier {
    start_with(Arc::clone(script) as Arc<dyn Fetcher>, clock, wait)
}

fn start_with(
    fetcher: Arc<dyn Fetcher>,
    clock: &Arc<ManualClock>,
    wait: Duration,
) -> DiscoveringVerifier {
    let config = DiscoveryConfig::new(fixtures::ISS_GOOD)
        .with_verify_wait(wait)
        .with_idle_tick(Duration::from_secs(3_600));
    DiscoveringVerifier::start(
        config,
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        fetcher,
        Arc::clone(clock) as Arc<dyn Clock>,
    )
    .expect("a valid discovery configuration starts")
}

const LONG_WAIT: Duration = Duration::from_secs(60);

fn healthy_script() -> Arc<Script> {
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, fixtures::JWKS_KEY_A);
    script
}

// ---- AC1 -------------------------------------------------------------------

/// AC1 a) issuer document -> `jwks_uri` -> keys, and a published `kid` verifies.
#[test]
fn a_published_key_verifies_through_discovery() {
    let script = healthy_script();
    let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
    let principal = verifier
        .verify(fixtures::TOKEN_GOOD_RS256)
        .expect("a key published by the issuer verifies");
    assert_eq!(principal.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(principal.issuer, fixtures::ISS_GOOD);
    assert!(verifier.is_healthy());
    assert_eq!(verifier.last_error(), None);
    assert_eq!(script.count(DOC_URL), 1);
    assert_eq!(script.count(JWKS_URL), 1);
}

/// AC1 b) an unknown `kid` causes exactly one more fetch, then verifies.
#[test]
fn an_unknown_kid_refreshes_once_and_then_verifies() {
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, &old_keys());
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, LONG_WAIT);
    // Joins the boot fetch: the published set holds only `old-key`.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(script.count(DOC_URL), 1);

    // The IdP rotates; the clock passes the limiter.
    script.serve(JWKS_URL, &rotated_keys());
    clock.set(31);
    let principal = verifier
        .verify(fixtures::TOKEN_UNKNOWN_KID)
        .expect("the rotated-in key verifies without a restart");
    assert_eq!(principal.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(script.count(DOC_URL), 2, "exactly one refresh");
    assert_eq!(script.count(JWKS_URL), 2);
    // The cached keys now answer without another fetch.
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert_eq!(script.count(DOC_URL), 2);
}

/// AC1 c) the limiter: a second unknown `kid` inside the minimum interval
/// causes no fetch; after the interval, exactly one.
#[test]
fn an_unknown_kid_inside_the_interval_causes_no_fetch() {
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, &old_keys());
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, LONG_WAIT);
    let unknown = || verifier.verify(fixtures::TOKEN_UNKNOWN_KID);

    assert_eq!(unknown(), Err(IdentityError::JwksUnavailable)); // joins boot fetch
    assert_eq!(script.count(DOC_URL), 1);
    clock.set(10);
    assert_eq!(unknown(), Err(IdentityError::JwksUnavailable));
    clock.set(29);
    assert_eq!(unknown(), Err(IdentityError::JwksUnavailable));
    assert_eq!(
        script.count(DOC_URL),
        1,
        "no fetch inside 30 s of the last refresh"
    );

    clock.set(30);
    assert_eq!(unknown(), Err(IdentityError::JwksUnavailable));
    assert_eq!(
        script.count(DOC_URL),
        2,
        "one fetch once the interval passed"
    );
    assert_eq!(unknown(), Err(IdentityError::JwksUnavailable));
    assert_eq!(script.count(DOC_URL), 2, "and the interval restarts");
}

/// AC1 d) past the TTL, with the fetcher failing, EVERY assertion fails closed
/// (including one whose `kid` was cached); when the fetcher recovers,
/// verification resumes.
#[test]
fn past_the_ttl_every_assertion_fails_closed_until_the_fetch_recovers() {
    let script = healthy_script();
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());

    // 3,599 s: still inside the TTL, served from cache with no fetch.
    clock.set(3_599);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert_eq!(script.count(DOC_URL), 1);
    assert!(verifier.is_healthy());

    // 3,600 s: the TTL has ended; the IdP is down.
    script.fail(DOC_URL, FetchError::Unavailable);
    clock.set(3_600);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable),
        "a cached kid is not served past the TTL"
    );
    assert!(!verifier.is_healthy());
    assert_eq!(
        verifier.last_error(),
        Some(DiscoveryError::DocumentFetch(FetchError::Unavailable))
    );
    let fetches = script.count(DOC_URL);
    // Inside the interval nothing is fetched and nothing is served.
    clock.set(3_610);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(script.count(DOC_URL), fetches);

    // The IdP recovers; the next allowed refresh restores service.
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    clock.set(3_631);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert!(verifier.is_healthy());
    assert_eq!(verifier.last_error(), None);
}

/// AC1 e) a hung fetch delays `verify` by no more than the bounded wait and
/// then fails `JwksUnavailable`; a caller that arrives while it is still hung
/// does not wait again.
///
/// The verdict is collected from a thread while the fetch is STILL held, and the gate's own
/// safety timer must not have fired: a verifier that waited for the fetch (no bound) would
/// deliver nothing until the fetch returned, and this test fails instead of passing on the
/// fetch's error. The second call is told apart from "waiting again" by the number of waits the
/// verifier has started, not by a duration.
#[test]
fn a_hung_fetch_is_bounded_and_fails_closed() {
    let script = healthy_script();
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, Duration::from_millis(50));
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    let waits_before = verifier.refresh_waits();

    let gate = Arc::new(Gate::default());
    *script.gate_on_document.lock().unwrap() = Some(Arc::clone(&gate));
    clock.set(40);
    let (verdict, delivered) = std::sync::mpsc::channel();
    let outcome = std::thread::scope(|scope| {
        scope.spawn(|| {
            let _ = verdict.send(verifier.verify(fixtures::TOKEN_UNKNOWN_KID));
        });
        gate.wait_entered();
        let first = delivered.recv_timeout(EVENT_BUDGET);
        let held = gate.held();
        let expired = gate.expired();
        let waits_after_first = verifier.refresh_waits();
        // The stalled refresh is not joined again.
        let second = verifier.verify(fixtures::TOKEN_UNKNOWN_KID);
        let waits_after_second = verifier.refresh_waits();
        // A cached, fresh key is unaffected by a hung refresh of the others.
        let cached = verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok();
        let still_held = gate.held();
        gate.release();
        (
            first,
            held,
            expired,
            waits_after_first,
            second,
            waits_after_second,
            cached,
            still_held,
        )
    });
    let (first, held, expired, waits_after_first, second, waits_after_second, cached, still_held) =
        outcome;
    assert_eq!(
        first,
        Ok(Err(IdentityError::JwksUnavailable)),
        "the bounded wait delivered a verdict"
    );
    assert!(held, "the verdict came while the fetch was still hung");
    assert!(!expired, "the fetch's safety timer had not fired");
    assert_eq!(
        waits_after_first,
        waits_before + 1,
        "the first call waited once"
    );
    assert_eq!(second, Err(IdentityError::JwksUnavailable));
    assert_eq!(
        waits_after_second, waits_after_first,
        "the second call was refused at once, not made to wait again"
    );
    assert!(cached);
    assert!(still_held);
}

// ---- AC2 -------------------------------------------------------------------

/// Join the boot fetch, then require: nothing verifies, unhealthy, and the
/// typed reason is `expected`.
fn assert_refused(verifier: &DiscoveringVerifier, expected: DiscoveryError) {
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert!(!verifier.is_healthy(), "no key was loaded");
    assert_eq!(verifier.last_error(), Some(expected));
}

/// AC2: the document's `issuer` must equal the configured one byte for byte.
#[test]
fn a_document_issuer_that_differs_by_any_byte_is_refused() {
    for other in [
        "https://idp.maos.example/",
        "https://IDP.maos.example",
        "https://other.maos.example",
        "https://idp.maos.example ",
        "http://idp.maos.example",
    ] {
        let script = Arc::new(Script::default());
        script.serve(DOC_URL, &document(other, JWKS_URL));
        script.serve(JWKS_URL, fixtures::JWKS_KEY_A);
        let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
        assert_refused(&verifier, DiscoveryError::IssuerMismatch);
        assert_eq!(
            script.count(JWKS_URL),
            0,
            "keys of a mismatched issuer are never fetched: {other:?}"
        );
    }
}

/// AC2: an issuer that is not https, or carries userinfo, query or fragment,
/// never starts.
#[test]
fn an_issuer_that_is_not_plain_https_never_starts() {
    for (issuer, expected) in [
        ("http://idp.maos.example", DiscoveryError::IssuerNotHttps),
        ("idp.maos.example", DiscoveryError::IssuerNotHttps),
        ("ftp://idp.maos.example", DiscoveryError::IssuerNotHttps),
        (
            "https://user:pw@idp.maos.example",
            DiscoveryError::IssuerInvalid,
        ),
        (
            "https://idp.maos.example?x=1",
            DiscoveryError::IssuerInvalid,
        ),
        (
            "https://idp.maos.example#frag",
            DiscoveryError::IssuerInvalid,
        ),
        ("https://", DiscoveryError::IssuerInvalid),
    ] {
        let script = healthy_script();
        let result = DiscoveringVerifier::start(
            DiscoveryConfig::new(issuer),
            &[OidcAlgorithm::Rs256],
            &[issuer],
            fixtures::AUD_EXPECTED,
            script.clone() as Arc<dyn Fetcher>,
            ManualClock::new() as Arc<dyn Clock>,
        );
        assert_eq!(result.err(), Some(expected), "{issuer}");
        assert!(script.hits.lock().unwrap().is_empty(), "nothing is fetched");
    }
}

/// D-23-2-06 A (enforced in the crate too): the issuer must be a trusted one.
#[test]
fn an_issuer_outside_the_trusted_list_never_starts() {
    let result = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_BAD],
        fixtures::AUD_EXPECTED,
        healthy_script() as Arc<dyn Fetcher>,
        ManualClock::new() as Arc<dyn Clock>,
    );
    assert_eq!(result.err(), Some(DiscoveryError::IssuerNotTrusted));
}

/// Out-of-range tuning is a typed refusal, never a silent default.
#[test]
fn out_of_range_tuning_never_starts() {
    for config in [
        DiscoveryConfig::new(fixtures::ISS_GOOD).with_ttl_secs(0),
        DiscoveryConfig::new(fixtures::ISS_GOOD).with_min_refresh_secs(0),
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_ttl_secs(100)
            .with_min_refresh_secs(101),
        DiscoveryConfig::new(fixtures::ISS_GOOD).with_ttl_secs(7 * 24 * 3_600 + 1),
        DiscoveryConfig::new(fixtures::ISS_GOOD).with_idle_tick(Duration::ZERO),
    ] {
        let result = DiscoveringVerifier::start(
            config,
            &[OidcAlgorithm::Rs256],
            &[fixtures::ISS_GOOD],
            fixtures::AUD_EXPECTED,
            healthy_script() as Arc<dyn Fetcher>,
            ManualClock::new() as Arc<dyn Clock>,
        );
        assert_eq!(result.err(), Some(DiscoveryError::ConfigInvalid));
    }
}

/// AC2: an `http` `jwks_uri`, or one with userinfo, is refused before any
/// request to it.
#[test]
fn a_jwks_uri_that_is_not_plain_https_is_refused() {
    for (uri, expected) in [
        (
            "http://keys.maos.example/jwks",
            DiscoveryError::JwksUriNotHttps,
        ),
        (
            "ftp://keys.maos.example/jwks",
            DiscoveryError::JwksUriNotHttps,
        ),
        (
            "https://u:p@keys.maos.example/jwks",
            DiscoveryError::JwksUriInvalid,
        ),
        ("https:///jwks", DiscoveryError::JwksUriInvalid),
    ] {
        let script = Arc::new(Script::default());
        script.serve(DOC_URL, &document(fixtures::ISS_GOOD, uri));
        script.serve(uri, fixtures::JWKS_KEY_A);
        let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
        assert_refused(&verifier, expected);
        assert_eq!(script.count(uri), 0, "{uri}");
    }
}

type Routes = Arc<Mutex<HashMap<String, Result<Vec<u8>, IoError>>>>;

/// A policed port over canned routes; records every request that reached it.
#[derive(Clone, Default)]
struct FakePort {
    routes: Routes,
    reached: Arc<Mutex<Vec<String>>>,
}

impl IoSubsystemPort for FakePort {
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        self.reached.lock().unwrap().push(url.to_owned());
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(IoError::Transport("no route".to_owned())))
    }
    fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        Err(IoError::Transport("post unused".to_owned()))
    }
    fn bind(
        &self,
        _: &maos_domain::outbound::VettedUrl,
    ) -> Result<Box<dyn IoSubsystemPort>, IoError> {
        Ok(Box::new(self.clone()))
    }
    fn never_follows_redirects(&self) -> bool {
        true
    }
}

struct Dns(HashMap<&'static str, Vec<IpAddr>>);

impl ResolveHost for Dns {
    fn resolve(&self, host: &str, _port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        self.0.get(host).cloned().ok_or(ResolveFailed)
    }
}

fn policed(port: &FakePort) -> Arc<dyn Fetcher> {
    let ip = |text: &str| text.parse::<IpAddr>().unwrap();
    let dns = Dns(HashMap::from([
        ("idp.maos.example", vec![ip("93.184.216.34")]),
        ("keys.other.example", vec![ip("93.184.216.35")]),
        ("internal.maos.example", vec![ip("10.0.0.9")]),
        (
            "mixed.maos.example",
            vec![ip("93.184.216.36"), ip("169.254.169.254")],
        ),
    ]));
    let policed = PolicedIo::new(port.clone(), dns, OutboundPolicy::strict())
        .expect("the fake port declares no-follow");
    Arc::new(IoFetcher::new(policed))
}

fn ok(body: &str) -> Result<Vec<u8>, IoError> {
    Ok(body.as_bytes().to_vec())
}

/// AC2: a `jwks_uri` the 23-0 policy classes as loopback, link-local, private
/// or reserved (literal or by name, mixed answers included) is refused, and
/// the request never reaches the transport.
#[test]
fn a_jwks_uri_the_outbound_policy_refuses_is_never_requested() {
    for (uri, class) in [
        ("https://127.0.0.1/jwks", OutboundRefusal::Loopback),
        ("https://[::1]/jwks", OutboundRefusal::Loopback),
        ("https://169.254.169.254/jwks", OutboundRefusal::LinkLocal),
        ("https://10.0.0.5/jwks", OutboundRefusal::Private),
        ("https://192.168.1.1/jwks", OutboundRefusal::Private),
        (
            "https://internal.maos.example/jwks",
            OutboundRefusal::Private,
        ),
        (
            "https://mixed.maos.example/jwks",
            OutboundRefusal::LinkLocal,
        ),
        (
            "https://unknown.maos.example/jwks",
            OutboundRefusal::Unresolved,
        ),
    ] {
        let port = FakePort::default();
        port.routes
            .lock()
            .unwrap()
            .insert(DOC_URL.to_owned(), ok(&document(fixtures::ISS_GOOD, uri)));
        port.routes
            .lock()
            .unwrap()
            .insert(uri.to_owned(), ok(fixtures::JWKS_KEY_A));
        let verifier = start_with(policed(&port), &ManualClock::new(), LONG_WAIT);
        assert_refused(
            &verifier,
            DiscoveryError::JwksFetch(FetchError::Refused(class)),
        );
        assert_eq!(
            *port.reached.lock().unwrap(),
            vec![DOC_URL.to_owned()],
            "only the document reached the transport: {uri}"
        );
    }
}

/// AC2: a `jwks_uri` on another PUBLIC https host is accepted.
#[test]
fn a_jwks_uri_on_another_public_host_is_accepted() {
    let port = FakePort::default();
    let uri = "https://keys.other.example/jwks";
    port.routes
        .lock()
        .unwrap()
        .insert(DOC_URL.to_owned(), ok(&document(fixtures::ISS_GOOD, uri)));
    port.routes
        .lock()
        .unwrap()
        .insert(uri.to_owned(), ok(fixtures::JWKS_KEY_A));
    let verifier = start_with(policed(&port), &ManualClock::new(), LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
}

/// AC2: a 3xx (a non-2xx the kernel adapter refuses) from either URL loads no
/// key, and the error carries no URL.
#[test]
fn a_redirect_from_either_url_loads_no_key() {
    let redirect = || {
        Err(IoError::Transport(
            "ureq GET https://u:p@x/?q=1: status 302".to_owned(),
        ))
    };
    for refused_doc in [true, false] {
        let port = FakePort::default();
        let mut routes = port.routes.lock().unwrap();
        if refused_doc {
            routes.insert(DOC_URL.to_owned(), redirect());
        } else {
            routes.insert(
                DOC_URL.to_owned(),
                ok(&document(
                    fixtures::ISS_GOOD,
                    "https://keys.other.example/jwks",
                )),
            );
            routes.insert("https://keys.other.example/jwks".to_owned(), redirect());
        }
        drop(routes);
        let verifier = start_with(policed(&port), &ManualClock::new(), LONG_WAIT);
        let expected = if refused_doc {
            DiscoveryError::DocumentFetch(FetchError::Unavailable)
        } else {
            DiscoveryError::JwksFetch(FetchError::Unavailable)
        };
        assert_refused(&verifier, expected);
        let text = format!("{expected} {:?}", verifier.last_error());
        assert!(!text.contains("u:p") && !text.contains("q=1"), "{text}");
    }
}

/// AC2: discovered keys obey the algorithm allowlist: `alg: none` and HS256
/// are refused even though their `kid` is published.
#[test]
fn discovered_keys_obey_the_algorithm_allowlist() {
    let script = healthy_script();
    let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    for token in [fixtures::TOKEN_ALG_NONE, fixtures::TOKEN_HS256_CONFUSION] {
        assert_eq!(
            verifier.verify(token),
            Err(IdentityError::AlgorithmRejected)
        );
    }
    // And the shared claim rules still hold on a discovered key.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_WRONG_KEY),
        Err(IdentityError::SignatureInvalid)
    );
    assert_eq!(
        verifier.verify(fixtures::TOKEN_EXPIRED),
        Err(IdentityError::Expired)
    );
    assert_eq!(
        verifier.verify(fixtures::TOKEN_WRONG_AUD),
        Err(IdentityError::AudienceMismatch)
    );
    // A disallowed algorithm asked for no refresh.
    assert_eq!(script.count(DOC_URL), 1);
}

/// Limits: an oversized body, too many keys, a duplicate `kid`, a set with no
/// signing key: each refuses the refresh and loads nothing.
#[test]
fn oversized_or_ambiguous_key_sets_load_nothing() {
    let many_keys = {
        let key = fixtures::JWKS_KEY_A
            .split_once("\"keys\":[")
            .unwrap()
            .1
            .trim_end_matches("]}")
            .to_owned();
        let keys = (0..65)
            .map(|index| key.replace("key-a", &format!("k{index}")))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"keys\":[{keys}]}}")
    };
    let duplicate = {
        let key = fixtures::JWKS_KEY_A
            .split_once("\"keys\":[")
            .unwrap()
            .1
            .trim_end_matches("]}")
            .to_owned();
        format!("{{\"keys\":[{key},{key}]}}")
    };
    let oversized = format!("{{\"keys\":[],\"pad\":\"{}\"}}", "x".repeat(300 * 1024));
    let encryption_only = fixtures::JWKS_KEY_A.replace("\"use\":\"sig\"", "\"use\":\"enc\"");
    for (body, expected) in [
        (many_keys, DiscoveryError::TooLarge),
        (duplicate, DiscoveryError::JwksMalformed),
        (oversized, DiscoveryError::TooLarge),
        (encryption_only, DiscoveryError::NoUsableKeys),
        ("{\"keys\":[]}".to_owned(), DiscoveryError::NoUsableKeys),
        ("not json".to_owned(), DiscoveryError::JwksMalformed),
    ] {
        let script = Arc::new(Script::default());
        script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
        script.serve(JWKS_URL, &body);
        let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
        assert_refused(&verifier, expected);
    }
}

/// A refresh that fails keeps the previous keys until their TTL ends.
#[test]
fn a_failed_refresh_keeps_the_previous_keys_until_the_ttl() {
    let script = healthy_script();
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    script.fail(DOC_URL, FetchError::Unavailable);
    clock.set(100);
    // An unknown kid asks for a refresh; it fails; the cached key still serves.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert!(verifier.last_error().is_some());
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert!(verifier.is_healthy());
}

/// Tells the test when the key set has been served.
struct Signalling {
    inner: Arc<Script>,
    served: Mutex<std::sync::mpsc::Sender<()>>,
}

impl Fetcher for Signalling {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let answer = self.inner.get(url);
        if url == JWKS_URL && answer.is_ok() {
            let _ = self.served.lock().unwrap().send(());
        }
        answer
    }
}

/// D-23-2-08 A: an issuer that is down at boot is retried by the refresher
/// itself, at the minimum interval, with no request to prompt it. The wake-up
/// is real time (a few ms) but the decision reads the injected clock, and the
/// test waits on the fetch itself (a channel), never on a duration.
#[test]
fn an_idle_refresher_recovers_a_failed_boot_without_any_request() {
    let script = Arc::new(Script::default()); // nothing is served: the boot fetch fails
    let (served, arrived) = std::sync::mpsc::channel();
    let fetcher = Arc::new(Signalling {
        inner: Arc::clone(&script),
        served: Mutex::new(served),
    });
    let clock = ManualClock::new();
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(LONG_WAIT)
            .with_idle_tick(Duration::from_millis(5)),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        fetcher,
        Arc::clone(&clock) as Arc<dyn Clock>,
    )
    .unwrap();
    // Joins the failed boot fetch.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert!(!verifier.is_healthy());

    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, fixtures::JWKS_KEY_A);
    clock.set(31); // the limiter opens; nobody calls verify
    arrived
        .recv_timeout(GATE_SAFETY)
        .expect("the idle refresher fetched by itself");
    // Joins the completion of that fetch.
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert!(verifier.is_healthy());
}

/// The default allowlist is `RS256,ES256`. A mixed allowlist must still verify an RS256
/// assertion (the decode library refuses a validation that spans key families), and the static
/// verifier shares the fix.
#[test]
fn a_mixed_algorithm_allowlist_verifies_an_rs256_assertion() {
    let script = healthy_script();
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(LONG_WAIT)
            .with_idle_tick(Duration::from_secs(3_600)),
        &[OidcAlgorithm::Rs256, OidcAlgorithm::Es256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        script as Arc<dyn Fetcher>,
        ManualClock::new() as Arc<dyn Clock>,
    )
    .unwrap();
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    let static_verifier = maos_sso::OidcVerifier::from_static_jwks(
        fixtures::JWKS_KEY_A,
        &[OidcAlgorithm::Rs256, OidcAlgorithm::Es256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
    )
    .unwrap();
    assert!(static_verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert_eq!(
        static_verifier.verify(fixtures::TOKEN_HS256_CONFUSION),
        Err(IdentityError::AlgorithmRejected)
    );
}

/// F1: discovered keys vouch for the discovery issuer only. A second listed issuer is refused
/// at start, and a token signed by the discovered key but claiming another issuer is refused.
#[test]
fn discovered_keys_are_bound_to_the_discovery_issuer() {
    let two = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD, fixtures::ISS_BAD],
        fixtures::AUD_EXPECTED,
        healthy_script() as Arc<dyn Fetcher>,
        ManualClock::new() as Arc<dyn Clock>,
    );
    assert_eq!(two.err(), Some(DiscoveryError::IssuersNotExclusive));
    let verifier = start(&healthy_script(), &ManualClock::new(), LONG_WAIT);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_BAD_ISS),
        Err(IdentityError::IssuerUntrusted),
        "signed by the discovered key, iss = another issuer"
    );
}

/// Whole-test wait budget for an event that must arrive while a fetch is still held.
const EVENT_BUDGET: Duration = Duration::from_secs(10);

/// Poll `condition` until it holds (the wait is on an event, not a duration: the budget is only
/// the point where a missing event fails the test).
fn eventually(what: &str, condition: impl Fn() -> bool) {
    let deadline = std::time::Instant::now() + GATE_SAFETY;
    while !condition() {
        assert!(
            std::time::Instant::now() < deadline,
            "never happened: {what}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// F3: a well-formed key set with no usable key retires the cached keys (the verifier turns
/// unhealthy at once); a malformed or failed refresh keeps them until the TTL.
#[test]
fn an_empty_key_set_retires_the_cached_keys() {
    let script = healthy_script();
    let clock = ManualClock::new();
    let verifier = start(&script, &clock, LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());

    // A refresh that fails or is malformed keeps the populated cache.
    script.serve(JWKS_URL, "not json");
    clock.set(31);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(verifier.last_error(), Some(DiscoveryError::JwksMalformed));
    assert!(verifier.is_healthy());
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());

    // The issuer then publishes a valid, empty set: every cached key is retired.
    script.serve(JWKS_URL, "{\"keys\":[]}");
    clock.set(62);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(verifier.last_error(), Some(DiscoveryError::NoUsableKeys));
    assert!(!verifier.is_healthy(), "no key survives an empty key set");
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable),
        "a key the issuer withdrew is not served from the cache"
    );

    // Service resumes when the issuer publishes keys again.
    script.serve(JWKS_URL, fixtures::JWKS_KEY_A);
    clock.set(93);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    assert!(verifier.is_healthy());
}

/// The three ways a set can hold keys and still hold none that may verify (F11 feeds F3).
fn jwks_key_ops_sign_only() -> String {
    fixtures::JWKS_KEY_A.replace(
        "\"use\":\"sig\",",
        "\"use\":\"sig\",\"key_ops\":[\"sign\"],",
    )
}

/// F11: a key whose `key_ops` excludes `verify`, an `EC` key off `P-256` or without a curve,
/// and an entry declaring one key family while carrying the other's material load nothing.
#[test]
fn keys_that_may_not_verify_or_declare_the_wrong_family_load_nothing() {
    let ec_with_rsa_material = fixtures::JWKS_KEY_A.replace("\"kty\":\"RSA\"", "\"kty\":\"EC\"");
    let ec_p384 = JWKS_KEY_ES256.replace("\"crv\":\"P-256\"", "\"crv\":\"P-384\"");
    let ec_no_curve = JWKS_KEY_ES256.replace("\"crv\":\"P-256\",", "");
    for body in [
        jwks_key_ops_sign_only(),
        ec_with_rsa_material,
        ec_p384,
        ec_no_curve,
    ] {
        let script = Arc::new(Script::default());
        script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
        script.serve(JWKS_URL, &body);
        let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
        assert_refused(&verifier, DiscoveryError::NoUsableKeys);
    }
    // A key that declares `verify` among other operations is still a verification key.
    let both = fixtures::JWKS_KEY_A.replace(
        "\"use\":\"sig\",",
        "\"use\":\"sig\",\"key_ops\":[\"sign\",\"verify\"],",
    );
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, &both);
    let verifier = start(&script, &ManualClock::new(), LONG_WAIT);
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
}

/// F11 at the verifier (the static source shares it): a key is used only under the algorithm
/// its declared `kty`/`crv` fits, and one that excludes `verify` is never used.
#[test]
fn a_key_is_used_only_under_the_algorithm_its_family_fits() {
    let verify_with = |jwks: &str, algorithms: &[OidcAlgorithm], token: &str| {
        maos_sso::OidcVerifier::from_static_jwks(
            jwks,
            algorithms,
            &[fixtures::ISS_GOOD],
            fixtures::AUD_EXPECTED,
        )
        .unwrap()
        .verify(token)
    };
    let rs = [OidcAlgorithm::Rs256];
    let es = [OidcAlgorithm::Es256];
    // The honest keys verify.
    assert!(verify_with(fixtures::JWKS_KEY_A, &rs, fixtures::TOKEN_GOOD_RS256).is_ok());
    assert!(verify_with(JWKS_KEY_ES256, &es, TOKEN_ES256).is_ok());
    // RSA material inside an entry that declares `EC`: not read for RS256.
    let ec_declared = fixtures::JWKS_KEY_A.replace("\"kty\":\"RSA\"", "\"kty\":\"EC\"");
    assert_eq!(
        verify_with(&ec_declared, &rs, fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::AlgorithmRejected)
    );
    // EC material inside an entry that declares `RSA`: not read for ES256.
    let rsa_declared = JWKS_KEY_ES256.replace("\"kty\":\"EC\"", "\"kty\":\"RSA\"");
    assert_eq!(
        verify_with(&rsa_declared, &es, TOKEN_ES256),
        Err(IdentityError::AlgorithmRejected)
    );
    // An EC key off P-256, or without a curve, is not an ES256 key.
    for bad in [
        JWKS_KEY_ES256.replace("\"crv\":\"P-256\"", "\"crv\":\"P-384\""),
        JWKS_KEY_ES256.replace("\"crv\":\"P-256\",", ""),
    ] {
        assert_eq!(
            verify_with(&bad, &es, TOKEN_ES256),
            Err(IdentityError::AlgorithmRejected)
        );
    }
    // A key whose operations exclude `verify` does not verify.
    assert_eq!(
        verify_with(&jwks_key_ops_sign_only(), &rs, fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
}

/// F16b: an ES256 assertion verifies through discovery (the `from_ec_components` path), also
/// beside RS256 in a mixed allowlist, and a tampered one does not.
#[test]
fn an_es256_assertion_verifies_through_discovery() {
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, JWKS_KEY_ES256);
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(LONG_WAIT)
            .with_idle_tick(Duration::from_secs(3_600)),
        &[OidcAlgorithm::Rs256, OidcAlgorithm::Es256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        script as Arc<dyn Fetcher>,
        ManualClock::new() as Arc<dyn Clock>,
    )
    .unwrap();
    let principal = verifier
        .verify(TOKEN_ES256)
        .expect("an ES256 assertion signed by the published P-256 key verifies");
    assert_eq!(principal.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(principal.issuer, fixtures::ISS_GOOD);
    // One flipped signature byte is refused.
    let tampered = {
        let mut text = TOKEN_ES256.to_owned();
        // Inside the signature (the last character carries unused bits).
        let index = text.len() - 10;
        let original = text.remove(index);
        text.insert(index, if original == 'A' { 'B' } else { 'A' });
        text
    };
    assert_eq!(
        verifier.verify(&tampered),
        Err(IdentityError::SignatureInvalid)
    );
    // An RS256 assertion cannot be checked against an EC-only key set.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
}

/// F9: the limiter runs from the moment the previous fetch ACTUALLY began, at millisecond
/// resolution: a fetch that began at 0.9 s is not 30 s old at 30.0 s.
#[test]
fn the_limiter_measures_from_the_actual_fetch_start_at_millisecond_resolution() {
    let script = Arc::new(Script::default());
    script.serve(DOC_URL, &document(fixtures::ISS_GOOD, JWKS_URL));
    script.serve(JWKS_URL, &old_keys());
    let clock = ManualClock::new();
    clock.set_millis(900);
    let verifier = start(&script, &clock, LONG_WAIT);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable),
        "joins the boot fetch, which began at 0.9 s"
    );
    assert_eq!(script.count(DOC_URL), 1);

    clock.set_millis(30_000);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(
        script.count(DOC_URL),
        1,
        "29.1 s after the last fetch began is inside the 30 s interval"
    );
    clock.set_millis(30_899);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(script.count(DOC_URL), 1, "29.999 s is still inside it");
    clock.set_millis(30_900);
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(script.count(DOC_URL), 2, "30.0 s after it began: one fetch");
}

/// F10: the TTL starts when the fetch COMPLETES, not when it began.
#[test]
fn the_ttl_starts_when_the_fetch_completes() {
    let script = healthy_script();
    let gate = Arc::new(Gate::default());
    *script.gate_on_document.lock().unwrap() = Some(Arc::clone(&gate));
    let clock = ManualClock::new();
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_ttl_secs(100)
            .with_verify_wait(LONG_WAIT)
            .with_idle_tick(Duration::from_secs(3_600)),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        script as Arc<dyn Fetcher>,
        Arc::clone(&clock) as Arc<dyn Clock>,
    )
    .unwrap();
    gate.wait_entered();
    // The fetch began at 0 and finishes at 20 (inside the 30 s fetch bound).
    clock.set(20);
    gate.release();
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());
    clock.set(119);
    assert!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok(),
        "99 s after the keys arrived, inside the 100 s TTL"
    );
    assert!(verifier.is_healthy());
    clock.set(120);
    assert!(!verifier.is_healthy(), "100 s after the keys arrived");
}

/// A fetcher that never answers the document while `hang` is set, and announces every entry.
struct Hung {
    inner: Arc<Script>,
    hang: std::sync::atomic::AtomicBool,
    gate: Gate,
    entered: Mutex<std::sync::mpsc::Sender<()>>,
    released_keys_served: Mutex<std::sync::mpsc::Sender<()>>,
}

impl Fetcher for Hung {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        if url == DOC_URL {
            let _ = self.entered.lock().unwrap().send(());
            if self.hang.load(Ordering::SeqCst) {
                // Held until the test releases it, then it answers like any other fetch.
                self.gate.pass();
                let answer = self.inner.get(url);
                return answer;
            }
        }
        let answer = self.inner.get(url);
        if url == JWKS_URL && self.gate.released() {
            let _ = self.released_keys_served.lock().unwrap().send(());
        }
        answer
    }
}

fn hung_fetcher(
    script: &Arc<Script>,
) -> (
    Arc<Hung>,
    std::sync::mpsc::Receiver<()>,
    std::sync::mpsc::Receiver<()>,
) {
    let (entered, entries) = std::sync::mpsc::channel();
    let (late, late_served) = std::sync::mpsc::channel();
    let hung = Arc::new(Hung {
        inner: Arc::clone(script),
        hang: std::sync::atomic::AtomicBool::new(true),
        gate: Gate::default(),
        entered: Mutex::new(entered),
        released_keys_served: Mutex::new(late),
    });
    (hung, entries, late_served)
}

fn start_hung(hung: &Arc<Hung>, clock: &Arc<ManualClock>) -> DiscoveringVerifier {
    DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(Duration::from_millis(50))
            .with_idle_tick(Duration::from_millis(5)),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        Arc::clone(hung) as Arc<dyn Fetcher>,
        Arc::clone(clock) as Arc<dyn Clock>,
    )
    .unwrap()
}

/// F6: a fetch that never answers does not leave discovery down: after the fetch bound the
/// refresher abandons it, the next refresh runs on a fresh worker and service resumes; the
/// abandoned worker's late answer changes nothing.
#[test]
fn a_fetch_that_never_answers_is_abandoned_and_discovery_recovers() {
    let script = healthy_script();
    let (hung, entries, late_served) = hung_fetcher(&script);
    let clock = ManualClock::new();
    let verifier = start_hung(&hung, &clock);
    entries
        .recv_timeout(EVENT_BUDGET)
        .expect("boot fetch entered");
    // The boot fetch never answers: callers fail closed after their bounded wait.
    assert_eq!(
        verifier.verify(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert!(!verifier.is_healthy());

    // Inside the fetch bound (30 s) it is still waited for: no second fetch.
    clock.set(29);
    assert!(
        entries.recv_timeout(Duration::from_millis(100)).is_err(),
        "a fetch younger than the bound is not abandoned"
    );
    // Past the bound and the refresh interval: abandoned, replacement starts, and the issuer
    // (now answering) restores service.
    hung.hang.store(false, Ordering::SeqCst);
    clock.set(31);
    entries
        .recv_timeout(EVENT_BUDGET)
        .expect("a replacement fetch started after the bound");
    eventually("keys installed by the replacement", || {
        verifier.is_healthy()
    });
    assert!(verifier.verify(fixtures::TOKEN_GOOD_RS256).is_ok());

    // The abandoned worker answers late (with keys): the cache is not touched, and no
    // further refresh result is produced by it.
    let refreshes = verifier.completed_refreshes();
    hung.gate.release();
    late_served
        .recv_timeout(EVENT_BUDGET)
        .expect("the abandoned worker finished");
    assert_eq!(verifier.completed_refreshes(), refreshes);
    assert!(verifier.is_healthy());
}

/// F6 limit: at most ONE abandoned worker exists at a time. While the first is still stuck, a
/// second hung fetch is waited for, not abandoned; once the first returns, the second can be.
#[test]
fn at_most_one_abandoned_fetch_worker_exists_at_a_time() {
    let script = healthy_script();
    let (hung, entries, _late) = hung_fetcher(&script);
    let clock = ManualClock::new();
    let verifier = start_hung(&hung, &clock);
    entries
        .recv_timeout(EVENT_BUDGET)
        .expect("fetch #1 entered");

    clock.set(31); // fetch #1 abandoned, fetch #2 starts (and hangs too)
    entries
        .recv_timeout(EVENT_BUDGET)
        .expect("fetch #2 entered");
    clock.set(62); // fetch #2 is past the bound but #1 is still stuck: not abandoned
    assert!(
        entries.recv_timeout(Duration::from_millis(300)).is_err(),
        "no third fetch while an abandoned worker is still stuck"
    );
    assert_eq!(verifier.completed_refreshes(), 1);

    // Both stuck workers return: the live one (#2) completes normally and service resumes.
    hung.hang.store(false, Ordering::SeqCst);
    hung.gate.release();
    eventually("fetch #2 installs the keys", || verifier.is_healthy());
    assert_eq!(verifier.completed_refreshes(), 2);
}
