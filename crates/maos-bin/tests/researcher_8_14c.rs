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
use std::time::Duration;

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;

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
    /// The raw request head (request line + headers) — Story 18-8 AC2.3.
    #[allow(dead_code)]
    head: String,
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
            let head = headers[..header_len.min(headers.len())].to_string();
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
            let _ = tx.send(RequestCapture {
                method,
                path,
                head,
                body,
            });
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
        // Story 16-1 / D-16-1-Q: empty scratch "HOME" so this root never reads
        // the developer's $HOME/.maos/control.json (EndpointInUse collision).
        .env("HOME", doorless_home::doorless_home())
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
        .env("HOME", doorless_home::doorless_home())
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
        .env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", home.path.clone())
        // Story 17-11 (FW-05 A): no local Ollama ⇒ router default `anthropic`,
        // the Researcher's declared provider.
        .env("MAOS_OLLAMA_URL", "skip")
        // Story 17-11 review: ambient keys must not flip the router default.
        .env_remove("MAOS_OPENAI_API_KEY")
        .env_remove("MAOS_ANTHROPIC_API_KEY")
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

    // Story 18-3 (CR-4, D-18-3-05 A): the zero-config box's only provider is
    // the `anthropic` placeholder, so after the fan-out the bound pass is
    // refused typed `Unconfigured` — never the summarize fallback's exit 0.
    assert_eq!(
        output.status.code(),
        Some(1),
        "a zero-config bound pass exits 1. stderr:\n{stderr_str}"
    );
    assert!(
        stderr_str.contains("researcher on_idle pass refused")
            && stderr_str.contains("Unconfigured"),
        "the refusal is typed `Unconfigured`. stderr:\n{stderr_str}"
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

/// The four Researcher servers: (server, URI var, bearer var, canary).
const RESEARCHER_BEARERS: [(&str, &str, &str, &str); 4] = [
    (
        "web",
        "MAOS_MCP_WEB_URI",
        "MAOS_MCP_WEB_BEARER",
        "canary-web-18-8",
    ),
    (
        "arxiv",
        "MAOS_MCP_ARXIV_URI",
        "MAOS_MCP_ARXIV_BEARER",
        "canary-arxiv-18-8",
    ),
    (
        "github",
        "MAOS_MCP_GITHUB_URI",
        "MAOS_MCP_GITHUB_BEARER",
        "canary-github-18-8",
    ),
    (
        "citation-graph",
        "MAOS_MCP_CITATION_GRAPH_URI",
        "MAOS_MCP_CITATION_GRAPH_BEARER",
        "canary-citation-graph-18-8",
    ),
];

/// Start the four mocks; run `maos run researcher --live --once` with every
/// URI and bearer set, the state root (`XDG_DATA_HOME` and `MAOS_HOME`)
/// inside `home`, the audit/journal overrides removed (review TIA-5), and
/// `extra` env applied last. Returns the combined output and each mock's
/// requests (server, head).
fn run_researcher_with_bearers(
    home: &IsolatedDataHome,
    extra: &[(&str, &str)],
) -> (String, Vec<(&'static str, Vec<String>)>) {
    let responses = [
        vec![WEB_SEARCH_RESP, WEB_FETCH_RESP],
        vec![ARXIV_SEARCH_RESP, ARXIV_FETCH_RESP],
        vec![GITHUB_SEARCH_RESP, GITHUB_FETCH_RESP],
        vec![CITATION_SEARCH_RESP, CITATION_FETCH_RESP],
    ];
    let mocks: Vec<_> = responses.into_iter().map(spawn_mock_mcp_server).collect();
    std::fs::create_dir_all(home.path.join("maos-home")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args([
            "run",
            "spirits/researcher/manifest.toml",
            "--live",
            "--once",
        ])
        .env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_HOME", home.path.join("maos-home"))
        .env_remove("MAOS_AUDIT_DB")
        .env_remove("MAOS_JOURNAL_PATH")
        .env("MAOS_OLLAMA_URL", "skip")
        .env_remove("MAOS_OPENAI_API_KEY")
        .env_remove("MAOS_ANTHROPIC_API_KEY")
        .env("MAOS_SECRETS_BACKEND", "env")
        .current_dir(workspace_root());
    for ((_, uri_var, bearer_var, canary), (url, _)) in RESEARCHER_BEARERS.iter().zip(&mocks) {
        command.env(uri_var, url).env(bearer_var, canary);
    }
    for (name, value) in extra {
        command.env(name, value);
    }
    let output = command
        .output()
        .expect("failed to execute maos run researcher --live --once");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let seen = RESEARCHER_BEARERS
        .iter()
        .zip(mocks)
        .map(|((server, ..), (_, rx))| {
            let heads = std::iter::from_fn(|| rx.recv_timeout(Duration::from_millis(300)).ok())
                .map(|capture| capture.head)
                .collect();
            (*server, heads)
        })
        .collect();
    (combined, seen)
}

/// No file under `root` (TL rows, journals, the MAOS home) holds `canary`.
fn assert_never_persisted(root: &std::path::Path, canary: &str) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                assert!(
                    !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
                    "the bearer must never be persisted: {}",
                    path.display()
                );
            }
        }
    }
}

