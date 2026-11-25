#![cfg(all(feature = "tls-sink", not(feature = "siem-fault-inject")))]
#![forbid(unsafe_code)]

//! Story 23-4 — the syslog-over-TLS sink and the durable cursor, against an in-process fake
//! SIEM (a rustls server with a generated CA). AC1, AC2, AC3 (delivery), AC5 (bounds and
//! timeouts) and AC8 (the wire canary). No sleeps: waits are channel receives or gates.

#[path = "support/fake_siem.rs"]
mod fake_siem;

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fake_siem::{
    external_ids, id_set, octet_frames, Ca, Event, FakeSiem, Mode, Options, StubResolver,
};
use maos_audit::AuditFilter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::outbound::OutboundPolicy;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::cursor::{Cursor, CursorStore, CURSOR_FILE};
use maos_siem::export::{
    run_tick, run_tick_cancellable, BatchSink, Frame, Tick, TickFailure, MAX_BATCH_BYTES,
    MAX_BATCH_FRAMES,
};
use maos_siem::tls_sink::{client_config, SetupError, Timeouts, TlsSink};
use tempfile::TempDir;

const HOST: &str = "siem.test";
const CANARY: &str = "AKIAIOSFODNN7EXAMPLE";

// ── fixtures ─────────────────────────────────────────────────────────────────

fn seed(db: &Path, count: usize, tag: &str) {
    let tl = TransparencyLogAdapter::open(db, 1).expect("open log");
    for n in 0..count {
        tl.insert_frame_event(
            FrameKind::CapabilityInvocation,
            1,
            None,
            "test.capability.invoke",
            format!("{tag} row {n}").as_bytes(),
            FrameOrigin::Kernel,
        );
    }
    drop(tl); // quiesce: no WAL left beside the file
}

/// A `VACUUM INTO` copy of the log (the daemon's quiesced export source).
fn snapshot(db: &Path) -> PathBuf {
    static SEQUENCE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = db.with_file_name(format!(
        "snapshot-{}.sqlite",
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    raw(db)
        .execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])
        .expect("snapshot");
    path
}

fn raw(db: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).expect("raw connection")
}

/// A row stamped `timestamp_ns` (a late insert carries an old stamp), with `payload` bytes.
fn raw_insert(db: &Path, timestamp_ns: i64, intent: &str, payload: &[u8]) {
    raw(db)
        .execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, \
             kind, intent, payload_redacted, origin) \
             VALUES (randomblob(16), ?1, 1, 1, 4, ?2, ?3, 0)",
            rusqlite::params![timestamp_ns, intent, payload],
        )
        .expect("raw insert");
}

fn frame_ids(db: &Path) -> Vec<String> {
    maos_audit::query(db, AuditFilter::default())
        .expect("query")
        .into_iter()
        .map(|entry| entry.frame_id_hex)
        .collect()
}

fn dummy_frame() -> Frame {
    Frame {
        insertion_id: 1,
        frame_id_hex: "00".repeat(16),
        ndjson: "{}".to_owned(),
        syslog: "<134>1 - - - - - - x".to_owned(),
    }
}

#[derive(Default)]
struct Journal(Mutex<Vec<(String, serde_json::Value)>>);

impl Journal {
    fn entries(&self) -> Vec<(String, serde_json::Value)> {
        self.0.lock().unwrap().clone()
    }
}

struct Rig {
    _dir: TempDir,
    db: PathBuf,
    ca: Ca,
    fake: FakeSiem,
    resolver: Arc<StubResolver>,
    journal: Journal,
}

impl Rig {
    fn new() -> Self {
        Self::with(Options::default())
    }

    fn with(options: Options) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let db = dir.path().join("audit.sqlite");
        let ca = Ca::new("siem-test-ca");
        let fake = FakeSiem::start(&ca.server(HOST), options);
        Self {
            _dir: dir,
            db,
            ca,
            fake,
            resolver: StubResolver::loopback(HOST),
            journal: Journal::default(),
        }
    }

    fn sink_with(&self, ca_pem: &str, client: Option<&fake_siem::Leaf>) -> TlsSink {
        let config = client_config(
            ca_pem.as_bytes(),
            client.map(|leaf| (leaf.cert_pem.as_bytes(), leaf.key_pem.as_bytes())),
        )
        .expect("client config");
        TlsSink::new(
            HOST,
            self.fake.addr.port(),
            config,
            OutboundPolicy::strict().allow_private(),
            self.resolver.clone(),
            Timeouts::default(),
        )
    }

    fn sink(&self) -> TlsSink {
        self.sink_with(&self.ca.pem, None)
    }

    fn store(&self) -> CursorStore {
        CursorStore::beside(&self.db, &self.sink().identity())
    }

    fn tick(&self, sink: &mut TlsSink, store: &CursorStore) -> Result<Tick, TickFailure> {
        // As the daemon does: export a quiesced snapshot, never the live file.
        let snapshot = snapshot(&self.db);
        let result = run_tick(&snapshot, store, sink, &|intent, payload| {
            self.journal
                .0
                .lock()
                .unwrap()
                .push((intent.to_owned(), payload));
            Ok(())
        });
        let _ = std::fs::remove_file(&snapshot);
        result
    }

    fn cursor_path(&self) -> PathBuf {
        self.db.with_file_name(CURSOR_FILE)
    }

    fn acked_frames(&self) -> Vec<String> {
        match self.fake.next_event() {
            Event::Acked { bytes, .. } => octet_frames(&bytes),
            other => panic!("expected an acknowledged batch, got {other:?}"),
        }
    }
}

// ── AC1 ──────────────────────────────────────────────────────────────────────

