#![forbid(unsafe_code)]

//! Story 16-3 / D-16-3-O — the `run_bounded` full-pipe regression. A child
//! that fills BOTH pipe buffers (~64 KiB each) must still exit inside the
//! bound with both streams captured whole. The pre-fix shape drained
//! stdout/stderr only after `try_wait` yielded `Some`, so such a child
//! blocked on write forever, the deadline fired, and the helper panicked
//! `did not exit within Ns` — a hang misreported as a timeout
//! (`deferred-work.md:938`, closed by this story).

use maos_journey_test::{run_bounded, AuditDb, JourneyWorld};

const MIB: usize = 1024 * 1024;

#[test]
fn child_filling_both_pipes_exits_inside_the_bound() {
    let world = JourneyWorld::builder().audit(AuditDb::temp()).build();
    // 1 MiB of 'a' down stdout, then 1 MiB of 'b' down stderr; dd's progress
    // lines go to /dev/null so each stream carries exactly its own pattern.
    let script = "dd if=/dev/zero bs=1048576 count=1 2>/dev/null | tr '\\0' 'a'; \
                  dd if=/dev/zero bs=1048576 count=1 2>/dev/null | tr '\\0' 'b' >&2";
    let run = run_bounded(&world, "sh", &["-c", script], &[], 10, "full-pipe-drain");

    assert_eq!(run.stdout.len(), MIB, "stdout must be captured whole");
    assert_eq!(run.stderr.len(), MIB, "stderr must be captured whole");
    assert!(
        run.stdout.bytes().all(|b| b == b'a'),
        "stdout must hold only the 'a' pattern"
    );
    assert!(
        run.stderr.bytes().all(|b| b == b'b'),
        "stderr must hold only the 'b' pattern"
    );
}

#[test]
fn child_output_is_drained_but_capture_is_capped() {
    let world = JourneyWorld::builder().audit(AuditDb::temp()).build();
    let script = "dd if=/dev/zero bs=1048576 count=9 2>/dev/null | tr '\\0' 'a'; \
                  dd if=/dev/zero bs=1048576 count=9 2>/dev/null | tr '\\0' 'b' >&2";
    let run = run_bounded(&world, "sh", &["-c", script], &[], 10, "bounded-capture");

    assert_eq!(run.stdout.len(), 8 * MIB, "stdout capture must be capped");
    assert_eq!(run.stderr.len(), 8 * MIB, "stderr capture must be capped");
    assert!(
        run.stdout.bytes().all(|b| b == b'a'),
        "stdout must retain its prefix"
    );
    assert!(
        run.stderr.bytes().all(|b| b == b'b'),
        "stderr must retain its prefix"
    );
}
