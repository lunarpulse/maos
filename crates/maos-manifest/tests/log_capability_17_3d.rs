use maos_domain::invariants::i1::Scope;
use maos_manifest::{capabilities_required_to_scopes, CapabilitiesRequired, LogCapabilities};

#[test]
fn schema_five_retains_both_log_grants() {
    let caps = CapabilitiesRequired::from_toml_str("[log]\nrecall = true\nfetch = true")
        .unwrap()
        .degrade_for_schema_version(5);
    assert_eq!(
        caps.log,
        LogCapabilities {
            recall: true,
            fetch: true
        }
    );
    assert_eq!(
        capabilities_required_to_scopes(&caps),
        vec![Scope::LogRecall, Scope::LogFetch]
    );
}

#[test]
fn log_grants_map_independently_and_default_to_denied() {
    for (fragment, expected) in [
        ("", vec![]),
        ("log.recall = true", vec![Scope::LogRecall]),
        ("log.fetch = true", vec![Scope::LogFetch]),
        ("[log]\nrecall = false\nfetch = false", vec![]),
    ] {
        let caps = CapabilitiesRequired::from_toml_str(fragment).unwrap();
        assert_eq!(capabilities_required_to_scopes(&caps), expected);
    }
}

#[test]
fn schema_four_drops_both_log_grants() {
    let caps = CapabilitiesRequired::from_toml_str("log.recall = true\nlog.fetch = true")
        .unwrap()
        .degrade_for_schema_version(4);
    assert_eq!(caps.log, LogCapabilities::default());
    assert!(capabilities_required_to_scopes(&caps).is_empty());
}

#[test]
fn unknown_log_keys_and_non_boolean_grants_are_refused() {
    for fragment in [
        "log.recalls = true",
        "log.recall = 1",
        "log.fetch = \"true\"",
    ] {
        assert!(
            CapabilitiesRequired::from_toml_str(fragment).is_err(),
            "{fragment}"
        );
    }
}