#[test]
fn every_row_arrives_octet_counted_and_equals_the_file_sink_line() {
    let rig = Rig::new();
    seed(&rig.db, 3, "ac1a");
    let (mut sink, store) = (rig.sink(), rig.store());
    let tick = rig.tick(&mut sink, &store).expect("tick");
    assert_eq!(tick.delivered, 3);

    let frames = rig.acked_frames();
    let mut wire = frames.clone();
    wire.sort();
    let mut file_lines: Vec<String> = maos_siem::export_from_tl(&rig.db, AuditFilter::default())
        .expect("export_from_tl")
        .into_iter()
        .map(|record| record.rfc5424)
        .collect();
    file_lines.sort();
    assert_eq!(
        wire, file_lines,
        "each SYSLOG-MSG must equal the file sink's line for the same row"
    );
    assert_eq!(
        id_set(&external_ids(&frames)),
        id_set(&frame_ids(&rig.db)),
        "externalId is the frame id"
    );
}

#[test]
fn a_plaintext_listener_sees_a_client_hello_and_no_frame() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("plaintext listener");
    let port = listener.local_addr().unwrap().port();
    let (sender, received) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        use std::io::Read;
        let (mut stream, _) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(fake_siem::OUTER)).unwrap();
        let mut buffer = [0_u8; 4096];
        let n = stream.read(&mut buffer).unwrap_or(0);
        let _ = sender.send(buffer[..n].to_vec());
    });
    let rig = Rig::new();
    let mut sink = TlsSink::new(
        HOST,
        port,
        client_config(rig.ca.pem.as_bytes(), None).unwrap(),
        OutboundPolicy::strict().allow_private(),
        rig.resolver.clone(),
        Timeouts::default(),
    );
    let failure = sink.deliver(&[dummy_frame()]).expect_err("plaintext peer");
    assert_eq!(failure.code, "siem_sink_io");
    let bytes = received.recv_timeout(fake_siem::OUTER).expect("captured");
    thread.join().unwrap();
    assert_eq!(
        bytes.first(),
        Some(&0x16),
        "the first byte is a TLS handshake record"
    );
    assert!(
        !bytes
            .windows(4)
            .any(|window| window == b"CEF:" || window == b"<134"),
        "nothing is ever written in clear"
    );
}

#[test]
fn a_server_certificate_from_another_ca_is_refused_and_the_bundle_is_the_only_root() {
    let rig = Rig::new();
    let other = Ca::new("another-ca");
    let mut sink = rig.sink_with(&other.pem, None);
    let failure = sink.deliver(&[dummy_frame()]).expect_err("foreign CA");
    assert_eq!(failure.code, "siem_sink_tls_refused");
    assert!(
        !matches!(rig.fake.next_event(), Event::Acked { .. }),
        "the server never read a batch"
    );
    // An empty or unusable bundle builds no client at all: every server is refused.
    assert_eq!(client_config(b"", None).err(), Some(SetupError::CaInvalid));
    assert_eq!(
        client_config(b"not a pem bundle", None).err(),
        Some(SetupError::CaInvalid)
    );
}

#[test]
fn a_client_certificate_is_presented_when_configured() {
    let client_ca = Ca::new("client-ca");
    let rig = Rig::with(Options {
        require_client_ca: Some(client_ca.der()),
        ..Options::default()
    });
    let client = client_ca.client("maos-host");

    let mut with_cert = rig.sink_with(&rig.ca.pem, Some(&client));
    with_cert
        .deliver(&[dummy_frame()])
        .expect("a server requiring client auth accepts the client certificate");
    assert!(matches!(rig.fake.next_event(), Event::Acked { .. }));

    let mut without = rig.sink();
    without
        .deliver(&[dummy_frame()])
        .expect_err("without a client certificate the server refuses");
    assert!(!matches!(rig.fake.next_event(), Event::Acked { .. }));

    // A key that does not belong to the certificate is a setup refusal, never a connection.
    let stranger = client_ca.client("someone-else");
    assert_eq!(
        client_config(
            rig.ca.pem.as_bytes(),
            Some((client.cert_pem.as_bytes(), stranger.key_pem.as_bytes()))
        )
        .err(),
        Some(SetupError::ClientKeyInvalid)
    );
    assert_eq!(
        client_config(
            rig.ca.pem.as_bytes(),
            Some((client.cert_pem.as_bytes(), b"not a key"))
        )
        .err(),
        Some(SetupError::ClientKeyInvalid)
    );
}

fn sink_for(rig: &Rig, policy: OutboundPolicy, resolver: Arc<StubResolver>) -> TlsSink {
    TlsSink::new(
        HOST,
        rig.fake.addr.port(),
        client_config(rig.ca.pem.as_bytes(), None).unwrap(),
        policy,
        resolver,
        Timeouts::default(),
    )
}

