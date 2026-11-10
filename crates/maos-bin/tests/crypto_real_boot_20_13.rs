#![cfg(feature = "network")]
#![forbid(unsafe_code)]

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_cohort_daemon_shares_provider_across_crl_token_and_seal() {
    use ed25519_dalek::SigningKey;
    use futures_util::{SinkExt, StreamExt};
    use maos_a2a_core::{
        A2AJsonRpcRequest, InMemoryTofuPinStore, PeerCertFingerprint, PeerId, TofuPinStore,
        COHORT_INTENT_DIGEST_READ,
    };
    use maos_a2a_tcp::{build_client_config, length_delimited_codec, TrustPosture};
    use maos_cohort::{
        CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ConsentTuple,
        DigestReadControl, ManifestSignature, COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT,
        RESERVED_INTENT_REISSUE,
    };
    use maos_domain::frame::{ConsentEnvelope, FrameAddress, FramePayload, IacFrame};
    use maos_domain::invariants::{
        i1::IntentClass, i13::IntentLineage, i3::FrameOrigin, i8::A2AIntent,
    };
    use maos_domain::ports::registry::SpiritId;
    use maos_domain::revocation::{
        canonical_entries_bytes, CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
    };
    use maos_spirit_abi::identity::{FrameKind, HostId};
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
    use tokio_util::codec::Framed;

    struct Reap(std::process::Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        "[crypto]\nprovider = \"recording\"\n",
    )
    .unwrap();
    let data = dir.path().join("data");

    let cert = |name: &str| {
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = rcgen::CertificateParams::new(vec!["127.0.0.1".into()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        let fp = PeerCertFingerprint::from_cert_der(cert.der().as_ref());
        std::fs::write(dir.path().join(format!("{name}.pem")), cert.pem()).unwrap();
        std::fs::write(dir.path().join(format!("{name}.key")), key.serialize_pem()).unwrap();
        (
            cert.der().clone(),
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
            fp,
        )
    };
    let (_server_der, _server_key, server_fp) = cert("server");
    let (client_der, client_key, client_fp) = cert("client");
    let authority = SigningKey::from_bytes(&[13; 32]);
    let cohort = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "crypto-real-boot".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host-a".into(),
                fingerprint: server_fp.wire(),
                roles: vec!["worker".into()],
                team: None,
            },
            CohortMember {
                host_id: "host-b".into(),
                fingerprint: client_fp.wire(),
                roles: vec!["worker".into()],
                team: None,
            },
        ],
        consent: ConsentMatrix {
            send: vec![],
            accept: vec![ConsentTuple {
                peer: "host-b".into(),
                role: "worker".into(),
                intent: COHORT_INTENT_DIGEST_READ.into(),
            }],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: vec![],
    }
    .signed_with(&authority);
    let cohort_path = dir.path().join("cohort.toml");
    std::fs::write(&cohort_path, toml::to_string(&cohort).unwrap()).unwrap();
    let config = format!("manifest_path = '{}'\nauthority_keys = ['{}']\nlocal_host = 'host-a'\ncontrol_spirit = 'researcher'\n[[peers]]\npeer_id = 'host-b'\nendpoint = 'tls://127.0.0.1:1'\ncert_fingerprint = {{ algo = 'sha256', hex = '{}' }}\nsend_allowlist = ['cohort:digest-read']\naccept_allowlist = ['cohort:digest-read']\n[tcp]\nlisten_addr = '127.0.0.1:0'\nown_cert_chain = '{}'\nown_private_key = '{}'\npeer_pins = [{{ peer_id = 'host-b', fingerprint = {{ algo = 'sha256', hex = '{}' }}, boot_nonce = 135001 }}]\n[digest_summary]\nframes = 0\nhalts = 0\nconflicts = 0\n", cohort_path.display(), hex::encode(authority.verifying_key().to_bytes()), client_fp.hex, dir.path().join("server.pem").display(), dir.path().join("server.key").display(), client_fp.hex);
    let config_path = dir.path().join("daemon.toml");
    std::fs::write(&config_path, config).unwrap();
    let crl_key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let crl_public: [u8; 32] = crl_key.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![RevocationEntry::new("never-matches", ">=0.1.0,<0.2.0", "compromised", None).unwrap()];
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).unwrap(),
        1,
        0,
        RevocationOrigin::Operator,
        entries.clone(),
        crl_key
            .sign(&canonical_entries_bytes(&entries).unwrap())
            .as_ref()
            .try_into()
            .unwrap(),
        crl_public,
    )
    .unwrap();
    let crl_path = dir.path().join("input.crl");
    std::fs::write(&crl_path, serde_json::to_vec(&crl).unwrap()).unwrap();
    let binary = fixtures::fixture("maos-crypto-recording");
    let ctl = std::path::Path::new(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    let env = |cmd: &mut Command| {
        cmd.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", dir.path())
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("MAOS_MEMORY_ROOT", dir.path().join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_OLLAMA_URL", "skip")
            .env("MAOS_CRL_TRUST_ANCHOR_PUB_HEX", hex::encode(crl_public));
    };
    let mut command = Command::new(&binary);
    env(&mut command);
    let mut init = Command::new(&binary);
    env(&mut init);
    let initialized = init
        .args(["init", "--crypto-provider", "recording"])
        .output()
        .unwrap();
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    command
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &config_path)
        .env("MAOS_KMS_MASTER_KEY", "42".repeat(32))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut root = Reap(command.spawn().unwrap());
    let stderr = root.0.stderr.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    let mut diagnostics = String::new();
    let listen_deadline = std::time::Instant::now() + Duration::from_secs(45);
    let port = loop {
        let remaining = listen_deadline.saturating_duration_since(std::time::Instant::now());
        let line = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|_| panic!("daemon not listening: {diagnostics}"));
        diagnostics.push_str(&line);
        diagnostics.push('\n');
        if let Some(addr) = line.split("cohort-a2a-daemon listening on ").nth(1) {
            break addr
                .trim()
                .rsplit(':')
                .next()
                .unwrap()
                .parse::<u16>()
                .unwrap();
        }
    };
    let identity_path = home.join("crypto-recording-identity.log");
    let before_crl = std::fs::read_to_string(&identity_path).unwrap_or_default();
    let boot_verifies = before_crl
        .lines()
        .filter(|line| line.ends_with(" verify_signature"))
        .count();
    let mut import = Command::new(&ctl);
    env(&mut import);
    let output = import
        .args(["revocations", "import"])
        .arg(&crl_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let log = std::fs::read_to_string(&identity_path).unwrap();
        if log
            .lines()
            .filter(|line| line.ends_with(" verify_signature"))
            .count()
            > boot_verifies
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "CRL was not verified: {log}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let pins = Arc::new(InMemoryTofuPinStore::new());
    pins.pin_first_contact(&PeerId::new("host-a"), &server_fp, &server_fp, 0)
        .await
        .unwrap();
    let client = build_client_config(
        std::slice::from_ref(&client_der),
        &client_key,
        pins,
        TrustPosture::LeafSelfAnchor,
        None,
        None,
    )
    .unwrap();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let stream = tokio_rustls::TlsConnector::from(Arc::new(client))
        .connect(ServerName::IpAddress(addr.ip().into()), tcp)
        .await
        .unwrap();
    let mut framed = Framed::new(stream, length_delimited_codec());
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-digest"),
        host_id: Some(HostId("host-b".into())),
        role: None,
    };
    let mut to = smallvec::SmallVec::new();
    to.push(FrameAddress {
        spirit_id: SpiritId::from("researcher"),
        host_id: Some(HostId("host-a".into())),
        role: None,
    });
    let control = DigestReadControl::Request {
        request_id: "crypto-real-boot".into(),
        scope: "daily".into(),
    };
    let frame = IacFrame {
        frame_id: [1; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to,
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(control.telemetry_payload().unwrap()),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(COHORT_INTENT_DIGEST_READ),
        )),
        intent_lineage: IntentLineage::default(),
    };
    let request = A2AJsonRpcRequest::new("iac.deliver", frame, 1)
        .with_boot_nonce(135001)
        .with_cohort_acting_role("worker")
        .with_cohort_manifest_version(1);
    framed
        .send(serde_json::to_vec(&request).unwrap().into())
        .await
        .unwrap();
    let response = tokio::time::timeout(Duration::from_secs(10), framed.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert!(
        response.get("error").is_none() || response["error"].is_null(),
        "{response}"
    );
    let operations = std::fs::read_to_string(home.join("crypto-recording-identity.log")).unwrap();
    let rows: Vec<_> = operations
        .lines()
        .map(|line| line.split_once(' ').unwrap())
        .collect();
    assert!(rows.iter().all(|(id, _)| *id == rows[0].0), "{operations}");
    for operation in [
        "verify_signature",
        "sign_capability_token",
        "seal_for_export",
    ] {
        assert!(rows.iter().any(|(_, op)| *op == operation), "{operations}");
    }
    assert!(
        rows.iter()
            .filter(|(_, op)| *op == "verify_signature")
            .count()
            > boot_verifies,
        "{operations}"
    );
}
