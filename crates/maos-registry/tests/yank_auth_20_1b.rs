#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::{SpiritId, YankOrigin};
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage, StorageError};
use support::*;
use trust::*;
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let pkg = package("owner", "1.0.0", &PUBLISHER);
    let mut unsigned = pkg.clone();
    unsigned.signature = [0; 64];
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(unsigned).unwrap()
    )
    .get("error")
    .is_some());
    assert!(!scratch.0.join("registry/spirits/owner/1.0.0").exists());
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(&pkg).unwrap()
    )
    .get("result")
    .is_some());
    let stored = std::fs::read(
        scratch
            .0
            .join("registry/spirits/owner/1.0.0/signed_package.json"),
    )
    .unwrap();
    for params in [
        serde_json::to_value(package("owner", "1.0.0", &[0x54; 32])).unwrap(),
        serde_json::to_value(package("owner", "2.0.0", &[0x54; 32])).unwrap(),
    ] {
        assert!(rpc(&server.addr, "registry.publish", params)
            .get("error")
            .is_some());
    }
    for params in [
        serde_json::json!({"spirit_id":"owner","version":"1.0.0","reason":"unauthenticated"}),
        yank(
            &server.addr,
            "owner",
            "1.0.0",
            &[0x54; 32],
            YankOrigin::Publisher,
        ),
        yank(&server.addr, "owner", "1.0.0", &VETTER, YankOrigin::Vetter),
    ] {
        assert!(rpc(&server.addr, "registry.deprecate", params)
            .get("error")
            .is_some());
    }
    assert_eq!(
        std::fs::read(
            scratch
                .0
                .join("registry/spirits/owner/1.0.0/signed_package.json")
        )
        .unwrap(),
        stored
    );
    assert!(!scratch.0.join("registry/spirits/owner/2.0.0").exists());
    assert!(!scratch.0.join("registry/yanks.json").exists());
}

#[test]
fn first_publisher_pin_is_typed_and_survives_restart_ac10() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let id = SpiritId::from("pinned");
    {
        let storage = LocalFsRegistryStorage::at_path(scratch.0.join("registry")).unwrap();
        storage
            .put(&id, "1.0.0", &package("pinned", "1.0.0", &PUBLISHER))
            .unwrap();
    }
    let storage = LocalFsRegistryStorage::at_path(scratch.0.join("registry")).unwrap();
    assert!(matches!(
        storage.put(&id, "2.0.0", &package("pinned", "2.0.0", &VETTER)),
        Err(StorageError::PublisherMismatch(_))
    ));
    storage
        .put(&id, "2.0.0", &package("pinned", "2.0.0", &PUBLISHER))
        .unwrap();
}

#[test]
fn publisher_and_enrolled_vetter_yanks_are_signed_labelled_and_restart_safe_ac7() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let root = public(&OPERATOR);
    let server = InProcessServer::start_config(&scratch, None, Some((keyring(), root)));
    for version in ["1.0.0", "2.0.0"] {
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(package("withdraw", "1.0.0", &PUBLISHER)).unwrap()
        )
        .get("result")
        .is_some());
        if version == "2.0.0" {
            assert!(rpc(
                &server.addr,
                "registry.publish",
                serde_json::to_value(package("withdraw", version, &PUBLISHER)).unwrap()
            )
            .get("result")
            .is_some());
        }
    }
    let first = yank(
        &server.addr,
        "withdraw",
        "1.0.0",
        &PUBLISHER,
        YankOrigin::Publisher,
    );
    assert!(rpc(&server.addr, "registry.deprecate", first.clone())
        .get("result")
        .is_some());
    assert!(rpc(&server.addr, "registry.deprecate", first)
        .get("result")
        .is_some());
    let initial = rpc(
        &server.addr,
        "registry.yanks_since",
        serde_json::json!({"since_ns":0}),
    );
    assert_eq!(initial["result"]["entries"].as_array().unwrap().len(), 1);
    let cursor = initial["result"]["entries"][0]["sequence"]
        .as_u64()
        .unwrap();
    assert_eq!(cursor, 1);
    drop(server);
    let server = InProcessServer::start_config(&scratch, None, Some((keyring(), root)));
    assert!(rpc(
        &server.addr,
        "registry.deprecate",
        yank(
            &server.addr,
            "withdraw",
            "2.0.0",
            &VETTER,
            YankOrigin::Vetter
        )
    )
    .get("result")
    .is_some());
    let list = rpc(
        &server.addr,
        "registry.yanks_since",
        serde_json::json!({"since_ns":cursor}),
    );
    let entries: Vec<maos_domain::ports::registry::YankEntry> =
        serde_json::from_value(list["result"]["entries"].clone()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].sequence, 2);
    assert_eq!(entries[0].origin, Some(YankOrigin::Vetter));
    maos_registry::yank_auth::verify_entry(
        &entries[0],
        &public(&PUBLISHER),
        Some((&keyring(), &root)),
        &format!("http://{}/mcp", server.addr),
        maos_registry::yank_auth::now_unix_ms().unwrap(),
    )
    .unwrap();
    let mut tampered = entries[0].clone();
    tampered.reason.push('!');
    assert!(maos_registry::yank_auth::verify_entry(
        &tampered,
        &public(&PUBLISHER),
        Some((&keyring(), &root)),
        &format!("http://{}/mcp", server.addr),
        maos_registry::yank_auth::now_unix_ms().unwrap()
    )
    .is_err());
}

