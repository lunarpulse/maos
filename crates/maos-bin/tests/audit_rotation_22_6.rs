#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 22-6 (ADR-072) — the bounded audit store, through the real adapter, the real sealer and
//! the real readers. Every test crosses a rotation; the five consumers that assume the insertion
//! id only grows (detector, progress stamper, forget proof, ratification sequence, forget sweep
//! windows) each have one.
//!
//! Proven red (recorded in the Dev Agent Record): see the `PROVEN RED` note on each test.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maos_audit::segment::{expected_pubkey, expired_segments, verify_chain, verify_segment};
use maos_bin::audit_rotation::floor_for;
use maos_bin::escape_detector_consumer::spawn_escape_anomaly_tail;
use maos_cli::audit_rotation::{rotate_once, DaemonSealer, RotationPolicy};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_iac::adapter::transparency_log::{
    ErasureSweep, FrameFilter, FrameKind, FrameRowWrite, RotationOutcome, SegmentRef,
    TransparencyLogAdapter,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const SEED: [u8; 32] = [0x22; 32];
const NONCE: u64 = 0x22_06;
const ALL: i64 = i64::MAX;

struct Fixture {
    _dir: tempfile::TempDir,
    db: PathBuf,
    tl: Arc<TransparencyLogAdapter>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("audit").join("transparency.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let tl = Arc::new(open(&db));
        Self { _dir: dir, db, tl }
    }

    fn segments(&self) -> PathBuf {
        self.db.parent().unwrap().join("segments")
    }

    fn manifest(&self, seq: u64) -> PathBuf {
        self.segments().join(format!("{seq:010}.manifest.json"))
    }

    fn rotate(&self) -> SegmentRef {
        match self.tl.rotate_through(ALL, ALL).expect("rotation") {
            RotationOutcome::Sealed(segment) => segment,
            other => panic!("expected a sealed segment, got {other:?}"),
        }
    }

    /// Rotate below every floor the Host registered (what the policy task does).
    fn rotate_below_floors(&self) -> SegmentRef {
        let floor = floor_for(&self.tl);
        match self.tl.rotate_through(ALL, floor).expect("rotation") {
            RotationOutcome::Sealed(segment) => segment,
            other => panic!("expected a sealed segment, got {other:?}"),
        }
    }

    fn live_rows(&self) -> i64 {
        raw(&self.db)
            .query_row("SELECT count(*) FROM transparency_log", [], |r| r.get(0))
            .unwrap()
    }
}

fn open(db: &Path) -> TransparencyLogAdapter {
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(db, db, NONCE).expect("open log");
    tl.install_segment_sealer(Arc::new(DaemonSealer::new(SEED, None, None)))
        .expect("one sealer");
    tl
}

fn raw(db: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).expect("raw connection")
}

fn id(n: u32) -> [u8; 16] {
    let mut id = [0xEE; 16];
    id[..4].copy_from_slice(&n.to_be_bytes());
    id
}

fn put(tl: &TransparencyLogAdapter, n: u32, body: &str) {
    put_as(tl, n, 7, body);
}

fn put_as(tl: &TransparencyLogAdapter, n: u32, pid: u32, body: &str) {
    let written = tl
        .insert_frame_event_with_id(
            Some(id(n)),
            FrameKind::TaskAssign,
            pid,
            "",
            "",
            None,
            &format!("test.frame.{n}"),
            body.as_bytes(),
            FrameOrigin::HumanAuthored,
        )
        .into_inner();
    assert_eq!(written, FrameRowWrite::Written, "frame {n}");
}

fn set_timestamp(db: &Path, rowid: i64, ts: i64) {
    raw(db)
        .execute(
            "UPDATE transparency_log SET timestamp_ns = ?1 WHERE rowid = ?2",
            [ts, rowid],
        )
        .unwrap();
}

fn pubkey() -> [u8; 32] {
    expected_pubkey(&SEED, None).unwrap()
}

fn now_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}

/// Every payload of segments ∪ live except the kernel's own `distillate.redacted` marker, which
/// names the forgotten principal by design (the live pass writes the same marker).
fn all_payloads(db: &Path) -> Vec<String> {
    maos_audit::query(db, maos_audit::AuditFilter::default())
        .unwrap()
        .into_iter()
        .filter(|entry| entry.intent != "distillate.redacted")
        .map(|entry| entry.payload)
        .collect()
}

// ── AC1 ──────────────────────────────────────────────────────────────────────

