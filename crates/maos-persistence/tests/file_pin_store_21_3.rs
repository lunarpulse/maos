//! Story 21-3 / AC1.a — `FilePinStore`: strict versioned schema, atomic write,
//! single writer. Fixtures are synthetic: hand-written JSON and made-up digests.

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{Invalidated, PinPersistence, PinRecord, TofuPin};
use maos_persistence::{FilePinStore, PinFileError, PIN_FILE_NAME};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn fp(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(&[seed; 8])
}

fn record(peer: &str, seed: u8) -> PinRecord {
    PinRecord {
        pin: TofuPin {
            peer: PeerId::new(peer),
            fingerprint: fp(seed),
            boot_nonce: u64::MAX,
            pinned_at_ns: 0,
            invalidated: None,
            repin_approval_id: None,
        },
        anchor: (fp(1), 7),
        generation: 2,
        pending_observed: None,
        promoted: false,
    }
}

fn path_in(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join(PIN_FILE_NAME)
}

fn row(peer: &str) -> Value {
    json!({
        "peer": peer,
        "fingerprint": fp(1).wire(),
        "boot_nonce": "7",
        "invalidated": null,
        "repin_approval_id": null,
        "anchor_fingerprint": fp(1).wire(),
        "anchor_boot_nonce": "7",
        "generation": 0,
        "pending_observed_boot_nonce": null,
    })
}

fn doc(version: u64, pins: Vec<Value>) -> Value {
    json!({"version": version, "pins": pins})
}

fn refusal(body: &Value) -> PinFileError {
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    fs::write(&path, serde_json::to_vec(body).unwrap()).unwrap();
    FilePinStore::open(&path).unwrap_err()
}

fn malformed_field(mutate: impl FnOnce(&mut Value)) -> &'static str {
    let mut pin = row("peer-a");
    mutate(&mut pin);
    match refusal(&doc(1, vec![pin])) {
        PinFileError::Malformed { peer, field, .. } => {
            assert_eq!(peer, "peer-a");
            field
        }
        other => panic!("expected Malformed, got {other:?}"),
    }
}

fn temps_in(dir: &Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        })
        .count()
}

#[test]
fn round_trips_every_durable_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let mut restarted = record("peer-b", 2);
    restarted.pin.invalidated = Some(Invalidated::SpiritRestarted {
        prior_boot_nonce: 41,
    });
    restarted.pin.repin_approval_id = Some([0xAB; 16]);
    restarted.pending_observed = Some(u64::MAX - 1);
    let mut manual = record("peer-c", 3);
    manual.pin.invalidated = Some(Invalidated::Manual);
    let records = vec![record("peer-a", 1), restarted, manual];

    let store = FilePinStore::open(&path).unwrap();
    store.store(&records, 0).unwrap();
    drop(store);
    assert_eq!(FilePinStore::open(&path).unwrap().load(), records);
}

#[test]
fn missing_file_is_an_empty_record_set() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("not-yet").join(PIN_FILE_NAME);
    assert!(FilePinStore::open(&nested).unwrap().load().is_empty());
}

#[cfg(unix)]
#[test]
fn the_file_is_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    FilePinStore::open(&path)
        .unwrap()
        .store(&[record("peer-a", 1)], 0)
        .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn unknown_fields_versions_and_malformed_json_are_typed_refusals() {
    let mut extra_top = doc(1, vec![row("peer-a")]);
    extra_top["surprise"] = json!(true);
    assert!(matches!(refusal(&extra_top), PinFileError::Schema { .. }));

    let mut extra_row = row("peer-a");
    extra_row["next_fingerprint"] = json!(fp(9).wire());
    assert!(matches!(
        refusal(&doc(1, vec![extra_row])),
        PinFileError::Schema { .. }
    ));

    assert!(matches!(
        refusal(&doc(3, vec![row("peer-a")])),
        PinFileError::Version { found: 3, .. }
    ));
    assert!(matches!(
        refusal(&json!({"pins": []})),
        PinFileError::Schema { .. }
    ));

    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    fs::write(&path, b"{not json").unwrap();
    assert!(matches!(
        FilePinStore::open(&path).unwrap_err(),
        PinFileError::Schema { .. }
    ));
}