#[test]
fn daemon_cursor_restart_does_not_replay_journalled_yanks_ac7() {
    let _env = ENV.lock().unwrap();
    use maos_domain::ports::registry::{RegistryError, YankEntry};
    use maos_registry::yank::{YankObserver, YankPoller};
    use std::sync::{Arc, Mutex};
    struct Journal(Mutex<Vec<u64>>, String);
    impl YankObserver for Journal {
        fn on_yank(
            &self,
            entry: &YankEntry,
        ) -> Result<maos_registry::yank::YankDisposition, RegistryError> {
            maos_registry::yank_auth::verify_entry(
                entry,
                &public(&PUBLISHER),
                None,
                &self.1,
                maos_registry::yank_auth::now_unix_ms().unwrap(),
            )
            .map_err(RegistryError::Transport)?;
            self.0.lock().unwrap().push(entry.sequence);
            Ok(maos_registry::yank::YankDisposition::Verified)
        }
    }
    let scratch = Scratch::new();
    std::env::set_var(
        "MAOS_REGISTRY_YANK_CURSOR_PATH",
        scratch.0.join("cursor.json"),
    );
    let server = InProcessServer::start(&scratch);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(package("cursor", "1.0.0", &PUBLISHER)).unwrap()
    )
    .get("result")
    .is_some());
    assert!(rpc(
        &server.addr,
        "registry.deprecate",
        yank(
            &server.addr,
            "cursor",
            "1.0.0",
            &PUBLISHER,
            YankOrigin::Publisher
        )
    )
    .get("result")
    .is_some());
    let client = Arc::new(
        maos_registry::McpSpiritRegistryClient::http(
            format!("http://{}/mcp", server.addr),
            Default::default(),
        )
        .unwrap(),
    );
    let journal = Arc::new(Journal(
        Mutex::new(vec![]),
        format!("http://{}/mcp", server.addr),
    ));
    let poller = YankPoller::new(
        client.clone(),
        journal.clone(),
        format!("http://{}/mcp", server.addr),
    );
    assert_eq!(poller.poll_once().unwrap(), 1);
    poller.save_cursor().unwrap();
    drop(poller);
    let restarted = YankPoller::new(
        client,
        journal.clone(),
        format!("http://{}/mcp", server.addr),
    );
    restarted.load_cursor().unwrap();
    assert_eq!(restarted.last_seen_ns(), 1);
    assert_eq!(restarted.poll_once().unwrap(), 0);
    assert_eq!(*journal.0.lock().unwrap(), vec![1]);
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
}

#[test]
fn installed_replacement_and_yanked_install_are_refused_before_commit_ac14_ac16() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    for name in [
        "MAOS_REGISTRY_TIER_FLOOR",
        "MAOS_REGISTRY_ORG_SIGNING_PUBKEY",
        "MAOS_REGISTRY_REQUIRE_SERVER_TIER_SIGNATURE",
    ] {
        std::env::remove_var(name);
    }
    let server = InProcessServer::start(&scratch);
    let uri = format!("http://{}/mcp", server.addr);
    let mut pkg = package("immutable", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(&pkg).unwrap()
    )
    .get("result")
    .is_some());
    maos_registry::install::install("immutable@1.0.0", &uri, None, None).unwrap();
    let root = scratch.0.join("home/.local/share/maos/registry");
    let index = std::fs::read(root.join("installed-index.json")).unwrap();
    // An identical inactive version reuses its immutable generation, rather
    // than reporting the requested version while another one remains active.
    let next = package("immutable", "1.0.1", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(next).unwrap()
    )
    .get("result")
    .is_some());
    maos_registry::install::install("immutable@1.0.1", &uri, None, None).unwrap();
    maos_registry::install::install("immutable@1.0.0", &uri, None, None).unwrap();
    assert_eq!(
        std::fs::read(root.join("installed-index.json")).unwrap(),
        index
    );
    pkg.artifact_bytes.push(1);
    sign(&mut pkg, &PUBLISHER);
    let version = scratch.0.join("registry/spirits/immutable/1.0.0");
    // Hostile feed mutation, not a successful same-version publish.
    std::fs::write(
        version.join("signed_package.json"),
        serde_json::to_vec(&pkg).unwrap(),
    )
    .unwrap();
    std::fs::write(version.join("artifact.bin"), &pkg.artifact_bytes).unwrap();
    assert!(
        maos_registry::install::install("immutable@1.0.0", &uri, None, None)
            .unwrap_err()
            .to_string()
            .contains("immutable")
    );
    assert_eq!(
        std::fs::read(root.join("installed-index.json")).unwrap(),
        index
    );
    let pkg = package("withdrawn", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    assert!(rpc(
        &server.addr,
        "registry.deprecate",
        yank(
            &server.addr,
            "withdrawn",
            "1.0.0",
            &PUBLISHER,
            YankOrigin::Publisher
        )
    )
    .get("result")
    .is_some());
    assert!(
        maos_registry::install::install("withdrawn@1.0.0", &uri, None, None)
            .unwrap_err()
            .to_string()
            .contains("yanked")
    );
    assert!(!root.join("installed/withdrawn").exists());
}

#[test]
fn signed_revocation_lists_roundtrip_cbor_and_reject_wrong_byte_lengths() {
    let _env = ENV.lock().unwrap();
    use maos_domain::revocation::*;
    use ring::signature::Ed25519KeyPair;
    let entries = vec![RevocationEntry::new("revoked", "=1.0.0", "withdrawn", None).unwrap()];
    let key = Ed25519KeyPair::from_seed_unchecked(&OPERATOR).unwrap();
    let signature = key
        .sign(&canonical_entries_bytes(&entries).unwrap())
        .as_ref()
        .try_into()
        .unwrap();
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).unwrap(),
        1,
        100,
        RevocationOrigin::Operator,
        entries,
        signature,
        public(&OPERATOR),
    )
    .unwrap();
    let bytes = serde_cbor::to_vec(&crl).unwrap();
    assert_eq!(
        serde_cbor::from_slice::<SignedRevocationList>(&bytes).unwrap(),
        crl
    );
    for (field, length) in [("signature", 63), ("signer_pub_key", 31)] {
        let mut value = serde_cbor::value::to_value(&crl).unwrap();
        let serde_cbor::Value::Map(fields) = &mut value else {
            panic!("CRL must encode a map")
        };
        assert!(fields
            .insert(
                serde_cbor::Value::Text(field.into()),
                serde_cbor::Value::Bytes(vec![1; length])
            )
            .is_some());
        assert!(serde_cbor::value::from_value::<SignedRevocationList>(value).is_err());
    }
}