#[test]
fn the_outbound_policy_gates_the_host_class_and_the_connection_binds_to_what_it_vetted() {
    let rig = Rig::new();
    let answer = |ip: [u8; 4]| StubResolver::new(HOST, vec![vec![IpAddr::from(ip)]]);
    let refused = |policy, ip| {
        sink_for(&rig, policy, answer(ip))
            .deliver(&[dummy_frame()])
            .expect_err("refused")
            .code
    };
    assert_eq!(
        refused(OutboundPolicy::strict(), [127, 0, 0, 1]),
        "outbound_loopback_refused"
    );
    assert_eq!(
        refused(OutboundPolicy::strict(), [10, 1, 2, 3]),
        "outbound_private_refused"
    );
    assert_eq!(
        refused(OutboundPolicy::strict(), [192, 168, 1, 1]),
        "outbound_private_refused"
    );
    assert_eq!(
        refused(
            OutboundPolicy::strict().allow_private(),
            [169, 254, 169, 254]
        ),
        "outbound_link_local_refused",
        "link-local is refused even when private ranges are admitted"
    );
    assert!(
        rig.fake.drain().is_empty(),
        "a refused host class never reaches the network"
    );

    // MAOS_SIEM_ALLOW_PRIVATE admits loopback: one resolution per connection, and the
    // connect goes to the address that resolution returned.
    let resolver = StubResolver::new(
        HOST,
        vec![
            vec![IpAddr::from([127, 0, 0, 1])],
            vec![IpAddr::from([127, 0, 0, 2])],
        ],
    );
    let mut sink = sink_for(
        &rig,
        OutboundPolicy::strict().allow_private(),
        resolver.clone(),
    );
    sink.deliver(&[dummy_frame()]).expect("first connection");
    assert_eq!(resolver.calls(), 1, "one resolution per connection");
    assert!(matches!(rig.fake.next_event(), Event::Acked { .. }));
    // The next connection is vetted again: the new answer (nothing listens there) is used,
    // never the earlier one.
    let failure = sink.deliver(&[dummy_frame()]).expect_err("second answer");
    assert_eq!(failure.code, "siem_sink_connect_failed");
    assert_eq!(resolver.calls(), 2);
}

#[test]
fn tls_13_and_tls_12_ecdhe_aes_128_gcm_negotiate() {
    let rig = Rig::new();
    rig.sink()
        .deliver(&[dummy_frame()])
        .expect("default server");
    match rig.fake.next_event() {
        Event::Acked { version, .. } => assert_eq!(version, "TLSv1_3"),
        other => panic!("{other:?}"),
    }

    let legacy = Rig::with(Options {
        tls12_ecdhe_gcm_only: true,
        ..Options::default()
    });
    legacy
        .sink()
        .deliver(&[dummy_frame()])
        .expect("TLS 1.2 server");
    match legacy.fake.next_event() {
        Event::Acked { version, suite, .. } => {
            assert_eq!(version, "TLSv1_2");
            assert_eq!(suite, "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256");
        }
        other => panic!("{other:?}"),
    }
}

// ── AC2 / AC3 ────────────────────────────────────────────────────────────────

#[test]
fn a_restart_resends_nothing_and_delivers_exactly_the_rows_inserted_since() {
    let rig = Rig::new();
    seed(&rig.db, 4, "before");
    {
        let (mut sink, store) = (rig.sink(), rig.store());
        assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 4);
        assert_eq!(rig.acked_frames().len(), 4);
    } // the exporter is dropped: a restart
    let before = id_set(&frame_ids(&rig.db));
    seed(&rig.db, 3, "after");
    let new_ids: Vec<String> = frame_ids(&rig.db)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(new_ids.len(), 3);

    let (mut sink, store) = (rig.sink(), rig.store());
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 3);
    assert_eq!(id_set(&external_ids(&rig.acked_frames())), id_set(&new_ids));
    // Nothing left: the next tick opens no connection at all.
    assert_eq!(rig.tick(&mut sink, &store).unwrap().batches, 0);
    assert!(rig.fake.drain().is_empty());
}

#[test]
fn a_collector_that_closes_without_close_notify_does_not_move_the_cursor() {
    let rig = Rig::new();
    seed(&rig.db, 3, "older");
    let (mut sink, store) = (rig.sink(), rig.store());
    rig.tick(&mut sink, &store)
        .expect("older rows acknowledged");
    let older = rig.acked_frames();
    assert_eq!(older.len(), 3);

    seed(&rig.db, 2, "in-flight");
    rig.fake.set_mode(Mode::CloseWithoutNotify);
    let before = std::fs::read(rig.cursor_path()).expect("cursor file");
    let failure = rig.tick(&mut sink, &store).expect_err("no acknowledgement");
    assert_eq!(failure.code(), "siem_sink_no_close_notify");
    assert_eq!(
        failure.buffered(),
        Some(2),
        "the rows after the cursor are reported"
    );
    assert!(matches!(rig.fake.next_event(), Event::Unacked { .. }));
    assert_eq!(
        std::fs::read(rig.cursor_path()).unwrap(),
        before,
        "an unacknowledged batch leaves the cursor file as it was"
    );

    rig.fake.set_mode(Mode::Ack);
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 2);
    let resent = external_ids(&rig.acked_frames());
    assert_eq!(
        resent.len(),
        2,
        "exactly the batch in flight, nothing older"
    );
    assert!(resent
        .iter()
        .all(|id| !id_set(&external_ids(&older)).contains(id)));
}

#[test]
fn a_failure_between_the_ack_and_the_cursor_write_resends_exactly_that_batch() {
    let rig = Rig::new();
    seed(&rig.db, 3, "batch");
    let (mut sink, store) = (rig.sink(), rig.store());
    // The temporary file's name is taken by a directory: the cursor cannot be written.
    let tmp = rig.cursor_path().with_extension("json.tmp");
    std::fs::create_dir(&tmp).expect("block the temporary file");
    let failure = rig.tick(&mut sink, &store).expect_err("cursor write fails");
    assert_eq!(failure.code(), "siem_cursor_io");
    let first = external_ids(&rig.acked_frames());
    assert_eq!(first.len(), 3, "the collector did acknowledge the batch");
    assert!(!rig.cursor_path().exists(), "no cursor was persisted");

    std::fs::remove_dir(&tmp).unwrap();
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 3);
    let second = external_ids(&rig.acked_frames());
    assert_eq!(
        id_set(&first),
        id_set(&second),
        "the duplicates carry the same externalId"
    );
}

