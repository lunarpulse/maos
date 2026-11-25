#![cfg(all(feature = "discovery", not(feature = "sso-fault-inject")))]

//! Story 23-2 review round 2 (P1-2) - `IdentityAssertionPort::verify_without_refresh` on the
//! discovering verifier answers from the fresh cached keys and NEVER asks the refresher, so a
//! lock-holding commit point can re-check a principal without waiting on the identity provider.
//!
//! Hermetic: an injected `Fetcher` that serves canned answers and an injected clock.

#[path = "fixtures.rs"]
mod fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_domain::ports::{IdentityAssertionPort, IdentityError};
use maos_sso::discovery::{Clock, DiscoveringVerifier, DiscoveryConfig, FetchError, Fetcher};
use maos_sso::OidcAlgorithm;

const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";

struct ManualClock(AtomicU64);

impl Clock for ManualClock {
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Vec<u8>>>,
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

fn started() -> (DiscoveringVerifier, Arc<ManualClock>) {
    let script = Arc::new(Script::default());
    script.routes.lock().unwrap().insert(
        DOC_URL.to_owned(),
        format!(
            "{{\"issuer\":\"{}\",\"jwks_uri\":\"{JWKS_URL}\"}}",
            fixtures::ISS_GOOD
        )
        .into_bytes(),
    );
    script.routes.lock().unwrap().insert(
        JWKS_URL.to_owned(),
        fixtures::JWKS_KEY_A.as_bytes().to_vec(),
    );
    let clock = Arc::new(ManualClock(AtomicU64::new(0)));
    let verifier = DiscoveringVerifier::start(
        DiscoveryConfig::new(fixtures::ISS_GOOD)
            .with_verify_wait(Duration::from_secs(60))
            .with_idle_tick(Duration::from_secs(3_600)),
        &[OidcAlgorithm::Rs256],
        &[fixtures::ISS_GOOD],
        fixtures::AUD_EXPECTED,
        script,
        Arc::clone(&clock) as Arc<dyn Clock>,
    )
    .expect("a valid discovery configuration starts");
    // Joins the boot fetch and proves the key set is loaded.
    verifier
        .verify(fixtures::TOKEN_GOOD_RS256)
        .expect("the published key verifies");
    (verifier, clock)
}

/// A cached key re-verifies, and an unknown `kid` is refused at once even when the limiter is
/// open (a plain `verify` would start a refresh and wait for it).
#[test]
fn an_unknown_kid_is_refused_without_asking_the_refresher() {
    let (verifier, clock) = started();
    assert!(verifier
        .verify_without_refresh(fixtures::TOKEN_GOOD_RS256)
        .is_ok());

    clock.0.store(31_000, Ordering::SeqCst); // past the 30 s limiter
    let waits = verifier.refresh_waits();
    let refreshes = verifier.completed_refreshes();
    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_UNKNOWN_KID),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(
        verifier.refresh_waits(),
        waits,
        "the call must not wait for a refresh"
    );
    assert_eq!(
        verifier.completed_refreshes(),
        refreshes,
        "and must not start one"
    );
}

/// Past the TTL the cached key is not served, with no refresh and no wait.
#[test]
fn an_expired_key_set_refuses_without_waiting() {
    let (verifier, clock) = started();
    clock.0.store(3_599_999, Ordering::SeqCst);
    assert!(verifier
        .verify_without_refresh(fixtures::TOKEN_GOOD_RS256)
        .is_ok());

    clock.0.store(3_600_000, Ordering::SeqCst);
    let waits = verifier.refresh_waits();
    assert_eq!(
        verifier.verify_without_refresh(fixtures::TOKEN_GOOD_RS256),
        Err(IdentityError::JwksUnavailable)
    );
    assert_eq!(verifier.refresh_waits(), waits);
}
