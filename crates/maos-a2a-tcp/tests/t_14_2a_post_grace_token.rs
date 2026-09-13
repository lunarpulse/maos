//! Story 14-2a / AC4.4 — the promoted-generation §7.2.1.a sentinel, moved into
//! its own test BINARY by Story 15-1 AC6(a) / D-7 (process isolation).
//!
//! WHY A SEPARATE BINARY: the sentinel assertion captures `tracing` output via
//! a thread-local `subscriber::with_default`, but `tracing_core` caches
//! callsite `Interest` **process-globally**. In the shared binary, a
//! co-resident test that traverses the same `router.rs` refusal `warn!`
//! callsite with NO subscriber installed could win the race and fix that
//! callsite's interest to `never`, silently starving this test's capture —
//! measured 26/30 failures at `--test-threads=8` (pre-fix, this box; the
//! story's preflight measured 24/30), 0/30 alone or at 2 threads. A separate
//! binary is a separate PROCESS: no co-resident test can register the callsite
//! first, so the defect is removed by construction rather than scheduled
//! around. D-7's permitted alternative (an explicit `register_callsite`
//! returning `Interest::sometimes()`) was not needed.
//!
//! The scenario itself is unchanged from the original leg: see
//! `t_14_2a_post_grace_journal.rs` for the journal-row assertions that stay
//! there (tests A and B).

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

/// THE REAL POST-GRACE SCENARIO, built rather than approximated: a window is
/// opened and PROMOTED, so `old` becomes a genuinely RETIRED generation that is
/// still perfectly valid as an X.509 certificate — and then the peer presents
/// it anyway.
///
/// This is the case §7.2.1.a is about, and measurement says it arrives as a
/// PIN MISMATCH, not as an expiry: the retired leaf's dates are fine, it is the
/// PIN that moved. The refusal must still be a queryable row.
///
/// ⚠ **WHERE THE SPEC'S TOKEN IS ASSERTABLE.** The durable `ConsentRupture`
/// row still cannot carry `cert_post_grace_reject` as machine-readable
/// evidence (the frame `journal_peer_identity_refusal` builds has no detail
/// field), but the detail — sentinel first — IS the existing tracing surface:
/// `router.rs` warns `detail = …` on every journaled refusal. This control
/// therefore captures that surface and asserts the token the spec names,
/// verbatim, on the promoted-generation mismatch path: reverting the sentinel
/// in `transport.rs` reds THIS test.
#[test]
fn t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-rotation");
    // BOTH leaves are valid certificates. Only the PIN moves.
    let retired = valid_leaf(&ca, &clock);
    let incoming = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    // The existing tracing surface, captured: `journal_peer_identity_refusal`
    // warns the refusal detail on every journaled row, and the detail is
    // where the spec's `cert_post_grace_reject` sentinel lives.
    let logged = Arc::new(Mutex::new(Vec::<String>::new()));

    tracing::subscriber::with_default(LogCapture(logged.clone()), || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let nash = bind_endpoint(
                &nash_leaf,
                Some(&ca),
                NASH_NONCE,
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

            // A REAL rotation: open the overlap, then let the grace close it.
            let pins = nash.pins();
            let peer = maos_a2a_core::PeerId::new("host_a");
            pins.open_rotation_window(&peer, &incoming.fingerprint)
                .expect("the window opens on a live pin");
            assert!(
                pins.verify_pinned_sync(&peer, &retired.fingerprint).is_ok(),
                "during the overlap the retiring generation is still accepted"
            );
            let promoted = pins
                .close_rotation_window_if(&peer, &incoming.fingerprint)
                .expect("the grace promotes the generation this window opened");
            assert_eq!(
                promoted, retired.fingerprint,
                "and the promotion retires exactly the old generation"
            );

            // The peer missed the rotation and keeps serving the retired leaf.
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
                "after the grace the retired certificate must not deliver a frame"
            );
            assert_eq!(
                nash.intake_entered(),
                0,
                "and it is refused at TLS, before intake"
            );
            assert!(
                wait_until(
                    || !journal.identity_refusals().is_empty(),
                    Duration::from_secs(2),
                )
                .await,
                "the post-grace refusal MUST leave a queryable row — this is the single \
                 failure mode the whole rotation mechanism exists to produce"
            );
        });
    });

    // THE control assertion: the promoted-generation mismatch path logs the
    // spec's token, verbatim. Before this the token was asserted NOWHERE —
    // removing the sentinel from `transport.rs` was invisible.
    let lines = logged.lock();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("cert_post_grace_reject")),
        "§7.2.1.a requires the refusal to be logged as `cert_post_grace_reject`; \
         captured tracing: {lines:?}"
    );
}

/// Minimal tracing capture for the token assertion above — the same job as
/// the `BufWriter` + `tracing_subscriber::fmt` convention in
/// `maos-a2a-core/tests/cross_host_consent_v1_5.rs`, hand-rolled so this
/// crate's dev-dependencies stay untouched: every event's fields are
/// flattened to one `name=value …` string.
struct LogCapture(Arc<Mutex<Vec<String>>>);

impl tracing::Subscriber for LogCapture {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut line = FieldLine(String::new());
        event.record(&mut line);
        self.0.lock().push(line.0);
    }

    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// Flattens one event's fields into `name=value` pairs.
struct FieldLine(String);

impl tracing::field::Visit for FieldLine {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(&format!("{}={:?}", field.name(), value));
    }
}
