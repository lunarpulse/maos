#![forbid(unsafe_code)]

//! ADR-012 consent **refusal** proofs for the J1 loopback delegation wire
//! (story `j1-crosshost-1b`, AC1).
//!
//! `j1-crosshost-1a` proved the wire *delivers*. This file proves it *refuses*,
//! with the three deny codes kept distinct:
//!
//! | code     | meaning                                    | seam                      |
//! |----------|--------------------------------------------|---------------------------|
//! | `-32001` | classified, but not allowlisted            | send AND accept           |
//! | `-32009` | unclassified (absent/non-canonical/oversized) | send AND accept        |
//! | `-32003` | consent envelope expired                   | accept                    |
//!
//! ## Why the assertions live at the router seam and not above it
//!
//! `map_a2a_error_to_iac_bus` (`crates/maos-a2a-core/src/router.rs:1671-1783`)
//! collapses the `-32009` vocabulary into a stringly `CrossHostRouteFailure`,
//! discarding the typed `UnclassifiedReason` and the direction, and
//! `DelegationLeg::delegate` then stringifies even that. A typed
//! `-32001`-vs-`-32009` comparison is therefore **impossible** above
//! `A2ARouterCore`: one side keeps a variant, the other becomes a sentence, so
//! there is nothing to compare. That lossy mapping is filed as decision **D18**
//! (owner John + Vex at 14-4, deadline "before `j1-crosshost-2b` writes its first
//! line"); it is NOT fixed here — `maos-a2a-core` sits at zero KLOC headroom and
//! widening its error surface is a scoped decision, not a side effect.
//!
//! ## Why `-32001` needs a hand-built asymmetric pairing
//!
//! `DelegationLeg::install` (`crates/maos-bin/src/delegation.rs:102-131`) builds
//! **both** endpoints from the SAME single intent, so any intent mismatch is
//! caught by `send_admits` before the frame reaches the wire — yielding
//! `IntentDenied { direction: Send }`, never `-32001` / `IntentDeniedAtPeer`. To
//! reach `-32001` the intent must be in the **destination's** `send_allowlist`
//! and absent from the **source's** `accept_allowlist`. `LoopbackEndpoint`'s four
//! fields are all `pub`, so that asymmetry is built directly — no production
//! change, and no hand-copied identity strings: the production
//! `maos_bin::delegation` constants and `maos_a2a::pairing` helpers are used
//! throughout, because on loopback the accept allowlist is keyed by the SOURCE
//! host (`router.rs:1087-1090`) and every hand-rolled copy of that gets it wrong.
//!
//! ## Stated non-coverage, inherited by `j1-crosshost-2b`
//!
//! (a) **Rung 1 does not exercise peer authentication.** `LoopbackA2ARouter`
//! calls `handle_intake` directly (`crates/maos-a2a/src/adapter.rs:82`, `:97`),
//! and only `handle_intake_verified` binds `frame.from.host_id` to a TLS-verified
//! peer. The field that selects *which* `accept_allowlist` applies is written by
//! the sender and never verified: **a frame picks its own judge.** Every refusal
//! proved below is one string assignment away from selecting a different
//! allowlist. Survivable in-process; NOT the inherited claim that rung 1 "proves
//! the wire so rung 2 only adds network".
//!
//! (b) **The production error path conflates these codes** (D18, above). A
//! cross-host operator consuming `IacBusError` cannot tell `-32001` from
//! `-32009`, and cannot see an unclassified reason at all.
//!
//! ## Enrollment is the control
//!
//! No CI job runs `-p maos-bin` unscoped; every invocation names explicit
//! `--test` targets, so this file only executes because
//! `.github/workflows/discipline.yml`'s `check-j1-loopback-delegation` job names
//! `--test consent_refusal_1b`. `check-j1-loopback-delegation`'s
//! `consent-refusal-proofs` leg reads this file structurally and its
//! `completion-vectors-enrolled` leg derives the enrolled set from
//! `crates/maos-bin/tests/`, so deleting either the assertions or the enrollment
//! line reds a `Blocking` gate.

use std::sync::Arc;

