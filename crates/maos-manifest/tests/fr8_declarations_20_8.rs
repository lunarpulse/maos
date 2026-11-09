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
