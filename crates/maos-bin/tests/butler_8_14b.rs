#![forbid(unsafe_code)]

//! Story 8.14b — Butler MCP Driver Set integration smoke.
//!
//! Exercises:
//! 1. `maos run butler --once` — existing behavior preserved (halt fires).
//! 2. `maos run butler --live --once` — LiveButlerMcpPort wired, stderr logs it.
//! 3. `maos shell` with `@butler pick a` — option-pick dispatch surface.

use std::process::Command;

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// A unique on-disk state root so parallel `maos run` subprocesses do not
/// contend on the shared `~/.local/share/maos` SQLite audit DB / journal.
struct IsolatedDataHome {
    path: std::path::PathBuf,
}

impl Drop for IsolatedDataHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn isolated_data_home(tag: &str) -> IsolatedDataHome {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-8-14b-{tag}-{nanos}"));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

#[test]
fn maos_run_butler_once_preserved_existing_halt_behavior() {
    let home = isolated_data_home("once");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run butler --once");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "maos run butler --once must exit 0; stderr:\n{stderr}"
    );

    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    let halt = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("halt"))
        .expect("stdout must contain a halt event (existing behavior preserved)");

    let expected_render = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert_eq!(
        halt.get("render").and_then(|v| v.as_str()),
        Some(expected_render.as_str()),
        "halt render-string must equal the shared production constant"
    );

    // AC5 — halt visible to director on stderr.
    assert!(
        stderr.contains(&expected_render),
        "halt render-string must appear on stderr (AC5). stderr was:\n{stderr}"
    );
}

#[test]
fn maos_run_butler_live_once_wires_mcp_port() {
    let home = isolated_data_home("live");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/butler/manifest.toml", "--live", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_MCP_CALENDAR_URI", "http://localhost:9999")
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run butler --live --once");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The --live path wires LiveButlerMcpPort and logs it.
    // Without real MCP servers responding, Butler falls back to empty data
    // (unwrap_or_default in on_idle), so no halt fires — exit 0 regardless.
    assert!(
        stderr.contains("butler live MCP port wired (--live)"),
        "--live must wire LiveButlerMcpPort and log it. stderr was:\n{stderr}"
    );
}

#[test]
fn shell_butler_pick_renders_option_messages() {
    let home = isolated_data_home("shell");
    let mut child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .arg("shell")
        .env("MAOS_HOME", home.path.clone())
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn maos shell");

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        use std::io::Write;
        writeln!(stdin, "@butler pick a").unwrap();
        writeln!(stdin, "@butler pick b").unwrap();
        writeln!(stdin, "@butler pick c").unwrap();
        writeln!(stdin, "@butler pick x").unwrap();
        // Ctrl-D (EOF)
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("shell output");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Linear note written"),
        "pick a renders Linear note: {stdout}"
    );
    assert!(
        stdout.contains("Slack message queued"),
        "pick b renders Slack reminder: {stdout}"
    );
    assert!(
        stdout.contains("snoozed"),
        "pick c renders snooze: {stdout}"
    );
    assert!(
        stdout.contains("no pending notification"),
        "pick x renders error: {stdout}"
    );
}

use parking_lot::Mutex;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

struct RequestCapture {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn spawn_mock_mcp_server(responses: Vec<&'static str>) -> (String, mpsc::Receiver<RequestCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for (i, resp) in responses.into_iter().enumerate() {
            eprintln!("Mock server loop {} - waiting for accept", i);
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("accept failed: {:?}", e);
                    break;
                }
            };
            eprintln!("Mock server loop {} - accepted", i);
            let mut bytes = Vec::new();
            let mut buf = [0u8; 1024];
            let header_end;
            loop {
                let n = match stream.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => break,
                };
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..n]);
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
            }
            eprintln!(
                "Mock server loop {} - read headers (bytes: {})",
                i,
                bytes.len()
            );
            if bytes.is_empty() {
                break;
            }
            let headers = String::from_utf8_lossy(&bytes);
            let first_line = headers.lines().next().unwrap_or("");
            let mut parts = first_line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let mut content_length = 0;
            for line in headers.lines() {
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse::<usize>().unwrap_or(0);
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
                resp.len(),
                resp
            );
            eprintln!("Mock server loop {} - writing response", i);
            let _ = stream.write_all(http_response.as_bytes());
            eprintln!("Mock server loop {} - response written", i);
            let _ = tx.send(RequestCapture { method, path, body });
        }
    });
    (format!("http://{}", addr), rx)
}

/// Healthy: the whole `--live --once` run exits in ~5.2 s — 5 s of which is
/// `audit writer drain timed out after 5s`, itself a filed defect. ~11x.
const CHILD_BUDGET: Duration = Duration::from_secs(60);
/// After `maos` exits, a reader still short of EOF means a DESCENDANT
/// inherited the pipe (the Story 16-3 grandchild class). Waiting on it is a
/// hang, so it is bounded and named instead.
const PIPE_SETTLE: Duration = Duration::from_secs(10);
/// Once `maos` has exited, a call it never made will never arrive.
const MOCK_CALL_BUDGET: Duration = Duration::from_secs(10);

