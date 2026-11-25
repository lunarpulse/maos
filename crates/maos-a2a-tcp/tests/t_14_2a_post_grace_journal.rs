//! Story 14-2a / AC4.4 — the post-grace rejection §7.2.1.a already required and
//! nothing produced.
//!
//! Architecture §7.2.1.a says, verbatim: *"rejection logged as
//! `cert_post_grace_reject`"*. Measured at `a22f0c01`: only
//! `classified.is_tofu_mismatch()` reached `journal_peer_identity_refusal`
//! (`transport.rs:872`), while `classify_handshake` routes an expired retired
//! leaf to `CertExpired` (`error.rs:123-130`), which returned bare at `:893`.
//!
//! So **the single failure mode the entire rotation mechanism exists to produce
//! — a peer still presenting the retired certificate after the grace closed —
//! left no trace at all.** That is the row an auditor asks for first, and it is
//! the smallest change in this story.
//!
//! These legs assert on the SERVER's journal, never on the dialer's error class:
//! under TLS 1.3 the dialer may only observe an `Io` close (the reason
//! `t_2c_pin_journal.rs` gives, and the same reason applies here).

mod support;

use std::sync::Arc;
use std::time::Duration;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::ConsentRuptureSink;
use maos_domain::frame::{FramePayload, IacFrame, RuptureReason};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::{FrameKind, HostId};
use parking_lot::Mutex;
use support::*;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;

#[derive(Default)]
struct RecordingRuptureSink {
    frames: Mutex<Vec<IacFrame>>,
}

impl RecordingRuptureSink {
    fn identity_refusals(&self) -> Vec<IacFrame> {
        self.frames
            .lock()
            .iter()
            .filter(|frame| {
                frame.kind == FrameKind::ConsentRupture
                    && matches!(&frame.payload, FramePayload::ConsentRupture(payload)
                        if payload.rejected.iter().any(|rejection| rejection.reason == RuptureReason::PeerIdentityUnverified))
            })
            .cloned()
            .collect()
    }
}

impl ConsentRuptureSink for RecordingRuptureSink {
    fn append(&self, frame: &IacFrame) -> Result<(), String> {
        self.frames.lock().push(frame.clone());
        Ok(())
    }
}

/// Nash pins host_a's retired leaf; host_a keeps presenting it after it has
/// expired. The handshake fails as a validity rejection — NOT a pin mismatch —
/// and the refusal must still land in Nash's journal, carrying the sentinel the
/// spec names.
#[tokio::test]
async fn t_14_2a_a_retired_leaf_presented_after_the_grace_is_journaled() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-rotation");
    // The RETIRED generation: correctly pinned, and no longer inside validity —
    // exactly what a peer that missed the rotation still holds.
    let retired = expired_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        // Pinned to the retired fingerprint, so this is NOT a pin mismatch: the
        // pin agrees and the CERTIFICATE is what has run out.
        vec![pin("host_a", &retired.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &retired.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        no_retry(),
    )
    .await;
    let journal = Arc::new(RecordingRuptureSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &retired,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;

    let result = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;

    assert!(
        result.is_err(),
        "a retired, expired leaf must not deliver a frame"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "the rejection is at TLS — intake must never be entered"
    );
    let landed = wait_until(
        || !journal.identity_refusals().is_empty(),
        Duration::from_secs(2),
    )
    .await;
    assert!(
        landed,
        "§7.2.1.a requires the post-grace rejection to be LOGGED; it used to leave \
         zero trace because `CertExpired` fell through the mismatch-only arm"
    );

    let refusals = journal.identity_refusals();
    let recorded_side = refusals[0]
        .to
        .first()
        .map(|address| address.spirit_id.0.clone())
        .unwrap_or_default();
    assert_eq!(
        recorded_side, "listen",
        "the row must say which side observed it: {:?}",
        refusals[0]
    );
    let peer_hint = refusals[0]
        .to
        .first()
        .and_then(|address| address.host_id.clone())
        .map(|host| host.0)
        .unwrap_or_default();
    assert!(
        peer_hint.starts_with("127.0.0.1:"),
        "the only identity available — the socket address — is carried, got {peer_hint:?}"
    );
}

/// Non-vacuous control: a peer presenting the CURRENT generation journals
/// nothing. Without this the leg above would pass against a journal that
/// recorded every connection, which is the null-control shape this epic keeps
/// finding.
#[tokio::test]
async fn t_14_2a_a_current_generation_handshake_journals_no_post_grace_row() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-rotation");
    let current = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &current.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &current.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        no_retry(),
    )
    .await;
    let journal = Arc::new(RecordingRuptureSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &current,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;

    let result = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;

    assert!(result.is_ok(), "the pinned current generation is accepted");
    assert!(
        journal.identity_refusals().is_empty(),
        "a healthy handshake produces no refusal row: {:?}",
        journal.identity_refusals()
    );
}

