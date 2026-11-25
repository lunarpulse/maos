//! Story 23-2 — OIDC discovery and key rotation (D-23-2-01…10).
//!
//! The operator names an issuer; this module fetches
//! `<issuer>/.well-known/openid-configuration`, follows its `jwks_uri`, and
//! keeps the keys it finds in a cache with a hard TTL. It performs no I/O of
//! its own: every request goes through an injected [`Fetcher`] and every
//! reading of time through an injected [`Clock`], so tests need no socket, no
//! sleep and no wall clock.
//!
//! Rules (each a typed [`DiscoveryError`] when broken, and no key is loaded):
//! - the issuer must be an `https` URL with no userinfo, query or fragment, and
//!   the document's `issuer` must equal it byte for byte (OpenID Connect
//!   Discovery 1.0 §4.3);
//! - `jwks_uri` must be `https` with no userinfo; whether the destination is
//!   allowed (loopback, link-local, private, redirects) is the fetcher's
//!   outbound policy (Story 23-0), never a copy of it here;
//! - the algorithm allowlist is the static verifier's: `none` and HS256 are
//!   refused whatever the keys say, and the claim checks are the very code the
//!   static verifier runs.
//!
//! Freshness: keys older than the TTL are never served (D-23-2-02 A); the TTL starts when a
//! fetch COMPLETES. An unknown `kid`, or stale keys, ask for a refresh at most once per minimum
//! interval, measured from the moment the previous fetch actually began, at millisecond
//! resolution; `verify` never fetches. One refresher thread per verifier owns every fetch
//! (D-23-2-07 A); `verify` waits for it at most the configured bound. A refresh that outlives
//! that bound is marked stalled, and later callers fail at once instead of queueing behind it.
//!
//! A fetch that never answers (the kernel agent has no read timeout) is abandoned once it has
//! run for the fetch bound (default 30 s): the refresher stops waiting, the next refresh runs on
//! a fresh worker, and the abandoned worker's answer, if it ever arrives, is dropped. At most
//! ONE abandoned worker exists at a time; while it is stuck a second hung fetch is waited for,
//! not abandoned, so a peer that never answers costs two threads, never one per refresh.
//!
//! Limits (all fail closed): bodies over [`MAX_BODY_BYTES`], more than [`MAX_KEYS`] keys, a
//! duplicate `kid` or a URL over [`MAX_URL_BYTES`] refuse the whole refresh and keep the
//! previous keys until their TTL ends. An entry that is not a usable verification key (a `kid`
//! over [`MAX_KID_BYTES`], `use` other than `sig`, `key_ops` without `verify`, an `EC` key not on
//! `P-256`, any other `kty`) is skipped; a well-formed set with NO usable key retires the cached
//! keys (the verifier turns unhealthy).

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use maos_domain::outbound::OutboundRefusal;
use maos_domain::ports::io_subsystem::{IoError, IoSubsystemPort};
use maos_domain::ports::{AuthenticatedPrincipal, IdentityAssertionPort, IdentityError};
use serde::Deserialize;

use crate::{Jwk, OidcAlgorithm, Rules};

/// Default key lifetime (D-23-2-02 A).
pub const DEFAULT_TTL_SECS: u64 = 3_600;
/// Default minimum interval between two refresh attempts (D-23-2-02 A).
pub const DEFAULT_MIN_REFRESH_SECS: u64 = 30;
/// Default bound on how long `verify` waits for a refresh (D-23-2-07 A).
pub const DEFAULT_VERIFY_WAIT: Duration = Duration::from_secs(2);
/// The longest TTL the configuration accepts (seven days).
pub const MAX_TTL_SECS: u64 = 7 * 24 * 3_600;
/// A discovery document or key set larger than this is refused.
pub const MAX_BODY_BYTES: usize = 256 * 1024;
/// A key set with more entries than this is refused.
pub const MAX_KEYS: usize = 64;
/// The longest issuer or `jwks_uri` accepted.
pub const MAX_URL_BYTES: usize = 2_048;
/// The longest `kid` accepted.
pub const MAX_KID_BYTES: usize = 256;
/// Default bound on one discovery round (document plus keys) before it is abandoned.
pub const DEFAULT_FETCH_TIMEOUT_SECS: u64 = 30;
/// The longest fetch bound the configuration accepts.
pub const MAX_FETCH_TIMEOUT_SECS: u64 = 300;

