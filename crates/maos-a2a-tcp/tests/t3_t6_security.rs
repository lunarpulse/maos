//! Security negative tests — AC-T3 (pin mismatch), AC-T4 (wrong-CA chain
//! rejection, chain mode), AC-T4b (pin-only posture), AC-T5 (expiry + retry),
//! AC-T6 (MITM cert-swap after pin). These consume the `TofuPinningVerifier`
//! (AC-A3) as the unit under test; the error taxonomy must keep "TOFU mismatch"
//! and "bad-cert / untrusted-issuer" distinguishable.

mod support;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, HandshakeRetryPolicy, PeerId, TofuPinStore};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;

/// Build a Mira→Nash pair and perform ONE outbound dial. `nash_serves` is the
/// cert Nash presents; `mira_pins_nash` is what Mira pins for host_b (allowing
/// deliberate mismatches). Returns the dial result + the live Nash transport
/// (for `intake_entered`) + the live Mira transport (for pin-store asserts).
#[allow(clippy::too_many_arguments)]
async fn mira_dials_nash(
    clock: &Clock,
    mira_ca: Option<&Ca>,
    nash_ca: Option<&Ca>,
    mira_leaf: &Leaf,
    nash_serves: &Leaf,
    mira_pins_nash: &maos_a2a_core::identity::PeerCertFingerprint,
    nash_pins_mira: &maos_a2a_core::identity::PeerCertFingerprint,
    retry: HandshakeRetryPolicy,
) -> (TcpA2ATransport, TcpA2ATransport, Result<(), A2AError>) {
    let nash = bind_endpoint(
        nash_serves,
        nash_ca,
        NASH_NONCE,
        vec![pin("host_a", nash_pins_mira, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            nash_pins_mira,
            &[],
            &["readonly"],
        )],
        clock,
        TcpTimeouts::test_profile(),
        retry.clone(),
    )
    .await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        mira_leaf,
        mira_ca,
        MIRA_NONCE,
        vec![pin("host_b", mira_pins_nash, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            mira_pins_nash,
            &["readonly"],
            &[],
        )],
        clock,
        TcpTimeouts::test_profile(),
        retry,
    )
    .await;

    let res = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;
    (mira, nash, res)
}

/// AC-T3 — TOFU pin mismatch (valid cert, wrong identity) → handshake REJECTED.
#[tokio::test]
async fn t3_tofu_pin_mismatch_rejected() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_a = valid_leaf(&ca, &clock); // server actually serves fp_B...
    let nash_b = valid_leaf(&ca, &clock); // ...but Mira pinned fp_A (nash_a)

    let (_mira, nash, res) = mira_dials_nash(
        &clock,
        Some(&ca),
        Some(&ca),
        &mira_leaf,
        &nash_b,             // Nash serves fp_B
        &nash_a.fingerprint, // Mira pins fp_A ≠ fp_B
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;

    let err = res.expect_err("AC-T3: unpinned server identity must be rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("pin_mismatch") || msg.contains("pin mismatch"),
        "AC-T3: must classify as TOFU pin mismatch (not generic IO), got: {err}"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T3: rejection at TLS → intake never entered"
    );
}

