#![forbid(unsafe_code)]

//! `maos-journey-test` — the MAOS journey-acceptance test harness.
//!
//! ## Coverage boundary
//!
//! This harness proves MAOS orchestration / audit / halt / budget / MCP / render
//! correctness given recorded inputs. It does NOT prove LLM reasoning quality
//! (see the eval corpora and NFR-Aud gates) or live-API non-drift (see Tier-2
//! nightly).
//!
//! ## Journey grades
//!
//! - **Grade A** (production entry surface): J0, J-Butler, J-Researcher —
//!   full PTY beat-by-beat suites via `maos run`.
//! - **Grade B** (orchestrated smoke wrap with receiver-side oracles): J1, J4 —
//!   until the 8.12 founder-class gap closes.
//!
//! ## Architecture
//!
//! Each journey test builds a [`JourneyWorld`] (mock MCPs, replay cassette,
//! isolated audit dir, pinned clock) then spawns the real `maos run` daemon in a
//! [`Pty`]. The daemon uses env seams (`MAOS_REPLAY_CASSETTE`, `MAOS_MCP_*_URI`,
//! `MAOS_HOME`, `XDG_DATA_HOME`) to consume harness-controlled inputs. Assertions
//! hit three surfaces: PTY screen render ([`Screen::contains`]), MockMcp request
//! log ([`MockMcp::writes`]), and the transparency log via `maos_audit::query`.
//!
//! ## What 8.11 made real vs. stubs
//! - [`Screen::contains`] is REAL (a byte search) — it is the assertion surface.
//! - [`Pty::screen`] returns a real `vt100` render of the PTY output.
//! - [`MockMcp`] is a real HTTP MCP server returning fixture-seeded responses.
//! - [`ReplayProvider`] writes a cassette file the daemon-side replay port reads.

use std::collections::BTreeMap;
use std::io::Read as IoRead;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;

/// The world a journey test drives: a pinned clock, mock MCP endpoints, a replay
/// LLM provider, and a temp audit DB. Construct via [`JourneyWorld::builder`].
pub struct JourneyWorld {
    _clock: TestClock,
    _mcp: BTreeMap<String, MockMcp>,
    llm: ReplayProvider,
    _audit: AuditDb,
    env: BTreeMap<String, String>,
}

impl JourneyWorld {
    pub fn builder() -> JourneyWorldBuilder {
        JourneyWorldBuilder::default()
    }

    pub fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    /// Access a named MockMcp endpoint for post-spawn write-oracle assertions.
    pub fn mcp(&self, name: &str) -> Option<&MockMcp> {
        self._mcp.get(name)
    }
}

/// Fluent builder for [`JourneyWorld`] (the JB-* shared construction surface).
#[derive(Default)]
pub struct JourneyWorldBuilder {
    clock: Option<TestClock>,
    mcp: BTreeMap<String, MockMcp>,
    llm: Option<ReplayProvider>,
    audit: Option<AuditDb>,
    /// Story 16-2 / D-16-2-K — generic env additions. `Pty::spawn` splits
    /// its command on whitespace and execs it, so env can never ride the
    /// command string; J0 sets `MAOS_INFERENCE_MODE=replay` here (which also
    /// overrides a job-wide record mode — J0's seed cassette is not a
    /// re-record target).
    extra_env: BTreeMap<String, String>,
}

impl JourneyWorldBuilder {
    pub fn clock(mut self, clock: TestClock) -> Self {
        self.clock = Some(clock);
        self
    }

    pub fn mcp(mut self, server: &str, mock: MockMcp) -> Self {
        self.mcp.insert(server.to_string(), mock);
        self
    }

    pub fn llm(mut self, provider: ReplayProvider) -> Self {
        self.llm = Some(provider);
        self
    }

    pub fn audit(mut self, audit: AuditDb) -> Self {
        self.audit = Some(audit);
        self
    }

