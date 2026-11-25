//! Story 23-8 — the external KMS adapter (ADR-051 amendment, `docs/reference/kms-wire-v1.md`).
//!
//! [`HttpKms`] implements [`KeyManagementPort`] over `maos.kms.v1`, a small
//! MAOS-defined JSON wire that an operator-run sidecar serves in front of the
//! organisation's KMS or HSM. The binary carries no vendor API: wrap and
//! unwrap are two POSTs to a configured endpoint, health is one GET.
//!
//! Three rules shape the code:
//! - `is_healthy` does no I/O. It reads a state that only a successful
//!   [`HttpKms::probe`] sets, and that goes stale on the injected clock, so a
//!   stopped prober means an unhealthy KMS and sealed writes are refused.
//! - Only an *availability* failure flips the state down at once: a transport
//!   error, a timeout, a 5xx (or any other non-answer) and a 2xx with a
//!   malformed or wrong-version body. A typed `422` refusal is a property of
//!   one payload, not of the endpoint, and leaves the state as it is.
//! - No error, `Debug` output or log line carries the bearer, a data key, a
//!   wrapped key or a response body.
//!
//! Limits of the transport (Story 23-8 review): [`TimedHttp`] trusts the
//! webpki root set only and has no CA-bundle setting, so an `https` endpoint
//! behind a private CA fails the handshake and reads as unavailable; and ureq
//! applies none of its timeouts to the DNS lookup, so for a host-name endpoint
//! a call can outlast them (it stays fail-closed: health goes stale).

use maos_domain::ports::io_subsystem::{HttpExchange, IoError};
use maos_domain::ports::{IoSubsystemPort, KeyManagementPort, KmsError};
use std::fmt;
use std::io::Read;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use base64::Engine as _;

/// The `version` member of every `maos.kms.v1` request and response.
pub const WIRE_VERSION: &str = "maos.kms.v1";
/// Largest wrapped data key either side may carry.
pub const MAX_WRAPPED_BYTES: usize = 4096;
/// Default period of the composition root's probe task.
pub const DEFAULT_PROBE_INTERVAL: Duration = Duration::from_secs(10);
/// A probe result stays good for this many probe intervals.
pub const STALENESS_INTERVALS: u32 = 3;

const DATA_KEY_LEN: usize = 32;
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A monotonic clock, injected so tests advance time without sleeping.
pub trait KmsClock: Send + Sync {
    /// Time since an arbitrary fixed origin; never goes backwards.
    fn now(&self) -> Duration;
}

/// The production clock.
#[derive(Debug)]
pub struct MonotonicClock {
    origin: Instant,
}

impl MonotonicClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl KmsClock for MonotonicClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Transport timeouts (D-23-8-06 A). The kernel `IoSubsystemAdapter` has none.
///
/// For a literal-IP endpoint these bound a call. ureq 2.12 applies none of
/// them to the DNS lookup, so for a host name the lookup is not bounded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub io: Duration,
    /// Overall limit of a `POST` (wrap, unwrap).
    pub overall: Duration,
    /// Overall limit of the health probe (`GET`): shorter than `overall`, so
    /// a probe that nobody can cancel cannot hold a shutdown for 5 s.
    pub probe: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(2),
            io: Duration::from_secs(2),
            overall: Duration::from_secs(5),
            probe: Duration::from_secs(2),
        }
    }
}

/// An [`IoSubsystemPort`] over its own `ureq` agent with bounded timeouts, no
/// redirects and a 64 KiB response cap (D-23-8-06 A). A 3xx is a refusal that
/// is returned as a status and never followed; a bad header value is withheld
/// from the error. Errors never carry the URL, a header or a body. `https`
/// trusts the webpki root set only: there is no CA-bundle setting.
#[derive(Debug, Clone)]
pub struct TimedHttp {
    agent: ureq::Agent,
    probe_overall: Duration,
}

