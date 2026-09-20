//! Story 16-1 — the operator door's maosctl-side client (T6, D-16-1-A/P).
//!
//! Discovery, in order (AC2): the `MAOS_OPERATOR_HTTP_ENDPOINT` +
//! `MAOS_OPERATOR_BEARER_TOKEN` env pair — BOTH or neither, a half-set pair is
//! a configuration error rather than a partial override — then
//! `<maos_home>/control.json` through
//! `maos_domain::operator_door::ControlFile::load`. That loader already
//! refuses a wider file mode, a foreign owner, an unknown schema version, a
//! non-`tcp://` scheme, a non-loopback endpoint and a malformed token, so
//! this module maps those typed errors instead of re-implementing the checks.
//!
//! NO HTTP CRATE. The wire client is the Story 5.5a hand-rolled
//! `fetch_live_sandbox_report` generalised to GET + POST with a bearer
//! header, an optional body and a `Content-Length`. The read deadline is the
//! route budget + 5 s: the server's own request-read deadline is 5 s
//! (D-16-1-E), so a client that gave up at exactly the budget would race a
//! legitimate handler that was still running.
//!
//! ⚠ The request/response shapes below are DUPLICATED from `maos-control` as
//! plain serde types on purpose: `maos-cli` must not gain a `maos-control`
//! edge — not even a dev edge — because the door adapter links
//! `maos-kernel-core` and `dep_kernel_core_free_test.rs` runs plain
//! `cargo tree -p maos-cli`. The duplication is the cost of the edge ban;
//! the contract tests in `crates/maos-cli/tests/` pin both sides of it.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Endpoint override; valid only together with [`ENV_TOKEN`].
pub const ENV_ENDPOINT: &str = "MAOS_OPERATOR_HTTP_ENDPOINT";
/// Bearer override; valid only together with [`ENV_ENDPOINT`].
pub const ENV_TOKEN: &str = "MAOS_OPERATOR_BEARER_TOKEN";

/// Route budget the server grants a default command (D-16-1-E).
pub const DEFAULT_ROUTE_BUDGET: Duration = Duration::from_secs(10);
/// Route budget for upgrade, uninstall, CRL import and hot-swap-precheck.
pub const LONG_ROUTE_BUDGET: Duration = Duration::from_secs(30);

/// Hard cap for one operator response, including headers.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// A discovered, validated door: where to connect and what to present.
pub struct DoorConfig {
    pub endpoint: SocketAddr,
    pub token: String,
}

/// Every way reaching the door can fail BEFORE a status comes back.
/// The D-16-1-P exit codes are applied by [`map_error`]; statuses (including
/// 401 and the 503 family) are [`Response`]s handled by [`map_response`].
pub enum DoorError {
    /// Neither the env pair nor `control.json` configured. Exit 78, naming
    /// `maos init`.
    NotInitialized(String),
    /// A door IS configured but unusable: a half-set env pair, or a
    /// `control.json` whose mode/owner/version/scheme/token failed
    /// validation. Exit 78 — `ControlFileError`'s Display already names the
    /// file and the offending field (e.g. the mode to `chmod`).
    Config(String),
    /// The OS refused the connect. The branch point of the whole routing
    /// decision: durable verbs fall back to the offline child, door-class
    /// verbs run the home-lock probe.
    ConnectRefused,
    /// Connected but no well-formed HTTP came back. Exit 69
    /// `DoorUnresponsive`, and NEVER the offline arm (a wedged daemon is
    /// precisely when the offline arm must not run).
    Unresponsive(String),
}