/// Clock readings. Injected so TTL, limiter and tests share one notion of time; it must never
/// go backwards.
pub trait Clock: Send + Sync {
    fn now_secs(&self) -> u64;

    /// Milliseconds on the same timeline. The limiter and the TTL compare at this resolution so
    /// a refresh that began at 0.9 s is not "30 s old" at 30.0 s. The default reads whole
    /// seconds; a clock with a finer reading overrides it.
    fn now_millis(&self) -> u64 {
        self.now_secs().saturating_mul(1_000)
    }
}

/// The process clock: monotonic seconds since construction.
#[derive(Debug, Clone, Copy)]
pub struct MonotonicClock {
    start: Instant,
}

impl MonotonicClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    fn now_secs(&self) -> u64 {
        self.start.elapsed().as_secs()
    }

    fn now_millis(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Why a fetch gave no body. Never carries a URL, header or body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// The outbound policy refused the destination (Story 23-0).
    #[error("outbound destination refused: {}", .0.code())]
    Refused(OutboundRefusal),
    /// Transport failure, a non-2xx answer (3xx included) or a decode failure.
    #[error("fetch failed")]
    Unavailable,
}

/// One HTTP GET. The production adapter is [`IoFetcher`] over a policed,
/// binding `IoSubsystemPort`.
pub trait Fetcher: Send + Sync {
    /// # Errors
    /// [`FetchError`] when no 2xx body was obtained.
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// [`Fetcher`] over an [`IoSubsystemPort`]. The port's error text can carry
/// the full URL (userinfo and query included); this adapter keeps only the
/// outbound-policy class and drops the rest.
pub struct IoFetcher<P: IoSubsystemPort> {
    port: P,
}

impl<P: IoSubsystemPort> IoFetcher<P> {
    #[must_use]
    pub fn new(port: P) -> Self {
        Self { port }
    }
}

impl<P: IoSubsystemPort> Fetcher for IoFetcher<P> {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.port.http_get(url).map_err(|error| match error {
            IoError::InvalidUrl(code) => {
                refusal_for(&code).map_or(FetchError::Unavailable, FetchError::Refused)
            }
            IoError::Transport(_) | IoError::Decode(_) => FetchError::Unavailable,
        })
    }
}

fn refusal_for(code: &str) -> Option<OutboundRefusal> {
    [
        OutboundRefusal::Http,
        OutboundRefusal::Loopback,
        OutboundRefusal::LinkLocal,
        OutboundRefusal::Private,
        OutboundRefusal::Reserved,
        OutboundRefusal::Redirect,
        OutboundRefusal::Unresolved,
        OutboundRefusal::UrlInvalid,
        OutboundRefusal::Unbound,
    ]
    .into_iter()
    .find(|refusal| refusal.code() == code)
}

/// Why discovery was refused or failed. No variant carries a URL or a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiscoveryError {
    #[error("sso_discovery_config_invalid")]
    ConfigInvalid,
    #[error("sso_discovery_issuer_not_trusted")]
    IssuerNotTrusted,
    /// Discovered keys are bound to the discovery issuer: another listed issuer is refused.
    #[error("sso_discovery_issuers_not_exclusive")]
    IssuersNotExclusive,
    #[error("sso_discovery_issuer_not_https")]
    IssuerNotHttps,
    #[error("sso_discovery_issuer_invalid")]
    IssuerInvalid,
    #[error("sso_discovery_document_fetch_failed: {0}")]
    DocumentFetch(FetchError),
    #[error("sso_discovery_document_malformed")]
    DocumentMalformed,
    #[error("sso_discovery_issuer_mismatch")]
    IssuerMismatch,
    #[error("sso_discovery_jwks_uri_not_https")]
    JwksUriNotHttps,
    #[error("sso_discovery_jwks_uri_invalid")]
    JwksUriInvalid,
    #[error("sso_discovery_jwks_fetch_failed: {0}")]
    JwksFetch(FetchError),
    #[error("sso_discovery_jwks_malformed")]
    JwksMalformed,
    #[error("sso_discovery_jwks_too_large")]
    TooLarge,
    #[error("sso_discovery_jwks_no_usable_keys")]
    NoUsableKeys,
    /// A fetch outlived the fetch bound and was abandoned (its answer, if it ever comes, is
    /// dropped).
    #[error("sso_discovery_fetch_abandoned")]
    FetchAbandoned,
    #[error("sso_discovery_refresher_unavailable")]
    RefresherUnavailable,
}

