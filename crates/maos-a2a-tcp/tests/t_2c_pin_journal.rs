//! Story `j1-crosshost-2c` AC3.6/AC3.7 — journal the pin mismatch on **both**
//! sides, and assert the listen side on the SERVER's journal.
//!
//! All three shipped pin-mismatch tests (`t3_tofu_pin_mismatch_rejected`,
//! `t4b_pin_only_unpinned_leaf_rejected_at_pin`,
//! `t6_mitm_cert_swap_after_pin_rejected`) drive `mira_dials_nash` and assert on
//! the DIALER's error class. No test exercised the listen side rejecting a client
//! cert — and that is the weaker side: `find_active_pin_by_fingerprint` accepts
//! ANY active pin there, while per-peer scoping exists only on the dial side.
//!
//! It is also the side that left **zero trace**: the refusal happened inside the
//! rustls verifier callback, which provably cannot reach a Transparency Log, and
//! the connection task then took a blanket `_ => return`.
//!
//! Under TLS 1.3 the dialer may not even observe the rejection — the server can
//! close after the client's `connect()` resolves, surfacing as `Io` rather than
//! `PinMismatch` — so **these legs assert on the server's journal, never on the
//! dialer's error class.**
//!
//! Kept fast on purpose: `TcpTimeouts::test_profile()` is 250ms and every test in
//! this directory runs 51x per push inside a 10-minute cap.

mod support;

use std::sync::Arc;
use std::time::Duration;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{ConsentRuptureSink, HandshakeRetryPolicy};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::frame::{FramePayload, IacFrame, RuptureReason};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::{FrameKind, HostId};
use parking_lot::Mutex;
use support::*;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;

/// A rupture sink that records what the router actually handed it. Not a flag:
/// the legs below read the frame's kind, reason and addresses.
#[derive(Default)]
struct RecordingRuptureSink {
    frames: Mutex<Vec<IacFrame>>,
}