/// One door response, body still raw bytes.
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    fn parse(raw: &[u8]) -> Result<Response, DoorError> {
        let (head, body) = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|offset| (&raw[..offset], &raw[offset + 4..]))
            .ok_or_else(|| DoorError::Unresponsive("no header terminator".to_owned()))?;
        let head = std::str::from_utf8(head)
            .map_err(|_| DoorError::Unresponsive("response header is not UTF-8".to_owned()))?;
        let mut lines = head.split("\r\n");
        let mut status_parts = lines.next().unwrap_or_default().split_ascii_whitespace();
        let version = status_parts
            .next()
            .ok_or_else(|| DoorError::Unresponsive("no HTTP version".to_owned()))?;
        if !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
            return Err(DoorError::Unresponsive(format!(
                "unsupported HTTP version {version:?}"
            )));
        }
        let status = status_parts
            .next()
            .ok_or_else(|| DoorError::Unresponsive("no status code".to_owned()))?
            .parse::<u16>()
            .map_err(|_| DoorError::Unresponsive("status is not a number".to_owned()))?;
        if status_parts.next().is_none() {
            return Err(DoorError::Unresponsive(
                "malformed HTTP status line".to_owned(),
            ));
        }
        let mut content_length = None;
        for line in lines {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| DoorError::Unresponsive("malformed response header".to_owned()))?;
            if name.eq_ignore_ascii_case("transfer-encoding") {
                return Err(DoorError::Unresponsive(
                    "chunked response framing is unsupported".to_owned(),
                ));
            }
            if name.eq_ignore_ascii_case("content-length") {
                if content_length.is_some() {
                    return Err(DoorError::Unresponsive(
                        "duplicate Content-Length".to_owned(),
                    ));
                }
                content_length =
                    Some(value.trim().parse::<usize>().map_err(|_| {
                        DoorError::Unresponsive("invalid Content-Length".to_owned())
                    })?);
            }
        }
        let declared = content_length
            .ok_or_else(|| DoorError::Unresponsive("missing Content-Length".to_owned()))?;
        if declared != body.len() {
            return Err(DoorError::Unresponsive(format!(
                "truncated response body: declared {declared} bytes, received {}",
                body.len()
            )));
        }
        serde_json::from_slice::<serde_json::Value>(body).map_err(|error| {
            DoorError::Unresponsive(format!("response body is not JSON: {error}"))
        })?;
        Ok(Response {
            status,
            body: body.to_vec(),
        })
    }

    /// `(code, detail)` from a fixed-body or typed error body. Fixed bodies
    /// carry only `{"error":<code>}`; typed outcomes add `detail`.
    pub fn error_parts(&self) -> (String, String) {
        let parsed: serde_json::Value =
            serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null); // xtask-serde-allow: foreign door body on an error-reporting path; a non-JSON body is expected input and Null is the documented sentinel, so propagating would replace the real error with a parse error
        let code = parsed
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let detail = parsed
            .get("detail")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        (code, detail)
    }

    /// The 200 body as JSON (Null when it is not — callers that only look for
    /// `terminal_code` treat Null as "no terminal code").
    pub fn body_json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null) // xtask-serde-allow: same fail-soft as error_parts; the doc comment above states callers treat Null as no terminal code
    }
}

/// Discover the door: env pair (both-or-neither) → `control.json`.
pub fn resolve_door() -> Result<DoorConfig, DoorError> {
    let read_override = |name: &str| -> Result<Option<String>, DoorError> {
        match std::env::var_os(name) {
            None => Ok(None),
            Some(value) if value.is_empty() => Ok(None),
            Some(value) => value.into_string().map(Some).map_err(|_| {
                DoorError::Config(format!("maos: {name} is present but is not valid UTF-8"))
            }),
        }
    };
    let endpoint = read_override(ENV_ENDPOINT)?;
    let token = read_override(ENV_TOKEN)?;
    match (endpoint, token) {
        (Some(endpoint), Some(token)) => {
            let address = parse_loopback_endpoint(&endpoint).map_err(|detail| {
                DoorError::Config(format!("maos: {ENV_ENDPOINT} is {endpoint:?}: {detail}"))
            })?;
            Ok(DoorConfig {
                endpoint: address,
                token,
            })
        }
        (Some(_), None) | (None, Some(_)) => Err(DoorError::Config(format!(
            "maos: {ENV_ENDPOINT} and {ENV_TOKEN} must be set together — \
             set both, or unset both to discover the door from <home>/control.json"
        ))),
        (None, None) => from_control_file(),
    }
}