#[test]
fn a_stray_temporary_file_is_ignored_at_start_and_the_cursor_is_written_atomically() {
    let rig = Rig::new();
    seed(&rig.db, 2, "tmp");
    std::fs::write(
        rig.cursor_path().with_extension("json.tmp"),
        b"{ half a cur",
    )
    .unwrap();
    let (mut sink, store) = (rig.sink(), rig.store());
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 2);
    assert!(
        rig.journal.entries().is_empty(),
        "a stray tmp is not a reset"
    );
    let text = std::fs::read_to_string(rig.cursor_path()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["version"], "maos.siem-cursor.v1");
    assert!(
        value["sink"].as_str().is_some_and(|hash| hash.len() == 64),
        "the sink identity is stored hashed: {text}"
    );
    assert!(!text.contains(HOST), "the endpoint is not stored in clear");
}

#[test]
fn a_row_inserted_late_with_an_older_stamp_is_delivered() {
    let rig = Rig::new();
    seed(&rig.db, 2, "ordered");
    let (mut sink, store) = (rig.sink(), rig.store());
    rig.tick(&mut sink, &store).unwrap();
    let _ = rig.acked_frames();

    raw_insert(&rig.db, 1, "late.row", b"stamped long ago");
    let tick = rig.tick(&mut sink, &store).unwrap();
    assert_eq!(
        tick.delivered, 1,
        "a timestamp watermark would skip this row"
    );
    let frames = rig.acked_frames();
    assert!(frames[0].contains("late.row"), "{frames:?}");
}

#[test]
fn a_cursor_that_does_not_match_the_log_takes_the_reset_path() {
    let rig = Rig::new();
    seed(&rig.db, 3, "anchor");
    let (mut sink, store) = (rig.sink(), rig.store());
    rig.tick(&mut sink, &store).unwrap();
    assert_eq!(rig.acked_frames().len(), 3);

    // A restore: the row at the cursor's insertion id is another row.
    raw(&rig.db)
        .execute(
            "UPDATE transparency_log SET frame_id = randomblob(16) \
             WHERE rowid = (SELECT max(rowid) FROM transparency_log)",
            [],
        )
        .unwrap();
    let tick = rig.tick(&mut sink, &store).unwrap();
    assert_eq!(tick.delivered, 3, "every retained row is re-sent");
    assert_eq!(rig.acked_frames().len(), 3);
    let journal = rig.journal.entries();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].0, "siem.cursor.reset");
    assert_eq!(journal[0].1["reason"], "siem_cursor_mismatch");
    assert!(
        tick.notes
            .iter()
            .any(|note| note.starts_with("siem_cursor_mismatch")),
        "{:?}",
        tick.notes
    );
    let aside = std::fs::read_dir(rig.db.parent().unwrap())
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("siem-cursor.json.corrupt-")
        })
        .count();
    assert_eq!(aside, 1, "the bad cursor is moved aside, not deleted");
}

#[test]
fn an_unreadable_wrong_sink_or_future_cursor_resets_and_resends_everything() {
    for (case, reason) in [
        ("junk", "siem_cursor_unreadable"),
        ("sink", "siem_cursor_sink_changed"),
        ("ahead", "siem_cursor_ahead_of_log"),
    ] {
        let rig = Rig::new();
        seed(&rig.db, 2, case);
        let (mut sink, store) = (rig.sink(), rig.store());
        match case {
            "junk" => std::fs::write(rig.cursor_path(), b"\0not json").unwrap(),
            "sink" => CursorStore::at(rig.cursor_path(), "tls://another.collector:6514")
                .save(&Cursor {
                    insertion_id: 1,
                    frame_id: None,
                })
                .unwrap(),
            _ => store
                .save(&Cursor {
                    insertion_id: 9_999,
                    frame_id: None,
                })
                .unwrap(),
        }
        let tick = rig.tick(&mut sink, &store).unwrap();
        assert_eq!(
            tick.delivered, 2,
            "{case}: a reset costs duplicates, never a hole"
        );
        let journal = rig.journal.entries();
        assert_eq!(journal.len(), 1, "{case}");
        assert_eq!(journal[0].1["reason"], reason, "{case}");
    }
}

// ── AC5: bounds and timeouts ─────────────────────────────────────────────────

#[test]
fn a_backlog_goes_out_in_bounded_batches_within_one_tick() {
    let rig = Rig::new();
    seed(&rig.db, 1, "schema");
    {
        let mut connection = raw(&rig.db);
        let transaction = connection.transaction().unwrap();
        for n in 0..1_099 {
            transaction
                .execute(
                    "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, \
                     kind, intent, payload_redacted, origin) \
                     VALUES (randomblob(16), ?1, 1, 1, 4, 'bulk.row', X'', 0)",
                    [n],
                )
                .unwrap();
        }
        transaction.commit().unwrap();
    }
    let (mut sink, store) = (rig.sink(), rig.store());
    let tick = rig.tick(&mut sink, &store).unwrap();
    assert_eq!((tick.delivered, tick.batches), (1_100, 3));
    let sizes: Vec<usize> = (0..3).map(|_| rig.acked_frames().len()).collect();
    assert_eq!(
        sizes,
        vec![
            MAX_BATCH_FRAMES,
            MAX_BATCH_FRAMES,
            1_100 - 2 * MAX_BATCH_FRAMES
        ]
    );

    // The byte bound: ten 150 kB rows cannot share one 1 MiB batch.
    let big = vec![b'a'; 150_000];
    for _ in 0..10 {
        raw_insert(&rig.db, 5, "big.row", &big);
    }
    let tick = rig.tick(&mut sink, &store).unwrap();
    assert_eq!(tick.delivered, 10);
    assert!(tick.batches >= 2);
    for _ in 0..tick.batches {
        let frames = rig.acked_frames();
        let bytes: usize = frames.iter().map(String::len).sum();
        assert!(
            bytes <= MAX_BATCH_BYTES,
            "a batch holds at most 1 MiB of frames"
        );
    }
}

