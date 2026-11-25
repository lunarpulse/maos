#![cfg(feature = "network")]
//! Story 23-7 / AC2 (D-23-7-02 C) — the production trigger of `swap_serving_cert`, against a REAL
//! daemon (`MAOS_ONE_SHOT=cohort-a2a-daemon`) through the shared 14-2a harness.
//!
//! The daemon swaps its own leaf only when the signed manifest declares it and the configured PEM
//! paths hold it. The reconciler wakes on the reissue and then on every confirmation interval
//! (`(t_stale_secs + 1) / 2` = 15 s for the fixture's signed lease); there is no clock injection
//! point in a separate process, so the legs poll an operator read against an outer deadline and
//! assert only ORDER and END STATE, never elapsed time.
//!
//! AC2(e) (the swap runs off the `cached` lock) is proven in-process by
//! `maos-cohort/tests/cert_rotation_14_2a.rs::the_local_leaf_observer_runs_off_the_cached_lock`.
//! AC2(d) (plane C) by `maos-a2a-tcp/tests/peer_rotation_23_7.rs`.

#[path = "support/rotation_daemon_14_2a.rs"]
mod rotation_daemon;

use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::PeerCertFingerprint;
use maos_cohort::{
    CohortDistributor, CohortManifestState, InMemoryCohortAuditSink, PinnedAuthorityKeys,
};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlFrameKind};
use maos_spirit_abi::identity::{HostId, SpiritId};
use rotation_daemon::*;

const OUTER_DEADLINE: Duration = Duration::from_secs(120);

fn self_identity(port: u16) -> String {
    operator_get(port, "/v1/cohort/self-identity")
}