impl RecordingRuptureSink {
    fn identity_refusals(&self) -> Vec<IacFrame> {
        self.frames
            .lock()
            .iter()
            .filter(|f| {
                f.kind == FrameKind::ConsentRupture
                    && matches!(&f.payload, FramePayload::ConsentRupture(p)
                        if p.rejected.iter().any(|r| r.reason == RuptureReason::PeerIdentityUnverified))
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

/// AC3.7 — Nash (the SERVER) refuses Mira's client leaf because Nash never
/// pinned it, and the refusal lands in Nash's own journal.
#[tokio::test]
async fn t_2c_listen_side_pin_mismatch_lands_in_the_servers_journal() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let stranger = valid_leaf(&ca, &clock); // well-formed, in-validity, NOT Mira
    let nash_leaf = valid_leaf(&ca, &clock);

    // Nash pins a leaf for host_a that is NOT the one Mira will present.
    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &stranger.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &stranger.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let journal = Arc::new(RecordingRuptureSink::default());
    nash.core().install_rupture_sink(journal.clone()).await;
    let nash_addr = nash.local_addr().unwrap();

    // Mira presents her own (unpinned-by-Nash) leaf but pins Nash correctly, so
    // the DIAL side has nothing to complain about — the refusal must come from
    // the listen side.
    let mira = bind_endpoint(
        &mira_leaf,
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
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    let res = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;
    assert!(
        res.is_err(),
        "an unpinned client leaf must not deliver a frame"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "rejection is at TLS — intake must never be entered"
    );

    // THE ASSERTION THAT MATTERS. Not the dialer's error class: the server's row.
    let landed = wait_until(
        || !journal.identity_refusals().is_empty(),
        Duration::from_secs(2),
    )
    .await;
    assert!(
        landed,
        "the listen side must journal a PeerIdentityUnverified rupture; it used to \
         leave zero trace"
    );

    let refusals = journal.identity_refusals();
    let rupture = &refusals[0];
    assert_eq!(rupture.kind, FrameKind::ConsentRupture);
    // The journal records WHICH side spoke — the two sides are not equally strong.
    let recorded_side = rupture
        .to
        .first()
        .map(|a| a.spirit_id.0.clone())
        .unwrap_or_default();
    assert_eq!(
        recorded_side, "listen",
        "the row must say the listen side observed it: {rupture:?}"
    );
    // And the only identity available — the socket address — is carried, not lost.
    let peer_hint = rupture
        .to
        .first()
        .and_then(|a| a.host_id.clone())
        .map(|h| h.0)
        .unwrap_or_default();
    assert!(
        peer_hint.starts_with("127.0.0.1:"),
        "the unverified peer's socket address must be recorded, got {peer_hint:?}"
    );
}

/// AC3.6 — the DIAL side journals too. It was already typed at the composition
/// root, but it left no queryable row either, so "journal on both sides" was
/// half-true.
#[tokio::test]
async fn t_2c_dial_side_pin_mismatch_also_lands_in_the_dialers_journal() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_serves = valid_leaf(&ca, &clock);
    let mira_pinned = valid_leaf(&ca, &clock); // ≠ what Nash serves

    let nash = bind_endpoint(
        &nash_serves,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &mira_leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &mira_pinned.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &mira_pinned.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let journal = Arc::new(RecordingRuptureSink::default());
    mira.core().install_rupture_sink(journal.clone()).await;

    let err = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 2),
            &HostId("host_b".into()),
        )
        .await
        .expect_err("a pinned-elsewhere server leaf must be refused");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("pin_mismatch") || msg.contains("pin mismatch"),
        "the dial side keeps its typed class: {err}"
    );

    let refusals = journal.identity_refusals();
    assert_eq!(
        refusals.len(),
        1,
        "the dial side must journal exactly one identity refusal, not zero and not \
         one per retry attempt"
    );
    let recorded_side = refusals[0]
        .to
        .first()
        .map(|a| a.spirit_id.0.clone())
        .unwrap_or_default();
    assert_eq!(recorded_side, "dial");
    let peer_hint = refusals[0]
        .to
        .first()
        .and_then(|a| a.host_id.clone())
        .map(|h| h.0)
        .unwrap_or_default();
    assert_eq!(
        peer_hint, "host_b",
        "the dial side knows the intended peer id, so it records that"
    );
}

/// Non-vacuous control: a HEALTHY handshake journals nothing. Without this the
/// legs above could pass against a sink that recorded every connection.
#[tokio::test]
async fn t_2c_a_healthy_handshake_journals_no_identity_refusal() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let nash_journal = Arc::new(RecordingRuptureSink::default());
    nash.core().install_rupture_sink(nash_journal.clone()).await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &mira_leaf,
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
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let mira_journal = Arc::new(RecordingRuptureSink::default());
    mira.core().install_rupture_sink(mira_journal.clone()).await;

    mira.route_outbound(
        make_frame("host_a", "host_b", IntentClass::Readonly, 3),
        &HostId("host_b".into()),
    )
    .await
    .expect("a correctly pinned pair must deliver");

    assert!(
        mira_journal.identity_refusals().is_empty(),
        "a healthy dial must journal no identity refusal"
    );
    assert!(
        nash_journal.identity_refusals().is_empty(),
        "a healthy accept must journal no identity refusal"
    );
    assert_eq!(
        nash.intake_entered(),
        1,
        "the frame must have been taken in"
    );
}

/// The seam must fail loudly when no sink is installed rather than silently
/// dropping the evidence — a deployment that expects journaling and has none is
/// a configuration error, not a quiet success.
#[tokio::test]
async fn t_2c_journaling_without_a_sink_is_an_error_not_a_silent_success() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let leaf = valid_leaf(&ca, &clock);
    let endpoint: TcpA2ATransport = bind_endpoint(
        &leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![],
        vec![],
        &clock,
        TcpTimeouts::test_profile(),
        HandshakeRetryPolicy::default(),
    )
    .await;
    // No rupture sink installed.
    let err = endpoint
        .core()
        .journal_peer_identity_refusal(
            maos_a2a_core::PeerRefusalDirection::Listen,
            "127.0.0.1:1",
            "synthetic",
        )
        .await
        .expect_err("a missing sink must surface");
    assert!(err.contains("not installed"), "{err}");
}