fn from_control_file() -> Result<DoorConfig, DoorError> {
    use maos_domain::operator_door::{control_file_path, maos_home, ControlFile};
    let home = maos_home()
        .map_err(|error| DoorError::Config(error.to_string()))?
        .ok_or_else(|| {
            DoorError::NotInitialized(
                "maos: no operator door configured — neither MAOS_HOME nor HOME is set; \
                 set HOME and run `maos init`"
                    .to_owned(),
            )
        })?;
    let path = control_file_path(&home);
    let control = ControlFile::load(&path).map_err(|error| {
        if error.is_missing() {
            DoorError::NotInitialized(error.to_string())
        } else {
            DoorError::Config(error.to_string())
        }
    })?;
    let endpoint = control
        .endpoint_addr()
        .map_err(|error| DoorError::Config(error.to_string()))?;
    Ok(DoorConfig {
        endpoint,
        token: control.token,
    })
}

/// The loopback endpoint rule every door address must pass (the HEAD sandbox
/// client's rule, now shared by the env override and `control.json`). A bare
/// `SocketAddr` or either scheme prefix is accepted; the address must be
/// loopback because the door's whole authorization boundary is the bearer
/// token on a host-local socket, never network position.
fn parse_loopback_endpoint(endpoint: &str) -> Result<SocketAddr, String> {
    let authority = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("tcp://"))
        .unwrap_or(endpoint);
    let address: SocketAddr = authority
        .parse()
        .map_err(|error: std::net::AddrParseError| format!("not a SocketAddr: {error}"))?;
    if !address.ip().is_loopback() {
        return Err("the operator door is loopback-only".to_owned());
    }
    if address.port() == 0 {
        return Err("operator door port zero is not connectable".to_owned());
    }
    Ok(address)
}

/// One door exchange: connect, send, read the whole response.
///
/// `Content-Length` is sent for EVERY method — the server answers 411 to a
/// POST without one, so `pause`-style body-less POSTs must still declare
/// zero. `Connection: close` keeps read-to-EOF framing honest without a
/// chunked parser; the server closes after one response. Connect timeout 2 s
/// (the HEAD sandbox rule); read deadline = budget + 5 s (see module doc).
pub fn exchange(
    config: &DoorConfig,
    method: &str,
    path: &str,
    body: Option<(&str, &[u8])>,
    budget: Duration,
) -> Result<Response, DoorError> {
    let read_budget = budget + Duration::from_secs(5);
    let deadline = Instant::now() + read_budget;
    let mut stream =
        TcpStream::connect_timeout(&config.endpoint, Duration::from_secs(2)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::ConnectionRefused {
                DoorError::ConnectRefused
            } else {
                DoorError::Unresponsive(error.to_string())
            }
        })?;
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n",
        config.endpoint, config.token,
    );
    match body {
        Some((content_type, bytes)) => {
            request.push_str(&format!(
                "Content-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
                bytes.len()
            ));
        }
        None => {
            request.push_str("Content-Length: 0\r\n\r\n");
        }
    }
    let mut wire = request.into_bytes();
    if let Some((_, bytes)) = body {
        wire.extend_from_slice(bytes);
    }
    stream
        .write_all(&wire)
        .map_err(|error| DoorError::Unresponsive(error.to_string()))?;
    let mut raw = Vec::new();
    loop {
        if raw.len() >= MAX_RESPONSE_BYTES {
            return Err(DoorError::Unresponsive(format!(
                "response exceeds {MAX_RESPONSE_BYTES} bytes"
            )));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(DoorError::Unresponsive(format!(
                "response deadline exceeded after {} ms",
                read_budget.as_millis()
            )));
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|error| DoorError::Unresponsive(error.to_string()))?;
        let mut chunk = [0_u8; 8192];
        let chunk_limit = chunk.len().min(MAX_RESPONSE_BYTES - raw.len());
        let read = stream
            .read(&mut chunk[..chunk_limit])
            .map_err(|error| DoorError::Unresponsive(error.to_string()))?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..read]);
    }
    Response::parse(&raw)
}