/// Discovery settings. Defaults are D-23-2-02 A and D-23-2-07 A.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    issuer: String,
    ttl_secs: u64,
    min_refresh_secs: u64,
    verify_wait: Duration,
    fetch_timeout_secs: u64,
    idle_tick: Duration,
}

impl DiscoveryConfig {
    #[must_use]
    pub fn new(issuer: &str) -> Self {
        Self {
            issuer: issuer.to_owned(),
            ttl_secs: DEFAULT_TTL_SECS,
            min_refresh_secs: DEFAULT_MIN_REFRESH_SECS,
            verify_wait: DEFAULT_VERIFY_WAIT,
            fetch_timeout_secs: DEFAULT_FETCH_TIMEOUT_SECS,
            idle_tick: Duration::from_secs(DEFAULT_MIN_REFRESH_SECS),
        }
    }

    #[must_use]
    pub fn with_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    #[must_use]
    pub fn with_min_refresh_secs(mut self, min_refresh_secs: u64) -> Self {
        self.min_refresh_secs = min_refresh_secs;
        self.idle_tick = Duration::from_secs(min_refresh_secs.max(1));
        self
    }

    /// How long `verify` waits for a refresh it asked for or joined.
    #[must_use]
    pub fn with_verify_wait(mut self, verify_wait: Duration) -> Self {
        self.verify_wait = verify_wait;
        self
    }

    /// Real-time interval at which an idle refresher re-checks whether it is
    /// due (the decision itself reads the injected clock).
    #[must_use]
    pub fn with_idle_tick(mut self, idle_tick: Duration) -> Self {
        self.idle_tick = idle_tick;
        self
    }

    /// How long one discovery round may run before it is abandoned and a replacement fetch
    /// may start (the kernel agent has no read timeout of its own).
    #[must_use]
    pub fn with_fetch_timeout_secs(mut self, fetch_timeout_secs: u64) -> Self {
        self.fetch_timeout_secs = fetch_timeout_secs;
        self
    }

    fn validate(&self) -> Result<(), DiscoveryError> {
        if self.ttl_secs == 0
            || self.ttl_secs > MAX_TTL_SECS
            || self.min_refresh_secs == 0
            || self.min_refresh_secs > self.ttl_secs
            || self.fetch_timeout_secs == 0
            || self.fetch_timeout_secs > MAX_FETCH_TIMEOUT_SECS
            || self.idle_tick.is_zero()
        {
            return Err(DiscoveryError::ConfigInvalid);
        }
        check_https(&self.issuer, true).map_err(|fault| match fault {
            UrlFault::NotHttps => DiscoveryError::IssuerNotHttps,
            UrlFault::Invalid => DiscoveryError::IssuerInvalid,
        })
    }

