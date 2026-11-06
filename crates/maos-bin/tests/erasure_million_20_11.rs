#![forbid(unsafe_code)]
//! Story 20-11: real disk-backed writer latency, completion and durable replay.

#[path = "support/shell_world_16_2.rs"]
mod shell_world;

use maos_control::{submit_and_wait, OperatorCommand, OperatorOutcome, SubmitOutcome};
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::FrameKind;
use shell_world::{kernel_world, regression_door, scheduler_with_hello, KernelWorld};
use std::sync::Arc;
use std::time::Duration;
#[test]
fn ac5_door_bounded_scan_admits_continuous_writer() {
    door_scan_with_continuous_writer(100_000);
}

#[test]
fn ac5_million_row_pending_forget_finishes_and_repeats_without_corruption() {
    door_scan_with_continuous_writer(1_000_000);
}

fn erasure_fixture_footprint(root: &std::path::Path) -> (u64, u64) {
    let mut bytes = (0, 0);
    for entry in std::fs::read_dir(root).expect("fixture footprint directory") {
        let entry = entry.expect("fixture footprint entry");
        let metadata = std::fs::symlink_metadata(entry.path()).expect("fixture metadata");
        if metadata.is_dir() {
            let child = erasure_fixture_footprint(&entry.path());
            bytes.0 += child.0;
            bytes.1 += child.1;
        } else {
            bytes.0 += metadata.len();
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt as _;
                bytes.1 += metadata.blocks() * 512;
            }
            #[cfg(not(unix))]
            {
                bytes.1 += metadata.len();
            }
        }
    }
    bytes
}

