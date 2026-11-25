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

use maos_bin::enterprise_identity::{
    AuditFilter, EnterpriseConfig, EnterpriseFailure, EnterpriseRuntime,
};

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

    let runtime = EnterpriseRuntime::from_config(&config).expect(
        "a configured runtime MUST build (down-state is observed at use, not construction)",
    );

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

    let runtime = EnterpriseRuntime::from_config(&config).expect(
        "a configured runtime MUST build (down-state is observed at use, not construction)",
    );

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

    let runtime = EnterpriseRuntime::from_config(&config).expect(
        "a configured runtime MUST build (down-state is observed at use, not construction)",
    );

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
        other => {
            panic!("SIEM configured-but-down MUST surface a distinct SiemSinkDown; got {other:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Story 23-2 AC4 — static keys remain; discovery configuration is typed.
//
// `from_env_with` is the production constructor with the `MAOS_SSO_*` values
// injected, so no test mutates the process environment.
// ---------------------------------------------------------------------------

#[path = "../../maos-sso/tests/fixtures.rs"]
mod sso_fixtures;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use maos_bin::enterprise_identity::{DiscoveryWiring, SsoDownReason, SsoEnv};
use maos_domain::ports::CryptoProvider;
use maos_kernel_core::security::RingCryptoProvider;

fn sso_runtime(
    env: &SsoEnv,
    discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
) -> EnterpriseRuntime {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    EnterpriseRuntime::from_env_with(
        env,
        discovery,
        crypto,
        std::env::temp_dir().join("maos-23-2-wiring-never-written.sqlite"),
        1,
        None,
    )
    .expect("a runtime always builds; a refused SSO configuration is ConfiguredDown")
}

fn static_env() -> SsoEnv {
    SsoEnv {
        jwks: Some(sso_fixtures::JWKS_KEY_A.to_owned()),
        issuers: Some(sso_fixtures::ISS_GOOD.to_owned()),
        audience: Some(sso_fixtures::AUD_EXPECTED.to_owned()),
        ..SsoEnv::default()
    }
}

fn assert_denied_with(runtime: &EnterpriseRuntime, code: &str) {
    match runtime.verify_principal_for_issuance(42, sso_fixtures::TOKEN_GOOD_RS256) {
        Err(EnterpriseFailure::SsoIssuanceDenied { reason, .. }) => {
            assert!(reason.contains(code), "{reason}");
        }
        other => panic!("a valid assertion must still be denied; got {other:?}"),
    }
}

/// AC4: with only `MAOS_SSO_JWKS`, the static verifier is built exactly as
/// before and discovery is never touched.
#[test]
fn static_jwks_still_verifies_without_discovery() {
    let touched = AtomicBool::new(false);
    let runtime = sso_runtime(&static_env(), &|| {
        touched.store(true, Ordering::SeqCst);
        Err(SsoDownReason::DiscoveryUnsupportedBuild)
    });
    assert!(runtime.sso_configured());
    assert_eq!(runtime.sso_down_reason(), None);
    let principal = runtime
        .verify_principal_for_issuance(42, sso_fixtures::TOKEN_GOOD_RS256)
        .expect("the static key verifies")
        .expect("SSO is configured, so a principal is returned");
    assert_eq!(principal.subject, sso_fixtures::PRINCIPAL_SUB);
    assert!(matches!(
        runtime.verify_principal_for_issuance(42, sso_fixtures::TOKEN_WRONG_KEY),
        Err(EnterpriseFailure::SsoIssuanceDenied { .. })
    ));
    assert!(
        !touched.load(Ordering::SeqCst),
        "no discovery wiring was built"
    );
}

/// AC4: `MAOS_SSO_JWKS` and `MAOS_SSO_DISCOVERY_ISSUER` together leave SSO
/// configured-down with a typed reason; issuance is denied even for an
/// assertion the static key would verify, and discovery is not taken.
#[test]
fn jwks_and_discovery_together_are_configured_down() {
    let touched = AtomicBool::new(false);
    let mut env = static_env();
    env.discovery_issuer = Some(sso_fixtures::ISS_GOOD.to_owned());
    let runtime = sso_runtime(&env, &|| {
        touched.store(true, Ordering::SeqCst);
        Err(SsoDownReason::DiscoveryUnsupportedBuild)
    });
    assert!(runtime.sso_configured());
    assert_eq!(
        runtime.sso_down_reason(),
        Some(&SsoDownReason::KeySourcesConflict)
    );
    assert_denied_with(&runtime, "sso_key_sources_exclusive");
    assert!(!touched.load(Ordering::SeqCst), "discovery was not taken");
}

/// AC4: a tuning knob without `MAOS_SSO_DISCOVERY_ISSUER` is configured-down,
/// alone or beside the static keys.
#[test]
fn a_tuning_knob_alone_is_configured_down() {
    for knob in ["ttl", "min_refresh"] {
        for with_static_keys in [false, true] {
            let mut env = if with_static_keys {
                static_env()
            } else {
                SsoEnv::default()
            };
            match knob {
                "ttl" => env.jwks_ttl_secs = Some("600".to_owned()),
                _ => env.jwks_min_refresh_secs = Some("10".to_owned()),
            }
            let runtime = sso_runtime(&env, &|| Err(SsoDownReason::DiscoveryUnsupportedBuild));
            assert!(runtime.sso_configured(), "{knob} {with_static_keys}");
            assert_eq!(
                runtime.sso_down_reason(),
                Some(&SsoDownReason::TuningWithoutDiscovery),
                "{knob} {with_static_keys}"
            );
            assert_denied_with(&runtime, "sso_tuning_without_discovery");
        }
    }
}

/// Story 23-2 F13: a `MAOS_SSO_*` variable that is PRESENT but not valid UTF-8 is never read as
/// absent. Whichever of the seven it is, and whether or not anything else is configured, SSO is
/// configured-down with a typed reason that names the variable and never echoes the value.
#[cfg(unix)]
#[test]
fn a_non_utf8_sso_variable_is_configured_down_never_absent() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    const NAMES: [&str; 7] = [
        "MAOS_SSO_JWKS",
        "MAOS_SSO_ISSUERS",
        "MAOS_SSO_AUDIENCE",
        "MAOS_SSO_ALGS",
        "MAOS_SSO_DISCOVERY_ISSUER",
        "MAOS_SSO_JWKS_TTL_SECS",
        "MAOS_SSO_JWKS_MIN_REFRESH_SECS",
    ];
    let bad = || OsString::from_vec(b"canary-value-\xff\xfe".to_vec());
    let good = |name: &str| -> Option<OsString> {
        Some(OsString::from(match name {
            "MAOS_SSO_JWKS" => sso_fixtures::JWKS_KEY_A,
            "MAOS_SSO_ISSUERS" => sso_fixtures::ISS_GOOD,
            "MAOS_SSO_AUDIENCE" => sso_fixtures::AUD_EXPECTED,
            _ => return None,
        }))
    };
    for name in NAMES {
        // Beside an otherwise valid static configuration, and entirely alone.
        type Lookup = Box<dyn Fn(&str) -> Option<OsString>>;
        let surroundings: [(&str, Lookup); 2] = [
            ("beside a valid configuration", Box::new(good)),
            ("alone", Box::new(|_| None)),
        ];
        for (label, others) in surroundings {
            let env = SsoEnv::from_lookup(|asked| {
                if asked == name {
                    Some(bad())
                } else {
                    others(asked)
                }
            });
            assert_eq!(env.invalid, Some(name), "{name} {label}");
            assert!(env.any(), "{name} {label}: present counts as configured");

            let touched = AtomicBool::new(false);
            let runtime = sso_runtime(&env, &|| {
                touched.store(true, Ordering::SeqCst);
                Err(SsoDownReason::DiscoveryUnsupportedBuild)
            });
            assert!(runtime.sso_configured(), "{name} {label}");
            assert_eq!(
                runtime.sso_down_reason(),
                Some(&SsoDownReason::EnvNotUnicode(name)),
                "{name} {label}"
            );
            assert_denied_with(&runtime, &format!("sso_env_not_unicode: {name}"));
            let shown = format!("{:?} {env:?}", runtime.sso_down_reason());
            assert!(!shown.contains("canary"), "{name} {label}: {shown}");
            assert!(!touched.load(Ordering::SeqCst), "{name} {label}");
        }
    }
    // The valid configuration without the bad variable is unharmed.
    let env = SsoEnv::from_lookup(good);
    assert_eq!(env.invalid, None);
    assert_eq!(
        sso_runtime(&env, &|| unreachable!()).sso_down_reason(),
        None
    );
}

#[cfg(feature = "online-adapters")]
mod discovery_mode {
    use super::*;
    use maos_sso::discovery::{DiscoveryError, FetchError, Fetcher, MonotonicClock};

    /// An issuer that never answers: every fetch fails.
    struct Unreachable;

    impl Fetcher for Unreachable {
        fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
            Err(FetchError::Unavailable)
        }
    }

    fn unreachable_wiring() -> Result<DiscoveryWiring, SsoDownReason> {
        Ok(DiscoveryWiring::new(
            Arc::new(Unreachable),
            Arc::new(MonotonicClock::new()),
        ))
    }

    fn discovery_env(issuers: &str) -> SsoEnv {
        SsoEnv {
            issuers: Some(issuers.to_owned()),
            audience: Some(sso_fixtures::AUD_EXPECTED.to_owned()),
            discovery_issuer: Some(sso_fixtures::ISS_GOOD.to_owned()),
            ..SsoEnv::default()
        }
    }

    /// AC4: a discovery issuer that `MAOS_SSO_ISSUERS` does not list is
    /// configured-down.
    #[test]
    fn a_discovery_issuer_missing_from_the_trusted_list_is_configured_down() {
        let runtime = sso_runtime(&discovery_env(sso_fixtures::ISS_BAD), &unreachable_wiring);
        assert_eq!(
            runtime.sso_down_reason(),
            Some(&SsoDownReason::Discovery(DiscoveryError::IssuerNotTrusted))
        );
        assert_denied_with(&runtime, "sso_discovery_issuer_not_trusted");
    }

    /// AC4: out-of-range or non-numeric tuning is a typed refusal.
    #[test]
    fn invalid_tuning_is_configured_down() {
        let mut env = discovery_env(sso_fixtures::ISS_GOOD);
        env.jwks_ttl_secs = Some("soon".to_owned());
        let runtime = sso_runtime(&env, &unreachable_wiring);
        assert_eq!(
            runtime.sso_down_reason(),
            Some(&SsoDownReason::TuningInvalid("MAOS_SSO_JWKS_TTL_SECS"))
        );
        env.jwks_ttl_secs = Some("0".to_owned());
        let runtime = sso_runtime(&env, &unreachable_wiring);
        assert_eq!(
            runtime.sso_down_reason(),
            Some(&SsoDownReason::Discovery(DiscoveryError::ConfigInvalid))
        );
    }

    /// D-23-2-08 A: an unreachable issuer at boot is NOT configured-down; the
    /// verifier stays and reports unhealthy, so issuance is denied until keys
    /// arrive.
    #[test]
    fn an_unreachable_issuer_at_boot_denies_issuance_without_demoting_sso() {
        let runtime = sso_runtime(&discovery_env(sso_fixtures::ISS_GOOD), &unreachable_wiring);
        assert!(runtime.sso_configured());
        assert_eq!(runtime.sso_down_reason(), None);
        assert_denied_with(&runtime, "unavailable");
    }
}
