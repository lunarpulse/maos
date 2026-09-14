#![allow(dead_code)]

//! Story 16-1 / T9 — the hand-rolled fixture door behind the maos-cli
//! contract tests (AC7).
//!
//! # Why hand-rolled, not `maos-control`
//!
//! `maos-control` links `maos-kernel-core`, and
//! `dep_kernel_core_free_test.rs` runs plain `cargo tree -p maos-cli`
//! (dev edges included). A dev-dependency on `maos-control` here would put
//! `maos-kernel-core` back into that tree and red the gate. A
//! `std::net::TcpListener` on loopback needs nothing but `std`, so the edge
//! ban costs the fixture nothing.
//!
//! # What makes these CONTRACT tests (E15-A6)
//!
//! Everything a test asserts about this fixture falls in one of two halves,
//! and both have production code between the planting and the reading:
//!
//! 1. *what maosctl sent* — the test plants `control.json` (the discovery
//!    contract), the client sends a request, and the recorded method, path,
//!    bearer and body are what `door_client::exchange` actually put on the
//!    wire; and
//! 2. *what maosctl did with the reply* — the door's status + body are the
//!    test's INPUT, and the child's exit code / stderr are the production
//!    mapping (`map_response` / `map_error`, D-16-1-P).
//!
//! Nothing here reads back a value it planted without the child in between.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

/// The bearer token the fixture's `control.json` carries. A fixed 64-hex
/// literal: what matters to the contract is that the child presents THIS
/// value, because it is the one the discovery file holds.
pub const TOKEN: &str = "4f1ec1a7b0d24e5c9a3d8f06b7c2e151d94a0c38f5e26b1a7d3c94e0b28f61ad";

/// One request the fixture door received — the headers the contract names,
/// plus the raw body bytes.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    /// The full header value, `Bearer <token>` included.
    pub authorization: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub body: Vec<u8>,
}

impl RecordedRequest {
    /// The body as JSON, for field-by-field assertions (never a
    /// byte-for-byte string compare — the JSON is the contract, its
    /// whitespace is not).
    pub fn json_body(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }
}

/// A canned reply.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    pub fn new(status: u16, body: &str) -> Self {
        Reply {
            status,
            body: body.to_owned(),
        }
    }
}

/// The fixture door: a loopback listener, a scratch `MAOS_HOME` holding a
/// valid `control.json`, and the record of everything that arrived.
///
/// Dropping it removes the scratch home; the accept thread is detached and
/// dies with the process — a test binary never outlives its tests.
pub struct FixtureDoor {
    addr: SocketAddr,
    scratch: TempDir,
    maos_home: PathBuf,
    recorded: Arc<Mutex<Vec<RecordedRequest>>>,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    /// True for [`FixtureDoor::spawn_dead`] — no listener exists, and the
    /// tests that use it assert exactly that (exit 69, zero round trips).
    dead: bool,
}

impl FixtureDoor {
    /// A live door answering `200 {}` to every request.
    pub fn spawn() -> Self {
        Self::spawn_scripted(vec![Reply::new(200, "{}")])
    }

    /// A live door answering one canned reply to every request.
    pub fn spawn_replying(status: u16, body: &str) -> Self {
        Self::spawn_scripted(vec![Reply::new(status, body)])
    }

    /// A live door answering each request with the next scripted reply,
    /// cycling back to the first when the script is exhausted.
    pub fn spawn_scripted(replies: Vec<Reply>) -> Self {
        Self::build(replies, true)
    }

    /// `control.json` pointing at a port NOTHING listens on: the AC5
    /// "daemon down" fixture. The connect is refused, which is the point.
    pub fn spawn_dead() -> Self {
        Self::build(Vec::new(), false)
    }