    /// Story 16-2 / D-16-2-K — set one environment variable on every child
    /// the world spawns (generic: no per-journey endpoint on the builder).
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.extra_env.insert(key.to_string(), value.to_string());
        self
    }

    pub fn cassette(mut self, path: &str) -> Self {
        self.llm = Some(ReplayProvider::cassette(path));
        self
    }

    pub fn build(self) -> JourneyWorld {
        let audit = self.audit.unwrap_or_default();
        let llm = self.llm.unwrap_or_default();
        let mcp = self.mcp;

        let mut env = BTreeMap::new();
        env.insert(
            "MAOS_HOME".into(),
            audit.path().to_string_lossy().into_owned(),
        );
        env.insert(
            "XDG_DATA_HOME".into(),
            audit.path().join("xdg").to_string_lossy().into_owned(),
        );
        // Story 16-1 / D-16-1-Q: every Pty child gets an empty scratch "HOME" —
        // no control.json, so journey roots cannot collide on one endpoint.
        env.insert(
            "HOME".into(),
            doorless_home::doorless_home()
                .to_string_lossy()
                .into_owned(),
        );
        if let Some(cassette_path) = llm.cassette_path() {
            env.insert(
                "MAOS_REPLAY_CASSETTE".into(),
                cassette_path.to_string_lossy().into_owned(),
            );
        }
        for (key, value) in &self.extra_env {
            env.insert(key.clone(), value.clone());
        }
        for (server_name, mock) in &mcp {
            let env_key = match server_name.as_str() {
                "calendar" => "MAOS_MCP_CALENDAR_URI",
                "slack" => "MAOS_MCP_SLACK_URI",
                "linear" => "MAOS_MCP_LINEAR_URI",
                "figma" => "MAOS_MCP_FIGMA_URI",
                "web" => "MAOS_MCP_WEB_URI",
                "arxiv" => "MAOS_MCP_ARXIV_URI",
                "github" => "MAOS_MCP_GITHUB_URI",
                "citation-graph" | "citation_graph" => "MAOS_MCP_CITATION_GRAPH_URI",
                _ => continue,
            };
            env.insert(env_key.into(), mock.url().to_string());
        }

        JourneyWorld {
            _clock: self.clock.unwrap_or_default(),
            _mcp: mcp,
            llm,
            _audit: audit,
            env,
        }
    }
}

/// A pinned virtual clock (H2 guard: one T0 governs the whole world).
#[derive(Default)]
pub struct TestClock {
    _t0_min_of_day: u32,
}

impl TestClock {
    /// Tuesday 1:00pm — the J-Butler scenario T0.
    pub fn tuesday_1pm() -> Self {
        Self {
            _t0_min_of_day: 13 * 60,
        }
    }
}

/// A captured request from a [`MockMcp`] server.
pub struct McpRequestCapture {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

/// A mock MCP endpoint seeded from fixture JSON responses.
///
/// Implements H3 (ephemeral port + readback) and H4 (readiness handshake via
/// immediate bind). Modeled on `spawn_mock_mcp_server` (`butler_8_14b.rs:149`).
pub struct MockMcp {
    _fixture: String,
    url: String,
    writes_rx: Mutex<mpsc::Receiver<McpRequestCapture>>,
    /// Shared shutdown flag — set to `true` on drop so the server thread exits.
    shutdown: Arc<AtomicBool>,
    /// Handle to the server thread so we can join on drop.
    _server_handle: Option<std::thread::JoinHandle<()>>,
}

impl Default for MockMcp {
    fn default() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (_tx, rx) = mpsc::channel();
        drop(listener);
        Self {
            _fixture: String::new(),
            url: format!("http://{addr}"),
            writes_rx: Mutex::new(rx),
            shutdown: Arc::new(AtomicBool::new(false)),
            _server_handle: None,
        }
    }
}

impl MockMcp {
    pub fn calendar(fixture_path: &str) -> Self {
        Self::from_fixture(fixture_path)
    }

    pub fn from_fixture(fixture_path: &str) -> Self {
        let content = std::fs::read_to_string(fixture_path)
            .unwrap_or_else(|e| panic!("MockMcp: failed to read fixture {fixture_path}: {e}"));
        Self::from_responses(vec![content])
    }

