use std::time::{Duration, Instant};

use maos_domain::ports::mcp::{McpRequest, McpTransportId};
use maos_mcp::transport::stdio::StdioTransport;
use maos_mcp::transport::{McpTransport, McpTransportError};
use serde_json::json;

#[test]
fn stdio_transport_spawn_and_exchange() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"hello"}]}}"#;
    let transport = StdioTransport::new(
        "sh".into(),
        vec!["-c".into(), format!("cat >/dev/null; echo '{}'", response)],
    )
    .unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let resp = transport.invoke(req).unwrap();

    assert!(!resp.is_error);
    assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    assert_eq!(resp.attribution.server_name, "test-server");
    assert_eq!(resp.attribution.tool, "echo");
}

#[test]
fn stdio_transport_nonexistent_binary_returns_transport_error() {
    let transport =
        StdioTransport::new("/nonexistent/binary/that/does/not/exist".into(), vec![]).unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let err = transport.invoke(req).unwrap_err();

    assert!(format!("{err}").contains("spawn failed"));
}

#[test]
fn stdio_transport_malformed_output_returns_transport_error() {
    let transport = StdioTransport::new(
        "sh".into(),
        vec![
            "-c".into(),
            // Story 15-1 AC6(c): the child must consume its request before
            // answering. A bare `echo` never reads stdin, so when the child
            // wins the scheduling race the request write returns EPIPE and
            // the test observes the wrong error class (measured 7/20; the
            // mechanism reproduced 300/300 with a 2 ms delay standing in for
            // load). Draining stdin first removes the race deterministically.
            "cat >/dev/null; echo 'not json at all'".into(),
        ],
    )
    .unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let err = transport.invoke(req).unwrap_err();

    assert!(format!("{err}").contains("parse"));
}

/// Story 18-8 AC3.1 (CR-6) — a server that exits before the request is
/// written. A request larger than the pipe buffer makes `write_all` block until
/// the child has exited, then EPIPE: deterministic, no scheduling race (Rust
/// test binaries ignore SIGPIPE). A typed `write:` error, never a panic.
#[test]
fn a_server_that_exits_before_the_request_is_written_is_a_typed_write_error() {
    let transport = StdioTransport::new("sh".into(), vec!["-c".into(), "exit 0".into()]).unwrap();
    let big = "x".repeat(2 * 1024 * 1024);
    let req = McpRequest::new(
        "srv".into(),
        "echo".into(),
        serde_json::json!({ "pad": big }),
    );
    match transport.invoke(req) {
        Err(McpTransportError::Transport(message)) => {
            assert!(
                message.starts_with("write: "),
                "expected the write error, got {message}"
            )
        }
        other => panic!("expected Transport(\"write: …\"), got {other:?}"),
    }
}

/// Story 18-8 AC3.1 — a server that consumes the request and exits silently.
#[test]
fn a_server_that_reads_then_exits_silently_is_a_typed_no_output_error() {
    let transport =
        StdioTransport::new("sh".into(), vec!["-c".into(), "cat >/dev/null".into()]).unwrap();
    let req = McpRequest::new("srv".into(), "echo".into(), serde_json::json!({}));
    match transport.invoke(req) {
        Err(McpTransportError::Transport(message)) => {
            assert_eq!(message, "child produced no output")
        }
        other => panic!("expected Transport(\"child produced no output\"), got {other:?}"),
    }
}

/// The pid a hung server wrote to `pid_file`, if it got that far (review
/// TIA-8: a loaded host may kill it first — then there is nothing to check).
fn written_pid(pid_file: &std::path::Path) -> Option<String> {
    let pid = std::fs::read_to_string(pid_file).ok();
    let _ = std::fs::remove_file(pid_file);
    pid.map(|pid| pid.trim().to_owned())
        .filter(|pid| !pid.is_empty())
}

/// Run `script` (with `$0` = a pid file) under a 2 s bound with `args` as the
/// request; returns the outcome, the elapsed time and the written pid.
fn hung(
    tag: &str,
    script: &str,
    args: serde_json::Value,
) -> (
    Result<maos_domain::ports::mcp::McpResponse, McpTransportError>,
    Duration,
    Option<String>,
) {
    let pid_file = std::env::temp_dir().join(format!("maos-18-8-{tag}-{}.pid", std::process::id()));
    let _ = std::fs::remove_file(&pid_file);
    let transport = StdioTransport::new(
        "sh".into(),
        vec!["-c".into(), script.into(), pid_file.display().to_string()],
    )
    .unwrap()
    .with_read_timeout(Duration::from_secs(2));
    let started = Instant::now();
    let outcome = transport.invoke(McpRequest::new("srv".into(), "echo".into(), args));
    (outcome, started.elapsed(), written_pid(&pid_file))
}

