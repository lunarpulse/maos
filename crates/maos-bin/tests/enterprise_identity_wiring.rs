//! Story 11.4c — **Task 5** composition-root wiring (GREEN phase).
//!
//! These tests pin the AC5 contract for the enterprise-identity / at-rest /
//! SIEM composition root that lives in `maos-bin`:
//!
//! 1. **Zero-config default** — with none of `MAOS_SSO_*` / `MAOS_KMS_*` /
//!    `MAOS_SIEM_*` set, the runtime reports no subsystem configured and the
//!    write/issuance/forward paths are byte-identical no-ops (Option-A plaintext
//!    passthrough, no principal required, no SIEM forwarding). The v1.5 default
//!    posture MUST NOT flip.
//! 2. **Fail-closed PER subsystem (Grumbal/D10)** — when an integration is
//!    *configured but unavailable*, each fails closed and loud with a DISTINCT
//!    outcome, never falling open:
//!      - SSO IdP unreachable  → capability issuance **DENIED** (never falls
//!        open to `spirit_pid`);
//!      - KMS unreachable      → sealed write **REFUSED** (never silently
//!        written as plaintext under an encryption posture);
//!      - SIEM sink down        → records **BUFFERED** + operator-visible error
//!        (never silently dropped).
//!
//! The `Available`-arm falsifiers (the adapter is REALLY invoked, not stubbed)
//! live as unit tests inside `enterprise_identity.rs::available_arm_tests`.
//! These integration tests pin the externally-observable posture contract via
//! the `from_config` test constructor (zero-config + configured-but-down).

#![cfg(feature = "network")]

use maos_bin::enterprise_identity::{AuditFilter, EnterpriseConfig, EnterpriseFailure, EnterpriseRuntime};

// ---------------------------------------------------------------------------
// AC5 (a) — zero-config default: no subsystem, byte-identical no-op.
// ---------------------------------------------------------------------------

#[test]
fn zero_config_reports_no_subsystems_and_is_a_byte_identical_noop() {
    let config = EnterpriseConfig::empty();

    assert!(
        !config.sso_configured(),
        "zero-config MUST NOT enable SSO (no MAOS_SSO_* set)"
    );
    assert!(
        !config.kms_configured(),
        "zero-config MUST NOT enable KMS (no MAOS_KMS_* set)"
    );
    assert!(
        !config.siem_configured(),
        "zero-config MUST NOT enable SIEM (no MAOS_SIEM_* set)"
    );

    let runtime = EnterpriseRuntime::from_config(&config)
        .expect("zero-config MUST build a no-op runtime (nothing to wire)");

    assert!(
        runtime.is_noop(),
        "zero-config runtime MUST be a pure no-op (v1.5 byte-identical posture)"
    );

    // At-rest write path: with no KMS configured the seal is an identity
    // passthrough — Option-A plaintext MUST survive byte-for-byte (L2/AC3).
    let plaintext_row: &[u8] = b"collective-memory-row-option-a-plaintext";
    let stored = runtime
        .seal_row_at_rest(plaintext_row)
        .expect("zero-config seal MUST pass through (no KMS configured)");
    assert_eq!(
        stored.as_slice(),
        plaintext_row,
        "zero-config stored bytes MUST be byte-identical to the plaintext input \
         (Option-A default preserved; no silent at-rest flip)"
    );

    // Issuance path: with no SSO configured, issuance proceeds without a
    // verified principal — the v1.5 `spirit_pid`-bound behavior is unchanged.
    runtime
        .issue_under_principal(42, "", "test-capability")
        .expect("zero-config MUST NOT gate capability issuance on an SSO principal");

    // Forward path: with no SIEM configured, forwarding is a no-op — nothing
    // buffered, nothing dropped, nothing exported off-Host.
    let forwarded = runtime
        .forward_audit_to_siem(AuditFilter::default())
        .expect("zero-config MUST NOT forward or buffer SIEM records");
    assert_eq!(
        forwarded, 0,
        "zero-config forward MUST be a no-op returning zero records forwarded"
    );
}

