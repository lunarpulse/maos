#![cfg(all(feature = "discovery", not(feature = "sso-fault-inject")))]

//! Story 23-2 (round 4, P1-1) - the freshness proof a discovering verification carries and the
//! non-waiting recheck an issuance point runs before it mints.
//!
//! A verification returns the `kid` it used, the publication stamp of the key set that held it and
//! that set's hard-TTL expiry. `key_still_fresh` is true only while that very set is still the
//! published one, unexpired on the verifier's own clock, and still holds the `kid`.
//!
//! Hermetic: an injected `Fetcher` answering at once and a manual clock. The refresher publishes on
//! its own thread; every wait is a counter with a safety bound, never a sleep used as an oracle.

#[path = "fixtures.rs"]
mod fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use maos_domain::ports::identity_assertion::{KeyFreshness, KeyFreshnessIssuer};
use maos_domain::ports::IdentityAssertionPort;
use maos_sso::discovery::{Clock, DiscoveringVerifier, DiscoveryConfig, FetchError, Fetcher};
use maos_sso::OidcAlgorithm;

const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";
/// A fixture that waits this long fails closed.
const SAFETY: Duration = Duration::from_secs(20);
/// The default key lifetime, in milliseconds.
const TTL_MS: u64 = 3_600_000;

struct ManualClock(AtomicU64);

impl ManualClock {
    fn set_millis(&self, millis: u64) {
        self.0.store(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Vec<u8>>>,
    gets: AtomicU64,
}

impl Script {
    fn serve(&self, url: &str, body: &str) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), body.as_bytes().to_vec());
    }

    /// Requests the fetcher has answered or refused so far.
    fn gets(&self) -> u64 {
        self.gets.load(Ordering::SeqCst)
    }
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.gets.fetch_add(1, Ordering::SeqCst);
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
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

fn await_refreshes(verifier: &DiscoveringVerifier, count: u64) {
    let until = Instant::now() + SAFETY;
    while verifier.completed_refreshes() < count {
        assert!(Instant::now() < until, "the refresher never finished");
        std::thread::yield_now();
    }
}

/// A verification proof from the cached keys, no wait.
fn proof(verifier: &DiscoveringVerifier) -> KeyFreshness {
    verifier
        .verify_with_freshness(fixtures::TOKEN_GOOD_RS256, false)
        .expect("a cached key verifies")
        .1
        .expect("a discovering verification carries a proof")
}

/// The proof names the `kid`, the key set and the TTL expiry, on both lookups (waiting and not),
/// and holds while nothing changes.
#[test]
fn a_verification_carries_the_key_the_set_and_the_expiry() {
    let (verifier, clock, _script) = started();
    let cached = proof(&verifier);
    assert_eq!(cached.kid(), "key-a");
    assert_eq!(cached.expires_at_ms(), TTL_MS, "fetched at 0, default TTL");
    let waiting = verifier
        .verify_with_freshness(fixtures::TOKEN_GOOD_RS256, true)
        .unwrap()
        .1
        .unwrap();
    assert_eq!(waiting, cached, "both lookups read the same published set");
    assert!(verifier.key_still_fresh(&cached));
    clock.set_millis(TTL_MS - 1);
    assert!(
        verifier.key_still_fresh(&cached),
        "1 ms before the hard TTL the proof still holds"
    );
}

/// The hard TTL lapses after verification: the proof no longer holds, exactly at the expiry.
#[test]
fn a_proof_does_not_hold_once_the_key_set_expired() {
    let (verifier, clock, _script) = started();
    let cached = proof(&verifier);
    clock.set_millis(TTL_MS);
    assert!(!verifier.is_healthy(), "the set is past its TTL");
    assert!(!verifier.key_still_fresh(&cached));
}

/// The `kid` is retired or replaced after verification: a set published without it refuses the
/// old proof.
#[test]
fn a_proof_does_not_hold_once_its_kid_was_retired() {
    let (verifier, clock, script) = started();
    let cached = proof(&verifier);
    script.serve(JWKS_URL, &fixtures::JWKS_KEY_A.replace("key-a", "key-z"));
    clock.set_millis(31_000);
    verifier.request_refresh();
    await_refreshes(&verifier, 2);
    assert!(!verifier.key_still_fresh(&cached));
}

/// A set published since verification refuses the proof even when it still holds the `kid` (the
/// caller re-verifies against the new set): the stamp is the publication, not the `kid`.
#[test]
fn a_proof_does_not_hold_once_another_key_set_was_published() {
    let (verifier, clock, _script) = started();
    let old = proof(&verifier);
    clock.set_millis(31_000);
    verifier.request_refresh();
    await_refreshes(&verifier, 2);
    assert!(
        !verifier.key_still_fresh(&old),
        "the same kid in a newer set is a different publication"
    );
    let fresh = proof(&verifier);
    assert_ne!(fresh.key_set(), old.key_set());
    assert!(verifier.key_still_fresh(&fresh), "a new proof holds");
}