/// AC-T4 — wrong CA (valid-but-untrusted root) → REJECTED at the chain layer,
/// even when the fingerprint is COINCIDENTALLY pinned. Chain mode only.
#[tokio::test]
async fn t4_wrong_ca_rejected_at_chain_layer_even_if_pinned() {
    let clock = Clock::capture();
    let ca_good = mk_ca(&clock, "ca-good");
    let ca_evil = mk_ca(&clock, "ca-evil");
    let mira_leaf = valid_leaf(&ca_good, &clock);
    let nash_evil = mk_leaf_signed_by(&ca_evil, &clock, -HOUR, HOUR); // well-formed, in-validity

    // Mira runs chain mode (ca_good) and — to prove the discriminating clause —
    // COINCIDENTALLY pins the evil leaf's fingerprint.
    let (_mira, nash, res) = mira_dials_nash(
        &clock,
        Some(&ca_good),
        Some(&ca_good),
        &mira_leaf,
        &nash_evil,             // Nash serves a ca_evil leaf
        &nash_evil.fingerprint, // ...whose fp IS pinned (coincidence)
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;

    let err = res.expect_err("AC-T4: untrusted-CA leaf must be rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(
        !(msg.contains("pin_mismatch") || msg.contains("pin mismatch")),
        "AC-T4: rejection must be at the CHAIN layer, NOT pin mismatch (fp was pinned), got: {err}"
    );
    assert!(
        msg.contains("bad_certificate") || msg.contains("untrusted") || msg.contains("certificate"),
        "AC-T4: must classify as bad-cert / untrusted-issuer, got: {err}"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T4: chain rejection → intake never entered"
    );
}

/// AC-T4b (1) — pin-only posture: unpinned leaf → REJECTED at the pin step.
#[tokio::test]
async fn t4b_pin_only_unpinned_leaf_rejected_at_pin() {
    let clock = Clock::capture();
    let mira_leaf = mk_self_signed(&clock, -HOUR, HOUR);
    let nash_a = mk_self_signed(&clock, -HOUR, HOUR); // Mira pins this...
    let nash_b = mk_self_signed(&clock, -HOUR, HOUR); // ...Nash serves this (unpinned)

    let (_mira, nash, res) = mira_dials_nash(
        &clock,
        None, // pin-only
        None,
        &mira_leaf,
        &nash_b,
        &nash_a.fingerprint, // pinned fp ≠ served fp
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;

    let err = res.expect_err("AC-T4b: pin-only unpinned leaf must be rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("pin_mismatch") || msg.contains("pin mismatch"),
        "AC-T4b: in pin-only mode AC-T3/the pin step is the trust oracle, got: {err}"
    );
    assert_eq!(nash.intake_entered(), 0);
}

/// AC-T4b (2) — pin-only happy path: a valid, pinned self-signed leaf round-trips
/// with NO roots configured (proves None is not fail-closed-on-everything).
#[tokio::test]
async fn t4b_pin_only_happy_path_roundtrips() {
    let clock = Clock::capture();
    let mira_leaf = mk_self_signed(&clock, -HOUR, HOUR);
    let nash_leaf = mk_self_signed(&clock, -HOUR, HOUR);

    let (_mira, _nash, res) = mira_dials_nash(
        &clock,
        None,
        None,
        &mira_leaf,
        &nash_leaf,
        &nash_leaf.fingerprint, // correctly pinned
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;
    res.expect("AC-T4b: pin-only valid pinned leaf must round-trip");
}

/// AC-T4b (3) — pin-only expiry: the validity gate still fires without a chain.
#[tokio::test]
async fn t4b_pin_only_expiry_still_rejected() {
    let clock = Clock::capture();
    let mira_leaf = mk_self_signed(&clock, -HOUR, HOUR);
    let nash_expired = mk_self_signed(&clock, -2 * HOUR, -HOUR); // expired

    let (_mira, nash, res) = mira_dials_nash(
        &clock,
        None,
        None,
        &mira_leaf,
        &nash_expired,
        &nash_expired.fingerprint, // pinned, but expired
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;
    let err = res.expect_err("AC-T4b: expired self-signed leaf must be rejected on validity");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("expired") || msg.contains("certificate"),
        "AC-T4b: pin-only validity gate must fire, got: {err}"
    );
    assert_eq!(nash.intake_entered(), 0);
}

/// AC-T5 — expired cert → REJECTED, retry policy engages on the cert code.
#[tokio::test]
async fn t5_expired_cert_retries_then_fails() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_expired = expired_leaf(&ca, &clock);

    let retry = HandshakeRetryPolicy {
        backoff_ms: vec![10, 10],
        jitter_pct: 0,
        max_attempts: 3,
    };
    let (mira, nash, res) = mira_dials_nash(
        &clock,
        Some(&ca),
        Some(&ca),
        &mira_leaf,
        &nash_expired,
        &nash_expired.fingerprint,
        &mira_leaf.fingerprint,
        retry.clone(),
    )
    .await;

    let err = res.expect_err("AC-T5: expired server cert must terminally fail");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("expired") || msg.contains("certificate"),
        "AC-T5: terminal cert-class err, got: {err}"
    );
    assert_eq!(
        mira.last_dial_attempts(),
        retry.max_attempts as usize,
        "AC-T5: retries fired up to policy.max_attempts"
    );
    assert_eq!(nash.intake_entered(), 0, "AC-T5: never entered intake");
}

/// AC-T5 (case 2) — not-yet-valid cert → REJECTED on validity.
#[tokio::test]
async fn t5_not_yet_valid_cert_rejected() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_future = not_yet_valid_leaf(&ca, &clock);

    let (_mira, nash, res) = mira_dials_nash(
        &clock,
        Some(&ca),
        Some(&ca),
        &mira_leaf,
        &nash_future,
        &nash_future.fingerprint,
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;
    res.expect_err("AC-T5: not-yet-valid server cert must be rejected");
    assert_eq!(nash.intake_entered(), 0);
}

/// AC-T6 — MITM cert-swap after pin (TOFU defends rotation). A valid prior pin
/// (fp_A) exists; a new connection presents fp_C ≠ fp_A (also ca_good). The pin
/// must win and must NOT be silently overwritten.
#[tokio::test]
async fn t6_mitm_cert_swap_after_pin_rejected() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_a = valid_leaf(&ca, &clock); // the legitimately pinned identity (fp_A)
    let nash_c = valid_leaf(&ca, &clock); // the attacker's swapped cert (fp_C, ca_good)

    let (mira, nash, res) = mira_dials_nash(
        &clock,
        Some(&ca),
        Some(&ca),
        &mira_leaf,
        &nash_c,             // Nash now serves fp_C
        &nash_a.fingerprint, // Mira holds the prior pin fp_A
        &mira_leaf.fingerprint,
        no_retry(),
    )
    .await;

    let err = res.expect_err("AC-T6: swapped cert (fp_C ≠ pinned fp_A) must be rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("pin_mismatch") || msg.contains("pin mismatch"),
        "AC-T6: TOFU mismatch (prior pin must win), got: {err}"
    );
    assert_eq!(nash.intake_entered(), 0);

    // The prior pin must still hold fp_A — not silently overwritten by fp_C.
    let pin_now = mira
        .pins()
        .get_pin(&PeerId::new("host_b"))
        .await
        .expect("mira still holds a pin for host_b");
    assert_eq!(
        pin_now.fingerprint, nash_a.fingerprint,
        "AC-T6: pin store still holds fp_A (not overwritten)"
    );
}
