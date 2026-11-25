//! Story 23-8 — the external KMS adapter against an in-process `maos.kms.v1`
//! endpoint (AC1 a–c, AC3, AC4, AC5). No container, no network, no sleeps: the
//! probe and the staleness bound run on an injected clock, and the
//! never-answering peer is asserted under an outer deadline, not by elapsed time.

#![cfg(all(feature = "http-kms", not(feature = "kms-fault-inject")))]

#[path = "support/kms_fixture_23_8.rs"]
mod kms_fixture;

use kms_fixture::{b64_decode, b64_encode, Disclosure, KmsFixture, Mode};
use maos_domain::ports::{KeyManagementPort, KmsError};
use maos_kernel_core::security::RingCryptoProvider;
use maos_secrets::http_kms::{
    HttpKms, KmsClock, MonotonicClock, TimedHttp, Timeouts, DEFAULT_PROBE_INTERVAL,
};
use maos_secrets::{open_at_rest, seal_at_rest};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

const PAYLOAD: &[u8] = b"{\"region\":\"default\",\"payload\":\"kms-canary-7731\"}";
const BEARER: &str = "kms-bearer-canary-9f3c";

struct FakeClock(AtomicU64);

impl FakeClock {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicU64::new(1_000)))
    }

    fn advance(&self, by: Duration) {
        self.0.fetch_add(by.as_millis() as u64, Ordering::SeqCst);
    }
}

impl KmsClock for FakeClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
}

fn adapter(url: &str, bearer: Option<&str>, clock: &Arc<FakeClock>) -> HttpKms {
    HttpKms::new(
        Arc::new(TimedHttp::default()),
        url,
        bearer.map(str::to_owned),
        Arc::clone(clock) as Arc<dyn KmsClock>,
    )
    .expect("a plain http endpoint")
}

fn healthy_adapter(fixture: &KmsFixture, clock: &Arc<FakeClock>) -> HttpKms {
    let kms = adapter(&fixture.url(), None, clock);
    assert!(kms.probe(), "the fixture answers /health");
    assert!(kms.is_healthy());
    kms
}

// ── AC1 (a) (b) ─────────────────────────────────────────────────────────────

#[test]
fn seal_wraps_through_the_endpoint_once_and_open_unwraps_through_it() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    let crypto = RingCryptoProvider;

    let sealed = seal_at_rest(&kms, &crypto, PAYLOAD).expect("seal through the endpoint");
    assert_eq!(fixture.count("POST", "/wrap"), 1, "exactly one POST /wrap");
    assert_eq!(fixture.count("POST", "/unwrap"), 0);
    assert_ne!(sealed.as_slice(), PAYLOAD);
    assert!(
        !sealed.windows(PAYLOAD.len()).any(|w| w == PAYLOAD),
        "sealed bytes must not carry the plaintext"
    );
    // Envelope: MAOSKMS1 | u16 wrapped_len | wrapped | nonce | ciphertext. A
    // wrapped key is the KMS's AEAD output (nonce + key + tag), so it is longer
    // than the 32-byte data key; a "wrap" that returns the key itself is not.
    let wrapped_len = usize::from(u16::from_be_bytes([sealed[8], sealed[9]]));
    assert!(
        wrapped_len > 32,
        "the wrapped data key must not be the 32-byte data key itself (len {wrapped_len})"
    );

    let opened = open_at_rest(&kms, &sealed).expect("open through the endpoint");
    assert_eq!(opened, PAYLOAD);
    assert_eq!(
        fixture.count("POST", "/unwrap"),
        1,
        "open calls POST /unwrap"
    );
}

// ── AC1 (c) ─────────────────────────────────────────────────────────────────