use maos_a2a::adapter::A2APeerRouter;
use maos_a2a::pairing::{paired_loopback_router, LoopbackEndpoint};
use maos_a2a::LoopbackA2ARouter;
use maos_a2a_core::error::{A2AError, IntentDirection, UnclassifiedReason};
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_CONSENT_EXPIRED, CODE_CONSENT_UNCLASSIFIED,
    CODE_INTENT_DENIED, METHOD_IAC_DELIVER,
};
use maos_bin::delegation::{FROM_HOST, FROM_SPIRIT, RECIPIENT_SPIRIT, TO_HOST};
use maos_domain::frame::IacFrame;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i8::{A2AIntent, MAX_CANONICAL_INTENT_LEN};
use maos_spirit_abi::identity::{HostId, SpiritRole};
use orchestrator::{Orchestrator, DELEGATION_CONSENT_INTENT};
use tokio::sync::mpsc::UnboundedReceiver;

const TEST_RUN_NONCE: u64 = 0x1B0000;

/// A canonical intent the operator did NOT grant. Canonical on purpose: an
/// un-allowlisted **classified** intent is the only thing `-32001` means, and a
/// non-canonical one would deny as `-32009` at a different seam entirely.
const DISALLOWED_INTENT: &str = "development-task:destroy-workspace";

/// Build a loopback pair with an EXPLICIT, possibly asymmetric allowlist on each
/// side. `sender_of` / `acceptor_of` cannot express the asymmetry `-32001`
/// requires, so the fields are set directly (all four are `pub`).
///
/// `to_send` is the DESTINATION peer's `send_allowlist` (checked by
/// `prepare_outbound`); `from_accept` is the SOURCE peer's `accept_allowlist`
/// (checked by `handle_intake`, keyed by `frame.from.host_id`). The ports match
/// the production `DelegationLeg::install` pairing.
async fn asymmetric_pair(
    to_send: &[&str],
    from_accept: &[&str],
) -> (
    Arc<LoopbackA2ARouter>,
    tokio::sync::mpsc::Receiver<IacFrame>,
) {
    let intents = |names: &[&str]| names.iter().map(|n| A2AIntent::new(*n)).collect::<Vec<_>>();
    paired_loopback_router(&[
        LoopbackEndpoint {
            host: TO_HOST.to_string(),
            port: 7451,
            send_allowlist: intents(to_send),
            accept_allowlist: Vec::new(),
        },
        LoopbackEndpoint {
            host: FROM_HOST.to_string(),
            port: 7452,
            send_allowlist: Vec::new(),
            accept_allowlist: intents(from_accept),
        },
    ])
    .await
    .expect("the loopback pairing validates and TOFU-pins both endpoints")
}

/// A real production delegation frame: emitted by the production
/// `Orchestrator::assign_frame_remote` with the production host/spirit
/// constants, carrying `envelope_intent` in its consent envelope.
fn delegation_frame(envelope_intent: &str) -> IacFrame {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let emitter = Orchestrator::new(FROM_SPIRIT);
    emitter
        .assign_frame_remote(
            1,
            TEST_RUN_NONCE,
            RECIPIENT_SPIRIT,
            SpiritRole::Worker,
            emitter.build_task_assign("write the workspace", "exit 0", None),
            // The lineage must be canonical regardless of what the envelope
            // carries — `assign_frame_remote` rejects a non-canonical lineage
            // before the envelope is ever built.
            IntentLineage::new(vec![A2AIntent::new(DELEGATION_CONSENT_INTENT)]),
            TO_HOST,
            FROM_HOST,
            A2AIntent::new(envelope_intent),
        )
        .expect("the lineage is canonical and non-empty")
}

fn intake_request(frame: IacFrame) -> A2AJsonRpcRequest {
    A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame, 1)
}