impl TimedHttp {
    #[must_use]
    pub fn new(timeouts: Timeouts) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(timeouts.connect)
                .timeout_read(timeouts.io)
                .timeout_write(timeouts.io)
                .timeout(timeouts.overall)
                .build(),
            probe_overall: timeouts.probe,
        }
    }
}

impl Default for TimedHttp {
    fn default() -> Self {
        Self::new(Timeouts::default())
    }
}

/// `truncate`: a non-2xx body over the cap is cut so its status survives; a
/// 2xx body over the cap is an error.
fn read_capped(response: ureq::Response, truncate: bool) -> Result<Vec<u8>, IoError> {
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|_| IoError::Decode("response body read failed".to_owned()))?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        if !truncate {
            return Err(IoError::Decode(
                "response exceeds the 64 KiB cap".to_owned(),
            ));
        }
        body.truncate(MAX_RESPONSE_BYTES as usize);
    }
    Ok(body)
}

fn transport_error(error: &ureq::Transport) -> IoError {
    if error.kind() == ureq::ErrorKind::BadHeader {
        return IoError::Transport("invalid request header (value withheld)".to_owned());
    }
    IoError::Transport(format!("request failed ({:?})", error.kind()))
}

impl IoSubsystemPort for TimedHttp {
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        let response = match self.agent.get(url).timeout(self.probe_overall).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(status, _)) => {
                return Err(IoError::Transport(format!("status {status}")))
            }
            Err(ureq::Error::Transport(error)) => return Err(transport_error(&error)),
        };
        // `redirects(0)` hands a 3xx back as a response; only 2xx is a result.
        if !(200..=299).contains(&response.status()) {
            return Err(IoError::Transport(format!("status {}", response.status())));
        }
        read_capped(response, false)
    }

    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        let exchange = self.http_post_exchange(url, body, headers, "")?;
        match exchange.status {
            200..=299 => Ok(exchange.body),
            status => Err(IoError::Transport(format!("status {status}"))),
        }
    }

    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }
        let response = match request.send_bytes(body) {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(error)) => return Err(transport_error(&error)),
        };
        let status = response.status();
        // A redirect is a refusal: status only, so no `Location` or body can
        // reach an error.
        if (300..400).contains(&status) {
            return Ok(HttpExchange {
                status,
                body: Vec::new(),
                header: None,
            });
        }
        let header = response.header(response_header).map(str::to_owned);
        let body = read_capped(response, !(200..=299).contains(&status))?;
        Ok(HttpExchange {
            status,
            body,
            header,
        })
    }
}

#[derive(Debug)]
enum BearerState {
    /// Constructed without a bearer decision; wrap, unwrap and health refuse.
    Pending,
    /// The bearer was read and could not be: wrap, unwrap and health refuse.
    Unreadable,
    Absent,
    Token(String),
}

/// Where the bearer decision of an adapter stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearerStatus {
    /// Not provisioned yet: every call is refused.
    Pending,
    /// The read failed: every call is refused until restart.
    Unreadable,
    /// Decided (a token, or none): calls are sent.
    Settled,
}

/// What one [`HttpKms::probe_checked`] did to the health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    /// No healthy answer; the endpoint is marked down.
    Unreachable,
    /// A healthy answer that was discarded: a call failed while it was in
    /// flight, so the adapter stays down.
    Discarded,
    /// A healthy answer, applied to the health state.
    Applied,
}

#[derive(Debug, Default)]
struct Health {
    last_ok: Option<Duration>,
    down: bool,
    /// Bumped by every `mark_down`. A probe takes it before its I/O and may
    /// only mark the endpoint up if no failure happened since.
    epoch: u64,
}

/// Why the adapter last refused or failed a call. Logged once per change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    BearerPending,
    BearerUnreadable,
    Unauthorized,
    Forbidden,
    Redirect,
    ClientError,
    Server,
    OtherStatus,
    Transport,
    Malformed,
    Disclosure,
}

