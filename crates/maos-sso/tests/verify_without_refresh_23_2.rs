#![cfg(all(feature = "discovery", not(feature = "sso-fault-inject")))]

//! Story 23-2 review round 3 (F4 revert) - `IdentityAssertionPort::verify_without_refresh` on the
//! discovering verifier answers from the fresh cached keys and NEVER waits: a lock-holding
//! commit point (the cohort digest-admission guard) can verify without waiting on the identity
//! provider. A refusal for want of a key (unknown `kid`, stale set) fires a NON-BLOCKING refresh
//! nudge (the refresher runs it), so a retry after the refresher published succeeds.
//!
//! Hermetic: an injected `Fetcher` that serves canned answers behind a gate (so the refresher can
//! be held "never answering") and an injected clock.

#[path = "fixtures.rs"]
mod fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use maos_domain::ports::{IdentityAssertionPort, IdentityError};
use maos_sso::discovery::{Clock, DiscoveringVerifier, DiscoveryConfig, FetchError, Fetcher};
use maos_sso::OidcAlgorithm;

const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";
/// A fixture that waits this long fails closed.
const SAFETY: Duration = Duration::from_secs(20);

struct ManualClock(AtomicU64);

impl Clock for ManualClock {
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
}

#[derive(Default)]
struct GateState {
    closed: bool,
    documents: usize,
}

/// Canned answers; a closed gate holds every fetch (the refresher "never answers") and a fetch
/// that waits `SAFETY` fails closed.
#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Vec<u8>>>,
    gate: Gate,
}

impl Script {
    fn serve(&self, url: &str, body: &str) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), body.as_bytes().to_vec());
    }
    fn close(&self) {
        self.gate.state.lock().unwrap().closed = true;
    }
    fn open(&self) {
        self.gate.state.lock().unwrap().closed = false;
        self.gate.changed.notify_all();
    }
    fn documents(&self) -> usize {
        self.gate.state.lock().unwrap().documents
    }
    /// Blocks until `count` discovery documents were requested; fails at the safety bound.
    fn await_documents(&self, count: usize) {
        let state = self
            .gate
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (state, result) = self
            .gate
            .changed
            .wait_timeout_while(state, SAFETY, |state| state.documents < count)
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            !result.timed_out(),
            "no refresh was requested ({} document fetches)",
            state.documents
        );
    }
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        {
            let mut state = self
                .gate
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if url == DOC_URL {
                state.documents += 1;
                self.gate.changed.notify_all();
            }
            let (_state, result) = self
                .gate
                .changed
                .wait_timeout_while(state, SAFETY, |state| state.closed)
                .unwrap_or_else(PoisonError::into_inner);
            if result.timed_out() {
                return Err(FetchError::Unavailable);
            }
        }
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

fn rotated_keys() -> String {
    // keyA's material under the `kid` TOKEN_UNKNOWN_KID names, beside its own.
    let own = fixtures::JWKS_KEY_A
        .split_once("\"keys\":[")
        .unwrap()
        .1
        .trim_end_matches("]}");
    format!(
        "{{\"keys\":[{own},{}]}}",
        own.replace("key-a", "missing-key")
    )
}

fn started() -> (DiscoveringVerifier, Arc<ManualClock>, Arc<Script>) {
    let script = Arc::new(Script::default());
    script.serve(
        DOC_URL,
        &format!(
            "{{\"issuer\":\"{}\",\"jwks_uri\":\"{JWKS_URL}\"}}",
            fixtures::ISS_GOOD
        ),
    );
    script.serve(JWKS_URL, fixtures::JWKS_KEY_A);
    let clock = Arc::new(ManualClock(AtomicU64::new(0)));
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(Duration::from_secs(60))
            .with_idle_tick(Duration::from_secs(3_600)),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        Arc::clone(&script) as Arc<dyn Fetcher>,
        Arc::clone(&clock) as Arc<dyn Clock>,
    )
    .expect("a valid discovery configuration starts");
    // Joins the boot fetch and proves the key set is loaded.
    verifier
        .verify(fixtures::TOKEN_GOOD_RS256)
        .expect("the published key verifies");
    (verifier, clock, script)
}