    fn build(replies: Vec<Reply>, live: bool) -> Self {
        let scratch = TempDir::new().expect("scratch home for the fixture door");
        let maos_home = scratch.path().join("maos-home");
        // D-16-1-Q: HOME and XDG_DATA_HOME are isolated alongside MAOS_HOME
        // so no store, journal or memory root reaches the developer's tree.
        std::fs::create_dir_all(scratch.path().join("home")).expect("scratch HOME");
        std::fs::create_dir_all(scratch.path().join("xdg")).expect("scratch XDG_DATA_HOME");
        std::fs::create_dir_all(&maos_home).expect("scratch MAOS_HOME");

        let (addr, listener) = match live {
            true => {
                let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                    .expect("bind loopback fixture door");
                (listener.local_addr().expect("local addr"), Some(listener))
            }
            // Bind once to learn a free port, then DROP the listener: the
            // discovery file now names an address where connect(2) is
            // refused — the deterministic "nothing is listening" state.
            false => {
                let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("probe a free port");
                let addr = probe.local_addr().expect("local addr");
                (addr, None)
            }
        };

        // The same type `maos init` writes, through the same atomic
        // mode-0600 write path — so the discovery half of the contract is
        // exercised against the real bytes, not a hand-rolled lookalike.
        let control = maos_domain::operator_door::ControlFile {
            version: maos_domain::operator_door::CONTROL_FILE_VERSION,
            endpoint: format!("{}{addr}", maos_domain::operator_door::ENDPOINT_SCHEME),
            token: TOKEN.to_owned(),
        };
        control
            .write_atomic(&maos_domain::operator_door::control_file_path(&maos_home))
            .expect("write control.json");

        let recorded = Arc::new(Mutex::new(Vec::new()));
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        if let Some(listener) = listener {
            let recorded = Arc::clone(&recorded);
            let replies = Arc::clone(&replies);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let reply = next_reply(&replies);
                    serve(stream, reply, &recorded);
                }
            });
        }

        FixtureDoor {
            addr,
            scratch,
            maos_home,
            recorded,
            replies,
            dead: !live,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn token(&self) -> &'static str {
        TOKEN
    }

    /// The scratch `MAOS_HOME` — set as the child's `MAOS_HOME` by
    /// [`FixtureDoor::run`].
    pub fn maos_home(&self) -> &Path {
        &self.maos_home
    }

    /// A directory the child may use as `HOME` holding NO `.maos` — so a
    /// discovery regression cannot silently find a second door.
    pub fn home_dir(&self) -> PathBuf {
        self.scratch.path().join("home")
    }

    pub fn xdg_dir(&self) -> PathBuf {
        self.scratch.path().join("xdg")
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Every request the door received so far, oldest first.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.recorded.lock().expect("recorded lock").clone()
    }

    pub fn request_count(&self) -> usize {
        self.recorded.lock().expect("recorded lock").len()
    }

    /// Replace the reply script (the door keeps its recorded requests).
    pub fn rewire(&self, replies: Vec<Reply>) {
        *self.replies.lock().expect("replies lock") = VecDeque::from(replies);
    }

    /// Widen `control.json` to 0644: the D-16-1-C custody refusal fixture
    /// (the token in a world-readable file is not a secret).
    #[cfg(unix)]
    pub fn widen_control_file(&self) {
        use std::os::unix::fs::PermissionsExt;
        let path = maos_domain::operator_door::control_file_path(&self.maos_home);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod 0644 control.json");
    }

    /// Run `maosctl` against this door with the D-16-1-Q isolation env:
    /// `env_clear`, `PATH` restored, per-fixture `HOME` / `MAOS_HOME` /
    /// `XDG_DATA_HOME`. Nothing else leaks in — notably no
    /// `MAOS_OPERATOR_*` pair and no developer `HOME`.
    pub fn run(&self, args: &[&str]) -> std::process::Output {
        self.run_with(&[], args)
    }

    /// [`FixtureDoor::run`] with extra env layered on top (e.g.
    /// `MAOS_BIN_PATH` pointed at a witness script).
    pub fn run_with(&self, extra_env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
        let mut cmd = Command::new(maosctl());
        cmd.env_clear();
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        cmd.env("HOME", self.home_dir());
        cmd.env("MAOS_HOME", self.maos_home());
        cmd.env("XDG_DATA_HOME", self.xdg_dir());
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        cmd.args(args);
        cmd.output().expect("spawn maosctl")
    }
}

/// `maosctl` from the same package (cargo injects the bin path).
pub fn maosctl() -> &'static str {
    env!("CARGO_BIN_EXE_maosctl")
}

/// Pop the next scripted reply, cycling back to the first when the script
/// is exhausted; an empty script means `200 {}`.
fn next_reply(replies: &Mutex<VecDeque<Reply>>) -> Reply {
    let mut queue = replies.lock().expect("replies lock");
    match queue.pop_front() {
        Some(reply) => {
            queue.push_back(reply.clone());
            reply
        }
        None => Reply::new(200, "{}"),
    }
}

