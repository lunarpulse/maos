use maos_domain::{
    invariants::i9::SandboxTier,
    ports::crypto::CryptoProvider,
    sandbox::{ContainerRuntimeKind, ImageAttestationId, T3ImageAttestation, T3ImageEntry},
};
use maos_kernel_core::security::{
    crypto::RingCryptoProvider,
    sandbox::{
        t3::{
            argv::build_runtime_argv,
            image_lock::{load_and_verify_lock_at, VerifiedImageAttestation},
            runtime_detect::ContainerRuntime,
        },
        SandboxSpec,
    },
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::path::Path;

fn verified_image() -> VerifiedImageAttestation {
    // Public deterministic test seed, following security::crypto's convention.
    let seed = [0u8; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let public: [u8; 32] = keypair.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![T3ImageEntry::new("registry.example/maos", [0x11; 32], "unit image", true).unwrap()];
    let signature = RingCryptoProvider
        .sign_capability_token(&seed, &serde_json::to_vec(&entries).unwrap())
        .unwrap();
    let attestation = T3ImageAttestation::new(
        ImageAttestationId([1; 32]),
        1,
        1,
        entries,
        signature.try_into().unwrap(),
        public,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let lock = directory.path().join("t3-image.lock");
    std::fs::write(&lock, serde_json::to_vec(&vec![attestation]).unwrap()).unwrap();
    load_and_verify_lock_at(&lock, &public, &RingCryptoProvider)
        .unwrap()
        .default_entry()
        .unwrap()
}

fn argv(env: &[(String, String)]) -> Result<Vec<String>, maos_domain::sandbox::T3Error> {
    let runtime = ContainerRuntime {
        kind: ContainerRuntimeKind::Podman,
        path: "/usr/bin/podman".into(),
        version: "unit".into(),
    };
    build_runtime_argv(
        &runtime,
        &verified_image(),
        &SandboxSpec::new_for_test(SandboxTier::T3),
        Path::new("/fixture"),
        &[],
        "worker",
        1,
        "worker-unit",
        Some(Path::new("/run/egress.sock")),
        env,
    )
}

#[test]
fn worker_environment_values_never_enter_runtime_arguments() {
    let secret = "PUBLIC_SCOPED_SENTINEL_17_1_NOT_FOR_ARGV";
    let args = argv(&[("CODEX_API_KEY".into(), secret.into())]).unwrap();
    assert!(
        args.iter().all(|arg| !arg.contains(secret)),
        "runtime argv exposed an environment value"
    );
}

#[test]
fn malformed_environment_names_are_refused_without_echoing_values() {
    let secret = "PUBLIC_SCOPED_SENTINEL_17_1_NOT_FOR_ERRORS";
    for name in ["", "CODEX_API_KEY=embedded", "CODEX_API_KEY\0suffix"] {
        let error = argv(&[(name.into(), secret.into())]).unwrap_err();
        assert!(
            !error.to_string().contains(secret),
            "refusal exposed an environment value"
        );
    }
}

#[test]
fn duplicate_environment_names_are_refused() {
    // Duplicate `--env` options resolve runtime-dependent (last-wins), which
    // would make the container environment non-enumerable.
    let error = argv(&[
        ("SCOPED_TEST_VALUE".into(), "first".into()),
        ("SCOPED_TEST_VALUE".into(), "second".into()),
    ])
    .unwrap_err();
    assert!(error.to_string().contains("duplicate"));
}
