#[cfg(feature = "crypto-recording")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn crypto_provider_one_boot_reaches_all_four_surfaces() {
    use maos_control::OperatorCommandPort;
    use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
    use maos_domain::ports::CapabilityRegistryPort;
    use maos_domain::revocation::{
        canonical_entries_bytes, CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
    };
    use maos_spirit_abi::compliance::{
        CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::{Read, Write};

    let _guard = env_lock();
    let mut fixture = fixture("crypto-integrated");
    std::fs::write(
        fixture.home.join("config.toml"),
        "[crypto]\nprovider = \"recording\"\n",
    )
    .unwrap();
    set_process_env(&fixture);
    let selected = maos_bin::crypto_provider::select(Some(&fixture.home)).unwrap();
    let world = Arc::new(crate::crypto_test_world::kernel_world_with_crypto(
        DAEMON_BOOT_NONCE,
        Arc::clone(&selected.provider),
    ));
    fixture.audit_db = world.dir.path().join("transparency.sqlite");
    let scheduler_world = Arc::clone(&world);
    let scheduler = tokio::task::spawn_blocking(move || {
        crate::crypto_test_world::scheduler_with_hello(&scheduler_world, DAEMON_BOOT_NONCE)
    })
    .await
    .unwrap();
    let security = Arc::new(maos_kernel_core::security::SecurityManagerAdapter::new(
        Arc::clone(&world.policy),
    ));

    // The signed package exists before this root's installed-boot pass.
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let manifest = b"[spirit]\nname = \"crypto-integrated\"\nversion = \"1.0.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n".to_vec();
    let artifact = b"installed component".to_vec();
    let mut hash = Sha256::new();
    hash.update((manifest.len() as u64).to_le_bytes());
    hash.update(&manifest);
    hash.update((artifact.len() as u64).to_le_bytes());
    hash.update(&artifact);
    let signature: [u8; 64] = key.sign(&hash.finalize()).as_ref().try_into().unwrap();
    let fields = maos_compliance::runtime_context::extract_manifest_fingerprint_fields(&manifest);
    let fingerprint = ExecutionContextFingerprint {
        manifest_hash: Sha256::digest(&manifest).into(),
        spirit_version: "1.0.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    assert_eq!(fields.crypto_provider, fingerprint.crypto_provider);
    let claim = maos_compliance::builder::build_signed_envelope(
        maos_compliance::builder::encode_claim_bytes(&fingerprint, None),
        &key,
        public,
    );
    let package = SignedPackage::new(
        SpiritId::from("crypto-integrated"),
        "1.0.0".into(),
        manifest.clone(),
        artifact.clone(),
        signature,
        public,
        claim,
    );
    let installed = fixture.home.join("installed");
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(installed.join("manifest.toml"), manifest).unwrap();
    std::fs::write(installed.join("artifact.bin"), artifact).unwrap();
    std::fs::write(
        installed.join("signed_package.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
    let pids = Arc::new(std::sync::RwLock::new(BTreeMap::new()));
    let deps = maos_bin::admission::AdmissionDeps {
        scheduler: &scheduler,
        security: &security,
        transparency_log: &world.transparency_log,
        journal: &world.journal,
        memory: &world.memory,
        pid_by_spirit_id: &pids,
        hooks: None,
        providers: None,
        iac: None,
    };
    let _installed = maos_bin::installed_spirits::InstalledSpirits::boot(
        vec![("crypto-integrated".into(), installed.join("manifest.toml"))],
        &deps,
        &selected,
        DAEMON_BOOT_NONCE,
        #[cfg(feature = "wasm-host")]
        None,
        #[cfg(feature = "wasm-host")]
        None,
    )
    .await;
    let identity_log = fixture.home.join("crypto-recording-identity.log");
    let boot_ops = std::fs::read_to_string(&identity_log).unwrap();
    assert_eq!(
        boot_ops
            .lines()
            .filter(|line| line.ends_with(" verify_signature"))
            .count(),
        2,
        "{boot_ops}"
    );

    let pid = scheduler.resolve_pid("hello-spirit").unwrap();
    let posture = [1u8; 32];
    let token = world
        .capability
        .issue(
            pid,
            Scope::ProviderInfer {
                provider: "anthropic".into(),
            },
            60,
            posture,
            IntentClass::Standard,
        )
        .unwrap();
    world
        .capability
        .verify(&token, posture, maos_domain::invariants::i9::SandboxTier(0))
        .unwrap();
    let before_crl = std::fs::read_to_string(&identity_log).unwrap();
    let prior_verifications = before_crl
        .lines()
        .filter(|line| line.ends_with(" verify_signature"))
        .count();

    let crl_key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let crl_public: [u8; 32] = crl_key.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![RevocationEntry::new("never-matches", ">=0.1.0,<0.2.0", "compromised", None).unwrap()];
    let crl_signature: [u8; 64] = crl_key
        .sign(&canonical_entries_bytes(&entries).unwrap())
        .as_ref()
        .try_into()
        .unwrap();
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).unwrap(),
        1,
        0,
        RevocationOrigin::Operator,
        entries,
        crl_signature,
        crl_public,
    )
    .unwrap();
    let door = crate::crypto_test_world::regression_door_with_crypto(
        &world,
        &scheduler,
        tokio::runtime::Handle::current(),
        DAEMON_BOOT_NONCE,
        Arc::new(crate::crypto_test_world::UnwiredBinOps::default()),
        Arc::clone(&selected.provider),
        Some(crl_public.to_vec()),
    );
    let bearer = maos_domain::operator_door::mint_token_hex();
    let mut server = maos_control::OperatorHttpServer::bind_with_commands(
        maos_control::OperatorHttpConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            bearer_token: bearer.clone(),
        },
        Arc::clone(&scheduler),
        None,
        None,
        None,
        Some(door.clone()),
    )
    .unwrap();
    let address = server.local_addr();
    let body = serde_json::to_vec(&crl).unwrap();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
    let ports = RecordingPorts::new(PolicyVerdict::Allow);
    let daemon = boot_and_drive_with_shared(
        &fixture,
        TestWiring::Full,
        &ports,
        Some(SharedBoot {
            log: Arc::clone(&world.transparency_log),
            capability: Arc::clone(&world.capability),
            security,
            crypto: Arc::clone(&selected.provider),
            ready: ready_tx,
            resume: resume_rx,
        }),
    );
    let door_action = async {
        ready_rx.await.expect("daemon did not become ready");
        let response = tokio::task::spawn_blocking(move || {
                let mut stream = std::net::TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
                write!(stream, "POST /v1/revocations HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {bearer}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                response
            }).await.unwrap();
        assert!(
            response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.1 202"),
            "{response}"
        );
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while door.applied_revocations().is_empty() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("CRL import did not complete through the live door");
        let before_seal = std::fs::read_to_string(&identity_log).unwrap();
        assert_eq!(
            before_seal
                .lines()
                .filter(|line| line.ends_with(" verify_signature"))
                .count(),
            prior_verifications + 1,
            "{before_seal}"
        );
        resume_tx.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(daemon, door_action);
    result.unwrap();
    server.shutdown();
    let operations = std::fs::read_to_string(identity_log).unwrap();
    let rows: Vec<_> = operations
        .lines()
        .map(|line| line.split_once(' ').unwrap())
        .collect();
    let identity = rows[0].0;
    assert!(
        rows.iter().all(|(instance, _)| *instance == identity),
        "{operations}"
    );
    for operation in [
        "verify_signature",
        "sign_capability_token",
        "seal_for_export",
    ] {
        assert!(
            rows.iter().any(|(_, actual)| *actual == operation),
            "{operations}"
        );
    }
}
