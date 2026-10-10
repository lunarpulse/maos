#![forbid(unsafe_code)]

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_domain::host_grant::EgressDestination;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::lifecycle::cli_wrapper::admission::{
    admit_cli_wrapper_journaled, check_egress,
};
use maos_kernel_core::security::manifest::{
    CliWrapperConfig, CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
};

fn destination(value: &str) -> EgressDestination {
    value.to_owned().try_into().unwrap()
}

fn config(egress: &[&str]) -> CliWrapperConfig {
    CliWrapperConfig {
        command: "/must-not-spawn-this-cli".into(),
        argv_prefix: vec![],
        output_shape_version: "1.0.0".into(),
        skill_bundle: vec![],
        recovery_policy: Default::default(),
        posture: CliWrapperPosture {
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
        },
        egress: egress.iter().map(|value| destination(value)).collect(),
    }
}

#[test]
fn empty_policy_admits_without_a_proxy_but_nonempty_policy_refuses() {
    check_egress(&config(&[]), false, &[]).unwrap();
    assert!(matches!(
        check_egress(&config(&["api.example.com:443"]), false, &[]),
        Err(CliWrapperAdmissionError::EEgressProxyUnavailable)
    ));
}

#[test]
fn every_requested_authority_requires_an_exact_host_grant() {
    let requested = config(&["api.example.com:443", "api.example.com:8443"]);
    let grants = vec![destination("api.example.com:443")];
    let error = check_egress(&requested, true, &grants).unwrap_err();
    assert!(
        matches!(error, CliWrapperAdmissionError::EEgressNotGranted { destination: denied }
        if denied == destination("api.example.com:8443"))
    );
    check_egress(
        &config(&["api.example.com:443"]),
        true,
        &[
            EgressDestination::parse_granted("api.example.com".into()).unwrap(),
            destination("other.example.com:443"),
        ],
    )
    .unwrap();
    assert!(matches!(
        check_egress(
            &config(&["api.example.com:443"]),
            true,
            &[destination("api.example.org:443")]
        ),
        Err(CliWrapperAdmissionError::EEgressNotGranted { .. })
    ));
}

#[test]
fn journaled_admission_rejects_egress_before_executing_the_probe() {
    let log = TransparencyLogAdapter::open_in_memory(0x17_1);
    let requested = config(&["api.example.com:443"]);
    assert!(matches!(
        admit_cli_wrapper_journaled(
            &requested,
            SandboxTier::T3,
            17,
            &log,
            false,
            &requested.egress
        ),
        Err(CliWrapperAdmissionError::EEgressProxyUnavailable)
    ));
    assert!(matches!(
        admit_cli_wrapper_journaled(&requested, SandboxTier::T3, 17, &log, true, &[]),
        Err(CliWrapperAdmissionError::EEgressNotGranted { .. })
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn missing_host_grant_does_not_report_the_requested_tier_as_permitted() {
    let allowlist = maos_domain::host_grant::StaticHostGrantAllowlist::new(vec![]);
    let error = maos_kernel_core::lifecycle::cli_wrapper::resolve_cli_wrapper_tier(
        SandboxTier::T3,
        "unknown-image",
        "unknown-publisher",
        &allowlist,
    )
    .unwrap_err();
    match error {
        CliWrapperAdmissionError::ECliWrapperTierNotGranted {
            requested,
            permitted,
            ..
        } => assert_ne!(
            requested, permitted,
            "missing grants must not claim a permitted tier"
        ),
        other => panic!("unexpected refusal: {other:?}"),
    }
}
