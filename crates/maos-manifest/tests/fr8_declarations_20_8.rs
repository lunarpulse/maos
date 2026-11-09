use maos_manifest::{
    ExplanationShapeSection, IntentPromotionSetSection, SkillsSection, SwapInvariantsSection,
};

#[test]
fn declarations_validate_bounded_unique_names_and_unknown_keys() {
    let parsers: [(&str, fn(&str) -> bool); 4] = [
        ("uses", |s| SkillsSection::from_toml_str(s).is_ok()),
        ("required_fields", |s| {
            ExplanationShapeSection::from_toml_str(s).is_ok()
        }),
        ("allowed", |s| {
            IntentPromotionSetSection::from_toml_str(s).is_ok()
        }),
        ("preserve", |s| {
            SwapInvariantsSection::from_toml_str(s).is_ok()
        }),
    ];
    for (field, accepts) in parsers {
        assert!(accepts(&format!("{field} = [\"valid.name:one-2\"]")));
        for list in [
            "[]".to_owned(),
            "[\"Upper\"]".to_owned(),
            "[\"same\", \"same\"]".to_owned(),
            format!(
                "[{}]",
                (0..33)
                    .map(|i| format!("\"item{i}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            format!("[\"{}\"]", "x".repeat(129)),
        ] {
            assert!(
                !accepts(&format!("{field} = {list}")),
                "accepted {field} = {list}"
            );
        }
        assert!(!accepts(&format!("{field} = [\"valid\"]\nunknown = 1")));
    }
}

#[test]
fn invalid_declaration_errors_are_bounded_and_distinguish_duplicates() {
    let invalid = SkillsSection::from_toml_str(&format!("uses = [\"{}\"]", "x".repeat(10_000)))
        .unwrap_err()
        .to_string();
    assert!(invalid.len() < 256, "unbounded refusal: {}", invalid.len());
    assert!(invalid.contains("invalid entry"));
    let duplicate = SkillsSection::from_toml_str("uses = [\"same\", \"same\"]")
        .unwrap_err()
        .to_string();
    assert!(duplicate.contains("duplicate entry"));
}

#[test]
fn skill_identifiers_must_survive_log_redaction() {
    for name in [
        "sk-review",
        "ghp_example",
        "ya29.example",
        "abcdef0123456789abcdef0123456789",
    ] {
        assert!(SkillsSection::from_toml_str(&format!("uses = [\"{name}\"]")).is_err());
    }
}