#[test]
fn published_versions_are_immutable_and_identical_publish_is_idempotent_d3() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let storage = LocalFsRegistryStorage::at_path(scratch.0.join("registry")).unwrap();
    let mut pkg = package("published", "1.0.0", &PUBLISHER);
    storage.put(&pkg.spirit_id, &pkg.version, &pkg).unwrap();
    storage.put(&pkg.spirit_id, &pkg.version, &pkg).unwrap();
    let original = std::fs::read(
        scratch
            .0
            .join("registry/spirits/published/1.0.0/artifact.bin"),
    )
    .unwrap();
    pkg.artifact_bytes.push(1);
    sign(&mut pkg, &PUBLISHER);
    assert!(
        matches!(
            storage.put(&pkg.spirit_id, &pkg.version, &pkg),
            Err(StorageError::ImmutableVersion { .. })
        ),
        "changed published bytes must be refused"
    );
    assert_eq!(
        std::fs::read(
            scratch
                .0
                .join("registry/spirits/published/1.0.0/artifact.bin")
        )
        .unwrap(),
        original
    );
}

#[test]
fn installed_publisher_and_version_pins_survive_uninstall_d2() {
    let _env = ENV.lock().unwrap();
    for changed_publisher in [false, true] {
        let scratch = Scratch::new();
        std::env::set_var("HOME", scratch.0.join("home"));
        let server = InProcessServer::start(&scratch);
        let uri = format!("http://{}/mcp", server.addr);
        let original = package("durable", "1.0.0", &PUBLISHER);
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(&original).unwrap()
        )
        .get("result")
        .is_some());
        maos_registry::install::install("durable@1.0.0", &uri, None, None).unwrap();
        let mut removal = LocalFsRegistryStorage::uninstall_package("durable")
            .unwrap()
            .unwrap();
        removal.delete_generation().unwrap();
        removal.accept();
        let replacement = Scratch::new();
        let other = InProcessServer::start(&replacement);
        let mut pkg = original.clone();
        let signer = if changed_publisher {
            &VETTER
        } else {
            &PUBLISHER
        };
        pkg.artifact_bytes.push(1);
        sign(&mut pkg, signer);
        assert!(rpc(
            &other.addr,
            "registry.publish",
            serde_json::to_value(pkg).unwrap()
        )
        .get("result")
        .is_some());
        let result = maos_registry::install::install(
            "durable@1.0.0",
            &format!("http://{}/mcp", other.addr),
            None,
            None,
        );
        assert!(result.is_err(), "uninstall must not release the durable publisher/version pin (changed_publisher={changed_publisher})");
    }
}

#[test]
fn renewed_promotion_replaces_only_evidence_for_identical_package_d4() {
    use std::os::unix::fs::PermissionsExt;
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let key = scratch.0.join("operator.key");
    std::fs::write(&key, hex::encode(OPERATOR)).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::env::set_var("MAOS_AUDIT_KEY", &key);
    let server = InProcessServer::start(&scratch);
    let uri = format!("http://{}/mcp", server.addr);
    let pkg = package("renewal", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(&pkg).unwrap()
    )
    .get("result")
    .is_some());
    let keyring = scratch.0.join("vetters.cbor");
    let attestation = scratch.0.join("vetting.cbor");
    std::fs::write(&keyring, serde_cbor::to_vec(&trust::keyring()).unwrap()).unwrap();
    let now = maos_registry::yank_auth::now_unix_ms().unwrap();
    for expires in [now + 60_000, now + 120_000] {
        std::fs::write(
            &attestation,
            serde_cbor::to_vec(&trust::attestation(&pkg, now, expires)).unwrap(),
        )
        .unwrap();
        let result = maos_registry::install::install(
            "renewal@1.0.0",
            &uri,
            Some(&attestation),
            Some(&keyring),
        );
        assert!(
            result.is_ok(),
            "identical-package promotion renewal must succeed: {result:?}"
        );
    }
    std::env::remove_var("MAOS_AUDIT_KEY");
}

#[test]
fn signed_yank_resigning_does_not_append_duplicate_rows_p13() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let pkg = package("dedupe", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    for _ in 0..3 {
        let authorization = maos_registry::yank_auth::YankAuthorization::sign(
            &PUBLISHER,
            maos_registry::yank_auth::YankScope {
                name: "dedupe",
                version: "1.0.0",
                reason: "withdrawn",
                registry_uri: &format!("http://{}/mcp", server.addr),
            },
            YankOrigin::Publisher,
            String::new(),
            maos_registry::yank_auth::now_unix_ms().unwrap(),
        )
        .unwrap();
        let response = rpc(
            &server.addr,
            "registry.deprecate",
            serde_json::json!({"spirit_id":"dedupe","version":"1.0.0","reason":"withdrawn","authorization":authorization}),
        );
        assert!(response.get("result").is_some(), "{response}");
    }
    let response = rpc(
        &server.addr,
        "registry.yanks_since",
        serde_json::json!({"since_ns":0}),
    );
    assert_eq!(
        response["result"]["entries"].as_array().unwrap().len(),
        1,
        "re-signing must reuse the original receipt"
    );
}

#[test]
fn unreadable_promotion_is_not_treated_as_absent_p17() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let server = InProcessServer::start(&scratch);
    let pkg = package("unreadable", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    maos_registry::install::install(
        "unreadable@1.0.0",
        &format!("http://{}/mcp", server.addr),
        None,
        None,
    )
    .unwrap();
    let manifest = LocalFsRegistryStorage::default_installed_manifests()
        .unwrap()
        .remove(0)
        .1;
    let metadata = manifest.parent().unwrap().join("vetting.json");
    std::os::unix::fs::symlink("vetting.json", &metadata).unwrap();
    assert!(
        maos_registry::install::verify_installed("unreadable", &manifest).is_err(),
        "ELOOP is an unreadable promotion, never missing evidence"
    );
}