    fn document_url(&self) -> String {
        format!(
            "{}/.well-known/openid-configuration",
            self.issuer.trim_end_matches('/')
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrlFault {
    NotHttps,
    Invalid,
}

/// Textual check only: scheme, size, userinfo. The destination policy belongs
/// to the fetcher. `issuer` also forbids a query and a fragment.
fn check_https(url: &str, issuer: bool) -> Result<(), UrlFault> {
    let rest = match url.get(..8) {
        Some(scheme) if scheme.eq_ignore_ascii_case("https://") => &url[8..],
        _ => return Err(UrlFault::NotHttps),
    };
    if url.len() > MAX_URL_BYTES
        || url
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ' || byte == b'\\')
        || (issuer && url.contains(['?', '#']))
    {
        return Err(UrlFault::Invalid);
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err(UrlFault::Invalid);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Document {
    issuer: String,
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct KeySetBody {
    keys: Vec<serde_json::Value>,
}

/// One discovery round: document, then keys. Pure over the fetcher.
fn fetch_keys(
    fetcher: &dyn Fetcher,
    config: &DiscoveryConfig,
) -> Result<HashMap<String, Jwk>, DiscoveryError> {
    let body = fetcher
        .get(&config.document_url())
        .map_err(DiscoveryError::DocumentFetch)?;
    if body.len() > MAX_BODY_BYTES {
        return Err(DiscoveryError::TooLarge);
    }
    let document: Document =
        serde_json::from_slice(&body).map_err(|_| DiscoveryError::DocumentMalformed)?;
    if document.issuer != config.issuer {
        return Err(DiscoveryError::IssuerMismatch);
    }
    check_https(&document.jwks_uri, false).map_err(|fault| match fault {
        UrlFault::NotHttps => DiscoveryError::JwksUriNotHttps,
        UrlFault::Invalid => DiscoveryError::JwksUriInvalid,
    })?;
    let body = fetcher
        .get(&document.jwks_uri)
        .map_err(DiscoveryError::JwksFetch)?;
    if body.len() > MAX_BODY_BYTES {
        return Err(DiscoveryError::TooLarge);
    }
    parse_key_set(&body)
}

/// A usable verification key: an `RSA` key, or an `EC` key on `P-256` (the curve ES256 names),
/// not marked for encryption, whose `key_ops` (when declared) names `verify`.
fn usable(key: &Jwk) -> bool {
    let family = match key.kty.as_str() {
        "RSA" => true,
        "EC" => key.crv.as_deref() == Some("P-256"),
        _ => false,
    };
    family
        && key.key_use.as_deref().is_none_or(|usage| usage == "sig")
        && key.allows_verify()
        && key.kid.len() <= MAX_KID_BYTES
}

fn parse_key_set(body: &[u8]) -> Result<HashMap<String, Jwk>, DiscoveryError> {
    let set: KeySetBody =
        serde_json::from_slice(body).map_err(|_| DiscoveryError::JwksMalformed)?;
    if set.keys.len() > MAX_KEYS {
        return Err(DiscoveryError::TooLarge);
    }
    let mut keys = HashMap::new();
    for value in set.keys {
        // An entry that is not a usable signing key is skipped (fewer keys is
        // the safe direction); an ambiguous `kid` refuses the whole set.
        let Ok(key) = serde_json::from_value::<Jwk>(value) else {
            continue;
        };
        if !usable(&key) {
            continue;
        }
        if keys.insert(key.kid.clone(), key).is_some() {
            return Err(DiscoveryError::JwksMalformed);
        }
    }
    if keys.is_empty() {
        return Err(DiscoveryError::NoUsableKeys);
    }
    Ok(keys)
}

struct KeySet {
    /// Clock milliseconds at which the fetch COMPLETED (the TTL starts here).
    fetched_at: u64,
    keys: HashMap<String, Jwk>,
}

struct State {
    keys: Option<KeySet>,
    /// Clock milliseconds at which the most recent fetch actually started.
    last_attempt: Option<u64>,
    /// A refresh is requested or running.
    pending: bool,
    /// A caller gave up waiting for the pending refresh.
    stalled: bool,
    /// Completed refresh attempts, successful or not.
    generation: u64,
    last_error: Option<DiscoveryError>,
    shutdown: bool,
    /// Id of the most recently started fetch.
    last_fetch: u64,
    /// Calls that started waiting for a refresh.
    waits: u64,
    /// The one fetch worker that was abandoned after the fetch bound and has not returned yet.
    abandoned: Option<u64>,
}

struct Shared {
    config: DiscoveryConfig,
    rules: Rules,
    fetcher: Arc<dyn Fetcher>,
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
    wake_refresher: Condvar,
    refresh_done: Condvar,
}

type Fetched = Result<HashMap<String, Jwk>, DiscoveryError>;

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn ttl_millis(&self) -> u64 {
        self.config.ttl_secs.saturating_mul(1_000)
    }

    fn fresh(&self, keys: &KeySet, now: u64) -> bool {
        now.saturating_sub(keys.fetched_at) < self.ttl_millis()
    }

    fn limiter_open(&self, state: &State, now: u64) -> bool {
        state.last_attempt.is_none_or(|last| {
            now.saturating_sub(last) >= self.config.min_refresh_secs.saturating_mul(1_000)
        })
    }

    /// An idle refresher refreshes when it has no fresh keys, or the keys are
    /// half-way through their life.
    fn due(&self, state: &State, now: u64) -> bool {
        let wanted = state.keys.as_ref().is_none_or(|keys| {
            now.saturating_sub(keys.fetched_at) >= self.ttl_millis() / 2 || !self.fresh(keys, now)
        });
        wanted && self.limiter_open(state, now)
    }

    /// The key for `kid`, asking the refresher at most once when the cache
    /// cannot answer. Never fetches.
    fn key_for(&self, kid: &str) -> Option<Jwk> {
        let mut state = self.lock();
        let mut asked = false;
        loop {
            let now = self.clock.now_millis();
            if let Some(keys) = state.keys.as_ref().filter(|keys| self.fresh(keys, now)) {
                if let Some(key) = keys.keys.get(kid) {
                    return Some(key.clone());
                }
            }
            if asked || state.shutdown {
                return None;
            }
            asked = true;
            if state.pending {
                if state.stalled {
                    return None;
                }
            } else if self.limiter_open(&state, now) {
                // Single flight: `pending` is the flag; the limiter's clock starts when the
                // refresher actually begins the fetch.
                state.pending = true;
                self.wake_refresher.notify_one();
            } else {
                return None;
            }
            state.waits += 1;
            let seen = state.generation;
            let (guard, wait) = self
                .refresh_done
                .wait_timeout_while(state, self.config.verify_wait, |state| {
                    state.generation == seen && !state.shutdown
                })
                .unwrap_or_else(PoisonError::into_inner);
            state = guard;
            if wait.timed_out() && state.generation == seen {
                state.stalled = true;
            }
        }
    }

    /// Run one discovery round on a worker thread and wait for it, abandoning it when it has
    /// run longer than the fetch bound (the kernel agent has no read timeout, so a silent peer
    /// would otherwise hold the refresher for good). At most ONE abandoned worker exists at a
    /// time: while it is still stuck, the replacement is waited for rather than abandoned, so a
    /// peer that never answers costs two threads, not one per refresh. A late answer from an
    /// abandoned worker is dropped with its channel and never touches the cache.
    fn run_fetch(self: &Arc<Self>, id: u64, started: u64) -> Fetched {
        let unavailable = || Err(DiscoveryError::DocumentFetch(FetchError::Unavailable));
        let (reply, answer) = std::sync::mpsc::channel();
        let worker = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("maos-sso-jwks-fetch".to_owned())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    fetch_keys(worker.fetcher.as_ref(), &worker.config)
                }))
                .unwrap_or(Err(DiscoveryError::DocumentFetch(FetchError::Unavailable)));
                let _ = reply.send(outcome);
                let mut state = worker.lock();
                if state.abandoned == Some(id) {
                    state.abandoned = None;
                }
            });
        if spawned.is_err() {
            return unavailable();
        }
        let poll = self.config.idle_tick.min(Duration::from_secs(1));
        let bound = self.config.fetch_timeout_secs.saturating_mul(1_000);
        loop {
            match answer.recv_timeout(poll) {
                Ok(outcome) => return outcome,
                Err(RecvTimeoutError::Disconnected) => return unavailable(),
                Err(RecvTimeoutError::Timeout) => {
                    let mut state = self.lock();
                    if state.shutdown {
                        return unavailable();
                    }
                    if self.clock.now_millis().saturating_sub(started) >= bound {
                        // Under the lock: the worker clears `abandoned` under it too, so an
                        // answer sent after this check still finds the mark and clears it.
                        match answer.try_recv() {
                            Ok(outcome) => return outcome,
                            Err(TryRecvError::Disconnected) => return unavailable(),
                            Err(TryRecvError::Empty) => {}
                        }
                        if state.abandoned.is_none() {
                            state.abandoned = Some(id);
                            return Err(DiscoveryError::FetchAbandoned);
                        }
                    }
                }
            }
        }
    }

