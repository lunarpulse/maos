//! Story 23-4 — composition-root wiring of the SIEM network sink and the durable cursor
//! (AC4 rotation and expiry through the real adapter, real sealer and real snapshot path; AC5
//! non-blocking writes; AC6 the client key; AC7 selection and boot refusals). The collector is
//! the in-process fake SIEM of `maos-siem`'s tests (a rustls server with a generated CA); no
//! sleeps: every wait is a channel receive or a gate, bounded by a deadline. Every
//! daemon-booting test runs the `maos` binary with a scrubbed environment: a stray
//! `MAOS_SIEM_*`, `MAOS_KMS_*`, `MAOS_SSO_*` or `MAOS_AUDIT_KEY` in the runner would flip the
//! posture (`any_env_with_prefix`), so nothing is inherited.

#![cfg(feature = "network")]

#[path = "../../maos-siem/tests/support/fake_siem.rs"]
mod fake_siem;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

use fake_siem::{external_ids, Ca, Event, FakeSiem, Mode, Options, StubResolver};
use maos_bin::enterprise_identity::{
    configure_siem, ConfiguredSiem, EnterpriseConfig, EnterpriseFailure, EnterpriseRuntime,
    SiemBootRefusal, SiemEnv, SiemReport,
};
use maos_domain::outbound::{ResolveFailed, ResolveHost};
use maos_domain::ports::{SecretKey, SiemProjectionPort};
use maos_siem::SiemExporter;

const CANARY_KEY: &str = "SIEM-KEY-CANARY-7731";
const HOST: &str = "siem.test";

// ── selection and refusals (AC7), in-process ─────────────────────────────────

struct Names(Vec<std::net::IpAddr>);

impl ResolveHost for Names {
    fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<std::net::IpAddr>, ResolveFailed> {
        Ok(self.0.clone())
    }
}

fn public() -> Arc<dyn ResolveHost> {
    Arc::new(Names(vec!["93.184.216.34".parse().unwrap()]))
}

fn select(env: &SiemEnv<'_>) -> Result<Option<ConfiguredSiem>, SiemBootRefusal> {
    configure_siem(env, public())
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write fixture file");
    path.to_string_lossy().into_owned()
}

#[test]
fn the_endpoint_and_the_file_together_are_a_conflict_naming_both_variables_and_no_value() {
    let refusal = select(&SiemEnv {
        endpoint: Some("collector.example:6514"),
        file: Some("/var/log/secret-path/siem.log"),
        ..SiemEnv::default()
    })
    .unwrap_err();
    assert_eq!(refusal, SiemBootRefusal::Conflict);
    assert_eq!(refusal.code(), "siem_config_conflict");
    let text = refusal.to_string();
    assert!(text.contains("MAOS_SIEM_ENDPOINT") && text.contains("MAOS_SIEM_FILE"));
    assert!(!text.contains("collector.example") && !text.contains("secret-path"));
}

#[test]
fn no_variable_is_no_sink_and_the_file_alone_keeps_the_file_sink() {
    assert!(select(&SiemEnv::default()).unwrap().is_none());
    let file = select(&SiemEnv {
        file: Some("/var/log/maos/siem.ndjson"),
        ..SiemEnv::default()
    })
    .unwrap()
    .expect("the file sink");
    assert_eq!(file.kind_name(), "local file");
    assert_eq!(
        file.file_path(),
        Some(PathBuf::from("/var/log/maos/siem.ndjson"))
    );
    assert!(!file.needs_client_key());
    file.provision_client_key(None)
        .expect("a file sink needs no key");
}

#[cfg(not(feature = "online-adapters"))]
#[test]
fn a_build_without_online_adapters_refuses_the_endpoint() {
    let refusal = select(&SiemEnv {
        endpoint: Some("collector.example:6514"),
        ca_file: Some("/etc/ca.pem"),
        ..SiemEnv::default()
    })
    .unwrap_err();
    assert_eq!(refusal, SiemBootRefusal::UnsupportedBuild);
    assert_eq!(refusal.code(), "siem_endpoint_unsupported_build");
}

#[cfg(feature = "online-adapters")]
mod online {
    use super::*;

    fn pki() -> (tempfile::TempDir, Ca, String) {
        let dir = tempfile::tempdir().expect("fixture dir");
        let ca = Ca::new("siem-test-ca");
        let ca_file = write(dir.path(), "ca.pem", &ca.pem);
        (dir, ca, ca_file)
    }

    #[test]
    fn the_endpoint_needs_a_ca_and_a_well_formed_host_port() {
        let (_dir, _ca, ca_file) = pki();
        assert_eq!(
            select(&SiemEnv {
                endpoint: Some("collector.example:6514"),
                ..SiemEnv::default()
            })
            .unwrap_err()
            .code(),
            "siem_ca_missing"
        );
        for malformed in [
            "http://collector.example:6514",
            "collector.example:0",
            "collector.example:65536",
            "collector.example:tls",
            ":6514",
            "2001:db8::1:6514",
            "[2001:db8::1",
            "[not-an-address]:6514",
            "collector example:6514",
            "collector.example/path:6514",
        ] {
            let refusal = select(&SiemEnv {
                endpoint: Some(malformed),
                ca_file: Some(&ca_file),
                ..SiemEnv::default()
            })
            .unwrap_err();
            assert_eq!(
                refusal,
                SiemBootRefusal::EndpointRefused("endpoint_invalid"),
                "{malformed}"
            );
            assert!(
                !refusal.to_string().contains(malformed),
                "no value is printed"
            );
        }
        // Port default 6514; IPv6 in brackets.
        let default_port = select(&SiemEnv {
            endpoint: Some("collector.example"),
            ca_file: Some(&ca_file),
            ..SiemEnv::default()
        })
        .unwrap()
        .expect("selected");
        assert_eq!(
            default_port.network().unwrap().endpoint(),
            "collector.example:6514"
        );
        let v6 = select(&SiemEnv {
            endpoint: Some("[2001:db8::7]:7000"),
            ca_file: Some(&ca_file),
            ..SiemEnv::default()
        });
        // 2001:db8::/32 is documentation space: refused by class, never by syntax.
        assert!(
            matches!(v6, Err(SiemBootRefusal::EndpointRefused(code)) if code == "outbound_reserved_refused")
        );
    }