#[test]
fn uninstall_waits_for_cross_process_mutation_lock_p8() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let server = InProcessServer::start(&scratch);
    let pkg = package("locked", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    maos_registry::install::install(
        "locked@1.0.0",
        &format!("http://{}/mcp", server.addr),
        None,
        None,
    )
    .unwrap();
    let file = std::fs::File::open(
        scratch
            .0
            .join("home/.local/share/maos/registry/.mutation.lock"),
    )
    .unwrap();
    rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive).unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let mut removal = LocalFsRegistryStorage::uninstall_package("locked")
            .unwrap()
            .unwrap();
        removal.delete_generation().unwrap();
        removal.accept();
        done_tx.send(()).unwrap();
    });
    started_rx.recv().unwrap();
    let result = done_rx.recv_timeout(std::time::Duration::from_millis(300));
    drop(file);
    worker.join().unwrap();
    assert!(
        matches!(result, Err(std::sync::mpsc::RecvTimeoutError::Timeout)),
        "uninstall escaped the held mutation lock: {result:?}"
    );
}

#[test]
fn yank_authorization_allows_bounded_clock_skew_p10() {
    let _env = ENV.lock().unwrap();
    let now = 1_000_000;
    for (ahead, accepted) in [(1_000, true), (3_600_000, false)] {
        let scope = maos_registry::yank_auth::YankScope {
            name: "clock",
            version: "1.0.0",
            reason: "withdrawn",
            registry_uri: "http://127.0.0.1:20101/mcp",
        };
        let bytes = maos_registry::yank_auth::YankAuthorization::sign(
            &PUBLISHER,
            scope,
            YankOrigin::Publisher,
            String::new(),
            now + ahead,
        )
        .unwrap();
        let auth = maos_registry::yank_auth::decode(&bytes).unwrap();
        assert_eq!(
            auth.verify(scope, &public(&PUBLISHER), None, now).is_ok(),
            accepted,
            "clock skew {ahead} ms"
        );
    }
}

#[test]
fn missing_home_never_selects_shared_temporary_cursor_p14() {
    let _env = ENV.lock().unwrap();
    let home = std::env::var_os("HOME");
    let cursor = std::env::var_os("MAOS_REGISTRY_YANK_CURSOR_PATH");
    std::env::remove_var("HOME");
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    let path = maos_registry::yank::cursor_file_path();
    if let Some(value) = home {
        std::env::set_var("HOME", value);
    }
    if let Some(value) = cursor {
        std::env::set_var("MAOS_REGISTRY_YANK_CURSOR_PATH", value);
    }
    assert!(
        path.is_err(),
        "no HOME must refuse cursor persistence, not select shared /tmp"
    );
}

#[test]
fn local_reevaluation_failure_does_not_prevent_fetch_p1() {
    use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
    use maos_registry::yank::{YankObserver, YankPoller, YankSource};
    use std::sync::{Arc, Mutex};
    let _env = ENV.lock().unwrap();
    struct Source;
    impl YankSource for Source {
        fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
            let mut entry = YankEntry::new("poll".into(), "1.0.0".into(), 0, "withdrawn".into());
            entry.sequence = 1;
            Ok(YankList::new(vec![entry]))
        }
    }
    struct Observer(Mutex<Vec<u64>>);
    impl YankObserver for Observer {
        fn on_yank(
            &self,
            entry: &YankEntry,
        ) -> Result<maos_registry::yank::YankDisposition, RegistryError> {
            self.0.lock().unwrap().push(entry.sequence);
            Ok(maos_registry::yank::YankDisposition::Verified)
        }
        fn on_poll(&self) -> Result<(), RegistryError> {
            Err(RegistryError::Transport(
                "one corrupt local promotion".into(),
            ))
        }
    }
    let observer = Arc::new(Observer(Mutex::new(Vec::new())));
    let poller = YankPoller::new(
        Arc::new(Source),
        observer.clone(),
        "fixture://poll-failure".into(),
    );
    assert_eq!(poller.poll_once().unwrap(), 1);
    assert_eq!(*observer.0.lock().unwrap(), vec![1]);
}

#[test]
fn signed_yank_cannot_be_replayed_to_another_registry_f4() {
    let _env = ENV.lock().unwrap();
    let first = Scratch::new();
    let second = Scratch::new();
    let a = InProcessServer::start(&first);
    let b = InProcessServer::start(&second);
    let pkg = package("audience", "1.0.0", &PUBLISHER);
    for server in [&a, &b] {
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(&pkg).unwrap()
        )
        .get("result")
        .is_some());
    }
    let authorization = yank(
        &a.addr,
        "audience",
        "1.0.0",
        &PUBLISHER,
        YankOrigin::Publisher,
    );
    let response = rpc(&b.addr, "registry.deprecate", authorization);
    assert!(
        response.get("error").is_some(),
        "registry A authorization must not mutate registry B: {response}"
    );
    assert!(!second.0.join("registry/yanks.json").exists());
}

#[test]
fn expired_signed_yank_request_cannot_mutate_registry_f4() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let pkg = package("stale", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    let issued = maos_registry::yank_auth::now_unix_ms().unwrap() - 3_600_000;
    let authorization = maos_registry::yank_auth::YankAuthorization::sign(
        &PUBLISHER,
        maos_registry::yank_auth::YankScope {
            name: "stale",
            version: "1.0.0",
            reason: "withdrawn",
            registry_uri: &format!("http://{}/mcp", server.addr),
        },
        YankOrigin::Publisher,
        String::new(),
        issued,
    )
    .unwrap();
    let response = rpc(
        &server.addr,
        "registry.deprecate",
        serde_json::json!({"spirit_id":"stale","version":"1.0.0","reason":"withdrawn","authorization":authorization}),
    );
    assert!(
        response.get("error").is_some(),
        "an hour-old mutation authorization must expire: {response}"
    );
    assert!(!scratch.0.join("registry/yanks.json").exists());
}

