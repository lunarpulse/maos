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
        t3: None,
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

#[test]
fn frame_writer_roundtrips_through_local_consumer() {
    let mut bridge = bridge("exec cat", CliWrapperStdioShape::JsonRpcOverStdio);
    let mut writer = bridge.take_frame_writer().expect("take child stdin");
    assert_eq!(
        writer
            .write_frame_until(
                &vec![0; maos_frame_codec::MAX_FRAME_BYTES + 1],
                Instant::now() + Duration::from_secs(10),
            )
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    writer.write_frame(b"first").expect("blocking write");
    writer
        .write_frame_until(b"second", Instant::now() + Duration::from_secs(10))
        .expect("bounded write to the same pipe");
    drop(writer);

    let (seen, closed) = poll_until_closed(&mut bridge, Duration::from_secs(10));
    assert!(closed, "closing the writer must deliver EOF");
    assert_eq!(
        seen,
        vec![
            (SubStream::Stdout, b"first".to_vec()),
            (SubStream::Stdout, b"second".to_vec()),
        ],
        "the real child must roundtrip both canonical frames in order"
    );
}

#[test]
fn frame_writer_expired_deadline_times_out_and_emits_no_frame() {
    use std::io::ErrorKind::{BrokenPipe, TimedOut};

    let mut bridge = bridge("exec cat", CliWrapperStdioShape::JsonRpcOverStdio);
    let mut writer = bridge.take_frame_writer().expect("take child stdin");
    assert_eq!(
        writer
            .write_frame_until(b"never", Instant::now())
            .unwrap_err()
            .kind(),
        TimedOut
    );
    assert_eq!(
        writer
            .write_frame_until(b"later", Instant::now() + Duration::from_secs(10))
            .unwrap_err()
            .kind(),
        BrokenPipe
    );
    assert_eq!(writer.write_frame(b"later").unwrap_err().kind(), BrokenPipe);
    drop(writer);

    let (seen, closed) = poll_until_closed(&mut bridge, Duration::from_secs(10));
    assert!(closed, "closing the timed-out writer must deliver EOF");
    assert!(
        seen.is_empty(),
        "the real child received a post-deadline frame: {seen:?}"
    );
}

#[test]
fn frame_writer_full_pipe_times_out_and_refuses_new_frames() {
    use std::io::ErrorKind::{BrokenPipe, TimedOut};

    // An ordinary child keeps its pipe open without reading it. No test sleeps
    // to induce backpressure: the frame itself is larger than the Linux pipe.
    let mut bridge = bridge("exec sleep 600", CliWrapperStdioShape::JsonRpcOverStdio);
    let mut writer = bridge.take_frame_writer().expect("take child stdin");
    let (done, result) = std::sync::mpsc::sync_channel(1);
    let attempt = std::thread::spawn(move || {
        let body = vec![b'x'; 8 * 1024 * 1024];
        let first = writer.write_frame_until(&body, Instant::now() + Duration::from_millis(100));
        let second = writer.write_frame_until(b"next", Instant::now() + Duration::from_secs(10));
        done.send((first, second))
    });
    // An independent failure bound, not a timing oracle for the 100 ms poll.
    // On timeout, unwinding drops the bridge, killing and reaping the child
    // even if a broken blocking writer still owns stdin.
    let (first, second) = result
        .recv_timeout(Duration::from_secs(10))
        .expect("backpressured writer did not return under the independent bound");
    assert_eq!(first.unwrap_err().kind(), TimedOut);
    assert_eq!(second.unwrap_err().kind(), BrokenPipe);
    attempt
        .join()
        .expect("writer unwound")
        .expect("result sent");
}

/// The pipe-backpressure witness of the pre-warm stage wall (Story 21-9 AC8e): a child that never
/// reads its stdin cannot hold the host's writer past the wall. The blocked writer is released by
/// the wall itself (typed `TimedOut`, the same kind the pre-warm maps to its deadline refusal),
/// its thread finishes and is joined, and dropping the bridge reaps the child and leaves no fd
/// to the child's stdin pipe in this process.
#[test]
fn backpressured_writer_is_released_at_the_wall_and_the_child_is_reaped() {
    use std::io::ErrorKind::TimedOut;

    /// The inode token of the pipe behind `/proc/<pid>/fd/0` (`pipe:[N]`).
    fn child_stdin_pipe(pid: u32) -> String {
        let target = std::fs::read_link(format!("/proc/{pid}/fd/0")).expect("child stdin link");
        let target = target.to_string_lossy().into_owned();
        assert!(
            target.starts_with("pipe:["),
            "child stdin is not a pipe: {target}"
        );
        target
    }
    /// How many of this process's fds point at `pipe`.
    fn own_fds_on(pipe: &str) -> usize {
        std::fs::read_dir("/proc/self/fd")
            .expect("own fd table")
            .filter_map(Result::ok)
            .filter(|entry| {
                std::fs::read_link(entry.path())
                    .is_ok_and(|target| target.to_string_lossy() == pipe)
            })
            .count()
    }

    const WALL: Duration = Duration::from_millis(200);
    // A hard bound for a loaded machine, not a timing oracle for `WALL`.
    const HARD: Duration = Duration::from_secs(20);

    let mut bridge = bridge("exec sleep 600", CliWrapperStdioShape::JsonRpcOverStdio);
    let pid = bridge.child_pid();
    let mut writer = bridge.take_frame_writer().expect("take child stdin");
    let pipe = child_stdin_pipe(pid);
    assert_eq!(own_fds_on(&pipe), 1, "the writer holds the one write end");

    let (done, result) = std::sync::mpsc::sync_channel(1);
    let started = Instant::now();
    let thread = std::thread::spawn(move || {
        // Larger than any Linux pipe buffer: the write must block on backpressure.
        let body = vec![b'x'; 8 * 1024 * 1024];
        let outcome = writer.write_frame_until(&body, started + WALL);
        let _ = done.send((outcome, Instant::now()));
        // `writer`, and with it the stdin fd, is dropped as this thread ends.
    });
    let (outcome, returned_at) = result
        .recv_timeout(HARD)
        .expect("the wall did not release the blocked writer");
    assert_eq!(outcome.unwrap_err().kind(), TimedOut);
    assert!(
        returned_at.duration_since(started) >= WALL,
        "released before the wall: {:?}",
        returned_at.duration_since(started)
    );
    // The writer thread is finished, not parked: an observable join, not a sleep.
    thread.join().expect("the writer thread unwound");
    assert_eq!(
        own_fds_on(&pipe),
        0,
        "a stdin write end outlived its writer"
    );

    assert!(
        std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "the child must still be live until its owner tears it down"
    );
    drop(bridge);
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "child {pid} was not killed and reaped (no zombie either)"
    );
    assert_eq!(own_fds_on(&pipe), 0, "a stdin pipe fd leaked past teardown");
}