/// The recheck trusts the verifier's own identity and state, not claims: the type is opaque, so
/// the same claims (or a later expiry) made by another issuer are refused, and the real proof
/// does not outlive its set's TTL.
#[test]
fn a_proof_cannot_be_assembled_from_its_claims() {
    let (verifier, clock, _script) = started();
    let real = proof(&verifier);
    // Every claim the real proof carries, minted by another issuer: refused at every clock.
    let other = KeyFreshnessIssuer::new();
    let same = other.issue(real.kid(), real.key_set(), real.expires_at_ms());
    assert!(
        !verifier.key_still_fresh(&same),
        "same claims, other issuer"
    );
    let later = other.issue(real.kid(), real.key_set(), u64::MAX);
    clock.set_millis(TTL_MS);
    assert!(!verifier.key_still_fresh(&later), "a later expiry");
    assert!(
        !verifier.key_still_fresh(&real),
        "and the real one has expired"
    );
}

/// A refusal fires the non-blocking nudge (so a retry can succeed) and never waits: the verifier
/// started with a one-minute `verify_wait` returns at once.
#[test]
fn a_refused_recheck_nudges_the_refresher_and_never_waits() {
    let (verifier, clock, _script) = started();
    let cached = proof(&verifier);
    clock.set_millis(TTL_MS);
    let before = verifier.completed_refreshes();
    let waits = verifier.refresh_waits();
    assert!(!verifier.key_still_fresh(&cached));
    await_refreshes(&verifier, before + 1);
    assert!(verifier.is_healthy(), "the nudged refresh republished");
    assert_eq!(verifier.refresh_waits(), waits, "the recheck never waited");
}

/// Round 5 (P2-1): a proof is bound to the verifier that made it. Two verifiers with the same
/// `kid` and each at its first publication stamp: B refuses A's genuine proof (and A refuses
/// B's), while each accepts its own.
#[test]
fn a_genuine_proof_from_another_verifier_is_refused() {
    let (a, _clock_a, _script_a) = started();
    let (b, _clock_b, _script_b) = started();
    let from_a = proof(&a);
    let from_b = proof(&b);
    assert_eq!(from_a.kid(), from_b.kid(), "the same kid");
    assert_eq!(from_a.key_set(), from_b.key_set(), "the same first stamp");
    assert_eq!(from_a.expires_at_ms(), from_b.expires_at_ms());
    assert!(a.key_still_fresh(&from_a) && b.key_still_fresh(&from_b));
    assert!(!b.key_still_fresh(&from_a), "B refuses A's proof");
    assert!(!a.key_still_fresh(&from_b), "A refuses B's proof");
}

/// Round 5 (P2-1): the type is opaque (no field is public), so a tuple that matches every
/// observable of the real proof can only come from another issuer, and is refused. The same
/// goes for a proof whose tuple the verifier's own set would otherwise accept.
#[test]
fn a_counterfeit_proof_matching_every_observable_is_refused() {
    let (verifier, _clock, _script) = started();
    let real = proof(&verifier);
    let counterfeit =
        KeyFreshnessIssuer::new().issue(real.kid(), real.key_set(), real.expires_at_ms());
    assert_eq!(counterfeit.kid(), real.kid());
    assert_eq!(counterfeit.key_set(), real.key_set());
    assert_eq!(counterfeit.expires_at_ms(), real.expires_at_ms());
    assert!(!verifier.key_still_fresh(&counterfeit));
    assert!(
        verifier.key_still_fresh(&real),
        "the real proof still holds"
    );
}

/// Round 5 (P2-2): a refusal that is ONLY an epoch supersession of a healthy set (the `kid` is
/// still published, the set unexpired) schedules no refresh, even with the limiter open: the
/// caller re-verifies against the already-published set. A retired `kid` still nudges (the
/// positive control also proves the limiter was open for the first refusal).
#[test]
fn an_epoch_only_refusal_schedules_no_refresh_but_a_retired_kid_does() {
    let (verifier, clock, script) = started();
    let old = proof(&verifier);
    clock.set_millis(31_000);
    verifier.request_refresh();
    await_refreshes(&verifier, 2);
    // The limiter (30 s) has reopened: a nudge now WOULD be accepted.
    clock.set_millis(62_000);
    let nudges = verifier.refresh_nudges();
    let fetches = script.gets();
    assert!(!verifier.key_still_fresh(&old), "a newer set was published");
    assert_eq!(
        verifier.refresh_nudges(),
        nudges,
        "an epoch-only refusal asks for no refresh"
    );
    assert_eq!(script.gets(), fetches, "and fetches nothing");
    let fresh = proof(&verifier);
    assert!(verifier.key_still_fresh(&fresh), "re-verified, it holds");

    // Control: the kid is retired by the next publication; the refusal nudges.
    script.serve(JWKS_URL, &fixtures::JWKS_KEY_A.replace("key-a", "key-z"));
    verifier.request_refresh();
    await_refreshes(&verifier, 3);
    clock.set_millis(93_000);
    let nudges = verifier.refresh_nudges();
    assert!(!verifier.key_still_fresh(&fresh));
    assert_eq!(
        verifier.refresh_nudges(),
        nudges + 1,
        "a retired kid asks for a refresh"
    );
    await_refreshes(&verifier, 4);
}