// The promoted-generation sentinel leg (formerly test C here, with its
// LogCapture/FieldLine helpers) moved to `t_14_2a_post_grace_token.rs` by
// Story 15-1 AC6(a) / D-7: `tracing_core`'s callsite-interest cache is
// PROCESS-global, and a co-resident subscriber-less test traversing the same
// refusal `warn!` callsite starved the capture 26/30 runs at 8 threads.
// A separate binary is a separate process; see that file's header.

// ───────────────────── Story 23-7 / AC4 — the typed post-grace reason ─────────────────────
//
// `RuptureReason::CertPostGraceReject` is journaled exactly when the refusal's SUBJECT peer (the
// dial target, or the peer a listen-side SAN names under `peer_san = "required"`) carries a
// durable promotion. Never-promoted peers, unknown leaves and SAN-less listen-side clients stay
// `PeerIdentityUnverified`.

/// Every rupture row's first rejected reason, in order. Each frame ALSO goes through the real
/// production `CohortRuptureLogSink` into an in-memory Transparency Log (review A1), so
/// [`Self::rows`] is what the daemon would have persisted; a recording sink alone skips the
/// envelope attribution that sink demands.
struct ReasonSink {
    frames: Mutex<Vec<IacFrame>>,
    log: Arc<maos_iac::TransparencyLogAdapter>,
    real: maos_cohort::CohortRuptureLogSink,
}

impl Default for ReasonSink {
    fn default() -> Self {
        let log = Arc::new(maos_iac::TransparencyLogAdapter::open_in_memory(2_314));
        Self {
            frames: Mutex::default(),
            real: maos_cohort::CohortRuptureLogSink::new(log.clone()),
            log,
        }
    }
}

impl ReasonSink {
    fn reasons(&self) -> Vec<RuptureReason> {
        self.frames
            .lock()
            .iter()
            .filter_map(|frame| match &frame.payload {
                FramePayload::ConsentRupture(payload) => {
                    payload.rejected.first().map(|rejection| rejection.reason)
                }
                _ => None,
            })
            .collect()
    }

    /// The kind-22 rows the real sink persisted: `(to_spirit_id, intent, reason)`.
    fn rows(&self) -> Vec<(String, String, RuptureReason)> {
        self.log
            .query_frames(maos_iac::adapter::transparency_log::FrameFilter {
                kind: Some(maos_iac::adapter::transparency_log::FrameKind::ConsentRupture),
                ..Default::default()
            })
            .expect("query rupture rows")
            .into_iter()
            .map(|row| {
                let payload: FramePayload =
                    serde_json::from_slice(&row.payload_redacted).expect("row payload");
                let FramePayload::ConsentRupture(rupture) = payload else {
                    panic!("a rupture row carries a rupture payload");
                };
                (row.to_spirit_id, row.intent, rupture.rejected[0].reason)
            })
            .collect()
    }
}

impl ConsentRuptureSink for ReasonSink {
    fn append(&self, frame: &IacFrame) -> Result<(), String> {
        self.frames.lock().push(frame.clone());
        self.real.append(frame)
    }
}

/// Open a window on `peer` for `next`, move plane A, and close it: the peer is now promoted.
fn promote(
    node: &maos_a2a_tcp::TcpA2ATransport,
    peer: &str,
    next: &maos_a2a_core::PeerCertFingerprint,
) {
    let id = maos_a2a_core::identity::PeerId::new(peer);
    node.pins().open_rotation_window(&id, next).expect("window");
    node.core()
        .set_peer_cert_fingerprint(&HostId(peer.to_string()), next.clone())
        .expect("plane A declares the peer");
    node.pins()
        .close_rotation_window_if(&id, next)
        .expect("promoted");
    assert!(node.pins().is_promoted(&id));
}