#[test]
fn a_duplicate_peer_is_refused() {
    let error = refusal(&doc(1, vec![row("peer-a"), row("peer-a")]));
    assert!(matches!(error, PinFileError::DuplicatePeer { ref peer, .. } if peer == "peer-a"));
}

#[test]
fn a_duplicated_key_is_refused_not_last_wins_and_the_diagnostic_is_value_free() {
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let good = serde_json::to_string(&doc(1, vec![row("peer-a")])).unwrap();
    let planted = fp(9).wire();
    let body = good.replacen(
        "\"peer\":\"peer-a\"",
        &format!("\"fingerprint\":\"{planted}\",\"peer\":\"peer-a\""),
        1,
    );
    assert_ne!(body, good, "the duplicate key was planted");
    fs::write(&path, body).unwrap();
    let error = FilePinStore::open(&path).unwrap_err();
    assert!(matches!(error, PinFileError::Schema { .. }), "{error:?}");
    assert!(!error.to_string().contains(&fp(9).hex), "{error}");
}

#[test]
fn malformed_fingerprints_are_refused() {
    let upper = fp(1).wire().to_uppercase().replace("SHA256", "sha256");
    for bad in [
        "".to_string(),
        "deadbeef".to_string(),
        "sha256:xyz".to_string(),
        format!("sha1:{}", fp(1).hex),
        upper,
    ] {
        assert_eq!(
            malformed_field(|pin| pin["fingerprint"] = json!(bad)),
            "fingerprint",
            "{bad}"
        );
    }
    assert_eq!(
        malformed_field(|pin| pin["anchor_fingerprint"] = json!("sha256:00")),
        "anchor_fingerprint"
    );
}

#[test]
fn non_decimal_or_non_canonical_nonces_are_refused() {
    for bad in [
        "",
        "0x10",
        "+5",
        "-1",
        "007",
        " 7",
        "7 ",
        "18446744073709551616",
    ] {
        assert_eq!(
            malformed_field(|pin| pin["boot_nonce"] = json!(bad)),
            "boot_nonce",
            "{bad:?}"
        );
    }
    assert_eq!(
        malformed_field(|pin| pin["anchor_boot_nonce"] = json!("1e3")),
        "anchor_boot_nonce"
    );
    // A JSON number is not a decimal string: it is a schema refusal.
    let mut numeric = row("peer-a");
    numeric["boot_nonce"] = json!(7);
    assert!(matches!(
        refusal(&doc(1, vec![numeric])),
        PinFileError::Schema { .. }
    ));
}

#[test]
fn inconsistent_invalidation_state_is_refused() {
    assert_eq!(
        malformed_field(|pin| {
            pin["invalidated"] = json!({"reason": "bogus", "prior_boot_nonce": null})
        }),
        "invalidated"
    );
    assert_eq!(
        malformed_field(|pin| {
            pin["invalidated"] = json!({"reason": "spirit_restarted", "prior_boot_nonce": null})
        }),
        "invalidated"
    );
    // A pending request without an invalidated pin names nothing to consent to.
    assert_eq!(
        malformed_field(|pin| pin["pending_observed_boot_nonce"] = json!("9")),
        "pending_observed_boot_nonce"
    );
    assert_eq!(
        malformed_field(|pin| pin["repin_approval_id"] = json!("AB")),
        "repin_approval_id"
    );
}

#[test]
fn a_second_opener_is_refused_until_the_first_drops() {
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let first = FilePinStore::open(&path).unwrap();
    assert!(matches!(
        FilePinStore::open(&path).unwrap_err(),
        PinFileError::Held { .. }
    ));
    drop(first);
    FilePinStore::open(&path).unwrap();
}

#[test]
fn a_failed_write_leaves_no_temp_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let store = FilePinStore::open(&path).unwrap();
    // The rename over a non-empty directory fails after the temp was created.
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep"), b"x").unwrap();
    assert!(store.store(&[record("peer-a", 1)], 0).is_err());
    assert_eq!(fs::read(path.join("keep")).unwrap(), b"x");
    assert_eq!(temps_in(dir.path()), 0);
}

