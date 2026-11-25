#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 21-3 / AC1.d, AC1.f, AC1.g — durable TOFU pins across a daemon restart.
//!
//! Three legs drive a REAL `maos` daemon through the shared 14-2a harness: a
//! signed reissue promotes the peer's pin past its grace, the daemon gets
//! `SIGTERM`, and the restart is read back through `GET /v1/a2a/pins`:
//!
//! * promotion carried into config and the signed manifest → the promoted
//!   fingerprint, `generation` ≥ 1;
//! * the pin file deleted between runs (the planted red) → the row reverts to
//!   the config seed, `generation` 0;
//! * promotion NOT carried → a typed boot refusal naming both fingerprints.
//!
//! A fourth, in-process leg drives the real consent door (`RepinControl`)
//! against the real file store: an approved nonce survives a restart, and
//! without the file the same peer's next frame is re-detected as a restart.

#[path = "support/rotation_daemon_14_2a.rs"]
mod rotation_daemon;

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_a2a_core::tofu::{InMemoryTofuPinStore, PinPersistence, TofuPinStore as _};
use maos_a2a_core::{PeerCertFingerprint, PeerId};
use maos_a2a_tcp::{PinnedFingerprint, TcpA2AConfig};
use maos_bin::repin_door::RepinControl;
use maos_control::{OperatorOutcome, PeerPinStatus};
use maos_persistence::{FilePinStore, PIN_FILE_NAME};
use rotation_daemon::{
    boot_daemon, drive_a_signed_rotation, operator_get, rotated_fingerprint, signed_manifest,
    signing_key, Fixture, Rotation, HOST_B_NONCE,
};
use serde_json::Value;

/// Kills the restarted daemon when a leg ends, pass or panic.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pins(operator_port: u16) -> Value {
    let response = operator_get(operator_port, "/v1/a2a/pins");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let (_, body) = response.split_once("\r\n\r\n").expect("response body");
    serde_json::from_str(body).unwrap_or_else(|error| panic!("pins JSON ({error}): {body}"))
}

/// The daemon's pin file: `$HOME/.maos/a2a-pins-<hex(local_host)>.json` (the fixture's host is `host-a`).
fn daemon_pin_file(fixture: &Fixture) -> std::path::PathBuf {
    let name = format!("a2a-pins-{}.json", hex::encode("host-a"));
    fixture.dir.join(".maos").join(name)
}

fn host_b(rows: &Value) -> &Value {
    rows.as_array()
        .expect("a JSON array")
        .iter()
        .find(|row| row["peer"] == "host-b")
        .unwrap_or_else(|| panic!("no host-b row in {rows}"))
}

fn sigterm_and_wait(child: &mut Child) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .expect("kill(1)");
    assert!(status.success(), "kill -TERM failed");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if child.try_wait().expect("try_wait").is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the daemon did not exit within 30 s of SIGTERM");
}