impl Failure {
    fn of_status(status: u16) -> Self {
        match status {
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            300..=399 => Self::Redirect,
            400..=499 => Self::ClientError,
            500..=599 => Self::Server,
            _ => Self::OtherStatus,
        }
    }

    /// A line without any value: no bearer, key, URL or body.
    fn notice(self) -> &'static str {
        match self {
            Self::BearerPending => {
                "maos: external KMS call refused: the bearer is not provisioned yet; sealed writes are refused until it is"
            }
            Self::BearerUnreadable => {
                "maos: external KMS call refused: the bearer could not be read at boot; every wrap and unwrap is refused until restart (check MAOS_KMS_BEARER and the secret store)"
            }
            Self::Unauthorized => {
                "maos: external KMS answered HTTP 401: the sidecar rejected the bearer (check MAOS_KMS_BEARER); sealed writes are refused"
            }
            Self::Forbidden => {
                "maos: external KMS answered HTTP 403: the sidecar forbids this caller (check MAOS_KMS_BEARER); sealed writes are refused"
            }
            Self::Redirect => {
                "maos: external KMS answered an HTTP redirect, which is never followed; sealed writes are refused"
            }
            Self::ClientError | Self::OtherStatus => {
                "maos: external KMS answered an unexpected HTTP status; sealed writes are refused"
            }
            Self::Server => {
                "maos: external KMS answered HTTP 5xx: the sidecar or its KMS is failing; sealed writes are refused"
            }
            Self::Transport => {
                "maos: external KMS could not be reached on wrap/unwrap (connection, timeout or TLS failure); sealed writes are refused"
            }
            Self::Malformed => {
                "maos: external KMS answered something that is not a maos.kms.v1 message; sealed writes are refused"
            }
            Self::Disclosure => {
                "maos: external KMS returned a wrapped key that discloses the data key; the write is refused (the wrapping endpoint is not trustworthy)"
            }
        }
    }
}

/// [`KeyManagementPort`] over `maos.kms.v1`.
pub struct HttpKms {
    io: Arc<dyn IoSubsystemPort>,
    base: String,
    bearer: Mutex<BearerState>,
    clock: Arc<dyn KmsClock>,
    probe_interval: Duration,
    health: Mutex<Health>,
    notice: Arc<dyn Fn(&str) + Send + Sync>,
    last_failure: Mutex<Option<Failure>>,
}

impl fmt::Debug for HttpKms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bearer = match &*lock(&self.bearer) {
            BearerState::Pending => "pending",
            BearerState::Unreadable => "unreadable",
            BearerState::Absent => "absent",
            BearerState::Token(_) => "set (withheld)",
        };
        f.debug_struct("HttpKms")
            .field("endpoint", &self.base)
            .field("bearer", &bearer)
            .field("probe_interval", &self.probe_interval)
            .finish_non_exhaustive()
    }
}

impl HttpKms {
    /// An adapter whose bearer decision is already made (`None` = none).
    ///
    /// # Errors
    /// `MalformedKey` when the endpoint is not a plain `http(s)://host[:port][/prefix]`
    /// base (userinfo, query and fragment are refused, so a credential never
    /// rides the URL).
    pub fn new(
        io: Arc<dyn IoSubsystemPort>,
        endpoint: &str,
        bearer: Option<String>,
        clock: Arc<dyn KmsClock>,
    ) -> Result<Self, KmsError> {
        let state = match bearer {
            Some(token) => BearerState::Token(token),
            None => BearerState::Absent,
        };
        Self::build(io, endpoint, state, clock)
    }

