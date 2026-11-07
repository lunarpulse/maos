#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::{SpiritId, YankOrigin};
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage, StorageError};
use support::*;
use trust::*;

#[test]
fn unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17() {
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
        yank("owner", "1.0.0", &[0x54; 32], YankOrigin::Publisher),
        yank("owner", "1.0.0", &VETTER, YankOrigin::Vetter),
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
    let first = yank("withdraw", "1.0.0", &PUBLISHER, YankOrigin::Publisher);
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
        yank("withdraw", "2.0.0", &VETTER, YankOrigin::Vetter)
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
        1000,
    )
    .unwrap();
    let mut tampered = entries[0].clone();
    tampered.reason.push('!');
    assert!(maos_registry::yank_auth::verify_entry(
        &tampered,
        &public(&PUBLISHER),
        Some((&keyring(), &root)),
        1000
    )
    .is_err());
}

#[test]
fn daemon_cursor_restart_does_not_replay_journalled_yanks_ac7() {
    use maos_domain::ports::registry::{RegistryError, YankEntry};
    use maos_registry::yank::{YankObserver, YankPoller};
    use std::sync::{Arc, Mutex};
    struct Journal(Mutex<Vec<u64>>);
    impl YankObserver for Journal {
        fn on_yank(&self, entry: &YankEntry) -> Result<(), RegistryError> {
            maos_registry::yank_auth::verify_entry(entry, &public(&PUBLISHER), None, 1000)
                .map_err(RegistryError::Transport)?;
            self.0.lock().unwrap().push(entry.sequence);
            Ok(())
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
        yank("cursor", "1.0.0", &PUBLISHER, YankOrigin::Publisher)
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
    let journal = Arc::new(Journal(Mutex::new(vec![])));
    let poller = YankPoller::new(client.clone(), journal.clone());
    assert_eq!(poller.poll_once().unwrap(), 1);
    poller.save_cursor().unwrap();
    drop(poller);
    let restarted = YankPoller::new(client, journal.clone());
    restarted.load_cursor();
    assert_eq!(restarted.last_seen_ns(), 1);
    assert_eq!(restarted.poll_once().unwrap(), 0);
    assert_eq!(*journal.0.lock().unwrap(), vec![1]);
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
}

#[test]
fn installed_replacement_and_yanked_install_are_refused_before_commit_ac14_ac16() {
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
    assert!(rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(pkg).unwrap()
    )
    .get("result")
    .is_some());
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
        yank("withdrawn", "1.0.0", &PUBLISHER, YankOrigin::Publisher)
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