/// PROVEN RED: with `delete_sealed_prefix` skipping the live delete the live row count and the
/// registry assertions fail; with the manifest write removed `verify_segment` fails.
#[test]
fn rotation_seals_a_signed_segment_and_the_live_log_continues() {
    let f = Fixture::new();
    for n in 1..=20 {
        put(&f.tl, n, &format!("payload-{n}"));
    }
    let segment = f.rotate();
    assert_eq!(segment.seq, 1);
    assert_eq!(segment.row_count, 19, "all but the newest row");
    assert_eq!((segment.after_rowid, segment.last_rowid), (0, 19));
    assert!(f.segments().join("0000000001.sqlite").is_file());
    let manifest = verify_segment(&f.manifest(1), &pubkey()).expect("the segment verifies alone");
    assert_eq!(manifest.row_count, 19);
    assert_eq!(manifest.prev_manifest_sha256, None);
    assert_eq!(f.live_rows(), 1, "the sealed prefix left the live file");
    assert_eq!(f.tl.sealed_id_count().unwrap(), 19);

    // The live log keeps serving: one more frame, and a query over both halves.
    put(&f.tl, 21, "payload-21");
    let rows =
        f.tl.query_frames(FrameFilter {
            order_by_insertion: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 21);
    assert_eq!(
        rows.iter().map(|r| r.insertion_id).collect::<Vec<_>>(),
        (1..=21).collect::<Vec<i64>>(),
        "one global insertion order across the segment and the live file"
    );
    let second = f.rotate();
    assert_eq!(second.seq, 2);
    assert_eq!(
        second.prev_manifest_sha256,
        Some(segment.manifest_sha256.clone())
    );
    assert_eq!(second.after_rowid, 19, "ranges are contiguous");
    let chain = verify_chain(&f.segments(), &[], &|_| Some(pubkey()));
    assert!(chain.problems.is_empty(), "{chain:?}");
    assert_eq!(chain.segments, 2);
}

// ── AC2 ──────────────────────────────────────────────────────────────────────

/// PROVEN RED: replacing the merged read with a per-file concatenation (segments, then live)
/// fails the clock-step fixture below: the live rows are OLDER than the sealed ones.
#[test]
fn cross_segment_query_equals_the_unrotated_twin() {
    let rotated = Fixture::new();
    let twin = Fixture::new();
    for f in [&rotated, &twin] {
        for n in 1..=30 {
            put(&f.tl, n, &format!("payload-{n}"));
        }
        // The clock steps BACK across the segment boundary: rows 21..=30 are older than 1..=20.
        for rowid in 1..=30 {
            let ts = if rowid <= 20 {
                10_000 + rowid
            } else {
                500 + rowid
            };
            set_timestamp(&f.db, rowid, ts);
        }
    }
    let segment = rotated.tl.rotate_through(ALL, 20).unwrap();
    assert!(matches!(&segment, RotationOutcome::Sealed(s) if s.last_rowid == 20));

    let json = |db: &Path, filter: maos_audit::AuditFilter| {
        serde_json::to_string(&maos_audit::query(db, filter).unwrap()).unwrap()
    };
    for (name, filter) in [
        ("all", maos_audit::AuditFilter::default()),
        (
            "limit",
            maos_audit::AuditFilter {
                limit: Some(12),
                ..Default::default()
            },
        ),
        (
            "range",
            maos_audit::AuditFilter {
                since_ns: Some(520),
                until_ns: Some(10_005),
                ..Default::default()
            },
        ),
    ] {
        assert_eq!(
            json(&rotated.db, filter.clone()),
            json(&twin.db, filter),
            "maos_audit::query `{name}` differs from the unrotated twin"
        );
    }
    let times: Vec<u64> = maos_audit::query(&rotated.db, maos_audit::AuditFilter::default())
        .unwrap()
        .iter()
        .map(|e| e.timestamp_ns)
        .collect();
    assert!(
        times.windows(2).all(|w| w[0] <= w[1]),
        "time order: {times:?}"
    );
    assert_eq!(times[0], 521, "the live (older) rows come first");

    for filter in [
        FrameFilter::default(),
        FrameFilter {
            descending: true,
            limit: Some(7),
            ..Default::default()
        },
        FrameFilter {
            order_by_insertion: true,
            cursor_frame_id: Some(id(5)), // the anchor sits in the sealed segment
            ..Default::default()
        },
        FrameFilter {
            spirit_pid: Some(7),
            limit: Some(25),
            ..Default::default()
        },
    ] {
        let view = |tl: &TransparencyLogAdapter| {
            tl.query_frames(filter.clone())
                .unwrap()
                .into_iter()
                .map(|e| (e.insertion_id, e.frame_id, e.timestamp_ns))
                .collect::<Vec<_>>()
        };
        assert_eq!(view(&rotated.tl), view(&twin.tl), "{filter:?}");
        let meta = |tl: &TransparencyLogAdapter| {
            tl.query_frame_metadata(filter.clone())
                .unwrap()
                .into_iter()
                .map(|m| (m.frame_id, m.timestamp_ns, m.payload_len))
                .collect::<Vec<_>>()
        };
        assert_eq!(meta(&rotated.tl), meta(&twin.tl), "metadata {filter:?}");
    }
    assert_eq!(
        rotated.tl.all_frame_ids().unwrap(),
        twin.tl.all_frame_ids().unwrap()
    );
    // Subject access: a sealed frame resolves by id and carries the same body.
    let sealed = rotated
        .tl
        .query_frame_by_id(id(3))
        .unwrap()
        .expect("sealed frame");
    assert_eq!(sealed.payload_redacted, b"payload-3");
    assert_eq!(sealed.insertion_id, 3);
}

/// PROVEN RED: with `query_frame_by_id` reading the live file only this write is refused with
/// `SourceFrameNotFound` (`distillate.rs:323-334`).
#[test]
fn distillate_citing_a_sealed_frame_is_written() {
    use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
    use maos_domain::ports::DistillationPort;
    let f = Fixture::new();
    for n in 1..=10 {
        put_as(&f.tl, n, 7, &format!("raw-{n}"));
    }
    f.rotate();
    assert!(f.live_rows() < 10);
    let writer =
        maos_iac::adapter::distillate::DistillateWriter::new(Arc::clone(&f.tl), Arc::new(0_u64));
    let request = DistillationRequest::new(
        vec![id(2), id(3)],
        1,
        DigestPayload::Text("digest of sealed frames".into()),
        None,
    )
    .unwrap();
    let receipt = writer
        .write_distillate(7, request)
        .expect("a sealed source frame is still a source frame");
    assert!(f
        .tl
        .query_frame_by_id(receipt.digest_frame_id)
        .unwrap()
        .is_some());
    let unknown =
        DistillationRequest::new(vec![[0xDE; 16]], 1, DigestPayload::Text("x".into()), None)
            .unwrap();
    assert!(matches!(
        writer.write_distillate(7, unknown),
        Err(DistillationError::SourceFrameNotFound { .. })
    ));
}

/// D-22-6-09 A: a peer that replays the id of a sealed frame gets `Duplicate`, not a second row.
#[test]
fn a_replayed_sealed_frame_id_is_a_duplicate_not_a_second_row() {
    let f = Fixture::new();
    for n in 1..=6 {
        put(&f.tl, n, "x");
    }
    f.rotate();
    let before = f.tl.all_frame_ids().unwrap().len();
    let replay =
        f.tl.insert_frame_event_with_id(
            Some(id(2)),
            FrameKind::TaskAssign,
            7,
            "",
            "",
            None,
            "replayed",
            b"again",
            FrameOrigin::HumanAuthored,
        )
        .into_inner();
    assert_eq!(replay, FrameRowWrite::Duplicate);
    assert_eq!(f.tl.all_frame_ids().unwrap().len(), before);
}

// ── AC3 ──────────────────────────────────────────────────────────────────────

fn bus_frame(id: [u8; 16], n: u32) -> maos_domain::frame::IacFrame {
    use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_iac::adapter::mailbox::KERNEL_SENDER_SPIRIT_ID;
    use maos_spirit_abi::identity::{FrameKind as WireKind, SpiritId};
    let address = |who: &str| FrameAddress {
        spirit_id: SpiritId::from(who),
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: address(KERNEL_SENDER_SPIRIT_ID),
        to: smallvec::smallvec![address("test-spirit")],
        kind: WireKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("goal-{n}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::Kernel,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

/// Offer `count` frames from one sender thread through the real bus over `log`; returns the ids
/// whose delivery the receiving handle observed.
fn offer(log: Arc<TransparencyLogAdapter>, sender: u8, count: u32) -> Vec<[u8; 16]> {
    use maos_domain::ports::IacBusPort;
    use maos_iac::adapter::mailbox::Mailbox;
    use maos_iac::adapter::{IacBusAdapter, IacRtMetrics};
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut handle = mailbox.register_spirit("test-spirit").expect("register");
    let bus = IacBusAdapter::new(Arc::clone(&mailbox), log);
    let mut delivered = Vec::new();
    for n in 0..count {
        let mut frame_id = [0xA0; 16];
        frame_id[0] = sender;
        frame_id[1..5].copy_from_slice(&n.to_be_bytes());
        let offered = bus_frame(frame_id, n);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(bus.deliver(offered))
        }));
        if matches!(handle.try_recv(), Ok(Some(_))) {
            delivered.push(frame_id);
        }
    }
    delivered
}

/// PROVEN RED: a rotation whose delete re-reads `MAX(rowid)` inside its transaction (instead of
/// using the cut it copied) deletes rows that arrived after the copy — they were never sealed —
/// and the exactly-once count below fails.
#[test]
fn log_before_deliver_holds_across_rotation() {
    let f = Fixture::new();
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let rotator = {
        let tl = Arc::clone(&f.tl);
        let done = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut sealed = 0;
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok(RotationOutcome::Sealed(_)) = tl.rotate_through(ALL, ALL) {
                    sealed += 1;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            sealed
        })
    };
    let senders: Vec<_> = (1..=3_u8)
        .map(|sender| {
            let tl = Arc::clone(&f.tl);
            std::thread::spawn(move || offer(tl, sender, 150))
        })
        .collect();
    let delivered: Vec<[u8; 16]> = senders
        .into_iter()
        .flat_map(|s| s.join().expect("sender"))
        .collect();
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    let sealed = rotator.join().expect("rotator");
    assert!(sealed >= 1, "rotations ran while senders delivered");
    assert_eq!(delivered.len(), 450, "every writable offer was delivered");

    let mut seen: BTreeMap<[u8; 16], usize> = BTreeMap::new();
    for frame_id in f.tl.all_frame_ids().unwrap() {
        *seen.entry(frame_id).or_default() += 1;
    }
    for frame_id in &delivered {
        assert_eq!(
            seen.get(frame_id),
            Some(&1),
            "a delivered frame is in segments ∪ live exactly once"
        );
    }

    // A failed write still delivers nothing: a read-only log refuses every write.
    let read_only = Arc::new(TransparencyLogAdapter::open_read_only(&f.db).expect("read-only log"));
    let rows_before = f.tl.all_frame_ids().unwrap().len();
    assert!(
        offer(read_only, 9, 10).is_empty(),
        "nothing is delivered unlogged"
    );
    assert_eq!(f.tl.all_frame_ids().unwrap().len(), rows_before);
}

