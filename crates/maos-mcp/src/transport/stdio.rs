//! Stdio Transport — subprocess MCP server over stdin/stdout NDJSON.
//!
//! Spawns the configured binary as a child process and exchanges
//! NDJSON frames bidirectionally.  JoinHandle self-prunes per
//! Story 5.4 §1368.
//!
//! Story 18-8 AC3 (D-18-1-12 B): an early close is a typed `Transport`
//! error, a hang is `Timeout(ms)` after the per-call bound (CR-7), and the
//! child is always reaped. Review BH-3/AA-3: the bound covers the write AND
//! the read (both on the helper thread); after an answer the child gets
//! [`EXIT_GRACE`] to exit. Review BH-4/AA-6: the child leads its own process
//! group, killed whole, so a wrapper's (`sh -c`, `npx`) grandchild dies too.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use maos_domain::ports::mcp::{McpRequest, McpResponse, McpTransportId};

use super::{McpTransport, McpTransportError};

/// CR-7 default bound on one exchange (write + read) — the Butler
/// `time_cap_seconds` (`spirits/butler/manifest.toml`).
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);
/// How long an answered child may take to exit before its group is killed.
pub const EXIT_GRACE: Duration = Duration::from_secs(1);

/// Stdio transport — launches the MCP server as a child process and
/// communicates via NDJSON over stdin/stdout.
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    read_timeout: Duration,
}

impl StdioTransport {
    /// Construct a new stdio transport.
    pub fn new(command: String, args: Vec<String>) -> Result<Self, McpTransportError> {
        // Validate the binary exists by probing spawnability (lazy — actual
        // spawn happens per-invocation; a new design may pre-spawn at
        // construction if needed).
        Ok(Self {
            command,
            args,
            read_timeout: DEFAULT_READ_TIMEOUT,
        })
    }

    /// Replace the per-call bound on the write + read (CR-7).
    pub fn with_read_timeout(mut self, read_timeout: Duration) -> Self {
        self.read_timeout = read_timeout;
        self
    }

    fn spawn_and_invoke(&self, request: &McpRequest) -> Result<McpResponse, McpTransportError> {
        let frame = build_stdio_request_body(request)
            .map_err(|e| McpTransportError::Transport(format!("encode: {e}")))?;
        let mut command = Command::new(&self.command);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        let mut child = command
            .spawn()
            .map_err(|e| McpTransportError::Transport(format!("spawn failed: {e}")))?;
        let (stdin, stdout, (tx, rx)) = (child.stdin.take(), child.stdout.take(), mpsc::channel());
        std::thread::spawn(move || tx.send(exchange(stdin, stdout, &frame)));
        let line = match rx.recv_timeout(self.read_timeout) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => Err(McpTransportError::Timeout(
                self.read_timeout.as_millis() as u64,
            )),
            Err(RecvTimeoutError::Disconnected) => Err(McpTransportError::Transport(
                "stdio exchange thread died".into(),
            )),
        };
        // Reap on every path (JoinHandle self-prune on wait): no zombie.
        let grace = match line {
            Ok(_) => EXIT_GRACE,
            Err(_) => Duration::ZERO,
        };
        reap(&mut child, grace)?;
        parse_stdio_response(&line?, &request.server)
    }
}

/// Write the NDJSON frame, close stdin (the child sees EOF), read one line.
fn exchange(
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    frame: &[u8],
) -> Result<String, McpTransportError> {
    let (Some(mut stdin), Some(stdout)) = (stdin, stdout) else {
        return Err(McpTransportError::Transport(
            "stdio pipes unavailable".into(),
        ));
    };
    stdin
        .write_all(frame)
        .map_err(|e| McpTransportError::Transport(format!("write: {e}")))?;
    drop(stdin);
    match BufReader::new(stdout).lines().next() {
        Some(line) => line.map_err(|e| McpTransportError::Transport(format!("read: {e}"))),
        None => Err(McpTransportError::Transport(
            "child produced no output".into(),
        )),
    }
}

/// Give the child `grace` to exit, else kill its whole process group, then
/// reap it. With no grace (every error path) the group is killed before the
/// leader is reaped, so its pgid cannot have been recycled. `forbid(unsafe_code)`
/// rules out `killpg`: the group gets `kill -KILL -- -<pgid>`, backed by the
/// direct kill.
fn reap(child: &mut Child, grace: Duration) -> Result<(), McpTransportError> {
    let wait = |e: std::io::Error| McpTransportError::Transport(format!("wait: {e}"));
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        if child.try_wait().map_err(wait)?.is_some() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    #[cfg(unix)]
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    child.wait().map(drop).map_err(wait)
}

impl McpTransport for StdioTransport {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        self.spawn_and_invoke(&request)
    }

    fn id(&self) -> McpTransportId {
        McpTransportId::Stdio
    }
}

/// Build a JSON-RPC 2.0 request body for stdio transport (NDJSON frame).
pub fn build_stdio_request_body(request: &McpRequest) -> Result<Vec<u8>, serde_json::Error> {
    let rpc = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": request.tool,
            "arguments": request.args,
        }
    });
    let mut bytes = serde_json::to_vec(&rpc)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Parse a single NDJSON line from stdio into an `McpResponse`.
pub fn parse_stdio_response(
    line: &str,
    server_name: &str,
) -> Result<McpResponse, McpTransportError> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| McpTransportError::Transport(format!("parse: {e}")))?;
    super::response_from(&value, server_name, McpTransportId::Stdio)
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // StdioTransport is stateless at v0.5-α — the child process is
        // spawned and waited per-invocation.  No persistent child to kill.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_stdio_request_body_produces_ndjson_line() {
        let req = McpRequest::new("test-server".into(), "echo".into(), json!({"msg": "hi"}));
        let body = build_stdio_request_body(&req).unwrap();
        let s = String::from_utf8(body).unwrap();
        assert!(s.contains("tools/call"));
        assert!(s.contains(r#""msg":"hi""#));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn parse_stdio_response_with_result() {
        let resp_line = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"hello"}]}}"#;
        let resp = parse_stdio_response(resp_line, "test-server").unwrap();
        assert!(!resp.is_error);
        assert_eq!(resp.attribution.server_name, "test-server");
        assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    }

    #[test]
    fn parse_stdio_response_with_error() {
        let resp_line =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let err = parse_stdio_response(resp_line, "test-server").unwrap_err();
        assert!(matches!(
            err,
            McpTransportError::ServerError { code: -32601, .. }
        ));
    }

    #[test]
    fn parse_stdio_response_malformed_returns_transport_error() {
        let resp_line = "not json at all";
        let err = parse_stdio_response(resp_line, "srv").unwrap_err();
        assert!(matches!(err, McpTransportError::Transport(_)));
    }
}