/// The four shapes `consent_decision` calls **Unclassified**, paired with the
/// reason each must produce. `Absent` is reachable two ways — no envelope at all,
/// and an envelope whose `intent_class` is `None` — and both are covered because
/// they are different production bugs.
fn unclassified_vectors() -> Vec<(&'static str, IacFrame, UnclassifiedReason)> {
    let mut no_envelope = delegation_frame(DELEGATION_CONSENT_INTENT);
    no_envelope.consent_envelope = None;

    let mut envelope_without_intent = delegation_frame(DELEGATION_CONSENT_INTENT);
    envelope_without_intent
        .consent_envelope
        .as_mut()
        .expect("assign_frame_remote always builds an envelope")
        .intent_class = None;

    vec![
        (
            "no consent_envelope",
            no_envelope,
            UnclassifiedReason::Absent,
        ),
        (
            "envelope with intent_class: None",
            envelope_without_intent,
            UnclassifiedReason::Absent,
        ),
        (
            "non-canonical intent (`task.assign` — the `.` is illegal)",
            delegation_frame("task.assign"),
            UnclassifiedReason::NonCanonical,
        ),
        (
            "oversized intent (129 bytes > MAX_CANONICAL_INTENT_LEN)",
            delegation_frame(&"a".repeat(MAX_CANONICAL_INTENT_LEN + 1)),
            UnclassifiedReason::Oversized,
        ),
    ]
}