#[tokio::test]
async fn t_23_7_a_dial_to_a_promoted_peer_presenting_its_retired_leaf_is_cert_post_grace_reject() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-dial");
    let retired = valid_leaf(&ca, &clock);
    let current = valid_leaf(&ca, &clock);
    let dialer_leaf = valid_leaf(&ca, &clock);

    // The target still serves the RETIRED leaf.
    let target = bind_endpoint(
        &retired,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &dialer_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &dialer_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        no_retry(),
    )
    .await;
    let dialer = bind_endpoint(
        &dialer_leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &retired.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{}", target.local_addr().unwrap()),
            &retired.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;
    let journal = Arc::new(ReasonSink::default());
    dialer.core().install_rupture_sink(journal.clone()).await;
    let dialer_ref = &dialer;
    let send = |seq| async move {
        let peer = HostId("host_b".into());
        dialer_ref
            .route_outbound(
                make_frame("host_a", "host_b", IntentClass::Readonly, seq),
                &peer,
            )
            .await
    };
    assert!(
        send(1).await.is_ok(),
        "the retired leaf is the pin: accepted before any rotation"
    );
    assert!(journal.reasons().is_empty());

    // The rotation promotes `current`; the target never swapped and still presents `retired`.
    promote(&dialer, "host_b", &current.fingerprint);
    assert!(
        send(2).await.is_err(),
        "the retired leaf is refused after promotion"
    );
    assert_eq!(journal.reasons(), vec![RuptureReason::CertPostGraceReject]);
    // Through the REAL sink: exactly one durable kind-22 row, attributed to the transport event
    // (no fabricated consent intent) and carrying the dial target.
    assert_eq!(
        journal.rows(),
        vec![(
            "dial@host_b".to_string(),
            maos_cohort::PEER_IDENTITY_REFUSAL_INTENT.to_string(),
            RuptureReason::CertPostGraceReject
        )]
    );
}

#[tokio::test]
async fn t_23_7_a_never_promoted_peer_and_an_unknown_leaf_stay_peer_identity_unverified() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-never");
    let served = valid_leaf(&ca, &clock);
    let pinned = valid_leaf(&ca, &clock);
    let dialer_leaf = valid_leaf(&ca, &clock);
    let target = bind_endpoint(
        &served,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &dialer_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &dialer_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        no_retry(),
    )
    .await;
    // The dialer pins a DIFFERENT leaf for the target and never rotated it: the presented leaf is
    // simply unknown to it.
    let dialer = bind_endpoint(
        &dialer_leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &pinned.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{}", target.local_addr().unwrap()),
            &pinned.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;
    let journal = Arc::new(ReasonSink::default());
    dialer.core().install_rupture_sink(journal.clone()).await;
    assert!(dialer
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await
        .is_err());
    assert_eq!(
        journal.reasons(),
        vec![RuptureReason::PeerIdentityUnverified]
    );
    assert!(!dialer
        .pins()
        .is_promoted(&maos_a2a_core::identity::PeerId::new("host_b")));
}