    pub fn from_responses(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        listener
            .set_nonblocking(true)
            .expect("MockMcp: set_nonblocking failed");
        let (tx, rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        let handle = std::thread::spawn(move || {
            for resp_body in responses {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }
                // Poll accept with a busy-wait; the shutdown flag breaks the loop.
                let (mut stream, _) = loop {
                    if shutdown_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    match listener.accept() {
                        Ok(s) => break s,
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(_) => return,
                    }
                };
                let mut bytes = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = match stream.read(&mut buf) {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if bytes.windows(4).position(|w| w == b"\r\n\r\n").is_some() {
                        break;
                    }
                }
                if bytes.is_empty() {
                    break;
                }
                let headers = String::from_utf8_lossy(&bytes);
                let first_line = headers.lines().next().unwrap_or("");
                let mut parts = first_line.split_whitespace();
                let method = parts.next().unwrap_or("").to_string();
                let path = parts.next().unwrap_or("").to_string();
                let mut content_length = 0usize;
                for line in headers.lines() {
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            content_length = value.trim().parse().unwrap_or(0);
                        }
                    }
                }
                let header_len = headers.find("\r\n\r\n").unwrap_or(headers.len()) + 4;
                while bytes.len() - header_len < content_length {
                    let n = match stream.read(&mut buf) {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                }
                let body = if bytes.len() >= header_len + content_length {
                    bytes[header_len..header_len + content_length].to_vec()
                } else {
                    vec![]
                };

                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                let _ = std::io::Write::write_all(&mut stream, http_response.as_bytes());
                let _ = tx.send(McpRequestCapture { method, path, body });
            }
        });
        Self {
            _fixture: String::new(),
            url: format!("http://{addr}"),
            writes_rx: Mutex::new(rx),
            shutdown,
            _server_handle: Some(handle),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn writes(&self) -> Vec<McpRequestCapture> {
        let rx = self.writes_rx.lock().unwrap();
        let mut captures = Vec::new();
        while let Ok(cap) = rx.try_recv() {
            captures.push(cap);
        }
        captures
    }
}

impl Drop for MockMcp {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self._server_handle.take() {
            let _ = handle.join();
        }
    }
}

/// A replay/record LLM provider keyed by a cassette file.
pub struct ReplayProvider {
    cassette_source: Option<PathBuf>,
    cassette_file: Option<PathBuf>,
}

impl Default for ReplayProvider {
    fn default() -> Self {
        Self {
            cassette_source: None,
            cassette_file: None,
        }
    }
}

impl ReplayProvider {
    pub fn cassette(path: &str) -> Self {
        let src = Path::new(path);
        let tmp = tempfile::Builder::new()
            .prefix("maos-cassette-")
            .suffix(".json")
            .tempfile()
            .expect("ReplayProvider: failed to create temp cassette file");
        let dest = tmp.into_temp_path().to_path_buf();
        if src.exists() {
            std::fs::copy(src, &dest)
                .unwrap_or_else(|e| panic!("ReplayProvider: failed to copy cassette {path}: {e}"));
        }
        Self {
            cassette_source: Some(src.to_path_buf()),
            cassette_file: Some(dest),
        }
    }

    /// Queue a computed scalar the Spirit emits this turn (e.g.
    /// `(butler::SCALAR_TAG_BELIEF_VARIANCE, 0.78)`).
    ///
    /// Placeholder for future cassette-injection use (no callers yet).
    /// TODO: inject queued scalars into the cassette JSON when a consumer exists.
    pub fn queue_scalar(&self, _tag: &str, _value: f64) {}

    pub fn cassette_path(&self) -> Option<&Path> {
        if std::env::var("MAOS_INFERENCE_MODE").as_deref() == Ok("record") {
            self.cassette_source.as_deref()
        } else {
            self.cassette_file.as_deref()
        }
    }
}

