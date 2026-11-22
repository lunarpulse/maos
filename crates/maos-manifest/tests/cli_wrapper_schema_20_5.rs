//! Story 20-5 AC3 — `CliWrapperConfig::take_schema_version` removes and window-checks
//! `[cli_wrapper] manifest_schema_version` BEFORE the strict parse (absent means 1).

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_manifest::manifest::CliWrapperConfig;
use maos_spirit_abi::{
    CLI_WRAPPER_SCHEMA_VERSION, MAX_SUPPORTED_CLI_WRAPPER_SCHEMA_VERSION,
    MIN_SUPPORTED_CLI_WRAPPER_SCHEMA_VERSION,
};

fn section(extra: &str) -> toml::Value {
    toml::from_str(&format!(
        "{extra}\ncommand = \"echo\"\noutput_shape_version = \"1.0.0\"\n\
         [posture]\nstdio_shape = \"raw\"\ncontrol_channel = \"signals\"\n"
    ))
    .expect("fixture is TOML")
}

fn peel(extra: &str) -> (Result<u32, CliWrapperAdmissionError>, toml::Value) {
    let mut value = section(extra);
    (CliWrapperConfig::take_schema_version(&mut value), value)
}

#[test]
fn the_window_is_one_through_two_and_the_counter_is_separate() {
    assert_eq!(CLI_WRAPPER_SCHEMA_VERSION, 2);
    assert_eq!(MIN_SUPPORTED_CLI_WRAPPER_SCHEMA_VERSION, 1);
    assert_eq!(MAX_SUPPORTED_CLI_WRAPPER_SCHEMA_VERSION, 2);
}

#[test]
fn absent_means_one_and_the_table_is_untouched() {
    let (result, value) = peel("");
    assert_eq!(result, Ok(1));
    assert_eq!(value, section(""));
}

#[test]
fn a_declared_version_in_the_window_is_removed_so_the_strict_parse_accepts_the_table() {
    let (result, value) = peel("manifest_schema_version = 1");
    assert_eq!(result, Ok(1));
    assert!(value.get("manifest_schema_version").is_none());
    let config = CliWrapperConfig::from_toml_str(&toml::to_string(&value).unwrap())
        .expect("the peeled table parses strictly");
    assert_eq!(config.command, "echo");
    // Without the peel the strict parse refuses the key: this is why the peel exists.
    let unpeeled = toml::to_string(&section("manifest_schema_version = 1")).unwrap();
    assert!(CliWrapperConfig::from_toml_str(&unpeeled).is_err());
}

#[test]
fn a_newer_version_is_refused_typed_before_an_unknown_key_can_be_reported() {
    let (result, value) = peel("manifest_schema_version = 3\nfrobnicate = true");
    assert_eq!(
        result,
        Err(CliWrapperAdmissionError::ECliWrapperSchemaUnsupported {
            declared: "3".to_string(),
            min_supported: 1,
            max_supported: 2,
        })
    );
    // The key is removed even on refusal; the unknown key is left for the strict parse
    // that never runs.
    assert!(value.get("manifest_schema_version").is_none());
    assert!(value.get("frobnicate").is_some());
}

#[test]
fn zero_negative_oversized_and_non_integer_values_are_refused_naming_the_window() {
    for (literal, shown) in [
        ("0", "0"),
        ("-1", "-1"),
        ("3", "3"),
        ("4294967297", "4294967297"),
        ("\"1\"", "\"1\""),
        ("1.0", "1.0"),
        ("true", "true"),
        ("[1]", "[1]"),
    ] {
        let (result, _) = peel(&format!("manifest_schema_version = {literal}"));
        let Err(CliWrapperAdmissionError::ECliWrapperSchemaUnsupported {
            declared,
            min_supported,
            max_supported,
        }) = result
        else {
            panic!("{literal} must be refused, got {result:?}");
        };
        assert_eq!(declared, shown);
        assert_eq!((min_supported, max_supported), (1, 2));
        assert!(CliWrapperAdmissionError::ECliWrapperSchemaUnsupported {
            declared,
            min_supported,
            max_supported,
        }
        .to_string()
        .contains("accepts 1..=2"));
    }
}

#[test]
fn a_very_long_value_is_bounded_in_the_error_text() {
    let long = "x".repeat(10_000);
    let (result, _) = peel(&format!("manifest_schema_version = \"{long}\""));
    let Err(CliWrapperAdmissionError::ECliWrapperSchemaUnsupported { declared, .. }) = result
    else {
        panic!("a string must be refused");
    };
    assert!(declared.chars().count() <= 64, "{}", declared.len());
}

#[test]
fn a_non_table_section_is_treated_as_absent_and_left_for_the_strict_parse() {
    let mut value = toml::Value::String("not a table".to_string());
    assert_eq!(CliWrapperConfig::take_schema_version(&mut value), Ok(1));
}

/// D-LRQC-04 A: schema 2 requires the CRL-addressable Worker identity, schema 1 cannot carry it,
/// and a taken identity leaves the table strict-parseable.
#[test]
fn the_worker_identity_is_required_at_schema_two_refused_at_schema_one_and_validated() {
    let take = |extra: &str, schema: u32| {
        let mut value = section(extra);
        let identity = CliWrapperConfig::take_worker_identity(&mut value, schema);
        (identity, value)
    };
    let (identity, value) = take("class_name = \"worker-x\"\nclass_version = \"1.2.3\"", 2);
    assert_eq!(
        identity.expect("valid identity"),
        Some(("worker-x".to_string(), "1.2.3".to_string()))
    );
    assert!(value.get("class_name").is_none() && value.get("class_version").is_none());
    CliWrapperConfig::from_toml_str(&toml::to_string(&value).unwrap())
        .expect("the table parses strictly once the identity is taken");
    assert!(take("", 2).0.is_err(), "schema 2 requires the identity");
    assert!(
        take("class_name = \"worker-x\"", 2).0.is_err(),
        "both keys are required"
    );
    assert!(take("", 1).0.expect("schema 1 without identity").is_none());
    assert!(
        take("class_name = \"worker-x\"\nclass_version = \"1.2.3\"", 1)
            .0
            .is_err(),
        "schema 1 cannot carry the identity"
    );
    for bad in [
        "class_name = \"Worker X\"\nclass_version = \"1.2.3\"",
        "class_name = \"\"\nclass_version = \"1.2.3\"",
        "class_name = \"worker-x\"\nclass_version = \"1.2\"",
        "class_name = \"worker-x\"\nclass_version = \"1.2.3-rc1\"",
        "class_name = \"worker-x\"\nclass_version = \"1.02.3\"",
        "class_name = \"worker-x\"\nclass_version = \"+1.2.3\"",
        "class_name = 7\nclass_version = \"1.2.3\"",
    ] {
        assert!(take(bad, 2).0.is_err(), "{bad} must be refused");
    }
}