    #[test]
    fn the_host_class_is_vetted_and_private_ranges_need_the_explicit_opt_in() {
        let (_dir, _ca, ca_file) = pki();
        let refused = |endpoint: &str, allow_private: bool| {
            configure_siem(
                &SiemEnv {
                    endpoint: Some(endpoint),
                    ca_file: Some(&ca_file),
                    allow_private,
                    ..SiemEnv::default()
                },
                public(),
            )
        };
        for (endpoint, code) in [
            ("127.0.0.1:6514", "outbound_loopback_refused"),
            ("10.1.2.3:6514", "outbound_private_refused"),
            ("192.168.0.9:6514", "outbound_private_refused"),
            ("169.254.169.254:6514", "outbound_link_local_refused"),
        ] {
            assert_eq!(
                refused(endpoint, false).unwrap_err(),
                SiemBootRefusal::EndpointRefused(code),
                "{endpoint}"
            );
        }
        // MAOS_SIEM_ALLOW_PRIVATE admits loopback and private; link-local stays refused.
        assert!(refused("127.0.0.1:6514", true).unwrap().is_some());
        assert!(refused("10.1.2.3:6514", true).unwrap().is_some());
        assert_eq!(
            refused("169.254.169.254:6514", true).unwrap_err(),
            SiemBootRefusal::EndpointRefused("outbound_link_local_refused")
        );
        // A name that does not resolve yet is not a boot refusal: the sink is down until it does.
        struct Dead;
        impl ResolveHost for Dead {
            fn resolve(&self, _: &str, _: u16) -> Result<Vec<std::net::IpAddr>, ResolveFailed> {
                Err(ResolveFailed)
            }
        }
        assert!(configure_siem(
            &SiemEnv {
                endpoint: Some("collector.example:6514"),
                ca_file: Some(&ca_file),
                ..SiemEnv::default()
            },
            Arc::new(Dead),
        )
        .unwrap()
        .is_some());
        // A name that resolves to a private address is refused by class.
        let private_names: Arc<dyn ResolveHost> =
            Arc::new(Names(vec!["10.0.0.8".parse().unwrap()]));
        assert_eq!(
            configure_siem(
                &SiemEnv {
                    endpoint: Some("collector.example:6514"),
                    ca_file: Some(&ca_file),
                    ..SiemEnv::default()
                },
                private_names,
            )
            .unwrap_err(),
            SiemBootRefusal::EndpointRefused("outbound_private_refused")
        );
    }

    #[test]
    fn an_unusable_ca_or_client_certificate_is_refused_and_a_stray_key_is_incomplete() {
        let (dir, ca, ca_file) = pki();
        let junk = write(dir.path(), "junk.pem", "not a certificate");
        let empty = write(dir.path(), "empty.pem", "");
        let cert = ca.client("maos-host");
        let cert_file = write(dir.path(), "client.pem", &cert.cert_pem);
        let base = SiemEnv {
            endpoint: Some("collector.example:6514"),
            ..SiemEnv::default()
        };
        for bad_ca in [junk.as_str(), empty.as_str(), "/nonexistent/ca.pem"] {
            let refusal = select(&SiemEnv {
                ca_file: Some(bad_ca),
                ..base
            })
            .unwrap_err();
            assert_eq!(refusal, SiemBootRefusal::CaInvalid, "{bad_ca}");
            assert!(!refusal.to_string().contains(bad_ca));
        }
        assert_eq!(
            select(&SiemEnv {
                ca_file: Some(&ca_file),
                client_cert_file: Some("/nonexistent/client.pem"),
                ..base
            })
            .unwrap_err(),
            SiemBootRefusal::ClientCertInvalid
        );
        assert_eq!(
            select(&SiemEnv {
                ca_file: Some(&ca_file),
                client_key_in_env: true,
                ..base
            })
            .unwrap_err(),
            SiemBootRefusal::ClientIncomplete,
            "MAOS_SIEM_CLIENT_KEY without MAOS_SIEM_CLIENT_CERT_FILE"
        );
        let selected = select(&SiemEnv {
            ca_file: Some(&ca_file),
            client_cert_file: Some(&cert_file),
            ..base
        })
        .unwrap()
        .expect("a certificate is read at selection; its key is settled later");
        assert!(selected.needs_client_key());
    }

    // ── AC6: the client key ──────────────────────────────────────────────────────

    #[test]
    fn the_client_key_is_a_closed_secret_key_that_purge_names() {
        assert!(SecretKey::ALL.contains(&SecretKey::SiemClientKey));
        assert_eq!(SecretKey::SiemClientKey.as_str(), "siem-client-key");
        assert_eq!(
            SecretKey::SiemClientKey.environment_variable(),
            "MAOS_SIEM_CLIENT_KEY"
        );
    }

