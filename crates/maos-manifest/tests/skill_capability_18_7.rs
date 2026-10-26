use maos_domain::invariants::i1::Scope;
use maos_manifest::{capabilities_required_to_scopes, CapabilitiesRequired};

#[test]
fn schema_six_declares_skill_author_self_only_when_requested() {
    for (fragment, expected) in [
        ("", vec![]),
        ("skill.author_self = false", vec![]),
        ("skill.author_self = true", vec![Scope::SkillAuthorSelf]),
    ] {
        let caps = CapabilitiesRequired::from_toml_str(fragment)
            .unwrap()
            .degrade_for_schema_version(6);
        assert_eq!(capabilities_required_to_scopes(&caps), expected);
    }
}

#[test]
fn older_schema_never_acquires_skill_authority() {
    for schema in 1..6 {
        let caps = CapabilitiesRequired::from_toml_str("skill.author_self = true")
            .unwrap()
            .degrade_for_schema_version(schema);
        assert!(!caps.skill.author_self);
        assert!(!capabilities_required_to_scopes(&caps).contains(&Scope::SkillAuthorSelf));
    }
}

#[test]
fn unknown_skill_keys_and_non_boolean_authority_are_refused() {
    for fragment in [
        "skill.author = true",
        "skill.author_self = 1",
        "skill.author_self = \"true\"",
    ] {
        assert!(
            CapabilitiesRequired::from_toml_str(fragment).is_err(),
            "{fragment}"
        );
    }
}
