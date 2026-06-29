#![forbid(unsafe_code)]

//! Story 8.14c — Researcher MCP Driver Set integration smoke.
//!
//! Exercises:
//! 1. `maos run researcher --once` — deterministic survey (baseline preserved).
//! 2. `maos run researcher --live --once` — LiveResearcherMcpPort wired, stderr
//!    logs it, two-phase fan-out (search → fetch) fires against mock MCP servers.
//!
//! Assertions:
//! - Fan-out fired: mock servers receive Phase-1 search/traverse requests.
//! - No `CapabilityDenied` in output — all declared tool scopes admitted.
//! - `output_shape` validates: `on_idle_fired` event appears in stdout,
//!   implying the survey produced a valid `SurveyOutput` with the four
//!   required fields (findings, open_questions, confidence_map, bibliography).
//!
//! Deferred (W6): BudgetWarning@80% observable — requires real wall-clock time.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

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
    let path = std::env::temp_dir().join(format!("maos-8-14c-{tag}-{nanos}"));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

// ---------------------------------------------------------------------------
// Mock MCP server (mirrors butler_8_14b::spawn_mock_mcp_server).
// ---------------------------------------------------------------------------

struct RequestCapture {
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    path: String,
    #[allow(dead_code)]
    body: Vec<u8>,
}

/// Spawn a minimal HTTP server on an ephemeral port that serves `responses`
/// in order, one per accepted connection. Returns `(url, Receiver)` where
/// each received `RequestCapture` records the HTTP method/path/body of one
/// incoming request.
fn spawn_mock_mcp_server(responses: Vec<&'static str>) -> (String, mpsc::Receiver<RequestCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for (i, resp) in responses.into_iter().enumerate() {
            eprintln!("mock-mcp[{i}] waiting for accept on {addr}");
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("mock-mcp[{i}] accept failed: {e}");
                    break;
                }
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
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
                if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
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
            let _ = stream.write_all(http_response.as_bytes());
            let _ = tx.send(RequestCapture { method, path, body });
        }
    });
    (format!("http://{addr}"), rx)
}

// ---------------------------------------------------------------------------
// Test 1: baseline deterministic (no --live, no MCP) — existing behavior.
// ---------------------------------------------------------------------------