fn door_scan_with_continuous_writer(count: u64) {
    use rusqlite::OptionalExtension as _;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;
    struct StopOnDrop<'a>(&'a AtomicBool);
    impl Drop for StopOnDrop<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let world = kernel_world(2011);
    let scheduler = scheduler_with_hello(&world, 2011);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let door = regression_door(&world, &scheduler, &rt, 2011);
    let mut db = rusqlite::Connection::open(world.dir.path().join("transparency.sqlite"))
        .expect("seed connection");
    let body = vec![b'x'; 1024];
    let target_id = [0u8; 16];
    let mut footprint = (0_u64, 0_u64);
    // A single million-row transaction retains a second database-sized WAL.
    // Keep the real 1 KiB payload and row count, but checkpoint each bounded
    // seed batch before the door/writer latency interval starts.
    for first in (1..=count).step_by(4096) {
        let tx = db.transaction().expect("seed transaction");
        {
            let mut insert = tx.prepare("INSERT INTO transparency_log (frame_id, timestamp_ns, kind, spirit_pid, boot_nonce, capability_token, intent, payload_redacted, origin, correlation_id) VALUES (?1,?2,1,7,2011,NULL,'scan.filler',?3,0,NULL)").expect("seed statement");
            for i in first..=first.saturating_add(4095).min(count) {
                let mut id = [0_u8; 16];
                id[..8].copy_from_slice(&i.to_be_bytes());
                id[8..].copy_from_slice(&i.to_be_bytes());
                insert
                    .execute(rusqlite::params![id.as_slice(), i as i64, &body])
                    .expect("seed row");
            }
            if first == 1 {
                insert
                    .execute(rusqlite::params![
                        target_id.as_slice(),
                        0,
                        br#"{"principal_id":"scan-target@example.org"}"#.as_slice(),
                    ])
                    .expect("real principal-bearing frame");
            }
        }
        tx.commit().expect("seed commit");
        // Before truncation, both the just-committed WAL and copied database
        // are present: this captures the seed's physical high-water.
        let current = erasure_fixture_footprint(world.dir.path());
        footprint.0 = footprint.0.max(current.0);
        footprint.1 = footprint.1.max(current.1);
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .expect("truncate seed WAL");
    }
    eprintln!(
        "EC-13 fixture rows={count} seed_peak_apparent_bytes={} seed_peak_allocated_bytes={}",
        footprint.0, footprint.1
    );
    assert!(
        footprint.1 <= count * 1792 + 32 * 1024 * 1024,
        "large fixture retained a second database-sized WAL: rows={count}, footprint={footprint:?}"
    );
    let tl = Arc::clone(&world.transparency_log);
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_guard = StopOnDrop(stop.as_ref());
    let running = Arc::clone(&stop);
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let ready = Arc::clone(&barrier);
    let writer = std::thread::spawn(move || {
        let mut delays = Vec::new();
        ready.wait();
        while !running.load(Ordering::Acquire) {
            let start = Instant::now();
            tl.insert_frame_event(
                FrameKind::TaskAssign,
                8,
                None,
                "continuous.writer",
                b"concurrent",
                FrameOrigin::Kernel,
            );
            delays.push(start.elapsed());
            std::thread::sleep(Duration::from_millis(1));
        }
        delays
    });
    let forget = || OperatorCommand::ForgetMemory {
        principal: "scan-target@example.org".into(),
        reason: None,
    };
    // The completion is necessarily newer than this operation's entry fence.
    // Do not make its observer scan the entire 1 GiB fixture on every poll
    // after the routed-append full indexes have correctly been removed.
    let completion_after: i64 = db
        .query_row(
            "SELECT coalesce(max(rowid),0) FROM transparency_log",
            [],
            |row| row.get(0),
        )
        .expect("completion observation fence");
    barrier.wait();
    let start = Instant::now();
    let result = submit_and_wait(door.as_ref(), forget());
    eprintln!(
        "EC-13 route rows={count} wall={:?} response={result:?}",
        start.elapsed()
    );
    match result {
        SubmitOutcome::Completed(OperatorOutcome::Completed(ref report)) => {
            assert_eq!(report["outcome"], "erased");
            assert_eq!(report["terminal_code"], 0);
        }
        SubmitOutcome::HandlerStillRunning { ref operation_id } => {
            // The shipped route deadline is unchanged. This response is only
            // pending, never an erased report; its durable completion must be
            // observable within one further unchanged route-budget interval.
            eprintln!("EC-13 pending rows={count} operation_id={operation_id}");
            let deadline = Instant::now() + forget().route_budget();
            let intent = format!("operator.forget.{operation_id}:completed");
            loop {
                let completed: Option<Vec<u8>> = db
                    .query_row(
                        "SELECT payload_redacted FROM transparency_log WHERE rowid>?3 AND kind=?1 AND intent=?2",
                        rusqlite::params![FrameKind::TelemetryEvent as i64, intent, completion_after],
                        |row| row.get(0),
                    )
                    .optional()
                    .expect("completion lookup");
                if let Some(bytes) = completed {
                    let receipt: serde_json::Value = serde_json::from_slice(&bytes)
                        .unwrap_or_else(|error| panic!("completion receipt: {error}"));
                    assert_eq!(receipt["operation_id"], operation_id.as_str());
                    assert_eq!(receipt["outcome"], "completed");
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "pending operation did not finish: {operation_id}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        other => panic!("forget failed rather than completing or reporting pending: {other:?}"),
    }
    let elapsed = start.elapsed();
    stop.store(true, Ordering::Release);
    let delays = writer.join().expect("writer");
    let max = delays.iter().copied().max().expect("writer attempted");
    let ratio = max.as_secs_f64() / elapsed.as_secs_f64();
    // The wall-latency criterion is independent of the constituent VM-work bound.
    eprintln!("EC-13 door rows={count} wall={elapsed:?} max_insert={max:?} writes={} wall_ratio={ratio} response={result:?}", delays.len());
    let read_body = |id: &[u8]| {
        db.query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
            [id],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("frame body")
    };
    let tombstone = read_body(&target_id);
    let redacted: serde_json::Value =
        serde_json::from_slice(&tombstone).unwrap_or_else(|error| panic!("tombstone: {error}"));
    assert_eq!(redacted["redacted"], true);
    let repeated = submit_and_wait(door.as_ref(), forget());
    assert!(
        matches!(&repeated, SubmitOutcome::Completed(OperatorOutcome::Completed(report))
            if report["outcome"] == "erased" && report["terminal_code"] == 0),
        "identical request must return its completed report after the prior handler finishes: {repeated:?}"
    );
    assert_eq!(read_body(&target_id), tombstone);
    assert_eq!(read_body(&count.to_be_bytes().repeat(2)), body);
    let current = erasure_fixture_footprint(world.dir.path());
    footprint.0 = footprint.0.max(current.0);
    footprint.1 = footprint.1.max(current.1);
    eprintln!(
        "EC-13 fixture rows={count} peak_apparent_bytes={} peak_allocated_bytes={}",
        footprint.0, footprint.1
    );
    assert!(
        footprint.1 <= count * 1792 + 32 * 1024 * 1024,
        "fixture peak exceeded its concurrency-safe disk envelope: {footprint:?}"
    );
    drop(door);
    drop(rt);
    drop(scheduler);
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("truncate completed fixture WAL");
    drop(db);
    // Close every owner before TempDir cleanup, not merely at process exit.
    let KernelWorld {
        dir,
        transparency_log,
        journal,
        halt_registry,
        memory,
        capability,
        policy,
    } = world;
    drop((
        transparency_log,
        journal,
        halt_registry,
        memory,
        capability,
        policy,
    ));
    let path = dir.path().to_owned();
    dir.close().expect("remove large fixture immediately");
    assert!(
        !path.exists(),
        "large fixture directory leaked: {}",
        path.display()
    );
    assert!(
        ratio < 0.25,
        "writer blocked for {max:?} of {elapsed:?} at {count} rows (ratio={ratio})"
    );
}
#[test]
fn ac5_completed_forget_is_durable_marker_scoped_replay() {
    let world = kernel_world(2011);
    let scheduler = scheduler_with_hello(&world, 2011);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let door = regression_door(&world, &scheduler, &rt, 2011);
    world.transparency_log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "replay.subject",
        b"replay-subject@example.org",
        FrameOrigin::Kernel,
    );
    let forget = || OperatorCommand::ForgetMemory {
        principal: "replay-subject@example.org".into(),
        reason: None,
    };
    let first = match submit_and_wait(door.as_ref(), forget()) {
        SubmitOutcome::Completed(OperatorOutcome::Completed(report)) => report,
        other => panic!("first forget did not complete: {other:?}"),
    };
    assert_eq!(first["frame_sweep_status"], "complete");
    let receipts = || {
        world
            .transparency_log
            .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
                intent: Some("principal.forget".into()),
                ..Default::default()
            })
            .expect("actual cascade receipt count")
            .len()
    };
    assert_eq!(receipts(), 1);
    rt.block_on(door.drain_started_tasks());
    drop(door);
    // Reconstruct the door: a process-local response cache is not the oracle.
    let reopened = regression_door(&world, &scheduler, &rt, 2011);
    let repeated = match submit_and_wait(reopened.as_ref(), forget()) {
        SubmitOutcome::Completed(OperatorOutcome::Completed(report)) => report,
        other => panic!("identical completed request was not replayed: {other:?}"),
    };
    assert_eq!(
        receipts(),
        1,
        "an identical completed request restarted the real forget cascade"
    );
    assert_eq!(
        repeated["frame_sweep_completion"], first["frame_sweep_completion"],
        "replay published a new frame-erasure marker"
    );
    assert!(first["operation_id"].as_str().is_some());
    assert_eq!(repeated["operation_id"], first["operation_id"]);
    assert_eq!(repeated["replayed"], true);
    rt.block_on(reopened.drain_started_tasks());
}