fn rows(fixture: &Fixture, event: &str) -> usize {
    let Ok(log) = maos_iac::adapter::TransparencyLogAdapter::open(&fixture.audit_db, 0) else {
        return 0;
    };
    log.query_frames(FrameFilter {
        kind: Some(TlFrameKind::TelemetryEvent),
        ..Default::default()
    })
    .map(|frames| {
        frames
            .iter()
            .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
            .filter(|row| {
                String::from_utf8_lossy(&row.payload_redacted)
                    .contains(&format!(r#""event":"{event}""#))
            })
            .count()
    })
    .unwrap_or(0)
}

async fn wait_for(what: &str, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + OUTER_DEADLINE;
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// host-b dials the daemon pinning `pinned_a` as host-a's leaf, then pushes the signed reissue
/// `v2` (host-a declared as `declared_a`) with the production courier. `Ok` only if the daemon
/// presented `pinned_a` AND ACKed.
async fn push_reissue(
    fixture: &Fixture,
    daemon_port: u16,
    pinned_a: &PeerCertFingerprint,
    declared_a: &PeerCertFingerprint,
) -> Result<(), String> {
    let host_b = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind(
            maos_a2a_tcp::TcpA2AConfig {
                listen_addr: "127.0.0.1:0".parse().unwrap(),
                own_cert_chain: fixture.host_b.cert_path.clone(),
                own_private_key: fixture.host_b.key_path.clone(),
                peer_pins: vec![maos_a2a_tcp::PinnedFingerprint {
                    peer_id: maos_a2a_core::PeerId::new("host-a"),
                    fingerprint: pinned_a.clone(),
                    boot_nonce: 7,
                }],
                handshake_timeout: Duration::from_secs(5),
                ca_roots: None,
                peer_san: Default::default(),
                peer_trust_domain: None,
            },
            vec![peer_config(
                "host-a",
                &format!("tls://127.0.0.1:{daemon_port}"),
                pinned_a,
            )],
            HOST_B_NONCE,
            maos_a2a_tcp::TcpTimeouts::production(Duration::from_secs(5)),
            maos_a2a_core::HandshakeRetryPolicy {
                backoff_ms: vec![],
                jitter_pct: 0,
                max_attempts: 1,
            },
            None,
            None,
        )
        .await
        .map_err(|error| error.to_string())?,
    );
    let v2 = signed_manifest(&signing_key(), 2, declared_a, &fixture.host_b.fingerprint);
    let state = Arc::new(
        CohortManifestState::load(
            HostId("host-b".into()),
            &v2,
            PinnedAuthorityKeys::from_keys(vec![signing_key().verifying_key()]).unwrap(),
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap(),
    );
    let router: Arc<dyn A2APeerRouter> = Arc::clone(&host_b) as Arc<dyn A2APeerRouter>;
    CohortDistributor::new(
        state,
        router,
        maos_domain::frame::FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: Some(HostId("host-b".into())),
            role: None,
        },
    )
    .push_to(&HostId("host-a".into()))
    .await
    .map_err(|error| error.to_string())
}

fn write_leaf(into: &Identity, from: &Identity) {
    std::fs::copy(&from.cert_path, &into.cert_path).expect("PKI writes the cert");
    std::fs::copy(&from.key_path, &into.key_path).expect("PKI writes the key");
}

/// AC2(a)+(b): paths still holding `F_old` when the reissue lands → no swap, `diverged`, ONE
/// `self_leaf_swap_pending` row; writing `F_new` afterwards swaps it with no restart and no
/// reissue, and a fresh handshake presents `F_new`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reissue_before_the_files_leaves_the_host_diverged_and_the_files_then_swap_it() {
    let fixture = fixture("self-leaf-ab");
    let (daemon, daemon_port, operator_port, _log) = boot_daemon(&fixture).expect("boot");
    struct Kill(std::process::Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _daemon = Kill(daemon);
    let f_old = fixture.host_a.fingerprint.clone();
    let f_new = mint_identity(&fixture.dir, "host-a-new");

    // The reissue lands first: it declares F_new, the files still hold F_old.
    push_reissue(&fixture, daemon_port, &f_old, &f_new.fingerprint)
        .await
        .expect("the signed reissue is ACKed over mTLS with the old leaf");
    wait_for("self-identity reports diverged", || {
        self_identity(operator_port).contains(r#""verdict":"diverged""#)
    })
    .await;
    wait_for("one self_leaf_swap_pending row", || {
        rows(&fixture, "self_leaf_swap_pending") >= 1
    })
    .await;
    // Not served: a handshake pinned to F_new is refused while the old leaf is still served.
    assert!(
        push_reissue(
            &fixture,
            daemon_port,
            &f_new.fingerprint,
            &f_new.fingerprint
        )
        .await
        .is_err(),
        "the daemon must still present the old leaf"
    );

    // The PKI writes F_new to the configured paths. No restart, no new reissue.
    write_leaf(&fixture.host_a, &f_new);
    let declared = format!(r#""declared":"{}""#, f_new.fingerprint.short());
    let serving = format!(r#""serving":"{}""#, f_new.fingerprint.short());
    wait_for("serving == declared and not diverged", || {
        let seen = self_identity(operator_port);
        seen.contains(&declared)
            && seen.contains(&serving)
            && !seen.contains(r#""verdict":"diverged""#)
    })
    .await;
    // The fixture's only peer (host-b) is a one-shot courier: it never `Pull`s the daemon, so it
    // never confirms v2 (review B5). The daemon therefore holds the swap until the manifest's own
    // stale lease has elapsed since it applied v2, then swaps and says who never confirmed. A hold
    // row may or may not be written first (it depends on where the tick falls inside the lease),
    // so only the terminal outcome is asserted.
    wait_for("self_leaf_swapped_unconfirmed row", || {
        rows(&fixture, "self_leaf_swapped_unconfirmed") >= 1
    })
    .await;
    assert_eq!(
        rows(&fixture, "self_leaf_swapped"),
        0,
        "host-b never confirmed v2, so the swap is the lease-bounded one"
    );
    assert!(
        rows(&fixture, "self_leaf_swap_held") <= 1,
        "a standing hold is ONE row, not one per tick"
    );
    assert_eq!(
        rows(&fixture, "self_leaf_swap_pending"),
        1,
        "a standing condition is ONE row, not one per tick"
    );
    // A fresh mTLS handshake presents F_new (pinned to it alone).
    push_reissue(
        &fixture,
        daemon_port,
        &f_new.fingerprint,
        &f_new.fingerprint,
    )
    .await
    .expect("a fresh handshake presents the new leaf");
}

/// AC2(c): PEMs holding a third leaf the manifest does not declare are never served; the refusal
/// is journaled with its own typed row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_leaf_the_manifest_does_not_declare_is_never_served() {
    let fixture = fixture("self-leaf-c");
    let (daemon, daemon_port, operator_port, _log) = boot_daemon(&fixture).expect("boot");
    struct Kill(std::process::Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _daemon = Kill(daemon);
    let f_old = fixture.host_a.fingerprint.clone();
    let declared_new = mint_identity(&fixture.dir, "host-a-declared");
    let third = mint_identity(&fixture.dir, "host-a-third");

    write_leaf(&fixture.host_a, &third);
    push_reissue(&fixture, daemon_port, &f_old, &declared_new.fingerprint)
        .await
        .expect("the reissue is ACKed with the old leaf still served");
    wait_for("self_leaf_refused_undeclared row", || {
        rows(&fixture, "self_leaf_refused_undeclared") >= 1
    })
    .await;
    assert!(self_identity(operator_port).contains(r#""verdict":"diverged""#));
    assert!(
        push_reissue(
            &fixture,
            daemon_port,
            &third.fingerprint,
            &declared_new.fingerprint
        )
        .await
        .is_err(),
        "the undeclared third leaf is never served"
    );
    assert_eq!(rows(&fixture, "self_leaf_swapped"), 0);
}