/// Story 18-8 AC2.3 / CR-5 — the composition root reads each server's bearer
/// from the secret store (env backend here, so no OS keyring is touched) and
/// the transport sends it as `Authorization: Bearer` to THAT server only.
/// Review TIA-4: four servers, four canaries — each mock sees exactly its own
/// bearer and no other canary. No canary appears in stdout/stderr or in any
/// file under the run's state root (review TIA-5: `MAOS_HOME` inside it).
#[test]
fn researcher_live_mcp_sends_the_secret_store_bearer_and_never_leaks_it() {
    let home = isolated_data_home("18-8-bearer");
    let (combined, seen) = run_researcher_with_bearers(&home, &[]);
    assert!(
        combined.contains("researcher live MCP port wired (--live)"),
        "the live MCP port must be wired. combined:\n{combined}"
    );
    for ((server, .., own), (_, heads)) in RESEARCHER_BEARERS.iter().zip(&seen) {
        assert!(
            !heads.is_empty(),
            "the {server} mock must receive a request"
        );
        for head in heads {
            let authorization: Vec<_> = head
                .lines()
                .filter(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                .collect();
            assert_eq!(
                authorization,
                [format!("Authorization: Bearer {own}")],
                "{server} must see exactly its own bearer; head:\n{head}"
            );
            for (other, .., canary) in &RESEARCHER_BEARERS {
                assert!(
                    other == server || !head.contains(canary),
                    "{server} must not see {other}'s bearer"
                );
            }
        }
    }
    for (.., canary) in &RESEARCHER_BEARERS {
        assert!(
            !combined.contains(canary),
            "the bearer must never reach stdout/stderr:\n{combined}"
        );
        assert_never_persisted(&home.path, canary);
    }
}

/// Review ruling D-18-8-R1 (EC-3/BH-5/AA-7) — one server's bearer lookup
/// ERROR (here: its sealed file is a symlink, refused by the encrypted-file
/// store) unconfigures only that server's lookup: exactly one lookup-failed
/// warning, naming the server and the error kind (never a value), web's mock
/// receives nothing, and the live port is still wired rather than refused as
/// wholly unconfigured. It does NOT prove that a sibling whose lookup silently
/// resolves to the wrong key is configured; that is outside R1. The pass fails
/// fast on web's `Unconfigured` and aborts its sibling calls
/// (`survey_literature_impl`), so whether arxiv/github/citation-graph are ever
/// reached is a race: any sibling request that was made is only checked to
/// carry no bearer or env canary.
#[cfg(unix)]
#[test]
fn one_bearer_lookup_error_unconfigures_only_that_server() {
    let home = isolated_data_home("18-8-bearer-error");
    let secrets = home.path.join("maos-home").join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::os::unix::fs::symlink("/dev/null", secrets.join("mcp-web-bearer.sealed")).unwrap();
    let key = "11".repeat(32);
    let (combined, seen) = run_researcher_with_bearers(
        &home,
        &[
            ("MAOS_SECRETS_BACKEND", "encrypted-file"),
            ("MAOS_KMS_MASTER_KEY", &key),
        ],
    );
    assert!(
        combined.contains(
            "the web MCP bearer lookup failed (secret store: access failed); web is unconfigured"
        ),
        "the warning names the server and the kind. combined:\n{combined}"
    );
    // R1 regressed (the lookup error fails every server): the builder errs
    // or yields no client at all, so the port is never wired and the pass
    // refuses as unconfigured instead of failing on web.
    assert!(
        combined.contains("researcher live MCP port wired (--live)"),
        "the live MCP port stays wired. combined:\n{combined}"
    );
    assert!(
        !combined.contains("no MCP server is configured"),
        "one lookup error must not unconfigure every server. combined:\n{combined}"
    );
    assert_eq!(
        combined.matches("MCP bearer lookup failed").count(),
        1,
        "only web's lookup fails. combined:\n{combined}"
    );
    for (server, heads) in &seen {
        if *server == "web" {
            assert!(heads.is_empty(), "web is unconfigured, never called");
            continue;
        }
        // The encrypted-file backend has no env fallback and holds no sealed
        // bearer for the siblings, so they are wired WITHOUT a bearer: any
        // request a sibling sent before the fail-fast abort carries no
        // `Authorization` header and none of the env canaries (in particular
        // never web's). Vacuous when no sibling request was made.
        for head in heads {
            let authorization: Vec<_> = head
                .lines()
                .filter(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                .collect();
            assert!(
                authorization.is_empty(),
                "{server} has no sealed bearer, so it sends none; head:\n{head}"
            );
            for (other, .., canary) in &RESEARCHER_BEARERS {
                assert!(
                    !head.contains(canary),
                    "{server} must not see {other}'s env bearer; head:\n{head}"
                );
            }
        }
    }
    assert!(
        !combined.contains("symbolic link"),
        "only the kind is printed, never the store's detail:\n{combined}"
    );
}

// ---------------------------------------------------------------------------
// Story 17-11 review — keyless provider entitlement at issuance.
// ---------------------------------------------------------------------------

/// A keyless box with Ollama at a dead port: the router default is `ollama`,
/// which neither the Researcher nor this fixture declares.
fn keyless_researcher(args: &[&str], home: &IsolatedDataHome) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args(args)
        .env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_OLLAMA_URL", "http://127.0.0.1:9")
        .env_remove("MAOS_OPENAI_API_KEY")
        .env_remove("MAOS_ANTHROPIC_API_KEY")
        .env_remove("MAOS_INFERENCE_MODE")
        .env_remove("MAOS_REPLAY_CASSETTE")
        .current_dir(workspace_root());
    command
}

/// AC3(a): an undeclared router default is refused TYPED at issuance — a
/// non-zero exit naming the kernel policy refusal, never a silent success.
#[test]
fn researcher_17_11_keyless_undeclared_provider_refused_typed() {
    let home = isolated_data_home("17-11-refused");
    let reference = std::fs::read_to_string(format!(
        "{}/spirits/researcher/manifest.toml",
        workspace_root()
    ))
    .unwrap();
    let line = reference
        .lines()
        .find(|l| l.starts_with("provider.complete"))
        .expect("the Researcher declares provider.complete");
    let fixture = reference.replace(
        line,
        r#"provider.complete = ["anthropic.claude-sonnet-5-5"]"#,
    );
    let manifest = home.path.join("researcher-anthropic-only.toml");
    std::fs::write(&manifest, fixture).unwrap();

    let output = keyless_researcher(
        &["run", manifest.to_str().unwrap(), "--live", "--once"],
        &home,
    )
    .output()
    .expect("failed to execute maos run researcher fixture --live --once");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "an undeclared provider must refuse the run; stderr:\n{stderr}"
    );
    // Story 18-3 (D-18-3-02 B): the declared provider is not registered, so
    // the refusal moves to boot and no token is minted for the undeclared one.
    assert!(
        stderr.contains(
            "researcher inference: no inference provider is configured for its declared \
             providers `anthropic`"
        ),
        "the refusal must be the typed boot refusal; stderr:\n{stderr}"
    );
}

/// Review ruling 2026-10-24: the legacy cassette arm mints the Spirit's FIRST
/// declared provider (`CassetteReplayPort` contacts none), so a keyless box
/// whose router default (`ollama`) is undeclared still replays.
#[test]
fn researcher_17_11_keyless_legacy_cassette_replays() {
    let home = isolated_data_home("17-11-cassette");
    let cassette = format!(
        "{}/crates/maos-journey-test/cassettes/j-researcher/survey-distill.json",
        workspace_root()
    );
    let output = keyless_researcher(
        &["run", "spirits/researcher/manifest.toml", "--once"],
        &home,
    )
    .env("MAOS_REPLAY_CASSETTE", cassette)
    .output()
    .expect("failed to execute maos run researcher --once (legacy cassette)");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "keyless legacy cassette replay must exit 0; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("researcher cassette-replay inference wired"),
        "the legacy cassette arm must be the one exercised; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("kernel policy denied capability issuance"),
        "stderr:\n{stderr}"
    );
}