#[test]
fn install_refuses_present_yank_with_unpinned_authority_f2() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let server = InProcessServer::start(&scratch);
    let uri = format!("http://{}/mcp", server.addr);
    let pkg = package("forged-feed", "1.0.0", &PUBLISHER);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
    let authorization = yank(
        &server.addr,
        "forged-feed",
        "1.0.0",
        &VETTER,
        YankOrigin::Publisher,
    );
    let mut entry = maos_domain::ports::registry::YankEntry::new(
        "forged-feed".into(),
        "1.0.0".into(),
        0,
        "withdrawn".into(),
    );
    entry.sequence = 1;
    entry.origin = Some(YankOrigin::Publisher);
    entry.authorization = serde_json::from_value(authorization["authorization"].clone()).unwrap();
    // Deliberately hostile feed fixture; the mutation endpoint rejects this signer.
    std::fs::write(
        scratch.0.join("registry/yanks.json"),
        serde_json::to_vec(&vec![entry]).unwrap(),
    )
    .unwrap();
    let error = maos_registry::install::install("forged-feed@1.0.0", &uri, None, None)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unauthorized publisher"),
        "a forged entry must not be accepted as an authenticated yank: {error}"
    );
    assert!(!scratch
        .0
        .join("home/.local/share/maos/registry/installed-index.json")
        .exists());
}

#[test]
fn unsigned_sequence_overshoot_cannot_hide_next_signed_yank_p2() {
    use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
    use maos_registry::yank::{YankObserver, YankPoller, YankSource};
    use std::sync::{Arc, Mutex};
    let _env = ENV.lock().unwrap();
    struct Source(Vec<YankEntry>);
    impl YankSource for Source {
        fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
            Ok(YankList::new(self.0.clone()))
        }
    }
    struct Observer(Mutex<Vec<u64>>);
    impl YankObserver for Observer {
        fn on_yank(
            &self,
            entry: &YankEntry,
        ) -> Result<maos_registry::yank::YankDisposition, RegistryError> {
            maos_registry::yank_auth::verify_entry(
                entry,
                &public(&PUBLISHER),
                None,
                "http://127.0.0.1:20101/mcp",
                maos_registry::yank_auth::now_unix_ms().unwrap(),
            )
            .map_err(RegistryError::Transport)?;
            self.0.lock().unwrap().push(entry.sequence);
            Ok(maos_registry::yank::YankDisposition::Verified)
        }
    }
    let mut valid = YankEntry::new("sequence".into(), "1.0.0".into(), 0, "withdrawn".into());
    valid.sequence = 1;
    valid.origin = Some(YankOrigin::Publisher);
    valid.authorization = maos_registry::yank_auth::YankAuthorization::sign(
        &PUBLISHER,
        maos_registry::yank_auth::YankScope {
            name: "sequence",
            version: "1.0.0",
            reason: "withdrawn",
            registry_uri: "http://127.0.0.1:20101/mcp",
        },
        YankOrigin::Publisher,
        String::new(),
        maos_registry::yank_auth::now_unix_ms().unwrap(),
    )
    .unwrap();
    let mut overshoot = valid.clone();
    overshoot.sequence = u64::MAX; // Server sequence is NOT in the publisher signature.
    let observer = Arc::new(Observer(Mutex::new(Vec::new())));
    let poller = YankPoller::new(
        Arc::new(Source(vec![overshoot, valid])),
        observer.clone(),
        "http://127.0.0.1:20101/mcp".into(),
    );
    poller.poll_once().unwrap();
    assert_eq!(
        *observer.0.lock().unwrap(),
        vec![1],
        "an unsigned sequence must not poison the authenticated cursor"
    );
    assert_eq!(poller.last_seen_ns(), 1);
}

#[test]
fn cursor_from_another_registry_is_replayed_from_zero_p2() {
    use maos_domain::ports::registry::{RegistryError, YankEntry};
    use maos_registry::yank::{YankObserver, YankPoller};
    let _env = ENV.lock().unwrap();
    struct Observer;
    impl YankObserver for Observer {
        fn on_yank(
            &self,
            _: &YankEntry,
        ) -> Result<maos_registry::yank::YankDisposition, RegistryError> {
            Ok(maos_registry::yank::YankDisposition::Verified)
        }
    }
    let scratch = Scratch::new();
    std::env::set_var(
        "MAOS_REGISTRY_YANK_CURSOR_PATH",
        scratch.0.join("cursor.json"),
    );
    std::env::set_var("MAOS_REGISTRY_URI", "http://127.0.0.1:20101/mcp");
    maos_registry::yank::save_cursor("http://127.0.0.1:20101/mcp", 500).unwrap();
    std::env::set_var("MAOS_REGISTRY_URI", "http://127.0.0.1:20102/mcp");
    let poller = YankPoller::new(
        std::sync::Arc::new(maos_registry::client::NullSpiritRegistryClient),
        std::sync::Arc::new(Observer),
        "http://127.0.0.1:20102/mcp".into(),
    );
    poller.load_cursor().unwrap();
    std::env::remove_var("MAOS_REGISTRY_URI");
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    assert_eq!(
        poller.last_seen_ns(),
        0,
        "a registry A cursor must not blind registry B"
    );
}

#[test]
fn zero_interval_cannot_silently_disable_local_vetting_p11() {
    let _env = ENV.lock().unwrap();
    std::env::set_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "0");
    let interval = maos_registry::yank::resolve_poll_interval();
    std::env::remove_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S");
    assert!(
        interval.is_err(),
        "zero cadence must be refused, not silently disable promotion expiry"
    );
}

