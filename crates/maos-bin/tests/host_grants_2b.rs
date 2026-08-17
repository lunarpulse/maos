#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! j1-crosshost-2b AC1.1 — the host-grant vectors, relocated out of `main.rs`.
//!
//! These four tests were an in-`src` `#[cfg(all(test, feature = "network"))] mod
//! host_grant_tests` inside `crates/maos-bin/src/main.rs`. That placement is the
//! exact defect `check-j1-loopback-delegation`'s `worker-cli-under-library` leg
//! (`xtask/src/check_j1_loopback_delegation.rs:570-582`) exists to forbid on
//! `worker_cli.rs`: an in-`src` test module is **charged to maos-bin's KLOC
//! ceiling** and **executed by NO CI job** (every `-p maos-bin` invocation in all
//! eleven workflow files is `--test`-scoped or name-filtered, so nothing ever ran
//! `main.rs`'s unit tests). Moving them here refunds the budget AND enrolls them:
//! the `_2b.rs` suffix is derived by `J1_TEST_SUFFIXES`, so the
//! `check-j1-loopback-delegation` job runs them or the gate reds by construction.
//!
//! The assertions are byte-identical to the originals — only `use super::*;`
//! became explicit `maos_bin::worker_spawn::` imports.

use maos_bin::worker_spawn::{
    builtin_fixture_grant, load_host_grant_allowlist, parse_host_grants_toml,
};
use maos_domain::host_grant::{HostGrantAllowlist, StaticHostGrantAllowlist};
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::lifecycle::cli_wrapper::resolve_cli_wrapper_tier;

/// §A6 review P15 — the "hermetic" fixture must not depend on ambient process
/// state: a developer or CI host with `MAOS_HOST_GRANTS` exported (pointing at
/// a real operator grant file) would silently change what this test exercises
/// and can flip its `codex` assertion. Serialize env-mutating tests and restore
/// on exit.
static HOST_GRANTS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn builtin_allowlist_grants_the_fixture_only() {
    let _guard = HOST_GRANTS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let prior = std::env::var_os("MAOS_HOST_GRANTS");
    std::env::remove_var("MAOS_HOST_GRANTS");
    // No MAOS_HOST_GRANTS in the test env (isolated above) → built-in only.
    let al = load_host_grant_allowlist();
    if let Some(value) = prior {
        std::env::set_var("MAOS_HOST_GRANTS", value);
    }
    assert!(
        al.lookup("worker-cli-fixture", "MAOS Project").is_some(),
        "the built-in host grant must cover the hermetic fixture"
    );
    assert!(
        al.lookup("codex", "OpenAI").is_none(),
        "codex must NOT be granted without an operator MAOS_HOST_GRANTS entry"
    );
}

#[test]
fn a_manifest_cannot_self_grant_an_unlisted_image() {
    // Trust-direction proof: a manifest claiming to be `codex` is NOT granted
    // by the host allowlist → fail closed. Under the old self-grant it would
    // have auto-granted itself.
    let al = StaticHostGrantAllowlist::new(vec![builtin_fixture_grant()]);
    assert!(
        resolve_cli_wrapper_tier(SandboxTier::T3, "codex", "anyone", &al).is_err(),
        "an unlisted image must fail closed, not self-grant"
    );
    assert!(matches!(
        resolve_cli_wrapper_tier(SandboxTier::T3, "worker-cli-fixture", "MAOS Project", &al),
        Ok(t) if t == SandboxTier::T3
    ));
}

#[test]
fn operator_grants_file_parses_real_cli_with_egress() {
    let toml = r#"
[[grant]]
attested_image = "codex"
signing_key_id = "OpenAI"
permitted_tier = "T3"
permitted_egress_destinations = ["api.openai.com"]
"#;
    let grants = parse_host_grants_toml(toml).expect("valid grants file parses");
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].attested_image, "codex");
    assert_eq!(
        grants[0].permitted_egress_destinations,
        vec!["api.openai.com".to_string()]
    );
    assert_eq!(grants[0].permitted_tier, SandboxTier::T3);
}

#[test]
fn malformed_grants_file_errors_never_silently_admits() {
    assert!(
        parse_host_grants_toml("[[grant]]\nsigning_key_id = \"x\"\n").is_err(),
        "a grant missing attested_image must error, never silently admit"
    );
}