    /// An adapter that refuses wrap, unwrap and health until
    /// [`Self::provision_bearer`] is called once. The composition root builds
    /// the adapter before the secret store exists (D-23-8-08 A) and provisions
    /// the bearer when the store can be read.
    ///
    /// # Errors
    /// As [`Self::new`].
    pub fn pending_bearer(
        io: Arc<dyn IoSubsystemPort>,
        endpoint: &str,
        clock: Arc<dyn KmsClock>,
    ) -> Result<Self, KmsError> {
        Self::build(io, endpoint, BearerState::Pending, clock)
    }

    fn build(
        io: Arc<dyn IoSubsystemPort>,
        endpoint: &str,
        bearer: BearerState,
        clock: Arc<dyn KmsClock>,
    ) -> Result<Self, KmsError> {
        let rest = endpoint
            .strip_prefix("https://")
            .or_else(|| endpoint.strip_prefix("http://"))
            .ok_or(KmsError::MalformedKey("KMS endpoint must be http(s)"))?;
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || endpoint.contains(['?', '#'])
            || endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(KmsError::MalformedKey(
                "KMS endpoint must be a plain base URL (no userinfo, query or fragment)",
            ));
        }
        Ok(Self {
            io,
            base: endpoint.trim_end_matches('/').to_owned(),
            bearer: Mutex::new(bearer),
            clock,
            probe_interval: DEFAULT_PROBE_INTERVAL,
            health: Mutex::new(Health::default()),
            notice: Arc::new(|line| eprintln!("{line}")),
            last_failure: Mutex::new(None),
        })
    }

    /// Set the probe interval the staleness bound is a multiple of.
    #[must_use]
    pub fn with_probe_interval(mut self, interval: Duration) -> Self {
        self.probe_interval = interval;
        self
    }

    /// Replace where the adapter's one-line notices go (default: stderr). A
    /// notice is logged once per change of failure class and never carries
    /// the bearer, a key, a URL or a response body.
    #[must_use]
    pub fn with_notice_sink(mut self, sink: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        self.notice = sink;
        self
    }

    /// Settle the bearer of a [`Self::pending_bearer`] adapter (`None` = none).
    /// A refusal logged while the bearer was pending no longer describes the
    /// adapter, so it is forgotten (and not reported as a recovery).
    pub fn provision_bearer(&self, bearer: Option<String>) {
        *lock(&self.bearer) = match bearer {
            Some(token) => BearerState::Token(token),
            None => BearerState::Absent,
        };
        *lock(&self.last_failure) = None;
    }

    /// Record that the bearer could not be read: every call stays refused.
    pub fn bearer_unreadable(&self) {
        *lock(&self.bearer) = BearerState::Unreadable;
        *lock(&self.last_failure) = None;
    }

    /// Where the bearer decision stands.
    pub fn bearer_status(&self) -> BearerStatus {
        match &*lock(&self.bearer) {
            BearerState::Pending => BearerStatus::Pending,
            BearerState::Unreadable => BearerStatus::Unreadable,
            BearerState::Absent | BearerState::Token(_) => BearerStatus::Settled,
        }
    }

    /// Whether the last wrap or unwrap was refused or failed (the bearer
    /// states and every availability failure). Cleared by the next success.
    pub fn calls_failing(&self) -> bool {
        lock(&self.last_failure).is_some()
    }

    fn staleness_bound(&self) -> Duration {
        self.probe_interval.saturating_mul(STALENESS_INTERVALS)
    }

    fn mark_down(&self) {
        let mut health = lock(&self.health);
        health.down = true;
        health.epoch = health.epoch.wrapping_add(1);
    }

    fn epoch(&self) -> u64 {
        lock(&self.health).epoch
    }

    /// Apply a successful probe that started at `epoch`. A failure recorded
    /// since (a call that failed while the probe was in flight) wins and the
    /// result is discarded (`false`).
    fn mark_ok(&self, epoch: u64) -> bool {
        let now = self.clock.now();
        let mut health = lock(&self.health);
        if health.epoch == epoch {
            health.last_ok = Some(now);
            health.down = false;
            true
        } else {
            false
        }
    }

    fn note_failure(&self, failure: Failure) {
        let changed = {
            let mut last = lock(&self.last_failure);
            let changed = *last != Some(failure);
            *last = Some(failure);
            changed
        };
        if changed {
            (self.notice)(failure.notice());
        }
    }

    fn note_success(&self) {
        if lock(&self.last_failure).take().is_some() {
            (self.notice)("maos: external KMS wrap/unwrap answered again");
        }
    }

    /// Run one `GET /health`. A success refreshes the cached state unless a
    /// call failed while it was in flight (then it is discarded); any failure
    /// marks the endpoint down. `true` only when the endpoint answered
    /// `{"version":"maos.kms.v1","status":"ok"}` **and** that result was
    /// applied to the health state.
    pub fn probe(&self) -> bool {
        self.probe_checked() == ProbeResult::Applied
    }

    /// As [`Self::probe`], telling a failed probe from one whose healthy answer
    /// was discarded because a call failed while it was in flight.
    pub fn probe_checked(&self) -> ProbeResult {
        let epoch = self.epoch();
        let ok = self
            .io
            .http_get(&format!("{}/health", self.base))
            .ok()
            .and_then(|body| parse_body(&body))
            .is_some_and(|value| {
                value.get("status").and_then(serde_json::Value::as_str) == Some("ok")
            });
        if !ok {
            self.mark_down();
            return ProbeResult::Unreachable;
        }
        if self.mark_ok(epoch) {
            ProbeResult::Applied
        } else {
            ProbeResult::Discarded
        }
    }

    fn authorization(&self) -> Result<Option<String>, KmsError> {
        let refused = match &*lock(&self.bearer) {
            BearerState::Pending => Failure::BearerPending,
            BearerState::Unreadable => Failure::BearerUnreadable,
            BearerState::Absent => return Ok(None),
            BearerState::Token(token) => return Ok(Some(format!("Bearer {token}"))),
        };
        self.note_failure(refused);
        Err(KmsError::Unavailable(
            if refused == Failure::BearerPending {
                "KMS bearer is not provisioned yet"
            } else {
                "KMS bearer could not be read"
            }
            .to_owned(),
        ))
    }

    /// POST `body` to `path` and classify the answer. `Ok` is a parsed 2xx
    /// body that carries the wire version.
    fn call(&self, path: &str, body: &str) -> Result<serde_json::Value, KmsError> {
        let authorization = self.authorization()?;
        let mut headers = vec![("Content-Type", "application/json")];
        if let Some(value) = authorization.as_deref() {
            headers.push(("Authorization", value));
        }
        let url = format!("{}{path}", self.base);
        let exchange = match self
            .io
            .http_post_exchange(&url, body.as_bytes(), &headers, "")
        {
            Ok(exchange) => exchange,
            Err(_) => {
                self.mark_down();
                self.note_failure(Failure::Transport);
                return Err(KmsError::Unavailable("KMS endpoint unreachable".to_owned()));
            }
        };
        match exchange.status {
            200..=299 => match parse_body(&exchange.body) {
                Some(value) => Ok(value),
                None => Err(self.malformed_answer()),
            },
            422 => match typed_refusal(&exchange.body) {
                Some(refusal) => Err(KmsError::Crypto(refusal.to_owned())),
                None => Err(self.malformed_answer()),
            },
            status => {
                self.mark_down();
                self.note_failure(Failure::of_status(status));
                Err(KmsError::Unavailable(format!(
                    "KMS endpoint answered status {status}"
                )))
            }
        }
    }

    fn malformed_answer(&self) -> KmsError {
        self.mark_down();
        self.note_failure(Failure::Malformed);
        KmsError::MalformedPayload("KMS answer is not a valid maos.kms.v1 message")
    }
}