#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_prior_file_valid() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let store = FilePinStore::open(&path).unwrap();
    let prior = vec![record("peer-a", 1)];
    store.store(&prior, 0).unwrap();
    let before = fs::read(&path).unwrap();

    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let writable_anyway = fs::File::create(dir.path().join("probe")).is_ok();
    let result = store.store(&[record("peer-b", 2)], 0);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    if writable_anyway {
        return; // privileged runner: permissions cannot inject the failure
    }
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(temps_in(dir.path()), 0);
    drop(store);
    assert_eq!(FilePinStore::open(&path).unwrap().load(), prior);
}

// Planted mutation (F6): dropping the positional-array check accepts the 9-element row.
#[test]
fn positional_arrays_are_not_schema_1() {
    let positional_row = json!([
        "peer-a",
        fp(1).wire(),
        "7",
        null,
        null,
        fp(1).wire(),
        "7",
        0,
        null
    ]);
    for body in [
        json!([1, []]),
        doc(1, vec![positional_row]),
        doc(
            1,
            vec![{
                let mut pin = row("peer-a");
                pin["invalidated"] = json!(["manual", null]);
                pin
            }],
        ),
    ] {
        assert!(
            matches!(refusal(&body), PinFileError::Schema { .. }),
            "{body}"
        );
    }
}

#[cfg(unix)]
#[test]
fn the_data_file_is_opened_nofollow_and_a_fifo_never_blocks() {
    // Planted mutation (F3): reading the pin file by path follows the link / blocks on the FIFO.
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.json");
    fs::write(
        &target,
        serde_json::to_vec(&doc(1, vec![row("peer-a")])).unwrap(),
    )
    .unwrap();
    let path = path_in(&dir);
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(matches!(
        FilePinStore::open(&path).unwrap_err(),
        PinFileError::Io { .. }
    ));

    fs::remove_file(&path).unwrap();
    let made = std::process::Command::new("mkfifo").arg(&path).status();
    if made.is_ok_and(|status| status.success()) {
        assert!(matches!(
            FilePinStore::open(&path).unwrap_err(),
            PinFileError::Io { .. }
        ));
    }
}

#[test]
fn a_fence_is_refused_at_open_and_lifted_only_by_a_successful_write() {
    // Planted mutation (F1): `open` ignoring a non-empty lock/fence file.
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let store = FilePinStore::open(&path).unwrap();
    store.store(&[record("peer-a", 1)], 0).unwrap();
    store.fence().unwrap();
    drop(store);
    assert!(matches!(
        FilePinStore::open(&path).unwrap_err(),
        PinFileError::Fenced { .. }
    ));

    // The operator empties the fence file; a later good write also lifts a fence.
    fs::write(dir.path().join(format!("{PIN_FILE_NAME}.lock")), b"").unwrap();
    let store = FilePinStore::open(&path).unwrap();
    store.fence().unwrap();
    store.store(&[record("peer-a", 1)], 0).unwrap();
    drop(store);
    FilePinStore::open(&path).unwrap();
}

// F1 end to end: a rotation retires fingerprint A in memory, the pin-file write fails, and
// the restart must not load the old snapshot that still names A as the current pin.
// Planted mutation: omit the `fence()` call in `persist_narrowed`.
#[cfg(unix)]
#[test]
fn a_failed_retirement_write_is_not_revived_by_a_restart() {
    use maos_a2a_core::tofu::InMemoryTofuPinStore;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = path_in(&dir);
    let port = std::sync::Arc::new(FilePinStore::open(&path).unwrap());
    let store = InMemoryTofuPinStore::new().with_persistence(port.clone());
    store.restore(record("peer-a", 1)).unwrap();
    store.flush().unwrap();
    let peer = PeerId::new("peer-a");
    store.open_rotation_window(&peer, &fp(2)).unwrap();

    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let writable_anyway = fs::File::create(dir.path().join("probe")).is_ok();
    let retired = store.close_rotation_window(&peer);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    if writable_anyway {
        return; // privileged runner: permissions cannot inject the failure
    }
    assert_eq!(retired, Some(fp(1)));
    assert!(store.is_degraded());
    drop(store);
    drop(port);
    assert!(matches!(
        FilePinStore::open(&path).unwrap_err(),
        PinFileError::Fenced { .. }
    ));
}
