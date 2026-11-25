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
    run_tick, BatchSink, ExportError, Frame, Tick, MAX_BATCH_BYTES, MAX_BATCH_FRAMES,
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

    fn tick(&self, sink: &mut TlsSink, store: &CursorStore) -> Result<Tick, ExportError> {
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
        2,
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
    assert_eq!(failure.buffered(), 2);
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