#[test]
fn blocked_registry_fetch_does_not_block_shutdown_or_boot() {
    use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
    use maos_registry::yank::{YankDisposition, YankObserver, YankPoller, YankSource};
    use std::sync::{Arc, Mutex};
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var(
        "MAOS_REGISTRY_YANK_CURSOR_PATH",
        scratch.0.join("cursor.json"),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        struct Source(
            std::sync::mpsc::Sender<()>,
            Mutex<std::sync::mpsc::Receiver<()>>,
        );
        impl YankSource for Source {
            fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
                self.0.send(()).unwrap();
                self.1.lock().unwrap().recv().unwrap();
                Ok(YankList::new(Vec::new()))
            }
        }
        struct Observer;
        impl YankObserver for Observer {
            fn on_yank(&self, _: &YankEntry) -> Result<YankDisposition, RegistryError> {
                Ok(YankDisposition::Verified)
            }
        }
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (stop, stopped) = tokio::sync::watch::channel(false);
        let stopped_at = Arc::new(Mutex::new(None));
        let controller_stop = stop.clone();
        let controller_time = stopped_at.clone();
        let controller = std::thread::spawn(move || {
            started_rx.recv().unwrap();
            *controller_time.lock().unwrap() = Some(std::time::Instant::now());
            controller_stop.send(true).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(3));
            release_tx.send(()).unwrap();
        });
        maos_registry::yank::yank_poller_production_loop(
            Arc::new(YankPoller::new(
                Arc::new(Source(started_tx, Mutex::new(release_rx))),
                Arc::new(Observer),
                "fixture://blocked-fetch".into(),
            )),
            stopped,
            std::time::Duration::from_secs(300),
        )
        .await;
        let elapsed = stopped_at.lock().unwrap().as_ref().unwrap().elapsed();
        controller.join().unwrap();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "shutdown waited for a blocked registry fetch: {elapsed:?}"
        );
    });
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
}

#[test]
fn every_registry_auth_test_holds_the_shared_environment_mutex_p15() {
    let _env = ENV.lock().unwrap();
    let source = include_str!("yank_auth_20_1b.rs");
    let tests = source
        .lines()
        .filter(|line| line.trim() == "#[test]")
        .count();
    let guarded = source
        .lines()
        .filter(|line| line.trim() == "let _env = ENV.lock().unwrap();")
        .count();
    assert_eq!(
        guarded, tests,
        "every test shares the same process environment and must take ENV"
    );
}

#[test]
fn v2_role_nonce_audience_and_lifetime_vectors_reject_rebinding_f4_f5() {
    use maos_registry::yank_auth::{YankAuthorization, YankScope, MAX_AUTHORIZATION_AGE_MS};
    use ring::signature::Ed25519KeyPair;
    let _env = ENV.lock().unwrap();
    let scope = YankScope {
        name: "vector",
        version: "1.0.0",
        reason: "withdrawn",
        registry_uri: "http://127.0.0.1:20101/mcp",
    };
    for (origin, seed, role) in [
        (
            YankOrigin::Publisher,
            &PUBLISHER,
            "maos.registry.deprecate.publisher.v2",
        ),
        (
            YankOrigin::Vetter,
            &VETTER,
            "maos.registry.deprecate.vetter.v2",
        ),
    ] {
        let mut auth = YankAuthorization {
            format_version: 2,
            registry_uri: scope.registry_uri.into(),
            nonce: [0x2a; 16],
            max_age_ms: MAX_AUTHORIZATION_AGE_MS,
            origin,
            public_key: public(seed),
            vetter_key_id: "vetter".into(),
            issued_at_unix_ms: 1000,
            signature: Vec::new(),
        };
        // Independently constructed deterministic wire vector, not production sign().
        let message = serde_cbor::to_vec(&(
            role,
            2_u32,
            scope.registry_uri,
            [0x2a_u8; 16],
            MAX_AUTHORIZATION_AGE_MS,
            "vector",
            "1.0.0",
            "withdrawn",
            origin,
            public(seed),
            "vetter",
            1000_u64,
        ))
        .unwrap();
        auth.signature = Ed25519KeyPair::from_seed_unchecked(seed)
            .unwrap()
            .sign(&message)
            .as_ref()
            .to_vec();
        let vetters = keyring();
        let root = public(&OPERATOR);
        assert!(auth
            .verify(scope, &public(&PUBLISHER), Some((&vetters, &root)), 1000)
            .is_ok());
        for field in ["nonce", "audience", "age", "role", "format"] {
            let mut changed = auth.clone();
            match field {
                "nonce" => changed.nonce[0] ^= 1,
                "audience" => changed.registry_uri.push('/'),
                "age" => changed.max_age_ms -= 1,
                "role" => {
                    changed.origin = if origin == YankOrigin::Publisher {
                        YankOrigin::Vetter
                    } else {
                        YankOrigin::Publisher
                    }
                }
                _ => changed.format_version = 1,
            }
            assert!(
                changed
                    .verify(scope, &public(&PUBLISHER), Some((&vetters, &root)), 1000)
                    .is_err(),
                "{field} must be signed"
            );
        }
        assert!(auth
            .verify(
                scope,
                &public(&PUBLISHER),
                Some((&vetters, &root)),
                3_601_000
            )
            .is_err());
        let mut entry = maos_domain::ports::registry::YankEntry::new(
            "vector".into(),
            "1.0.0".into(),
            0,
            "withdrawn".into(),
        );
        entry.origin = Some(origin);
        entry.authorization = serde_cbor::to_vec(&auth).unwrap();
        maos_registry::yank_auth::verify_entry(
            &entry,
            &public(&PUBLISHER),
            Some((&vetters, &root)),
            scope.registry_uri,
            3_601_000,
        )
        .unwrap();
        println!(
            "role={role} preimage={} signature={}",
            hex::encode(message),
            hex::encode(&auth.signature)
        );
    }
}