    #[test]
    fn a_missing_unparsable_or_foreign_client_key_refuses_and_no_canary_escapes() {
        let (dir, ca, ca_file) = pki();
        let client = ca.client("maos-host");
        let cert_file = write(dir.path(), "client.pem", &client.cert_pem);
        let selected = select(&SiemEnv {
            endpoint: Some("collector.example:6514"),
            ca_file: Some(&ca_file),
            client_cert_file: Some(&cert_file),
            ..SiemEnv::default()
        })
        .unwrap()
        .expect("selected");
        let foreign = ca.client("someone-else").key_pem;
        for key in [
            None,
            Some(""),
            Some("   "),
            Some(CANARY_KEY),
            Some(foreign.as_str()),
        ] {
            let refusal = selected.provision_client_key(key).unwrap_err();
            assert_eq!(refusal, SiemBootRefusal::ClientKeyInvalid, "{key:?}");
            assert_eq!(refusal.code(), "siem_client_key_invalid");
            let shown = format!("{refusal} {refusal:?} {selected:?}");
            assert!(
                !shown.contains(CANARY_KEY) && !shown.contains("PRIVATE KEY"),
                "{shown}"
            );
        }
        selected
            .provision_client_key(Some(client.key_pem.as_str()))
            .expect("the certificate's own key is accepted");
        let shown = format!("{selected:?} {:?}", selected.network());
        assert!(
            !shown.contains("PRIVATE KEY"),
            "Debug withholds key material: {shown}"
        );
    }

    // ── the runtime over a real adapter: rotation, expiry, the snapshot path ─────

    use maos_cli::audit_rotation::DaemonSealer;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_iac::adapter::transparency_log::{
        FrameKind, FrameRowWrite, RotationOutcome, TransparencyLogAdapter,
    };

    const SEED: [u8; 32] = [0x23; 32];
    const NONCE: u64 = 0x23_04;
    const ALL: i64 = i64::MAX;

    struct Host {
        _dir: tempfile::TempDir,
        db: PathBuf,
        tl: Arc<TransparencyLogAdapter>,
        fake: FakeSiem,
        runtime: Arc<EnterpriseRuntime>,
    }

    impl Host {
        fn new() -> Self {
            Self::with(Options::default(), true)
        }

        /// A Host whose audit store is the real adapter (live WAL, signed rotation) and whose
        /// SIEM sink is the network sink toward the fake collector.
        fn with(options: Options, listening: bool) -> Self {
            let dir = tempfile::tempdir().expect("fixture dir");
            let db = dir.path().join("audit").join("transparency.sqlite");
            std::fs::create_dir_all(db.parent().unwrap()).unwrap();
            let tl = Arc::new(
                TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, NONCE)
                    .expect("open log"),
            );
            tl.install_segment_sealer(Arc::new(DaemonSealer::new(SEED, None, None)))
                .expect("one sealer");
            let ca = Ca::new("siem-test-ca");
            let fake = FakeSiem::start(&ca.server(HOST), options);
            let ca_file = write(dir.path(), "ca.pem", &ca.pem);
            let endpoint = format!("{HOST}:{}", fake.addr.port());
            let siem = configure_siem(
                &SiemEnv {
                    endpoint: Some(&endpoint),
                    ca_file: Some(&ca_file),
                    allow_private: true,
                    ..SiemEnv::default()
                },
                StubResolver::loopback(HOST),
            )
            .expect("select the sink")
            .expect("a network sink");
            let _ = listening;
            let runtime = EnterpriseRuntime::from_ports(
                EnterpriseConfig::empty().with_siem_available(),
                None,
                None,
                Some(Arc::new(SiemExporter) as Arc<dyn SiemProjectionPort>),
                None,
                Some(db.clone()),
                None,
                NONCE,
            )
            .with_configured_siem(&siem);
            let runtime = Arc::new(runtime);
            Self {
                _dir: dir,
                db,
                tl,
                fake,
                runtime,
            }
        }

        fn segments(&self) -> PathBuf {
            self.db.parent().unwrap().join("segments")
        }

        fn cursor(&self) -> PathBuf {
            self.db.with_file_name("siem-cursor.json")
        }

        fn put(&self, n: u32) {
            let written = self
                .tl
                .insert_frame_event_with_id(
                    Some(id(n)),
                    FrameKind::TaskAssign,
                    7,
                    "",
                    "",
                    None,
                    &format!("test.frame.{n}"),
                    format!("payload-{n}").as_bytes(),
                    FrameOrigin::HumanAuthored,
                )
                .into_inner();
            assert_eq!(written, FrameRowWrite::Written, "frame {n}");
        }

        fn rotate(&self) {
            match self.tl.rotate_through(ALL, ALL).expect("rotation") {
                RotationOutcome::Sealed(_) => {}
                other => panic!("expected a sealed segment, got {other:?}"),
            }
        }

        /// One export tick; the acknowledged frames' `externalId`s, in wire order.
        fn export(&self) -> Result<Vec<String>, EnterpriseFailure> {
            let tick = self.runtime.forward_audit_to_siem()?;
            let mut ids = Vec::new();
            for _ in 0..tick.batches {
                match self.fake.next_event() {
                    Event::Acked { bytes, .. } => {
                        ids.extend(external_ids(&fake_siem::octet_frames(&bytes)));
                    }
                    other => panic!("expected an acknowledged batch, got {other:?}"),
                }
            }
            assert_eq!(ids.len(), tick.delivered);
            Ok(ids)
        }