fn completed_report(
    door: &impl maos_control::OperatorCommandPort,
    principal: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    match submit_and_wait(
        door,
        OperatorCommand::ForgetMemory {
            principal: principal.into(),
            reason: reason.map(str::to_owned),
        },
    ) {
        SubmitOutcome::Completed(OperatorOutcome::Completed(report))
            if report["outcome"] == "erased" && report["terminal_code"] == 0 =>
        {
            report
        }
        other => panic!("forget did not complete: {other:?}"),
    }
}

#[test]
fn ac5_replay_invalidates_for_new_matching_frames_and_changed_reason() {
    let world = kernel_world(2011);
    let scheduler = scheduler_with_hello(&world, 2011);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let door = regression_door(&world, &scheduler, &rt, 2011);
    let principal = "fresh-replay@example.org";
    let first = completed_report(door.as_ref(), principal, None);
    let id = world.transparency_log.insert_kernel_event_returning_id(
        7,
        FrameKind::TaskAssign,
        None,
        "post-marker.match",
        principal.as_bytes(),
    );
    let fresh = completed_report(door.as_ref(), principal, None);
    assert_ne!(fresh["operation_id"], first["operation_id"]);
    assert_eq!(fresh["replayed"], false);
    let frame = world
        .transparency_log
        .query_frame_by_id(id)
        .expect("post-marker frame")
        .expect("the real frame remains as a tombstone");
    let body = match serde_json::from_slice::<serde_json::Value>(&frame.payload_redacted) {
        Ok(body) => body,
        Err(error) => panic!("post-marker body was not a tombstone: {error}"),
    };
    assert_eq!(body["redacted"], true);
    let repeat = completed_report(door.as_ref(), principal, None);
    assert_eq!(repeat["operation_id"], fresh["operation_id"]);
    assert_eq!(repeat["replayed"], true);

    let changed = completed_report(door.as_ref(), principal, Some("new collection"));
    assert_ne!(changed["operation_id"], fresh["operation_id"]);
    assert_eq!(changed["replayed"], false);
    let repeat = completed_report(door.as_ref(), principal, Some("new collection"));
    assert_eq!(repeat["operation_id"], changed["operation_id"]);
    assert_eq!(repeat["replayed"], true);
    rt.block_on(door.drain_started_tasks());
}

