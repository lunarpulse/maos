//! Story 23-0 AC2 — the `online-adapters` umbrella (D-23-6-02 A, D-23-6-01 A iii).
//!
//! The feature gates the online adapters (23-8: the external KMS adapter). These tests pin its place in the feature
//! graph: it is in `default`, it enables `network`, and nothing but `default`
//! enables it, so `--no-default-features --features network,wasm-host` is the
//! profile without online adapters.

use std::collections::{BTreeMap, BTreeSet};

const UMBRELLA: &str = "online-adapters";

fn features() -> BTreeMap<String, Vec<String>> {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read maos-bin Cargo.toml");
    let manifest: toml::Table = manifest.parse().expect("parse maos-bin Cargo.toml");
    manifest["features"]
        .as_table()
        .expect("[features]")
        .iter()
        .map(|(name, members)| {
            let members = members
                .as_array()
                .expect("feature list")
                .iter()
                .map(|member| member.as_str().expect("feature member").to_owned())
                .collect();
            (name.clone(), members)
        })
        .collect()
}

/// Every feature `name` enables, transitively, through plain feature names
/// (`dep:x` and `crate/feature` members enable no feature of this crate).
fn reaches(features: &BTreeMap<String, Vec<String>>, name: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![name.to_owned()];
    while let Some(current) = stack.pop() {
        for member in features.get(&current).into_iter().flatten() {
            if features.contains_key(member) && seen.insert(member.clone()) {
                stack.push(member.clone());
            }
        }
    }
    seen
}

#[test]
fn the_umbrella_is_in_default_and_enables_network() {
    let features = features();
    assert!(
        features["default"].iter().any(|member| member == UMBRELLA),
        "default must contain {UMBRELLA}: {:?}",
        features["default"]
    );
    // Story 23-8 (D-23-8-09 A): the umbrella enables `network` and the
    // external KMS adapter of maos-secrets, and no other feature of this crate.
    assert_eq!(
        features[UMBRELLA],
        ["network", "maos-secrets/http-kms"],
        "{UMBRELLA} enables network and the KMS adapter (23-8) and nothing else"
    );
    for kept in ["network", "wasm-host"] {
        assert!(
            features["default"].iter().any(|member| member == kept),
            "default keeps {kept}"
        );
    }
}

#[test]
fn no_feature_but_default_reaches_the_umbrella() {
    let features = features();
    let reaching: Vec<&String> = features
        .keys()
        .filter(|name| name.as_str() != UMBRELLA && reaches(&features, name).contains(UMBRELLA))
        .collect();
    assert_eq!(
        reaching,
        ["default"],
        "only `default` may enable {UMBRELLA}"
    );
    for name in [
        "network",
        "spirit_test",
        "fixture_replay",
        "crypto-recording",
        "kms-fault-inject",
        "sso-fault-inject",
    ] {
        assert!(
            !reaches(&features, name).contains(UMBRELLA),
            "{name} must not enable {UMBRELLA}"
        );
    }
}

/// Compiled only in the `network,wasm-host`-without-umbrella shape (AC2, AC6).
/// CI runs it by name and requires `1 passed`, so a build that silently
/// enables the umbrella (the test then does not exist) turns the step red.
#[cfg(all(
    feature = "network",
    feature = "wasm-host",
    not(feature = "online-adapters")
))]
#[test]
fn the_profile_without_online_adapters_has_network_and_wasm_host() {
    assert!(!cfg!(feature = "online-adapters"));
    assert!(cfg!(feature = "network") && cfg!(feature = "wasm-host"));
}

/// Compiled only when the umbrella is on: it must have brought `network` in.
#[cfg(feature = "online-adapters")]
#[test]
fn the_umbrella_build_has_network() {
    assert!(
        reaches(&features(), UMBRELLA).contains("network"),
        "online-adapters without network"
    );
}
