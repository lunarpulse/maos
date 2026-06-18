//! `maos.skill.v1` schema — markdown + TOML frontmatter per ADR-027
//! (intentionally close to the Anthropic Skills format).
//!
//! The kernel validates the SCHEMA (frontmatter well-formed + body present)
//! and treats the markdown `body` as OPAQUE (§4.0.7 kernel non-interpretability:
//! the kernel does NOT parse, rank, curate, or execute skill content).

use std::collections::BTreeSet;
use std::fmt;

use maos_spirit_abi::compliance::CapabilityId;

use crate::errors::ESkillSchema;

/// A skill identifier (the `id` of a `maos.skill.v1` document, or an FR57
/// proposal's `target_skill_id`). Newtype over `String` for type safety at
/// the queue / proposal boundary.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct SkillId(pub String);

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SkillId {
    fn from(s: &str) -> Self {
        SkillId(s.to_string())
    }
}


/// A skill version (semver). Newtype over `String`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct SkillVersion(pub String);

impl fmt::Display for SkillVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SkillVersion {
    fn from(s: &str) -> Self {
        SkillVersion(s.to_string())
    }
}


/// The TOML frontmatter of a `maos.skill.v1` document.
///
/// `#[serde(deny_unknown_fields)]` is load-bearing: an unknown field is a
/// `ESkillSchema::UnknownField`, never a silently coerced default.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillManifest {
    /// Skill id — non-empty, kebab/dotted charset.
    pub id: String,
    /// Semver version string (validated via `semver::Version::parse`).
    pub version: String,
    /// Human-readable name — non-empty.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Capabilities the skill requires (opaque to the kernel beyond the set
    /// membership; sorted/canonical via `CapabilityId`'s `Ord`).
    #[serde(default)]
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// Minimum substrate version the skill targets. PARSED for schema
    /// completeness; kernel-load ENFORCEMENT is Story 7.5a (the ABI Stability
    /// Triple), NOT this story.
    #[serde(default)]
    pub min_substrate_version: Option<String>,
}

impl SkillManifest {
    /// The skill id as a typed [`SkillId`].
    pub fn skill_id(&self) -> SkillId {
        SkillId(self.id.clone())
    }

    /// The skill version as a typed [`SkillVersion`].
    pub fn skill_version(&self) -> SkillVersion {
        SkillVersion(self.version.clone())
    }
}

/// A parsed `maos.skill.v1` skill: validated frontmatter + opaque markdown body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    /// The validated TOML frontmatter.
    pub manifest: SkillManifest,
    /// The markdown body — OPAQUE to the kernel (§4.0.7). The kernel does NOT
    /// parse this for meaning.
    pub body: String,
}

const FENCE: &str = "---";

/// Parse and validate a `maos.skill.v1` document.
///
/// Splits the leading `---` … `---` TOML frontmatter fence from the markdown
/// body, parses the frontmatter STRICTLY (`deny_unknown_fields`), and validates
/// `id` (non-empty + charset), `version` (valid semver), `name` (non-empty),
/// and body presence. The body is NOT parsed for meaning (§4.0.7).
pub fn parse_skill(src: &str) -> Result<Skill, ESkillSchema> {
    // The document must OPEN with a fence line. Allow a leading BOM / blank
    // lines before the opening fence (common in editor output), but the first
    // non-blank line must be exactly `---`.
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

    // First content line must be the opening fence.
    let after_open = {
        let line_end = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let (line, after) = rest.split_at(line_end);
        if line.trim() != FENCE {
            return Err(ESkillSchema::MissingFence);
        }
        after
    };

    // Find the closing fence line; everything after it is the body.
    let mut frontmatter = String::new();
    let mut cursor = after_open;
    let body = loop {
        if cursor.is_empty() {
            // Reached EOF without a closing fence.
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

    // Parse the frontmatter strictly.
    let manifest: SkillManifest = toml::from_str(&frontmatter).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unknown field") {
            ESkillSchema::UnknownField(msg)
        } else {
            ESkillSchema::TomlParse(msg)
        }
    })?;

    validate_manifest(&manifest)?;

    if body.trim().is_empty() {
        return Err(ESkillSchema::EmptyBody);
    }

    Ok(Skill { manifest, body })
}

/// Validate a [`SkillManifest`]'s field contents (charset, semver, non-empty).
pub fn validate_manifest(manifest: &SkillManifest) -> Result<(), ESkillSchema> {
    if manifest.id.is_empty() {
        return Err(ESkillSchema::EmptyId);
    }
    if !manifest
        .id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
    {
        return Err(ESkillSchema::InvalidIdCharset(manifest.id.clone()));
    }
    if manifest.name.is_empty() {
        return Err(ESkillSchema::EmptyName);
    }
    semver::Version::parse(&manifest.version)
        .map_err(|e| ESkillSchema::InvalidSemver(manifest.version.clone(), e.to_string()))?;
    Ok(())
}