#[test]
fn every_wait_is_bounded_and_a_held_collector_never_moves_the_cursor() {
    let rig = Rig::with(Options {
        hold_ack: true,
        ..Options::default()
    });
    seed(&rig.db, 2, "held");
    let short = Timeouts {
        connect: Duration::from_millis(300),
        handshake: Duration::from_millis(300),
        write: Duration::from_millis(300),
        close: Duration::from_millis(300),
    };
    let make = |rig: &Rig| {
        TlsSink::new(
            HOST,
            rig.fake.addr.port(),
            client_config(rig.ca.pem.as_bytes(), None).unwrap(),
            OutboundPolicy::strict().allow_private(),
            rig.resolver.clone(),
            short,
        )
    };
    let store = rig.store();

    // The handshake never completes: the connection is accepted and then ignored.
    rig.fake.set_mode(Mode::HoldHandshake);
    let failure = rig
        .tick(&mut make(&rig), &store)
        .expect_err("handshake held");
    assert_eq!(failure.code(), "siem_sink_timeout");
    assert_eq!(failure.buffered(), Some(2));
    assert!(matches!(rig.fake.next_event(), Event::Held));
    rig.fake.handshake_gate.open();
    assert!(matches!(rig.fake.next_event(), Event::Unacked { .. }));

    // The collector reads the batch but holds its close_notify.
    rig.fake.set_mode(Mode::Ack);
    let failure = rig.tick(&mut make(&rig), &store).expect_err("ack held");
    assert_eq!(failure.code(), "siem_sink_no_close_notify");
    assert!(!rig.cursor_path().exists(), "no acknowledgement, no cursor");
    rig.fake.ack_gate.open();
    let stale = external_ids(&rig.acked_frames());

    // Released: the same two rows, now acknowledged.
    assert_eq!(rig.tick(&mut make(&rig), &store).unwrap().delivered, 2);
    assert_eq!(id_set(&external_ids(&rig.acked_frames())), id_set(&stale));
}

// ── AC8: redaction on the wire ───────────────────────────────────────────────

#[test]
fn the_secret_canary_never_reaches_the_collector() {
    let rig = Rig::new();
    let tl = TransparencyLogAdapter::open(&rig.db, 1).unwrap();
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1,
        None,
        "test.capability.invoke",
        format!("invocation notes key={CANARY} endpoint=prod").as_bytes(),
        FrameOrigin::Kernel,
    );
    drop(tl);
    let (mut sink, store) = (rig.sink(), rig.store());
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 1);
    let Event::Acked { bytes, .. } = rig.fake.next_event() else {
        panic!("no batch");
    };
    let wire = String::from_utf8_lossy(&bytes);
    assert!(
        !wire.contains(CANARY),
        "the canary reached the wire: {wire}"
    );
    assert!(
        wire.contains("REDACTED"),
        "the redaction marker is on the wire: {wire}"
    );
}

// ── review round 1 ───────────────────────────────────────────────────────────

/// A collector that closes first (its close_notify and FIN arrive with the last handshake
/// flight, before it read the batch) is NOT an acknowledgement: counting that close_notify
/// would move the cursor past rows it never saw. TLS 1.2 is the deterministic case (the
/// server's `close_notify` shares a flight with its Finished); the TLS 1.3 ordering depends on
/// when the server's post-handshake record lands and is not asserted (see the README).
#[test]
fn a_collector_that_closes_first_is_not_an_acknowledgement() {
    let rig = Rig::with(Options {
        tls12_ecdhe_gcm_only: true,
        ..Options::default()
    });
    rig.fake.set_mode(Mode::CloseFirst);
    seed(&rig.db, 3, "closed-first");
    let (mut sink, store) = (rig.sink(), rig.store());
    let failure = rig
        .tick(&mut sink, &store)
        .expect_err("a close that came first");
    assert_eq!(failure.code(), "siem_sink_no_close_notify");
    assert!(matches!(rig.fake.next_event(), Event::Unacked { .. }));
    assert!(!rig.cursor_path().exists(), "the cursor did not move");

    // The same collector answering properly is acknowledged and the same rows go.
    rig.fake.set_mode(Mode::Ack);
    assert_eq!(rig.tick(&mut sink, &store).unwrap().delivered, 3);
}

/// Every handshake step re-checks the deadline: a peer that hands over one byte and then
/// goes quiet (a trickle held by a gate, not a sleep) ends the phase with a timeout.
#[test]
fn a_trickling_peer_cannot_stretch_the_handshake() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("trickle listener");
    let port = listener.local_addr().unwrap().port();
    let (release, gate) = std::sync::mpsc::channel::<()>();
    let thread = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().expect("accept");
        let mut hello = [0_u8; 4096];
        let _ = stream.read(&mut hello);
        let _ = stream.write_all(&[0x16]); // one byte of a handshake record header
        let _ = gate.recv_timeout(fake_siem::OUTER); // then nothing until released
    });
    let rig = Rig::new();
    let mut sink = TlsSink::new(
        HOST,
        port,
        client_config(rig.ca.pem.as_bytes(), None).unwrap(),
        OutboundPolicy::strict().allow_private(),
        rig.resolver.clone(),
        Timeouts {
            handshake: Duration::from_millis(400),
            ..Timeouts::default()
        },
    );
    let failure = sink
        .deliver(&[dummy_frame()])
        .expect_err("trickled handshake");
    assert_eq!(failure.code, "siem_sink_timeout");
    release.send(()).unwrap();
    thread.join().unwrap();
}