    fn refresher(self: &Arc<Self>) {
        let mut state = self.lock();
        loop {
            while !state.pending && !state.shutdown {
                let (guard, wait) = self
                    .wake_refresher
                    .wait_timeout(state, self.config.idle_tick)
                    .unwrap_or_else(PoisonError::into_inner);
                state = guard;
                if wait.timed_out() && !state.pending && !state.shutdown {
                    let now = self.clock.now_millis();
                    if self.due(&state, now) {
                        state.pending = true;
                    }
                }
            }
            if state.shutdown {
                return;
            }
            // The limiter measures from the moment the fetch really begins.
            let started = self.clock.now_millis();
            state.last_attempt = Some(started);
            state.last_fetch += 1;
            let id = state.last_fetch;
            drop(state);
            let outcome = self.run_fetch(id, started);
            state = self.lock();
            match outcome {
                Ok(keys) => {
                    state.keys = Some(KeySet {
                        // The TTL starts when the keys arrived, not when the fetch began.
                        fetched_at: self.clock.now_millis(),
                        keys,
                    });
                    state.last_error = None;
                }
                // A well-formed answer that names no usable key is the issuer withdrawing
                // every key: the cached keys are retired, not kept until their TTL.
                Err(DiscoveryError::NoUsableKeys) => {
                    state.keys = None;
                    state.last_error = Some(DiscoveryError::NoUsableKeys);
                }
                Err(error) => state.last_error = Some(error),
            }
            state.pending = false;
            state.stalled = false;
            state.generation += 1;
            self.refresh_done.notify_all();
        }
    }
}