// ---------------------------------------------------------------------------
// AC5 (b) — SSO configured-but-down: issuance DENIED, never falls open.
// ---------------------------------------------------------------------------

#[test]
fn sso_configured_but_down_denies_issuance_and_never_falls_open() {
    let config = EnterpriseConfig::empty().with_sso_down();
    assert!(
        config.sso_configured(),
        "test vector must reflect SSO configured-but-down"
    );

    let runtime = EnterpriseRuntime::from_config(&config)
        .expect("a configured runtime MUST build (down-state is observed at use, not construction)");

    // IdP unreachable ⇒ the capability issuance governed by the SSO assertion
    // is DENIED. The composition root MUST NEVER fall open to a bare
    // `spirit_pid` principal when SSO is configured (the 3am pager).
    let outcome = runtime.issue_under_principal(42, "oidc-assertion-presented", "test-capability");

    assert!(
        matches!(outcome, Err(EnterpriseFailure::SsoIssuanceDenied { .. })),
        "SSO configured-but-down MUST deny issuance with a distinct SsoIssuanceDenied \
         failure (never fall open to spirit_pid); got {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// AC5 (c) — KMS configured-but-down: sealed write REFUSED, never silent plaintext.
// ---------------------------------------------------------------------------

#[test]
fn kms_configured_but_down_refuses_sealed_write_and_never_writes_plaintext() {
    let config = EnterpriseConfig::empty().with_kms_down();
    assert!(
        config.kms_configured(),
        "test vector must reflect KMS configured-but-down"
    );

    let runtime = EnterpriseRuntime::from_config(&config)
        .expect("a configured runtime MUST build (down-state is observed at use, not construction)");

    let sensitive_row: &[u8] = b"collective-row-that-must-not-leak-as-plaintext";

    // KMS unreachable ⇒ the at-rest write is REFUSED. A silent plaintext
    // fallback under an encryption posture is the #1 real-world at-rest defeat
    // (Vex); the composition root MUST refuse rather than degrade.
    match runtime.seal_row_at_rest(sensitive_row) {
        Err(EnterpriseFailure::KmsSealedWriteRefused { .. }) => {
            // pass: the write was refused loudly, not written as plaintext.
        }
        Ok(returned) => panic!(
            "KMS configured-but-down MUST refuse the sealed write; instead Ok({} bytes) was \
             returned — a silent plaintext fallback under an encryption posture",
            returned.len()
        ),
        other => panic!(
            "KMS configured-but-down MUST surface a distinct KmsSealedWriteRefused; got {other:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// AC5 (d) — SIEM configured-but-down: operator-visible error,
//           never silently dropped.
// ---------------------------------------------------------------------------

#[test]
fn siem_configured_but_down_surfaces_an_operator_error_and_never_drops() {
    let config = EnterpriseConfig::empty().with_siem_down();
    assert!(
        config.siem_configured(),
        "test vector must reflect SIEM configured-but-down"
    );

    let runtime = EnterpriseRuntime::from_config(&config)
        .expect("a configured runtime MUST build (down-state is observed at use, not construction)");

    // Sink unreachable ⇒ the forward surfaces a SiemSinkDown operator-visible
    // error. The composition root MUST NOT silently drop the audit trail under
    // backpressure. (The buffered COUNT is determined by a real TL read on the
    // Available path; the from_config posture vector has no TL, so it is 0
    // here — the contract under test is the distinct error, never Ok.)
    match runtime.forward_audit_to_siem(AuditFilter::default()) {
        Err(EnterpriseFailure::SiemSinkDown { .. }) => {
            // pass: surfaced loudly, never Ok (silent drop).
        }
        Ok(n) => panic!(
            "SIEM configured-but-down MUST surface an operator-visible SiemSinkDown error; \
             Ok({n}) is a silent drop of the audit trail"
        ),
        other => panic!(
            "SIEM configured-but-down MUST surface a distinct SiemSinkDown; got {other:?}"
        ),
    }
}
