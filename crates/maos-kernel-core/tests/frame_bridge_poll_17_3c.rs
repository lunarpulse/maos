#![forbid(unsafe_code)]
//! Story 17-3c — the duplex frame-bridge seam a governed session drives:
//! `poll_frame` termination, `kill`, and bounded stderr diagnostics, against a
//! real `/bin/sh` child (T0, hermetic, no network).
#![cfg(target_os = "linux")]

use std::time::{Duration, Instant};

use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgePoll;
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeSpawnSpec, ExitCause, SpawnedBridge,
    SubStream,
};
use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};

fn bridge(script: &str, stdio_shape: CliWrapperStdioShape) -> SpawnedBridge {
    let argv_prefix = vec!["-c".to_string()];
    spawn_and_bridge(BridgeSpawnSpec {
        program: "sh".to_string(),
        sandbox: maos_kernel_core::security::sandbox::SandboxSpec::new_for_test(
            maos_domain::invariants::i9::SandboxTier::T0,
        ),
        expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
        argv_prefix,
        task_args: vec![script.to_string()],
        from_spirit_id: "worker".to_string(),
        stdio_shape,
        control_channel: CliWrapperControlChannel::Signals,
        shutdown_signal: None,
        channel_capacity: 8,
        backpressure: Backpressure::Block,
        env: vec![],
    })
    .expect("spawn sh")
}

/// Poll until `Closed` or `deadline`, returning every data record seen.
fn poll_until_closed(
    bridge: &mut SpawnedBridge,
    deadline: Duration,
) -> (Vec<(SubStream, Vec<u8>)>, bool) {
    let until = Instant::now() + deadline;
    let mut seen = Vec::new();
    while Instant::now() < until {
        match bridge
            .poll_frame(Duration::from_millis(50))
            .expect("well-framed child")
        {
            BridgePoll::Data(stream, bytes) => seen.push((stream, bytes)),
            BridgePoll::Idle => {}
            BridgePoll::Closed => return (seen, true),
        }
    }
    (seen, false)
}

#[test]
fn stdout_eof_closes_the_bridge_even_while_stderr_stays_open() {
    // One frame, then stdout closes; `sleep` keeps stderr open for 3 s.
    let mut bridge = bridge(
        "printf 'Content-Length: 2\\r\\n\\r\\nok'; exec 1>&-; sleep 3",
        CliWrapperStdioShape::JsonRpcOverStdio,
    );
    let started = Instant::now();
    let (seen, closed) = poll_until_closed(&mut bridge, Duration::from_secs(10));
    assert!(
        closed,
        "stdout EOF must end the stream, not leave it Idle forever"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "closed while stderr was still held open"
    );
    assert_eq!(seen, vec![(SubStream::Stdout, b"ok".to_vec())]);
    // The child is still alive (sleeping): `kill` ends it without reaping.
    assert_eq!(bridge.try_exit_cause().unwrap(), None);
    bridge.kill().unwrap();
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let exit = bridge.wait_and_finalize(&journal, 3, None, |_| {});
    assert_eq!(exit.cause, ExitCause::Signaled { signal: 9 });
}

#[test]
fn malformed_frame_header_is_a_terminal_error_not_eof() {
    let mut bridge = bridge(
        "printf 'not a header\\n\\n'; sleep 1",
        CliWrapperStdioShape::JsonRpcOverStdio,
    );
    let until = Instant::now() + Duration::from_secs(10);
    let error = loop {
        assert!(Instant::now() < until, "framing error never surfaced");
        match bridge.poll_frame(Duration::from_millis(50)) {
            Err(error) => break error,
            Ok(BridgePoll::Closed) => panic!("a framing error must not look like a clean EOF"),
            Ok(_) => {}
        }
    };
    assert!(format!("{error:?}").contains("framing"), "{error:?}");
}

#[test]
fn kill_after_exit_is_a_no_op() {
    let mut bridge = bridge("exit 0", CliWrapperStdioShape::NdjsonOverStdio);
    let (_, closed) = poll_until_closed(&mut bridge, Duration::from_secs(10));
    assert!(closed);
    let until = Instant::now() + Duration::from_secs(10);
    while bridge.try_exit_cause().unwrap().is_none() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    bridge
        .kill()
        .expect("an exited child is already where kill wants it");
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let exit = bridge.wait_and_finalize(&journal, 3, None, |_| {});
    assert_eq!(exit.cause, ExitCause::Exited { code: 0 });
    bridge.kill().expect("a finalized child is a no-op too");
}

#[test]
fn overlong_stderr_never_journals_a_token_split_across_records() {
    // 64 lowercase hex chars: the log's capability-token shape.
    let token = "a1b2c3d4".repeat(8);
    // Lead lengths put the token across the 4096-byte record bound, keeping
    // a short and a long hex prefix inside the bound.
    for lead in [4050, 4020] {
        let script = format!(
            "awk 'BEGIN {{ s = sprintf(\"%{lead}s\", \"\"); gsub(/ /, \"x\", s); \
             printf \"%s{token}tail\\n\", s > \"/dev/stderr\"; \
             printf \"after\\n\" > \"/dev/stderr\" }}'"
        );
        let mut bridge = bridge(&script, CliWrapperStdioShape::NdjsonOverStdio);
        let journal = TransparencyLogAdapter::open_in_memory(0);
        let outcome = bridge.pump_to_journal(&journal, 5, "orchestrator", "sh", &[]);
        assert_eq!(
            outcome.stderr_lines, 2,
            "the long line is one record, the next intact"
        );
        bridge.wait_and_finalize(&journal, 5, None, |_| {});
        let rows = journal
            .query_frames(FrameFilter {
                spirit_pid: Some(5),
                ..Default::default()
            })
            .unwrap();
        let text = rows
            .iter()
            .map(|row| String::from_utf8_lossy(&row.payload_redacted).into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("after"), "the following line survives");
        for window in token.as_bytes().windows(12) {
            let fragment = std::str::from_utf8(window).unwrap();
            assert!(
                !text.contains(fragment),
                "lead {lead}: token fragment {fragment} reached the journal"
            );
        }
    }
}