#[test]
fn an_encrypted_file_store_over_the_endpoint_round_trips_a_credential() {
    use maos_domain::ports::{SecretKey, SecretStore};

    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = Arc::new(healthy_adapter(&fixture, &clock));
    let directory = tempfile::tempdir().expect("fixture directory");
    let root = directory.path().join("vault");
    let store = maos_secrets::EncryptedFileSecretStore::new(
        root.clone(),
        kms,
        Arc::new(RingCryptoProvider),
    );

    store
        .put(SecretKey::AnthropicApiKey, "sk-ant-through-the-kms")
        .expect("store a credential");
    assert_eq!(fixture.count("POST", "/wrap"), 1);
    let on_disk = std::fs::read(root.join("anthropic-api-key.sealed")).expect("sealed file");
    assert!(
        !on_disk
            .windows(b"sk-ant-through-the-kms".len())
            .any(|w| w == b"sk-ant-through-the-kms"),
        "the file on disk holds no plaintext"
    );
    assert_eq!(
        store.get(SecretKey::AnthropicApiKey).unwrap().as_deref(),
        Some("sk-ant-through-the-kms")
    );
    assert_eq!(
        fixture.count("POST", "/unwrap"),
        1,
        "the read went through /unwrap"
    );
}

#[test]
fn with_the_endpoint_down_a_read_is_an_error_never_plaintext() {
    use maos_domain::ports::{SecretKey, SecretStore};

    let mut fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = Arc::new(healthy_adapter(&fixture, &clock));
    let directory = tempfile::tempdir().expect("fixture directory");
    let store = maos_secrets::EncryptedFileSecretStore::new(
        directory.path().join("vault"),
        kms.clone(),
        Arc::new(RingCryptoProvider),
    );
    store
        .put(SecretKey::OpenAiApiKey, "sk-openai-down")
        .unwrap();
    fixture.stop();
    assert!(!kms.probe(), "a refused connection is a failed probe");
    let error = store
        .get(SecretKey::OpenAiApiKey)
        .expect_err("a read with the KMS down must fail");
    assert!(!error.to_string().contains("sk-openai-down"));
}

// ── AC4 — the wire ──────────────────────────────────────────────────────────

#[test]
fn requests_carry_the_version_and_a_32_byte_key_and_compare_by_key() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    let data_key = [7u8; 32];
    let wrapped = kms.wrap_data_key(&data_key).expect("wrap");
    assert_eq!(kms.unwrap_data_key(&wrapped).expect("unwrap"), data_key);

    let requests = fixture.requests();
    let wrap: serde_json::Value =
        serde_json::from_str(&requests.iter().find(|r| r.path == "/wrap").unwrap().body).unwrap();
    assert_eq!(wrap["version"], "maos.kms.v1");
    assert_eq!(
        b64_decode(wrap["plaintext"].as_str().unwrap()).unwrap(),
        data_key
    );
    let unwrap: serde_json::Value =
        serde_json::from_str(&requests.iter().find(|r| r.path == "/unwrap").unwrap().body).unwrap();
    assert_eq!(unwrap["version"], "maos.kms.v1");
    assert_eq!(
        b64_decode(unwrap["wrapped"].as_str().unwrap()).unwrap(),
        wrapped
    );
    let health = requests.iter().find(|r| r.path == "/health").unwrap();
    assert_eq!(health.method, "GET");
}

fn malformed(error: KmsError) -> bool {
    matches!(error, KmsError::MalformedPayload(_))
}

