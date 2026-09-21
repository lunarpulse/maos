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
use maos_a2a_tcp::TcpTimeouts;
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
        TcpTimeouts::test_profile(),
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
        TcpTimeouts::test_profile(),
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
        TcpTimeouts::test_profile(),
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
        TcpTimeouts::test_profile(),
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