/// A live daemon pins host-b, is rotated by a signed reissue past the
/// production grace, and is stopped with `SIGTERM`. The promotion is durable
/// before the stop is sent: the pins surface already shows it.
async fn promoted_then_stopped(tag: &str) -> Rotation {
    let mut rotation = drive_a_signed_rotation(tag).await;
    let before = pins(rotation.operator_port);
    assert_eq!(host_b(&before)["generation"], 0, "{before}");
    assert_eq!(
        host_b(&before)["fingerprint"],
        rotation.fixture.host_b.fingerprint.wire()
    );
    assert_eq!(host_b(&before)["boot_nonce"], HOST_B_NONCE.to_string());

    let deadline =
        rotation.acked_at + maos_cohort::cold_deployment_t_grace() + Duration::from_secs(45);
    loop {
        let rows = pins(rotation.operator_port);
        let row = host_b(&rows);
        if row["fingerprint"] == rotation.rotated.wire() && row["generation"].as_u64() >= Some(1) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the promotion never showed: {rows}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(
        daemon_pin_file(&rotation.fixture).exists(),
        "the promotion must be on disk before the daemon stops"
    );
    sigterm_and_wait(&mut rotation.daemon);
    rotation
}

/// The operator's recovery: name the promoted fingerprint in `[[peers]]`,
/// `[[tcp.peer_pins]]` and the signed on-disk manifest (13.6a requires all three).
fn carry_promotion(fixture: &Fixture, rotated: &PeerCertFingerprint) {
    std::fs::write(
        fixture.dir.join("manifest.toml"),
        signed_manifest(&signing_key(), 2, &fixture.host_a.fingerprint, rotated),
    )
    .expect("write the v2 manifest");
    let config = std::fs::read_to_string(&fixture.config_path).expect("read daemon config");
    std::fs::write(
        &fixture.config_path,
        config.replace(&fixture.host_b.fingerprint.hex, &rotated.hex),
    )
    .expect("write daemon config");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_21_3_a_promotion_carried_into_config_survives_a_restart() {
    let rotation = promoted_then_stopped("21-3-carried").await;
    carry_promotion(&rotation.fixture, &rotation.rotated);

    let (child, _a2a_port, operator_port, log) = boot_daemon(&rotation.fixture)
        .unwrap_or_else(|error| panic!("the restart must boot: {error}"));
    let _running = Running(child);
    let rows = pins(operator_port);
    let row = host_b(&rows);
    assert_eq!(row["fingerprint"], rotation.rotated.wire(), "{rows}\n{log}");
    assert!(row["generation"].as_u64() >= Some(1), "{rows}");
    assert_eq!(row["boot_nonce"], HOST_B_NONCE.to_string());
    assert_eq!(row["invalidated"], Value::Null);
}

/// The planted red (one per AC): without `a2a-pins.json` the row is the config
/// seed again — the proof that the file, and nothing else, carries the state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_21_3_deleting_the_pin_file_reverts_the_row_to_the_config_seed() {
    let rotation = promoted_then_stopped("21-3-deleted").await;
    std::fs::remove_file(daemon_pin_file(&rotation.fixture)).expect("delete the pin file");

    let (child, _a2a_port, operator_port, log) = boot_daemon(&rotation.fixture)
        .unwrap_or_else(|error| panic!("the restart must boot: {error}"));
    let _running = Running(child);
    let rows = pins(operator_port);
    let row = host_b(&rows);
    assert_eq!(
        row["fingerprint"],
        rotation.fixture.host_b.fingerprint.wire(),
        "{rows}\n{log}"
    );
    assert_eq!(row["generation"], 0);
    assert_eq!(row["boot_nonce"], HOST_B_NONCE.to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_21_3_a_promotion_not_carried_into_config_refuses_to_boot() {
    let rotation = promoted_then_stopped("21-3-refused").await;
    let file = daemon_pin_file(&rotation.fixture);
    let before = std::fs::read(&file).expect("read the pin file");

    let error = boot_daemon(&rotation.fixture)
        .err()
        .expect("a daemon whose config names the retired fingerprint must not boot");
    assert!(error.contains("pin store refuses to boot"), "{error}");
    assert!(
        error.contains(&rotation.fixture.host_b.fingerprint.wire())
            && error.contains(&rotation.rotated.wire()),
        "the refusal names the retired and the promoted fingerprint: {error}"
    );
    assert_eq!(
        std::fs::read(&file).unwrap(),
        before,
        "a refused boot writes nothing"
    );
    // Falsifier: the same fixture boots once the promotion is carried.
    carry_promotion(&rotation.fixture, &rotation.rotated);
    let (child, ..) = boot_daemon(&rotation.fixture).expect("the carried promotion boots");
    let _running = Running(child);
}

async fn boot_pins(config: &TcpA2AConfig, file: &Path) -> Arc<InMemoryTofuPinStore> {
    // A previous run's store is dropped just before this call, but a task of its consent door
    // can still hold its last Arc for a moment on a loaded runner (hosted loom leg, PR #46):
    // the lock then reads `Held`. That is the lock working, so wait for the release (bounded)
    // instead of failing; any other open error, and a lock that is never released, still fail.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let store = loop {
        match FilePinStore::open(file) {
            Ok(store) => break store,
            Err(error)
                if format!("{error:?}").starts_with("Held")
                    && std::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => panic!("open the pin file: {error:?}"),
        }
    };
    let port = Arc::new(store);
    config
        .build_pin_store(Some(port as Arc<dyn PinPersistence>))
        .await
        .expect("reconcile the pin store")
}

/// AC1.d / AC1.f (consent half) — the real consent door against the real file
/// store, in process: the approved nonce is what a restart restores, and the
/// restart's re-armed request is what the door lists.
#[tokio::test]
async fn t_21_3_a_consented_repin_survives_a_restart_only_through_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(PIN_FILE_NAME);
    let peer = PeerId::new("host-b");
    let fingerprint = rotated_fingerprint();
    let config = TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        own_cert_chain: "/nonexistent/chain.pem".into(),
        own_private_key: "/nonexistent/key.pem".into(),
        peer_pins: vec![PinnedFingerprint {
            peer_id: peer.clone(),
            fingerprint: fingerprint.clone(),
            boot_nonce: 7,
        }],
        handshake_timeout: Duration::from_secs(5),
        ca_roots: None,
    };

    // Run 1: the peer restarts (nonce 8), the operator approves.
    let store = boot_pins(&config, &file).await;
    let control = RepinControl::new();
    assert!(control.install(Arc::clone(&store)));
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer, 8)
            .await
            .unwrap(),
        Some(7)
    );
    let outcome = control
        .decide("host-b", 8, true, "alice", |_row| Ok(()))
        .await;
    assert!(
        matches!(outcome, OperatorOutcome::Completed(_)),
        "{outcome:?}"
    );
    let Some(PeerPinStatus::Healthy(rows)) = control.peer_pins() else {
        panic!("an installed, healthy store");
    };
    assert_eq!(
        (
            rows[0].boot_nonce,
            rows[0].generation,
            rows[0].invalidated.is_none()
        ),
        (8, 0, true)
    );
    drop(control);
    drop(store);

    // Run 2, file kept: the approved nonce is accepted, no restart is detected.
    let kept = boot_pins(&config, &file).await;
    assert_eq!(kept.get_pin_sync(&peer).unwrap().boot_nonce, 8);
    assert_eq!(
        kept.invalidate_if_boot_nonce_differs(&peer, 8)
            .await
            .unwrap(),
        None
    );
    // A new restart is detected, left invalidated and re-armed across run 3.
    assert_eq!(
        kept.invalidate_if_boot_nonce_differs(&peer, 9)
            .await
            .unwrap(),
        Some(8)
    );
    drop(kept);
    let rearmed = boot_pins(&config, &file).await;
    assert!(rearmed.get_pin_sync(&peer).unwrap().invalidated.is_some());
    let door = RepinControl::new();
    assert!(door.install(Arc::clone(&rearmed)));
    let listed = door.pending().expect("an installed control");
    assert_eq!(
        (
            listed.len(),
            listed[0].observed_boot_nonce,
            listed[0].expired
        ),
        (1, 9, false)
    );
    drop(door);
    drop(rearmed);

    // Run 3, file deleted (the planted red): the config seed returns and the
    // approved nonce's frame is re-detected as a restart.
    std::fs::remove_file(&file).expect("delete the pin file");
    let seeded = boot_pins(&config, &file).await;
    let pin = seeded.get_pin_sync(&peer).unwrap();
    assert_eq!((pin.boot_nonce, pin.invalidated), (7, None));
    assert_eq!(
        seeded
            .invalidate_if_boot_nonce_differs(&peer, 8)
            .await
            .unwrap(),
        Some(7)
    );
}