/// The momentary liveness probe behind door-class verbs' connect-refused
/// branch (D-16-1-D): one `flock` attempt on the home directory's own
/// handle, released when the handle drops. No lock file is ever created.
///
/// ⚠ EXCLUSIVE, although the D-16-1-D prose says "shared probe" — that prose
/// is measurably wrong: roots hold `LOCK_SH`, and a shared probe is
/// compatible with a shared hold, so it could never see a live daemon. AC5's
/// vector (`control.json` present, nothing listening, a root on an
/// env-override endpoint holding the home ⇒ `StoreInUse`) is decidable only
/// with an exclusive attempt. `EWOULDBLOCK` ⇒ someone holds the set (a
/// daemon root or an offline child — V-4 concedes `flock` cannot tell them
/// apart, hence the neutral `StoreInUse`); success ⇒ genuinely free ⇒
/// `DaemonNotRunning`.
#[cfg(unix)]
pub fn probe_home_lock() -> Result<bool, String> {
    use rustix::fs::FlockOperation;
    let Some(home) = maos_domain::operator_door::maos_home().map_err(|error| error.to_string())?
    else {
        return Ok(false); // no home ⇒ nothing can hold its lock
    };
    let file = match std::fs::File::open(&home) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "maos: cannot open home {}: {error}",
                home.display()
            ))
        }
    };
    match rustix::fs::flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(false), // released when `file` drops at scope end
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(true),
        Err(error) => Err(format!(
            "maos: cannot probe the lock on home {}: {error}",
            home.display()
        )),
    }
}

/// Map a door response to the D-16-1-P exit table, printing a 200 body
/// verbatim. `verb` prefixes every diagnostic; output is ANSI-free.
pub fn map_response(verb: &str, response: Response) -> ExitCode {
    match response.status {
        200 => {
            // Terminal-code verbs (uninstall 0/3/4/5, forget 0/3, precheck
            // 0/2) carry their preserved one-shot code in the body; the door
            // mints no process exit codes itself.
            let terminal_code = response
                .body_json()
                .get("terminal_code")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0)
                .clamp(0, 255) as u8;
            print_body(&response.body);
            ExitCode::from(terminal_code)
        }
        401 => {
            eprintln!(
                "maosctl: {verb} — the operator bearer was rejected (HTTP 401); the token in \
                 the environment or control.json does not match this daemon"
            );
            ExitCode::from(77)
        }
        503 => map_retryable_503(verb, &response.body),
        status => {
            let (code, detail) = response.error_parts();
            // 404/409/500 typed outcomes and 400/405/411/413 protocol errors
            // are all exit 1 per D-16-1-P; the stderr names the status so a
            // client/daemon version skew is diagnosable.
            if detail.is_empty() {
                eprintln!("maosctl: {verb} — {code} (HTTP {status})");
            } else {
                eprintln!("maosctl: {verb} — {code}: {detail} (HTTP {status})");
            }
            ExitCode::from(1)
        }
    }
}