        /// Rows of the LIVE file whose intent contains `intent` (the merged view would itself
        /// refuse a log whose registered segment file is missing).
        fn rows_with_intent(&self, intent: &str) -> Vec<LiveRow> {
            let raw = rusqlite::Connection::open(&self.db).expect("raw connection");
            let mut statement = raw
                .prepare(
                    "SELECT CAST(payload_redacted AS TEXT) FROM transparency_log \
                     WHERE intent LIKE '%' || ?1 || '%' ORDER BY rowid",
                )
                .unwrap();
            let rows = statement
                .query_map([intent], |row| {
                    Ok(LiveRow {
                        payload: row.get(0)?,
                    })
                })
                .unwrap();
            rows.map(Result::unwrap).collect()
        }
    }

    struct LiveRow {
        payload: String,
    }
    fn id(n: u32) -> [u8; 16] {
        let mut id = [0xEE; 16];
        id[..4].copy_from_slice(&n.to_be_bytes());
        id
    }

    fn hex(n: u32) -> String {
        format!("{n:08x}{}", "ee".repeat(12))
    }

    fn mine(ids: &[String]) -> Vec<String> {
        ids.iter()
            .filter(|id| id.ends_with(&"ee".repeat(12)))
            .cloned()
            .collect()
    }

    fn now_ns() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64
    }

    /// AC4: rows sealed into segments before the export are delivered once, in insertion
    /// order, through the daemon's real snapshot path (the adapter holds the live WAL); a
    /// restart (a new runtime over the same cursor file) resends nothing.
    #[test]
    fn rotated_rows_are_delivered_exactly_once_and_in_order() {
        let host = Host::new();
        for n in 1..=6 {
            host.put(n);
        }
        host.rotate();
        for n in 7..=10 {
            host.put(n);
        }
        host.rotate();
        for n in 11..=12 {
            host.put(n);
        }
        assert_eq!(
            host.tl.segment_registry().unwrap().len(),
            2,
            "two sealed segments"
        );
        assert!(
            host.db.with_extension("sqlite-wal").exists(),
            "the adapter holds the live WAL"
        );

        let first = mine(&host.export().expect("first tick"));
        assert_eq!(
            first,
            (1..=12).map(hex).collect::<Vec<_>>(),
            "all rows, in order, once"
        );
        assert!(
            host.export().expect("second tick").is_empty(),
            "nothing is re-sent"
        );

        host.put(13);
        assert_eq!(
            mine(&host.export().unwrap()),
            vec![hex(13)],
            "exactly the new row"
        );
        assert!(host.cursor().is_file());
    }

    /// AC4 / D-23-4-06 A: an expired segment that held undelivered ids produces exactly one
    /// `siem.gap` row; the cursor moves past it.
    #[test]
    fn an_expired_segment_that_held_unsent_rows_journals_exactly_one_gap() {
        let host = Host::new();
        let old = now_ns() - 200 * 86_400 * 1_000_000_000;
        for n in 1..=3 {
            host.put(n);
        }
        let raw = rusqlite::Connection::open(&host.db).expect("raw connection");
        for rowid in 1..=2 {
            raw.execute(
                "UPDATE transparency_log SET timestamp_ns = ?1 WHERE rowid = ?2",
                [old, rowid],
            )
            .unwrap();
        }
        host.rotate();
        host.put(4);
        let expired = host
            .tl
            .expire_segments(90, now_ns() as u64)
            .expect("expiry");
        assert_eq!(expired, vec![1]);
        assert!(!host.segments().join("0000000001.sqlite").exists());

        let first = mine(&host.export().expect("tick over the gap"));
        assert_eq!(
            first,
            vec![hex(3), hex(4)],
            "only the retained rows are delivered"
        );
        let gaps = host.rows_with_intent("siem.gap");
        assert_eq!(gaps.len(), 1, "exactly one siem.gap row");
        let payload: serde_json::Value = serde_json::from_str(&gaps[0].payload).unwrap();
        assert_eq!(
            (
                payload["from"].as_i64(),
                payload["to"].as_i64(),
                payload["segment_seq"].as_i64()
            ),
            (Some(1), Some(2), Some(1))
        );
        assert_eq!(payload["sink"].as_str().map(str::len), Some(16));

        // The next tick delivers the journal rows themselves and invents no second gap.
        let next = host.export().expect("second tick");
        assert!(!next.is_empty());
        assert_eq!(host.rows_with_intent("siem.gap").len(), 1);
        assert!(host.export().unwrap().is_empty());
    }

    /// AC4: a registered segment file missing at read time is a tick error, never a gap.
    #[test]
    fn a_missing_segment_file_is_a_tick_error_never_a_gap() {
        let host = Host::new();
        for n in 1..=3 {
            host.put(n);
        }
        host.rotate();
        std::fs::remove_file(host.segments().join("0000000001.sqlite")).unwrap();
        match host.runtime.forward_audit_to_siem() {
            Err(EnterpriseFailure::SiemSinkDown { reason, .. }) => {
                assert_eq!(reason, "siem_source_unavailable")
            }
            other => panic!("expected siem_source_unavailable, got {other:?}"),
        }
        assert!(
            host.rows_with_intent("siem.gap").is_empty(),
            "no gap is invented"
        );
        assert!(!host.cursor().exists(), "nothing was acknowledged");
        assert!(host.fake.drain().is_empty(), "nothing was sent");
    }

    /// AC5: a collector that accepts TCP and never completes the handshake does not hold up
    /// an audit write. The export blocks on its own thread; a burst of Transparency Log
    /// inserts completes while the gate is still held (asserted before the gate opens: no
    /// elapsed-time assertion), then the held export finishes and every row is delivered.
    #[test]
    fn audit_writes_complete_while_the_collector_holds_the_handshake() {
        let host = Arc::new(Host::with(Options::default(), true));
        host.put(1);
        host.fake.set_mode(Mode::HoldHandshake);
        let exporter = {
            let runtime = Arc::clone(&host.runtime);
            std::thread::spawn(move || runtime.forward_audit_to_siem())
        };
        // The export is now committed to a handshake the collector will not complete.
        assert!(matches!(host.fake.next_event(), Event::Held));
        for n in 2..=51 {
            host.put(n);
        }
        let rows = host.rows_with_intent("test.frame.");
        assert_eq!(
            rows.len(),
            51,
            "the burst of inserts completed while the gate is held"
        );
        assert!(
            !exporter.is_finished(),
            "the export is still waiting on the collector"
        );

        host.fake.set_mode(Mode::Ack);
        host.fake.handshake_gate.open();
        let tick = exporter
            .join()
            .expect("export thread")
            .expect("the held export succeeds");
        assert!(
            tick.delivered >= 1,
            "the held tick exports the snapshot it took before the burst"
        );
        assert!(matches!(host.fake.next_event(), Event::Acked { .. }));
        // The rest of the burst follows in the next tick, in order.
        let rest = host.export().expect("next tick");
        assert_eq!(mine(&rest), (2..=51).map(hex).collect::<Vec<_>>());
    }

    /// AC5: a sink that is down leaves the cursor and reports the rows after it with a typed
    /// reason; the consumer prints one line per distinct reason; when the sink returns every
    /// row since the cursor is delivered within one tick.
    #[test]
    fn a_down_collector_keeps_the_cursor_reports_the_backlog_and_recovers_in_one_tick() {
        let host = Host::new();
        for n in 1..=3 {
            host.put(n);
        }
        assert_eq!(mine(&host.export().unwrap()).len(), 3);
        let cursor_before = std::fs::read(host.cursor()).unwrap();

        // Down: nothing answers the handshake and every bounded wait is short.
        let mut report = SiemReport::default();
        for n in 4..=8 {
            host.put(n);
        }
        host.fake.set_mode(Mode::CloseWithoutNotify);
        let outcome = host.runtime.forward_audit_to_siem();
        let Err(EnterpriseFailure::SiemSinkDown {
            buffered, reason, ..
        }) = &outcome
        else {
            panic!("expected a sink-down outcome, got {outcome:?}");
        };
        assert_eq!((*buffered, *reason), (5, "siem_sink_no_close_notify"));
        assert!(matches!(host.fake.next_event(), Event::Unacked { .. }));
        assert_eq!(
            std::fs::read(host.cursor()).unwrap(),
            cursor_before,
            "the cursor stays"
        );
        assert_eq!(report.lines(&outcome).len(), 1, "one line for a new reason");
        // The same reason again prints nothing.
        host.put(9);
        let again = host.runtime.forward_audit_to_siem();
        assert!(matches!(host.fake.next_event(), Event::Unacked { .. }));
        assert!(
            report.lines(&again).is_empty(),
            "one typed line per distinct reason"
        );

        // Back: every row since the cursor, in one tick.
        host.fake.set_mode(Mode::Ack);
        let recovered = host.runtime.forward_audit_to_siem();
        let lines = report.lines(&recovered);
        assert!(
            lines.iter().any(|line| line.contains("acknowledged again")),
            "{lines:?}"
        );
        let ids = {
            let tick = recovered.expect("recovered");
            assert_eq!(tick.delivered, 6);
            let mut ids = Vec::new();
            for _ in 0..tick.batches {
                let Event::Acked { bytes, .. } = host.fake.next_event() else {
                    panic!("batch")
                };
                ids.extend(external_ids(&fake_siem::octet_frames(&bytes)));
            }
            ids
        };
        assert_eq!(mine(&ids), (4..=9).map(hex).collect::<Vec<_>>());
    }

    /// AC5/AC6: before the client key is settled a network sink with a certificate sends
    /// nothing; afterwards it presents the certificate to a server that requires one.
    #[test]
    fn a_sink_with_a_client_certificate_sends_nothing_until_its_key_is_settled() {
        let client_ca = Ca::new("client-ca");
        let host_ca = Ca::new("host-ca");
        let dir = tempfile::tempdir().expect("fixture dir");
        let db = dir.path().join("audit.sqlite");
        {
            let tl = TransparencyLogAdapter::open(&db, NONCE).expect("open log");
            tl.insert_frame_event(
                maos_kernel_core::iac::transparency_log::FrameKind::TaskAssign,
                1,
                None,
                "test.frame",
                b"x",
                FrameOrigin::HumanAuthored,
            );
        }
        let fake = FakeSiem::start(
            &host_ca.server(HOST),
            Options {
                require_client_ca: Some(client_ca.der()),
                ..Options::default()
            },
        );
        let client = client_ca.client("maos-host");
        let ca_file = write(dir.path(), "ca.pem", &host_ca.pem);
        let cert_file = write(dir.path(), "client.pem", &client.cert_pem);
        let endpoint = format!("{HOST}:{}", fake.addr.port());
        let siem = configure_siem(
            &SiemEnv {
                endpoint: Some(&endpoint),
                ca_file: Some(&ca_file),
                client_cert_file: Some(&cert_file),
                allow_private: true,
                ..SiemEnv::default()
            },
            StubResolver::loopback(HOST),
        )
        .unwrap()
        .unwrap();
        let runtime = EnterpriseRuntime::from_ports(
            EnterpriseConfig::empty().with_siem_available(),
            None,
            None,
            Some(Arc::new(SiemExporter) as Arc<dyn SiemProjectionPort>),
            None,
            Some(db.clone()),
            None,
            NONCE,
        )
        .with_configured_siem(&siem);

        match runtime.forward_audit_to_siem() {
            Err(EnterpriseFailure::SiemSinkDown { reason, .. }) => {
                assert_eq!(reason, "siem_client_key_pending")
            }
            other => panic!("expected a pending key, got {other:?}"),
        }
        assert!(
            fake.drain().is_empty(),
            "no connection before the key is settled"
        );

        siem.provision_client_key(Some(client.key_pem.as_str()))
            .expect("key");
        let tick = runtime
            .forward_audit_to_siem()
            .expect("mutual TLS delivery");
        assert!(tick.delivered >= 1);
        assert!(matches!(fake.next_event(), Event::Acked { .. }));
    }

    // ── review round 1 ───────────────────────────────────────────────────────────

    /// An expired segment with unsent rows (as in the gap test); returns the Host ready to export.
    fn host_with_expired_segment(options: Options) -> Host {
        let host = Host::with(options, true);
        let old = now_ns() - 200 * 86_400 * 1_000_000_000;
        for n in 1..=3 {
            host.put(n);
        }
        let raw = rusqlite::Connection::open(&host.db).expect("raw connection");
        for rowid in 1..=2 {
            raw.execute(
                "UPDATE transparency_log SET timestamp_ns = ?1 WHERE rowid = ?2",
                [old, rowid],
            )
            .unwrap();
        }
        host.rotate();
        host.put(4);
        assert_eq!(
            host.tl.expire_segments(90, now_ns() as u64).unwrap(),
            vec![1]
        );
        host
    }

    fn snapshots(host: &Host) -> Vec<String> {
        std::fs::read_dir(host.db.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".siem-snapshot-"))
            .collect()
    }

    /// A blocked cursor temp file fails every tick AFTER the gap row is written. However many
    /// ticks fail, the log holds exactly one `siem.gap` row (the writer dedupes by the
    /// deterministic payload), and no snapshot is left behind by a failed tick.
    #[test]
    fn a_blocked_cursor_write_adds_one_gap_row_however_many_ticks_fail() {
        let host = host_with_expired_segment(Options::default());
        let tmp = host.cursor().with_extension("json.tmp");
        std::fs::create_dir(&tmp).expect("block the cursor temp file");
        for _ in 0..4 {
            match host.runtime.forward_audit_to_siem() {
                Err(EnterpriseFailure::SiemSinkDown { reason, .. }) => {
                    assert_eq!(reason, "siem_cursor_io")
                }
                other => panic!("expected a cursor failure, got {other:?}"),
            }
            assert!(snapshots(&host).is_empty(), "{:?}", snapshots(&host));
        }
        assert_eq!(
            host.rows_with_intent("siem.gap").len(),
            1,
            "one row after four failing ticks"
        );
        std::fs::remove_dir(&tmp).unwrap();
        assert!(host.export().is_ok());
        assert_eq!(host.rows_with_intent("siem.gap").len(), 1);
    }

    /// A gap journaled earlier in a tick that then fails on the sink is still reported: the
    /// failure carries the tick's typed lines, and `SiemReport` prints them.
    #[test]
    fn a_gap_noted_before_a_sink_failure_is_not_lost() {
        let host = host_with_expired_segment(Options::default());
        host.fake.set_mode(Mode::CloseWithoutNotify);
        let outcome = host.runtime.forward_audit_to_siem();
        let Err(EnterpriseFailure::SiemSinkDown { notes, reason, .. }) = &outcome else {
            panic!("expected a sink failure, got {outcome:?}");
        };
        assert_eq!(*reason, "siem_sink_no_close_notify");
        assert!(
            notes.iter().any(|note| note.starts_with("siem_gap")),
            "{notes:?}"
        );
        let lines = SiemReport::default().lines(&outcome);
        assert!(
            lines.iter().any(|line| line.contains("siem_gap")),
            "{lines:?}"
        );
    }

    /// Snapshots a crashed export left behind are removed when the sink is built at boot.
    #[test]
    fn stale_snapshots_are_removed_when_the_exporter_is_built() {
        let host = Host::new();
        let stale = host.db.with_extension("siem-snapshot-1-0.sqlite");
        std::fs::write(&stale, b"a copy of the log").unwrap();
        let siem = configure_siem(
            &SiemEnv {
                file: Some("/tmp/e234-unused-sink.log"),
                ..SiemEnv::default()
            },
            public(),
        )
        .unwrap()
        .unwrap();
        let _runtime = EnterpriseRuntime::from_ports(
            EnterpriseConfig::empty().with_siem_available(),
            None,
            None,
            Some(Arc::new(SiemExporter) as Arc<dyn SiemProjectionPort>),
            None,
            Some(host.db.clone()),
            None,
            NONCE,
        )
        .with_configured_siem(&siem);
        assert!(!stale.exists(), "the stale snapshot is removed at boot");
    }

    /// `cancel_siem_export` reaches the blocking export through the runtime.
    #[test]
    fn the_runtime_cancel_ends_an_export_held_in_the_handshake() {
        let host = Arc::new(Host::with(Options::default(), true));
        host.put(1);
        host.fake.set_mode(Mode::HoldHandshake);
        let exporter = {
            let runtime = Arc::clone(&host.runtime);
            std::thread::spawn(move || runtime.forward_audit_to_siem())
        };
        assert!(matches!(host.fake.next_event(), Event::Held));
        host.runtime.cancel_siem_export();
        match exporter.join().unwrap() {
            Err(EnterpriseFailure::SiemSinkDown { reason, .. }) => {
                assert_eq!(reason, "siem_export_cancelled")
            }
            other => panic!("expected a cancelled export, got {other:?}"),
        }
        assert!(
            snapshots(&host).is_empty(),
            "the cancelled export removed its snapshot"
        );
        host.fake.handshake_gate.open();
    }
}