/// A JSON object that carries `"version":"maos.kms.v1"`.
fn parse_body(body: &[u8]) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    (value.get("version").and_then(serde_json::Value::as_str) == Some(WIRE_VERSION))
        .then_some(value)
}

/// `{"error":"wrap_failed"|"unwrap_failed"}` as a static string.
fn typed_refusal(body: &[u8]) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    match value.get("error").and_then(serde_json::Value::as_str)? {
        "wrap_failed" => Some("wrap_failed"),
        "unwrap_failed" => Some("unwrap_failed"),
        _ => None,
    }
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64_field(value: &serde_json::Value, name: &str) -> Option<Vec<u8>> {
    let text = value.get(name)?.as_str()?;
    base64::engine::general_purpose::STANDARD.decode(text).ok()
}

/// Whether `wrapped` visibly discloses `key`: it contains the key bytes, is
/// a prefix or suffix of the key, or contains the key's base64 (standard or
/// URL-safe, padded or not) or hex (either case) text. An opaque answer cannot
/// prove encryption: this rejects obvious disclosure only, and
/// confidentiality still needs a trusted wrapping endpoint.
fn discloses(key: &[u8], wrapped: &[u8]) -> bool {
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle)
    }
    if wrapped.is_empty()
        || contains(wrapped, key)
        || key.starts_with(wrapped)
        || key.ends_with(wrapped)
    {
        return true;
    }
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    let hex = |upper: bool| -> String {
        key.iter()
            .map(|byte| {
                if upper {
                    format!("{byte:02X}")
                } else {
                    format!("{byte:02x}")
                }
            })
            .collect()
    };
    [
        STANDARD.encode(key),
        STANDARD_NO_PAD.encode(key),
        URL_SAFE.encode(key),
        URL_SAFE_NO_PAD.encode(key),
        hex(false),
        hex(true),
    ]
    .iter()
    .any(|text| contains(wrapped, text.as_bytes()))
}