/// Blocks until the verifier finished `count` refresh attempts (the refresher publishes on its
/// own thread); fails at the safety bound.
fn await_refreshes(verifier: &DiscoveringVerifier, count: u64) {
    let until = Instant::now() + SAFETY;
    while verifier.completed_refreshes() < count {
        assert!(Instant::now() < until, "the refresher never finished");
        std::thread::yield_now();
    }
}

/// A cached key re-verifies, with no refresh and no wait.
#[test]
fn a_cached_key_verifies_without_a_refresh() {
    let (verifier, _clock, script) = started();
    let waits = verifier.refresh_waits();
    assert!(verifier
        .verify_without_refresh(fixtures::TOKEN_GOOD_RS256)
        .is_ok());
    assert_eq!(verifier.refresh_waits(), waits);
    assert_eq!(script.documents(), 1, "a hit never asks for a refresh");
}

/// Round 3 (a): an unknown `kid` is refused at once while the refresher never answers (the call
/// returns with the fetch held), fires exactly one non-blocking refresh, and a retry after the
/// refresher published the rotated key set succeeds.
#[test]
fn an_unknown_kid_is_refused_without_waiting_nudges_one_refresh_and_a_retry_succeeds() {
    let (verifier, clock, script) = started();
    script.serve(JWKS_URL, &rotated_keys());
    script.close();
    clock.0.store(31_000, Ordering::SeqCst); // past the 30 s limiter
    let waits = verifier.refresh_waits();

    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(verifier.refresh_waits(), waits, "the call must not wait");
    // The nudge started a refresh (held at the gate); a second refusal while it is pending
    // starts no second one.
    script.await_documents(2);
    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(script.documents(), 2, "one nudge, not one per refusal");

    script.open();
    await_refreshes(&verifier, 2);
    let principal = verifier
        .verify_without_refresh(fixtures::TOKEN_UNKNOWN_KID)
        .expect("the retry after the refresher published succeeds");
    assert_eq!(principal.subject, fixtures::PRINCIPAL_SUB);
    assert_eq!(script.documents(), 2, "the retry fetched nothing itself");
}

/// Round 3 (b): past the TTL the cached key is not served, with no wait; the refusal nudges a
/// refresh and, once the refresher published, the key verifies again.
#[test]
fn an_expired_key_set_refuses_without_waiting_and_recovers_after_the_refresh() {
    let (verifier, clock, script) = started();
    clock.0.store(3_599_999, Ordering::SeqCst);
    assert!(verifier
        .verify_without_refresh(fixtures::TOKEN_GOOD_RS256)
        .is_ok());
    assert_eq!(script.documents(), 1, "a fresh hit asks for nothing");

    script.close();
    clock.0.store(3_600_000, Ordering::SeqCst);
    let waits = verifier.refresh_waits();
    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(verifier.refresh_waits(), waits);
    script.await_documents(2);

    script.open();
    await_refreshes(&verifier, 2);
    assert!(verifier
        .verify_without_refresh(fixtures::TOKEN_GOOD_RS256)
        .is_ok());
}

/// The nudge honours the minimum refresh interval: inside it, a refusal asks for nothing. Seen
/// without a clock: had the refusal set the pending bit, a waiting `verify` for the same unknown
/// key would now join that refresh and count a wait; with the limiter closed and nothing pending
/// it is refused at once.
#[test]
fn a_refusal_inside_the_refresh_interval_asks_for_nothing() {
    let (verifier, clock, script) = started();
    script.close();
    clock.0.store(1_000, Ordering::SeqCst); // 1 s after the boot fetch: inside the 30 s limiter
    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    let waits = verifier.refresh_waits();
    assert_eq!(
        verifier.verify(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(
        verifier.refresh_waits(),
        waits,
        "nothing was pending, so the waiting verify did not wait"
    );
    assert_eq!(script.documents(), 1);
}

/// `request_refresh` is the same nudge, callable on its own, and never waits.
#[test]
fn request_refresh_starts_one_refresh_and_does_not_wait() {
    let (verifier, clock, script) = started();
    script.close();
    clock.0.store(31_000, Ordering::SeqCst);
    verifier.request_refresh();
    verifier.request_refresh();
    script.await_documents(2);
    assert_eq!(
        script.documents(),
        2,
        "pending: the second call starts nothing"
    );
    script.open();
    await_refreshes(&verifier, 2);
}