#[test]
fn legacy_inactive_version_pins_are_migrated_before_uninstall_d2() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let server = InProcessServer::start(&scratch);
    let uri = format!("http://{}/mcp", server.addr);
    for version in ["1.0.0", "2.0.0"] {
        let pkg = package("legacy-pin", version, &PUBLISHER);
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(pkg).unwrap()
        )
        .get("result")
        .is_some());
        maos_registry::install::install(&format!("legacy-pin@{version}"), &uri, None, None)
            .unwrap();
    }
    let root = scratch.0.join("home/.local/share/maos/registry");
    // Reproduce the actual pre-correction layout: signed generations, no ledger.
    std::fs::remove_file(root.join("installed-pins.json")).unwrap();
    let mut removal = LocalFsRegistryStorage::uninstall_package("legacy-pin")
        .unwrap()
        .unwrap();
    removal.delete_generation().unwrap();
    removal.accept();
    let mut changed = package("legacy-pin", "1.0.0", &PUBLISHER);
    changed.artifact_bytes.push(1);
    sign(&mut changed, &PUBLISHER);
    let version = scratch.0.join("registry/spirits/legacy-pin/1.0.0");
    std::fs::write(
        version.join("signed_package.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    std::fs::write(version.join("artifact.bin"), &changed.artifact_bytes).unwrap();
    assert!(maos_registry::install::install("legacy-pin@1.0.0", &uri, None, None).is_err(),
        "uninstall must preserve every legacy generation's version hash, not just the active version");
}

#[test]
fn only_explicit_operator_release_allows_repin_after_uninstall_d2() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    std::env::set_var("HOME", scratch.0.join("home"));
    let server = InProcessServer::start(&scratch);
    let uri = format!("http://{}/mcp", server.addr);
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(package("release", "1.0.0", &PUBLISHER)).unwrap()
    )
    .get("result")
    .is_some());
    maos_registry::install::install("release@1.0.0", &uri, None, None).unwrap();
    let mut removal = LocalFsRegistryStorage::uninstall_package("release")
        .unwrap()
        .unwrap();
    removal.delete_generation().unwrap();
    removal.accept();
    let other = Scratch::new();
    let replacement = InProcessServer::start(&other);
    let replacement_uri = format!("http://{}/mcp", replacement.addr);
    assert!(rpc(
        &replacement.addr,
        "registry.publish",
        serde_json::to_value(package("release", "1.0.0", &VETTER)).unwrap()
    )
    .get("result")
    .is_some());
    assert!(
        maos_registry::install::install("release@1.0.0", &replacement_uri, None, None).is_err()
    );
    let root = scratch.0.join("home/.local/share/maos/registry");
    let release = std::process::Command::new(env!("CARGO_BIN_EXE_maos-registry-server"))
        .arg("--root")
        .arg(&root)
        .args(["release-pin", "release"])
        .output()
        .unwrap();
    assert!(
        release.status.success(),
        "{}",
        String::from_utf8_lossy(&release.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&release.stdout).unwrap();
    assert_eq!(receipt["released"], true);
    maos_registry::install::install("release@1.0.0", &replacement_uri, None, None).unwrap();
}

#[test]
fn attestation_rejects_unscoped_digest_signature_review_f5() {
    use ring::signature::Ed25519KeyPair;
    use sha2::{Digest, Sha256};
    let _env = ENV.lock().unwrap();
    let pkg = package("roles", "1.0.0", &PUBLISHER);
    let mut att = trust::attestation(&pkg, 10, 1000);
    let original = att.signature;
    let key = Ed25519KeyPair::from_seed_unchecked(&VETTER).unwrap();
    att.signature = key
        .sign(&Sha256::digest(&att.claim_bytes))
        .as_ref()
        .try_into()
        .unwrap();
    assert!(
        matches!(
            maos_compliance::verify_attestation_signature(&att),
            Err(maos_compliance::vetting::VettingRejection::ForgedSignature)
        ),
        "a raw claim digest has no signed role and must not authorize vetting"
    );
    att.signature = original;
    maos_compliance::verify_attestation_signature(&att).unwrap();
    let mut hash = Sha256::new();
    hash.update(b"maos.vetting.attestation.v2\0");
    hash.update((att.claim_bytes.len() as u64).to_le_bytes());
    hash.update(&att.claim_bytes);
    let digest = hash.finalize();
    assert_eq!(
        original.as_slice(),
        key.sign(&digest).as_ref(),
        "the v2 role/framing vector changed"
    );
    println!(
        "attestation-role-v2 digest={} signature={}",
        hex::encode(digest),
        hex::encode(original)
    );
}

#[test]
fn keyring_event_rejects_unscoped_digest_signature_review_f5() {
    use ring::signature::Ed25519KeyPair;
    use sha2::{Digest, Sha256};
    let _env = ENV.lock().unwrap();
    let mut vetters = keyring();
    let original = vetters.events[0].signature;
    let key = Ed25519KeyPair::from_seed_unchecked(&OPERATOR).unwrap();
    vetters.events[0].signature = key
        .sign(&Sha256::digest(&vetters.events[0].event_bytes))
        .as_ref()
        .try_into()
        .unwrap();
    assert!(
        matches!(
            vetters.verify_enrollment(&public(&OPERATOR), "vetter", &public(&VETTER), 10, 1000),
            Err(maos_compliance::vetting::VettingRejection::OperatorRootSignatureInvalid)
        ),
        "an audit-root signature without its key-event role must be refused"
    );
    vetters.events[0].signature = original;
    vetters
        .verify_enrollment(&public(&OPERATOR), "vetter", &public(&VETTER), 10, 1000)
        .unwrap();
    let mut hash = Sha256::new();
    hash.update(b"maos.vetting.key-event.v2\0");
    hash.update((vetters.events[0].event_bytes.len() as u64).to_le_bytes());
    hash.update(&vetters.events[0].event_bytes);
    let digest = hash.finalize();
    assert_eq!(
        original.as_slice(),
        key.sign(&digest).as_ref(),
        "the v2 role/framing vector changed"
    );
    println!(
        "key-event-role-v2 digest={} signature={}",
        hex::encode(digest),
        hex::encode(original)
    );
}