/// Read the typed `UnclassifiedReason` out of a `-32009` NACK's `data`. A
/// numeric-only assertion makes the deny illegible, which is the defect
/// `crates/maos-a2a-core/tests/fail_closed_8_8.rs:128-135` exists to prevent.
fn nack_reason(nack: &maos_a2a_core::NackError) -> UnclassifiedReason {
    nack.data
        .as_ref()
        .and_then(|d| d.get("reason"))
        .and_then(|v| serde_json::from_value::<UnclassifiedReason>(v.clone()).ok())
        .expect("the -32009 NACK carries a typed reason in `data`")
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1.1 — the LOCAL positive control.
//
// Without a working positive in the same test binary, every negative below can
// pass vacuously: a pairing that admits nothing "refuses" everything. The
// end-to-end production positive already exists and runs in CI
// (`crates/maos-journey-test/tests/journey_j1.rs:107`,
// `crates/maos-bin/tests/delegation_leg_1a.rs:75-87`) and is NOT rebuilt here.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn allowlisted_delegation_intent_is_admitted_and_its_intent_class_delivered() {
    let (router, mut rx) =
        asymmetric_pair(&[DELEGATION_CONSENT_INTENT], &[DELEGATION_CONSENT_INTENT]).await;

    <LoopbackA2ARouter as A2APeerRouter>::route_outbound(
        &router,
        delegation_frame(DELEGATION_CONSENT_INTENT),
        &HostId(TO_HOST.to_string()),
    )
    .await
    .expect("the granted delegation intent routes and is admitted at both seams");

    let delivered = rx
        .try_recv()
        .expect("an admitted frame reaches the peer's intake sink");
    assert_eq!(
        delivered
            .consent_envelope
            .and_then(|e| e.intent_class)
            .map(|i| i.as_str().to_string()),
        Some(DELEGATION_CONSENT_INTENT.to_string()),
        "the delivered frame must carry the granted intent_class, not a band projection"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1.2 — `-32001` / `IntentDeniedAtPeer`, via the asymmetric pairing.
// ─────────────────────────────────────────────────────────────────────────────

/// The peer-side deny, and the source-keying asymmetry made observable: the NACK
/// message names **`founder-loop-host`** — the SOURCE host whose
/// `accept_allowlist` was consulted — while `IntentDeniedAtPeer.peer` names the
/// DESTINATION the frame was addressed to. Two different hosts in one refusal is
/// the whole point; an assertion phrased "Host B's accept_allowlist admits X"
/// would be wrong on loopback.
#[tokio::test]
async fn disallowed_intent_is_denied_at_peer_with_minus_32001_naming_the_source_host() {
    // Both peer_id strings, literally: the refusal is only legible if the two
    // hosts are distinguishable, and a production rename must red this test
    // rather than silently re-point the asymmetry.
    assert_eq!(TO_HOST, "developer-remote-host");
    assert_eq!(FROM_HOST, "founder-loop-host");

    // Destination MAY send it; source does NOT accept it. This is the only
    // configuration that reaches `-32001` — see the module docs.
    let (router, mut rx) =
        asymmetric_pair(&[DISALLOWED_INTENT], &[DELEGATION_CONSENT_INTENT]).await;

    let error = <LoopbackA2ARouter as A2APeerRouter>::route_outbound(
        &router,
        delegation_frame(DISALLOWED_INTENT),
        &HostId(TO_HOST.to_string()),
    )
    .await
    .expect_err("an intent absent from the source's accept_allowlist must be refused");

    match error {
        A2AError::IntentDeniedAtPeer { peer, message } => {
            assert_eq!(
                peer, TO_HOST,
                "`peer` is the DESTINATION the frame was routed to"
            );
            assert!(
                message.contains(FROM_HOST),
                "the peer's NACK must name the SOURCE host whose accept_allowlist was \
                 consulted (`{FROM_HOST}`) — that is the loopback source-keying asymmetry; \
                 got: {message}"
            );
            assert!(
                message.contains(DISALLOWED_INTENT),
                "the NACK must name the denied intent so the refusal is legible in the \
                 Transparency Log; got: {message}"
            );
        }
        other => panic!("expected -32001 IntentDeniedAtPeer, got {other:?}"),
    }

    assert!(
        rx.try_recv().is_err(),
        "a refused frame must NEVER reach the intake sink"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1.3 — `-32009` / `ConsentUnclassified` at BOTH seams, reason typed.
// ─────────────────────────────────────────────────────────────────────────────

/// Send seam: `prepare_outbound` denies unclassified BEFORE the send-allowlist,
/// so the frame never leaves. Every reachable `UnclassifiedReason` is covered.
#[tokio::test]
async fn unclassified_at_the_send_seam_denies_every_reachable_reason() {
    let (router, _rx) =
        asymmetric_pair(&[DELEGATION_CONSENT_INTENT], &[DELEGATION_CONSENT_INTENT]).await;

    for (label, frame, expected) in unclassified_vectors() {
        let error = <LoopbackA2ARouter as A2APeerRouter>::route_outbound(
            &router,
            frame,
            &HostId(TO_HOST.to_string()),
        )
        .await
        .unwrap_err();
        match error {
            A2AError::ConsentUnclassified {
                direction: IntentDirection::Send,
                reason,
            } => assert_eq!(reason, expected, "send-seam deny reason for {label}"),
            other => panic!("expected send-seam ConsentUnclassified for {label}, got {other:?}"),
        }
    }
}

/// Accept seam: driven through `handle_intake` DIRECTLY, because
/// `prepare_outbound` denies first and would never let an unclassified frame
/// reach intake. The reason is asserted from the NACK's typed `data`.
#[tokio::test]
async fn unclassified_at_the_accept_seam_denies_every_reachable_reason_with_a_typed_reason() {
    let (router, mut rx) =
        asymmetric_pair(&[DELEGATION_CONSENT_INTENT], &[DELEGATION_CONSENT_INTENT]).await;

    for (label, frame, expected) in unclassified_vectors() {
        match <LoopbackA2ARouter as A2APeerRouter>::handle_intake(&router, intake_request(frame))
            .await
        {
            A2AJsonRpcResponse::Nack(nack) => {
                assert_eq!(
                    nack.error.code, CODE_CONSENT_UNCLASSIFIED,
                    "accept-seam code for {label}"
                );
                assert_eq!(
                    nack_reason(&nack.error),
                    expected,
                    "accept-seam reason for {label}"
                );
            }
            other => panic!("expected -32009 NACK for {label}, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "an unclassified frame must never reach the intake sink ({label})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1.4 — non-conflation, asserted in BOTH directions across all THREE codes.
// ─────────────────────────────────────────────────────────────────────────────

/// Direction 1: the classified-but-not-allowlisted leg must be `-32001` and
/// **never** `-32009`, at both seams.
#[tokio::test]
async fn classified_not_allowlisted_is_minus_32001_never_minus_32009() {
    // Accept seam: source accepts nothing, so the classified frame is denied as
    // not-allowlisted — not as unclassified.
    let (accept_side, _rx) = asymmetric_pair(&[DISALLOWED_INTENT], &[]).await;
    match <LoopbackA2ARouter as A2APeerRouter>::handle_intake(
        &accept_side,
        intake_request(delegation_frame(DISALLOWED_INTENT)),
    )
    .await
    {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(
                nack.error.code, CODE_INTENT_DENIED,
                "classified-but-not-allowlisted is -32001"
            );
            assert_ne!(
                nack.error.code, CODE_CONSENT_UNCLASSIFIED,
                "a policy deny must NEVER be reported as unclassified — the operator would \
                 read a granted-but-refused intent as a malformed frame"
            );
        }
        other => panic!("expected -32001 NACK, got {other:?}"),
    }

    // Send seam: the destination does not permit sending it at all.
    let (send_side, _rx) = asymmetric_pair(&[], &[DISALLOWED_INTENT]).await;
    match <LoopbackA2ARouter as A2APeerRouter>::route_outbound(
        &send_side,
        delegation_frame(DISALLOWED_INTENT),
        &HostId(TO_HOST.to_string()),
    )
    .await
    .unwrap_err()
    {
        A2AError::IntentDenied {
            direction: IntentDirection::Send,
            inner,
        } => {
            assert_eq!(inner.intent, DISALLOWED_INTENT);
            assert_eq!(inner.peer, TO_HOST);
        }
        other @ A2AError::ConsentUnclassified { .. } => {
            panic!("send-seam policy deny conflated into unclassified: {other:?}")
        }
        other => panic!("expected send-seam IntentDenied, got {other:?}"),
    }
}

/// Direction 2 — the both-ways shape: the unclassified leg must be `-32009` and
/// **never** `-32001`, at both seams. Asserting only direction 1 would leave the
/// inverse conflation (a malformed frame reported as a policy refusal) untested.
#[tokio::test]
async fn unclassified_is_minus_32009_never_minus_32001() {
    // Accept seam: the source accepts the canonical intent, so the ONLY reason
    // these frames are refused is that they are unclassified. If the codes were
    // conflated this would come back -32001.
    let (router, _rx) =
        asymmetric_pair(&[DELEGATION_CONSENT_INTENT], &[DELEGATION_CONSENT_INTENT]).await;
    for (label, frame, _) in unclassified_vectors() {
        match <LoopbackA2ARouter as A2APeerRouter>::handle_intake(&router, intake_request(frame))
            .await
        {
            A2AJsonRpcResponse::Nack(nack) => assert_ne!(
                nack.error.code, CODE_INTENT_DENIED,
                "unclassified must NEVER be reported as a policy deny ({label}) — an operator \
                 would go hunting an allowlist for a frame that never carried an intent"
            ),
            other => panic!("expected a NACK for {label}, got {other:?}"),
        }
    }

    // Send seam: same allowlists; an unclassified frame is refused BEFORE
    // `send_admits` runs, so it cannot surface as `IntentDenied`.
    for (label, frame, _) in unclassified_vectors() {
        let error = <LoopbackA2ARouter as A2APeerRouter>::route_outbound(
            &router,
            frame,
            &HostId(TO_HOST.to_string()),
        )
        .await
        .unwrap_err();
        assert!(
            !matches!(error, A2AError::IntentDenied { .. }),
            "send-seam unclassified conflated into IntentDenied ({label}): {error:?}"
        );
    }
}

/// The THIRD code. Per Story 8.9 `prepare_outbound` stamps a TTL on any envelope
/// carrying `None`, and `handle_intake_inner` enforces it — so expiry is live on
/// every real frame, and it must stay distinct from both deny codes. An envelope
/// with an EXPLICIT `valid_until_ns` is left untouched by the transport (the
/// granter is authoritative), which is what makes this vector constructible.
#[tokio::test]
async fn expired_envelope_is_minus_32003_and_neither_deny_code() {
    let (router, mut rx) =
        asymmetric_pair(&[DELEGATION_CONSENT_INTENT], &[DELEGATION_CONSENT_INTENT]).await;

    let mut frame = delegation_frame(DELEGATION_CONSENT_INTENT);
    frame
        .consent_envelope
        .as_mut()
        .expect("assign_frame_remote always builds an envelope")
        .valid_until_ns = Some(1);

    match <LoopbackA2ARouter as A2APeerRouter>::handle_intake(&router, intake_request(frame)).await
    {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(
                nack.error.code, CODE_CONSENT_EXPIRED,
                "an expired envelope is -32003"
            );
            assert_ne!(nack.error.code, CODE_INTENT_DENIED);
            assert_ne!(nack.error.code, CODE_CONSENT_UNCLASSIFIED);
        }
        other => panic!("expected -32003 NACK, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "an expired frame must never reach the intake sink"
    );
}