/// Serve ONE connection: read exactly one request (headers + the
/// Content-Length the client always declares — a body-less POST declares
/// zero, `door_client::exchange`), record it, answer with `reply`, close.
fn serve(mut stream: TcpStream, reply: Reply, recorded: &Mutex<Vec<RecordedRequest>>) {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (head_end, body_start) = loop {
        if let Some(split) = find_window(&raw) {
            break split;
        }
        let read = stream.read(&mut chunk).expect("read fixture request");
        if read == 0 {
            panic!("fixture door: client closed before sending a full header block");
        }
        raw.extend_from_slice(&chunk[..read]);
    };

    let head_text = String::from_utf8_lossy(&raw[..head_end]).to_owned();
    let mut lines = head_text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();

    let mut authorization = None;
    let mut content_type = None;
    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.to_owned()),
            "content-type" => content_type = Some(value.to_owned()),
            "content-length" => content_length = value.parse::<usize>().ok(),
            _ => {}
        }
    }

    let mut body = raw[body_start..].to_vec();
    let wanted = content_length.unwrap_or(0);
    while body.len() < wanted {
        let read = stream.read(&mut chunk).expect("read fixture body");
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(wanted);
    recorded
        .lock()
        .expect("recorded lock")
        .push(RecordedRequest {
            method,
            path,
            authorization,
            content_type,
            content_length,
            body,
        });

    let phrase = match reply.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        reply.status,
        phrase,
        reply.body.len(),
        reply.body
    );
    // `door_client::exchange` reads to EOF, so dropping the stream IS the
    // response terminator.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    drop(stream);
}

/// Locate the `\r\n\r\n` header terminator; `Some((head_end, body_start))`.
fn find_window(raw: &[u8]) -> Option<(usize, usize)> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| (at, at + 4))
}

// ─── The D-16-1-P exit table, shared verbatim by all seven verb files ────

/// One typed-response case of the D-16-1-P matrix.
pub struct TypedCase {
    pub label: &'static str,
    pub status: u16,
    pub body: &'static str,
    pub expected_exit: u8,
    /// Substrings the stderr MUST carry (the operation id, the status
    /// number, the audit-query remedy).
    pub stderr_contains: &'static [&'static str],
}

/// The full response→exit mapping every door verb shares (D-16-1-P). The
/// mapping lives once in `door_client`; each verb file proves ITS verb
/// reaches it by running this matrix against its own route.
pub fn d16_1_p_cases() -> Vec<TypedCase> {
    vec![
        TypedCase {
            label: "404 spirit_not_loaded",
            status: 404,
            body: r#"{"error":"spirit_not_loaded"}"#,
            expected_exit: 1,
            stderr_contains: &["spirit_not_loaded", "HTTP 404"],
        },
        TypedCase {
            label: "409 invalid_state_transition",
            status: 409,
            body: r#"{"error":"invalid_state_transition"}"#,
            expected_exit: 1,
            stderr_contains: &["invalid_state_transition", "HTTP 409"],
        },
        TypedCase {
            label: "401 rejected bearer",
            status: 401,
            body: "{}",
            expected_exit: 77,
            stderr_contains: &["HTTP 401"],
        },
        TypedCase {
            label: "503 handler_still_running",
            status: 503,
            body: r#"{"error":"handler_still_running","operation_id":"op-7"}"#,
            expected_exit: 75,
            stderr_contains: &["op-7", "maosctl audit query --intent-contains"],
        },
        TypedCase {
            label: "503 spirit_busy",
            status: 503,
            body: r#"{"error":"spirit_busy"}"#,
            expected_exit: 75,
            stderr_contains: &[],
        },
        TypedCase {
            label: "503 busy",
            status: 503,
            body: r#"{"error":"busy"}"#,
            expected_exit: 75,
            stderr_contains: &[],
        },
        TypedCase {
            label: "400 protocol error",
            status: 400,
            body: "{}",
            expected_exit: 1,
            stderr_contains: &["HTTP 400"],
        },
        TypedCase {
            label: "405 protocol error",
            status: 405,
            body: "{}",
            expected_exit: 1,
            stderr_contains: &["HTTP 405"],
        },
        TypedCase {
            label: "411 protocol error",
            status: 411,
            body: "{}",
            expected_exit: 1,
            stderr_contains: &["HTTP 411"],
        },
        TypedCase {
            label: "413 protocol error",
            status: 413,
            body: "{}",
            expected_exit: 1,
            stderr_contains: &["HTTP 413"],
        },
    ]
}