#[test]
fn a_malformed_answer_is_a_malformed_payload_and_flips_health() {
    let answers = [
        // wrong version
        r#"{"version":"maos.kms.v0","wrapped":"AAAA"}"#.to_owned(),
        // missing field
        r#"{"version":"maos.kms.v1"}"#.to_owned(),
        // bad base64
        r#"{"version":"maos.kms.v1","wrapped":"!!not base64!!"}"#.to_owned(),
        // not JSON
        "not json".to_owned(),
        // a wrapped blob over 4,096 bytes
        format!(
            r#"{{"version":"maos.kms.v1","wrapped":"{}"}}"#,
            b64_encode(&vec![9u8; 4097])
        ),
    ];
    for answer in answers {
        let fixture = KmsFixture::start();
        let clock = FakeClock::new();
        let kms = healthy_adapter(&fixture, &clock);
        fixture.set_mode(Mode::Fixed(200, answer.clone()));
        let error = kms.wrap_data_key(&[1u8; 32]).expect_err(&answer);
        assert!(malformed(error), "{answer}");
        assert!(!kms.is_healthy(), "a malformed 2xx flips health: {answer}");
    }

    // An unwrap result that is not 32 bytes.
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.set_mode(Mode::Fixed(
        200,
        format!(
            r#"{{"version":"maos.kms.v1","plaintext":"{}"}}"#,
            b64_encode(&[1u8; 16])
        ),
    ));
    assert!(malformed(kms.unwrap_data_key(&[2u8; 40]).unwrap_err()));
    assert!(!kms.is_healthy());

    // A real endpoint that answers with another version.
    let fixture = KmsFixture::start();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.set_mode(Mode::WrongVersion);
    assert!(malformed(kms.wrap_data_key(&[1u8; 32]).unwrap_err()));
    assert!(!kms.is_healthy());
}

#[test]
fn an_oversized_wrapped_key_is_refused_before_any_request() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    let before = fixture.requests().len();
    assert!(malformed(
        kms.unwrap_data_key(&vec![0u8; 4097]).unwrap_err()
    ));
    assert!(malformed(kms.unwrap_data_key(&[]).unwrap_err()));
    assert_eq!(fixture.requests().len(), before, "nothing was sent");
    assert!(
        kms.is_healthy(),
        "a local refusal is not an availability failure"
    );
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 31]),
        Err(KmsError::MalformedKey(_))
    ));
}

#[test]
fn a_typed_422_is_a_crypto_refusal_and_leaves_health_alone() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    // The real endpoint refuses a foreign wrapped key with a typed 422.
    let error = kms
        .unwrap_data_key(&[3u8; 64])
        .expect_err("foreign wrapped key");
    assert!(matches!(error, KmsError::Crypto(_)), "{error:?}");
    assert!(kms.is_healthy(), "D-23-8-10 A: a 422 does not flip health");

    fixture.set_mode(Mode::Fixed(422, r#"{"error":"wrap_failed"}"#.to_owned()));
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Crypto(_))
    ));
    assert!(kms.is_healthy());

    // A 422 that is not the typed refusal is not a typed refusal.
    fixture.set_mode(Mode::Fixed(422, r#"{"error":"whatever"}"#.to_owned()));
    assert!(malformed(kms.wrap_data_key(&[1u8; 32]).unwrap_err()));
    assert!(!kms.is_healthy());
}

// ── AC5 — health ────────────────────────────────────────────────────────────

#[test]
fn a_5xx_flips_health_down_at_once_and_a_probe_restores_it() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.set_mode(Mode::Fixed(503, "{}".to_owned()));
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Unavailable(_))
    ));
    assert!(!kms.is_healthy(), "a 5xx flips health at once");

    fixture.set_mode(Mode::Normal);
    assert!(!kms.is_healthy(), "only a probe restores health");
    assert!(kms.probe());
    assert!(kms.is_healthy());
}

#[test]
fn an_unreachable_endpoint_is_unavailable_and_a_probe_after_restart_restores_health() {
    let mut fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.stop();

    assert!(!kms.probe(), "the probe of a stopped endpoint fails");
    assert!(!kms.is_healthy());
    assert!(matches!(
        seal_at_rest(&kms, &RingCryptoProvider, PAYLOAD),
        Err(KmsError::Unavailable(_))
    ));
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Unavailable(_))
    ));

    let restarted = KmsFixture::restart(&fixture);
    assert!(kms.probe(), "the probe sees the restarted endpoint");
    assert!(seal_at_rest(&kms, &RingCryptoProvider, PAYLOAD).is_ok());
    assert_eq!(restarted.count("POST", "/wrap"), 1);
}