// ── boot refusals: the `maos` binary, scrubbed environment (AC7, AC6) ────────

fn boot(root: &Path, name: &str, vars: &[(&str, &str)]) -> Output {
    use std::io::Write;

    let home = root.join(format!("home-{name}"));
    let maos_home = root.join(format!("maos-home-{name}"));
    std::fs::create_dir_all(&maos_home).expect("create MAOS_HOME");
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .env_clear()
        .current_dir(root)
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("MAOS_HOME", &maos_home)
        .env("XDG_DATA_HOME", root.join(format!("xdg-data-{name}")))
        .env("XDG_CONFIG_HOME", root.join(format!("xdg-config-{name}")))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env("MAOS_OLLAMA_URL", "skip")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in vars {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("start maos");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"/quit\n")
        .ok();
    child.wait_with_output().expect("wait for maos")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: &Output, code: &str) -> String {
    let text = stderr(output);
    assert_eq!(output.status.code(), Some(78), "{text}");
    assert!(text.contains(code), "expected {code} in: {text}");
    text
}

#[test]
fn boot_refuses_a_conflict_and_a_stray_client_key_and_names_no_value() {
    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();
    let ca = Ca::new("siem-boot-ca");
    let ca_file = write(root, "ca.pem", &ca.pem);

    let conflict = boot(
        root,
        "conflict",
        &[
            ("MAOS_SIEM_ENDPOINT", "collector.example:6514"),
            ("MAOS_SIEM_FILE", "/var/log/secret-path/siem.log"),
        ],
    );
    let text = assert_refused(&conflict, "siem_config_conflict");
    assert!(text.contains("MAOS_SIEM_ENDPOINT") && text.contains("MAOS_SIEM_FILE"));
    assert!(!text.contains("secret-path") && !text.contains("collector.example"));

    #[cfg(feature = "online-adapters")]
    {
        let missing_ca = boot(
            root,
            "no-ca",
            &[("MAOS_SIEM_ENDPOINT", "collector.example:6514")],
        );
        assert_refused(&missing_ca, "siem_ca_missing");

        let private = boot(
            root,
            "private",
            &[
                ("MAOS_SIEM_ENDPOINT", "10.9.9.9:6514"),
                ("MAOS_SIEM_CA_FILE", &ca_file),
            ],
        );
        let text = assert_refused(&private, "siem_endpoint_refused");
        assert!(text.contains("outbound_private_refused"));
        assert!(!text.contains("10.9.9.9"));

        let stray = boot(
            root,
            "stray-key",
            &[
                ("MAOS_SIEM_ENDPOINT", "127.0.0.1:1"),
                ("MAOS_SIEM_ALLOW_PRIVATE", "1"),
                ("MAOS_SIEM_CA_FILE", &ca_file),
                ("MAOS_SIEM_CLIENT_KEY", CANARY_KEY),
            ],
        );
        let text = assert_refused(&stray, "siem_client_incomplete");
        assert!(!text.contains(CANARY_KEY));
    }
    #[cfg(not(feature = "online-adapters"))]
    {
        let unsupported = boot(
            root,
            "unsupported",
            &[
                ("MAOS_SIEM_ENDPOINT", "collector.example:6514"),
                ("MAOS_SIEM_CA_FILE", &ca_file),
            ],
        );
        assert_refused(&unsupported, "siem_endpoint_unsupported_build");
    }
}

