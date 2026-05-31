//! `maos.skill.v1` schema validation — Story 7.4 AC2.

use maos_skill::{parse_skill, ESkillSchema};

const VALID: &str = r#"---
id = "code-review.rust"
version = "1.2.0"
name = "Rust Code Review"
description = "Reviews Rust diffs for correctness and idiom."
required_capabilities = ["fs.read:/src", "fs.read:/tests"]
min_substrate_version = "0.5.0"
---
# Rust Code Review

When invoked, walk the diff and flag correctness bugs first, then idiom.
"#;

#[test]
fn valid_skill_parses_with_all_fields() {
    let skill = parse_skill(VALID).expect("valid skill must parse");
    assert_eq!(skill.manifest.id, "code-review.rust");
    assert_eq!(skill.manifest.version, "1.2.0");
    assert_eq!(skill.manifest.name, "Rust Code Review");
    assert_eq!(skill.manifest.required_capabilities.len(), 2);
    assert_eq!(
        skill.manifest.min_substrate_version.as_deref(),
        Some("0.5.0")
    );
    // The body is opaque markdown — present, but never parsed for meaning.
    assert!(skill.body.contains("Rust Code Review"));
}

#[test]
fn minimal_skill_without_optional_fields_parses() {
    let src = r#"---
id = "min"
version = "0.1.0"
name = "Minimal"
description = "A minimal skill."
---
Body present.
"#;
    let skill = parse_skill(src).expect("minimal skill must parse");
    assert!(skill.manifest.required_capabilities.is_empty());
    assert_eq!(skill.manifest.min_substrate_version, None);
}

#[test]
fn unknown_frontmatter_field_is_rejected_not_defaulted() {
    let src = r#"---
id = "x"
version = "1.0.0"
name = "X"
description = "d"
surprise_field = "should be rejected"
---
body
"#;
    let err = parse_skill(src).expect_err("unknown field must reject");
    assert!(
        matches!(err, ESkillSchema::UnknownField(_)),
        "expected UnknownField, got {err:?}"
    );
}

#[test]
fn non_semver_version_is_rejected() {
    let src = r#"---
id = "x"
version = "not-a-version"
name = "X"
description = "d"
---
body
"#;
    let err = parse_skill(src).expect_err("non-semver must reject");
    assert!(
        matches!(err, ESkillSchema::InvalidSemver(ref v, _) if v == "not-a-version"),
        "expected InvalidSemver, got {err:?}"
    );
}

#[test]
fn missing_required_field_is_a_toml_parse_error() {
    // No `name` field — toml deserialize fails (missing field), NOT a silent default.
    let src = r#"---
id = "x"
version = "1.0.0"
description = "d"
---
body
"#;
    let err = parse_skill(src).expect_err("missing required field must reject");
    assert!(
        matches!(err, ESkillSchema::TomlParse(_)),
        "expected TomlParse, got {err:?}"
    );
}

#[test]
fn missing_opening_fence_is_rejected() {
    let src = "id = \"x\"\nversion = \"1.0.0\"\nname = \"X\"\ndescription = \"d\"\nbody\n";
    assert_eq!(parse_skill(src).unwrap_err(), ESkillSchema::MissingFence);
}

#[test]
fn missing_closing_fence_is_rejected() {
    let src = "---\nid = \"x\"\nversion = \"1.0.0\"\nname = \"X\"\ndescription = \"d\"\n";
    assert_eq!(parse_skill(src).unwrap_err(), ESkillSchema::MissingFence);
}

#[test]
fn empty_body_is_rejected() {
    let src = r#"---
id = "x"
version = "1.0.0"
name = "X"
description = "d"
---
"#;
    assert_eq!(parse_skill(src).unwrap_err(), ESkillSchema::EmptyBody);
}

#[test]
fn empty_id_is_rejected() {
    let src = r#"---
id = ""
version = "1.0.0"
name = "X"
description = "d"
---
body
"#;
    assert_eq!(parse_skill(src).unwrap_err(), ESkillSchema::EmptyId);
}

#[test]
fn invalid_id_charset_is_rejected() {
    let src = r#"---
id = "Has Spaces And CAPS"
version = "1.0.0"
name = "X"
description = "d"
---
body
"#;
    let err = parse_skill(src).unwrap_err();
    assert!(
        matches!(err, ESkillSchema::InvalidIdCharset(_)),
        "expected InvalidIdCharset, got {err:?}"
    );
}

#[test]
fn leading_blank_lines_before_fence_are_tolerated() {
    let src = "\n\n---\nid = \"x\"\nversion = \"1.0.0\"\nname = \"X\"\ndescription = \"d\"\n---\nbody\n";
    assert!(parse_skill(src).is_ok());
}