#[test]
fn a_stopped_prober_goes_unhealthy() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = adapter(&fixture.url(), None, &clock);
    assert!(!kms.is_healthy(), "never probed is unhealthy");
    assert!(kms.probe());
    assert!(kms.is_healthy());

    clock.advance(DEFAULT_PROBE_INTERVAL * 3);
    assert!(kms.is_healthy(), "exactly three intervals is still fresh");
    clock.advance(Duration::from_millis(1));
    assert!(
        !kms.is_healthy(),
        "a result older than 3 intervals is stale"
    );
    assert!(matches!(
        seal_at_rest(&kms, &RingCryptoProvider, PAYLOAD),
        Err(KmsError::Unavailable(_))
    ));
    assert!(matches!(
        open_at_rest(&kms, b"MAOSKMS1 never read"),
        Err(KmsError::Unavailable(_))
    ));

    assert!(kms.probe());
    assert!(kms.is_healthy());
}

#[test]
fn is_healthy_does_no_io() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    let before = fixture.requests().len();
    for _ in 0..50 {
        let _ = kms.is_healthy();
    }
    assert_eq!(fixture.requests().len(), before);
}

// ── AC3 — transport and trust ───────────────────────────────────────────────

#[test]
fn a_redirect_is_a_refusal_and_is_never_followed() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.set_mode(Mode::Redirect);
    let error = kms
        .wrap_data_key(&[1u8; 32])
        .expect_err("a 3xx is a refusal");
    assert!(matches!(error, KmsError::Unavailable(_)));
    assert!(!error.to_string().contains("elsewhere"), "no Location text");
    assert!(!kms.is_healthy());
    assert!(
        fixture.requests().iter().all(|r| r.path != "/elsewhere"),
        "the redirect target was never requested"
    );
}

#[test]
fn a_peer_that_never_answers_makes_wrap_unavailable() {
    let fixture = KmsFixture::start();
    fixture.set_mode(Mode::Hang);
    let clock = FakeClock::new();
    let kms = HttpKms::new(
        Arc::new(TimedHttp::new(Timeouts {
            connect: Duration::from_millis(300),
            io: Duration::from_millis(300),
            overall: Duration::from_secs(2),
            probe: Duration::from_secs(2),
        })),
        &fixture.url(),
        None,
        Arc::clone(&clock) as Arc<dyn KmsClock>,
    )
    .unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send((kms.wrap_data_key(&[1u8; 32]), kms.is_healthy()));
    });
    // The outer deadline is a failure bound, never an elapsed-time assertion.
    let (result, healthy) = receiver
        .recv_timeout(Duration::from_secs(30))
        .expect("wrap_data_key must return under the configured timeout");
    assert!(matches!(result, Err(KmsError::Unavailable(_))));
    assert!(!healthy);
}

#[test]
fn a_response_over_the_64_kib_cap_is_an_error() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = healthy_adapter(&fixture, &clock);
    fixture.set_mode(Mode::Fixed(
        200,
        format!(
            r#"{{"version":"maos.kms.v1","pad":"{}"}}"#,
            "x".repeat(70_000)
        ),
    ));
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Unavailable(_))
    ));
}