/// Shutdown reaches a blocking export: with the collector holding the handshake, raising the
/// flag ends the tick at its next read slice, and nothing is left behind (the tick can run
/// again at once, the cursor never moved).
#[test]
fn a_cancel_stops_an_export_stuck_in_a_held_handshake() {
    let rig = Rig::new();
    seed(&rig.db, 2, "cancel");
    rig.fake.set_mode(Mode::HoldHandshake);
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut sink = rig.sink();
    sink.set_cancel(Arc::clone(&cancel));
    let store = rig.store();
    let snapshot = snapshot(&rig.db);
    let flag = Arc::clone(&cancel);
    let worker = std::thread::spawn(move || {
        run_tick_cancellable(&snapshot, &store, &mut sink, &|_, _| Ok(()), &flag)
    });
    assert!(matches!(rig.fake.next_event(), Event::Held));
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    let failure = worker.join().unwrap().expect_err("cancelled");
    assert_eq!(failure.code(), "siem_export_cancelled");
    rig.fake.handshake_gate.open();
    assert!(matches!(rig.fake.next_event(), Event::Unacked { .. }));
    assert!(!rig.cursor_path().exists());
}

/// Negative control for the CA-only witness: a client whose root store is WIDENED with the
/// foreign collector's CA accepts that collector, which is exactly what
/// `a_server_certificate_from_another_ca_is_refused...` must be able to see. The shipped
/// `client_config` built from the operator's bundle alone still refuses it.
#[test]
fn a_widened_root_store_is_detected_by_the_ca_only_witness() {
    let rig = Rig::new();
    let foreign = Ca::new("foreign-collector-ca");
    let foreign_fake = FakeSiem::start(&foreign.server(HOST), Options::default());
    let sink_for = |config| {
        TlsSink::new(
            HOST,
            foreign_fake.addr.port(),
            config,
            OutboundPolicy::strict().allow_private(),
            rig.resolver.clone(),
            Timeouts::default(),
        )
    };

    let mut roots = rustls::RootCertStore::empty();
    roots.add(rig.ca.der()).unwrap();
    roots.add(foreign.der()).unwrap();
    let widened = Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth(),
    );
    sink_for(widened)
        .deliver(&[dummy_frame()])
        .expect("a widened root store accepts the foreign collector (the control)");
    assert!(matches!(foreign_fake.next_event(), Event::Acked { .. }));

    let only = client_config(rig.ca.pem.as_bytes(), None).unwrap();
    let failure = sink_for(only)
        .deliver(&[dummy_frame()])
        .expect_err("the CA-only config refuses it");
    assert_eq!(failure.code, "siem_sink_tls_refused");
}

/// A tick that fails does not leave a snapshot or a lock: the next tick runs at once.
#[test]
fn the_gap_note_survives_a_sink_failure_in_the_same_tick() {
    let rig = Rig::new();
    seed(&rig.db, 3, "notes");
    let (mut sink, store) = (rig.sink(), rig.store());
    // An unreadable cursor is reset (journaled, noted), then the sink fails: the typed line
    // of the reset must travel with the failure.
    std::fs::write(rig.cursor_path(), b"\0junk").unwrap();
    rig.fake.set_mode(Mode::CloseWithoutNotify);
    let failure = rig
        .tick(&mut sink, &store)
        .expect_err("sink fails after the reset");
    assert_eq!(failure.code(), "siem_sink_no_close_notify");
    assert!(
        failure
            .tick
            .notes
            .iter()
            .any(|note| note.starts_with("siem_cursor_unreadable")),
        "{:?}",
        failure.tick.notes
    );
    assert_eq!(rig.journal.entries().len(), 1);
}

// ── review round 3 ───────────────────────────────────────────────────────────

use std::io::{Cursor as IoCursor, Read, Write};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use maos_siem::tls_sink::{drain_pending, drive_handshake, CloseStage, StepIo, MAX_DRAIN_READS};
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, ServerConnection};

/// A client and a server joined in memory, handshake complete and every flight delivered.
fn memory_pair(options: &Options) -> (ClientConnection, ServerConnection) {
    let ca = Ca::new("memory-ca");
    let leaf = ca.server(HOST);
    let mut client = ClientConnection::new(
        client_config(ca.pem.as_bytes(), None).unwrap(),
        ServerName::try_from(HOST).unwrap(),
    )
    .unwrap();
    let mut server = ServerConnection::new(FakeSiem::server_config(&leaf, options)).unwrap();
    loop {
        let mut moved = false;
        let mut bytes = Vec::new();
        while client.wants_write() {
            client.write_tls(&mut bytes).unwrap();
        }
        if !bytes.is_empty() {
            moved = true;
            let mut input = IoCursor::new(bytes);
            while (input.position() as usize) < input.get_ref().len() {
                server.read_tls(&mut input).unwrap();
                server.process_new_packets().unwrap();
            }
        }
        let mut bytes = Vec::new();
        while server.wants_write() {
            server.write_tls(&mut bytes).unwrap();
        }
        if !bytes.is_empty() {
            moved = true;
            let mut input = IoCursor::new(bytes);
            while (input.position() as usize) < input.get_ref().len() {
                client.read_tls(&mut input).unwrap();
                client.process_new_packets().unwrap();
            }
        }
        if !moved {
            break;
        }
    }
    assert!(!client.is_handshaking() && !server.is_handshaking());
    (client, server)
}

/// What the server puts on the wire for one application record.
fn data_record(server: &mut ServerConnection) -> Vec<u8> {
    server.writer().write_all(b"x").unwrap();
    let mut out = Vec::new();
    while server.wants_write() {
        server.write_tls(&mut out).unwrap();
    }
    out
}

fn alert_record(server: &mut ServerConnection) -> Vec<u8> {
    server.send_close_notify();
    let mut out = Vec::new();
    while server.wants_write() {
        server.write_tls(&mut out).unwrap();
    }
    out
}

/// A scripted non-blocking source: one queued TLS record per read, then `WouldBlock`.
struct Script(std::collections::VecDeque<Vec<u8>>);