/// `reaped`: the pid is absent from `/proc` (our direct child, which we reap).
/// Otherwise dead is enough: absent or a zombie (an orphaned grandchild is
/// re-parented to a reaper that is not ours and may reap it late).
fn assert_gone_or_dead(pid: Option<String>, what: &str, reaped: bool) {
    match pid {
        #[cfg(target_os = "linux")]
        Some(pid) => {
            let status = std::fs::read_to_string(format!("/proc/{pid}/status"));
            let zombie = status.is_ok_and(|s| s.lines().any(|l| l.starts_with("State:\tZ")));
            let exists = std::path::Path::new(&format!("/proc/{pid}")).exists();
            assert!(
                !exists || (!reaped && zombie),
                "{what} {pid} must be killed{}",
                if reaped { " and reaped" } else { "" }
            );
        }
        _ => eprintln!("{what}: no pid written before the kill; nothing to check"),
    }
}

fn assert_gone(pid: Option<String>, what: &str) {
    assert_gone_or_dead(pid, what, true);
}

/// Story 18-8 AC3.2 (CR-7) — a server that never answers is `Timeout(ms)`
/// within the bound, and the child is killed and reaped (no zombie).
#[test]
fn a_server_that_never_answers_times_out_and_is_reaped() {
    let (outcome, elapsed, pid) = hung("hang", r#"echo $$ > "$0"; exec sleep 60"#, json!({}));
    assert!(
        matches!(outcome, Err(McpTransportError::Timeout(2000))),
        "expected Timeout(2000), got {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "bounded, took {elapsed:?}"
    );
    assert_gone(pid, "the timed-out child");
}

/// Review BH-3/AA-3 — a server that never reads stdin and never exits, sent a
/// request larger than the pipe buffer: the blocked WRITE is under the bound
/// too — `Timeout`, never a wedge — and the child is reaped.
#[test]
fn a_blocked_write_to_a_server_that_never_reads_times_out() {
    let big = "x".repeat(256 * 1024);
    let (outcome, elapsed, pid) = hung(
        "noread",
        r#"echo $$ > "$0"; exec sleep 60"#,
        json!({ "pad": big }),
    );
    assert!(
        matches!(outcome, Err(McpTransportError::Timeout(2000))),
        "expected Timeout(2000), got {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "bounded, took {elapsed:?}"
    );
    assert_gone(pid, "the never-reading child");
}

/// Review BH-4/AA-6 — a wrapper (no `exec`) whose grandchild holds stdout:
/// the timeout kills the whole process group, so the grandchild is gone too.
#[test]
fn a_timeout_kills_the_wrappers_grandchild() {
    let (outcome, elapsed, pid) = hung(
        "wrapper",
        r#"sleep 60 & echo $! > "$0"; wait; :"#,
        json!({}),
    );
    assert!(
        matches!(outcome, Err(McpTransportError::Timeout(2000))),
        "expected Timeout(2000), got {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "bounded, took {elapsed:?}"
    );
    // The grandchild is re-parented, not ours to reap: killed is the claim.
    std::thread::sleep(Duration::from_millis(200));
    assert_gone_or_dead(pid, "the wrapper's grandchild", false);
}

/// Review BH-3 — a server that answers but ignores stdin EOF and stays alive:
/// the answer is returned after at most the exit grace, and the child is
/// killed and reaped.
#[test]
fn an_answering_server_that_stays_alive_is_reaped_after_the_grace() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[]}}"#;
    let script = format!(r#"echo $$ > "$0"; read line; echo '{response}'; exec sleep 60"#);
    let (outcome, elapsed, pid) = hung("linger", &script, json!({}));
    assert_eq!(
        outcome.expect("the answer is returned").attribution.tool,
        "echo"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "the grace bounds the wait, took {elapsed:?}"
    );
    assert_gone(pid, "the lingering child");
}