struct Lines(UnboundedSender<String>);

impl Write for Lines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for line in String::from_utf8_lossy(buf).lines() {
            let _ = self.0.send(line.to_string());
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(line) = rx.try_recv() {
        lines.push(line);
    }
    lines
}

fn kill(tl: &TransparencyLogAdapter, pid: u32) {
    tl.insert_frame_event(
        FrameKind::SandboxBlock,
        pid,
        None,
        "sandbox.block.seccomp-kill-process (SIGSYS; syscall number unavailable)",
        b"tier=2",
        FrameOrigin::Kernel,
    );
}

/// Spawn a tail that never ticks (1 h), put filler rows around a kill row written after the
/// tail's cursor, rotate with the floor `floor_for` publishes (or without it), then run the
/// final scan. Returns the anomaly lines and whether the kill row is still live.
async fn tail_over_a_rotation(with_floor: bool) -> (Vec<String>, bool) {
    let f = Fixture::new();
    for n in 1..=4 {
        put(&f.tl, n, "before");
    }
    let (tx, mut rx) = unbounded_channel();
    let tail = spawn_escape_anomaly_tail(
        f.db.clone(),
        NONCE,
        Duration::from_secs(3600),
        Box::new(Lines(tx)),
        Box::new(|_| {}),
    );
    tail.declare(4242, SandboxTier::T2);
    // After the tail's last scan (it has none): the kill, then rows that outrank it.
    kill(&f.tl, 4242);
    for n in 5..=8 {
        put(&f.tl, n, "after");
    }
    let floor = if with_floor { floor_for(&f.tl) } else { ALL };
    f.tl.rotate_through(ALL, floor).expect("rotation");
    let kill_live = raw(&f.db)
        .query_row(
            "SELECT count(*) FROM transparency_log WHERE intent LIKE 'sandbox.block.%'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
        == 1;
    tail.shutdown(true, false).await;
    let lines = drain(&mut rx);
    (
        lines
            .into_iter()
            .filter(|l| l.starts_with("escape-anomaly:"))
            .collect(),
        kill_live,
    )
}

/// PROVEN RED: with the tail's floor dropped from the cut (`floor_for` replaced by `i64::MAX`)
/// the kill row is sealed away from the tail's read-only connection and the final scan surfaces
/// nothing. The `without the floor` leg below is that plant, run as a control.
#[tokio::test(flavor = "multi_thread")]
async fn escape_tail_surfaces_a_kill_sealed_by_a_rotation_exactly_once() {
    let (anomalies, kill_live) = tail_over_a_rotation(true).await;
    assert!(
        kill_live,
        "the floor kept the unscanned kill row in the live log"
    );
    assert_eq!(anomalies.len(), 1, "surfaced exactly once: {anomalies:?}");
    assert!(anomalies[0].contains("spirit_pid=4242"), "{anomalies:?}");

    let (lost, kill_live) = tail_over_a_rotation(false).await;
    assert!(
        !kill_live,
        "control: without the floor the kill row is sealed"
    );
    assert!(
        lost.is_empty(),
        "control: and the anomaly is lost silently: {lost:?}"
    );
}

/// PROVEN RED: a rotation that seals the newest row (cut = high-water) lets SQLite reuse an id;
/// the strictly-increasing assertion fails.
#[test]
fn insertion_ids_keep_growing_after_a_rotation_that_seals_all_but_the_newest_row() {
    let f = Fixture::new();
    for n in 1..=5 {
        put(&f.tl, n, "x");
    }
    let segment = f.rotate();
    assert_eq!(segment.last_rowid, 4, "the newest row is never sealed");
    put(&f.tl, 6, "after");
    let live: Vec<i64> = raw(&f.db)
        .prepare("SELECT rowid FROM transparency_log ORDER BY rowid")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(live, vec![5, 6], "the next id is above every sealed id");
    let second = f.rotate();
    assert_eq!(second.last_rowid, 5);
    put(&f.tl, 7, "later");
    let ids: Vec<i64> =
        f.tl.query_frames(FrameFilter {
            order_by_insertion: true,
            ..Default::default()
        })
        .unwrap()
        .iter()
        .map(|e| e.insertion_id)
        .collect();
    assert_eq!(ids, (1..=7).collect::<Vec<_>>());
}

/// PROVEN RED: with the stamper's floor dropped from the cut the row it has yet to poll is sealed
/// and polled from the segment instead — here the high-water cursor would skip it; the pid set
/// below loses 13.
#[test]
fn progress_stamper_misses_no_row_across_rotation() {
    use maos_bin::supervision::ProgressCursor;
    let f = Fixture::new();
    let output = |n: u32, pid: u32| {
        f.tl.insert_frame_event_with_id(
            Some(id(n)),
            FrameKind::CliSubprocessOutput,
            pid,
            "",
            "",
            None,
            "cli.stdout",
            b"line",
            FrameOrigin::Kernel,
        );
    };
    let mut stamper = ProgressCursor::new(&f.tl);
    for n in 1..=4 {
        put(&f.tl, n, "filler");
    }
    output(5, 11);
    output(6, 12);
    assert_eq!(stamper.poll(&f.tl).unwrap(), BTreeSet::from([11, 12]));
    assert_eq!(
        floor_for(&f.tl),
        6,
        "the stamper publishes its committed cursor"
    );

    output(7, 13); // written after the last poll, before the rotation
    put(&f.tl, 8, "filler");
    let sealed = f.rotate_below_floors();
    assert_eq!(
        sealed.last_rowid, 6,
        "the cut stops at the stamper's cursor"
    );
    output(9, 14);
    output(10, 15);
    assert_eq!(
        stamper.poll(&f.tl).unwrap(),
        BTreeSet::from([13, 14, 15]),
        "no row missed across the rotation, none repeated"
    );
    f.rotate_below_floors();
    assert!(stamper.poll(&f.tl).unwrap().is_empty());
    output(11, 16);
    assert_eq!(stamper.poll(&f.tl).unwrap(), BTreeSet::from([16]));
}

/// Consumer: the governance ratification sequence is the insertion id — one order across segments.
#[test]
fn ratification_sequence_is_one_order_across_rotation() {
    use maos_domain::governance::{
        AbiExtensionProposal, GovernanceEventKind, GovernanceEventPayload, RatificationStatus,
    };
    let f = Fixture::new();
    let ratified = |proposal: &str| {
        serde_json::to_vec(&GovernanceEventPayload {
            recorded_at_ns: 1,
            effective_at_ns: 1,
            event: GovernanceEventKind::AbiExtension(AbiExtensionProposal {
                proposal_id: proposal.into(),
                summary: "22-6 rotation proposal".into(),
                adr_ref: "ADR-045".into(),
                status: RatificationStatus::Ratified,
                covered_changes: vec!["IssuanceQuota".into()],
            }),
        })
        .unwrap()
    };
    let write = |proposal: &str| {
        f.tl.insert_frame_event(
            FrameKind::GovernanceEvent,
            7,
            None,
            "governance:abi-extension-ratified",
            &ratified(proposal),
            FrameOrigin::Kernel,
        );
    };
    put(&f.tl, 1, "filler");
    write("first");
    put(&f.tl, 2, "filler");
    let before: Vec<(String, i64)> = maos_audit::load_ratification_frames(&f.db)
        .unwrap()
        .into_iter()
        .map(|r| (r.proposal_id, r.seq))
        .collect();
    f.rotate();
    write("second");
    let after: Vec<(String, i64)> = maos_audit::load_ratification_frames(&f.db)
        .unwrap()
        .into_iter()
        .map(|r| (r.proposal_id, r.seq))
        .collect();
    assert_eq!(
        after.len(),
        2,
        "the sealed ratification is still read: {after:?}"
    );
    assert_eq!(after[0], before[0], "a sealed frame keeps its sequence");
    assert!(
        after[0].1 < after[1].1,
        "strictly ordered across the boundary: {after:?}"
    );
}

// ── AC4 ──────────────────────────────────────────────────────────────────────

/// PROVEN RED: with `rotation_due` comparing file size (which never shrinks) the second half
/// re-triggers every pass; with rotation disabled the live file grows past 2·B.
#[test]
fn live_log_stays_within_its_bound() {
    const B: u64 = 1024 * 1024;
    let f = Fixture::new();
    let policy = RotationPolicy {
        rotate_bytes: B,
        rotate_days: 36_500,
        keep_days: None,
    };
    let body = |n: u32| format!("{n:08}-{}", "x".repeat(4_000));
    let mut written = 0u64;
    let mut n = 0;
    while written < 10 * B {
        for _ in 0..64 {
            n += 1;
            put(&f.tl, n, &body(n));
            written += 4_100;
        }
        rotate_once(&f.tl, &policy, ALL, false, false).expect("policy pass");
    }
    raw(&f.db)
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    let size = std::fs::metadata(&f.db).unwrap().len();
    assert!(
        size <= 2 * B,
        "live file {size} bytes exceeds 2·B = {}",
        2 * B
    );
    let segments = f.tl.segment_registry().unwrap();
    assert!(segments.len() >= 5, "{} segments", segments.len());
    let sealed: u64 = segments.iter().map(|s| s.row_count).sum();
    assert_eq!(
        sealed + f.live_rows() as u64,
        u64::from(n),
        "no row lost or duplicated"
    );
    let chain = verify_chain(&f.segments(), &[], &|_| Some(pubkey()));
    assert!(chain.problems.is_empty(), "{:?}", chain.problems);
    assert!(
        segments
            .iter()
            .all(|s| f.segments().join(s.file_name()).is_file()),
        "without audit_keep_days no segment is ever deleted"
    );
}

/// PROVEN RED: with the hold check removed from `expire_segments` the held, old segment expires.
#[test]
fn expiry_is_off_by_default_and_never_removes_a_held_or_young_segment() {
    let f = Fixture::new();
    let old = now_ns() - 200 * 86_400 * 1_000_000_000;
    // Segment 1: old, holds a legal-held principal's frame. Segment 2: old, unheld.
    // Segment 3: young. A newest row stays live each time.
    put(&f.tl, 1, r#"{"principal_id":"held@example.org"}"#);
    put(&f.tl, 2, "plain");
    put(&f.tl, 3, "plain");
    f.tl.place_legal_hold("held@example.org", "case", Some("CASE-1"), 1)
        .expect("hold");
    for rowid in 1..=2 {
        set_timestamp(&f.db, rowid, old);
    }
    f.rotate();
    for n in 4..=5 {
        put(&f.tl, n, "plain");
    }
    for rowid in 3..=4 {
        set_timestamp(&f.db, rowid, old);
    }
    f.rotate();
    for n in 6..=7 {
        put(&f.tl, n, "young");
    }
    f.rotate();
    assert_eq!(f.tl.segment_registry().unwrap().len(), 3);

    // Off by default: no keep_days, nothing is ever deleted.
    let off = RotationPolicy {
        rotate_bytes: u64::MAX,
        rotate_days: 36_500,
        keep_days: None,
    };
    let report = rotate_once(&f.tl, &off, ALL, false, true).unwrap();
    assert!(report.expired.is_empty());
    assert!(f.manifest(1).is_file() && f.manifest(2).is_file() && f.manifest(3).is_file());

    // On: below the floor is refused.
    assert!(f.tl.expire_segments(30, now_ns() as u64).is_err());
    // On at 90 days: only the old, unheld segment goes.
    let on = RotationPolicy {
        keep_days: Some(90),
        ..off
    };
    let report = rotate_once(&f.tl, &on, ALL, false, true).unwrap();
    assert_eq!(
        report.expired,
        vec![2],
        "the held and the young segments stay"
    );
    assert!(f.manifest(1).is_file(), "held segment kept");
    assert!(!f.manifest(2).exists() && !f.segments().join("0000000002.sqlite").exists());
    assert!(f.manifest(3).is_file(), "young segment kept");
    let journaled =
        f.tl.query_frames(FrameFilter {
            intent: Some("audit.segment.expired".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(journaled.len(), 1, "the expiry is journaled");

    // The chain reports the gap as explained — and only because the row exists.
    let expired = expired_segments(&f.db).unwrap();
    assert_eq!(expired.len(), 1);
    let explained = verify_chain(&f.segments(), &expired, &|_| Some(pubkey()));
    assert!(explained.problems.is_empty(), "{explained:?}");
    assert_eq!(explained.explained_gaps, 1);
    let unexplained = verify_chain(&f.segments(), &[], &|_| Some(pubkey()));
    assert_eq!(unexplained.problems.len(), 1, "{unexplained:?}");

    // The next rotation still chains past the expired segment and ids stay contiguous.
    put(&f.tl, 8, "later");
    put(&f.tl, 9, "later");
    let fourth = f.rotate();
    assert_eq!(fourth.seq, 4);
}

fn intent_payload(principal: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "principal_hex": TransparencyLogAdapter::grouped_hex(principal.as_bytes()),
        "reason": "gdpr",
    }))
    .unwrap()
}

fn kernel_row(tl: &TransparencyLogAdapter, intent: &str, payload: &[u8]) -> [u8; 16] {
    tl.insert_kernel_event_returning_id(0, FrameKind::Decision, None, intent, payload)
}

/// PROVEN RED: with the open-intent floor removed from `rotation_floor` the intent is sealed
/// and `outstanding_forget_intents` no longer finds it.
#[test]
fn rotation_never_seals_an_open_forget_intent() {
    let f = Fixture::new();
    for n in 1..=3 {
        put(&f.tl, n, r#"{"principal_id":"user:carol"}"#);
    }
    let intent = kernel_row(
        &f.tl,
        "principal.forget.intent",
        &intent_payload("user:carol"),
    );
    for n in 4..=6 {
        put(&f.tl, n, "after");
    }
    let first = f.rotate();
    assert_eq!(first.row_count, 3, "the cut stops below the open intent");
    let open = f.tl.outstanding_forget_intents(None).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].0, intent);
    assert_eq!(open[0].1.as_deref(), Some("user:carol"));
    assert!(matches!(
        f.tl.rotate_through(ALL, ALL).unwrap(),
        RotationOutcome::NothingToSeal { .. }
    ));

    // An outstanding redistill request and the newest completed-forget marker hold the cut too.
    let closed = format!(
        "{{\"completed_from_intent\":\"{}\"}}",
        TransparencyLogAdapter::grouped_hex(&intent)
    );
    kernel_row(&f.tl, "principal.forget", closed.as_bytes());
    assert!(f.tl.outstanding_forget_intents(None).unwrap().is_empty());
    let request = maos_domain::memory::RedistillRequest {
        redacted_distillate_frame_id: format!("{:032x}", 0xD157_u128),
        writer_spirit_pid: 7,
        writer_boot_nonce: (NONCE as u128).to_le_bytes(),
        surviving_source_log_ref: vec![],
    };
    f.tl.insert_redistill_request(&request).unwrap();
    put(&f.tl, 7, "tail");
    f.rotate();
    assert_eq!(
        raw(&f.db)
            .query_row(
                "SELECT count(*) FROM transparency_log WHERE intent = 'distillate.redistill.requested'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1,
        "the outstanding request stays in the live log"
    );
    let pending = f.tl.outstanding_redistill_requests().unwrap();
    assert_eq!(
        pending.len(),
        1,
        "the request is still readable from the live log"
    );
    kernel_row(&f.tl, "principal.uninstall.frame_sweep_complete", b"{}");
    put(&f.tl, 8, "tail");
    let before = f.live_rows();
    f.tl.rotate_through(ALL, ALL).unwrap();
    assert!(
        raw(&f.db)
            .query_row(
                "SELECT count(*) FROM transparency_log WHERE intent = 'principal.uninstall.frame_sweep_complete'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap()
            == 1,
        "the completed-forget marker stays live ({before} live rows)"
    );
}

// ── AC5 ──────────────────────────────────────────────────────────────────────

/// PROVEN RED: with the sealed-segment sweep removed (`sweep_sealed_segments` a no-op) the
/// subject-access assertion fails: the principal's payloads are still in the segment.
#[test]
fn forget_scrubs_sealed_segments_and_reseals_them() {
    use maos_domain::distillation::{DigestPayload, DistillationRequest};
    use maos_domain::ports::DistillationPort;
    let f = Fixture::new();
    for n in 1..=4 {
        put_as(
            &f.tl,
            n,
            7,
            &format!(r#"{{"principal_id":"user:alice","note":"secret-{n}"}}"#),
        );
    }
    for n in 5..=6 {
        put_as(
            &f.tl,
            n,
            8,
            r#"{"principal_id":"user:bob","note":"bob-keeps"}"#,
        );
    }
    let writer =
        maos_iac::adapter::distillate::DistillateWriter::new(Arc::clone(&f.tl), Arc::new(0_u64));
    let receipt = writer
        .write_distillate(
            7,
            DistillationRequest::new(
                vec![id(1)],
                1,
                DigestPayload::Text("alice digest".into()),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    put(&f.tl, 7, "newest");
    let segment = f.rotate();
    assert!(segment.row_count >= 7);
    let sealed_manifest = std::fs::read(f.manifest(1)).unwrap();
    assert!(
        all_payloads(&f.db).iter().any(|p| p.contains("user:alice")),
        "precondition: the principal is in the sealed segment"
    );

    let (distillates, frames, requests, held, malformed) =
        f.tl.scrub_forget_frames("user:alice")
            .expect("forget reaches the segment");
    assert_eq!(frames.len(), 4, "the four sealed frames: {frames:?}");
    assert_eq!(
        distillates,
        vec![format!(
            "{:032x}",
            u128::from_be_bytes(receipt.digest_frame_id)
        )]
    );
    assert_eq!(
        requests.len(),
        1,
        "the sealed distillate asks to be re-distilled"
    );
    assert!(held.is_empty() && malformed.is_empty());

    // Subject access afterwards: zero rows for the principal; the other principal is intact.
    let payloads = all_payloads(&f.db);
    assert!(
        payloads
            .iter()
            .all(|p| !p.contains("user:alice") && !p.contains("secret-")),
        "{payloads:?}"
    );
    assert_eq!(
        payloads.iter().filter(|p| p.contains("bob-keeps")).count(),
        2
    );

    // The re-sealed segment verifies, names what it supersedes and what it scrubbed.
    let manifest = verify_segment(&f.manifest(1), &pubkey()).expect("re-sealed segment verifies");
    assert_eq!(manifest.supersedes.len(), 1);
    assert_eq!(manifest.supersedes[0], sha256_hex(&sealed_manifest));
    assert_eq!(
        manifest.scrubbed_frame_ids.len(),
        5,
        "four frames and the distillate"
    );
    assert!(manifest.scrubbed_frame_ids.contains(&hex_of(id(1))));
    let resealed =
        f.tl.query_frames(FrameFilter {
            intent: Some("audit.segment.resealed".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(resealed.len(), 1, "the re-seal is journaled");
    // The uninstall proof inventory covers segments.
    assert_eq!(
        f.tl.all_frame_ids().unwrap().len() as i64,
        f.live_rows() + segment.row_count as i64
    );
    // A second forget finds nothing left and does not re-seal.
    f.tl.scrub_forget_frames("user:alice").unwrap();
    assert_eq!(
        verify_segment(&f.manifest(1), &pubkey())
            .unwrap()
            .supersedes
            .len(),
        1,
        "an idempotent forget does not re-seal"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

fn hex_of(id: [u8; 16]) -> String {
    format!("{:032x}", u128::from_be_bytes(id))
}

/// PROVEN RED: with the intent floor removed the intent is sealed and the restart finds no open
/// forget; here that is the `outstanding_forget_intents` assertion after the reopen.
#[test]
fn interrupted_forget_with_sealed_frames_completes_after_restart() {
    let f = Fixture::new();
    for n in 1..=4 {
        put(
            &f.tl,
            n,
            &format!(r#"{{"principal_id":"user:dora","note":"secret-{n}"}}"#),
        );
    }
    let intent = kernel_row(
        &f.tl,
        "principal.forget.intent",
        &intent_payload("user:dora"),
    );
    put(&f.tl, 5, "after");
    put(&f.tl, 6, "after");
    let segment = f.rotate();
    assert_eq!(
        segment.row_count, 4,
        "the frames sealed; the intent did not"
    );
    let db = f.db.clone();
    drop(f.tl); // the Host dies before the scrub

    // Restart: a new adapter finds the open intent and completes the erasure forward.
    let tl = Arc::new(open(&db));
    let open_intents = tl.outstanding_forget_intents(Some("user:dora")).unwrap();
    assert_eq!(open_intents.len(), 1, "the intent survived rotation");
    assert_eq!(open_intents[0].0, intent);
    assert!(all_payloads(&db).iter().any(|p| p.contains("user:dora")));
    tl.scrub_forget_frames("user:dora")
        .expect("forward completion");
    assert!(all_payloads(&db).iter().all(|p| !p.contains("user:dora")));
    let closed = format!(
        "{{\"completed_from_intent\":\"{}\"}}",
        TransparencyLogAdapter::grouped_hex(&intent)
    );
    kernel_row(&tl, "principal.forget", closed.as_bytes());
    assert!(tl.outstanding_forget_intents(None).unwrap().is_empty());
    verify_segment(
        &db.parent()
            .unwrap()
            .join("segments")
            .join("0000000001.manifest.json"),
        &pubkey(),
    )
    .expect("the re-sealed segment verifies");
}

/// Consumers: the forget sweep windows and the forget proof's `through_rowid` cross a rotation.
#[test]
fn forget_sweep_windows_and_proof_cross_a_rotation() {
    let f = Fixture::new();
    // More rows than one scan window (4096): the principal sits on both sides of the boundary.
    for n in 1..=4_300_u32 {
        let body = if n == 100 || n == 4_250 {
            r#"{"principal_id":"user:erin","note":"secret"}"#.to_string()
        } else {
            format!("filler-{n}")
        };
        put(&f.tl, n, &body);
    }
    let segment = f.rotate();
    assert!(segment.last_rowid > 4_000);
    assert!(f.live_rows() < 4_300);
    // A new row lands above every sealed id before the sweep.
    put(
        &f.tl,
        4_301,
        r#"{"principal_id":"user:erin","note":"late"}"#,
    );

    let mut state = ErasureSweep::new(vec!["user:erin".to_string()], None);
    f.tl.sweep_erasure(&mut state).expect("sweep");
    assert!(f.tl.certify_erasure(&mut state).expect("certify"));
    assert_eq!(
        state.principal_frames.len(),
        3,
        "sealed, live and late: {:?}",
        state.principal_frames
    );
    let completion = state.completion.expect("a completion marker");
    let high = f.tl.max_insertion_id().unwrap() as u64;
    assert!(completion.through_rowid > segment.last_rowid as u64);
    assert!(completion.through_rowid <= high);
    let marker =
        f.tl.query_frame_by_id(completion.frame_id)
            .unwrap()
            .expect("marker");
    assert!(
        marker.insertion_id as u64 > completion.through_rowid,
        "the door's re-check holds"
    );
    assert!(all_payloads(&f.db).iter().all(|p| !p.contains("user:erin")));
}