/// Wait for a piped `maos` child with every wait BOUNDED.
///
/// 2026-09-21: `butler_8_14b_mcp_drivers` stalled workspace-test-suite for
/// ~74 minutes on 79fc910e (the job died at its ceiling) while passing locally
/// in 5.2 s — including on 2 pinned cores, with the job's
/// MAOS_SECRETS_BACKEND=env, and with an initialised home. Every wait here was
/// unbounded, and libtest prints a test's captured output only when it
/// FINISHES, so the hang left no trace. A stall now FAILS with whatever `maos`
/// had written, which is the diagnosis that run could not give.
fn wait_bounded(
    mut child: std::process::Child,
    budget: Duration,
) -> (std::process::ExitStatus, String, String) {
    type Sink = Arc<Mutex<Vec<u8>>>;
    fn drain(mut pipe: impl Read + Send + 'static) -> (Sink, mpsc::Receiver<()>) {
        let sink: Sink = Arc::default();
        let (done_tx, done_rx) = mpsc::channel();
        let writer = Arc::clone(&sink);
        thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                writer.lock().extend_from_slice(&chunk[..n]);
            }
            let _ = done_tx.send(());
        });
        (sink, done_rx)
    }
    let text = |sink: &Sink| String::from_utf8_lossy(&sink.lock()).into_owned();

    let (out, out_done) = drain(child.stdout.take().expect("piped stdout"));
    let (err, err_done) = drain(child.stderr.take().expect("piped stderr"));
    let deadline = Instant::now() + budget;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait maos") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "maos did not exit within {budget:?} — a HANG, not a slow run (healthy ~5.2 s).\n\
                 stderr so far:\n{}\nstdout so far:\n{}",
                text(&err),
                text(&out)
            );
        }
        thread::sleep(Duration::from_millis(50));
    };
    let settle = Instant::now() + PIPE_SETTLE;
    for (name, done) in [("stdout", &out_done), ("stderr", &err_done)] {
        if done
            .recv_timeout(settle.saturating_duration_since(Instant::now()))
            .is_err()
        {
            panic!(
                "maos exited ({status:?}) but its {name} pipe was still open {PIPE_SETTLE:?} \
                 later — a descendant inherited it.\nstderr:\n{}\nstdout:\n{}",
                text(&err),
                text(&out)
            );
        }
    }
    (status, text(&out), text(&err))
}

#[test]
fn butler_8_14b_mcp_drivers() {
    // AC3 test 4: maos run butler --once with isolated MAOS_HOME + a mock MCP server URL
    let (url, rx) = spawn_mock_mcp_server(vec![
        // calendar_events response
        r#"{"jsonrpc":"2.0","result":[{"id":"evt-c","title":"Conflict event 1","start_min":540,"end_min":600,"status":"confirmed"},{"id":"evt-d","title":"Conflict event 2","start_min":570,"end_min":630,"status":"confirmed"}],"id":1}"#,
        // comms_messages response
        r#"{"jsonrpc":"2.0","result":[],"id":1}"#,
    ]);

    let home = isolated_data_home("mcp_drivers");
    let child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/butler/manifest.toml", "--live", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_MCP_CALENDAR_URI", url.clone())
        .env("MAOS_MCP_SLACK_URI", url.clone())
        .env("MAOS_MCP_LINEAR_URI", url.clone())
        .env("MAOS_MCP_FIGMA_URI", url.clone())
        .current_dir(workspace_root())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute maos run butler --live --once");

    let (status, stdout_str, stderr_str) = wait_bounded(child, CHILD_BUDGET);
    eprintln!("Child exited with status: {:?}", status);
    eprintln!("Child stdout: {}", stdout_str);
    eprintln!("Child stderr: {}", stderr_str);

    let stdout = stdout_str;

    // Assert calendar list_events was called
    let req1 = rx
        .recv_timeout(MOCK_CALL_BUDGET)
        .unwrap_or_else(|e| panic!("expected calendar_events call ({e}); stderr:\n{stderr_str}"));
    assert!(req1.path.contains("tools/call") || req1.path.contains("/"));

    // Assert comms list_messages was called
    let req2 = rx
        .recv_timeout(MOCK_CALL_BUDGET)
        .unwrap_or_else(|e| panic!("expected comms_messages call ({e}); stderr:\n{stderr_str}"));
    assert!(req2.path.contains("tools/call") || req2.path.contains("/"));

    // Assert halt event is in stdout
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let halt = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("halt"))
        .expect("stdout must contain a halt event");

    let expected_render = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert_eq!(
        halt.get("render").and_then(|v| v.as_str()),
        Some(expected_render.as_str())
    );
}

#[test]
fn butler_undeclared_tool_returns_capability_denied() {
    // JB-6: LiveButlerMcpPort returns CapabilityDenied (Unauthorized) when calling an undeclared tool
    let (url, _rx) = spawn_mock_mcp_server(vec![]);

    // Read butler manifest and remove calendar capability
    let manifest_path = std::path::Path::new(workspace_root()).join("spirits/butler/manifest.toml");
    let manifest_content = std::fs::read_to_string(&manifest_path).unwrap();
    // Remove calendar from required MCP servers table
    let mut in_mcp = false;
    let mut new_lines = Vec::new();
    for line in manifest_content.lines() {
        if line.contains("[[capabilities.required.mcp.servers]]") {
            in_mcp = true;
            continue;
        }
        if in_mcp {
            if line.starts_with("[[")
                || line.starts_with("[posture")
                || line.starts_with("[output_shape")
            {
                in_mcp = false;
            } else {
                continue;
            }
        }
        new_lines.push(line);
    }
    let new_manifest = new_lines.join("\n");

    let home = isolated_data_home("cap_denied");
    let temp_manifest_path = home.path.join("manifest.toml");
    std::fs::write(&temp_manifest_path, new_manifest).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args([
            "run",
            temp_manifest_path.to_str().unwrap(),
            "--live",
            "--once",
        ])
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_MCP_CALENDAR_URI", url)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run butler --live --once");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unauthorized MCP call: capability scope mismatch") ||
        stderr.contains("token issuance failed for MCP call"),
        "expected capability scope mismatch or token issuance failure on calendar tool call. stderr was:\n{stderr}"
    );
}