/// Handle that stops the refresher when the last owner drops it.
struct Stop(Arc<Shared>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.lock().shutdown = true;
        self.0.wake_refresher.notify_all();
        self.0.refresh_done.notify_all();
    }
}

/// An [`IdentityAssertionPort`] over discovered, rotating keys.
pub struct DiscoveringVerifier {
    shared: Arc<Shared>,
    _stop: Stop,
}

impl DiscoveringVerifier {
    /// Validate the configuration and start the refresher with the first
    /// refresh already requested (D-23-2-08 A: an unreachable issuer at boot
    /// leaves the verifier unhealthy, not absent).
    ///
    /// # Errors
    /// [`DiscoveryError`] for an invalid configuration, an issuer that is not
    /// one of `trusted_issuers`, or a refresher thread that cannot start.
    pub fn start(
        config: DiscoveryConfig,
        allowed_algorithms: &[OidcAlgorithm],
        trusted_issuers: &[&str],
        expected_audience: &str,
        fetcher: Arc<dyn Fetcher>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, DiscoveryError> {
        config.validate()?;
        if !trusted_issuers.contains(&config.issuer.as_str()) {
            return Err(DiscoveryError::IssuerNotTrusted);
        }
        // A key discovered for issuer A must never vouch for `iss` = B (F1): in discovery mode
        // the discovery issuer is the only trusted `iss`, so any other listed issuer is refused.
        if trusted_issuers
            .iter()
            .any(|issuer| *issuer != config.issuer)
        {
            return Err(DiscoveryError::IssuersNotExclusive);
        }
        let rules = Rules::new(allowed_algorithms, trusted_issuers, expected_audience)
            .map_err(|_| DiscoveryError::ConfigInvalid)?;
        let shared = Arc::new(Shared {
            config,
            rules,
            fetcher,
            clock,
            state: Mutex::new(State {
                keys: None,
                last_attempt: None,
                pending: true,
                stalled: false,
                generation: 0,
                last_error: None,
                shutdown: false,
                last_fetch: 0,
                waits: 0,
                abandoned: None,
            }),
            wake_refresher: Condvar::new(),
            refresh_done: Condvar::new(),
        });
        let worker = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("maos-sso-jwks".to_owned())
            .spawn(move || worker.refresher())
            .map_err(|_| DiscoveryError::RefresherUnavailable)?;
        Ok(Self {
            _stop: Stop(Arc::clone(&shared)),
            shared,
        })
    }

    /// The error of the most recent refresh, `None` after a success or before
    /// the first one finished.
    #[must_use]
    pub fn last_error(&self) -> Option<DiscoveryError> {
        self.shared.lock().last_error
    }

    /// Refresh attempts finished so far (successful or not).
    #[must_use]
    pub fn completed_refreshes(&self) -> u64 {
        self.shared.lock().generation
    }

    /// Times a `verify` call has started waiting for a refresh (a diagnostic: a caller refused
    /// at once because the refresh is already stalled does not count).
    #[must_use]
    pub fn refresh_waits(&self) -> u64 {
        self.shared.lock().waits
    }
}

impl IdentityAssertionPort for DiscoveringVerifier {
    fn verify(&self, assertion: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        #[cfg(feature = "sso-fault-inject")]
        {
            let claims = crate::parse_unverified_claims(assertion)?;
            return Ok(claims.into_principal());
        }

        #[cfg(not(feature = "sso-fault-inject"))]
        {
            self.shared
                .rules
                .verify(assertion, |kid| self.shared.key_for(kid))
        }
    }

    /// Configuration valid and unexpired keys present.
    fn is_healthy(&self) -> bool {
        let now = self.shared.clock.now_millis();
        let state = self.shared.lock();
        self.shared.rules.configured()
            && state
                .keys
                .as_ref()
                .is_some_and(|keys| self.shared.fresh(keys, now))
    }
}
