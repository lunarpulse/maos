//! Anthropic Skills format adapter — bridges YAML-frontmatter skill documents
//! to the `maos.skill.v1` `SkillManifest` + `Skill` representation per ADR-027.
//!
//! The Anthropic Skills format uses `---` … `---` **YAML** frontmatter (vs.
//! `maos.skill.v1`'s TOML frontmatter). This adapter parses the YAML header,
//! maps the fields to a [`SkillManifest`], and returns a [`Skill`] that can
//! flow through the existing admission pipeline unchanged.
//!
//! The adapter lives in `maos-skill`, NOT in `maos-kernel-core` — the kernel
//! ABI is unchanged (verified by `abi-diff` + `check-kernel-baseline`).

use std::collections::BTreeSet;

use crate::errors::ESkillSchema;
use crate::schema::{Skill, SkillManifest};

/// YAML frontmatter fields for the Anthropic Skills format.
///
/// Only `name` is required; `description` defaults to empty.  Other fields
/// are ignored (no `deny_unknown_fields` — the adapter is lenient on the
/// third-party side; strictness is the kernel's `maos.skill.v1` concern).
#[derive(Debug, serde::Deserialize)]
struct AnthropicFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

const FENCE: &str = "---";

/// Parse an Anthropic Skills format document (YAML frontmatter + markdown body)
/// and bridge it to a `maos.skill.v1` [`Skill`].
///
/// Field mapping:
/// - `name` → `SkillManifest.name` AND `SkillManifest.id` (kebab-cased)
/// - `description` → `SkillManifest.description`
/// - `version` → defaults to `"0.1.0"` (Anthropic format has no version field)
/// - `required_capabilities` → empty (not part of the Anthropic format)
pub fn parse_anthropic_skill(src: &str) -> Result<Skill, ESkillSchema> {
    let trimmed = src.trim_start_matches('\u{feff}');
    let mut rest = trimmed;

    // Strip leading blank lines.
    loop {
        let line_end = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let (line, after) = rest.split_at(line_end);
        if line.trim().is_empty() && !after.is_empty() {
            rest = after;
        } else {
            break;
        }
    }

    // Opening fence.
    let after_open = {
        let line_end = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let (line, after) = rest.split_at(line_end);
        if line.trim() != FENCE {
            return Err(ESkillSchema::MissingFence);
        }
        after
    };

    // Collect YAML frontmatter until closing fence.
    let mut frontmatter = String::new();
    let mut cursor = after_open;
    let body = loop {
        if cursor.is_empty() {
            return Err(ESkillSchema::MissingFence);
        }
        let line_end = cursor.find('\n').map(|i| i + 1).unwrap_or(cursor.len());
        let (line, after) = cursor.split_at(line_end);
        if line.trim() == FENCE {
            break after.to_string();
        }
        frontmatter.push_str(line);
        cursor = after;
    };

    // Parse YAML frontmatter.
    let fm: AnthropicFrontmatter = serde_yaml::from_str(&frontmatter)
        .map_err(|e| ESkillSchema::TomlParse(format!("YAML frontmatter: {e}")))?;

    let name = fm.name.unwrap_or_default();
    if name.is_empty() {
        return Err(ESkillSchema::EmptyName);
    }

    // Derive a kebab-case id from the name.
    let id = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let description = fm.description.unwrap_or_default();

    if body.trim().is_empty() {
        return Err(ESkillSchema::EmptyBody);
    }

    let manifest = SkillManifest {
        id,
        version: "0.1.0".into(),
        name,
        description,
        required_capabilities: BTreeSet::new(),
        min_substrate_version: None,
    };

    // Validate the derived manifest.
    crate::schema::validate_manifest(&manifest)?;

    Ok(Skill { manifest, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_anthropic_skill() {
        let src = "---\nname: example-skill\ndescription: A test skill\n---\n\n# Hello\n\nBody content.\n";
        let skill = parse_anthropic_skill(src).unwrap();
        assert_eq!(skill.manifest.name, "example-skill");
        assert_eq!(skill.manifest.id, "example-skill");
        assert_eq!(skill.manifest.version, "0.1.0");
        assert_eq!(skill.manifest.description, "A test skill");
        assert!(!skill.body.trim().is_empty());
    }

    #[test]
    fn parse_anthropic_skill_missing_name() {
        let src = "---\ndescription: No name\n---\n\nBody.\n";
        let err = parse_anthropic_skill(src).unwrap_err();
        assert!(matches!(err, ESkillSchema::EmptyName));
    }

    #[test]
    fn parse_anthropic_skill_empty_body() {
        let src = "---\nname: test\ndescription: Has no body\n---\n";
        let err = parse_anthropic_skill(src).unwrap_err();
        assert!(matches!(err, ESkillSchema::EmptyBody));
    }

    #[test]
    fn parse_anthropic_skill_missing_fence() {
        let src = "name: test\ndescription: No fences\n\nBody.\n";
        let err = parse_anthropic_skill(src).unwrap_err();
        assert!(matches!(err, ESkillSchema::MissingFence));
    }

    #[test]
    fn parse_real_fixture() {
        let fixture = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/anthropic-skill/SKILL.md"
        ))
        .expect("fixture must exist");
        let skill = parse_anthropic_skill(&fixture).unwrap();
        assert_eq!(skill.manifest.name, "skill-creator");
        assert_eq!(skill.manifest.id, "skill-creator");
    }

    #[test]
    fn parse_invalid_fixture_is_red() {
        let fixture = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/anthropic-skill-invalid/SKILL.md"
        ))
        .expect("invalid fixture must exist");
        let err = parse_anthropic_skill(&fixture).unwrap_err();
        assert!(matches!(err, ESkillSchema::EmptyName));
    }
}