#[test]
fn maos_run_researcher_once_deterministic() {
    let home = isolated_data_home("once");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run researcher --once");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "maos run researcher --once must exit 0; stderr:\n{stderr}"
    );

    // The --once path fires on_idle and prints an on_idle_fired event.
    assert!(
        stdout.contains("on_idle_fired"),
        "stdout must contain on_idle_fired event. stdout:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: --live wires MCP port (no real servers — port wires but survey
//          falls back / on_idle completes).
// ---------------------------------------------------------------------------

#[test]
fn maos_run_researcher_live_once_wires_mcp_port() {
    let home = isolated_data_home("live");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args([
            "run",
            "spirits/researcher/manifest.toml",
            "--live",
            "--once",
        ])
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_MCP_WEB_URI", "http://localhost:19999")
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run researcher --live --once");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The --live path wires LiveResearcherMcpPort when any MAOS_MCP_*_URI is
    // set and logs it.  Without a responsive server the MCP survey will fail
    // (on_idle prints "MCP survey failed" and returns), but the process still
    // exits 0.
    assert!(
        stderr.contains("researcher live MCP port wired (--live)")
            || stderr.contains("researcher live-inference seam wired"),
        "--live must wire researcher MCP/inference ports and log it. stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: two-phase fan-out with mock MCP servers (the P17 deferred test).
// ---------------------------------------------------------------------------

/// Phase-1 (search/traverse) mock responses — one per MCP endpoint.
/// Each returns exactly one source key so Phase-2 has one fetch per server.
const WEB_SEARCH_RESP: &str =
    r#"{"jsonrpc":"2.0","result":{"results":[{"url":"https://example.com/paper1"}]},"id":1}"#;
const ARXIV_SEARCH_RESP: &str =
    r#"{"jsonrpc":"2.0","result":{"papers":[{"arxiv_id":"2401.00001"}]},"id":1}"#;
const GITHUB_SEARCH_RESP: &str =
    r#"{"jsonrpc":"2.0","result":{"results":[{"repo":"octocat/hello-world"}]},"id":1}"#;
const CITATION_SEARCH_RESP: &str =
    r#"{"jsonrpc":"2.0","result":{"edges":[{"from":"paper-alpha","to":"paper-alpha"}]},"id":1}"#;

/// Phase-2 (fetch) mock responses — each carries a `ClaimPayload` + `source_key`.
const WEB_FETCH_RESP: &str = r#"{"jsonrpc":"2.0","result":{"claim":{"claim_id":"web-1","statement":"Web search found positional bias in LLM agents","topic":"positional-bias","methodology_strength":0.8,"confidence":0.85,"load_bearing":false,"polarity":true,"hedges":["may"]},"source_key":"https://example.com/paper1"},"id":1}"#;
const ARXIV_FETCH_RESP: &str = r#"{"jsonrpc":"2.0","result":{"claim":{"claim_id":"arxiv-1","statement":"ArXiv paper quantifies positional bias in transformer attention","topic":"positional-bias","methodology_strength":0.9,"confidence":0.92,"load_bearing":true,"polarity":true,"hedges":[]},"source_key":"2401.00001"},"id":1}"#;
const GITHUB_FETCH_RESP: &str = r#"{"jsonrpc":"2.0","result":{"claim":{"claim_id":"gh-1","statement":"GitHub repo implements bias mitigation for positional effects","topic":"positional-bias","methodology_strength":0.6,"confidence":0.7,"load_bearing":false,"polarity":true,"hedges":["appears to"]},"source_key":"octocat/hello-world"},"id":1}"#;
const CITATION_FETCH_RESP: &str = r#"{"jsonrpc":"2.0","result":{"claim":{"claim_id":"cit-1","statement":"Citation graph shows cross-domain evidence for positional bias","topic":"positional-bias","methodology_strength":0.75,"confidence":0.8,"load_bearing":false,"polarity":true,"hedges":[]},"source_key":"paper-alpha"},"id":1}"#;

#[test]
fn researcher_8_14c_mcp_fanout() {
    let (web_url, web_rx) = spawn_mock_mcp_server(vec![WEB_SEARCH_RESP, WEB_FETCH_RESP]);
    let (arxiv_url, arxiv_rx) = spawn_mock_mcp_server(vec![ARXIV_SEARCH_RESP, ARXIV_FETCH_RESP]);
    let (github_url, github_rx) =
        spawn_mock_mcp_server(vec![GITHUB_SEARCH_RESP, GITHUB_FETCH_RESP]);
    let (cit_url, cit_rx) = spawn_mock_mcp_server(vec![CITATION_SEARCH_RESP, CITATION_FETCH_RESP]);

    let home = isolated_data_home("fanout");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args([
            "run",
            "spirits/researcher/manifest.toml",
            "--live",
            "--once",
        ])
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_MCP_WEB_URI", &web_url)
        .env("MAOS_MCP_ARXIV_URI", &arxiv_url)
        .env("MAOS_MCP_GITHUB_URI", &github_url)
        .env("MAOS_MCP_CITATION_GRAPH_URI", &cit_url)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos run researcher --live --once");

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout_str}{stderr_str}");

    assert!(
        output.status.success(),
        "maos run researcher --live --once must exit 0. stderr:\n{stderr_str}"
    );
    assert!(
        combined.contains("researcher live MCP port wired (--live)"),
        "output must confirm the live MCP wiring. combined:\n{combined}"
    );
    // Fan-out must succeed — no failure message when all four mock servers respond.
    assert!(
        !combined.contains("MCP survey failed"),
        "MCP fan-out must not fail when all four mock servers are configured. combined:\n{combined}"
    );
    // No capability denied — all declared tool scopes admitted per manifest.
    assert!(
        !combined.contains("CapabilityDenied"),
        "no CapabilityDenied must appear (scopes declared in manifest). combined:\n{combined}"
    );

    // output_shape: on_idle_fired event in stdout.
    let events: Vec<serde_json::Value> = stdout_str
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let idle_event = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("on_idle_fired"));
    assert!(
        idle_event.is_some(),
        "stdout must contain on_idle_fired event (output_shape validated). stdout:\n{stdout_str}"
    );

    // Phase-1 assertion: each mock server received at least one request (search/traverse).
    let timeout = std::time::Duration::from_secs(1);
    assert!(
        web_rx.recv_timeout(timeout).is_ok(),
        "web mock must receive Phase-1 search request"
    );
    assert!(
        arxiv_rx.recv_timeout(timeout).is_ok(),
        "arxiv mock must receive Phase-1 search request"
    );
    assert!(
        github_rx.recv_timeout(timeout).is_ok(),
        "github mock must receive Phase-1 search_code request"
    );
    assert!(
        cit_rx.recv_timeout(timeout).is_ok(),
        "citation-graph mock must receive Phase-1 traverse request"
    );

    // Phase-2 assertion: each mock server received a second request (fetch).
    assert!(
        web_rx.recv_timeout(timeout).is_ok(),
        "web mock must receive Phase-2 fetch request"
    );
    assert!(
        arxiv_rx.recv_timeout(timeout).is_ok(),
        "arxiv mock must receive Phase-2 get_paper request"
    );
    assert!(
        github_rx.recv_timeout(timeout).is_ok(),
        "github mock must receive Phase-2 get_repo request"
    );
    assert!(
        cit_rx.recv_timeout(timeout).is_ok(),
        "citation-graph mock must receive Phase-2 get_citations request"
    );
}
