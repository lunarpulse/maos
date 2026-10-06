//! Story 17-3c — `[capabilities.required.iac].send` (closed peer-class set)
//! and the optional `[capabilities.required.provider]` section.

use maos_domain::invariants::i1::Scope;
use maos_manifest::{capabilities_required_to_scopes, CapabilitiesRequired, ManifestError};

fn refusal(toml: &str) -> String {
    match CapabilitiesRequired::from_toml_str(toml).unwrap_err() {
        ManifestError::Toml(msg) => msg,
        other => panic!("unexpected error variant: {other:?}"),
    }
}

fn iac_scopes(caps: &CapabilitiesRequired) -> Vec<Scope> {
    capabilities_required_to_scopes(caps)
        .into_iter()
        .filter(|s| matches!(s, Scope::IacSend { .. }))
        .collect()
}

#[test]
fn each_closed_set_entry_parses_and_maps_to_iac_send_scope() {
    for entry in ["broadcast", "spirit:peer"] {
        let caps = CapabilitiesRequired::from_toml_str(&format!("iac.send = [\"{entry}\"]"))
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        assert_eq!(caps.iac.send, vec![entry.to_string()]);
        assert_eq!(
            capabilities_required_to_scopes(&caps),
            vec![Scope::IacSend {
                peer_class: entry.to_string()
            }]
        );
    }
    let both =
        CapabilitiesRequired::from_toml_str("iac.send = [\"broadcast\", \"spirit:peer\"]").unwrap();
    assert_eq!(iac_scopes(&both).len(), 2);
}

#[test]
fn unknown_empty_and_duplicate_entries_are_refused_naming_the_field() {
    for toml in [
        "iac.send = [\"spirit:any\"]",
        "iac.send = []",
        "iac.send = [\"broadcast\", \"broadcast\"]",
    ] {
        let msg = refusal(toml);
        assert!(
            msg.starts_with("validation failed for capabilities.required.iac.send:"),
            "{toml}: {msg}"
        );
    }
    assert!(CapabilitiesRequired::from_toml_str("iac.recv = [\"broadcast\"]").is_err());
}

#[test]
fn provider_section_is_optional_but_refused_when_present_and_empty() {
    let caps = CapabilitiesRequired::from_toml_str("iac.send = [\"spirit:peer\"]").unwrap();
    assert!(caps.provider.complete.is_empty());
    assert!(CapabilitiesRequired::from_toml_str("")
        .unwrap()
        .provider
        .complete
        .is_empty());
    let msg = refusal("provider.complete = []");
    assert!(
        msg.starts_with("validation failed for capabilities.required.provider.complete:"),
        "{msg}"
    );
}

#[test]
fn schema_below_v5_drops_iac() {
    let caps = CapabilitiesRequired::from_toml_str(
        "provider.complete = [\"anthropic.default\"]\niac.send = [\"spirit:peer\"]",
    )
    .unwrap();
    let v4 = caps.clone().degrade_for_schema_version(4);
    assert!(v4.iac.send.is_empty());
    assert!(iac_scopes(&v4).is_empty());
    assert_eq!(
        iac_scopes(&caps.degrade_for_schema_version(5)),
        vec![Scope::IacSend {
            peer_class: "spirit:peer".into()
        }]
    );
}