impl Read for Script {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.pop_front() {
            Some(record) => {
                buf[..record.len()].copy_from_slice(&record);
                Ok(record.len())
            }
            None => Err(std::io::ErrorKind::WouldBlock.into()),
        }
    }
}

fn script(server: &mut ServerConnection, data: usize, alert: bool) -> Script {
    let mut records = std::collections::VecDeque::new();
    for _ in 0..data {
        records.push_back(data_record(server));
    }
    if alert {
        records.push_back(alert_record(server));
    }
    Script(records)
}

fn far() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

/// R3-4: the drain boundary, with a scripted record source (no flood that rustls could refuse
/// first). 63 records then dry => the batch may be closed; 64 records => exhausted, whatever
/// is queued behind them (here: the collector's alert) => NOT acknowledgeable.
#[test]
fn the_drain_proceeds_at_63_records_and_refuses_at_exhaustion_with_an_alert_queued_behind() {
    assert_eq!(MAX_DRAIN_READS, 64);
    for tls12 in [false, true] {
        let options = Options {
            tls12_ecdhe_gcm_only: tls12,
            ..Options::default()
        };
        let (mut client, mut server) = memory_pair(&options);
        let mut source = script(&mut server, 63, false);
        drain_pending(&mut client, &mut source, far(), &Instant::now, &|| false).unwrap_or_else(
            |failure| panic!("63 records then dry proceeds (tls12={tls12}): {failure}"),
        );

        let (mut client, mut server) = memory_pair(&options);
        let mut source = script(&mut server, 64, true);
        let failure = drain_pending(&mut client, &mut source, far(), &Instant::now, &|| false)
            .expect_err("64 records then an alert behind them");
        assert_eq!(failure.code, "siem_sink_drain_exhausted", "tls12={tls12}");
        assert_eq!(
            source.0.len(),
            1,
            "the alert was never read: exhaustion is not success"
        );

        let (mut client, mut server) = memory_pair(&options);
        let mut source = script(&mut server, 64, false);
        let failure = drain_pending(&mut client, &mut source, far(), &Instant::now, &|| false)
            .expect_err("64 records is exhaustion even when the source then runs dry");
        assert_eq!(failure.code, "siem_sink_drain_exhausted");

        let (mut client, mut server) = memory_pair(&options);
        let mut source = script(&mut server, 3, true);
        let failure = drain_pending(&mut client, &mut source, far(), &Instant::now, &|| false)
            .expect_err("an alert within the cap is a close");
        assert_eq!(failure.code, "siem_sink_no_close_notify");
    }
}

/// R3-6: the drain checks the cancel flag and the deadline on EVERY read: a flag raised
/// mid-drain and a clock that passes the deadline mid-drain both stop it before the cap.
#[test]
fn the_drain_checks_cancellation_and_the_deadline_at_every_read() {
    let (mut client, mut server) = memory_pair(&Options::default());
    let mut source = script(&mut server, 40, false);
    let calls = AtomicUsize::new(0);
    let failure = drain_pending(&mut client, &mut source, far(), &Instant::now, &|| {
        calls.fetch_add(1, Ordering::SeqCst) >= 7
    })
    .expect_err("cancelled mid-drain");
    assert_eq!(failure.code, "siem_export_cancelled");
    assert_eq!(
        source.0.len(),
        40 - 7,
        "seven reads happened, then the flag stopped it"
    );

    let (mut client, mut server) = memory_pair(&Options::default());
    let mut source = script(&mut server, 100, false);
    let base = Instant::now();
    let ticks = AtomicU64::new(0);
    let clock = || base + Duration::from_secs(ticks.fetch_add(1, Ordering::SeqCst));
    let failure = drain_pending(
        &mut client,
        &mut source,
        base + Duration::from_secs(5),
        &clock,
        &|| false,
    )
    .expect_err("the deadline passes mid-drain");
    assert_eq!(failure.code, "siem_sink_timeout");
    assert!(
        source.0.len() > 100 - MAX_DRAIN_READS,
        "stopped before the cap"
    );
}

/// A client stream in memory: the server answers in lockstep, `chunk` bytes per read, every
/// read succeeds (none ever times out), and the INJECTED clock advances `step` per read.
struct Lockstep {
    server: ServerConnection,
    outbox: std::collections::VecDeque<u8>,
    chunk: usize,
    clock_ms: Arc<AtomicU64>,
    step_ms: u64,
    reads: usize,
    timeouts: usize,
}

impl Read for Lockstep {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.clock_ms.fetch_add(self.step_ms, Ordering::SeqCst);
        if self.outbox.is_empty() {
            self.timeouts += 1;
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        self.reads += 1;
        let take = self.chunk.min(buf.len()).min(self.outbox.len());
        for slot in &mut buf[..take] {
            *slot = self.outbox.pop_front().unwrap();
        }
        Ok(take)
    }
}