impl KeyManagementPort for HttpKms {
    fn wrap_data_key(&self, data_key: &[u8]) -> Result<Vec<u8>, KmsError> {
        if data_key.len() != DATA_KEY_LEN {
            return Err(KmsError::MalformedKey("data key must be 32 bytes"));
        }
        let request = serde_json::json!({
            "version": WIRE_VERSION,
            "plaintext": b64(data_key),
        });
        let answer = self.call("/wrap", &request.to_string())?;
        match b64_field(&answer, "wrapped") {
            Some(wrapped) if !wrapped.is_empty() && wrapped.len() <= MAX_WRAPPED_BYTES => {
                if discloses(data_key, &wrapped) {
                    self.note_failure(Failure::Disclosure);
                    return Err(KmsError::Crypto(
                        "wrapped key discloses the data key".to_owned(),
                    ));
                }
                self.note_success();
                Ok(wrapped)
            }
            _ => Err(self.malformed_answer()),
        }
    }

    fn unwrap_data_key(&self, wrapped_data_key: &[u8]) -> Result<Vec<u8>, KmsError> {
        if wrapped_data_key.is_empty() || wrapped_data_key.len() > MAX_WRAPPED_BYTES {
            return Err(KmsError::MalformedPayload("wrapped data key size invalid"));
        }
        let request = serde_json::json!({
            "version": WIRE_VERSION,
            "wrapped": b64(wrapped_data_key),
        });
        let answer = self.call("/unwrap", &request.to_string())?;
        match b64_field(&answer, "plaintext") {
            Some(plaintext) if plaintext.len() == DATA_KEY_LEN => {
                self.note_success();
                Ok(plaintext)
            }
            _ => Err(self.malformed_answer()),
        }
    }

    fn is_healthy(&self) -> bool {
        if !matches!(self.bearer_status(), BearerStatus::Settled) {
            return false;
        }
        let health = lock(&self.health);
        match health.last_ok {
            Some(last_ok) if !health.down => {
                self.clock.now().saturating_sub(last_ok) <= self.staleness_bound()
            }
            _ => false,
        }
    }
}