#[test]
fn signed_nonce_cannot_be_reused_for_another_withdrawal_f4() {
    use maos_registry::yank_auth::YankScope;
    use ring::signature::Ed25519KeyPair;
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    for version in ["1.0.0", "2.0.0"] {
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(package("nonce", version, &PUBLISHER)).unwrap()
        )
        .get("result")
        .is_some());
    }
    let first = yank(
        &server.addr,
        "nonce",
        "1.0.0",
        &PUBLISHER,
        YankOrigin::Publisher,
    );
    assert!(rpc(&server.addr, "registry.deprecate", first.clone())
        .get("result")
        .is_some());
    let bytes: Vec<u8> = serde_json::from_value(first["authorization"].clone()).unwrap();
    let mut auth = maos_registry::yank_auth::decode(&bytes).unwrap();
    let uri = format!("http://{}/mcp", server.addr);
    let scope = YankScope {
        name: "nonce",
        version: "2.0.0",
        reason: "withdrawn",
        registry_uri: &uri,
    };
    let message = serde_cbor::to_vec(&(
        "maos.registry.deprecate.publisher.v2",
        auth.format_version,
        &auth.registry_uri,
        auth.nonce,
        auth.max_age_ms,
        scope.name,
        scope.version,
        scope.reason,
        auth.origin,
        auth.public_key,
        &auth.vetter_key_id,
        auth.issued_at_unix_ms,
    ))
    .unwrap();
    let key = Ed25519KeyPair::from_seed_unchecked(&PUBLISHER).unwrap();
    auth.signature = key.sign(&message).as_ref().to_vec();
    auth.verify(
        scope,
        &public(&PUBLISHER),
        None,
        maos_registry::yank_auth::now_unix_ms().unwrap(),
    )
    .unwrap();
    let response = rpc(
        &server.addr,
        "registry.deprecate",
        serde_json::json!({
        "spirit_id":scope.name,"version":scope.version,"reason":scope.reason,
        "authorization":serde_cbor::to_vec(&auth).unwrap()}),
    );
    assert!(
        response.get("error").is_some(),
        "even an otherwise valid signature must not rebind a used nonce: {response}"
    );
    let rows: Vec<maos_domain::ports::registry::YankEntry> =
        serde_json::from_slice(&std::fs::read(scratch.0.join("registry/yanks.json")).unwrap())
            .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn verified_cursor_is_persisted_before_later_transient_failure_p2() {
    use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
    use maos_registry::yank::{YankDisposition, YankObserver, YankPoller, YankSource};
    use std::sync::Arc;
    let _env = ENV.lock().unwrap();
    struct Source;
    impl YankSource for Source {
        fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
            Ok(YankList::new(
                (1..=2)
                    .map(|sequence| {
                        let mut entry =
                            YankEntry::new("cursor".into(), "1.0.0".into(), 0, "withdrawn".into());
                        entry.sequence = sequence;
                        entry
                    })
                    .collect(),
            ))
        }
    }
    struct Observer;
    impl YankObserver for Observer {
        fn on_yank(&self, entry: &YankEntry) -> Result<YankDisposition, RegistryError> {
            if entry.sequence == 2 {
                Err(RegistryError::Transport("transient local effect".into()))
            } else {
                Ok(YankDisposition::Verified)
            }
        }
    }
    let scratch = Scratch::new();
    std::env::set_var(
        "MAOS_REGISTRY_YANK_CURSOR_PATH",
        scratch.0.join("cursor.json"),
    );
    let uri = "fixture://partial-batch";
    let poller = YankPoller::new(Arc::new(Source), Arc::new(Observer), uri.into());
    poller.load_cursor().unwrap();
    assert!(poller.poll_once().is_err());
    assert_eq!(poller.last_seen_ns(), 1);
    let restarted = YankPoller::new(Arc::new(Source), Arc::new(Observer), uri.into());
    restarted.load_cursor().unwrap();
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    assert_eq!(
        restarted.last_seen_ns(),
        1,
        "the first verified row must survive a later per-row transient failure"
    );
}

#[test]
fn signer_key_in_two_authorized_roles_keeps_each_yank_origin_p13() {
    let _env = ENV.lock().unwrap();
    let scratch = Scratch::new();
    // Explicit fixture enrollment: the operator permits this publisher key
    // to act as a vetter too. The two v2 role authorities remain distinct.
    let mut vetters = keyring();
    let mut claim: maos_compliance::vetting::VetterKeyEventClaim =
        serde_cbor::from_slice(&vetters.events[0].event_bytes).unwrap();
    claim.vetter_pubkey = public(&PUBLISHER);
    vetters.events[0] = maos_compliance::vetting::keyring::issue_event(&OPERATOR, &claim);
    let server = InProcessServer::start_config(&scratch, None, Some((vetters, public(&OPERATOR))));
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(package("roles", "1.0.0", &PUBLISHER)).unwrap()
    )
    .get("result")
    .is_some());
    for _ in 0..2 {
        for origin in [YankOrigin::Vetter, YankOrigin::Publisher] {
            let response = rpc(
                &server.addr,
                "registry.deprecate",
                yank(&server.addr, "roles", "1.0.0", &PUBLISHER, origin),
            );
            assert!(response.get("result").is_some(), "{response}");
        }
    }
    let response = rpc(
        &server.addr,
        "registry.yanks_since",
        serde_json::json!({"since_ns":0}),
    );
    let entries = response["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2, "deduplication must not discard an independent publisher withdrawal under a key also enrolled as vetter");
    assert_eq!(entries[0]["origin"], "vetter");
    assert_eq!(entries[1]["origin"], "publisher");
}