#[test]
fn ac5_replay_tail_cap_starts_fresh_forget_instead_of_trusting_an_unchecked_tail() {
    let world = kernel_world(2011);
    let scheduler = scheduler_with_hello(&world, 2011);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let door = regression_door(&world, &scheduler, &rt, 2011);
    let principal = "tail-cap@example.org";
    let first = completed_report(door.as_ref(), principal, None);
    let mut db = rusqlite::Connection::open(world.dir.path().join("transparency.sqlite"))
        .expect("independent post-marker writer");
    let transaction = db.transaction().expect("finite unrelated tail");
    {
        let mut insert = transaction.prepare(
            "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
             VALUES(?1,0,99,2011,1,'tail.filler',X'78',3)",
        ).expect("real tail insert");
        for i in 1_u64..=4096 {
            let mut id = [0_u8; 16];
            id[..8].copy_from_slice(&i.to_be_bytes());
            id[8..].copy_from_slice(&i.to_be_bytes());
            insert
                .execute([id.as_slice()])
                .expect("unrelated committed tail row");
        }
    }
    transaction
        .commit()
        .expect("publish the full unchecked tail");
    let fresh = completed_report(door.as_ref(), principal, None);
    assert_ne!(fresh["operation_id"], first["operation_id"]);
    assert_eq!(fresh["replayed"], false);
    let repeat = completed_report(door.as_ref(), principal, None);
    assert_eq!(repeat["operation_id"], fresh["operation_id"]);
    assert_eq!(repeat["replayed"], true);
    rt.block_on(door.drain_started_tasks());
}

#[test]
fn ac5_replay_rechecks_the_current_global_hold() {
    let world = kernel_world(2011);
    let scheduler = scheduler_with_hello(&world, 2011);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let door = regression_door(&world, &scheduler, &rt, 2011);
    let principal = "held-replay@example.org";
    completed_report(door.as_ref(), principal, None);
    world
        .transparency_log
        .place_legal_hold(principal, "litigation", None, 1)
        .expect("new authoritative hold");
    let result = submit_and_wait(
        door.as_ref(),
        OperatorCommand::ForgetMemory {
            principal: principal.into(),
            reason: None,
        },
    );
    assert!(
        matches!(result, SubmitOutcome::Completed(OperatorOutcome::Completed(ref report))
            if report["outcome"] == "held" && report["terminal_code"] == 3),
        "a historical completion bypassed the current hold: {result:?}"
    );
    rt.block_on(door.drain_started_tasks());
}
