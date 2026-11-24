//! Story 23-0 AC2 — the `online-adapters` umbrella (D-23-6-02 A, D-23-6-01 A iii).
//!
//! The feature gates no code in 23-0. These tests pin its place in the feature
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
    assert_eq!(
        features[UMBRELLA],
        ["network"],
        "{UMBRELLA} enables network and nothing else"
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

#[test]
fn the_compiled_shape_matches_the_features() {
    // The umbrella implies `network`. The `network,wasm-host`-only build CI adds
    // (AC6) has `network` on and the umbrella off, and must stay a distinct shape.
    let (network, wasm_host, umbrella) = (
        cfg!(feature = "network"),
        cfg!(feature = "wasm-host"),
        cfg!(feature = "online-adapters"),
    );
    assert!(network || !umbrella, "online-adapters without network");
    let without_umbrella = cfg!(all(
        feature = "network",
        feature = "wasm-host",
        not(feature = "online-adapters")
    ));
    assert_eq!(without_umbrella, network && wasm_host && !umbrella);
}