/// A temp-dir audit DB. The test subprocess writes its journal/audit here via
/// `MAOS_HOME` env seam. After the subprocess exits, use `maos_audit::query` on
/// `self.transparency_log_path()` to assert journaled rows.
pub struct AuditDb {
    _temp: (),
    dir: PathBuf,
    _tempdir_handle: Option<tempfile::TempDir>,
}

impl Default for AuditDb {
    fn default() -> Self {
        Self::temp()
    }
}

impl AuditDb {
    pub fn temp() -> Self {
        let td = tempfile::Builder::new()
            .prefix("maos-journey-audit-")
            .tempdir()
            .expect("AuditDb::temp(): failed to create tempdir");
        std::fs::create_dir_all(td.path().join("audit")).ok();
        std::fs::create_dir_all(td.path().join("journal")).ok();
        std::fs::create_dir_all(td.path().join("xdg").join("maos").join("audit")).ok();
        let path = td.path().to_path_buf();
        Self {
            _temp: (),
            dir: path,
            _tempdir_handle: Some(td),
        }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    pub fn transparency_log_path(&self) -> PathBuf {
        self.dir.join("audit").join("transparency.sqlite")
    }
}

/// A pseudo-terminal driving a `maos run ...` subprocess against a [`JourneyWorld`].
pub struct Pty {
    _command: String,
    child: Mutex<Option<Box<dyn portable_pty::Child + Send>>>,
    reader_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    screen_buf: Arc<Mutex<Vec<u8>>>,
    master: Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>,
    /// Story 16-2 / D-16-2-K — the master's writer, taken ONCE at spawn
    /// (`take_writer` yields the only handle). `send_line` and `send_eof`
    /// write through it; before 16-2 the harness could only READ a PTY.
    writer: parking_lot::Mutex<Option<Box<dyn std::io::Write + Send>>>,
}

impl Pty {
    /// Spawn the given `maos run ...` command against the world's seams.
    pub fn spawn(command: &str, world: &JourneyWorld) -> Self {
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows: 50,
                cols: 240,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Pty::spawn: failed to open PTY");

        let parts: Vec<&str> = command.split_whitespace().collect();
        let (prog, args) = if parts.is_empty() {
            panic!("Pty::spawn: empty command");
        } else {
            (parts[0], &parts[1..])
        };

        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(|d| PathBuf::from(d).join("..").join(".."))
            .unwrap_or_else(|_| PathBuf::from("."));

        let mut cmd = portable_pty::CommandBuilder::new(prog);
        cmd.args(args);
        cmd.cwd(&workspace_root);
        for (k, v) in world.env() {
            cmd.env(k, v);
        }

        // 15-6 §A6 review P1: a cassette-free world must not inherit an
        // outer MAOS_INFERENCE_MODE — the nightly rerecord leg exports
        // `record` job-wide, and the selector would refuse a cassette-free
        // child before the journey runs. Cassette-bearing worlds keep it:
        // they are that leg's re-record targets.
        if !world.env().contains_key("MAOS_REPLAY_CASSETTE") {
            cmd.env_remove("MAOS_INFERENCE_MODE");
        }
        let pty_writer = pair
            .master
            .take_writer()
            .expect("Pty::spawn: failed to take master writer");

        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("Pty::spawn: failed to spawn command");

        let screen_buf = Arc::new(Mutex::new(Vec::new()));
        let buf_clone = Arc::clone(&screen_buf);
        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("Pty::spawn: failed to clone PTY reader");

        let reader_handle = std::thread::spawn(move || {
            let mut tmp = [0u8; 4096];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf_clone.lock().unwrap().extend_from_slice(&tmp[..n]);
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            _command: command.to_string(),
            child: Mutex::new(Some(child)),
            reader_handle: Mutex::new(Some(reader_handle)),
            screen_buf,
            master: Mutex::new(Some(pair.master)),
            writer: parking_lot::Mutex::new(Some(pty_writer)),
        }
    }

    /// Story 16-2 / D-16-2-K — type a line into the child's stdin:
    /// CR-terminated (a PTY line ends at CR; LF alone does not submit in
    /// canonical mode).
    pub fn send_line(&self, line: &str) {
        let mut writer = self.writer.lock();
        let writer = writer
            .as_mut()
            .expect("Pty::send_line: writer taken (Pty dropped?)");
        writer
            .write_all(format!("{line}\r").as_bytes())
            .expect("Pty::send_line: write failed");
        writer.flush().expect("Pty::send_line: flush failed");
    }

    /// Story 16-2 / D-16-2-K — end the REPL: `0x04` (Ctrl-D) in canonical
    /// mode delivers EOF. `send_line` cannot end the session.
    pub fn send_eof(&self) {
        let mut writer = self.writer.lock();
        let writer = writer
            .as_mut()
            .expect("Pty::send_eof: writer taken (Pty dropped?)");
        writer
            .write_all(&[0x04])
            .expect("Pty::send_eof: write failed");
        writer.flush().expect("Pty::send_eof: flush failed");
    }

    /// The current rendered screen via `vt100::Parser`.
    pub fn screen(&self) -> Screen {
        let buf = self.screen_buf.lock().unwrap();
        let mut parser = vt100::Parser::new(50, 240, 0);
        parser.process(&buf);
        let text = parser.screen().contents();
        Screen(text)
    }

    /// Poll the rendered screen until it contains ANY of `needles`, or
    /// `timeout` elapses. Returns `true` if one appeared in time.
    ///
    /// The bounded wall-clock poll lives here in the harness (not in test
    /// bodies) so `tests/journey_*.rs` stay clean under the JB-7 / H4
    /// no-wallclock guard ([`guards::assert_no_wallclock_or_fixed_sleep`]).
    pub fn wait_for_screen(&self, needles: &[&str], timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let screen = self.screen();
            if needles.iter().any(|n| screen.contains(n)) {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Wait for the child process to exit with a 30 s timeout.
    ///
    /// Returns `Some(status)` if the child exited within the deadline,
    /// or `None` if it is still running (or was already reaped).
    pub fn wait(&self) -> Option<portable_pty::ExitStatus> {
        self.wait_with_timeout(Duration::from_secs(30))
    }

    /// Poll `try_wait()` at 100 ms intervals until `timeout` elapses.
    ///
    /// If the child hasn't exited by the deadline, kills it and returns `None`.
    pub fn wait_with_timeout(&self, timeout: Duration) -> Option<portable_pty::ExitStatus> {
        let mut child_lock = self.child.lock().unwrap();
        let child = match child_lock.as_mut() {
            Some(c) => c,
            None => return None,
        };
        let deadline = std::time::Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {} // still running
                Err(_) => return None,
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                return None;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            // Bounded wait: poll try_wait for up to 5 s, then give up.
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) if std::time::Instant::now() >= deadline => break,
                    Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                }
            }
        }
        drop(self.master.lock().unwrap().take());
        if let Some(handle) = self.reader_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

/// A rendered terminal screen. [`Screen::contains`] is REAL (the assertion
/// surface); the rendering that fills it is `vt100`-based.
pub struct Screen(String);

impl Screen {
    pub fn contains(&self, needle: &str) -> bool {
        self.0.contains(needle)
    }