#[cfg(feature = "online-adapters")]
#[test]
fn boot_refuses_a_client_certificate_without_a_usable_key_and_boots_with_one() {
    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();
    let ca = Ca::new("siem-boot-ca");
    let client = ca.client("maos-host");
    let ca_file = write(root, "ca.pem", &ca.pem);
    let cert_file = write(root, "client.pem", &client.cert_pem);
    let base = |extra: &[(&'static str, String)]| {
        let mut vars: Vec<(&'static str, String)> = vec![
            ("MAOS_SIEM_ENDPOINT", "127.0.0.1:1".to_owned()),
            ("MAOS_SIEM_ALLOW_PRIVATE", "1".to_owned()),
            ("MAOS_SIEM_CA_FILE", ca_file.clone()),
            ("MAOS_SIEM_CLIENT_CERT_FILE", cert_file.clone()),
        ];
        vars.extend(extra.iter().cloned());
        vars
    };
    let run = |name: &str, vars: Vec<(&'static str, String)>| {
        let borrowed: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
        boot(root, name, &borrowed)
    };

    // No key at all, a canary that is not a PEM key, and another certificate's key.
    for (name, extra) in [
        ("no-key", vec![]),
        (
            "canary-key",
            vec![("MAOS_SIEM_CLIENT_KEY", CANARY_KEY.to_owned())],
        ),
        (
            "foreign-key",
            vec![("MAOS_SIEM_CLIENT_KEY", ca.client("other").key_pem)],
        ),
    ] {
        let output = run(name, base(&extra));
        let text = assert_refused(&output, "siem_client_key_invalid");
        assert!(
            !text.contains(CANARY_KEY) && !text.contains("PRIVATE KEY"),
            "{text}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains(CANARY_KEY),
            "the key is not printed on stdout either"
        );
    }

    // The certificate's own key boots (the endpoint need not be reachable).
    let good = run(
        "good-key",
        base(&[("MAOS_SIEM_CLIENT_KEY", client.key_pem.clone())]),
    );
    let text = stderr(&good);
    assert!(good.status.success(), "{text}");
    assert!(
        text.contains("SIEM sink selected: syslog over TLS"),
        "{text}"
    );
    assert!(text.contains("SIEM client key read"), "{text}");
    assert!(!text.contains("PRIVATE KEY"), "{text}");
}

#[test]
fn boot_with_no_siem_variable_starts_no_consumer_and_the_file_sink_alone_boots() {
    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();

    let none = boot(root, "none", &[]);
    let text = stderr(&none);
    assert!(none.status.success(), "{text}");
    assert!(!text.contains("SIEM sink selected"), "{text}");
    assert!(
        !text.contains("SIEM export consumer spawned"),
        "no consumer without a sink: {text}"
    );

    let sink = root.join("siem.ndjson");
    let file = boot(root, "file", &[("MAOS_SIEM_FILE", sink.to_str().unwrap())]);
    let text = stderr(&file);
    assert!(file.status.success(), "{text}");
    assert!(text.contains("SIEM sink selected: local file"), "{text}");
}

/// The mTLS key is read through the configured store: with `encrypted-file` and the key ONLY
/// in the sealed vault (not in the environment) the boot succeeds and settles it.
#[cfg(feature = "online-adapters")]
#[test]
fn boot_reads_the_client_key_from_the_encrypted_file_store() {
    use maos_domain::ports::SecretStore as _;

    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();
    let ca = Ca::new("siem-boot-ca");
    let client = ca.client("maos-host");
    let ca_file = write(root, "ca.pem", &ca.pem);
    let cert_file = write(root, "client.pem", &client.cert_pem);
    let maos_home = root.join("maos-home-sealed");
    std::fs::create_dir_all(&maos_home).unwrap();
    let kms = maos_secrets::LocalMasterKeyKms::from_master_key(&[0x42_u8; 32]).expect("kms");
    maos_secrets::EncryptedFileSecretStore::new(
        maos_home.join("secrets"),
        Arc::new(kms),
        Arc::new(maos_kernel_core::security::RingCryptoProvider),
    )
    .put(SecretKey::SiemClientKey, &client.key_pem)
    .expect("seal the client key");

    let master = "42".repeat(32);
    let output = boot(
        root,
        "sealed",
        &[
            ("MAOS_SECRETS_BACKEND", "encrypted-file"),
            ("MAOS_KMS_MASTER_KEY", &master),
            ("MAOS_SIEM_ENDPOINT", "127.0.0.1:1"),
            ("MAOS_SIEM_ALLOW_PRIVATE", "1"),
            ("MAOS_SIEM_CA_FILE", &ca_file),
            ("MAOS_SIEM_CLIENT_CERT_FILE", &cert_file),
        ],
    );
    let text = stderr(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("SIEM client key read"), "{text}");
    assert!(!text.contains("PRIVATE KEY"), "{text}");
}

/// R2-1: `MAOS_AUDIT_DB=/x/audit.db` is supported. SQLite's sidecar is `audit.db-wal` (the name
/// is APPENDED to the full file name), so a guard that swaps the extension never sees it. The
/// committed rows below exist ONLY in the live WAL; the export must still deliver every one.
#[test]
fn a_live_database_without_the_sqlite_extension_exports_rows_that_exist_only_in_the_wal() {
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_iac::adapter::transparency_log::{FrameKind, TransparencyLogAdapter};

    let dir = tempfile::tempdir().expect("fixture dir");
    let db = dir.path().join("audit.db");
    let tl = TransparencyLogAdapter::open(&db, 0x23_04).expect("open log");
    for n in 0..5 {
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            &format!("wal.only.{n}"),
            b"x",
            FrameOrigin::HumanAuthored,
        );
    }
    let wal = dir.path().join("audit.db-wal");
    assert!(
        wal.exists() && std::fs::metadata(&wal).unwrap().len() > 0,
        "rows are in the WAL"
    );
    let sink = dir.path().join("siem.log");
    let siem = configure_siem(
        &SiemEnv {
            file: Some(sink.to_str().unwrap()),
            ..SiemEnv::default()
        },
        public(),
    )
    .unwrap()
    .unwrap();
    let runtime = EnterpriseRuntime::from_ports(
        EnterpriseConfig::empty().with_siem_available(),
        None,
        None,
        Some(Arc::new(SiemExporter) as Arc<dyn SiemProjectionPort>),
        None,
        Some(db.clone()),
        None,
        0x23_04,
    )
    .with_configured_siem(&siem);
    let tick = runtime.forward_audit_to_siem().expect("export");
    let text = std::fs::read_to_string(&sink).unwrap();
    for n in 0..5 {
        assert!(
            text.contains(&format!("wal.only.{n}")),
            "row {n} missing: delivered {}",
            tick.delivered
        );
    }
    drop(tl);
}