#[test]
fn the_bearer_goes_only_on_wrap_and_unwrap_and_never_into_errors_or_debug() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = adapter(&fixture.url(), Some(BEARER), &clock);
    assert!(kms.probe());
    let wrapped = kms.wrap_data_key(&[4u8; 32]).unwrap();
    kms.unwrap_data_key(&wrapped).unwrap();

    for request in fixture.requests() {
        match request.path.as_str() {
            "/wrap" | "/unwrap" => assert_eq!(
                request.authorization.as_deref(),
                Some(format!("Bearer {BEARER}").as_str()),
                "{} carries the bearer",
                request.path
            ),
            _ => assert_eq!(request.authorization, None, "{} carries none", request.path),
        }
        assert!(!request.body.contains(BEARER));
    }

    let mut rendered = vec![format!("{kms:?}")];
    for mode in [
        Mode::Fixed(401, r#"{"detail":"nope"}"#.to_owned()),
        Mode::Fixed(503, "{}".to_owned()),
        Mode::Fixed(200, "garbage".to_owned()),
        Mode::Fixed(422, r#"{"error":"unwrap_failed"}"#.to_owned()),
        Mode::Redirect,
    ] {
        fixture.set_mode(mode);
        rendered.push(format!("{:?}", kms.wrap_data_key(&[4u8; 32])));
        rendered.push(format!("{:?}", kms.unwrap_data_key(&wrapped)));
    }
    let mut fixture = fixture;
    fixture.stop();
    rendered.push(format!("{:?}", kms.wrap_data_key(&[4u8; 32])));
    for text in rendered {
        assert!(!text.contains(BEARER), "the bearer leaked into: {text}");
    }
}

#[test]
fn an_adapter_without_its_bearer_decision_refuses_everything_until_provisioned() {
    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let kms = HttpKms::pending_bearer(
        Arc::new(TimedHttp::default()),
        &fixture.url(),
        Arc::clone(&clock) as Arc<dyn KmsClock>,
    )
    .unwrap();
    assert!(kms.probe(), "a probe carries no credential");
    assert!(!kms.is_healthy(), "a pending bearer is never healthy");
    let before = fixture.requests().len();
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Unavailable(_))
    ));
    assert_eq!(
        fixture.requests().len(),
        before,
        "nothing is sent while pending"
    );

    kms.provision_bearer(Some(BEARER.to_owned()));
    assert!(kms.is_healthy());
    kms.wrap_data_key(&[1u8; 32]).unwrap();
    assert_eq!(
        fixture.requests().last().unwrap().authorization.as_deref(),
        Some(format!("Bearer {BEARER}").as_str())
    );
}

#[test]
fn an_endpoint_must_be_a_plain_base_url() {
    let clock = FakeClock::new();
    let build = |url: &str| {
        HttpKms::new(
            Arc::new(TimedHttp::default()),
            url,
            None,
            Arc::clone(&clock) as Arc<dyn KmsClock>,
        )
    };
    for refused in [
        "ftp://127.0.0.1:1",
        "127.0.0.1:1",
        "http://user:pass@127.0.0.1:1",
        "http://127.0.0.1:1/?token=x",
        "http://127.0.0.1:1/#frag",
        "http://",
        "http:// 127.0.0.1",
    ] {
        assert!(build(refused).is_err(), "{refused}");
    }
    assert!(build("https://kms.example.org/v1/maos/").is_ok());
    assert!(HttpKms::new(
        Arc::new(TimedHttp::default()),
        "http://127.0.0.1:1",
        None,
        Arc::new(MonotonicClock::new())
    )
    .is_ok());
}

// ── Review fixes: logging, probe ordering, probe timeout, echoing endpoints ──

type Notices = Arc<std::sync::Mutex<Vec<String>>>;

fn with_notices(kms: HttpKms) -> (HttpKms, Notices) {
    let notices: Notices = Arc::default();
    let sink = Arc::clone(&notices);
    let kms = kms.with_notice_sink(Arc::new(move |line| {
        sink.lock().unwrap().push(line.to_owned());
    }));
    (kms, notices)
}