    /// The raw rendered text (for richer assertions).
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// The world's replay LLM provider (the sketch's `world_llm` free-fn — bound
/// here so JB-3's `world_llm(&world).queue_scalar(...)` resolves).
pub fn world_llm(world: &JourneyWorld) -> &ReplayProvider {
    &world.llm
}

/// H1–H6 hermeticity guards.
pub mod guards {
    /// JB-7 — assert a test source reads no wall-clock and uses no fixed `sleep`.
    pub fn assert_no_wallclock_or_fixed_sleep(test_source_path: &str) {
        let content = std::fs::read_to_string(test_source_path)
            .unwrap_or_else(|e| panic!("guards: failed to read {test_source_path}: {e}"));

        let forbidden = [
            "Instant::now()",
            "SystemTime::now()",
            "thread::sleep(",
            "std::thread::sleep(",
            "tokio::time::sleep(",
            "sleep(Duration",
        ];

        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            for pat in &forbidden {
                if line.contains(pat) {
                    panic!(
                        "guards: H4 violation in {}:{}: forbidden pattern '{}' found in: {}",
                        test_source_path,
                        line_no + 1,
                        pat,
                        trimmed
                    );
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Story 16-2 / D-16-2-K — bounded subprocess helpers (the no-wallclock
// guard's rule: bounded waits live HERE, never in a test body)
// ─────────────────────────────────────────────────────────────────────────────

/// One bounded child run with the WORLD's env only — never the runner's
/// `HOME` (16-1's decoy door holds the runner's real `control.json`
/// endpoint inside the workspace suite, so any inherited home collides).
#[derive(Debug, Clone)]
pub struct BoundedRun {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Per-stream diagnostic capture ceiling. Readers continue draining after the
/// ceiling so a noisy child cannot block on a full pipe or grow the harness
/// without bound.
const CAPTURE_LIMIT_BYTES: usize = 8 * 1024 * 1024;

fn drain_bounded(mut pipe: impl std::io::Read) -> Vec<u8> {
    let mut captured = Vec::with_capacity(CAPTURE_LIMIT_BYTES);
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let Ok(read) = pipe.read(&mut chunk) else {
            break;
        };
        if read == 0 {
            break;
        }
        let remaining = CAPTURE_LIMIT_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&chunk[..read.min(remaining)]);
    }
    captured
}

/// Run `program args` against the world's env, bounded by `timeout_secs`.
/// `extra_env` rides ALONGSIDE the world env (e.g. a cassette copy).
/// The pipes drain on two threads WHILE the bounded loop runs (D-16-3-O), so
/// a child that fills either pipe buffer still exits inside the bound.
pub fn run_bounded(
    world: &JourneyWorld,
    program: &str,
    args: &[&str],
    extra_env: &[(&str, &str)],
    timeout_secs: u64,
    label: &str,
) -> BoundedRun {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default());
    for (k, v) in world.env() {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{label}: spawn failed: {e}"));
    // D-16-3-O — drain BOTH pipes WHILE the bounded loop runs, not after it:
    // a child writing past the OS pipe buffer (~64 KiB) blocks on write
    // forever when nobody reads, so the old drain-after-exit shape
    // (`wait_with_output` only once `try_wait` yielded `Some`) never reached
    // its drain, the deadline fired, and a full-pipe hang was misreported as
    // a timeout (`deferred-work.md:938`, closed by Story 16-3).
    let stdout_pipe = child.stdout.take().expect("stdout was piped");
    let stderr_pipe = child.stderr.take().expect("stderr was piped");
    let stdout_drain = std::thread::spawn(move || drain_bounded(stdout_pipe));
    let stderr_drain = std::thread::spawn(move || drain_bounded(stderr_pipe));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // The child is reaped, so both write ends are closed and the
                // drainers see EOF; the joins carry the whole capture.
                let out = stdout_drain.join().expect("stdout drain thread panicked");
                let err = stderr_drain.join().expect("stderr drain thread panicked");
                return BoundedRun {
                    code: status.code(),
                    stdout: String::from_utf8_lossy(&out).into_owned(),
                    stderr: String::from_utf8_lossy(&err).into_owned(),
                };
            }
            Ok(None) if std::time::Instant::now() >= deadline => {
                // The drainers are deliberately NOT joined on this path: the
                // kill closes the child's write ends so they reach EOF on
                // their own, and a descendant still holding a pipe is 17-1
                // AC4's tree teardown, not this helper's bound.
                let _ = child.kill();
                let _ = child.wait();
                panic!("{label}: did not exit within {timeout_secs}s");
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => panic!("{label}: wait failed: {e}"),
        }
    }
}

/// Poll a condition at 50 ms until it holds or `timeout_secs` elapses.
/// Returns `true` when it held; `false` on timeout (the caller asserts).
pub fn wait_until(timeout_secs: u64, label: &str, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        if cond() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            eprintln!("{label}: condition not met within {timeout_secs}s");
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