impl Write for Lockstep {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut input = IoCursor::new(buf);
        while (input.position() as usize) < buf.len() {
            self.server.read_tls(&mut input)?;
            self.server
                .process_new_packets()
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        }
        let mut out = Vec::new();
        while self.server.wants_write() {
            self.server.write_tls(&mut out)?;
        }
        self.outbox.extend(out);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl StepIo for Lockstep {
    fn bound(&mut self, _left: Duration) -> std::io::Result<()> {
        Ok(())
    }
}

fn lockstep() -> (ClientConnection, Lockstep, Arc<AtomicU64>) {
    let ca = Ca::new("lockstep-ca");
    let leaf = ca.server(HOST);
    let client = ClientConnection::new(
        client_config(ca.pem.as_bytes(), None).unwrap(),
        ServerName::try_from(HOST).unwrap(),
    )
    .unwrap();
    let clock = Arc::new(AtomicU64::new(0));
    let io = Lockstep {
        server: ServerConnection::new(FakeSiem::server_config(&leaf, &Options::default())).unwrap(),
        outbox: Default::default(),
        chunk: 1,
        clock_ms: Arc::clone(&clock),
        step_ms: 10,
        reads: 0,
        timeouts: 0,
    };
    (client, io, clock)
}

/// R3-6 (replaces the relay witness): N steps of progress, each read succeeding within its
/// own timeout, whose total exceeds the phase deadline on the INJECTED clock, end in
/// `siem_sink_timeout`. The old whole-handshake `complete_io` loop never consults the clock
/// and lets the same peer finish.
#[test]
fn steady_progress_beyond_the_handshake_deadline_is_a_timeout_on_an_injected_clock() {
    let base = Instant::now();
    let (mut client, mut io, clock) = lockstep();
    let now = {
        let clock = Arc::clone(&clock);
        move || base + Duration::from_millis(clock.load(Ordering::SeqCst))
    };
    let failure = drive_handshake(
        &mut client,
        &mut io,
        Duration::from_millis(100),
        &now,
        &|| false,
    )
    .expect_err("progress beyond the phase deadline is still a timeout");
    assert_eq!(failure.code, "siem_sink_timeout");
    assert!(
        io.reads >= 5,
        "the peer was making progress: {} reads",
        io.reads
    );
    assert_eq!(io.timeouts, 0, "no single read ever timed out");

    // The same peer inside a generous budget completes: the witness is not trivially red.
    let (mut client, mut io, clock) = lockstep();
    let now = {
        let clock = Arc::clone(&clock);
        move || base + Duration::from_millis(clock.load(Ordering::SeqCst))
    };
    drive_handshake(
        &mut client,
        &mut io,
        Duration::from_secs(3600),
        &now,
        &|| false,
    )
    .expect("within the budget the handshake completes");
    assert_eq!(io.timeouts, 0);
}

/// R3-7: the collector's alert is on the wire BEFORE our pre-close look (the fake proves it
/// has written it; the client proves it can read it), so it is refused, in TLS 1.3 and 1.2.
#[test]
fn an_alert_readable_before_our_pre_close_look_is_refused_in_tls_13_and_12() {
    for tls12 in [false, true] {
        let rig = Rig::with(Options {
            tls12_ecdhe_gcm_only: tls12,
            ..Options::default()
        });
        rig.fake.set_mode(Mode::CloseWithoutReading);
        let sent = rig.fake.alert_sent.clone();
        let mut sink = rig.sink().with_close_hook(Arc::new(move |tcp, stage| {
            if stage == CloseStage::BeforeEarlyCheck {
                assert!(sent.wait_open(), "the collector never sent its alert");
                // Readable now: peek blocks (one read slice) until bytes are queued.
                let _ = tcp.peek(&mut [0_u8; 1]);
            }
        }));
        let failure = sink
            .deliver(&[dummy_frame()])
            .expect_err("a close that is already readable is not an acknowledgement");
        assert_eq!(failure.code, "siem_sink_no_close_notify", "tls12={tls12}");
        match rig.fake.next_event() {
            Event::Unacked { bytes } => assert!(bytes.is_empty(), "the collector read nothing"),
            other => panic!("{other:?}"),
        }
    }
}

/// R3-7, the DOCUMENTED RESIDUAL, precisely: a collector that reads NOTHING and whose alert
/// becomes readable only AFTER our pre-close look (it crosses our close) is accepted as the
/// acknowledgement, in TLS 1.3 and 1.2. What is refused: an alert readable before the look
/// (test above). What is accepted: one that arrives later, whatever the collector did with
/// the batch (RELEASE-HOLDS 23-4.i, D-23-4-16).
#[test]
fn an_alert_that_arrives_after_our_pre_close_look_is_accepted_though_nothing_was_read() {
    for tls12 in [false, true] {
        let rig = Rig::with(Options {
            tls12_ecdhe_gcm_only: tls12,
            ..Options::default()
        });
        rig.fake.set_mode(Mode::CloseWithoutReading);
        rig.fake.alert_hold.close();
        let (hold, sent) = (rig.fake.alert_hold.clone(), rig.fake.alert_sent.clone());
        let mut sink = rig.sink().with_close_hook(Arc::new(move |tcp, stage| {
            if stage == CloseStage::AfterEarlyCheck {
                // The look found nothing; only now does the collector's alert leave.
                hold.open();
                assert!(sent.wait_open(), "the collector never sent its alert");
                let _ = tcp.peek(&mut [0_u8; 1]);
            }
        }));
        sink.deliver(&[dummy_frame()]).unwrap_or_else(|failure| {
            panic!("the crossed alert is accepted (tls12={tls12}): {failure}")
        });
        match rig.fake.next_event() {
            Event::Unacked { bytes } => assert!(bytes.is_empty(), "the collector read nothing"),
            other => panic!("{other:?}"),
        }
    }
}

/// R2-8 (a), documented residual: a collector that completes a normal TLS close after OUR
/// close_notify without having kept the batch is indistinguishable from an acknowledging one,
/// so the batch counts as acknowledged (RELEASE-HOLDS 23-4.i, D-23-4-16).
#[test]
fn a_collector_that_closes_after_ours_without_keeping_the_batch_is_acknowledged() {
    let rig = Rig::new();
    rig.fake.set_mode(Mode::DiscardThenAck);
    rig.sink()
        .deliver(&[dummy_frame()])
        .expect("indistinguishable from an acknowledgement: the documented residual");
    match rig.fake.next_event() {
        Event::Acked { bytes, .. } => assert!(bytes.is_empty(), "the collector kept nothing"),
        other => panic!("{other:?}"),
    }
}