#[tokio::test]
async fn t_23_7_under_required_a_listen_side_client_with_a_promoted_peers_san_and_retired_leaf() {
    use maos_a2a_tcp::PeerSanMode;
    const DOMAIN: &str = "corp.example";
    let spiffe = |peer: &str| format!("spiffe://{DOMAIN}/maos/peer/{peer}");
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-listen");
    let nash_leaf = mk_san_leaf(&ca, &clock, &[spiffe("host_b")]);
    let a_old = mk_san_leaf(&ca, &clock, &[spiffe("host_a")]);
    let a_new = mk_san_leaf(&ca, &clock, &[spiffe("host_a")]);
    let c_pinned = mk_san_leaf(&ca, &clock, &[spiffe("host_c")]);
    let c_unknown = mk_san_leaf(&ca, &clock, &[spiffe("host_c")]);
    let a_no_san = mk_san_leaf(&ca, &clock, &[]);

    let pems = write_pem(&nash_leaf, Some(&ca));
    let mut cfg = tcp_config(
        &pems,
        vec![
            pin("host_a", &a_old.fingerprint, MIRA_NONCE),
            pin("host_c", &c_pinned.fingerprint, MIRA_NONCE),
        ],
        Duration::from_secs(30),
    );
    cfg.peer_san = PeerSanMode::Required;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let nash = maos_a2a_tcp::TcpA2ATransport::bind(
        cfg,
        vec![
            peer_cfg(
                "host_a",
                "tls://127.0.0.1:0",
                &a_old.fingerprint,
                &[],
                &["readonly"],
            ),
            peer_cfg(
                "host_c",
                "tls://127.0.0.1:0",
                &c_pinned.fingerprint,
                &[],
                &["readonly"],
            ),
        ],
        NASH_NONCE,
        maos_a2a_tcp::TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("nash binds");
    let journal = Arc::new(ReasonSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    // host_a rotated and was promoted on nash: a_new is its pin, a_old is retired.
    promote(&nash, "host_a", &a_new.fingerprint);
    let nash_addr = nash.local_addr().unwrap();

    let dial = |leaf: Leaf, name: &'static str| {
        let clock = clock;
        let ca = &ca;
        let nash_leaf = &nash_leaf;
        async move {
            let client = bind_endpoint(
                &leaf,
                Some(ca),
                MIRA_NONCE,
                vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
                vec![peer_cfg(
                    "host_b",
                    &format!("tls://{nash_addr}"),
                    &nash_leaf.fingerprint,
                    &["readonly"],
                    &[],
                )],
                &clock,
                no_retry(),
            )
            .await;
            client
                .route_outbound(
                    make_frame(name, "host_b", IntentClass::Readonly, 1),
                    &HostId("host_b".into()),
                )
                .await
        }
    };

    // (ii) the SAN of a PROMOTED peer with its RETIRED leaf: the typed reason.
    assert!(dial(a_old, "host_a").await.is_err());
    assert!(
        wait_until(|| !journal.reasons().is_empty(), Duration::from_secs(3)).await,
        "the listen-side refusal is journaled"
    );
    assert_eq!(journal.reasons(), vec![RuptureReason::CertPostGraceReject]);
    // The listen-side row keeps the SAN-named subject, not the socket address (review A3).
    assert_eq!(
        journal.rows(),
        vec![(
            "listen@host_a".to_string(),
            maos_cohort::PEER_IDENTITY_REFUSAL_INTENT.to_string(),
            RuptureReason::CertPostGraceReject
        )]
    );

    // A never-promoted peer's SAN with an unknown leaf: the untyped reason.
    assert!(dial(c_unknown, "host_c").await.is_err());
    assert!(wait_until(|| journal.reasons().len() >= 2, Duration::from_secs(3)).await);
    // A leaf with no SAN names no subject — even where it stands in for a promoted peer.
    assert!(dial(a_no_san, "host_a").await.is_err());
    assert!(wait_until(|| journal.reasons().len() >= 3, Duration::from_secs(3)).await);
    assert_eq!(
        journal.reasons(),
        vec![
            RuptureReason::CertPostGraceReject,
            RuptureReason::PeerIdentityUnverified,
            RuptureReason::PeerIdentityUnverified,
        ]
    );
    // The current generation is still accepted.
    assert!(dial(a_new, "host_a").await.is_ok());
}

// Review A3 (a) — a promoted peer that RESTARTED presents its CURRENT leaf; its pin here is
// invalidated pending re-pin approval (`promoted` stays set). That refusal is not a retired-key
// signal. (The dial side never reaches TLS for an invalidated pin: the router refuses first.)
#[tokio::test]
async fn t_23_7_a_restarted_promoted_peer_with_its_current_leaf_is_not_a_retired_key_signal() {
    use maos_a2a_core::TofuPinStore as _;
    use maos_a2a_tcp::PeerSanMode;
    const DOMAIN: &str = "corp.example";
    let spiffe = |peer: &str| format!("spiffe://{DOMAIN}/maos/peer/{peer}");
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-restart");
    let nash_leaf = mk_san_leaf(&ca, &clock, &[spiffe("host_b")]);
    let a_old = mk_san_leaf(&ca, &clock, &[spiffe("host_a")]);
    let a_new = mk_san_leaf(&ca, &clock, &[spiffe("host_a")]);

    let pems = write_pem(&nash_leaf, Some(&ca));
    let mut cfg = tcp_config(
        &pems,
        vec![pin("host_a", &a_old.fingerprint, MIRA_NONCE)],
        Duration::from_secs(30),
    );
    cfg.peer_san = PeerSanMode::Required;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let nash = maos_a2a_tcp::TcpA2ATransport::bind(
        cfg,
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &a_old.fingerprint,
            &[],
            &["readonly"],
        )],
        NASH_NONCE,
        maos_a2a_tcp::TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("nash binds");
    let journal = Arc::new(ReasonSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    promote(&nash, "host_a", &a_new.fingerprint);
    // host_a restarts: nash invalidates its pin, but it is still the promoted peer.
    let peer = maos_a2a_core::identity::PeerId::new("host_a");
    nash.pins()
        .invalidate_for_restart(&peer, MIRA_NONCE)
        .await
        .unwrap();
    assert!(nash.pins().is_promoted(&peer));

    let client = bind_endpoint(
        &a_new,
        Some(&ca),
        MIRA_NONCE + 1,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{}", nash.local_addr().unwrap()),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;
    assert!(client
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await
        .is_err());
    assert!(
        wait_until(|| !journal.reasons().is_empty(), Duration::from_secs(3)).await,
        "the refusal is journaled"
    );
    assert_eq!(
        journal.reasons(),
        vec![RuptureReason::PeerIdentityUnverified],
        "a restart awaiting re-pin approval is not a retired-key presentation"
    );
    assert_eq!(journal.rows()[0].2, RuptureReason::PeerIdentityUnverified);
}

// Review A3 (b) — under `required` with `ca_roots` unset anyone can mint a self-signed leaf whose
// SAN names a promoted peer; the refusal must not be filed as that peer's retired-key signal.
#[tokio::test]
async fn t_23_7_a_forged_self_signed_leaf_naming_a_promoted_peer_is_not_a_retired_key_signal() {
    use maos_a2a_tcp::PeerSanMode;
    const DOMAIN: &str = "corp.example";
    let spiffe = |peer: &str| format!("spiffe://{DOMAIN}/maos/peer/{peer}");
    let clock = Clock::capture();
    let nash_leaf = mk_self_signed_san(&clock, &[spiffe("host_b")]);
    let a_pinned = mk_self_signed_san(&clock, &[spiffe("host_a")]);
    let a_new = mk_self_signed_san(&clock, &[spiffe("host_a")]);
    let forged = mk_self_signed_san(&clock, &[spiffe("host_a")]);

    let pems = write_pem(&nash_leaf, None);
    let mut cfg = tcp_config(
        &pems,
        vec![pin("host_a", &a_pinned.fingerprint, MIRA_NONCE)],
        Duration::from_secs(30),
    );
    cfg.peer_san = PeerSanMode::Required;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let nash = maos_a2a_tcp::TcpA2ATransport::bind(
        cfg,
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &a_pinned.fingerprint,
            &[],
            &["readonly"],
        )],
        NASH_NONCE,
        maos_a2a_tcp::TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("nash binds");
    let journal = Arc::new(ReasonSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    promote(&nash, "host_a", &a_new.fingerprint);

    let client = bind_endpoint(
        &forged,
        None,
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{}", nash.local_addr().unwrap()),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        no_retry(),
    )
    .await;
    assert!(client
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await
        .is_err());
    assert!(
        wait_until(|| !journal.reasons().is_empty(), Duration::from_secs(3)).await,
        "the refusal is journaled"
    );
    assert_eq!(
        journal.reasons(),
        vec![RuptureReason::PeerIdentityUnverified],
        "a leaf nothing chains for is no proof the promoted peer's old key was used"
    );
    assert_eq!(journal.rows()[0].0, "listen@host_a");
}