/// The retryable 503 family (D-16-1-P): everything here is EX_TEMPFAIL 75 —
/// nothing started (`busy`, `spirit_busy`), or it started and outlived the
/// route budget but WILL complete (`handler_still_running`, whose
/// `operation_id` is the lookup key into the Transparency Log).
fn map_retryable_503(verb: &str, body: &[u8]) -> ExitCode {
    let parsed: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null); // xtask-serde-allow: foreign 503 body; retry classification must survive a non-JSON payload
    let code = parsed
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    match code {
        "handler_still_running" => {
            let operation_id = parsed
                .get("operation_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<unknown>");
            eprintln!(
                "maosctl: {verb} — the command outlived its route budget but is running to \
                 completion; operation_id {operation_id}; inspect its row later with: \
                 maosctl audit query --intent-contains {operation_id}"
            );
        }
        "spirit_busy" => {
            eprintln!(
                "maosctl: {verb} — spirit_busy: another command on this spirit holds the \
                 per-spirit serialization lock; nothing was started; retry"
            );
        }
        _ => {
            eprintln!(
                "maosctl: {verb} — busy: the door's worker pool is saturated; nothing was \
                 started; retry"
            );
        }
    }
    ExitCode::from(75)
}

/// Map a transport/discovery failure to the D-16-1-P exit table.
pub fn map_error(verb: &str, error: DoorError) -> ExitCode {
    match error {
        DoorError::NotInitialized(message) | DoorError::Config(message) => {
            eprintln!("{message}");
            ExitCode::from(78)
        }
        DoorError::Unresponsive(detail) => {
            eprintln!(
                "maosctl: {verb} — DoorUnresponsive: the door accepted the connection but \
                 sent no well-formed HTTP response: {detail}"
            );
            ExitCode::from(69)
        }
        // Callers branch on this BEFORE mapping; reaching it here means a
        // caller forgot the offline/probe fork.
        DoorError::ConnectRefused => {
            eprintln!("maosctl: {verb} — the door refused the connection");
            ExitCode::from(69)
        }
    }
}

/// Door-class verbs' connect-refused branch: one momentary exclusive probe
/// of the home lock tells `DaemonNotRunning` (free) from `StoreInUse` (held)
/// — see [`probe_home_lock`] for why the probe is exclusive. Non-unix has no
/// probe and no release target ⇒ typed `OfflineUnsupported`.
pub fn door_class_connect_refused(verb: &str, endpoint: SocketAddr) -> ExitCode {
    #[cfg(unix)]
    {
        match probe_home_lock() {
            Ok(false) => {
                eprintln!(
                    "maosctl: {verb} — DaemonNotRunning: nothing is listening at {endpoint} \
                     and nothing holds the home's store locks; start a daemon with \
                     `maos run` or `maos shell`"
                );
                ExitCode::from(69)
            }
            Ok(true) => {
                eprintln!(
                    "maosctl: {verb} — StoreInUse: nothing is listening at {endpoint} and the \
                     home's stores are locked by a daemon root or an offline operation; \
                     retry when it exits"
                );
                ExitCode::from(69)
            }
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(78)
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (verb, endpoint);
        eprintln!(
            "maosctl: {verb} — OfflineUnsupported: the daemon-liveness lock probe exists \
             only on unix"
        );
        ExitCode::from(69)
    }
}

/// Durable verbs' non-unix arm: the offline child cannot take the store lock
/// set there, and "never offline while a daemon may run" has no substitute —
/// typed `OfflineUnsupported` (no release target is non-unix).
#[cfg(not(unix))]
pub fn offline_unsupported(verb: &str) -> ExitCode {
    eprintln!(
        "maosctl: {verb} — OfflineUnsupported: the exclusive store-lock offline arm exists \
         only on unix; configure the door (env pair or `maos init`) to reach a running daemon"
    );
    ExitCode::from(69)
}

/// Print a door body to stdout verbatim (compact JSON in, compact JSON out).
fn print_body(body: &[u8]) {
    let mut out = std::io::stdout();
    let _ = out.write_all(body);
    let _ = out.write_all(b"\n");
}

/// The client-side twin of the server's id validation: same charset, same
/// bound, so a request the door must refuse never leaves maosctl. `Err`
/// carries the operator-facing reason.
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 {
        return Err("identifiers must be 1..=128 characters".to_owned());
    }
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(
            "identifiers must match [A-Za-z0-9._-] (the door route segment charset)".to_owned(),
        );
    }
    Ok(())
}