/// P3-1: a wrap or unwrap that fails is logged once per change of class, with
/// the status class and no secret, URL or body; recovery is logged once.
#[test]
fn a_failed_call_is_logged_once_per_change_with_its_status_class_and_never_a_secret() {
    let mut fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let (kms, notices) = with_notices(adapter(&fixture.url(), Some(BEARER), &clock));
    assert!(kms.probe());
    assert!(!kms.calls_failing());

    fixture.set_mode(Mode::Fixed(401, "{}".to_owned()));
    for _ in 0..3 {
        assert!(matches!(
            kms.wrap_data_key(&[1u8; 32]),
            Err(KmsError::Unavailable(_))
        ));
    }
    assert!(kms.calls_failing());
    assert_eq!(notices.lock().unwrap().len(), 1, "once per state change");
    assert!(notices.lock().unwrap()[0].contains("401"));

    fixture.set_mode(Mode::Fixed(503, "{}".to_owned()));
    let _ = kms.wrap_data_key(&[1u8; 32]);
    let _ = kms.wrap_data_key(&[1u8; 32]);
    fixture.set_mode(Mode::Fixed(403, "{}".to_owned()));
    let _ = kms.wrap_data_key(&[1u8; 32]);
    {
        let seen = notices.lock().unwrap();
        assert_eq!(seen.len(), 3, "{seen:?}");
        assert!(seen[1].contains("5xx"), "{}", seen[1]);
        assert!(seen[2].contains("403"), "{}", seen[2]);
    }

    // A typed 422 is a refusal of one payload, not a failure of the endpoint.
    fixture.set_mode(Mode::Fixed(422, r#"{"error":"wrap_failed"}"#.to_owned()));
    assert!(matches!(
        kms.wrap_data_key(&[1u8; 32]),
        Err(KmsError::Crypto(_))
    ));
    assert_eq!(notices.lock().unwrap().len(), 3);

    fixture.set_mode(Mode::Normal);
    kms.wrap_data_key(&[1u8; 32])
        .expect("the endpoint answers again");
    assert!(!kms.calls_failing());
    kms.wrap_data_key(&[1u8; 32]).unwrap();
    assert_eq!(notices.lock().unwrap().len(), 4, "recovery is logged once");
    assert!(notices.lock().unwrap()[3].contains("answered again"));

    let url = fixture.url();
    fixture.stop();
    let _ = kms.wrap_data_key(&[1u8; 32]);
    let seen = notices.lock().unwrap();
    assert_eq!(seen.len(), 5, "{seen:?}");
    assert!(seen[4].contains("could not be reached"), "{}", seen[4]);
    for line in seen.iter() {
        assert!(
            !line.contains(BEARER) && !line.contains(&url) && !line.contains("127.0.0.1"),
            "a notice must carry no secret or address: {line}"
        );
    }
}

/// P3-1: a bearer that is pending or unreadable refuses every call, sends
/// nothing, and says so once.
#[test]
fn a_pending_or_unreadable_bearer_is_logged_once_and_sends_nothing() {
    use maos_secrets::http_kms::BearerStatus;

    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let (kms, notices) = with_notices(
        HttpKms::pending_bearer(
            Arc::new(TimedHttp::default()),
            &fixture.url(),
            Arc::clone(&clock) as Arc<dyn KmsClock>,
        )
        .unwrap(),
    );
    assert_eq!(kms.bearer_status(), BearerStatus::Pending);
    for _ in 0..2 {
        assert!(kms.wrap_data_key(&[1u8; 32]).is_err());
    }
    assert_eq!(notices.lock().unwrap().len(), 1);
    assert!(notices.lock().unwrap()[0].contains("not provisioned"));

    kms.bearer_unreadable();
    assert_eq!(kms.bearer_status(), BearerStatus::Unreadable);
    assert!(kms.probe(), "a probe carries no credential");
    assert!(!kms.is_healthy(), "an unreadable bearer is never healthy");
    for _ in 0..2 {
        assert!(kms.unwrap_data_key(&[1u8; 16]).is_err());
    }
    assert_eq!(notices.lock().unwrap().len(), 2);
    assert!(notices.lock().unwrap()[1].contains("could not be read"));
    assert_eq!(
        fixture.count("POST", "/wrap") + fixture.count("POST", "/unwrap"),
        0
    );

    kms.provision_bearer(Some(BEARER.to_owned()));
    assert_eq!(kms.bearer_status(), BearerStatus::Settled);
    assert!(kms.probe() && kms.is_healthy());
    assert!(
        !kms.calls_failing(),
        "a refusal from before the bearer was settled no longer describes the adapter"
    );
}

/// An endpoint that reflects the `Authorization` it was sent in its body and
/// headers, for every status class, must not get that value into any error,
/// `Debug` output or notice (P3-5).
#[test]
fn a_response_that_echoes_the_bearer_never_reaches_an_error_a_debug_or_a_log_line() {
    use maos_domain::ports::IoSubsystemPort;

    let fixture = KmsFixture::start();
    let clock = FakeClock::new();
    let (kms, notices) = with_notices(adapter(&fixture.url(), Some(BEARER), &clock));
    assert!(kms.probe());
    let wrapped = kms.wrap_data_key(&[4u8; 32]).unwrap();

    let mut rendered = vec![format!("{kms:?}")];
    for status in [200u16, 401, 403, 422, 500, 302] {
        fixture.set_mode(Mode::EchoAuthorization(status));
        for result in [kms.wrap_data_key(&[4u8; 32]), kms.unwrap_data_key(&wrapped)] {
            let error = result.expect_err("a reflected header is never a good answer");
            rendered.push(format!("{error:?}"));
            rendered.push(error.to_string());
        }
    }
    // The transport on its own, for a non-2xx and a 3xx.
    let io = TimedHttp::default();
    let authorization = format!("Bearer {BEARER}");
    for status in [500u16, 302] {
        fixture.set_mode(Mode::EchoAuthorization(status));
        let url = format!("{}/wrap", fixture.url());
        let result = io.http_post(&url, b"{}", &[("Authorization", authorization.as_str())]);
        rendered.push(format!("{:?}", result.expect_err("non-2xx")));
    }
    rendered.extend(notices.lock().unwrap().iter().cloned());

    let echoing_requests = fixture
        .requests()
        .into_iter()
        .filter(|r| r.path == "/wrap" || r.path == "/unwrap")
        .count();
    assert!(
        echoing_requests >= 14,
        "the fixture was asked {echoing_requests} times"
    );
    assert!(
        fixture
            .requests()
            .last()
            .and_then(|r| r.authorization.clone())
            .is_some_and(|value| value.contains(BEARER)),
        "the echoed value really was the bearer"
    );
    assert!(
        notices.lock().unwrap().len() >= 3,
        "the failures were logged"
    );
    for text in rendered {
        assert!(!text.contains(BEARER), "the bearer leaked into: {text}");
    }
}

/// A scripted transport: `http_get` runs a one-shot hook before it answers
/// healthy; every POST is a 503.
#[derive(Default)]
struct ScriptedIo {
    during_probe: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl maos_domain::ports::IoSubsystemPort for ScriptedIo {
    fn http_get(&self, _url: &str) -> Result<Vec<u8>, maos_domain::ports::io_subsystem::IoError> {
        let hook = self.during_probe.lock().unwrap().take();
        if let Some(hook) = hook {
            hook();
        }
        Ok(br#"{"version":"maos.kms.v1","status":"ok"}"#.to_vec())
    }

    fn http_post(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, maos_domain::ports::io_subsystem::IoError> {
        Err(maos_domain::ports::io_subsystem::IoError::Transport(
            "scripted".to_owned(),
        ))
    }

    fn http_post_exchange(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(&str, &str)],
        _response_header: &str,
    ) -> Result<
        maos_domain::ports::io_subsystem::HttpExchange,
        maos_domain::ports::io_subsystem::IoError,
    > {
        Ok(maos_domain::ports::io_subsystem::HttpExchange {
            status: 503,
            body: Vec::new(),
            header: None,
        })
    }
}

/// P3-2: a probe that started before a call failed must not clear the `down`
/// that call set. Deterministic: the failing call runs inside the probe's own
/// transport, so no thread, sleep or race is involved.
#[test]
fn a_probe_in_flight_cannot_clear_a_failure_recorded_after_it_started() {
    let io = Arc::new(ScriptedIo::default());
    let clock = FakeClock::new();
    let kms = Arc::new(
        HttpKms::new(
            Arc::clone(&io) as Arc<dyn maos_domain::ports::IoSubsystemPort>,
            "http://127.0.0.1:1",
            None,
            Arc::clone(&clock) as Arc<dyn KmsClock>,
        )
        .unwrap()
        .with_notice_sink(Arc::new(|_| {})),
    );
    let inner = Arc::clone(&kms);
    *io.during_probe.lock().unwrap() = Some(Box::new(move || {
        assert!(matches!(
            inner.wrap_data_key(&[1u8; 32]),
            Err(KmsError::Unavailable(_))
        ));
    }));

    // The healthy answer is discarded, so the probe does not report health.
    assert_eq!(
        kms.probe_checked(),
        maos_secrets::http_kms::ProbeResult::Discarded,
        "a healthy answer that was discarded must say so, not report success"
    );
    assert!(
        !kms.is_healthy(),
        "a failure recorded while the probe was in flight must stand"
    );
    assert!(
        kms.probe(),
        "a probe that starts afterwards restores health"
    );
    assert!(kms.is_healthy());
}

/// P3-3: the probe has its own overall limit, shorter than a POST's. With the
/// other limits far out of reach, only the probe limit can end this probe;
/// the outer deadline is a failure bound, not an elapsed-time assertion.
#[test]
fn the_probe_has_its_own_overall_limit_and_a_hanging_endpoint_fails_it() {
    assert_eq!(Timeouts::default().probe, Duration::from_secs(2));
    assert!(Timeouts::default().probe < Timeouts::default().overall);

    let fixture = KmsFixture::start();
    fixture.set_mode(Mode::Hang);
    let clock = FakeClock::new();
    let kms = HttpKms::new(
        Arc::new(TimedHttp::new(Timeouts {
            connect: Duration::from_secs(120),
            io: Duration::from_secs(120),
            overall: Duration::from_secs(120),
            probe: Duration::from_millis(400),
        })),
        &fixture.url(),
        None,
        Arc::clone(&clock) as Arc<dyn KmsClock>,
    )
    .unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send((kms.probe(), kms.is_healthy()));
    });
    let (reachable, healthy) = receiver
        .recv_timeout(Duration::from_secs(30))
        .expect("the probe must end under its own limit, not the 120 s ones");
    assert!(!reachable && !healthy);
}

/// KMS-1: a wrapping endpoint that hands the data key back (verbatim, padded,
/// headered, half of it, or as base64 or hex text) is refused with a typed
/// `Crypto` error; no envelope is produced, no bytes are in the error and a
/// refused payload is not an outage. An opaque answer cannot prove encryption:
/// this rejects obvious disclosure only.
#[test]
fn a_wrapped_key_that_discloses_the_data_key_is_refused_and_nothing_is_sealed() {
    for how in [
        Disclosure::Verbatim,
        Disclosure::Padded,
        Disclosure::Headered,
        Disclosure::Prefix,
        Disclosure::Suffix,
        Disclosure::Base64Text,
        Disclosure::HexText,
    ] {
        let fixture = KmsFixture::start();
        let clock = FakeClock::new();
        let (kms, notices) = with_notices(healthy_adapter(&fixture, &clock));
        fixture.set_mode(Mode::HostileWrap(how));

        let key = [7u8; 32];
        let error = kms
            .wrap_data_key(&key)
            .expect_err("a disclosing wrapped key must be refused");
        assert!(matches!(error, KmsError::Crypto(_)), "{how:?}: {error:?}");
        let text = format!("{error:?} {error}");
        assert!(
            !text.contains(&b64_encode(&key)) && !text.contains("0707"),
            "{how:?}: bytes in the error: {text}"
        );
        assert!(
            kms.is_healthy(),
            "{how:?}: a refused payload is not an outage"
        );

        // The seal path: never an envelope, never plaintext.
        match seal_at_rest(&kms, &RingCryptoProvider, PAYLOAD) {
            Err(KmsError::Crypto(_)) => {}
            other => panic!(
                "{how:?}: expected a Crypto refusal, got {:?}",
                other.map(|b| b.len())
            ),
        }
        assert!(
            notices
                .lock()
                .unwrap()
                .iter()
                .any(|l| l.contains("discloses")),
            "{how:?}: the refusal is logged"
        );
    }
}