/// Drive the full D-16-1-P matrix for one verb invocation shape: for every
/// typed case a fresh door answers `status`+`body`; the child must exit
/// with the mapped code, its stderr must carry every required substring,
/// and exactly one request with the expected method, route and BEARER FROM
/// `control.json` must have hit the door.
pub fn assert_typed_matrix(args: &[&str], expected_method: &str, expected_route: &str) {
    for case in d16_1_p_cases() {
        let door = FixtureDoor::spawn_replying(case.status, case.body);
        let out = door.run(args);
        let code = out.status.code().unwrap_or(255);
        assert_eq!(
            code,
            case.expected_exit as i32,
            "[{}]: expected exit {}, got {} (stdout: {}, stderr: {})",
            case.label,
            case.expected_exit,
            code,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        for needle in case.stderr_contains {
            assert!(
                stderr.contains(needle),
                "[{}]: stderr must name {needle:?} — got: {stderr}",
                case.label
            );
        }
        let requests = door.requests();
        assert_eq!(
            requests.len(),
            1,
            "[{}]: exactly one round trip expected, door saw {}",
            case.label,
            requests.len()
        );
        assert_eq!(
            requests[0].method, expected_method,
            "[{}]: wrong method on the wire",
            case.label
        );
        assert_eq!(
            requests[0].path, expected_route,
            "[{}]: request hit the wrong route",
            case.label
        );
        assert_eq!(
            requests[0].authorization.as_deref(),
            Some(format!("Bearer {}", door.token()).as_str()),
            "[{}]: the bearer must be the token from control.json",
            case.label
        );
    }
}

/// The four discovery failures every door verb shares (AC5), which need no
/// live door: absent config ⇒ 78 naming `maos init`; half-set env pair ⇒
/// 78; world-readable config ⇒ 78 naming the mode; configured-but-dead
/// endpoint ⇒ 69 `DaemonNotRunning`.
pub fn assert_discovery_failures(verb_args: &[&str]) {
    // 1. No control.json, no env pair — the pristine-home case.
    let bare = TempDir::new().expect("scratch for missing control.json");
    let home = bare.path().join("home");
    let maos_home = bare.path().join("maos-home");
    std::fs::create_dir_all(&home).expect("scratch HOME");
    std::fs::create_dir_all(&maos_home).expect("scratch MAOS_HOME");
    let out = run_isolated(
        &[
            ("HOME", home.to_str().expect("utf8 home")),
            ("MAOS_HOME", maos_home.to_str().expect("utf8 maos home")),
            (
                "XDG_DATA_HOME",
                bare.path().join("xdg").to_str().expect("utf8 xdg"),
            ),
        ],
        verb_args,
    );
    assert_eq!(
        out.status.code(),
        Some(78),
        "missing control.json must exit 78 — got {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("maos init"),
        "exit 78 must name the remedy `maos init` — got: {stderr}"
    );
    drop(bare);

    // 2. Half-set env pair — BOTH halves, because each alone must refuse
    //    (both-or-neither, a half pair is a config error, not a partial
    //    override). The valid control.json stays in place: the pair takes
    //    priority, so the refusal must fire WITHOUT a round trip.
    for (set_key, label) in [
        ("MAOS_OPERATOR_HTTP_ENDPOINT", "endpoint-only"),
        ("MAOS_OPERATOR_BEARER_TOKEN", "token-only"),
    ] {
        let door = FixtureDoor::spawn();
        let extra: Vec<(&str, &str)> = vec![(set_key, "tcp://127.0.0.1:1")];
        let out = door.run_with(&extra, verb_args);
        assert_eq!(
            out.status.code(),
            Some(78),
            "[half-set pair, {label}]: must exit 78 — got {:?}, stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("must be set together"),
            "[half-set pair, {label}]: stderr must name the both-or-neither rule — got: {stderr}"
        );
        assert_eq!(
            door.request_count(),
            0,
            "[half-set pair, {label}]: a refused half-pair must not reach the door"
        );
    }

    // 3. `control.json` with mode 0644 — custody refused, mode named.
    //    (Unix-gated with the chmod; the custody rule is mode-based only
    //    there.)
    #[cfg(unix)]
    {
        let door = FixtureDoor::spawn();
        door.widen_control_file();
        let out = door.run(verb_args);
        assert_eq!(
            out.status.code(),
            Some(78),
            "0644 control.json must exit 78 — got {:?}, stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("0644"),
            "exit 78 must name the offending mode — got: {stderr}"
        );
        assert_eq!(
            door.request_count(),
            0,
            "a custody-refused config must never reach the door"
        );
    }

    // 4. Configured endpoint with NOTHING listening — connect refused, home
    //    lock free ⇒ typed DaemonNotRunning 69.
    let door = FixtureDoor::spawn_dead();
    let out = door.run(verb_args);
    assert_eq!(
        out.status.code(),
        Some(69),
        "dead door must exit 69 — got {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("DaemonNotRunning"),
        "exit 69 must be typed DaemonNotRunning (the home lock is free) — got: {stderr}"
    );
}

/// A bare env-isolated `maosctl` run for the discovery vectors (no door).
fn run_isolated(env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(maosctl());
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.args(args);
    cmd.output().expect("spawn maosctl")
}

/// A stand-in `maos` binary whose only job is to TOUCH A FILE when run —
/// the witness proving a door verb spawned NO child (§4: the one-shot
/// pause wrote journal rows while the spirit kept running; the contract is
/// that these verbs spawn nothing at all).
#[cfg(unix)]
pub fn witness_script(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let witness = dir.join(format!("{name}.witness"));
    let script = dir.join(format!("{name}.fake-maos"));
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", witness.display()),
    )
    .expect("write witness script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod witness script");
    (script, witness)
}
